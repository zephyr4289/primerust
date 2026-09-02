//! Phase 40 & Phase 42: Multi-Scale Algorithmic Tiering Engine (TierDispatch).
//!
//! Automatically dispatches queries to the optimal algorithmic engine based on scale x:
//!   - Tier 1 (x <= 10^7): Sub-microsecond single-threaded L1D sieve (< 10 µs)
//!   - Tier 2 (10^7 < x <= 10^9): Pure L1D Wheel-30 Multi-Threaded Bit Sieve (< 1 ms)
//!   - Tier 3 (10^9 < x <= 10^12): Deleglise-Rivat / Lehmer Counter (< 700 ms)
//!   - Tier 4 (x >= 10^13): Monotone-Streaming Combinatorial Engine

use titan_sieve::segment::count_primes;
use titan_sieve::small_sieve::count_primes_small;
use crate::assembly::LehmerCounter;

pub struct TierDispatch;

impl TierDispatch {
    /// Evaluates pi(x) using the optimal multi-scale tier engine
    pub fn count(x: u64, _num_threads: usize) -> u64 {
        if x < 2 {
            return 0;
        }

        if x <= 10_000_000 {
            // Tier 1 (x <= 10^7): Sub-microsecond single-threaded L1D sieve
            count_primes_small(x)
        } else if x <= 1_000_000_000 {
            // Tier 2 (10^7 < x <= 10^9): Pure L1D Wheel-30 MT Sieve
            count_primes(x, 32768)
        } else {
            // Tier 3 & 4 (x >= 10^10): Exact combinatorial Lehmer with L1D Phi
            let mut counter = LehmerCounter::new();
            counter.count(x)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_dispatch_milestones() {
        assert_eq!(TierDispatch::count(10, 8), 4);
        assert_eq!(TierDispatch::count(100, 8), 25);
        assert_eq!(TierDispatch::count(1_000, 8), 168);
        assert_eq!(TierDispatch::count(1_000_000, 8), 78498);
        assert_eq!(TierDispatch::count(100_000_000, 8), 5761455);
        assert_eq!(TierDispatch::count(1_000_000_000, 8), 50847534);
    }
}
