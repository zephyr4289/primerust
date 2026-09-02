//! Phase 40: ARM64 NEON L1 Flat Popcount (L1FlatPopcount).
//!
//! Replaces Binary Indexed Trees with a flat L1D-locked prefix table,
//! achieving true O(1) prefix queries using hardware NEON `cnt` instructions.

#[cfg(target_arch = "aarch64")]
use std::arch::aarch64::*;

pub const NUM_WORDS_16K: usize = 2048; // 16 KiB segment = 2,048 u64 words
pub const NUM_BLOCKS_64B: usize = 256;  // 16 KiB / 64B = 256 blocks

#[repr(C, align(64))]
pub struct L1FlatPopcount {
    /// Cumulative sum at every 64-byte (512-bit) block boundary = 1 KiB (fits in L1D)
    pub block_prefix: [u32; NUM_BLOCKS_64B],
    pub total_count: u32,
}

impl L1FlatPopcount {
    pub fn new() -> Self {
        Self {
            block_prefix: [0u32; NUM_BLOCKS_64B],
            total_count: 0,
        }
    }

    /// Build prefix index across a 16 KiB segment using true ARM64 NEON vectorization
    #[inline(always)]
    pub unsafe fn build(&mut self, segment: &[u64; NUM_WORDS_16K]) {
        let mut running_sum: u32 = 0;
        let ptr = segment.as_ptr() as *const u8;

        for i in 0..NUM_BLOCKS_64B {
            self.block_prefix[i] = running_sum;

            #[cfg(target_arch = "aarch64")]
            {
                // Load 64 bytes (4 x 128-bit NEON registers)
                let q0 = vld1q_u8(ptr.add(i * 64));
                let q1 = vld1q_u8(ptr.add(i * 64 + 16));
                let q2 = vld1q_u8(ptr.add(i * 64 + 32));
                let q3 = vld1q_u8(ptr.add(i * 64 + 48));

                // Hardware bit-count instruction (cnt)
                let c0 = vcntq_u8(q0);
                let c1 = vcntq_u8(q1);
                let c2 = vcntq_u8(q2);
                let c3 = vcntq_u8(q3);

                // Sum across vectors
                let s01 = vaddq_u8(c0, c1);
                let s23 = vaddq_u8(c2, c3);
                let s0123 = vaddq_u8(s01, s23);

                // Pairwise lengthen to u16 to avoid 8-bit overflow
                let u16_vec = vpaddlq_u8(s0123);
                let block_sum = vaddvq_u16(u16_vec) as u32;

                running_sum += block_sum;
            }

            #[cfg(not(target_arch = "aarch64"))]
            {
                let start_w = i * 8;
                let mut block_sum: u32 = 0;
                for w in 0..8 {
                    block_sum += segment[start_w + w].count_ones() as u32;
                }
                running_sum += block_sum;
            }
        }
        self.total_count = running_sum;
    }

    /// O(1) Prefix Count up to bit offset k within 16 KiB segment
    #[inline(always)]
    pub fn count_to(&self, segment: &[u64; NUM_WORDS_16K], k: usize) -> u32 {
        let word_idx = k >> 6;
        if word_idx >= NUM_WORDS_16K {
            return self.total_count;
        }

        let bit_idx = (k & 63) as u32;
        let block_idx = word_idx >> 3; // 8 words per 64-byte block

        let mut count = self.block_prefix[block_idx];

        // Sum complete words within the block (0 to 7 words, fully unrolled)
        let block_start_word = block_idx << 3;
        for w in block_start_word..word_idx {
            count += segment[w].count_ones();
        }

        // Mask remaining bits in current word
        if bit_idx > 0 {
            let mask = (1u64 << bit_idx) - 1;
            count += (segment[word_idx] & mask).count_ones();
        }

        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_l1_flat_popcount_exact() {
        let mut segment = [0u64; NUM_WORDS_16K];
        for i in 0..NUM_WORDS_16K {
            segment[i] = 0xAA_55_AA_55_AA_55_AA_55; // 32 bits set per word
        }

        let mut popcount = L1FlatPopcount::new();
        unsafe {
            popcount.build(&segment);
        }

        // Check total count
        let total = popcount.count_to(&segment, NUM_WORDS_16K * 64);
        assert_eq!(total, (NUM_WORDS_16K * 32) as u32);

        // Check prefix at 1000 bits
        let p1000 = popcount.count_to(&segment, 1000);
        let mut naive = 0u32;
        for bit in 0..1000 {
            let w = bit / 64;
            let b = bit % 64;
            if (segment[w] & (1 << b)) != 0 {
                naive += 1;
            }
        }
        assert_eq!(p1000, naive);
    }
}
