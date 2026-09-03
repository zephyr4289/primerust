//! Phase 6.13: Zero-Copy Pointer-Swapping Frontier Ring (frontier_ring.rs).
//!
//! Instead of copying 16 KiB per segment via `copy_nonoverlapping` (4.76 GB at 10^18),
//! worker threads and ring slots swap buffer ownership via atomic pointers.
//! Eliminates the 4.76 GB memcpy tax, collapsing into an 8-byte pointer swap.

use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU64, Ordering};
use std::sync::Arc;
use crate::wheel30::SEGMENT_BYTES;

pub const RING_SLOTS: usize = 16; // 16 x 16 KiB = 256 KiB (Fits in A78 L2)

#[repr(C, align(64))]
pub struct FrontierSlot {
    pub seg_idx: AtomicU64,
    pub is_ready: AtomicBool,
    pub popcount: AtomicU64,
    pub buffer_ptr: AtomicPtr<u8>,
}

unsafe impl Send for FrontierSlot {}
unsafe impl Sync for FrontierSlot {}

#[repr(C, align(64))]
pub struct FrontierRingBuffer {
    pub slots: [FrontierSlot; RING_SLOTS],
    pub commit_cursor: AtomicU64,
    pub base_z: u64,
    pub span_per_seg: u64,
    pub total_segments: u64,
}

unsafe impl Send for FrontierRingBuffer {}
unsafe impl Sync for FrontierRingBuffer {}

impl FrontierRingBuffer {
    pub fn new(base_z: u64, span_per_seg: u64, total_segments: u64) -> Arc<Self> {
        let slots: [FrontierSlot; RING_SLOTS] = std::array::from_fn(|_| {
            let initial_buf = Box::into_raw(Box::new([0xFFu8; SEGMENT_BYTES])) as *mut u8;
            FrontierSlot {
                seg_idx: AtomicU64::new(u64::MAX),
                is_ready: AtomicBool::new(false),
                popcount: AtomicU64::new(0),
                buffer_ptr: AtomicPtr::new(initial_buf),
            }
        });

        Arc::new(Self {
            slots,
            commit_cursor: AtomicU64::new(0),
            base_z,
            span_per_seg,
            total_segments,
        })
    }

    /// Zero-copy buffer exchange: swaps local buffer pointer with slot buffer pointer.
    /// Completely eradicates the 16 KiB memcpy per segment.
    #[inline(always)]
    pub fn publish_segment_swap(
        &self,
        seg_idx: u64,
        popcount: u64,
        local_buf: &mut Box<[u8; SEGMENT_BYTES]>,
    ) {
        let slot_idx = (seg_idx as usize) % RING_SLOTS;
        let slot = &self.slots[slot_idx];

        // Wait if B-consumer hasn't drained previous cycle
        while slot.is_ready.load(Ordering::Acquire) {
            std::hint::spin_loop();
        }

        // Atomic pointer swap: 0 bytes copied, zero heap allocations
        let local_raw = unsafe {
            let b = std::ptr::read(local_buf);
            Box::into_raw(b) as *mut u8
        };

        let recycled_ptr = slot.buffer_ptr.swap(local_raw, Ordering::AcqRel);
        unsafe {
            std::ptr::write(local_buf, Box::from_raw(recycled_ptr as *mut [u8; SEGMENT_BYTES]));
        }

        slot.popcount.store(popcount, Ordering::Relaxed);
        slot.seg_idx.store(seg_idx, Ordering::Relaxed);
        slot.is_ready.store(true, Ordering::Release);
    }

    /// Fallback copy publisher for slices or non-heap buffers
    #[inline(always)]
    pub fn publish_segment(&self, seg_idx: u64, popcount: u64, src_buf: &[u8; SEGMENT_BYTES]) {
        let slot_idx = (seg_idx as usize) % RING_SLOTS;
        let slot = &self.slots[slot_idx];

        while slot.is_ready.load(Ordering::Acquire) {
            std::hint::spin_loop();
        }

        let ptr = slot.buffer_ptr.load(Ordering::Relaxed);
        unsafe {
            std::ptr::copy_nonoverlapping(src_buf.as_ptr(), ptr, SEGMENT_BYTES);
        }

        slot.popcount.store(popcount, Ordering::Relaxed);
        slot.seg_idx.store(seg_idx, Ordering::Relaxed);
        slot.is_ready.store(true, Ordering::Release);
    }

    /// Claim next sequential segment for B evaluation.
    /// Returns: (target_seg, low, high, popcount, buf_ptr, slot_idx)
    #[inline(always)]
    pub fn try_acquire_committed(&self) -> Option<(u64, u64, u64, u64, *const u8, usize)> {
        let target_seg = self.commit_cursor.load(Ordering::Relaxed);
        if target_seg >= self.total_segments {
            return None;
        }

        let slot_idx = (target_seg as usize) % RING_SLOTS;
        let slot = &self.slots[slot_idx];

        if slot.is_ready.load(Ordering::Acquire) && slot.seg_idx.load(Ordering::Relaxed) == target_seg {
            let low = self.base_z + target_seg * self.span_per_seg;
            let high = low + self.span_per_seg;
            let popcnt = slot.popcount.load(Ordering::Relaxed);
            let ptr = slot.buffer_ptr.load(Ordering::Relaxed) as *const u8;
            Some((target_seg, low, high, popcnt, ptr, slot_idx))
        } else {
            None
        }
    }

    /// Release slot back to D-workers after B finishes evaluation
    #[inline(always)]
    pub fn release_committed(&self, slot_idx: usize) {
        let slot = &self.slots[slot_idx];
        slot.is_ready.store(false, Ordering::Release);
        self.commit_cursor.fetch_add(1, Ordering::Release);
    }

    /// Returns popcount of a slot
    #[inline(always)]
    pub fn get_slot_popcount(&self, slot_idx: usize) -> u64 {
        self.slots[slot_idx].popcount.load(Ordering::Relaxed)
    }
}

impl Drop for FrontierRingBuffer {
    fn drop(&mut self) {
        for slot in &mut self.slots {
            let ptr = slot.buffer_ptr.load(Ordering::Relaxed);
            if !ptr.is_null() {
                unsafe {
                    drop(Box::from_raw(ptr as *mut [u8; SEGMENT_BYTES]));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frontier_ring_swap_fifo() {
        let ring = FrontierRingBuffer::new(1000, 491520, 10);
        let mut local_buf0 = Box::new([0x11u8; SEGMENT_BYTES]);
        let mut local_buf1 = Box::new([0x22u8; SEGMENT_BYTES]);

        assert!(ring.try_acquire_committed().is_none());

        // Publish segment 0 via swap
        ring.publish_segment_swap(0, 100, &mut local_buf0);

        // Segment 0 must be acquirable
        let (seg0, low0, high0, pop0, ptr0, slot0) = ring.try_acquire_committed().unwrap();
        assert_eq!(seg0, 0);
        assert_eq!(low0, 1000);
        assert_eq!(high0, 1000 + 491520);
        assert_eq!(pop0, 100);
        assert_eq!(unsafe { *ptr0 }, 0x11);
        ring.release_committed(slot0);

        // Publish segment 1 via swap
        ring.publish_segment_swap(1, 42, &mut local_buf1);
        let (seg1, low1, high1, pop1, ptr1, slot1) = ring.try_acquire_committed().unwrap();
        assert_eq!(seg1, 1);
        assert_eq!(pop1, 42);
        assert_eq!(unsafe { *ptr1 }, 0x22);
        ring.release_committed(slot1);

        assert!(ring.try_acquire_committed().is_none());
    }
}
