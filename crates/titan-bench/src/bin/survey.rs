//! Per-core solo survey, all-core contention pass, thermal heatsoak.
//! Usage:
//!   survey [--baseline] [--samples N] [--proxy-limit N] [--heatsoak SECS] [--heatsoak-only]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Barrier, Mutex};
use std::time::{Duration, Instant};
use titan_bench::{canary, pin, proxy, snapshot, stats, topology};

#[derive(Clone, Debug)]
struct CoreResult {
    cpu: usize,
    part: Option<u32>,
    cluster: String,
    canary: f64,
    mem: f64,
    proxy_rate: f64,
    allcore_rate: f64,
}

fn median3_proxy(limit: u64) -> f64 {
    let mut ts = Vec::new();
    for _ in 0..3 {
        let t = Instant::now();
        let c = proxy::pi_proxy(limit);
        ts.push(t.elapsed());
        std::hint::black_box(c);
    }
    ts.sort();
    limit as f64 / ts[1].as_secs_f64()
}

fn get_arg_value<T: std::str::FromStr>(args: &[String], flag: &str, default: T) -> T {
    for (i, arg) in args.iter().enumerate() {
        if arg == flag {
            if let Some(next) = args.get(i + 1) {
                if let Ok(v) = next.parse::<T>() {
                    return v;
                }
            }
        } else if let Some(stripped) = arg.strip_prefix(flag) {
            if let Some(val_str) = stripped.strip_prefix('=') {
                if let Ok(v) = val_str.parse::<T>() {
                    return v;
                }
            }
        }
    }
    default
}

fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|a| a == flag || a.starts_with(&format!("{flag}=")))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let do_baseline = has_flag(&args, "--baseline");
    let heatsoak_only = has_flag(&args, "--heatsoak-only");
    let heatsoak_secs: Option<u64> = if has_flag(&args, "--heatsoak") || heatsoak_only {
        Some(get_arg_value(&args, "--heatsoak", 90u64))
    } else {
        None
    };
    let samples: u32 = get_arg_value(&args, "--samples", 25u32);
    let proxy_limit: u64 = get_arg_value(&args, "--proxy-limit", 100_000_000u64);

    let _wl = snapshot::WakeLock::acquire();
    let (aborts, warns) = snapshot::hygiene();
    for w in warns {
        eprintln!("[hygiene warn] {w}");
    }
    if !aborts.is_empty() && !has_flag(&args, "--force") {
        eprintln!("[hygiene ABORT] {:?}", aborts);
        eprintln!("Pass --force to override for debugging.");
        return;
    }

    let cores = topology::read();
    let n = cores.len().max(1);
    println!("== TITAN-PHASE0 SURVEY == Total Cores Detected: {n}");
    for c in &cores {
        let name = c
            .part
            .map(|p| topology::part_name(p))
            .unwrap_or_else(|| "?".into());
        println!(
            "  cpu{} : {} (max_freq: {:?} kHz)",
            c.cpu,
            name,
            topology::read_max_freq(c.cpu)
        );
    }

    let can = canary::CpuCanary::with_epochs(canary::CANARY_M);
    let mem = canary::MemCanary::new();

    if !heatsoak_only {
        println!("\n--- 1. SOLO PASS (3 Probes per core) ---");
        let mut solo: Vec<CoreResult> = Vec::new();
        for c in &cores {
            if let Err(e) = pin::set_affinity(c.cpu) {
                eprintln!("cpu{} pin failed: {e} (Termux backgrounded? skipping)", c.cpu);
                continue;
            }
            let cr = can.rate(3, samples);
            let mr = mem.rate(2, 11);
            let pr = median3_proxy(proxy_limit);
            println!(
                "  cpu{} [{:>3}]: Canary={:>6.1} ep/s | Mem={:>5.1} ep/s | Proxy={:>6.2}M n/s",
                c.cpu,
                c.part.map(|p| topology::part_name(p)).unwrap_or_else(|| "?".into()),
                cr,
                mr,
                pr / 1e6
            );
            solo.push(CoreResult {
                cpu: c.cpu,
                part: c.part,
                cluster: "unknown".into(),
                canary: cr,
                mem: mr,
                proxy_rate: pr,
                allcore_rate: 0.0,
            });
        }

        // Restore affinity mask invariant before clustering & multi-threading
        let _ = pin::set_full_affinity();

        // 1-D Clustering by maximum ratio gap in proxy sieve throughput
        let mut ranked = solo.clone();
        ranked.sort_by(|a, b| b.proxy_rate.partial_cmp(&a.proxy_rate).unwrap_or(std::cmp::Ordering::Equal));
        let mut best_gap = 1.0f64;
        let mut split_idx = 0usize;
        for i in 1..ranked.len() {
            let gap = ranked[i - 1].proxy_rate / ranked[i].proxy_rate.max(1.0);
            if gap > best_gap {
                best_gap = gap;
                split_idx = i;
            }
        }

        println!("\n--- 2. CLUSTER INFERENCE (Max Gap = {:.2}x) ---", best_gap);
        let is_heterogeneous = best_gap >= 1.5;
        for r in &mut solo {
            let is_big = is_heterogeneous && {
                let pos = ranked.iter().position(|x| x.cpu == r.cpu).unwrap_or(999);
                pos < split_idx
            };
            r.cluster = if is_big { "big".into() } else { "little".into() };
        }

        let big_canary_mean = {
            let bigs: Vec<f64> = solo.iter().filter(|r| r.cluster == "big").map(|r| r.canary).collect();
            if bigs.is_empty() { 1.0 } else { bigs.iter().sum::<f64>() / bigs.len() as f64 }
        };
        let lit_canary_mean = {
            let lits: Vec<f64> = solo.iter().filter(|r| r.cluster == "little").map(|r| r.canary).collect();
            if lits.is_empty() { 1.0 } else { lits.iter().sum::<f64>() / lits.len() as f64 }
        };
        let big_proxy_mean = {
            let bigs: Vec<f64> = solo.iter().filter(|r| r.cluster == "big").map(|r| r.proxy_rate).collect();
            if bigs.is_empty() { 1.0 } else { bigs.iter().sum::<f64>() / bigs.len() as f64 }
        };
        let lit_proxy_mean = {
            let lits: Vec<f64> = solo.iter().filter(|r| r.cluster == "little").map(|r| r.proxy_rate).collect();
            if lits.is_empty() { 1.0 } else { lits.iter().sum::<f64>() / lits.len() as f64 }
        };
        let big_mem_mean = {
            let bigs: Vec<f64> = solo.iter().filter(|r| r.cluster == "big").map(|r| r.mem).collect();
            if bigs.is_empty() { 1.0 } else { bigs.iter().sum::<f64>() / bigs.len() as f64 }
        };
        let lit_mem_mean = {
            let lits: Vec<f64> = solo.iter().filter(|r| r.cluster == "little").map(|r| r.mem).collect();
            if lits.is_empty() { 1.0 } else { lits.iter().sum::<f64>() / lits.len() as f64 }
        };

        let canary_ratio = big_canary_mean / lit_canary_mean.max(1.0);
        let proxy_ratio = big_proxy_mean / lit_proxy_mean.max(1.0);
        let mem_ratio = big_mem_mean / lit_mem_mean.max(1.0);

        println!("  Inferred Clusters: {} big cores, {} little cores",
            solo.iter().filter(|r| r.cluster == "big").count(),
            solo.iter().filter(|r| r.cluster == "little").count()
        );
        println!("  Big:Little CPU Canary Ratio : {:.2}x", canary_ratio);
        println!("  Big:Little Proxy Sieve Ratio: {:.2}x", proxy_ratio);
        println!("  Big:Little Memory Bandwidth : {:.2}x (Asymmetry factor)", mem_ratio);

        // --- 3. ALL-CORE CONTENTION PASS ---
        println!("\n--- 3. ALL-CORE CONTENTION PASS ---");
        let _ = pin::set_full_affinity();
        let barrier = Arc::new(Barrier::new(cores.len()));
        let mut handles = Vec::new();
        let allcore_results = Arc::new(Mutex::new(Vec::new()));

        for c in &cores {
            let cpu = c.cpu;
            let bar = barrier.clone();
            let res_arc = allcore_results.clone();
            let limit = proxy_limit;

            handles.push(std::thread::spawn(move || {
                if let Err(e) = pin::set_affinity(cpu) {
                    eprintln!("Worker cpu{} pin failed: {e}", cpu);
                    bar.wait();
                    return;
                }
                bar.wait(); // Synchronous start
                let rate = median3_proxy(limit);
                res_arc.lock().unwrap().push((cpu, rate));
            }));
        }

        for h in handles {
            let _ = h.join();
        }
        let _ = pin::set_full_affinity();

        let ac_guard = allcore_results.lock().unwrap();
        for r in &mut solo {
            if let Some(&(_, ac_rate)) = ac_guard.iter().find(|(cpu, _)| *cpu == r.cpu) {
                r.allcore_rate = ac_rate;
            }
        }

        let solo_sum: f64 = solo.iter().map(|r| r.proxy_rate).sum();
        let allcore_sum: f64 = solo.iter().map(|r| r.allcore_rate).sum();
        let contention_eff = allcore_sum / solo_sum.max(1.0);

        println!("  Solo Sum Throughput    : {:.2}M n/s", solo_sum / 1e6);
        println!("  All-Core Aggregate     : {:.2}M n/s", allcore_sum / 1e6);
        println!("  Contention Efficiency  : {:.1}% (All-Core / Solo Sum)", contention_eff * 100.0);
        for r in &solo {
            let eff = r.allcore_rate / r.proxy_rate.max(1.0) * 100.0;
            println!("    cpu{} ({}): Solo={:>6.2}M -> AllCore={:>6.2}M ({:.1}% retained)",
                r.cpu, r.cluster, r.proxy_rate / 1e6, r.allcore_rate / 1e6, eff);
        }

        // Write baselines.json if requested
        if do_baseline {
            let env = snapshot::snapshot();
            let json = format!(
                r#"{{"rustc":"{}","canary_ratio":{:.2},"proxy_ratio":{:.2},"contention_eff":{:.4},"cores":[{}]}}"#,
                snapshot::json_esc(&env.rustc),
                canary_ratio,
                proxy_ratio,
                contention_eff,
                solo.iter()
                    .map(|r| format!(
                        r#"{{"cpu":{},"cluster":"{}","canary":{:.1},"proxy":{:.2},"allcore":{:.2}}}"#,
                        r.cpu, r.cluster, r.canary, r.proxy_rate, r.allcore_rate
                    ))
                    .collect::<Vec<_>>()
                    .join(",")
            );
            std::fs::create_dir_all("bench").unwrap();
            std::fs::write("bench/baselines.json", &json).unwrap();
            println!("\n[+] Baselines persisted to bench/baselines.json");
        }

        // Record JSON
        let env = snapshot::snapshot();
        let record_json = format!(
            r#"{{"timestamp":{},"canary_ratio":{:.2},"proxy_ratio":{:.2},"contention_efficiency":{:.4},"cores":[{}]}}"#,
            env.epoch_secs,
            canary_ratio,
            proxy_ratio,
            contention_eff,
            solo.iter()
                .map(|r| format!(
                    r#"{{"cpu":{},"cluster":"{}","canary":{:.1},"proxy":{:.2},"allcore":{:.2}}}"#,
                    r.cpu, r.cluster, r.canary, r.proxy_rate, r.allcore_rate
                ))
                .collect::<Vec<_>>()
                .join(",")
        );
        let _ = snapshot::write_record("survey", &record_json);
    }

    // --- 4. HEATSOAK (THERMAL DERATE CHARACTERIZATION) ---
    if let Some(soak_secs) = heatsoak_secs {
        println!("\n--- 4. HEATSOAK THERMAL CHARACTERIZATION ({}s) ---", soak_secs);
        let _ = pin::set_full_affinity();

        // Identify second fastest big core for canary probe
        let ranked = {
            let mut s = Vec::new();
            for c in &cores {
                if pin::set_affinity(c.cpu).is_ok() {
                    s.push((c.cpu, can.rate(1, 5)));
                }
            }
            let _ = pin::set_full_affinity();
            s.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            s
        };

        let canary_cpu = if ranked.len() > 1 { ranked[1].0 } else { ranked[0].0 };
        let worker_cpus: Vec<usize> = cores.iter().map(|c| c.cpu).filter(|&c| c != canary_cpu).collect();

        // Baseline cold canary
        let _ = pin::set_affinity(canary_cpu);
        let cold_canary = can.rate(3, 15);
        let _ = pin::set_full_affinity();
        println!("  Canary Core: cpu{} (Cold Baseline: {:.1} ep/s)", canary_cpu, cold_canary);
        println!("  Worker Cores: {:?}", worker_cpus);

        let running = Arc::new(AtomicBool::new(true));
        let worker_completions = Arc::new(Mutex::new(Vec::new()));
        let mut worker_handles = Vec::new();

        let t0 = Instant::now();
        for &cpu in &worker_cpus {
            let run_flag = running.clone();
            let comps_arc = worker_completions.clone();
            worker_handles.push(std::thread::spawn(move || {
                let _ = pin::set_affinity(cpu);
                let mut count = 0u64;
                while run_flag.load(Ordering::Relaxed) {
                    let c = proxy::pi_proxy(10_000_000);
                    std::hint::black_box(c);
                    count += 1;
                    comps_arc.lock().unwrap().push((cpu, t0.elapsed().as_secs_f64(), count));
                }
            }));
        }

        // Sampling loop on canary core
        let mut derate_samples = Vec::new();
        let deadline = Duration::from_secs(soak_secs);
        println!("  Sampling thermal derate at 250ms cadence...");

        while t0.elapsed() < deadline {
            let _ = pin::set_affinity(canary_cpu);
            let (rate, chk) = can.sample_once();
            std::hint::black_box(chk);
            let derate = rate / cold_canary.max(1.0);
            let elapsed = t0.elapsed().as_secs_f64();
            derate_samples.push((elapsed, derate));
            std::thread::sleep(Duration::from_millis(240));
        }

        running.store(false, Ordering::Relaxed);
        for h in worker_handles {
            let _ = h.join();
        }
        let _ = pin::set_full_affinity();

        let derates_only: Vec<f64> = derate_samples.iter().map(|(_, d)| *d).collect();
        let st = stats::describe(derates_only);
        let end_derate = derate_samples.last().map(|(_, d)| *d).unwrap_or(1.0);
        let min_derate = st.min;
        let samples_below_90 = derate_samples.iter().filter(|(_, d)| *d < 0.90).count();

        println!("\n=== HEATSOAK DERATE VERDICT ===");
        println!("  Cold Baseline Canary : {:.1} ep/s", cold_canary);
        println!("  Min Derate           : {:.3} ({:.1}% speed)", min_derate, min_derate * 100.0);
        println!("  End Derate           : {:.3} ({:.1}% speed)", end_derate, end_derate * 100.0);
        println!("  Median Derate        : {:.3} (MAD = {:.4})", st.median, st.mad);
        println!("  Throttled Samples (<0.90): {} / {}", samples_below_90, derate_samples.len());

        let env = snapshot::snapshot();
        let record_json = format!(
            r#"{{"heatsoak_secs":{},"cold_canary":{:.1},"min_derate":{:.4},"end_derate":{:.4},"median_derate":{:.4},"mad":{:.4},"throttled_count":{}}}"#,
            soak_secs, cold_canary, min_derate, end_derate, st.median, st.mad, samples_below_90
        );
        let _ = snapshot::write_record("heatsoak", &record_json);
    }
}
