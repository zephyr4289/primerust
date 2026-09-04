//! Phase 47.1: Calibrated Multi-Scale Algorithmic Tier Dispatcher.
//!
//! Strict dispatch boundaries:
//!   - Tier 1 (x <= 10^7): Sub-microsecond single-threaded L1D bitset (< 20 ms)
//!   - Tier 2 (10^7 < x <= 10^11): Combinatorial Lehmer Engine (< 70 ms at 10^11)
//!   - Tier 3 (x >= 10^12): Heterogeneous Xavier Gourdon Engine (GourdonHetero)

use crate::assembly::LehmerCounter;
use crate::gourdon_hetero::GourdonHetero;

pub struct TierDispatch;

impl TierDispatch {
    /// Evaluates pi(x) using the optimal multi-scale tier engine with transparent execution tracing.
    /// Phase 9.2.x: strict 3-tier match (Tier 3 >= 1e13 always routes to pure-Rust Gourdon).
    /// CI-2: thread requests resolve via CpuTopology (free CI = 4, SD4G2 = 8).
    /// Thresholds unchanged in CI-2 (measure before moving Gourdon earlier).
    pub fn count(x: u64, num_threads: usize) -> u64 {
        let threads = titan_core::cpu::CpuTopology::detect().optimal_threads(num_threads);
        match x {
            0..=10_000_000 => {
                // Tier 1: Single-Threaded Cortex-A78 L1D Bitset
                println!("[TITAN-DISPATCH] Tier 1: Single-Threaded Cortex-A78 L1D Bitset (x = {})", x);
                titan_sieve::small_sieve::count_primes_small(x)
            }
            10_000_001..=9_999_999_999_999 => {
                // Tier 2: Combinatorial Lehmer (1e7 < x < 1e13)
                if x <= 1_000_000_000 {
                    println!("[TITAN-DISPATCH] Tier 2: Single-Threaded Combinatorial Lehmer (x = {})", x);
                    let mut counter = LehmerCounter::new();
                    counter.count(x)
                } else {
                    println!("[TITAN-DISPATCH] Tier 2: Multi-Threaded Combinatorial Lehmer (x = {}, threads = {} resolved {})", x, num_threads, threads);
                    let counter = LehmerCounter::new();
                    counter.count_mt(x, threads)
                }
            }
            _ => {
                // Tier 3: Pure-Rust Xavier Gourdon Engine (x >= 1e13)
                // Hard rule: never silently run Lehmer on Tier 3 (see GourdonHetero gating).
                println!("[TITAN-DISPATCH] Tier 3: Heterogeneous Xavier Gourdon Engine (x = {}, threads = {} resolved {})", x, num_threads, threads);
                GourdonHetero::count(x, threads)
            }
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
