//! Arena: single construction-time allocation for sieve workspace.

use crate::base::generate_base_primes;
use crate::erat_medium::MediumPrime;
use crate::erat_small::SmallPrime;
use crate::presieve::PreSieve;
use titan_core::roots::isqrt;

pub struct SieveArena {
    pub segment_buf: Box<[u8]>,
    pub presieve: PreSieve,
    pub base_primes: Vec<u64>,
    pub small_primes: Vec<SmallPrime>,
    pub medium_primes: Vec<MediumPrime>,
    pub small_threshold: u64,
    pub base_frontier_idx: usize,
}

impl SieveArena {
    pub fn new(n: u64, seg_size_bytes: usize) -> Self {
        let sqrt_n = isqrt(n);
        let all_base_primes = generate_base_primes(sqrt_n);

        // Filter primes >= 7 for the wheel sieve
        let base_primes: Vec<u64> = all_base_primes
            .into_iter()
            .filter(|&p| p >= 7)
            .collect();

        let small_threshold = (seg_size_bytes / 4) as u64;

        Self {
            segment_buf: vec![0u8; seg_size_bytes].into_boxed_slice(),
            presieve: PreSieve::new(),
            base_primes,
            small_primes: Vec::with_capacity(2048),
            medium_primes: Vec::with_capacity(32768),
            small_threshold,
            base_frontier_idx: 0,
        }
    }

    /// Reset state for a new run without reallocating
    pub fn reset(&mut self) {
        self.small_primes.clear();
        self.medium_primes.clear();
        self.base_frontier_idx = 0;
    }
}
