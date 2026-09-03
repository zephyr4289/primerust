//! Phase 6.3 Step 2: Tier 2 Medium Prime Sieve Kernel (wheel30_medium.rs).
//!
//! For primes 1,200 < p <= 32,768, advances exceed 255 bytes and cannot fit in an 8-bit strip.
//! We precompute an array of 8 16-bit byte deltas ([u16; 8]), eliminating division and modulo operations.

use crate::wheel30::{RESIDUE_TO_BIT, SEGMENT_BYTES, WHEEL_GAPS};

#[repr(C, align(64))]
#[derive(Clone, Copy, Debug)]
pub struct MediumPrimeState {
    pub next_byte: u32,
    pub phase: u8,
    pub _pad: [u8; 3],
    pub advances: [u16; 8],
    pub masks: [u8; 8],
}

impl MediumPrimeState {
    pub fn compile(p: u32, low: u64) -> Self {
        let p_u64 = p as u64;
        let mut m = if low % p_u64 == 0 {
            low
        } else {
            low + (p_u64 - low % p_u64)
        };
        if m < p_u64 * p_u64 {
            m = p_u64 * p_u64;
        }

        let mut r = (m % 30) as usize;
        let mut k = (m / p_u64) % 30;

        while RESIDUE_TO_BIT[r] == 0xFF {
            m += p_u64;
            r = (m % 30) as usize;
            k = (m / p_u64) % 30;
        }

        let next_byte = ((m - low) / 30) as u32;

        let mut advances = [0u16; 8];
        let mut masks = [0u8; 8];
        let mut curr_m = m;
        let mut k_idx = RESIDUE_TO_BIT[k as usize] as usize;

        for step in 0..8 {
            let res = (curr_m % 30) as usize;
            masks[step] = 1u8 << RESIDUE_TO_BIT[res];
            let gap = WHEEL_GAPS[k_idx] as u64;
            let next_m = curr_m + p_u64 * gap;
            advances[step] = ((next_m / 30) - (curr_m / 30)) as u16;
            curr_m = next_m;
            k_idx = (k_idx + 1) & 7;
        }

        Self {
            next_byte,
            phase: 0,
            _pad: [0; 3],
            advances,
            masks,
        }
    }

    #[inline(always)]
    pub unsafe fn sieve_segment(&mut self, sieve_buf: &mut [u8; SEGMENT_BYTES]) {
        let mut byte_idx = self.next_byte as usize;
        if byte_idx >= SEGMENT_BYTES {
            self.next_byte -= SEGMENT_BYTES as u32;
            return;
        }

        let buf_ptr = sieve_buf.as_mut_ptr();
        let mut phase = self.phase as usize;

        while byte_idx < SEGMENT_BYTES {
            let mask = *self.masks.get_unchecked(phase);
            let adv = *self.advances.get_unchecked(phase) as usize;

            *buf_ptr.add(byte_idx) &= !mask;
            byte_idx += adv;
            phase = (phase + 1) & 7;
        }

        self.next_byte = (byte_idx - SEGMENT_BYTES) as u32;
        self.phase = phase as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wheel30::BIT_TO_RESIDUE;

    #[test]
    fn test_medium_prime_marking_exactness() {
        let mut buf = [0xFFu8; SEGMENT_BYTES];
        let p = 1201u32;
        let mut state = MediumPrimeState::compile(p, 0);

        unsafe {
            state.sieve_segment(&mut buf);
        }

        for byte in 0..SEGMENT_BYTES {
            for bit in 0..8 {
                let res = BIT_TO_RESIDUE[bit] as u64;
                let n = (byte as u64) * 30 + res;
                let is_marked = (buf[byte] & (1 << bit)) == 0;

                let should_be_marked = n >= (p as u64) * (p as u64) && n % (p as u64) == 0;
                assert_eq!(
                    is_marked,
                    should_be_marked,
                    "Mismatch for n = {} at byte {}, bit {}",
                    n, byte, bit
                );
            }
        }
    }
}
