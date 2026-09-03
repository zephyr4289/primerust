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

const WHEEL30_RESIDUES: [u8; 8] = [1, 7, 11, 13, 17, 19, 23, 29];

const fn build_wheel210_template() -> [u8; 7] {
    let mut template = [0xFFu8; 7];
    let mut b = 0;
    while b < 7 {
        let mut bit = 0;
        while bit < 8 {
            let r = WHEEL30_RESIDUES[bit] as usize;
            if (b * 30 + r) % 7 == 0 {
                template[b] &= !(1u8 << bit);
            }
            bit += 1;
        }
        b += 1;
    }
    template
}

const WHEEL210_BASE_TEMPLATE: [u8; 7] = build_wheel210_template();

/// Initializes the 16,016-byte segment directly with Prime 7 pre-marked.
/// Eliminates memset(0xFF) and skips Prime 7 scalar sieving entirely.
#[inline(always)]
pub unsafe fn init_segment_fused_wheel210(dst: *mut u8, seg_low: u64) {
    let byte_offset = ((seg_low / 30) % 7) as usize;

    let mut rotated = [0u8; 7];
    for i in 0..7 {
        rotated[i] = WHEEL210_BASE_TEMPLATE[(i + byte_offset) % 7];
    }

    // Expand 7 bytes to 112 bytes (exactly 7 x 16-byte vectors)
    let mut block112 = [0u8; 112];
    for i in 0..112 {
        block112[i] = rotated[i % 7];
    }

    #[cfg(target_arch = "aarch64")]
    {
        use core::arch::aarch64::*;
        let v0 = vld1q_u8(block112.as_ptr());
        let v1 = vld1q_u8(block112.as_ptr().add(16));
        let v2 = vld1q_u8(block112.as_ptr().add(32));
        let v3 = vld1q_u8(block112.as_ptr().add(48));
        let v4 = vld1q_u8(block112.as_ptr().add(64));
        let v5 = vld1q_u8(block112.as_ptr().add(80));
        let v6 = vld1q_u8(block112.as_ptr().add(96));

        let mut ptr = dst;
        for _ in 0..143 {
            vst1q_u8(ptr, v0);
            vst1q_u8(ptr.add(16), v1);
            vst1q_u8(ptr.add(32), v2);
            vst1q_u8(ptr.add(48), v3);
            vst1q_u8(ptr.add(64), v4);
            vst1q_u8(ptr.add(80), v5);
            vst1q_u8(ptr.add(96), v6);
            ptr = ptr.add(112);
        }
    }

    #[cfg(not(target_arch = "aarch64"))]
    {
        let mut ptr = dst;
        for _ in 0..143 {
            core::ptr::copy_nonoverlapping(block112.as_ptr(), ptr, 112);
            ptr = ptr.add(112);
        }
    }

    // If this is segment covering 0, preserve prime 7 itself
    if seg_low == 0 {
        *dst |= 1u8 << 1; // Bit 1 is residue 7
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

    #[test]
    fn test_fused_wheel210_init() {
        for seg_idx in [0u64, 1, 2, 7, 13, 21, 100] {
            let seg_low = seg_idx * 491_520;
            let mut buf = [0xAAu8; SEGMENT_BYTES];
            unsafe {
                init_segment_fused_wheel210(buf.as_mut_ptr(), seg_low);
            }

            for byte in 0..1000 {
                for bit in 0..8 {
                    let res = BIT_TO_RESIDUE[bit] as u64;
                    let n = seg_low + (byte as u64) * 30 + res;
                    let is_marked = (buf[byte] & (1 << bit)) == 0;
                    let should_be_marked = (n % 7 == 0) && (n > 7 || seg_low > 0);
                    assert_eq!(
                        is_marked, should_be_marked,
                        "Mismatch for n = {} (seg_idx {}) at byte {}, bit {}: is_marked={}, should={}",
                        n, seg_idx, byte, bit, is_marked, should_be_marked
                    );
                }
            }
        }
    }
}
