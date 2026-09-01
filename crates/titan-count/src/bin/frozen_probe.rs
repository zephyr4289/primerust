//! Frozen Probe Benchmark at 10^14 (Phase 1.29 Production Deliverable).
//!
//! Executes multi-iteration frozen probe on Snapdragon 4 Gen 2 on 8 threads:
//!   - Evaluates pi(10^14) = 3,204,941,750,802 bit-exact
//!   - Computes median, p25, p75 wall-clock times across repetitions
//!   - Emits standardized machine certification receipt

use std::time::Instant;
use titan_count::gourdon::GourdonCounter;
use titan_count::scale_dispatch::ScaleDispatch;

fn main() {
    println!("=========================================================================================");
    println!("             PHASE 1.29: FROZEN PROBE CERTIFICATION AT 10^14 (8 THREADS)                ");
    println!("=========================================================================================");

    let x = 100_000_000_000_000u64; // 10^14
    let expected = 3_204_941_750_802u64;
    let threads = 8;
    let runs = 3;

    let dial = ScaleDispatch::select(x, threads);

    println!(" Scale                       : 10^14 (100,000,000,000,000)");
    println!(" Expected pi(10^14)          : 3,204,941,750,802 (OEIS A006880)");
    println!(" Silicon                     : Snapdragon 4 Gen 2 (SM4450)");
    println!(" Dial Configuration          : alpha_y = {:.3}, beta = {:.1}", dial.alpha_y, dial.beta);
    println!(" Thread Topology             : 8 Threads");
    println!("-----------------------------------------------------------------------------------------");

    let mut times = Vec::with_capacity(runs);
    let mut last_tag = "arena25/C[AB-VERIFIED]";
    let mut last_cells = 776_070_926;
    let mut last_blocks = 32_723;

    for r in 1..=runs {
        let t0 = Instant::now();
        let (computed, tag, cells, blocks) = GourdonCounter::eval_mt(x, threads, true);
        let elapsed = t0.elapsed().as_secs_f64();
        times.push(elapsed);
        last_tag = tag;
        last_cells = cells;
        last_blocks = blocks;

        let status = if computed == expected { "BIT-EXACT MATCH (PASS)" } else { "FAIL" };
        println!(" Run #{:<2}                     : {:>10.4}s | pi(10^14) = {} | {}", r, elapsed, computed, status);
        assert_eq!(computed, expected, "Mismatch on run {}", r);
    }

    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = times[times.len() / 2];
    let p25 = times[0];
    let p75 = times[times.len() - 1];

    let pass_status = if median <= 60.0 { "PASS" } else { "FAIL" };

    println!("-----------------------------------------------------------------------------------------");
    println!(
        "FROZEN-PROBE 1e14 8T n={} median={:.3}s p25={:.3}s p75={:.3}s d_path={} dial=({:.3},{:.1}) cells={} blocks={} -> {}",
        runs, median, p25, p75, last_tag, dial.alpha_y, dial.beta, last_cells, last_blocks, pass_status
    );
    println!("=========================================================================================");
}
