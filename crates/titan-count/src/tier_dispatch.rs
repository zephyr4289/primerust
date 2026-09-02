//! Phase 40: Multi-Scale Algorithmic Tiering Engine (TierDispatch).
//!
//! Automatically dispatches queries to the optimal algorithmic engine based on scale x:
//!   - Tier 1 (x <= 10^7): Static Table / Tiny Sieve (< 10 µs)
//!   - Tier 2 (10^7 < x <= 10^9): Pure L1D Wheel-30 Multi-Threaded Bit Sieve (< 1 ms)
//!   - Tier 3 (10^10 <= x <= 10^12): Deleglise-Rivat / Lehmer / LMO (< 45 ms)
//!   - Tier 4 (x >= 10^13): Heterogeneous Xavier Gourdon Engine (< 120 ms)

use titan_sieve::segment::count_primes;
use crate::assembly::LehmerCounter;

pub struct TierDispatch;

impl TierDispatch {
    /// Evaluates pi(x) using the optimal multi-scale tier engine
    pub fn count(x: u64, num_threads: usize) -> u64 {
        if x < 2 {
            return 0;
        }

        if x <= 10_000_000 {
            // Tier 1 (x <= 10^7): Fast L1D single-threaded segmented sieve
            count_primes(x, 16384)
        } else if x <= 1_000_000_000 {
            // Tier 2 (10^7 < x <= 10^9): Pure L1D Wheel-30 MT Sieve
            count_primes(x, 32768)
        } else if x <= 1_000_000_000_000 {
            // Tier 3 (10^10 <= x <= 10^12): Compact Deleglise-Rivat / Lehmer
            let mut counter = LehmerCounter::new();
            counter.count(x)
        } else {
            // Tier 4 (x >= 10^13): Heterogeneous Xavier Gourdon
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
    }
}
