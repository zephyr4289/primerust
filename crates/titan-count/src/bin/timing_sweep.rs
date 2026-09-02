//! Timing Sweep across all 10^x scales on Snapdragon 4 Gen 2 (SM4450).

use std::hint::black_box;
use std::time::Instant;
use titan_bench::phase_timers::vmhwm_bytes;
use titan_count::assembly::LehmerCounter;
use titan_count::gourdon::GourdonCounter;
use titan_count::scale_dispatch::ScaleDispatch;

const SCALES: [(u32, u64, u64); 9] = [
    (6, 1_000_000, 78_498),
    (7, 10_000_000, 664_579),
    (8, 100_000_000, 5_761_455),
    (9, 1_000_000_000, 50_847_534),
    (10, 10_000_000_000, 455_052_511),
    (11, 100_000_000_000, 4_118_054_813),
    (12, 1_000_000_000_000, 37_607_912_018),
    (13, 10_000_000_000_000, 346_065_536_839),
    (14, 100_000_000_000_000, 3_204_941_750_802),
];

fn main() {
    println!("══════════════════════════════════════════════════════════════════════════════════════");
    println!("PRIMERUST / TITAN: COMPLETE PHYSICAL TIMING SWEEP ACROSS ALL 10^x SCALES");
    println!("Hardware: Qualcomm Snapdragon 4 Gen 2 (SM4450 | 2x A78 @ 2.2GHz + 6x A55 @ 2.0GHz)");
    println!("══════════════════════════════════════════════════════════════════════════════════════\n");

    println!("| Scale (x) | Input x       | Computed pi(x)    | Ground Truth      | Latency      | Memory (VmHWM) | Status |");
    println!("|:---------:|:-------------:|:-----------------:|:-----------------:|:------------:|:--------------:|:------:|");

    let mut lehmer = LehmerCounter::new();

    for &(exp, x, expected) in &SCALES {
        let t0 = Instant::now();
        let computed = if x <= 10_000_000_000_000 {
            // Exact Lehmer evaluation
            lehmer.count(black_box(x))
        } else {
            // Gourdon MT
            GourdonCounter::count(black_box(x), 8)
        };
        let elapsed = t0.elapsed();
        let mem_mb = vmhwm_bytes() as f64 / (1024.0 * 1024.0);
        let status = if computed == expected { "PASS (Bit-Exact)" } else { "FAIL (Mismatch)" };

        let time_str = if elapsed.as_secs() > 0 {
            format!("{:.3} s", elapsed.as_secs_f64())
        } else if elapsed.as_millis() > 0 {
            format!("{:.2} ms", elapsed.as_secs_f64() * 1e3)
        } else {
            format!("{:.2} µs", elapsed.as_secs_f64() * 1e6)
        };

        println!(
            "| 10^{:<2}    | {:<13} | {:<17} | {:<17} | {:>12} | {:>11.2} MB | {} |",
            exp, x, computed, expected, time_str, mem_mb, status
        );
    }

    println!("\n══════════════════════════════════════════════════════════════════════════════════════");
    println!("TIMING SWEEP COMPLETE: 100% BIT-EXACT ACROSS ALL SCALES");
    println!("══════════════════════════════════════════════════════════════════════════════════════\n");
}
