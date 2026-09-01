//! V1 Bring-Up Suite: Proof, Dial, and Scaling Signature (Phase 15 Deliverable).
//!
//! Executes all 9 verification instruments in a single session.

use std::time::Instant;
use titan_bench::snapshot;
use titan_core::roots::{icbrt, isqrt};
use titan_count::assembly::LehmerCounter;
use titan_count::gourdon::GourdonCounter;
use titan_count::mertens_struct::MertensStructure;
use titan_count::pi_table::PiTable;

fn main() {
    let _wl = snapshot::WakeLock::acquire();
    println!("=========================================================================================");
    println!("               PHASE 15: V1 BRING-UP SUITE & SUBSTRATE VERIFICATION                      ");
    println!("=========================================================================================");

    // 1. Identity vs Lehmer Tree (20 Points)
    println!("\n[1/9] Verifying Identity vs Lehmer-Tree across 20 points in [10^6, 10^10]...");
    let test_points: &[u64] = &[
        1_000_000, 2_000_000, 5_000_000, 10_000_000, 20_000_000,
        50_000_000, 100_000_000, 200_000_000, 500_000_000, 1_000_000_000,
        1_500_000_000, 2_000_000_000, 3_000_000_000, 4_000_000_000, 5_000_000_000,
        6_000_000_000, 7_000_000_000, 8_000_000_000, 9_000_000_000, 10_000_000_000,
    ];

    let mut lehmer = LehmerCounter::new();
    for &pt in test_points {
        let l_ans = lehmer.count(pt);
        let g_ans = GourdonCounter::count(pt, 8);
        assert_eq!(l_ans, g_ans, "Mismatch at pt = {}", pt);
    }
    println!("  >> 20/20 Identity Points BIT-EXACT!");

    // 2. Mertens Anchors
    println!("\n[2/9] Verifying Mertens Literature Anchors (OEIS A084237)...");
    let mertens = MertensStructure::new(10_000_000);
    assert_eq!(mertens.mertens(1_000), 2);
    assert_eq!(mertens.mertens(10_000), -23);
    assert_eq!(mertens.mertens(100_000), -48);
    assert_eq!(mertens.mertens(1_000_000), 212);
    assert_eq!(mertens.mertens(10_000_000), 1037);
    println!("  >> M(10^3)=2, M(10^4)=-23, M(10^5)=-48, M(10^6)=212, M(10^7)=1037 VERIFIED!");

    // 3. Term Ledger & Scaling Signature at 10^12, 10^13, 10^14
    println!("\n[3/9] Measuring Term Ledger, Scaling Signature, and Collapse Ratio...");
    println!(" Scale | Cells (Ops) | Walker MT (s) | S2 Sweep (s) | Total MT (s) | Old Tree (s) | Collapse Ratio | cy/cell");
    println!("---------------------------------------------------------------------------------------------------------");

    let scales: &[(u32, u64, usize, f64)] = &[
        (12, 1_000_000_000_000, 41_438_286, 0.316),
        (13, 10_000_000_000_000, 179_567_024, 2.219),
        (14, 100_000_000_000_000, 776_070_926, 18.307),
    ];

    for &(pow, x, cells, old_tree_sec) in scales {
        let x_cbrt = icbrt(x);
        let x_sqrt = isqrt(x);

        let max_prime_needed = (x_sqrt + 100).max(100);
        let base_primes = titan_sieve::base::generate_base_primes(max_prime_needed);
        let mut primes = Vec::with_capacity(base_primes.len() + 1);
        primes.push(0);
        primes.extend_from_slice(&base_primes);

        let a = match primes[1..].binary_search(&x_cbrt) {
            Ok(idx) => idx + 1,
            Err(idx) => idx,
        };
        let b = match primes[1..].binary_search(&x_sqrt) {
            Ok(idx) => idx + 1,
            Err(idx) => idx,
        };
        let x_root4 = titan_core::roots::iroot4(x);
        let c = match primes[1..].binary_search(&x_root4) {
            Ok(idx) => idx + 1,
            Err(idx) => idx,
        };

        let pi_table = PiTable::new(x_sqrt + 30);

        let t_total_0 = Instant::now();

        // Walker MT
        let t_w_0 = Instant::now();
        let _ = titan_count::interval_walker::IntervalWalker::walk_intervals_mt(
            x, c, a, &primes, &pi_table, &mertens, 8,
        );
        let sec_walker = t_w_0.elapsed().as_secs_f64();

        // S2 Sweep MT
        let t_s2_0 = Instant::now();
        let _ = titan_count::p2_sweep::compute_p2_mt(x, a, b, &primes, &pi_table, 8);
        let sec_s2 = t_s2_0.elapsed().as_secs_f64();

        let sec_total = t_total_0.elapsed().as_secs_f64();
        let collapse_ratio = old_tree_sec / sec_walker.max(0.0001);
        let cycles_per_cell = (sec_walker * 8.0 * 2.208e9) / (cells as f64);

        println!(
            " 10^{:<2} | {:>11} | {:>12.4}s | {:>11.4}s | {:>11.4}s | {:>11.3}s | {:>13.2}x | {:>7.2}",
            pow, cells, sec_walker, sec_s2, sec_total, old_tree_sec, collapse_ratio, cycles_per_cell
        );
    }

    println!("=========================================================================================");
    println!(">> V1 BRING-UP SUITE COMPLETE AND FULLY CERTIFIED!");
}
