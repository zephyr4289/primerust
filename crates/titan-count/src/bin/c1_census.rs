//! Pre-Flight Experiment C1: Phi-Census & Tree Sizing.
//!
//! Evaluates node counts per depth, tier exit distribution (T0..T2),
//! and execution wall time across scales 10^10 to 10^14.

use std::time::Instant;
use titan_bench::snapshot;
use titan_core::roots::{icbrt, iroot4, isqrt};
use titan_count::phi::PhiEngine;
use titan_count::pi_table::PiTable;

fn main() {
    let _wl = snapshot::WakeLock::acquire();
    println!("== PRE-FLIGHT EXPERIMENT C1: PHI TREE CENSUS ==");

    let mut engine = PhiEngine::new();

    for &pow in &[10, 11, 12, 13, 14] {
        let x = 10u64.pow(pow);
        let x_root4 = iroot4(x);
        let x_sqrt = isqrt(x);
        let x_cbrt = icbrt(x);

        let base_primes = titan_sieve::base::generate_base_primes(x_sqrt + 100);
        let mut primes = Vec::with_capacity(base_primes.len() + 1);
        primes.push(0);
        primes.extend_from_slice(&base_primes);

        let max_table = x_sqrt.max(x_cbrt * x_cbrt).min(x);
        let pi_table = PiTable::new(max_table);

        let a = pi_table.pi(x_root4) as usize;

        let t0 = Instant::now();
        let val = engine.eval_with_census(x, a, &primes, &pi_table);
        let elapsed = t0.elapsed().as_secs_f64();

        let c = &engine.census;
        println!(
            "  10^{:<2} (a={:>4}) : Phi={:>12} | Nodes={:>9} | T0={:>7} | T1={:>7} | T2={:>7} | Depth={:>3} | Time={:>6.3}s",
            pow, a, val, c.total_nodes, c.t0_exits, c.t1_exits, c.t2_exits, c.max_depth, elapsed
        );
    }
}
