//! S0 benchmark runner: pre-registered sessions, paired medians, Law-2 output check.
//!
//! Config schema (strict minjson subset):
//! {
//!   "name": "s0-smoke", "seed": 12345, "cooldown_secs": 2, "discard_first": 1,
//!   "logger_cpu": 0,                       // optional; runner self-pin
//!   "pairs": [ {"a": RUN, "b": RUN, "expect_pi": "50847534"} ],
//!   "singles": [ {"run": RUN, "expect_pi": "123"} ]   // optional
//! }
//! RUN = {"label": "...", "argv": ["prog", "args..."], "env": {"K":"V"}}
//!
//! Measurement rules (spec S0/S3):
//! - Wall clock is `Instant::now()` around fork/exec+waitpid, i.e.
//!   CLOCK_MONOTONIC on Linux — never wall-clock time, never in-process
//!   self-timing for the claim.
//! - d = wall(b) − wall(a) per pair; analysis uses kept pairs only.
//! - Child stdout MUST contain `expect_pi` digits and exit 0, else the
//!   session aborts immediately (exit 3): a wrong π is a bug, not noise.
//! - Fixed cooldown sleep after every run except the session's last.

use crate::minjson::J;
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// tiny RNG (xorshift64*, seeded, reproducible coin flips)
// ---------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        if x == 0 {
            x = 0x9E3779B97F4A7C15;
        }
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
}

// ---------------------------------------------------------------------------
// config model
// ---------------------------------------------------------------------------

struct RunSpec {
    label: String,
    argv: Vec<String>,
    env: Vec<(String, String)>,
    /// Optional path: full stderr bytes are written here (e.g. oracle
    /// per-unit partial dumps, which are too large for session JSON).
    capture_stderr: Option<String>,
}

struct PairSpec {
    a: RunSpec,
    b: RunSpec,
    expect_pi: Option<String>,
}

struct SingleSpec {
    run: RunSpec,
    expect_pi: Option<String>,
}

struct Config {
    name: String,
    seed: u64,
    cooldown_secs: u64,
    discard_first: usize,
    logger_cpu: Option<usize>,
    pairs: Vec<PairSpec>,
    singles: Vec<SingleSpec>,
}

fn get_str(v: &J, key: &str) -> Result<Option<String>, String> {
    match v.get(key) {
        None => Ok(None),
        Some(J::Str(s)) => Ok(Some(s.clone())),
        Some(_) => Err(format!("config: '{}' must be a string", key)),
    }
}

fn get_u64(v: &J, key: &str, default: u64) -> Result<u64, String> {
    match v.get(key) {
        None => Ok(default),
        Some(x) => x.as_u64().ok_or_else(|| format!("config: '{}' must be a uint", key)),
    }
}

fn parse_run(v: &J) -> Result<RunSpec, String> {
    let label = get_str(v, "label")?.ok_or("config: run missing 'label'")?;
    let argv_j = v.get("argv").ok_or("config: run missing 'argv'")?;
    let argv_arr = argv_j.as_arr().ok_or("config: 'argv' must be an array")?;
    if argv_arr.is_empty() {
        return Err("config: 'argv' must not be empty".into());
    }
    let mut argv = Vec::new();
    for a in argv_arr {
        argv.push(a.as_str().ok_or("config: argv entries must be strings")?.to_string());
    }
    let mut env = Vec::new();
    if let Some(e) = v.get("env") {
        match e {
            J::Obj(pairs) => {
                for (k, val) in pairs {
                    let vs = val.as_str().ok_or("config: env values must be strings")?;
                    env.push((k.clone(), vs.to_string()));
                }
            }
            _ => return Err("config: 'env' must be an object".into()),
        }
    }
    let capture_stderr = get_str(v, "capture_stderr")?;
    Ok(RunSpec { label, argv, env, capture_stderr })
}

fn parse_config(v: &J) -> Result<Config, String> {
    let name = get_str(v, "name")?.unwrap_or_else(|| "session".into());
    let seed = get_u64(v, "seed", 0x12345678)?;
    let cooldown_secs = get_u64(v, "cooldown_secs", 5)?;
    let discard_first = get_u64(v, "discard_first", 0)? as usize;
    let logger_cpu = match v.get("logger_cpu") {
        None => None,
        Some(x) => Some(x.as_u64().ok_or("config: 'logger_cpu' must be a uint")? as usize),
    };
    let mut pairs = Vec::new();
    if let Some(pj) = v.get("pairs") {
        for p in pj.as_arr().ok_or("config: 'pairs' must be an array")? {
            pairs.push(PairSpec {
                a: parse_run(p.get("a").ok_or("config: pair missing 'a'")?)?,
                b: parse_run(p.get("b").ok_or("config: pair missing 'b'")?)?,
                expect_pi: get_str(p, "expect_pi")?,
            });
        }
    }
    let mut singles = Vec::new();
    if let Some(sj) = v.get("singles") {
        for s in sj.as_arr().ok_or("config: 'singles' must be an array")? {
            singles.push(SingleSpec {
                run: parse_run(s.get("run").ok_or("config: single missing 'run'")?)?,
                expect_pi: get_str(s, "expect_pi")?,
            });
        }
    }
    if pairs.is_empty() && singles.is_empty() {
        return Err("config: need at least one pair or single".into());
    }
    Ok(Config { name, seed, cooldown_secs, discard_first, logger_cpu, pairs, singles })
}

// ---------------------------------------------------------------------------
// environment facts (all best-effort; null when unmeasurable)
// ---------------------------------------------------------------------------

fn cmd_out(argv: &[&str]) -> Option<String> {
    let out = std::process::Command::new(argv[0])
        .args(&argv[1..])
        .output()
        .ok()?;
    if out.status.success() {
        String::from_utf8(out.stdout).ok()
    } else {
        None
    }
}

fn git_sha() -> J {
    // Workspace root discovery: walk up from CWD to a Cargo.toml with [workspace].
    let mut dir = std::env::current_dir().ok();
    let mut root: Option<std::path::PathBuf> = None;
    while let Some(d) = dir {
        let cand = d.join("Cargo.toml");
        if let Ok(txt) = std::fs::read_to_string(&cand) {
            if txt.contains("[workspace]") {
                root = Some(d.clone());
                break;
            }
        }
        dir = d.parent().map(|p| p.to_path_buf());
    }
    match root {
        None => J::Null,
        Some(r) => {
            let mut cmd = std::process::Command::new("git");
            cmd.arg("-C")
                .arg(&r)
                .arg("rev-parse")
                .arg("HEAD");
            match cmd.output() {
                Ok(o) if o.status.success() => {
                    J::Str(String::from_utf8_lossy(&o.stdout).trim().to_string())
                }
                _ => J::Null,
            }
        }
    }
}

fn primecount_path() -> Option<String> {
    if let Ok(prefix) = std::env::var("PREFIX") {
        let p = format!("{}/bin/primecount", prefix);
        if std::fs::metadata(&p).is_ok() {
            return Some(p);
        }
    }
    for p in [
        "/data/data/com.termux/files/usr/bin/primecount",
        "/usr/local/bin/primecount",
        "/usr/bin/primecount",
    ] {
        if std::fs::metadata(p).is_ok() {
            return Some(p.to_string());
        }
    }
    None
}

fn opponent_sha256() -> J {
    match primecount_path() {
        None => J::Null,
        Some(p) => match cmd_out(&["sha256sum", &p]) {
            None => J::Null,
            Some(o) => J::Str(o.split_whitespace().next().unwrap_or("").to_string()),
        },
    }
}

fn battery_pct() -> J {
    // termux-battery-status absent on this box → null (documented).
    match cmd_out(&["termux-battery-status"]) {
        None => J::Null,
        Some(o) => {
            // Crude: first integer after "percentage".
            match o.find("percentage").and_then(|i| {
                o[i..]
                    .split(|c: char| !c.is_ascii_digit())
                    .filter(|s| !s.is_empty())
                    .next()
                    .and_then(|n| n.parse::<f64>().ok())
            }) {
                Some(p) => J::Num(p),
                None => J::Null,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// freq logger thread (100ms cpu clocks, 1s thermal zones)
// ---------------------------------------------------------------------------

struct Sample {
    t_ms: u64,
    cpu_khz: Vec<Option<u64>>,
    zones: Vec<(String, i64)>,
}

fn cpu_inventory() -> Vec<usize> {
    let mut cpus: Vec<usize> = (0..64)
        .filter(|c| {
            std::fs::metadata(format!("/sys/devices/system/cpu/cpu{}", c)).is_ok()
        })
        .collect();
    cpus.truncate(32);
    cpus
}

fn spawn_logger(stop: Arc<AtomicBool>, out: Arc<Mutex<Vec<Sample>>>) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let cpus = cpu_inventory();
        let t0 = Instant::now();
        let mut tick: u64 = 0;
        while !stop.load(Ordering::Relaxed) {
            let mut khz = Vec::with_capacity(cpus.len());
            for c in &cpus {
                let v = std::fs::read_to_string(format!(
                    "/sys/devices/system/cpu/cpu{}/cpufreq/scaling_cur_freq",
                    c
                ))
                .ok()
                .and_then(|s| s.trim().parse::<u64>().ok());
                khz.push(v);
            }
            let mut zones = Vec::new();
            if tick % 10 == 0 {
                for z in 0..64 {
                    let base = format!("/sys/class/thermal/thermal_zone{}", z);
                    if std::fs::metadata(&base).is_err() {
                        if z > 30 {
                            break;
                        }
                        continue;
                    }
                    let (ty, tp) = (
                        std::fs::read_to_string(format!("{}/type", base))
                            .map(|s| s.trim().to_string())
                            .unwrap_or_default(),
                        std::fs::read_to_string(format!("{}/temp", base))
                            .ok()
                            .and_then(|s| s.trim().parse::<i64>().ok())
                            .unwrap_or(-273000),
                    );
                    if tp > 0 {
                        zones.push((ty, tp));
                    }
                }
            }
            out.lock().unwrap().push(Sample { t_ms: t0.elapsed().as_millis() as u64, cpu_khz: khz, zones });
            tick += 1;
            std::thread::sleep(Duration::from_millis(100));
        }
    })
}

// ---------------------------------------------------------------------------
// run execution
// ---------------------------------------------------------------------------

struct RunResult {
    label: String,
    argv: Vec<String>,
    wall_ms: f64,
    exit_code: Option<i32>,
    pi_ok: bool,
    stdout_tail: String,
    stderr_file: Option<String>,
}

fn run_once(spec: &RunSpec, expect_pi: Option<&str>) -> Result<RunResult, String> {
    let mut cmd = std::process::Command::new(&spec.argv[0]);
    if spec.argv.len() > 1 {
        cmd.args(&spec.argv[1..]);
    }
    for (k, v) in &spec.env {
        cmd.env(k, v);
    }
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    let t0 = Instant::now();
    let out = cmd.output().map_err(|e| format!("spawn '{}' failed: {}", spec.argv[0], e))?;
    let wall_ms = t0.elapsed().as_secs_f64() * 1000.0;
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    if let Some(path) = &spec.capture_stderr {
        std::fs::write(path, &out.stderr)
            .map_err(|e| format!("capture_stderr write '{}' failed: {}", path, e))?;
    }
    let pi_ok = match expect_pi {
        None => true,
        Some(digits) => stdout.contains(digits),
    };
    let tail: String = stdout.lines().rev().take(3).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n");
    Ok(RunResult {
        label: spec.label.clone(),
        argv: spec.argv.clone(),
        wall_ms,
        exit_code: out.status.code(),
        pi_ok,
        stdout_tail: tail.chars().take(600).collect(),
        stderr_file: spec.capture_stderr.clone(),
    })
}

fn run_to_json(r: &RunResult, order: &str, pair: Option<usize>) -> J {
    J::Obj(vec![
        ("label".into(), J::Str(r.label.clone())),
        ("argv".into(), J::Arr(r.argv.iter().map(|a| J::Str(a.clone())).collect())),
        ("wall_ms".into(), J::Num(r.wall_ms)),
        ("exit_code".into(), r.exit_code.map(|c| J::Num(c as f64)).unwrap_or(J::Null)),
        ("pi_ok".into(), J::Bool(r.pi_ok)),
        ("order".into(), J::Str(order.into())),
        ("stderr_file".into(), r.stderr_file.clone().map(J::Str).unwrap_or(J::Null)),
        ("pair".into(), pair.map(|p| J::Num(p as f64)).unwrap_or(J::Null)),
        ("stdout_tail".into(), J::Str(r.stdout_tail.clone())),
    ])
}

// ---------------------------------------------------------------------------
// session driver
// ---------------------------------------------------------------------------

/// Run a session. Returns (exit_code, session_json).
/// Exit 0 = complete; exit 3 = aborted (bad exit / π mismatch).
pub fn run_session(config_text: &str) -> (i32, J) {
    let cfg = match crate::minjson::parse(config_text).and_then(|v| {
        parse_config(&v).map_err(|e| e)
    }) {
        Ok(c) => c,
        Err(e) => {
            return (
                2,
                J::Obj(vec![
                    ("error".into(), J::Str(format!("config: {}", e))),
                ]),
            );
        }
    };

    // Self-pin the runner/logger (logged; best-effort).
    let logger_pinned = match cfg.logger_cpu {
        None => J::Str("unpinned".into()),
        Some(cpu) => match titan_bench::pin::set_affinity(cpu) {
            Ok(()) => J::Str(format!("cpu{}", cpu)),
            Err(e) => J::Str(format!("pin-failed cpu{}: {}", cpu, e)),
        },
    };

    let mut rng = Rng(cfg.seed);
    let stop = Arc::new(AtomicBool::new(false));
    let samples: Arc<Mutex<Vec<Sample>>> = Arc::new(Mutex::new(Vec::new()));
    let logger = spawn_logger(stop.clone(), samples.clone());

    let mut runs_json = Vec::new();
    let mut pairs_json = Vec::new();
    let mut walls_a: HashMap<usize, f64> = HashMap::new();
    let mut walls_b: HashMap<usize, f64> = HashMap::new();
    let mut orders: HashMap<usize, String> = HashMap::new();
    let mut aborted: Option<String> = None;
    let mut total_runs = cfg.pairs.len() * 2 + cfg.singles.len();
    let mut done_runs = 0usize;

    macro_rules! cooldown_unless_last {
        () => {
            done_runs += 1;
            if done_runs < total_runs && cfg.cooldown_secs > 0 {
                std::thread::sleep(Duration::from_secs(cfg.cooldown_secs));
            }
        };
    }

    for (i, p) in cfg.pairs.iter().enumerate() {
        if aborted.is_some() {
            break;
        }
        let b_first = rng.next() & 1 == 1;
        let order = if b_first { "ba" } else { "ab" };
        orders.insert(i, order.to_string());
        let seq: [(&RunSpec, &str); 2] = if b_first { [(&p.b, "b"), (&p.a, "a")] } else { [(&p.a, "a"), (&p.b, "b")] };
        for (spec, side) in seq {
            match run_once(spec, p.expect_pi.as_deref()) {
                Err(e) => {
                    aborted = Some(format!("pair {} run '{}': {}", i, spec.label, e));
                    break;
                }
                Ok(r) => {
                    if r.exit_code != Some(0) {
                        aborted = Some(format!(
                            "pair {} run '{}': exit {:?}\nstderr-tail:\n{}",
                            i, r.label, r.exit_code, r.stdout_tail
                        ));
                        runs_json.push(run_to_json(&r, side, Some(i)));
                        cooldown_unless_last!();
                        break;
                    }
                    if !r.pi_ok {
                        aborted = Some(format!(
                            "pair {} run '{}': PI MISMATCH (Law 2 - aborting, no median)",
                            i, r.label
                        ));
                        runs_json.push(run_to_json(&r, side, Some(i)));
                        cooldown_unless_last!();
                        break;
                    }
                    if side == "a" {
                        walls_a.insert(i, r.wall_ms);
                    } else {
                        walls_b.insert(i, r.wall_ms);
                    }
                    runs_json.push(run_to_json(&r, side, Some(i)));
                    cooldown_unless_last!();
                }
            }
            if aborted.is_some() {
                break;
            }
        }
        // d = wall(b) - wall(a); only when both sides present.
        let d = match (walls_a.get(&i), walls_b.get(&i)) {
            (Some(a), Some(b)) => Some(b - a),
            _ => None,
        };
        pairs_json.push(J::Obj(vec![
            ("i".into(), J::Num(i as f64)),
            ("order".into(), J::Str(order.to_string())),
            ("kept".into(), J::Bool(i >= cfg.discard_first && d.is_some())),
            ("d_ms".into(), d.map(J::Num).unwrap_or(J::Null)),
        ]));
    }

    if aborted.is_none() {
        for s in &cfg.singles {
            match run_once(&s.run, s.expect_pi.as_deref()) {
                Err(e) => {
                    aborted = Some(format!("single '{}': {}", s.run.label, e));
                    break;
                }
                Ok(r) => {
                    if r.exit_code != Some(0) || !r.pi_ok {
                        aborted = Some(format!(
                            "single '{}': exit {:?} pi_ok={} (aborting)",
                            r.label, r.exit_code, r.pi_ok
                        ));
                    }
                    runs_json.push(run_to_json(&r, "single", None));
                    cooldown_unless_last!();
                }
            }
            if aborted.is_some() {
                break;
            }
        }
    }
    stop.store(true, Ordering::Relaxed);
    logger.join().ok();
    let trace = samples.lock().unwrap();
    let mut trace_json = Vec::with_capacity(trace.len());
    for s in trace.iter() {
        trace_json.push(J::Obj(vec![
            ("t_ms".into(), J::Num(s.t_ms as f64)),
            (
                "cpu_khz".into(),
                J::Arr(s.cpu_khz.iter().map(|v| v.map(|x| J::Num(x as f64)).unwrap_or(J::Null)).collect()),
            ),
            (
                "zones".into(),
                J::Arr(s.zones.iter().map(|(t, v)| {
                    J::Obj(vec![("type".into(), J::Str(t.clone())), ("temp_mC".into(), J::Num(*v as f64))])
                }).collect()),
            ),
        ]));
    }

    // Summary over kept pairs.
    let mut kept_d: Vec<(usize, f64)> = Vec::new();
    for (i, _) in cfg.pairs.iter().enumerate() {
        if i < cfg.discard_first {
            continue;
        }
        if let (Some(a), Some(b)) = (walls_a.get(&i), walls_b.get(&i)) {
            kept_d.push((i, b - a));
        }
    }
    let dvals: Vec<f64> = kept_d.iter().map(|(_, d)| *d).collect();
    let summary = if dvals.is_empty() {
        J::Obj(vec![("kept_pairs".into(), J::Num(0.0))])
    } else {
        let st = titan_bench::stats::describe(dvals.clone());
        let sign_frac = dvals.iter().filter(|&&d| d > 0.0).count() as f64 / dvals.len() as f64;
        let mut ab = Vec::new();
        let mut ba = Vec::new();
        for (i, d) in &kept_d {
            match orders.get(i).map(|s| s.as_str()) {
                Some("ab") => ab.push(*d),
                _ => ba.push(*d),
            }
        }
        let med = |mut v: Vec<f64>| -> J {
            if v.is_empty() {
                return J::Null;
            }
            J::Num(titan_bench::stats::describe(v).median)
        };
        // Per-label p50 over kept runs.
        let mut by_label: HashMap<String, Vec<f64>> = HashMap::new();
        for r in runs_json.iter() {
            let pair_idx = r.get("pair").and_then(|v| v.as_f64()).map(|v| v as usize);
            let keep = match pair_idx {
                Some(pi) => pi >= cfg.discard_first && walls_a.contains_key(&pi) && walls_b.contains_key(&pi),
                None => true,
            };
            if keep {
                if let (Some(J::Str(lab)), Some(J::Num(w))) = (r.get("label"), r.get("wall_ms")) {
                    by_label.entry(lab.clone()).or_default().push(*w);
                }
            }
        }
        let mut per_label = Vec::new();
        let mut labels: Vec<String> = by_label.keys().cloned().collect();
        labels.sort();
        for lab in labels {
            let stl = titan_bench::stats::describe(by_label[&lab].clone());
            per_label.push(J::Obj(vec![
                ("label".into(), J::Str(lab)),
                ("n".into(), J::Num(stl.n as f64)),
                ("p50_ms".into(), J::Num(stl.median)),
                ("mad_ms".into(), J::Num(stl.mad)),
                ("min_ms".into(), J::Num(stl.min)),
                ("max_ms".into(), J::Num(stl.max)),
            ]));
        }
        J::Obj(vec![
            ("kept_pairs".into(), J::Num(dvals.len() as f64)),
            ("median_d_ms".into(), J::Num(st.median)),
            ("mad_d_ms".into(), J::Num(st.mad)),
            ("sign_frac_b_slower".into(), J::Num(sign_frac)),
            ("median_d_ab_order".into(), med(ab)),
            ("median_d_ba_order".into(), med(ba)),
            ("per_label_p50".into(), J::Arr(per_label)),
        ])
    };

    let mut doc = vec![
        ("tool".into(), J::Str("titan-rig run (Phase 0 S0)".into())),
        ("name".into(), J::Str(cfg.name.clone())),
        ("seed".into(), J::Num(cfg.seed as f64)),
        ("titan_sha".into(), git_sha()),
        ("opponent_sha256".into(), opponent_sha256()),
        ("battery_pct".into(), battery_pct()),
        ("airplane_mode".into(), J::Null),
        ("screen".into(), J::Null),
        ("charging".into(), J::Null),
        (
            "loadavg_start".into(),
            std::fs::read_to_string("/proc/loadavg")
                .ok()
                .and_then(|s| s.split_whitespace().next().and_then(|v| v.parse::<f64>().ok()))
                .map(J::Num)
                .unwrap_or(J::Null),
        ),
        ("logger_pin".into(), logger_pinned),
        ("cooldown_secs".into(), J::Num(cfg.cooldown_secs as f64)),
        ("discard_first".into(), J::Num(cfg.discard_first as f64)),
        ("runs".into(), J::Arr(runs_json)),
        ("pairs".into(), J::Arr(pairs_json)),
        ("summary".into(), summary),
        ("freq_trace".into(), J::Arr(trace_json)),
    ];
    if let Some(e) = aborted {
        doc.push(("aborted".into(), J::Str(e)));
        return (3, J::Obj(doc));
    }
    (0, J::Obj(doc))
}
