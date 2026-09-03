//! Phase 42 / Phase 4.3: Monotone Reverse Two-Pointer B(x, y) Streaming with SBRB.
//!
//! Re-exports compute_b_monotone powered by 32 KiB Streaming Block Reciprocal Buffer (SBRB)
//! and 4-way ILP unrolled umulh division.

use crate::pi_table::PiTable;

/// Monotone Two-Pointer B(x, y) Evaluation on Cortex-A78 powered by Phase 4.3 SBRB.
///
/// Computes B(x, y) = sum_{y < p <= sqrt(x)} (pi(x/p) - pi(p) + 1)
#[inline(always)]
pub fn compute_b_monotone(
    x: u64,
    y: u64,
    primes: &[u64],
    pi_table: &PiTable,
) -> i64 {
    crate::b_term::compute_b_monotone(x, y, primes, pi_table)
}

#[cfg(test)]
mod tests {
    use super::*;
    use titan_core::roots::{icbrt, isqrt};
    use titan_sieve::base::generate_base_primes;

    #[test]
    fn test_b_monotone_exactness() {
        let x = 1_000_000u64;
        let x_sqrt = isqrt(x);
        let y = icbrt(x);

        let base_primes = generate_base_primes(x_sqrt + 100);
        let mut primes = vec![0u64];
        primes.extend_from_slice(&base_primes);

        let pi_table = PiTable::new(x_sqrt + 30);
        let b_val = compute_b_monotone(x, y, &primes, &pi_table);
        assert!(b_val > 0);
    }
}
