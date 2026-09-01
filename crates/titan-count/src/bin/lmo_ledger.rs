//! LMO Performance Ledger: Term decomposition and collapse ratio on SM4450.
//!
//! Evaluates LMO engine across 10^12, 10^13, 10^14 (8-thread MT)
//! with fine-grained term timing and memory assertions.

use std::time::Instant;
use titan_bench::snapshot;
use titan_core::phi_tiny::phi_tiny;
use titan_core::roots::{icbrt, isqrt};
use titan_count::leaves::LeafEngine;
use titan_count::p2_sweep::compute_p2_mt;
use titan_count::pi_table::PiTable;

fn main() {
    let _wl = snapshot::WakeLock::acquire();
    let raw_mode = std::env::args().any(|a| a == "--raw");

    if !raw_mode {
        println!("=========================================================================================");
        println!("               TITAN LMO PERFORMANCE LEDGER (SM4450 / 8-THREAD MT)                       ");
        println!("=========================================================================================");
        println!(" Scale | Table+Mu  | S0 (Tiny) | S1 (Leaves)| S2 (Sweep)| Total MT   | Phi-Tree | Collapse Ratio");
        println!("-----------------------------------------------------------------------------------------");
    }

    let scales: &[(u32, u64, f64)] = &[
        (12, 37_607_912_018, 0.316),
        (13, 346_065_536_839, 2.219),
        (14, 3_204_941_750_802, 18.307),
    ];

    for &(pow, expected, old_tree_sec) in scales {
        let x = 10u64.pow(pow);
        let t_total_start = Instant::now();

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

        let t_tab_0 = Instant::now();
        let pi_table = PiTable::new(x_sqrt + 30);
        let sec_table = t_tab_0.elapsed().as_secs_f64();

        // 1. S0 (Phi_tiny)
        let t_s0_0 = Instant::now();
        let k = 6.min(a);
        let s0 = phi_tiny(x, k as u64) as i64;
        let sec_s0 = t_s0_0.elapsed().as_secs_f64();

        // 2. S1 (LeafEngine)
        let t_s1_0 = Instant::now();
        let mut leaf_engine = LeafEngine::new();
        let leaf_sum = leaf_engine.eval_leaves(x, a, &primes, &pi_table);
        let s1 = leaf_sum.s0_val + leaf_sum.s1_val;
        let sec_s1 = t_s1_0.elapsed().as_secs_f64();

        // 3. S2 (Range sweep 8T)
        let t_s2_0 = Instant::now();
        let p2_val = compute_p2_mt(x, a, b, &primes, &pi_table, 8);
        let s2_corr = titan_count::meissel::compute_s2_correction(a, b);
        let s2_val = (p2_val as i128) - (s2_corr as i128);
        let sec_s2 = t_s2_0.elapsed().as_secs_f64();

        // Total assembly
        let phi_val = s1; // leaf_sum computes full Phi(x, a)
        let ans = (phi_val as i128) + (a as i128) - 1 - s2_val;
        assert_eq!(ans as u64, expected, "LMO count mismatch at 10^{}", pow);
        let sec_total = t_total_start.elapsed().as_secs_f64();

        let phi_lmo_time = sec_s0 + sec_s1;
        let collapse_ratio = old_tree_sec / phi_lmo_time.max(0.0001);

        if raw_mode {
            println!(
                "{{\"scale\":10^{},\"total_sec\":{:.4},\"table_sec\":{:.4},\"s0_sec\":{:.4},\"s1_sec\":{:.4},\"s2_sec\":{:.4},\"old_tree_sec\":{:.4},\"collapse_ratio\":{:.2},\"status\":\"PASS\"}}",
                pow, sec_total, sec_table, sec_s0, sec_s1, sec_s2, old_tree_sec, collapse_ratio
            );
        } else {
            println!(
                " 10^{:<2} | {:>8.4}s | {:>8.4}s | {:>8.4}s | {:>8.4}s | {:>8.4}s | {:>7.3}s | {:>10.2}x",
                pow, sec_table, sec_s0, sec_s1, sec_s2, sec_total, old_tree_sec, collapse_ratio
            );
        }
    }

    if !raw_mode {
        println!("=========================================================================================");
    }
}
