//! Phase 5 Combinatorial Benchmark Runner.
//!
//! Evaluates timing breakdown for Table, Phi, T, P2, P3 across scales up to 10^14.

use std::time::Instant;
use titan_bench::snapshot;
use titan_core::roots::{icbrt, iroot4, isqrt};
use titan_count::assembly::compute_t;
use titan_count::p2_sweep::compute_p2;
use titan_count::p3::compute_p3;
use titan_count::phi::PhiEngine;
use titan_count::pi_table::PiTable;

fn main() {
    let _wl = snapshot::WakeLock::acquire();
    println!("== TITAN-COUNT COMBINATORIAL BENCHMARK ==");

    let milestones: &[(u32, u64)] = &[
        (10, 455_052_511),
        (11, 4_118_054_813),
        (12, 37_607_912_018),
        (13, 346_065_536_839),
        (14, 3_204_941_750_802),
    ];

    let mut phi_engine = PhiEngine::new();

    for &(pow, expected) in milestones {
        let x = 10u64.pow(pow);
        let t_start = Instant::now();

        let x_root4 = iroot4(x);
        let x_sqrt = isqrt(x);
        let x_cbrt = icbrt(x);

        let t_table_0 = Instant::now();
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

        let max_table = (x / primes[a + 1] + 30).max(x_sqrt);
        let pi_table = PiTable::new(max_table);
        let sec_table = t_table_0.elapsed().as_secs_f64();

        let t_phi_0 = Instant::now();
        let phi_val = phi_engine.eval(x, a, &primes, &pi_table);
        let sec_phi = t_phi_0.elapsed().as_secs_f64();

        let t_val = compute_t(a, b);

        let t_p2_0 = Instant::now();
        let p2_val = compute_p2(x, a, b, &primes, &pi_table);
        let sec_p2 = t_p2_0.elapsed().as_secs_f64();

        let t_p3_0 = Instant::now();
        let p3_val = compute_p3(x, a, c, &primes, &pi_table);
        let sec_p3 = t_p3_0.elapsed().as_secs_f64();

        let ans = (phi_val as i128) + (t_val as i128) - (p2_val as i128) - (p3_val as i128);
        assert_eq!(ans as u64, expected, "Mismatch at 10^{}!", pow);
        let sec_total = t_start.elapsed().as_secs_f64();

        println!(
            "  10^{:<2} = {:>14} | Table: {:>5.3}s | Phi: {:>5.3}s | P2: {:>5.3}s | P3: {:>5.3}s | Total: {:>6.3}s",
            pow, expected, sec_table, sec_phi, sec_p2, sec_p3, sec_total
        );
    }
}
