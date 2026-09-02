//! Phase 43: Restructured Multi-Scale Algorithmic Tier Dispatcher.
//!
//! Strict dispatch boundaries:
//!   - Tier 1 (x <= 10^7): Sub-microsecond single-threaded L1D sieve (< 20 ms)
//!   - Tier 2 (10^7 < x <= 10^10): Combinatorial LMO / Lehmer Engine (< 25 ms)
//!   - Tier 3 (x >= 10^11): Heterogeneous Xavier Gourdon Engine (GourdonHetero)

use titan_sieve::small_sieve::count_primes_small;
use crate::assembly::LehmerCounter;
use crate::gourdon_hetero::GourdonHetero;

pub struct TierDispatch;

impl TierDispatch {
    /// Evaluates pi(x) using the optimal multi-scale tier engine
    pub fn count(x: u64, num_threads: usize) -> u64 {
        if x <= 10_000_000 {
            // Tier 1 (x <= 10^7): Single-Threaded Cortex-A78 L1D Bitset
            count_primes_small(x)
        } else if x <= 10_000_000_000 {
            // Tier 2 (10^7 < x <= 10^10): Combinatorial Lehmer / LMO
            let mut counter = LehmerCounter::new();
            counter.count(x)
        } else {
            // Tier 3 (x >= 10^11): Multi-Threaded Heterogeneous Xavier Gourdon
            GourdonHetero::count(x, num_threads)
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
        assert_eq!(TierDispatch::count(10_000_000_000, 8), 455052511);
        assert_eq!(TierDispatch::count(100_000_000_000, 8), 4118054813);
        assert_eq!(TierDispatch::count(1_000_000_000_000, 8), 37607912018);
    }
}
