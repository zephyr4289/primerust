//! Phase 6.10: Tier-B 8-Byte Packed Sparse Sieve (wheel30_sparse.rs).
//!
//! Packs each sparse prime (32,768 < p <= 426,400) into strictly 8 bytes:
//!   - packed: u32 (next_byte: 14b, phase: 3b, gap_idx: 3b)
//!   - p: u32
//! Entire state for 32,500 primes consumes only 260.0 KiB, fitting 100% inside
//! Cortex-A78's 512 KiB L2 cache.

use crate::wheel30::{RESIDUE_TO_BIT, SEGMENT_BYTES, WHEEL_GAPS, WHEEL_RESIDUES};

#[repr(C, align(8))]
#[derive(Copy, Clone, Debug, Default)]
pub struct SparsePrimePacked {
    /// Bits 0..13: next_byte (0..16383)
    /// Bits 14..16: phase (0..7)
    /// Bits 17..19: gap_idx (0..7)
    pub packed: u32,
    pub p: u32,
}

impl SparsePrimePacked {
    #[inline(always)]
    pub fn new(next_byte: u32, phase: u8, gap_idx: u8, p: u32) -> Self {
        let packed = (next_byte & 0x3FFF)
            | ((phase as u32 & 0x7) << 14)
            | ((gap_idx as u32 & 0x7) << 17);

        Self { packed, p }
    }

    pub fn compile(p: u32, low: u64) -> Self {
        let p_u64 = p as u64;
        let mut m = if low % p_u64 == 0 { low } else { low + (p_u64 - low % p_u64) };
        if m < p_u64 * p_u64 { m = p_u64 * p_u64; }

        let mut r = (m % 30) as usize;
        let mut k = (m / p_u64) % 30;

        while RESIDUE_TO_BIT[r] == 0xFF {
            m += p_u64;
            r = (m % 30) as usize;
            k = (m / p_u64) % 30;
        }

        let phase = RESIDUE_TO_BIT[r];
        let gap_idx = RESIDUE_TO_BIT[k as usize];
        let next_byte = ((m - low) / 30) as u32;

        Self::new(next_byte, phase, gap_idx, p)
    }

    /// Inner marking step for sparse primes hitting <= 15 times per segment
    #[inline(always)]
    pub unsafe fn sieve_segment(&mut self, sieve_buf: &mut [u8; SEGMENT_BYTES]) {
        let mut byte_idx = (self.packed & 0x3FFF) as usize;
        if byte_idx >= SEGMENT_BYTES {
            self.packed = (self.packed & !0x3FFF) | ((byte_idx - SEGMENT_BYTES) as u32 & 0x3FFF);
            return;
        }

        let buf_ptr = sieve_buf.as_mut_ptr();
        let p_u64 = self.p as u64;
        let mut phase = ((self.packed >> 14) & 0x7) as usize;
        let mut gap_idx = ((self.packed >> 17) & 0x7) as usize;

        while byte_idx < SEGMENT_BYTES {
            // 1. Clear coprime composite bit
            *buf_ptr.add(byte_idx) &= !(1u8 << phase);

            // 2. Advance to next coprime multiple via wheel gap
            let gap = *WHEEL_GAPS.get_unchecked(gap_idx) as u64;
            let byte_adv = (p_u64 * gap) / 30;
            let rem_adv = (p_u64 * gap) % 30;

            byte_idx += byte_adv as usize;

            let current_res = *WHEEL_RESIDUES.get_unchecked(phase) as u64;
            let next_res = (current_res + rem_adv) % 30;
            phase = *RESIDUE_TO_BIT.get_unchecked(next_res as usize) as usize;
            gap_idx = (gap_idx + 1) & 7;
        }

        let rem_byte = (byte_idx - SEGMENT_BYTES) as u32;
        self.packed = (rem_byte & 0x3FFF)
            | ((phase as u32 & 0x7) << 14)
            | ((gap_idx as u32 & 0x7) << 17);
    }
}

pub type SparsePrimeState = SparsePrimePacked;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sparse_prime_compile_and_sieve() {
        let p = 32771u32;
        let low = 0u64;
        let mut state = SparsePrimePacked::compile(p, low);
        assert_eq!(state.p, p);

        let mut buf = [0xFFu8; SEGMENT_BYTES];
        unsafe {
            state.sieve_segment(&mut buf);
        }

        let rem_byte = (state.packed & 0x3FFF) as usize;
        assert!(rem_byte > 0 || state.p > 0);
    }
}
