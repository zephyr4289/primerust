//! Pre-Flight Experiment E1: Per-Core Physical Sieve Engine Survey.
//!
//! Evaluates titan-sieve on each of the 8 cores with core-matched segment geometry:
//!   - Cortex-A55 Little Cores (cpu0..cpu5): 32 KiB segment
//!   - Cortex-A76 Big Cores (cpu6..cpu7):   64 KiB segment
//! Evaluates at 10^9 and 10^10 to derive the foundational weight vector.

use std::time::Instant;
use titan_bench::{pin, snapshot};
use titan_sieve::pi_with_segment_size;

fn main() {
    let _wl = snapshot::WakeLock::acquire();
    println!("== PRE-FLIGHT EXPERIMENT E1: PER-CORE ENGINE SURVEY ==");

    let mut rates_1e9 = [0.0f64; 8];
    let mut rates_1e10 = [0.0f64; 8];

    for cpu in 0..8 {
        if let Err(e) = pin::set_affinity(cpu) {
            eprintln!("  Failed to pin to cpu{}: {}", cpu, e);
            continue;
        }

        let is_big = cpu >= 6;
        let seg_sz = if is_big { 65536 } else { 32768 };
        let core_type = if is_big { "Cortex-A76 (Big)  " } else { "Cortex-A55 (Little)" };

        // Test 1: 10^9 (1 Billion)
        let t0 = Instant::now();
        let cnt_1e9 = pi_with_segment_size(1_000_000_000, seg_sz);
        let sec_1e9 = t0.elapsed().as_secs_f64();
        assert_eq!(cnt_1e9, 50_847_534, "Mismatch at 10^9 on cpu{}!", cpu);
        let rate_1e9 = 1_000_000_000.0 / sec_1e9 / 1e9;
        rates_1e9[cpu] = rate_1e9;

        // Test 2: 10^10 (10 Billion)
        let t1 = Instant::now();
        let cnt_1e10 = pi_with_segment_size(10_000_000_000, seg_sz);
        let sec_1e10 = t1.elapsed().as_secs_f64();
        assert_eq!(cnt_1e10, 455_052_511, "Mismatch at 10^10 on cpu{}!", cpu);
        let rate_1e10 = 10_000_000_000.0 / sec_1e10 / 1e9;
        rates_1e10[cpu] = rate_1e10;

        println!(
            "  cpu{} [{}] (seg={:>2}K) | 10^9: {:>5.3}s ({:.3} B/s) | 10^10: {:>6.3}s ({:.3} B/s)",
            cpu, core_type, seg_sz / 1024, sec_1e9, rate_1e9, sec_1e10, rate_1e10
        );
    }

    let _ = pin::set_full_affinity();

    // Calculate cluster averages at 10^10
    let a55_avg_1e10: f64 = rates_1e10[0..6].iter().sum::<f64>() / 6.0;
    let a76_avg_1e10: f64 = rates_1e10[6..8].iter().sum::<f64>() / 2.0;

    let total_capacity = 6.0 * a55_avg_1e10 + 2.0 * a76_avg_1e10;
    let a55_share = (6.0 * a55_avg_1e10) / total_capacity * 100.0;
    let a76_share = (2.0 * a76_avg_1e10) / total_capacity * 100.0;

    println!("\n--- CLUSTER SYNTHESIS (N = 10^10) ---");
    println!("  Cortex-A55 Average (r_55) : {:.3} B/s each ({:.3} B/s total across 6 cores)", a55_avg_1e10, a55_avg_1e10 * 6.0);
    println!("  Cortex-A76 Average (r_76) : {:.3} B/s each ({:.3} B/s total across 2 cores)", a76_avg_1e10, a76_avg_1e10 * 2.0);
    println!("  Raw Sieve Capacity Ratio   : {:.2}x (A76 / A55)", a76_avg_1e10 / a55_avg_1e10);
    println!("  Workload Capacity Split    : {:.1}% Little (6x A55) vs {:.1}% Big (2x A76)", a55_share, a76_share);

    print!("  Normalized Weight Vector   : [");
    for cpu in 0..8 {
        let w = rates_1e10[cpu] / total_capacity;
        print!("{:.3}{}", w, if cpu < 7 { ", " } else { "" });
    }
    println!("]");

    if a55_avg_1e10 >= 0.85 {
        println!("  [GATE OPENER] r_55 = {:.3} B/s >= 0.85 B/s! The gate to 9.15 B/s is OPEN.", a55_avg_1e10);
    } else {
        println!("  [NOTICE] r_55 = {:.3} B/s < 0.85 B/s. Sub-rungs R2/R3/R4 recommended for A55 acceleration.", a55_avg_1e10);
    }
}
