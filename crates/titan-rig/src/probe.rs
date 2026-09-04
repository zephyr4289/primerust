//! S0 silicon probe: inventory the chip from sysfs, assume nothing.
//!
//! Reads (best-effort each; missing file → null, never a guess):
//! - `/sys/devices/system/cpu/{present,possible}`
//! - per-cpu `cpuinfo_max_freq` / `scaling_cur_freq` / `scaling_governor`
//! - raw ARM part id from /proc/cpuinfo (recorded verbatim; class comes from
//!   frequency, because part tables rot — e.g. SM4450 big cores read 0xd41,
//!   which legacy tables don't know)
//! - per-cpu cache topology: `cache/index*/{level,type,size,shared_cpu_list}`
//! - `thread_siblings_list` per cpu (SMT check)
//!
//! Class rule (spec): max_freq ≥ 2.0 GHz → A78, else A55. When the L1D sizes
//! can't be read, the documented SM4450 values are attached under
//! "assumed" (explicitly flagged, never merged into measured fields).

use crate::minjson::J;

fn read_trim(path: &str) -> Option<String> {
    std::fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

fn read_u64(path: &str) -> Option<u64> {
    read_trim(path)?.parse::<u64>().ok()
}

/// Raw ARM "CPU part" hex per cpu index, from /proc/cpuinfo.
fn cpu_parts() -> std::collections::HashMap<usize, String> {
    let mut map = std::collections::HashMap::new();
    let txt = std::fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    let mut cur: Option<usize> = None;
    for line in txt.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("processor") {
            cur = rest.split(':').nth(1).and_then(|v| v.trim().parse::<usize>().ok());
        } else if let Some(rest) = t.strip_prefix("CPU part") {
            if let Some(cpu) = cur {
                if let Some(hex) = rest.split(':').nth(1) {
                    map.insert(cpu, hex.trim().to_string());
                }
            }
        }
    }
    map
}

fn cpu_list(path: &str) -> Vec<usize> {
    // Handles "0-7" and "0,1,2" forms.
    let mut out = Vec::new();
    let txt = match read_trim(path) {
        Some(t) => t,
        None => return out,
    };
    for part in txt.split(',') {
        let part = part.trim();
        if let Some(dash) = part.find('-') {
            if let (Ok(a), Ok(b)) = (part[..dash].parse::<usize>(), part[dash + 1..].parse::<usize>()) {
                out.extend(a..=b);
            }
        } else if let Ok(n) = part.parse::<usize>() {
            out.push(n);
        }
    }
    out
}

fn cache_kb(text: &str) -> Option<u64> {
    // Forms like "32K".
    let t = text.trim();
    if let Some(k) = t.strip_suffix('K') {
        return k.parse::<u64>().ok();
    }
    if let Some(m) = t.strip_suffix('M') {
        return m.parse::<u64>().ok().map(|v| v * 1024);
    }
    None
}

pub fn probe() -> J {
    let parts = cpu_parts();
    let mut cpus: Vec<usize> = cpu_list("/sys/devices/system/cpu/present");
    if cpus.is_empty() {
        // Fallback: enumerate cpu[0-9]* dirs.
        cpus = (0..32)
            .filter(|c| {
                std::fs::metadata(format!("/sys/devices/system/cpu/cpu{}", c)).is_ok()
            })
            .collect();
    }

    let mut cpu_arr = Vec::new();
    let mut smt_found = false;
    for cpu in &cpus {
        let base = format!("/sys/devices/system/cpu/cpu{}", cpu);
        let max_khz = read_u64(&format!("{}/cpufreq/cpuinfo_max_freq", base));
        let cur_khz = read_u64(&format!("{}/cpufreq/scaling_cur_freq", base));
        let gov = read_trim(&format!("{}/cpufreq/scaling_governor", base));
        let sibs = read_trim(&format!("{}/topology/thread_siblings_list", base));
        if let Some(ref s) = sibs {
            // "6" alone = no SMT; "6-7"/"6,7" = threads share a core.
            if s.contains('-') || s.contains(',') {
                let only_self = s.trim() == cpu.to_string();
                if !only_self {
                    smt_found = true;
                }
            }
        }
        let class = match max_khz {
            Some(f) if f >= 2_000_000 => "A78",
            Some(_) => "A55",
            None => "unknown",
        };
        // Per-cpu cache entries.
        let mut caches = Vec::new();
        for idx in 0..8 {
            let cb = format!("{}/cache/index{}", base, idx);
            if std::fs::metadata(&cb).is_err() {
                break;
            }
            let level = read_trim(&format!("{}/level", cb));
            let ctype = read_trim(&format!("{}/type", cb));
            let size_kb = read_trim(&format!("{}/size", cb)).and_then(|s| cache_kb(&s));
            let shared = read_trim(&format!("{}/shared_cpu_list", cb));
            caches.push(J::Obj(vec![
                ("index".into(), J::Num(idx as f64)),
                ("level".into(), level.map(J::Str).unwrap_or(J::Null)),
                ("type".into(), ctype.map(J::Str).unwrap_or(J::Null)),
                ("size_kb".into(), size_kb.map(|v| J::Num(v as f64)).unwrap_or(J::Null)),
                ("shared_cpu_list".into(), shared.map(J::Str).unwrap_or(J::Null)),
            ]));
        }
        cpu_arr.push(J::Obj(vec![
            ("cpu".into(), J::Num(*cpu as f64)),
            ("part".into(), parts.get(cpu).cloned().map(J::Str).unwrap_or(J::Null)),
            ("max_freq_khz".into(), max_khz.map(|v| J::Num(v as f64)).unwrap_or(J::Null)),
            ("cur_freq_khz".into(), cur_khz.map(|v| J::Num(v as f64)).unwrap_or(J::Null)),
            ("governor".into(), gov.map(J::Str).unwrap_or(J::Null)),
            ("class".into(), J::Str(class.into())),
            ("thread_siblings".into(), sibs.map(J::Str).unwrap_or(J::Null)),
            ("caches".into(), J::Arr(caches)),
        ]));
    }

    // Class counts + measured L1D per class (null when unreadable).
    let mut l1d_by_class: std::collections::HashMap<String, Vec<u64>> =
        std::collections::HashMap::new();
    for cpu in &cpu_arr {
        let class = cpu.get("class").and_then(|v| v.as_str()).unwrap_or("unknown");
        if let Some(arr) = cpu.get("caches").and_then(|v| v.as_arr()) {
            for c in arr {
                let is_l1d = c.get("level").and_then(|v| v.as_str()) == Some("1")
                    && c.get("type").and_then(|v| v.as_str()) == Some("Data");
                if is_l1d {
                    if let Some(kb) = c.get("size_kb").and_then(|v| v.as_f64()) {
                        l1d_by_class.entry(class.to_string()).or_default().push(kb as u64);
                    }
                }
            }
        }
    }
    let mut l1d_measured = Vec::new();
    for (class, mut sizes) in l1d_by_class {
        sizes.sort_unstable();
        sizes.dedup();
        l1d_measured.push(J::Obj(vec![
            ("class".into(), J::Str(class)),
            ("l1d_kb_values".into(), J::Arr(sizes.into_iter().map(|v| J::Num(v as f64)).collect())),
        ]));
    }

    J::Obj(vec![
        ("tool".into(), J::Str("titan-rig probe (Phase 0 S0)".into())),
        (
            "present".into(),
            read_trim("/sys/devices/system/cpu/present").map(J::Str).unwrap_or(J::Null),
        ),
        (
            "possible".into(),
            read_trim("/sys/devices/system/cpu/possible").map(J::Str).unwrap_or(J::Null),
        ),
        ("cpus".into(), J::Arr(cpu_arr)),
        ("smt_detected".into(), J::Bool(smt_found)),
        ("l1d_kb_measured_by_class".into(), J::Arr(l1d_measured)),
        (
            "assumed".into(),
            J::Obj(vec![
                ("note".into(), J::Str("used ONLY where sysfs unreadable; measured fields stay null".into())),
                ("sm4450_l1d_kb".into(), J::Obj(vec![
                    ("A55".into(), J::Num(32.0)),
                    ("A78".into(), J::Num(64.0)),
                ])),
                ("sm4450_l3_shared_kb".into(), J::Num(2048.0)),
            ]),
        ),
    ])
}
