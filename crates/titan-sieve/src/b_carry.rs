//! Phase 33: MarkCarry Zero-Division Segment Marking Kernel.
//!
//! Maintains carried state across contiguous segments per thread, eliminating
//! 1.67M runtime 64-bit divisions in the B-marking sweep.
//!
//! Law: candidate-multiples of p form a global increasing bit-index sequence
//! with delta period 8 (wheel.rs const proof). Segments are 30-aligned and contiguous.

use crate::kernels::{compute_wheel_deltas_for_prime, first_mark, mark_wheel8};

pub struct MarkCarry {
    pub i_global: u64,
    pub d: [u32; 8],
    pub j: usize,
}

impl MarkCarry {
    /// Bootstrap: ONE u64 division per prime per thread across entire range.
    pub fn new(p: u64, thread_lo: u64) -> Self {
        let (i0, _r, s) = first_mark(p, thread_lo);
        let d = compute_wheel_deltas_for_prime(p, s);
        let base_bits = (thread_lo / 30) * 8;
        Self {
            i_global: base_bits + (i0 as u64),
            d,
            j: 0,
        }
    }

    /// Mark a contiguous segment with zero divisions.
    ///
    /// # Safety
    /// `bits` must be valid for writes up to `bits.len()`.
    #[inline(always)]
    pub unsafe fn mark(&mut self, bits: &mut [u8], seg_base_bits: u64, p: u32) {
        let nbits = (bits.len() * 8) as u64;
        if self.i_global < seg_base_bits {
            // Already behind, shouldn't happen for strictly ascending segments
            return;
        }

        let mut i = self.i_global - seg_base_bits;
        if i >= nbits {
            // No marks in this segment, carried index remains unchanged
            return;
        }

        let base_ptr = bits.as_mut_ptr();

        // 1. Prologue to cycle boundary (j == 0)
        while self.j != 0 && i < nbits {
            *base_ptr.add((i >> 3) as usize) |= 1 << (i & 7);
            i += self.d[self.j] as u64;
            self.j = (self.j + 1) & 7;
        }

        // 2. 8-unrolled body for full wheel cycles
        let p_u64 = p as u64;
        let cycle_bits = 8 * p_u64;
        let stop = nbits.saturating_sub(cycle_bits);

        let d0 = self.d[0] as u64;
        let d1 = self.d[1] as u64;
        let d2 = self.d[2] as u64;
        let d3 = self.d[3] as u64;
        let d4 = self.d[4] as u64;
        let d5 = self.d[5] as u64;
        let d6 = self.d[6] as u64;
        let d7 = self.d[7] as u64;

        while i < stop {
            *base_ptr.add((i >> 3) as usize) |= 1 << (i & 7);
            i += d0;
            *base_ptr.add((i >> 3) as usize) |= 1 << (i & 7);
            i += d1;
            *base_ptr.add((i >> 3) as usize) |= 1 << (i & 7);
            i += d2;
            *base_ptr.add((i >> 3) as usize) |= 1 << (i & 7);
            i += d3;
            *base_ptr.add((i >> 3) as usize) |= 1 << (i & 7);
            i += d4;
            *base_ptr.add((i >> 3) as usize) |= 1 << (i & 7);
            i += d5;
            *base_ptr.add((i >> 3) as usize) |= 1 << (i & 7);
            i += d6;
            *base_ptr.add((i >> 3) as usize) |= 1 << (i & 7);
            i += d7;
        }

        // 3. Rotating tail
        while i < nbits {
            *base_ptr.add((i >> 3) as usize) |= 1 << (i & 7);
            i += self.d[self.j] as u64;
            self.j = (self.j + 1) & 7;
        }

        // Record the next mark's global position for the next segment
        self.i_global = seg_base_bits + i;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernels::mark_wheel8;

    #[test]
    fn test_mark_carry_matches_stateless_kernel() {
        let p = 11u64;
        let seg_len = 1024usize;
        let nbits = (seg_len * 8) as u64;

        let seg0_lo = 300_000u64;
        let seg0_base_bits = (seg0_lo / 30) * 8;

        let seg1_lo = seg0_lo + (seg_len as u64 / 8) * 30;
        let seg1_base_bits = (seg1_lo / 30) * 8;

        let mut carry = MarkCarry::new(p, seg0_lo);

        let mut bits0_carry = vec![0u8; seg_len];
        let mut bits1_carry = vec![0u8; seg_len];

        unsafe {
            carry.mark(&mut bits0_carry, seg0_base_bits, p as u32);
            carry.mark(&mut bits1_carry, seg1_base_bits, p as u32);
        }

        // Compare against stateless reference
        let mut bits0_ref = vec![0u8; seg_len];
        let mut bits1_ref = vec![0u8; seg_len];

        let (i0, _r0, s0) = first_mark(p, seg0_lo);
        let d0 = compute_wheel_deltas_for_prime(p, s0);
        unsafe {
            mark_wheel8(&mut bits0_ref, p, i0, &d0);
        }

        let (i1, _r1, s1) = first_mark(p, seg1_lo);
        let d1 = compute_wheel_deltas_for_prime(p, s1);
        unsafe {
            mark_wheel8(&mut bits1_ref, p, i1, &d1);
        }

        assert_eq!(bits0_carry, bits0_ref, "Seg 0 bits must match exactly");
        assert_eq!(bits1_carry, bits1_ref, "Seg 1 carried bits must match exactly");
    }
}
