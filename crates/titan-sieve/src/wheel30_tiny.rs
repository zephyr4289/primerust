//! Phase 6.2 Step 3: Tiny Prime Vector Mask Engine (wheel30_tiny.rs).
//!
//! For primes p in {7, 11, 13, 17, 19, 23, 29, 31}, precomputes their periodic
//! 16-byte vector masks once. At runtime, sifts 16 bytes (480 integers) per ARM64
//! NEON instruction (`vandq_u8`).

use crate::wheel30::SEGMENT_BYTES;

pub const TINY_PRIMES: [u32; 8] = [7, 11, 13, 17, 19, 23, 29, 31];

#[repr(C, align(64))]
pub struct TinyPrimeMaskTable {
    pub masks: [Vec<[u8; 16]>; 8],
}

impl TinyPrimeMaskTable {
    pub fn new() -> Self {
        let mut masks: [Vec<[u8; 16]>; 8] = Default::default();

        for (idx, &p) in TINY_PRIMES.iter().enumerate() {
            let period = p as usize;
            let mut pattern = vec![0xFFu8; period * 16];

            for k in 1..=(period * 16 * 30) {
                if k % 2 == 0 || k % 3 == 0 || k % 5 == 0 {
                    continue;
                }
                if k % (p as usize) == 0 {
                    let byte_idx = k / 30;
                    let bit = crate::wheel30::RESIDUE_TO_BIT[k % 30];
                    if bit != 0xFF && byte_idx < pattern.len() {
                        pattern[byte_idx] &= !(1u8 << bit);
                    }
                }
            }

            let num_vectors = period;
            let mut vec_list = Vec::with_capacity(num_vectors);
            for v in 0..num_vectors {
                let mut chunk = [0u8; 16];
                chunk.copy_from_slice(&pattern[v * 16..(v + 1) * 16]);
                vec_list.push(chunk);
            }
            masks[idx] = vec_list;
        }

        Self { masks }
    }

    /// Sifts all 8 tiny primes (7..=31) across the 16 KiB segment buffer.
    /// Uses ARM64 NEON vector registers where available.
    #[inline(always)]
    pub unsafe fn sieve_tiny_primes(&self, sieve_buf: &mut [u8; SEGMENT_BYTES], seg_idx: u64) {
        let ptr = sieve_buf.as_mut_ptr();
        let start_vec = seg_idx * ((SEGMENT_BYTES / 16) as u64);

        for (idx, &p) in TINY_PRIMES.iter().enumerate() {
            let vec_masks = &self.masks[idx];
            let period = p as usize;
            let mut phase = (start_vec % (period as u64)) as usize;

            #[cfg(target_arch = "aarch64")]
            {
                use core::arch::aarch64::*;
                for offset in (0..SEGMENT_BYTES).step_by(16) {
                    let mask_ptr = vec_masks.get_unchecked(phase).as_ptr();
                    let v_data = vld1q_u8(ptr.add(offset));
                    let v_mask = vld1q_u8(mask_ptr);
                    let v_res = vandq_u8(v_data, v_mask);
                    vst1q_u8(ptr.add(offset), v_res);

                    phase += 1;
                    if phase == period {
                        phase = 0;
                    }
                }
            }

            #[cfg(not(target_arch = "aarch64"))]
            {
                for offset in (0..SEGMENT_BYTES).step_by(16) {
                    let mask = vec_masks.get_unchecked(phase);
                    for b in 0..16 {
                        *ptr.add(offset + b) &= mask[b];
                    }
                    phase += 1;
                    if phase == period {
                        phase = 0;
                    }
                }
            }
        }

        // If this is the very first segment (covering 0..491,520), restore
        // the tiny primes 7..=31 themselves (they are prime, not composite)
        if seg_idx == 0 {
            *ptr |= 0b1111_1110;       // primes 7, 11, 13, 17, 19, 23, 29 in byte 0
            *ptr.add(1) |= 0b0000_0001; // prime 31 in byte 1
        }
    }

    /// Fused buffer initialization and Tiny Prime sieve.
    /// Completely eradicates the 16 KiB memset(0xFF) pass by having prime 7
    /// directly store its periodic mask via vst1q_u8, initializing the uninitialized buffer.
    #[inline(always)]
    pub unsafe fn sieve_tiny_primes_fused(&self, sieve_buf: &mut [u8; SEGMENT_BYTES], seg_idx: u64) {
        let ptr = sieve_buf.as_mut_ptr();
        let start_vec = seg_idx * ((SEGMENT_BYTES / 16) as u64);

        // 1. Prime 7: DIRECT STORE SEEDING (Overwrites uninitialized L1D buffer)
        {
            let p7_masks = &self.masks[0];
            let period = 7usize;
            let mut phase = (start_vec % 7) as usize;

            #[cfg(target_arch = "aarch64")]
            {
                use core::arch::aarch64::*;
                for offset in (0..SEGMENT_BYTES).step_by(16) {
                    let mask_ptr = p7_masks.get_unchecked(phase).as_ptr();
                    let v_mask = vld1q_u8(mask_ptr);
                    // Direct vector store: zero prior loads, zero memset
                    vst1q_u8(ptr.add(offset), v_mask);

                    phase += 1;
                    if phase == period {
                        phase = 0;
                    }
                }
            }

            #[cfg(not(target_arch = "aarch64"))]
            {
                for offset in (0..SEGMENT_BYTES).step_by(16) {
                    let mask = p7_masks.get_unchecked(phase);
                    std::ptr::copy_nonoverlapping(mask.as_ptr(), ptr.add(offset), 16);
                    phase += 1;
                    if phase == period {
                        phase = 0;
                    }
                }
            }
        }

        // 2. Primes 11..=31: In-place vector bitwise AND
        for idx in 1..8 {
            let p = TINY_PRIMES[idx];
            let vec_masks = &self.masks[idx];
            let period = p as usize;
            let mut phase = (start_vec % (period as u64)) as usize;

            #[cfg(target_arch = "aarch64")]
            {
                use core::arch::aarch64::*;
                for offset in (0..SEGMENT_BYTES).step_by(16) {
                    let mask_ptr = vec_masks.get_unchecked(phase).as_ptr();
                    let v_data = vld1q_u8(ptr.add(offset));
                    let v_mask = vld1q_u8(mask_ptr);
                    let v_res = vandq_u8(v_data, v_mask);
                    vst1q_u8(ptr.add(offset), v_res);

                    phase += 1;
                    if phase == period {
                        phase = 0;
                    }
                }
            }

            #[cfg(not(target_arch = "aarch64"))]
            {
                for offset in (0..SEGMENT_BYTES).step_by(16) {
                    let mask = vec_masks.get_unchecked(phase);
                    for b in 0..16 {
                        *ptr.add(offset + b) &= mask[b];
                    }
                    phase += 1;
                    if phase == period {
                        phase = 0;
                    }
                }
            }
        }

        // If this is the very first segment (covering 0..491,520), restore
        // the tiny primes 7..=31 themselves
        if seg_idx == 0 {
            *ptr |= 0b1111_1110;
            *ptr.add(1) |= 0b0000_0001;
        }
    }
}

impl Default for TinyPrimeMaskTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wheel30::BIT_TO_RESIDUE;

    #[test]
    fn test_tiny_prime_table_creation() {
        let table = TinyPrimeMaskTable::new();
        for (idx, &p) in TINY_PRIMES.iter().enumerate() {
            assert_eq!(table.masks[idx].len(), p as usize);
        }
    }

    #[test]
    fn test_tiny_prime_sifting_parity() {
        let table = TinyPrimeMaskTable::new();
        let mut buf = [0xFFu8; SEGMENT_BYTES];

        unsafe {
            table.sieve_tiny_primes(&mut buf, 0);
        }

        // Check that any integer coprime to 30 divisible by any tiny prime is cleared (0)
        for byte in 0..1000 {
            for bit in 0..8 {
                let res = BIT_TO_RESIDUE[bit] as u64;
                let n = (byte as u64) * 30 + res;
                let is_marked = (buf[byte] & (1 << bit)) == 0;

                let div_tiny = TINY_PRIMES.iter().any(|&p| n >= (p as u64) * (p as u64) && n % (p as u64) == 0);
                assert_eq!(
                    is_marked, div_tiny,
                    "Mismatch for n = {} at byte {}, bit {}",
                    n, byte, bit
                );
            }
        }
    }

    #[test]
    fn test_tiny_prime_fused_exact_parity() {
        let table = TinyPrimeMaskTable::new();

        for seg_idx in [0, 1, 2, 7, 13, 100] {
            let mut buf_std = [0xFFu8; SEGMENT_BYTES];
            let mut buf_fused = [0xAAu8; SEGMENT_BYTES]; // uninitialized garbage

            unsafe {
                table.sieve_tiny_primes(&mut buf_std, seg_idx);
                table.sieve_tiny_primes_fused(&mut buf_fused, seg_idx);
            }

            assert_eq!(buf_std, buf_fused, "Fused parity mismatch at seg_idx {}", seg_idx);
        }
    }
}
