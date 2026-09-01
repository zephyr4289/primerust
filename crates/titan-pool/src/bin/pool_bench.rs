//! Phase 3 Multi-Threaded Benchmark Runner.
//!
//! Evaluates scaling across k = 1, 2, 4, 6, 8 workers at 10^10,
//! and runs sustained 10^11 benchmark with per-worker telemetry.

use std::time::Instant;
use titan_bench::snapshot;
use titan_pool::pi_mt_full;

fn main() {
    let _wl = snapshot::WakeLock::acquire();
    println!("== TITAN-POOL MULTI-CORE BENCHMARK ==");

    const N: u64 = 10_000_000_000;

    println!("\n[1/2] Worker Scaling Sweep at 10^10 (Expected: 455,052,511)...");
    let mut baseline_1t = 0.0f64;

    for &k in &[1, 2, 4, 6, 8] {
        let t0 = Instant::now();
        let (cnt, _) = pi_mt_full(N, k, 64);
        let sec = t0.elapsed().as_secs_f64();
        assert_eq!(cnt, 455_052_511);
        let rate = (N as f64) / sec / 1e9;

        if k == 1 {
            baseline_1t = rate;
            println!("  k = {:<1} workers: Wall={:>6.3}s | Rate={:>6.3} B/s | Speedup=1.00x", k, sec, rate);
        } else {
            let speedup = rate / baseline_1t;
            println!("  k = {:<1} workers: Wall={:>6.3}s | Rate={:>6.3} B/s | Speedup={:.2}x", k, sec, rate, speedup);
        }
    }

    println!("\n[2/2] Sustained 10^11 Run with 8 Workers...");
    let t_1e11 = Instant::now();
    let (cnt_1e11, telemetries) = pi_mt_full(100_000_000_000, 8, 128);
    let sec_1e11 = t_1e11.elapsed().as_secs_f64();
    assert_eq!(cnt_1e11, 4_118_054_813);
    let rate_1e11 = 100_000_000_000.0 / sec_1e11 / 1e9;

    println!("  8-Core Sustained 10^11: Wall={:>6.3}s | Rate={:>6.3} B/s", sec_1e11, rate_1e11);

    println!("\n--- WORKER TELEMETRY LEDGER (10^11) ---");
    for (id, t) in telemetries.iter().enumerate() {
        let (cpu, units, primes, time_ns) = t.snapshot();
        let is_big = cpu >= 6;
        println!(
            "  Worker {:<2} [cpu{}] ({}) : {:>3} units | {:>10} primes | {:>6.3}s active",
            id, cpu, if is_big { "A76 Big   " } else { "A55 Little" }, units, primes, (time_ns as f64) / 1e9
        );
    }
}
