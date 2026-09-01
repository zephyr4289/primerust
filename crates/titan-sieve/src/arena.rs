//! Arena: single construction-time allocation for sieve workspace.

use crate::base::generate_base_primes;
use crate::erat_big::BucketRing;
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
    pub bucket_ring: Option<BucketRing>,
    pub small_threshold: u64,
    pub medium_threshold: u64,
    pub base_frontier_idx: usize,
    pub window_size: usize,
}

impl SieveArena {
    pub fn new(n: u64, seg_size_bytes: usize) -> Self {
        Self::new_with_window(n, seg_size_bytes, 16)
    }

    pub fn new_with_window(n: u64, seg_size_bytes: usize, window_size: usize) -> Self {
        let sqrt_n = isqrt(n);
        let all_base_primes = generate_base_primes(sqrt_n);

        // Filter primes >= 7 for the wheel sieve
        let base_primes: Vec<u64> = all_base_primes
            .into_iter()
            .filter(|&p| p >= 7)
            .collect();

        let small_threshold = (seg_size_bytes / 4) as u64;
        let medium_threshold = (seg_size_bytes * 4) as u64;

        let bucket_ring = if sqrt_n > medium_threshold {
            let pool_cap = ((sqrt_n - medium_threshold) as usize / 10).max(1024);
            Some(BucketRing::new(window_size, pool_cap))
        } else {
            None
        };

        Self {
            segment_buf: vec![0u8; seg_size_bytes].into_boxed_slice(),
            presieve: PreSieve::new(),
            base_primes,
            small_primes: Vec::with_capacity(2048),
            medium_primes: Vec::with_capacity(32768),
            bucket_ring,
            small_threshold,
            medium_threshold,
            base_frontier_idx: 0,
            window_size,
        }
    }

    /// Reset state for a new run without reallocating
    pub fn reset(&mut self) {
        self.small_primes.clear();
        self.medium_primes.clear();
        if let Some(ref mut ring) = self.bucket_ring {
            ring.reset();
        }
        self.base_frontier_idx = 0;
    }
}
