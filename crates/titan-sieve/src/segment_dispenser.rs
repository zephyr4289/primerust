//! Phase 40: Zero-Copy Lockless Segment Dispenser (SegmentDispenser).
//!
//! Replaces heavy memcpy queues with a cache-line aligned atomic task dispenser,
//! eliminating false sharing and redundant DRAM/L3 traffic across the ARM DynamIQ cluster.

use std::sync::atomic::{AtomicU64, Ordering};

#[repr(C, align(64))]
pub struct SegmentDispenser {
    current_segment: AtomicU64,
    total_segments: u64,
    segment_size: u64, // Number of odd residue slots (e.g. 32,768)
    base_offset: u64,
}

impl SegmentDispenser {
    pub fn new(total_range_lo: u64, total_range_hi: u64, segment_size: u64) -> Self {
        let total_span = total_range_hi.saturating_sub(total_range_lo);
        let span_per_seg = segment_size * 2; // Each bit is odd integer (step of 2)
        let total_segments = (total_span + span_per_seg - 1) / span_per_seg;

        Self {
            current_segment: AtomicU64::new(0),
            total_segments,
            segment_size,
            base_offset: total_range_lo,
        }
    }

    /// Fetches the next 8-byte range descriptor (low, high) locklessly.
    #[inline(always)]
    pub fn fetch_next_range(&self) -> Option<(u64, u64)> {
        let seg_idx = self.current_segment.fetch_add(1, Ordering::Relaxed);
        if seg_idx >= self.total_segments {
            return None;
        }
        let low = self.base_offset + seg_idx * self.segment_size * 2;
        let high = low + (self.segment_size * 2);
        Some((low, high))
    }

    /// Reset dispenser for a new sieve pass
    pub fn reset(&self) {
        self.current_segment.store(0, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_segment_dispenser_partition() {
        let dispenser = SegmentDispenser::new(100, 1000, 100);
        let mut ranges = Vec::new();
        while let Some(range) = dispenser.fetch_next_range() {
            ranges.push(range);
        }

        assert!(!ranges.is_empty());
        assert_eq!(ranges[0].0, 100);
        assert_eq!(ranges[0].1, 300);
        assert_eq!(ranges.last().unwrap().1 >= 1000, true);
    }
}
