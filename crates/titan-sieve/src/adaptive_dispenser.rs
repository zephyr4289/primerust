//! Phase 41: Adaptive Chunk Dispenser with Geometric Decay (AdaptiveChunkDispenser).
//!
//! Replaces uniform task partitioning with asymmetric geometric decay.
//! Fast Cortex-A78 cores consume large chunks early; trailing slices decay to 1 segment
//! to completely eliminate barrier starvation at the finish line.

use std::sync::atomic::{AtomicU64, Ordering};

#[repr(C, align(64))]
pub struct AdaptiveChunkDispenser {
    current_cursor: AtomicU64,
    total_elements: u64,
}

impl AdaptiveChunkDispenser {
    pub const fn new(total: u64) -> Self {
        Self {
            current_cursor: AtomicU64::new(0),
            total_elements: total,
        }
    }

    /// Dynamically scales chunk size based on remaining distance.
    /// Fast A78s consume large chunks early; trailing slices are single-block to eliminate stalls.
    #[inline(always)]
    pub fn claim_work(&self, is_big_core: bool) -> Option<(u64, u64)> {
        let mut curr = self.current_cursor.load(Ordering::Relaxed);

        loop {
            if curr >= self.total_elements {
                return None;
            }

            let remaining = self.total_elements - curr;

            // A78 grabs larger initial chunks; chunk size decays smoothly to 1
            let chunk_size = if is_big_core {
                ((remaining >> 4).clamp(1, 64)) // Big cores take up to 64 segments
            } else {
                ((remaining >> 6).clamp(1, 16)) // Little cores take up to 16 segments
            };

            let next = (curr + chunk_size).min(self.total_elements);

            match self.current_cursor.compare_exchange_weak(
                curr,
                next,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ) {
                Ok(_) => return Some((curr, next)),
                Err(actual) => curr = actual,
            }
        }
    }

    pub fn reset(&self) {
        self.current_cursor.store(0, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adaptive_chunk_dispenser_decay() {
        let dispenser = AdaptiveChunkDispenser::new(1000);
        let mut chunks = Vec::new();

        while let Some((lo, hi)) = dispenser.claim_work(true) {
            chunks.push((lo, hi, hi - lo));
        }

        assert!(!chunks.is_empty());
        // First chunk should be 64 (max for big core)
        assert_eq!(chunks[0].2, 62); // 1000 >> 4 = 62
        // Last chunk should be 1
        assert_eq!(chunks.last().unwrap().2, 1);
        assert_eq!(chunks.last().unwrap().1, 1000);
    }
}
