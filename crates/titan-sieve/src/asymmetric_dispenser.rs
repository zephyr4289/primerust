//! Phase 3.2: The Asymmetric Decaying Chunk Dispenser (asymmetric_dispenser.rs).
//!
//! Aligned to 64 bytes (ARM64 cache line) to prevent false sharing.
//! Implements guided decay partitioning across heterogeneous Big.LITTLE cores.

use std::sync::atomic::{AtomicU64, Ordering};
use titan_core::affinity::CoreClass;

#[repr(C, align(64))]
pub struct AsymmetricChunkDispenser {
    cursor: AtomicU64,
    total_segments: u64,
}

impl AsymmetricChunkDispenser {
    pub const fn new(total_segments: u64) -> Self {
        Self {
            cursor: AtomicU64::new(0),
            total_segments,
        }
    }

    /// Atomically claims the next slice of segment indices [start, end).
    /// Dynamically sizes the chunk according to the caller's CoreClass and remaining workload.
    #[inline(always)]
    pub fn claim_chunk(&self, core_class: CoreClass) -> Option<(u64, u64)> {
        let mut curr = self.cursor.load(Ordering::Relaxed);

        loop {
            if curr >= self.total_segments {
                return None;
            }

            let remaining = self.total_segments - curr;

            let chunk_size = match core_class {
                CoreClass::Big => {
                    if remaining > 256 {
                        64
                    } else if remaining > 32 {
                        16
                    } else if remaining > 8 {
                        4
                    } else {
                        1
                    }
                }
                CoreClass::Little => {
                    if remaining > 256 {
                        16
                    } else if remaining > 32 {
                        4
                    } else {
                        // Crucial: Little cores are throttled to 1 segment at the tail
                        // to guarantee zero barrier waiting.
                        1
                    }
                }
            };

            let next = (curr + chunk_size).min(self.total_segments);

            match self.cursor.compare_exchange_weak(
                curr,
                next,
                Ordering::AcqRel,
                Ordering::Relaxed,
            ) {
                Ok(_) => return Some((curr, next)),
                Err(actual) => curr = actual,
            }
        }
    }

    #[inline(always)]
    pub fn is_exhausted(&self) -> bool {
        self.cursor.load(Ordering::Relaxed) >= self.total_segments
    }
}

#[repr(C, align(64))]
pub struct HeterogeneousQuantumDispenser {
    cursor: AtomicU64,
    total_quanta: u64,
}

impl HeterogeneousQuantumDispenser {
    pub const fn new(total_quanta: u64) -> Self {
        Self {
            cursor: AtomicU64::new(0),
            total_quanta,
        }
    }

    /// Returns a slice of quanta [start, end) aligned to core geometry.
    /// Big cores pull even blocks of 2, 8, or 32 quanta.
    /// Little cores pull single quanta or blocks of 4.
    #[inline(always)]
    pub fn claim_quanta(&self, core_class: CoreClass) -> Option<(u64, u64)> {
        let mut curr = self.cursor.load(Ordering::Relaxed);

        loop {
            if curr >= self.total_quanta {
                return None;
            }

            let remaining = self.total_quanta - curr;

            let chunk_size = match core_class {
                CoreClass::Big => {
                    if remaining >= 64 {
                        32
                    } else if remaining >= 16 {
                        8
                    } else if remaining >= 2 {
                        2
                    } else {
                        1
                    }
                }
                CoreClass::Little => {
                    if remaining >= 16 {
                        4
                    } else {
                        1
                    }
                }
            };

            let next = (curr + chunk_size).min(self.total_quanta);

            match self.cursor.compare_exchange_weak(
                curr,
                next,
                Ordering::AcqRel,
                Ordering::Relaxed,
            ) {
                Ok(_) => return Some((curr, next)),
                Err(actual) => curr = actual,
            }
        }
    }

    #[inline(always)]
    pub fn is_exhausted(&self) -> bool {
        self.cursor.load(Ordering::Relaxed) >= self.total_quanta
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asymmetric_dispenser_partition() {
        let total = 500u64;
        let dispenser = AsymmetricChunkDispenser::new(total);

        let mut collected = Vec::new();
        while let Some((start, end)) = dispenser.claim_chunk(CoreClass::Big) {
            for seg in start..end {
                collected.push(seg);
            }
        }

        assert_eq!(collected.len(), total as usize);
        for (idx, &seg) in collected.iter().enumerate() {
            assert_eq!(idx as u64, seg);
        }
    }

    #[test]
    fn test_hetero_quantum_dispenser_partition() {
        let total = 100u64;
        let dispenser = HeterogeneousQuantumDispenser::new(total);

        let mut collected = Vec::new();
        while let Some((start, end)) = dispenser.claim_quanta(CoreClass::Big) {
            for q in start..end {
                collected.push(q);
            }
        }

        assert_eq!(collected.len(), total as usize);
        for (idx, &q) in collected.iter().enumerate() {
            assert_eq!(idx as u64, q);
        }
    }
}

