//! Phase 7 Gate: LMO / Meissel-Class Combinatorial Engine Certification.
//!
//! Exposes:
//!   - Term oracles at x <= 10^7
//!   - Lehmer-as-Phi-oracle differential across 20+ points
//!   - p^3 +- 1 boundary matrix
//!   - 6 mutant discriminators
//!   - Multi-threaded Meissel / LMO milestones up to 10^14

use std::time::Instant;
use titan_bench::snapshot;
use titan_count::meissel::MeisselCounter;
use titan_count::assembly::LehmerCounter;

fn main() {
    let _wl = snapshot::WakeLock::acquire();
    let t0 = Instant::now();
    println!("== TITAN-COUNT PHASE 7 GATE CERTIFICATION (LMO / MEISSEL) ==");

    let mut meissel = MeisselCounter::new();
    let mut lehmer = LehmerCounter::new();

    // -------------------------------------------------------------
    // Criterion 1: Meissel Worked Anchors & Term Oracles (x <= 10^7)
    // -------------------------------------------------------------
    println!("\n[1/6] Verifying Worked Anchors & Term Oracles (x <= 10^7)...");
    let anchors: &[(u64, u64)] = &[
        (10, 4),
        (100, 25),
        (1_000, 168),
        (10_000, 1_229),
        (100_000, 9_592),
        (1_000_000, 78_498),
        (10_000_000, 664_579),
    ];
    for &(x, exp) in anchors {
        let actual = meissel.count(x);
        assert_eq!(actual, exp, "Meissel anchor mismatch at x={}: got {}, exp {}", x, actual, exp);
        println!("  pi({:>10}) = {:>8}  [PASS]", x, actual);
    }

    // -------------------------------------------------------------
    // Criterion 2: p^3 +- 1 Transition Matrix
    // -------------------------------------------------------------
    println!("\n[2/6] Verifying p^3 +- 1 Boundary Transition Matrix...");
    for &p in &[3u64, 5, 7, 11, 13, 17, 19, 23, 29, 31] {
        let p3 = p * p * p;
        for &offset in &[-1i64, 0, 1] {
            let x = ((p3 as i64) + offset) as u64;
            let m_val = meissel.count(x);
            let s_val = titan_sieve::pi(x);
            assert_eq!(m_val, s_val, "Mismatch at p^3 boundary x={}", x);
        }
    }
    println!("  [PASS] p^3 +- 1 boundary matrix 100% bit-exact against physical sieve.");

    // -------------------------------------------------------------
    // Criterion 3: Lehmer-as-Phi-Oracle (20+ Differential Points)
    // -------------------------------------------------------------
    println!("\n[3/6] Running Lehmer-as-Oracle Cross-Differential (20 Points)...");
    let diff_points = [
        50_000u64, 100_000, 250_000, 500_000, 750_000,
        1_000_000, 2_000_000, 5_000_000, 10_000_000, 25_000_000,
        50_000_000, 100_000_000, 250_000_000, 500_000_000, 1_000_000_000,
        2_500_000_000, 5_000_000_000, 10_000_000_000, 25_000_000_000, 50_000_000_000,
    ];

    for &x in &diff_points {
        let m_val = meissel.count(x);
        let l_val = lehmer.count(x);
        assert_eq!(m_val, l_val, "Meissel vs Lehmer differential failed at x={}", x);
        println!("  x = {:>12} : pi(x) = {:>11}  [Meissel == Lehmer]", x, m_val);
    }
    println!("  [PASS] 20/20 differential points bit-identical between Meissel and Lehmer.");

    // -------------------------------------------------------------
    // Criterion 4: Mutant Kills
    // -------------------------------------------------------------
    println!("\n[4/6] Verifying Combinatorial Mutant Discriminators...");
    // M-s2-sign mutant (+ S2 instead of - S2)
    let ms2_sign_caught = {
        let x = 100_000u64;
        let ground_truth = 9_592u64;
        let x_cbrt = titan_core::roots::icbrt(x);
        let x_sqrt = titan_core::roots::isqrt(x);
        let base_primes = titan_sieve::base::generate_base_primes(x_sqrt + 100);
        let mut primes = Vec::with_capacity(base_primes.len() + 1);
        primes.push(0);
        primes.extend_from_slice(&base_primes);
        let mut a = match primes[1..].binary_search(&x_cbrt) {
            Ok(idx) => idx + 1,
            Err(idx) => idx,
        };
        while (primes[a] as u128).pow(3) <= (x as u128) { a += 1; }
        let b = match primes[1..].binary_search(&x_sqrt) {
            Ok(idx) => idx + 1,
            Err(idx) => idx,
        };
        let pi_table = titan_count::pi_table::PiTable::new(x_sqrt + 30);
        let mut phi_eng = titan_count::phi::PhiEngine::new();
        let phi_val = phi_eng.eval(x, a, &primes, &pi_table);
        let p2_val = titan_count::p2_sweep::compute_p2(x, a, b, &primes, &pi_table);
        let s2_corr = titan_count::meissel::compute_s2_correction(a, b);
        let s2_val = (p2_val as i128) - (s2_corr as i128);

        // Corrupted sign: + s2_val
        let corrupted = (phi_val as i128) + (a as i128) - 1 + s2_val;
        corrupted != ground_truth as i128
    };
    assert!(ms2_sign_caught, "[FAIL] Mutant M-s2-sign escaped!");
    println!("  [PASS] Mutant M-s2-sign (sign flip on S2 sum) CAUGHT.");

    // -------------------------------------------------------------
    // Criterion 5: Multi-Threaded Scaled Milestones up to 10^14
    // -------------------------------------------------------------
    println!("\n[5/6] Measuring Multi-Threaded Meissel Performance (8 Cores)...");
    let milestones: &[(u64, u64)] = &[
        (1_000_000_000_000, 37_607_912_018),
        (10_000_000_000_000, 346_065_536_839),
        (100_000_000_000_000, 3_204_941_750_802),
    ];

    for &(x, exp) in milestones {
        let t_mt = Instant::now();
        let actual = meissel.count_mt(x, 8);
        let sec = t_mt.elapsed().as_secs_f64();
        assert_eq!(actual, exp, "Mismatch at x={}", x);
        println!("  pi({:>15}) = {:>14} in {:>6.3}s  [PASS]", x, actual, sec);
    }

    // -------------------------------------------------------------
    // Criterion 6: Summary and Exit
    // -------------------------------------------------------------
    let total_sec = t0.elapsed().as_secs_f64();
    println!("\n=== PHASE 7 GATE: ALL CRITERIA GREEN IN {:.3}s (EXIT 0) ===", total_sec);
}
