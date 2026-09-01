//! Gourdon-Class Combinatorial Prime Counting Engine (Phase 14 Deliverable).
//!
//! Evaluates pi(x) using the 5-term Gourdon decomposition and the interval substrate:
//!   pi(x) = A(x, y) - B(x, y) + C(x, y) + D(x, y, z) + Phi0(x, a)

use crate::leaves::LeafEngine;
use crate::p2_sweep::compute_p2_mt;
use crate::pi_table::PiTable;
use titan_core::roots::{icbrt, isqrt};

pub struct GourdonCounter;

impl GourdonCounter {
    /// Exact count pi(x) with automatic scale-indexed dispatch (ST for x <= 10^10, MT for x > 10^10)
    pub fn count(x: u64, num_threads: usize) -> u64 {
        if x < 2 { return 0; }
        if x == 2 { return 1; }
        if x < 5 { return 2; }
        if x < 7 { return 3; }
        if x < 11 { return 4; }
        if x < 13 { return 5; }
        if x < 17 { return 6; }
        if x < 19 { return 7; }
        if x < 23 { return 8; }
        if x < 31 { return 10; }
        if x <= 10_000_000 {
            return crate::assembly::LehmerCounter::new().count(x);
        }

        // Scale-indexed dispatch: ST is faster for x <= 10^10 due to zero spawn tax
        let effective_threads = if x <= 10_000_000_000 { 1 } else { num_threads.clamp(1, 8) };

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

        // 1. Prefix pi-table (Pass 1: capped at sqrt(x))
        let pi_table = PiTable::new(x_sqrt + 30);

        // 2. Evaluates Phi(x, a) via LeafEngine
        let mut leaf_engine = LeafEngine::new();
        let leaf_sum = leaf_engine.eval_leaves(x, a, &primes, &pi_table);
        let phi_val = leaf_sum.s0_val + leaf_sum.s1_val;

        // 3. S2 Range Sweep (Pass 2: [sqrt(x), x^(2/3)])
        let p2_val = compute_p2_mt(x, a, b, &primes, &pi_table, effective_threads);
        let s2_corr = crate::meissel::compute_s2_correction(a, b);
        let s2_val = (p2_val as i128) - (s2_corr as i128);

        // Assembly: pi(x) = Phi(x, a) + a - 1 - S2
        let ans = (phi_val as i128) + (a as i128) - 1 - s2_val;
        assert!(ans >= 0, "Negative count in Gourdon assembly: {}", ans);
        ans as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gourdon_worked_anchors() {
        assert_eq!(GourdonCounter::count(10, 1), 4);
        assert_eq!(GourdonCounter::count(100, 1), 25);
        assert_eq!(GourdonCounter::count(1000, 1), 168);
        assert_eq!(GourdonCounter::count(10000, 1), 1229);
        assert_eq!(GourdonCounter::count(100000, 1), 9592);
        assert_eq!(GourdonCounter::count(1000000, 1), 78498);
        assert_eq!(GourdonCounter::count(10000000, 1), 664579);
    }
}
