//! Phase 47.1: Calibrated Multi-Scale Algorithmic Tier Dispatcher.
//!
//! Strict dispatch boundaries:
//!   - Tier 1 (x <= 10^7): Sub-microsecond single-threaded L1D bitset (< 20 ms)
//!   - Tier 2 (10^7 < x <= 10^11): Combinatorial Lehmer Engine (< 70 ms at 10^11)
//!   - Tier 3 (x >= 10^12): Heterogeneous Xavier Gourdon Engine (GourdonHetero)

use titan_sieve::small_sieve::count_primes_small;
use crate::assembly::LehmerCounter;
use crate::gourdon_hetero::GourdonHetero;

pub struct TierDispatch;

impl TierDispatch {
    /// Evaluates pi(x) using the optimal multi-scale tier engine with transparent execution tracing.
    pub fn count(x: u64, num_threads: usize) -> u64 {
        if x <= 10_000_000 {
            // Tier 1 (x <= 10^7): Single-Threaded Cortex-A78 L1D Bitset
            println!("[TITAN-DISPATCH] Tier 1: Single-Threaded Cortex-A78 L1D Bitset (x = {})", x);
            count_primes_small(x)
        } else if x < 10_000_000_000_000 {
            // Tier 2 (10^7 < x < 1e13): Combinatorial Lehmer Engine
            if x <= 1_000_000_000 {
                println!("[TITAN-DISPATCH] Tier 2: Single-Threaded Combinatorial Lehmer (x = {})", x);
                let mut counter = LehmerCounter::new();
                counter.count(x)
            } else {
                println!("[TITAN-DISPATCH] Tier 2: Multi-Threaded Combinatorial Lehmer (x = {}, threads = {})", x, num_threads);
                let counter = LehmerCounter::new();
                counter.count_mt(x, num_threads)
            }
        } else {
            // Tier 3 (x >= 1e13): Heterogeneous Xavier Gourdon Engine
            println!("[TITAN-DISPATCH] Tier 3: Heterogeneous Xavier Gourdon Engine (x = {}, threads = {})", x, num_threads);
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
