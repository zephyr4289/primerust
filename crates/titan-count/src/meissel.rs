//! Meissel: Meissel-class prime counting engine (P3-free combinatorial algorithm).
//!
//! Theorem:
//!   For any a >= pi(x^(1/3)) and b = pi(x^(1/2)):
//!     pi(x) = Phi(x, a) + a - 1 - S2(x, a, b)
//!   where:
//!     S2(x, a, b) = sum_{i=a+1}^b [ pi(floor(x / p_i)) - i + 1 ]
//!                 = P2(x, a, b) - (b + a - 1) * (b - a) / 2
//!
//! Guarantees:
//!   - P3 term vanishes identically (0 cycles spent on P3)
//!   - P2 range sweep span shrinks from x^(3/4) to x^(2/3) (21x reduction at 10^16)
//!   - PiTable span remains hard-capped at x^(1/2) (RAM Law)

use crate::p2_sweep::{compute_p2, compute_p2_mt};
use crate::phi::{eval_mt, PhiEngine};
use crate::pi_table::PiTable;
use titan_core::roots::{icbrt, isqrt};

/// Compute the correction term (b + a - 1) * (b - a) / 2
#[inline(always)]
pub fn compute_s2_correction(a: usize, b: usize) -> u128 {
    if a >= b {
        return 0;
    }
    let a = a as u128;
    let b = b as u128;
    ((b + a - 1) * (b - a)) / 2
}

pub struct MeisselCounter {
    phi_engine: PhiEngine,
}

impl MeisselCounter {
    pub fn new() -> Self {
        Self {
            phi_engine: PhiEngine::new(),
        }
    }

    /// Single-threaded Meissel prime count pi(x)
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

        let phi_val = self.phi_engine.eval(x, a, &primes, &pi_table);
        let p2_val = compute_p2(x, a, b, &primes, &pi_table);
        let s2_corr = compute_s2_correction(a, b);
        let s2_val = (p2_val as i128) - (s2_corr as i128);

        let ans = (phi_val as i128) + (a as i128) - 1 - s2_val;
        assert!(ans >= 0, "Negative count in Meissel assembly: {}", ans);
        ans as u64
    }

    /// Multi-threaded Meissel prime count pi(x) using Spine-Split and multi-threaded P2 sweep
    pub fn count_mt(&self, x: u64, num_threads: usize) -> u64 {
        if x < 100_000 || num_threads <= 1 {
            let mut c = MeisselCounter::new();
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

        let phi_val = eval_mt(x, a, &primes, &pi_table, num_threads);
        let p2_val = compute_p2_mt(x, a, b, &primes, &pi_table, num_threads);
        let s2_corr = compute_s2_correction(a, b);
        let s2_val = (p2_val as i128) - (s2_corr as i128);

        let ans = (phi_val as i128) + (a as i128) - 1 - s2_val;
        assert!(ans >= 0, "Negative count in Meissel MT assembly: {}", ans);
        ans as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meissel_worked_anchors() {
        let mut counter = MeisselCounter::new();
        // x = 100
        assert_eq!(counter.count(100), 25);
        // x = 1000
        assert_eq!(counter.count(1000), 168);
        // x = 10000
        assert_eq!(counter.count(10000), 1229);
        // x = 100000
        assert_eq!(counter.count(100000), 9592);
        // x = 1000000
        assert_eq!(counter.count(1000000), 78498);
        // x = 10000000
        assert_eq!(counter.count(10000000), 664579);
    }

    #[test]
    fn test_meissel_vs_lehmer_differential() {
        let mut meissel = MeisselCounter::new();
        let mut lehmer = crate::assembly::LehmerCounter::new();

        for &x in &[100u64, 500, 1000, 5000, 10000, 50000, 100000, 500000, 1000000, 5000000] {
            let m_val = meissel.count(x);
            let l_val = lehmer.count(x);
            assert_eq!(m_val, l_val, "Meissel and Lehmer disagree at x={}", x);
        }
    }

    #[test]
    fn test_p3_boundary_matrix() {
        let mut meissel = MeisselCounter::new();
        // Test near p^3 boundaries where p_a transitions
        for &p in &[3u64, 5, 7, 11, 13, 17, 19, 23] {
            let p3 = p * p * p;
            for &offset in &[-1i64, 0, 1] {
                let x = ((p3 as i64) + offset) as u64;
                let actual = meissel.count(x);
                let truth = titan_sieve::pi(x);
                assert_eq!(actual, truth, "Meissel failed at p^3 boundary x={}", x);
            }
        }
    }
}
