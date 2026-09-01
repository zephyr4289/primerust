//! Honest, canary-normalized external binary benchmark runner.
//! Usage:
//!   benchref --cmd "..." --work N [--expect N] [--runs K] [--duration S] [--canary s|m|l]

use std::process::Command;
use std::time::{Duration, Instant};
use titan_bench::{canary, pin, snapshot};

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

fn get_str_arg(args: &[String], flag: &str) -> Option<String> {
    for (i, arg) in args.iter().enumerate() {
        if arg == flag {
            if let Some(next) = args.get(i + 1) {
                return Some(next.clone());
            }
        } else if let Some(stripped) = arg.strip_prefix(flag) {
            if let Some(val_str) = stripped.strip_prefix('=') {
                return Some(val_str.to_string());
            }
        }
    }
    None
}

fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|a| a == flag || a.starts_with(&format!("{flag}=")))
}

fn parse_first_integer(output: &str) -> Option<u64> {
    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Primes:") {
            if let Some(num_str) = trimmed.strip_prefix("Primes:") {
                if let Ok(v) = num_str.trim().parse::<u64>() {
                    return Some(v);
                }
            }
        }
    }
    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Sieve size") || trimmed.starts_with("Threads") || trimmed.ends_with('%') {
            continue;
        }
        for token in trimmed.split_whitespace() {
            let digits: String = token.chars().take_while(|c| c.is_ascii_digit()).collect();
            if !digits.is_empty() {
                if let Ok(val) = digits.parse::<u64>() {
                    return Some(val);
                }
            }
        }
    }
    None
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cmd_str = match get_str_arg(&args, "--cmd") {
        Some(s) => s,
        None => {
            eprintln!("Usage: benchref --cmd \"binary arg1 arg2\" --work N [--expect N] [--runs K] [--duration S] [--canary s|m|l]");
            std::process::exit(1);
        }
    };
    let work: u64 = get_arg_value(&args, "--work", 0u64);
    let expected: Option<u64> = get_str_arg(&args, "--expect").and_then(|s| s.parse().ok());
    let runs_count: Option<u32> = if has_flag(&args, "--runs") {
        Some(get_arg_value(&args, "--runs", 1u32))
    } else {
        None
    };
    let duration_secs: Option<u64> = if has_flag(&args, "--duration") {
        Some(get_arg_value(&args, "--duration", 10u64))
    } else {
        None
    };
    let canary_type = get_str_arg(&args, "--canary").unwrap_or_else(|| "m".into());

    let epochs = match canary_type.as_str() {
        "s" => 1000,
        "l" => 20000,
        _ => canary::CANARY_M,
    };

    let _wl = snapshot::WakeLock::acquire();
    let (aborts, warns) = snapshot::hygiene();
    for w in warns {
        eprintln!("[hygiene warn] {w}");
    }
    if !aborts.is_empty() && !has_flag(&args, "--force") {
        eprintln!("[hygiene ABORT] {:?}", aborts);
        std::process::exit(1);
    }

    let canary_probe = canary::CpuCanary::with_epochs(epochs);

    // Pick fastest big core for canary sampling
    let (canary_cpu, cold_baseline) = {
        let cores = titan_bench::topology::read();
        let mut best_cpu = 0;
        let mut best_rate = 0.0;
        for c in &cores {
            if pin::set_affinity(c.cpu).is_ok() {
                let r = canary_probe.rate(1, 3);
                if r > best_rate {
                    best_rate = r;
                    best_cpu = c.cpu;
                }
            }
        }
        let _ = pin::set_full_affinity();
        (best_cpu, best_rate)
    };

    println!("== TITAN BENCHREF ==");
    println!("  Command: {}", cmd_str);
    println!("  Work Units: {}", work);
    println!("  Canary Core: cpu{} (Cold Baseline: {:.1} ep/s)", canary_cpu, cold_baseline);

    let mut run_results = Vec::new();
    let start_all = Instant::now();
    let mut iter = 0u32;

    loop {
        iter += 1;
        if let Some(r) = runs_count {
            if iter > r {
                break;
            }
        }
        if let Some(d) = duration_secs {
            if start_all.elapsed() >= Duration::from_secs(d) && iter > 1 {
                break;
            }
        }
        if runs_count.is_none() && duration_secs.is_none() && iter > 1 {
            break;
        }

        // 1. Pre-canary sample
        let _ = pin::set_affinity(canary_cpu);
        let (pre_rate, _) = canary_probe.sample_once();

        // 2. CRITICAL PROTOCOL: Restore full affinity before spawning child!
        let _ = pin::set_full_affinity();

        let t_child_0 = Instant::now();
        let output = Command::new("sh")
            .arg("-c")
            .arg(&cmd_str)
            .output()
            .expect("failed to execute child command");
        let child_duration = t_child_0.elapsed();

        // 3. Post-canary sample
        let _ = pin::set_affinity(canary_cpu);
        let (post_rate, _) = canary_probe.sample_once();
        let _ = pin::set_full_affinity();

        let stdout_str = String::from_utf8_lossy(&output.stdout).to_string();
        let parsed_val = parse_first_integer(&stdout_str);

        if let Some(exp) = expected {
            if parsed_val != Some(exp) {
                eprintln!("[ERROR] Output verification failed! Expected {}, got {:?}", exp, parsed_val);
                eprintln!("Stdout: {}", stdout_str);
                std::process::exit(1);
            }
        }

        let mean_canary = 0.5 * (pre_rate + post_rate);
        let derate = mean_canary / cold_baseline.max(1.0);
        let raw_rate = if child_duration.as_secs_f64() > 0.0 {
            work as f64 / child_duration.as_secs_f64()
        } else {
            0.0
        };
        let norm_rate = raw_rate / derate.max(0.01);

        println!(
            "  Run {:>2}: Wall={:>7.3}s | RawRate={:>7.2}M n/s | Derate={:.3} | NormRate={:>7.2}M n/s | Output={:?}",
            iter,
            child_duration.as_secs_f64(),
            raw_rate / 1e6,
            derate,
            norm_rate / 1e6,
            parsed_val
        );

        run_results.push((child_duration.as_secs_f64(), raw_rate, derate, norm_rate));
    }

    let median_raw = {
        let mut v: Vec<f64> = run_results.iter().map(|r| r.1).collect();
        v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        v[v.len() / 2]
    };
    let median_norm = {
        let mut v: Vec<f64> = run_results.iter().map(|r| r.3).collect();
        v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        v[v.len() / 2]
    };

    println!("\n=== BENCHREF SUMMARY ===");
    println!("  Total Runs       : {}", run_results.len());
    println!("  Median Raw Rate  : {:.3} Million numbers/sec", median_raw / 1e6);
    println!("  Median Norm Rate : {:.3} Million numbers/sec (Cool-Clock Equivalent)", median_norm / 1e6);

    let env = snapshot::snapshot();
    let record_json = format!(
        r#"{{"cmd":"{}","work":{},"expected":{:?},"total_runs":{},"median_raw_rate":{:.2},"median_norm_rate":{:.2},"runs":[{}]}}"#,
        snapshot::json_esc(&cmd_str),
        work,
        expected,
        run_results.len(),
        median_raw,
        median_norm,
        run_results.iter()
            .map(|r| format!(r#"{{"wall_sec":{:.4},"raw_rate":{:.2},"derate":{:.4},"norm_rate":{:.2}}}"#, r.0, r.1, r.2, r.3))
            .collect::<Vec<_>>()
            .join(",")
    );
    let _ = snapshot::write_record("benchref", &record_json);
}
