//! Phase 2 Single-Threaded Benchmark Runner.
//!
//! Evaluates burst (10^10, min-of-5) and sustained (10^11) throughput
//! pinned to the fastest big core with canary normalization.

use std::time::Instant;
use titan_bench::{canary, pin, snapshot};
use titan_sieve::pi;

fn main() {
    let _wl = snapshot::WakeLock::acquire();
    println!("== TITAN-SIEVE SINGLE-THREAD BENCHMARK ==");

    let can = canary::CpuCanary::with_epochs(canary::CANARY_M);

    // Pin to Cortex-A76 Big Core (cpu6)
    let best_cpu = 6usize;
    let _ = pin::set_affinity(best_cpu);
    let cold_baseline = can.rate(1, 5);

    println!("  Pinned Core: cpu{} (Cold Baseline: {:.1} ep/s)", best_cpu, cold_baseline);
    let _ = pin::set_affinity(best_cpu);

    // -------------------------------------------------------------
    // Part A: Burst Benchmark at 10^10 (Min of 5 runs, within 14.5s cliff)
    // -------------------------------------------------------------
    println!("\n[1/2] Measuring Burst Throughput at 10^10 (Expected pi: 455,052,511)...");
    let mut burst_rates = Vec::new();

    for i in 1..=5 {
        let (pre_can, _) = can.sample_once();
        let t0 = Instant::now();
        let count = pi(10_000_000_000);
        let elapsed = t0.elapsed().as_secs_f64();
        let (post_can, _) = can.sample_once();

        assert_eq!(count, 455_052_511, "Wrong count at 10^10!");

        let mean_can = 0.5 * (pre_can + post_can);
        let derate = mean_can / cold_baseline.max(1.0);
        let raw_rate = 10_000_000_000.0 / elapsed / 1e9;
        let norm_rate = raw_rate / derate.max(0.01);

        println!(
            "  Run {}: Wall={:>6.3}s | Raw={:>6.3} B/s | Derate={:.3} | Norm={:>6.3} B/s",
            i, elapsed, raw_rate, derate, norm_rate
        );
        burst_rates.push((elapsed, raw_rate, norm_rate));
    }

    burst_rates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    let best_burst = burst_rates[0].1;
    println!("  Peak Burst Rate: {:.3} Billion numbers/sec", best_burst);
    if best_burst >= 1.5 {
        println!("  [PASS] Burst rate >= 1.5 B/s target achieved! ({}% of target)", (best_burst / 1.5 * 100.0) as u32);
    } else {
        println!("  [WARN] Burst rate {:.3} B/s below 1.5 B/s target", best_burst);
    }

    // -------------------------------------------------------------
    // Part B: Sustained Benchmark at 10^11 (Canary normalized)
    // -------------------------------------------------------------
    println!("\n[2/2] Measuring Sustained Throughput at 10^11 (Expected pi: 4,118,054,813)...");
    let (pre_can, _) = can.sample_once();
    let t0 = Instant::now();
    let count = pi(100_000_000_000);
    let elapsed = t0.elapsed().as_secs_f64();
    let (post_can, _) = can.sample_once();

    assert_eq!(count, 4_118_054_813, "Wrong count at 10^11!");

    let mean_can = 0.5 * (pre_can + post_can);
    let derate = mean_can / cold_baseline.max(1.0);
    let raw_rate = 100_000_000_000.0 / elapsed / 1e9;
    let norm_rate = raw_rate / derate.max(0.01);

    println!(
        "  Sustained: Wall={:>6.3}s | Raw={:>6.3} B/s | Derate={:.3} | Norm={:>6.3} B/s",
        elapsed, raw_rate, derate, norm_rate
    );
    if norm_rate >= 1.5 {
        println!("  [PASS] Sustained rate >= 1.5 B/s target achieved! ({}% of target)", (norm_rate / 1.5 * 100.0) as u32);
    }

    let _ = pin::set_full_affinity();
}
