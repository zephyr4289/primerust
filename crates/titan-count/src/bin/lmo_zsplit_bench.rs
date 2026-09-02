//! LMO Z-Split Benchmark

use std::time::Instant;
use titan_count::LmoCounter;

fn main() {
    let scales = [
        (10u32, 455_052_511u64),
        (11, 4_118_054_813),
        (12, 37_607_912_018),
        (13, 346_065_536_839),
        (14, 3_204_941_750_802),
    ];

    println!("=== LMO Z-SPLIT BENCHMARK (8T) ===");
    for &(pow, expected) in &scales {
        let x = 10u64.pow(pow);
        let mut lmo = LmoCounter::new();
        let t = Instant::now();
        let ans = lmo.count_zsplit(x, 8);
        let elapsed = t.elapsed().as_secs_f64();
        let status = if ans == expected { "✓" } else { "✗" };
        println!("  10^{:<2} = {:>14} | {:>6.3}s | {}", pow, ans, elapsed, status);
    }
}