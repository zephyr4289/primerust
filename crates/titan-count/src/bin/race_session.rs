//! Race Session R0: Live Head-to-Head Benchmark on Snapdragon 4 Gen 2.
//!
//! Measures:
//!   - primecount (1T & 8T) vs titan (1T & 8T)
//!   - primesieve (1T & 8T) vs titan-sieve (1T & 8T)
//!   - 8T/1T multi-core scaling efficiency

use std::process::Command;
use std::time::Instant;
use titan_bench::snapshot;
use titan_count::assembly::LehmerCounter;

fn run_cmd_time(cmd: &str, args: &[&str]) -> Option<(f64, String)> {
    let t0 = Instant::now();
    let output = Command::new(cmd).args(args).output().ok()?;
    let dur = t0.elapsed().as_secs_f64();
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Some((dur, stdout))
    } else {
        None
    }
}

fn measure_primecount(x_str: &str, threads: usize) -> Option<f64> {
    let t_str = threads.to_string();
    let (dur, _) = run_cmd_time("primecount", &[x_str, "-t", &t_str, "--time"])?;
    Some(dur)
}

fn measure_primesieve(n_str: &str, threads: usize) -> Option<f64> {
    let t_str = threads.to_string();
    let (dur, _) = run_cmd_time("primesieve", &[n_str, "-t", &t_str, "--time"])?;
    Some(dur)
}

fn main() {
    let _wl = snapshot::WakeLock::acquire();
    println!("=========================================================================================");
    println!("                RACE SESSION R0: TITAN VS OPPONENT ON SNAPDRAGON 4 GEN 2                 ");
    println!("=========================================================================================");

    // 1. Combinatorial Race: Titan Lehmer vs Primecount Gourdon
    println!("\n--- 1. COMBINATORIAL RACE: TITAN LEHMER VS PRIMECOUNT ---");
    println!(" Scale | primecount 1T | primecount 8T | pc 8T/1T | Titan 1T   | Titan 8T   | Ti 8T/1T | Verdict");
    println!("-----------------------------------------------------------------------------------------");

    let scales: &[(u32, u64)] = &[
        (10, 10_000_000_000),
        (11, 100_000_000_000),
        (12, 1_000_000_000_000),
        (13, 10_000_000_000_000),
        (14, 100_000_000_000_000),
    ];

    for &(pow, x) in scales {
        let x_str = format!("1e{}", pow);

        let pc_1t = measure_primecount(&x_str, 1).unwrap_or(0.0);
        let pc_8t = measure_primecount(&x_str, 8).unwrap_or(0.0);
        let pc_scale = if pc_8t > 0.0 { pc_1t / pc_8t } else { 0.0 };

        // Titan 1T
        let t0_1t = Instant::now();
        let mut lehmer_st = LehmerCounter::new();
        let _ = lehmer_st.count(x);
        let ti_1t = t0_1t.elapsed().as_secs_f64();

        // Titan 8T
        let t0_8t = Instant::now();
        let lehmer_mt = LehmerCounter::new();
        let _ = lehmer_mt.count_mt(x, 8);
        let ti_8t = t0_8t.elapsed().as_secs_f64();
        let ti_scale = if ti_8t > 0.0 { ti_1t / ti_8t } else { 0.0 };

        let verdict = if ti_8t < pc_8t {
            format!("TITAN ({:.2}x)", pc_8t / ti_8t)
        } else {
            format!("PC ({:.2}x)", ti_8t / pc_8t)
        };

        println!(
            " 10^{:<2} | {:>11.4}s | {:>11.4}s | {:>7.2}x | {:>8.4}s | {:>8.4}s | {:>7.2}x | {:>12}",
            pow, pc_1t, pc_8t, pc_scale, ti_1t, ti_8t, ti_scale, verdict
        );
    }

    // 2. Physical Sieve Race: Titan Sieve vs Primesieve
    println!("\n--- 2. PHYSICAL SIEVE RACE: TITAN-SIEVE VS PRIMESIEVE ---");
    println!(" Limit | primesieve 1T | primesieve 8T | ps 8T/1T | Titan 1T   | Titan 8T   | Ti 8T/1T | Verdict");
    println!("-----------------------------------------------------------------------------------------");

    let sieve_scales = [1_000_000_000u64, 10_000_000_000u64, 100_000_000_000u64];
    for &n in &sieve_scales {
        let n_str = format!("1e{}", (n as f64).log10().round() as u32);

        let ps_1t = measure_primesieve(&n_str, 1).unwrap_or(0.0);
        let ps_8t = measure_primesieve(&n_str, 8).unwrap_or(0.0);
        let ps_scale = if ps_8t > 0.0 { ps_1t / ps_8t } else { 0.0 };

        // Titan 1T
        let t0_1t = Instant::now();
        let _ = titan_sieve::pi(n);
        let ti_1t = t0_1t.elapsed().as_secs_f64();

        // Titan 8T
        let t0_8t = Instant::now();
        let _ = titan_pool::pi_mt(n);
        let ti_8t = t0_8t.elapsed().as_secs_f64();
        let ti_scale = if ti_8t > 0.0 { ti_1t / ti_8t } else { 0.0 };

        let verdict = if ti_8t < ps_8t {
            format!("TITAN ({:.2}x)", ps_8t / ti_8t)
        } else {
            format!("PS ({:.2}x)", ti_8t / ps_8t)
        };

        println!(
            " {:<5} | {:>11.4}s | {:>11.4}s | {:>7.2}x | {:>8.4}s | {:>8.4}s | {:>7.2}x | {:>12}",
            n_str, ps_1t, ps_8t, ps_scale, ti_1t, ti_8t, ti_scale, verdict
        );
    }
    println!("=========================================================================================");
}
