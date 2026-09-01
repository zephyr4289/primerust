//! Pre-Flight Experiment E2: Cool All-Core Burst & Contention Factor.
//!
//! Measures titan-pool across all 8 cores on 10^10 (min of 5 runs).
//! Calculates the real-engine contention efficiency vs E1's 9.275 B/s sum.

use std::time::Instant;
use titan_bench::snapshot;
use titan_pool::pi_mt_full;

fn main() {
    let _wl = snapshot::WakeLock::acquire();
    println!("== PRE-FLIGHT EXPERIMENT E2: COOL ALL-CORE BURST ==");

    const N: u64 = 10_000_000_000;
    let mut burst_rates = Vec::new();

    for i in 1..=5 {
        let t0 = Instant::now();
        let (count, telemetries) = pi_mt_full(N, 8, 96);
        let elapsed = t0.elapsed().as_secs_f64();

        assert_eq!(count, 455_052_511, "Wrong count at 10^10!");
        let rate = N as f64 / elapsed / 1e9;
        println!("  Run {}: Wall={:>6.3}s | Aggregate Throughput = {:>6.3} Billion n/s", i, elapsed, rate);

        burst_rates.push((elapsed, rate, telemetries));
    }

    burst_rates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    let best_rate = burst_rates[0].1;
    let best_time = burst_rates[0].0;

    println!("\n--- E2 SYNTHESIS ---");
    println!("  Peak 8-Core Burst Rate : {:.3} B/s in {:.3}s", best_rate, best_time);

    // Theoretical sum from E1 was 9.275 B/s
    const E1_THEORETICAL_SUM: f64 = 9.275;
    let contention_eff = (best_rate / E1_THEORETICAL_SUM) * 100.0;
    println!("  E1 Uncontended Sum     : {:.3} B/s", E1_THEORETICAL_SUM);
    println!("  Real Engine Contention : {:.1}% retained efficiency", contention_eff);

    if best_rate >= 9.15 {
        println!("  [TARGET ACHIEVED] Peak rate {:.3} B/s >= 9.15 B/s! The phone's primesieve record is BROKEN!", best_rate);
    } else {
        println!("  [PROGRESS] Peak rate {:.3} B/s ({:.1}% of 9.15 B/s target)", best_rate, best_rate / 9.15 * 100.0);
    }

    // Telemetry per worker
    println!("\n--- PER-WORKER LOAD DISTRIBUTION ---");
    let best_telemetries = &burst_rates[0].2;
    for (id, t) in best_telemetries.iter().enumerate() {
        let (cpu, units, primes, time_ns) = t.snapshot();
        let is_big = cpu >= 6;
        println!(
            "  Worker {:<2} [cpu{}] ({}) : {:>3} units | {:>9} primes | {:>6.3}s active",
            id, cpu, if is_big { "A76 Big   " } else { "A55 Little" }, units, primes, (time_ns as f64) / 1e9
        );
    }
}
