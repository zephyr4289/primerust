//! Assembly: Combines Lehmer-class components into exact prime count pi(x).
//!
//! Formula:
//!   pi(x) = Phi(x, a) + T(a, b) - P2(x, a, b) - P3(x, a, c)
//!
//! Verified for any a in [pi(x^1/4), pi(x^1/3)].

use crate::p2_sweep::compute_p2;
use crate::p3::compute_p3;
use crate::phi::PhiEngine;
use crate::pi_table::PiTable;
use titan_core::roots::{icbrt, iroot4, isqrt};

/// Compute T(a, b) = (b + a - 2) * (b - a + 1) / 2
#[inline(always)]
pub fn compute_t(a: usize, b: usize) -> u128 {
    if a > b {
        return 0;
    }
    let a = a as u128;
    let b = b as u128;
    ((b + a - 2) * (b - a + 1)) / 2
}

pub struct LehmerCounter {
    phi_engine: PhiEngine,
}

impl LehmerCounter {
    pub fn new() -> Self {
        Self {
            phi_engine: PhiEngine::new(),
        }
    }

    /// Exact evaluation of pi(x) using the Lehmer identity with a = pi(x^1/4)
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

        let x_root4 = iroot4(x);
        let x_cbrt = icbrt(x);
        let x_sqrt = isqrt(x);

        let max_prime_needed = (x_sqrt + 100).max(100);
        let base_primes = titan_sieve::base::generate_base_primes(max_prime_needed);
        let mut primes = Vec::with_capacity(base_primes.len() + 1);
        primes.push(0); // 1-indexed: primes[1]=2, primes[2]=3, ...
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

        // Compute PiTable up to max(x / primes[a + 1] + 30, x_sqrt)
        let max_table = if a + 1 < primes.len() {
            (x / primes[a + 1] + 30).max(x_sqrt)
        } else {
            x_sqrt + 30
        };
        let pi_table = PiTable::new(max_table);

        let phi_val = self.phi_engine.eval(x, a, &primes, &pi_table);
        let t_val = compute_t(a, b);
        let p2_val = compute_p2(x, a, b, &primes, &pi_table);
        let p3_val = compute_p3(x, a, c, &primes, &pi_table);

        let ans = (phi_val as i128) + (t_val as i128) - (p2_val as i128) - (p3_val as i128);
        assert!(ans >= 0, "Negative count in assembly: {}", ans);
        ans as u64
    }

    /// Evaluates pi(x) with custom a parameter (for alpha sweep)
    pub fn count_with_a(&mut self, x: u64, a: usize) -> u64 {
        if x < 31 {
            return self.count(x);
        }

        let x_cbrt = icbrt(x);
        let x_sqrt = isqrt(x);

        let base_primes = titan_sieve::base::generate_base_primes(x_sqrt + 100);
        let mut primes = Vec::with_capacity(base_primes.len() + 1);
        primes.push(0);
        primes.extend_from_slice(&base_primes);

        let max_table = x_sqrt.max(x_cbrt * x_cbrt).min(x);
        let pi_table = PiTable::new(max_table);

        let b = pi_table.pi(x_sqrt) as usize;
        let c = pi_table.pi(x_cbrt) as usize;

        let phi_val = self.phi_engine.eval(x, a, &primes, &pi_table);
        let t_val = compute_t(a, b);
        let p2_val = compute_p2(x, a, b, &primes, &pi_table);
        let p3_val = compute_p3(x, a, c, &primes, &pi_table);

        let ans = (phi_val as i128) + (t_val as i128) - (p2_val as i128) - (p3_val as i128);
        ans as u64
    }
}
