//! Frozen Probe Benchmark at 10^14 (Phase 1.28 Deliverable).
//!
//! Executes multi-iteration frozen probe on Snapdragon 4 Gen 2 on 8 threads:
//!   - Evaluates pi(10^14) = 3,204,941,750,802 bit-exact
//!   - Computes median wall-clock time across repetitions
//!   - Emits official production certification receipt

use std::time::Instant;
use titan_count::gourdon::GourdonCounter;

fn main() {
    println!("=========================================================================================");
    println!("             PHASE 1.28: FROZEN PROBE CERTIFICATION AT 10^14 (8 THREADS)                ");
    println!("=========================================================================================");

    let x = 100_000_000_000_000u64; // 10^14
    let expected = 3_204_941_750_802u64;
    let threads = 8;
    let runs = 3;

    println!(" Scale                       : 10^14 (100,000,000,000,000)");
    println!(" Expected pi(10^14)          : 3,204,941,750,802 (OEIS A006880)");
    println!(" Silicon                     : Snapdragon 4 Gen 2 (SM4450: 2x Cortex-A78 + 6x Cortex-A55)");
    println!(" Thread Topology             : 8 Threads (Heterogeneous Work-Stealing Pool)");
    println!("-----------------------------------------------------------------------------------------");

    let mut times = Vec::with_capacity(runs);

    for r in 1..=runs {
        let t0 = Instant::now();
        let computed = GourdonCounter::count(x, threads);
        let elapsed = t0.elapsed().as_secs_f64();
        times.push(elapsed);

        let status = if computed == expected { "BIT-EXACT MATCH (PASS)" } else { "FAIL" };
        println!(" Run #{:<2}                     : {:>10.4}s | pi(10^14) = {} | {}", r, elapsed, computed, status);
        assert_eq!(computed, expected, "Mismatch on run {}", r);
    }

    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median_time = times[times.len() / 2];
    let min_time = times[0];

    println!("-----------------------------------------------------------------------------------------");
    println!(">> PROBE STATISTICS:");
    println!("   Best Run Time             : {:.4}s", min_time);
    println!("   Median Run Time           : {:.4}s", median_time);
    println!("   Status                    : 100% BIT-EXACT PASS");
    println!("=========================================================================================");
}
