//! D Term: Gourdon's D(x, y, z) closed-form evaluation.
//!
//! D(x, y, z) = sum_{i=1}^{a} sum_{j=i}^{pi(sqrt(x/p_i))} [ pi(x / (p_i * p_j)) - pi(p_j) + 1 ]
//! where a = pi(y), z = y * beta
//!
//! This is the "hard" special leaves term that requires the interval substrate.
//! The z-split restricts this to (y, z] instead of the full (y, x^(2/3)].

use crate::interval_walker::IntervalWalker;
use crate::pi_table::PiTable;
use crate::mertens_struct::MertensStructure;

/// Evaluates the D term for Gourdon's algorithm.
/// D(x, y, z) covers the "hard" special leaves in (y, z] where z = y * beta.
pub fn compute_d_term(
    x: u64,
    a: usize,
    c: usize,  // pi(z)
    primes: &[u64],
    pi_table: &PiTable,
    mertens: &MertensStructure,
) -> i64 {
    if a >= c {
        return 0; // D vanishes when a >= c
    }

    // D term uses the interval walker for the range (y, z]
    // where y corresponds to prime index a, z corresponds to prime index c
    let d_val = IntervalWalker::walk_intervals(
        x,
        a,
        c,
        primes,
        pi_table,
        mertens,
    );

    d_val
}

/// Multi-threaded D term evaluation.
pub fn compute_d_term_mt(
    x: u64,
    a: usize,
    c: usize,
    primes: &[u64],
    pi_table: &PiTable,
    mertens: &MertensStructure,
    num_threads: usize,
) -> i64 {
    if a >= c || num_threads <= 1 {
        return compute_d_term(x, a, c, primes, pi_table, mertens);
    }

    IntervalWalker::walk_intervals_mt(
        x, a, c, primes, pi_table, mertens, num_threads
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use titan_sieve::base::generate_base_primes;
    use crate::pi_table::PiTable;
    use crate::mertens_struct::MertensStructure;
    use titan_core::roots::{icbrt, isqrt};

    #[test]
    fn test_d_term_basic() {
        let x = 1_000_000u64;
        let x_cbrt = icbrt(x);
        let x_sqrt = isqrt(x);

        let base_primes = generate_base_primes(x_sqrt + 100);
        let mut primes = Vec::with_capacity(base_primes.len() + 1);
        primes.push(0);
        primes.extend_from_slice(&base_primes);

        let a = match primes[1..].binary_search(&x_cbrt) {
            Ok(idx) => idx + 1,
            Err(idx) => idx,
        };

        // z = x^(1/3) * beta, beta = 1.5
        let z = ((x_cbrt as f64) * 1.5).round() as u64;
        let c = match primes[1..].binary_search(&z) {
            Ok(idx) => idx + 1,
            Err(idx) => idx,
        };

        let pi_table = PiTable::new(x_sqrt + 30);
        let mertens = MertensStructure::new(x_sqrt as usize + 100);

        let d_val = compute_d_term(x, a, c, &primes, &pi_table, &mertens);
        // D can be negative, just verify it runs without panic
        let _ = d_val;
    }
}