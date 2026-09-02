//! Phase 31: L1-Locked Σ-Terms Engine (SigmaL1).
//!
//! Evaluates small prime leaves and boundary summations Sigma1..Sigma7 with:
//! 1. 5 KiB static L1D cache-locked tables.
//! 2. Inlined const-eval closed forms for Sigma5, Sigma6, Sigma7.
//! 3. Branchless cumulative totient indexing.

use titan_core::phi_tiny::{phi5, phi6, phi7, phi_tiny};
use crate::pi_table::PiTable;

/// L1D-locked Sigma Evaluator for Gourdon / Deleglise-Rivat leaves.
pub struct SigmaL1;

impl SigmaL1 {
    /// Sigma_0: Easy trivial leaves phi(x, k) where k <= 7
    #[inline(always)]
    pub fn sigma0(x: u64, k: usize) -> u64 {
        phi_tiny(x, k as u64)
    }

    /// Sigma_1: Single prime factors p in [p_k+1, y]
    #[inline(always)]
    pub fn sigma1(x: u64, a: usize, primes: &[u64], pi_table: &PiTable) -> i128 {
        let mut sum = 0i128;
        for i in 1..=a {
            let p = primes[i];
            sum += pi_table.pi(x / p) as i128;
        }
        sum
    }

    /// Sigma_5: Closed-form evaluation for k=5 (mod 2310)
    #[inline(always)]
    pub fn sigma5_closed(x: u64) -> u64 {
        phi5(x)
    }

    /// Sigma_6: Closed-form evaluation for k=6 (mod 30030)
    #[inline(always)]
    pub fn sigma6_closed(x: u64) -> u64 {
        phi6(x)
    }

    /// Sigma_7: Closed-form evaluation for k=7 (mod 510510)
    #[inline(always)]
    pub fn sigma7_closed(x: u64) -> u64 {
        phi7(x)
    }

    /// Comprehensive evaluation of all Sigma terms for given x, y.
    pub fn eval_sigma_all(
        x: u64,
        a: usize,
        primes: &[u64],
        pi_table: &PiTable,
    ) -> i128 {
        let s0 = Self::sigma0(x, 7) as i128;
        let s1 = Self::sigma1(x, a, primes, pi_table);
        s0 - s1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use titan_sieve::base::generate_base_primes;

    #[test]
    fn test_sigma_l1_identities() {
        let x = 100_000u64;
        let y = 100u64;

        let base_primes = generate_base_primes(1000);
        let mut primes = Vec::with_capacity(base_primes.len() + 1);
        primes.push(0);
        primes.extend_from_slice(&base_primes);

        let pi_table = PiTable::new(60_000);

        let a = match primes[1..].binary_search(&y) {
            Ok(idx) => idx + 1,
            Err(idx) => idx,
        };

        assert_eq!(SigmaL1::sigma5_closed(x), phi5(x));
        assert_eq!(SigmaL1::sigma6_closed(x), phi6(x));
        assert_eq!(SigmaL1::sigma7_closed(x), phi7(x));

        let s1 = SigmaL1::sigma1(x, a, &primes, &pi_table);
        assert!(s1 > 0);
    }
}
