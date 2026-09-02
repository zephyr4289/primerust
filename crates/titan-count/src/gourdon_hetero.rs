//! Phase 43: Multi-Threaded Heterogeneous Combinatorial Engine (GourdonHetero).
//!
//! Evaluates pi(x) using multi-threaded Lehmer-Gourdon identity:
//!   pi(x) = Phi(x, a) + T(a, b) - P2(x, a, b) - P3(x, a, c)
//!
//! Parallelized across Snapdragon 4 Gen 2 (SM4450) DynamIQ cluster:
//!   - Cores 6, 7 (Cortex-A78): Coordinator and Phi(x, a) parallel spine
//!   - Cores 0..=5 (Cortex-A55): Multi-threaded P2(x, a, b) and P3(x, a, c) sweep

use titan_core::roots::{icbrt, iroot4, isqrt};
use titan_sieve::base::generate_base_primes;
use crate::assembly::compute_t;
use crate::p2_sweep::compute_p2_mt;
use crate::p3::compute_p3_mt;
use crate::phi::eval_mt;
use crate::pi_table::PiTable;

pub struct GourdonHetero;

impl GourdonHetero {
    /// Multi-threaded evaluation of pi(x) across heterogeneous CPU clusters
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
        if x < 29 { return 9; }
        if x < 31 { return 10; }

        let x_root4 = iroot4(x);
        let x_cbrt = icbrt(x);
        let x_sqrt = isqrt(x);

        let max_prime_needed = (x_sqrt + 100).max(100);
        let base_primes = generate_base_primes(max_prime_needed);
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

        // RAM Law: PiTable span is hard-capped at max(x^1/2, p_{a+1}^2)
        let p_a1 = if a + 1 < primes.len() { primes[a + 1] } else { x_root4 + 1 };
        let max_table = x_sqrt.max(p_a1 * p_a1) + 30;
        let pi_table = PiTable::new(max_table);

        let phi_val = eval_mt(x, a, &primes, &pi_table, num_threads);
        let t_val = compute_t(a, b);
        let p2_val = compute_p2_mt(x, a, b, &primes, &pi_table, num_threads);
        let p3_val = compute_p3_mt(x, a, c, &primes, &pi_table, num_threads);

        let ans = (phi_val as i128) + (t_val as i128) - (p2_val as i128) - (p3_val as i128);
        assert!(ans >= 0, "Negative count in GourdonHetero: {}", ans);
        ans as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gourdon_hetero_exactness() {
        assert_eq!(GourdonHetero::count(10, 8), 4);
        assert_eq!(GourdonHetero::count(100, 8), 25);
        assert_eq!(GourdonHetero::count(1_000, 8), 168);
        assert_eq!(GourdonHetero::count(10_000, 8), 1229);
        assert_eq!(GourdonHetero::count(100_000, 8), 9592);
        assert_eq!(GourdonHetero::count(1_000_000, 8), 78498);
        assert_eq!(GourdonHetero::count(10_000_000, 8), 664579);
        assert_eq!(GourdonHetero::count(100_000_000, 8), 5761455);
        assert_eq!(GourdonHetero::count(1_000_000_000, 8), 50847534);
        assert_eq!(GourdonHetero::count(10_000_000_000, 8), 455052511);
        assert_eq!(GourdonHetero::count(100_000_000_000, 8), 4118054813);
        assert_eq!(GourdonHetero::count(1_000_000_000_000, 8), 37607912018);
    }
}
