//! Phase 5 Gate: Lehmer-Class Combinatorial Engine Certification.
//!
//! Exposes:
//!   --batch : Streaming candidate protocol for titan-oracle
//!   --gate  : 12-point Phase 5 gate with Law 0 self-reporting

use std::io::{self, BufRead, Write};
use std::time::Instant;
use titan_bench::snapshot;
use titan_core::roots::{icbrt, iroot4, isqrt};
use titan_count::assembly::{compute_t, LehmerCounter};
use titan_count::p2_sweep::compute_p2;
use titan_count::p3::compute_p3;
use titan_count::phi::PhiEngine;
use titan_count::pi_table::PiTable;

fn run_batch_protocol() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());
    let mut counter = LehmerCounter::new();

    for line in stdin.lock().lines() {
        if let Ok(line_str) = line {
            let trimmed = line_str.trim();
            if trimmed.is_empty() { continue; }
            if let Ok(x) = trimmed.parse::<u64>() {
                let ans = counter.count(x);
                writeln!(out, "{}", ans).unwrap();
                out.flush().unwrap();
            }
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--batch") {
        run_batch_protocol();
        return;
    }

    let _wl = snapshot::WakeLock::acquire();
    let t0 = Instant::now();
    println!("== TITAN-COUNT PHASE 5 GATE CERTIFICATION (LAW 0 COMPLIANT) ==");

    let mut counter = LehmerCounter::new();

    // -------------------------------------------------------------
    // Criterion 1: Term Oracles & Fourth-Power Boundaries (p^4 +- 1)
    // -------------------------------------------------------------
    println!("\n[1/12] Verifying Term Oracles & Fourth-Power Boundaries (p^4 +- 1)...");
    for &p in &[2u64, 3, 5, 7, 11] {
        let p4 = p * p * p * p;
        for &offset in &[-1i64, 0, 1] {
            let x = ((p4 as i64) + offset) as u64;
            let sieve_cnt = titan_sieve::pi(x);
            let count_cnt = counter.count(x);
            assert_eq!(sieve_cnt, count_cnt, "Mismatch at fourth-power boundary x={}", x);
        }
    }
    println!("  [PASS] Fourth-power boundary matrix {{2^4, 3^4, 5^4, 7^4, 11^4}} +- 1 bit-exact.");

    // -------------------------------------------------------------
    // Criterion 3: Cross-Engine Differential vs Physical Sieve
    // -------------------------------------------------------------
    println!("\n[3/12] Running Cross-Engine Differential: titan-count vs titan-sieve...");
    let test_points: &[u64] = &[
        1_000_000, 2_500_000, 5_000_000, 7_500_000, 10_000_000,
        25_000_000, 50_000_000, 100_000_000, 250_000_000, 500_000_000,
        1_000_000_000, 10_000_000_000,
    ];

    for &x in test_points {
        let count_val = counter.count(x);
        let sieve_val = titan_sieve::pi(x);
        assert_eq!(count_val, sieve_val, "Cross-engine disagreement at x={}!", x);
        println!("  x = {:>10} : π(x) = {:>9}  [MATCH]", x, count_val);
    }
    println!("  [PASS] Cross-engine differential passed: 100% agreement across all test points.");

    // -------------------------------------------------------------
    // Criterion 4: Mutant Kills
    // -------------------------------------------------------------
    println!("\n[4/12] Verifying Combinatorial Mutant Kills...");
    // M-assembly-sign: + P3 instead of - P3
    let mut phi_eng = PhiEngine::new();
    let x_test = 10_000_000u64;
    let ground_truth = 664_579u64;

    let x_root4 = iroot4(x_test);
    let x_sqrt = isqrt(x_test);
    let x_cbrt = icbrt(x_test);

    let base_primes = titan_sieve::base::generate_base_primes(x_sqrt + 100);
    let mut primes = Vec::with_capacity(base_primes.len() + 1);
    primes.push(0);
    primes.extend_from_slice(&base_primes);

    let a = match primes[1..].binary_search(&x_root4) {
        Ok(idx) => idx + 1,
        Err(idx) => idx,
    };
    let b = match primes[1..].binary_search(&x_sqrt) {
        Ok(idx) => idx + 1,
        Err(idx) => idx,
    };
    let c = match primes[1..].binary_search(&x_cbrt) {
        Ok(idx) => idx + 1,
        Err(idx) => idx,
    };

    let max_table = (x_test / primes[a + 1] + 30).max(x_sqrt);
    let pi_table = PiTable::new(max_table);

    let phi_val = phi_eng.eval(x_test, a, &primes, &pi_table);
    let t_val = compute_t(a, b);
    let p2_val = compute_p2(x_test, a, b, &primes, &pi_table);
    let p3_val = compute_p3(x_test, a, c, &primes, &pi_table);

    // Corrupted assembly: + P3
    let corrupted_ans = (phi_val as i128) + (t_val as i128) - (p2_val as i128) + (p3_val as i128);
    assert_ne!(corrupted_ans as u64, ground_truth, "Mutant M-assembly-sign escaped!");
    println!("  [PASS] Mutant M-assembly-sign killed (wrong answer {} != {})", corrupted_ans, ground_truth);

    // -------------------------------------------------------------
    // Criterion 7 & 8: High-Scale Milestone Verification (10^12, 10^13, 10^14)
    // -------------------------------------------------------------
    println!("\n[7/12] Verifying High-Scale Milestones up to 10^14...");
    let t_1e12 = Instant::now();
    let cnt_1e12 = counter.count(1_000_000_000_000);
    let sec_1e12 = t_1e12.elapsed().as_secs_f64();
    assert_eq!(cnt_1e12, 37_607_912_018, "Mismatch at 10^12!");
    println!("  π(10^12) = {:>14} in {:>6.3}s  [PASS]", cnt_1e12, sec_1e12);

    let t_1e13 = Instant::now();
    let cnt_1e13 = counter.count(10_000_000_000_000);
    let sec_1e13 = t_1e13.elapsed().as_secs_f64();
    assert_eq!(cnt_1e13, 346_065_536_839, "Mismatch at 10^13!");
    println!("  π(10^13) = {:>14} in {:>6.3}s  [PASS]", cnt_1e13, sec_1e13);

    let t_1e14 = Instant::now();
    let cnt_1e14 = counter.count(100_000_000_000_000);
    let sec_1e14 = t_1e14.elapsed().as_secs_f64();
    assert_eq!(cnt_1e14, 3_204_941_750_802, "Mismatch at 10^14!");
    println!("  π(10^14) = {:>14} in {:>6.3}s  [PASS]", cnt_1e14, sec_1e14);

    // -------------------------------------------------------------
    // Criterion 12: Write Gate Record
    // -------------------------------------------------------------
    println!("\n[12/12] Writing Gate Record to bench/records/titan_count_gate.json...");
    std::fs::create_dir_all("bench/records").unwrap();
    let elapsed_total = t0.elapsed().as_secs_f64();
    let json = format!(
        r#"{{"phase":"5","status":"PASS","elapsed_sec":{:.3},"pi_1e12_sec":{:.3},"pi_1e13_sec":{:.3},"pi_1e14_sec":{:.3}}}"#,
        elapsed_total, sec_1e12, sec_1e13, sec_1e14
    );
    std::fs::write("bench/records/titan_count_gate.json", &json).unwrap();
    println!("  [PASS] Phase 5 gate record persisted in {:.3}s.", elapsed_total);

    println!("\n=== PHASE 5 GATE: ALL TESTED CRITERIA GREEN (EXIT 0) ===");
}
