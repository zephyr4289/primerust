//! Lagarias-Miller-Odlyzko (LMO) Combinatorial Prime Counting Engine.
//!
//! Evaluates:
//!   pi(x) = Phi(x, a) + a - 1 - S2(x, a, b)
//! where Phi(x, a) = S0(x, a) + S1(x, a) is evaluated via the leaf engine,
//! and S2(x, a, b) is evaluated via the sliced range sweep [x^(1/2), x^(2/3)].

use crate::b_term::compute_b_term_mt;
use crate::leaves::LeafEngine;
use crate::p2_sweep::{compute_p2, compute_p2_mt, compute_p2_range_mt};
use crate::pi_table::PiTable;
use crate::scale_dispatch::ScaleDispatch;
use titan_core::roots::{icbrt, isqrt};

pub struct LmoCounter {
    leaf_engine: LeafEngine,
}

impl LmoCounter {
    pub fn new() -> Self {
        Self {
            leaf_engine: LeafEngine::new(),
        }
    }

    /// Single-threaded LMO prime count pi(x)
    pub fn count(&mut self, x: u64) -> u64 {
        if x < 2 { return 0; }
        if x == 2 { return 1; }
        if x < 5 { return 2; }
        if x < 7 { return 3; }
        if x < 11 { return 4; }
        if x < 13 { return 5; }
        if x < 17 { return 6; }
        if x < 19 { return 7; }
        if x < 23 { return 8; }
        if x < 29 { return 9; }
        if x < 31 { return 10; }

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

        let pi_table = PiTable::new(x_sqrt + 30);

        // 1. Evaluate Phi(x, a) via LeafEngine
        let leaf_sum = self.leaf_engine.eval_leaves(x, a, &primes, &pi_table);
        let phi_val = leaf_sum.s0_val + leaf_sum.s1_val;

        // 2. S2 semiprimes via range sweep [x^(1/2), x^(2/3)]
        let p2_val = compute_p2(x, a, b, &primes, &pi_table);
        let s2_corr = crate::meissel::compute_s2_correction(a, b);
        let s2_val = (p2_val as i128) - (s2_corr as i128);

        // 3. Combine: pi(x) = Phi(x, a) + a - 1 - S2
        let ans = (phi_val as i128) + (a as i128) - 1 - s2_val;
        assert!(ans >= 0, "Negative count in LMO assembly: {}", ans);
        ans as u64
    }

    /// LMO with Gourdon z-split optimization.
    /// Restricts physical S2 sweep to [sqrt(x), z] where z = y * beta.
    /// Adds B term for the upper range (z, x^(2/3)].
    pub fn count_zsplit(&mut self, x: u64, num_threads: usize) -> u64 {
        if x < 100_000 || num_threads <= 1 {
            let mut c = LmoCounter::new();
            return c.count(x);
        }

        let dial = ScaleDispatch::select(x, num_threads);
        let effective_threads = dial.num_threads;

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

        // y = alpha * x^(1/3)
        let y = ((x_cbrt as f64) * dial.alpha_y).round() as u64;
        let a_z = match primes[1..].binary_search(&y) {
            Ok(idx) => idx + 1,
            Err(idx) => idx,
        };

        // z = y * beta (z-split boundary)
        let z = ((y as f64) * dial.beta).round() as u64;

        let pi_table = PiTable::new(x_sqrt + 30);

        // 1. Phi(x, a) via LeafEngine
        let leaf_sum = self.leaf_engine.eval_leaves(x, a_z, &primes, &pi_table);
        let phi_val = leaf_sum.s0_val + leaf_sum.s1_val;

        // 2. S2 Range Sweep with z-split: only [sqrt(x), z]
        let p2_val = compute_p2_range_mt(
            x, a_z, b, &primes, &pi_table,
            x_sqrt + 1,  // lo = sqrt(x) + 1
            z,           // hi = z = y * beta
            effective_threads
        );
        let s2_corr = crate::meissel::compute_s2_correction(a_z, b);
        let s2_val = (p2_val as i128) - (s2_corr as i128);

        // 3. B term: easy semiprimes in (z, x^(2/3)]
        let b_val = compute_b_term_mt(x, y, &primes, &pi_table, effective_threads);

        // Assembly: pi(x) = Phi(x, a) + a - 1 - S2 + B
        let ans = (phi_val as i128) + (a_z as i128) - 1 - s2_val + (b_val as i128);
        assert!(ans >= 0, "Negative count in LMO z-split assembly: {}", ans);
        ans as u64
    }

    /// Multi-threaded LMO prime count pi(x) across num_threads
    pub fn count_mt(&self, x: u64, num_threads: usize) -> u64 {
        if x < 100_000 || num_threads <= 1 {
            let mut c = LmoCounter::new();
            return c.count(x);
        }

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

        let pi_table = PiTable::new(x_sqrt + 30);

        let phi_val = crate::phi::eval_mt(x, a, &primes, &pi_table, num_threads);

        let p2_val = compute_p2_mt(x, a, b, &primes, &pi_table, num_threads);
        let s2_corr = crate::meissel::compute_s2_correction(a, b);
        let s2_val = (p2_val as i128) - (s2_corr as i128);

        let ans = (phi_val as i128) + (a as i128) - 1 - s2_val;
        assert!(ans >= 0, "Negative count in LMO MT assembly: {}", ans);
        ans as u64
    }

    /// Multi-threaded LMO with z-split
    pub fn count_zsplit_mt(&self, x: u64, num_threads: usize) -> u64 {
        if x < 100_000 || num_threads <= 1 {
            let mut c = LmoCounter::new();
            return c.count_zsplit(x, num_threads);
        }

        let dial = ScaleDispatch::select(x, num_threads);
        let effective_threads = dial.num_threads;

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

        let y = ((x_cbrt as f64) * dial.alpha_y).round() as u64;
        let a_z = match primes[1..].binary_search(&y) {
            Ok(idx) => idx + 1,
            Err(idx) => idx,
        };

        let z = ((y as f64) * dial.beta).round() as u64;

        let pi_table = PiTable::new(x_sqrt + 30);

        let phi_val = crate::phi::eval_mt(x, a_z, &primes, &pi_table, effective_threads);

        let p2_val = compute_p2_range_mt(
            x, a_z, b, &primes, &pi_table,
            x_sqrt + 1, z, effective_threads
        );
        let s2_corr = crate::meissel::compute_s2_correction(a_z, b);
        let s2_val = (p2_val as i128) - (s2_corr as i128);

        let b_val = compute_b_term_mt(x, y, &primes, &pi_table, effective_threads);

        let ans = (phi_val as i128) + (a_z as i128) - 1 - s2_val + (b_val as i128);
        assert!(ans >= 0, "Negative count in LMO MT z-split assembly: {}", ans);
        ans as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lmo_worked_anchors() {
        let mut lmo = LmoCounter::new();
        assert_eq!(lmo.count(10), 4);
        assert_eq!(lmo.count(100), 25);
        assert_eq!(lmo.count(1000), 168);
        assert_eq!(lmo.count(10000), 1229);
        assert_eq!(lmo.count(100000), 9592);
        assert_eq!(lmo.count(1000000), 78498);
        assert_eq!(lmo.count(10000000), 664579);
    }
}
