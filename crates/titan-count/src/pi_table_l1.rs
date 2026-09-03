//! Phase 7.1: Cache-Locked Two-Level SegmentedPiTable (pi_table_l1.rs).
//!
//! Designed to fit strictly within Cortex-A55 32 KiB L1D cache.
//! - Tier 1: Coarse Counter (u32 every 2,048 integers)
//! - Tier 2: Packed Wheel-30 prime bitmask (16,016 bytes per 480,480 segment)
//! - Zero pipeline stalls, fast vector/popcnt resolution.

use titan_sieve::wheel30::{RESIDUE_TO_BIT, WHEEL_RESIDUES};

pub const COARSE_STRIDE: usize = 2048;

#[derive(Clone)]
pub struct PiTableL1 {
    pub low: u64,
    pub high: u64,
    pub base_pi: u64,
    pub coarse: Vec<u32>,
    pub bitmask: Vec<u8>,
}

impl PiTableL1 {
    pub fn new(low: u64, high: u64, base_pi: u64, bitmask: &[u8]) -> Self {
        let span = (high - low) as usize;
        let num_coarse = (span + COARSE_STRIDE - 1) / COARSE_STRIDE + 2;
        let mut coarse = Vec::with_capacity(num_coarse);

        // Precompute coarse prefix sums every 2048 integers
        let mut running_sum = 0u32;
        let mut curr_int = 0usize;

        while curr_int <= span {
            coarse.push(running_sum);
            let next_int = (curr_int + COARSE_STRIDE).min(span);
            let byte_start = curr_int / 30;
            let byte_end = next_int / 30;

            let mut block_cnt = 0u32;
            for b in byte_start..byte_end.min(bitmask.len()) {
                block_cnt += bitmask[b].count_ones();
            }
            running_sum += block_cnt;
            curr_int = next_int;
            if curr_int >= span {
                coarse.push(running_sum);
                break;
            }
        }

        Self {
            low,
            high,
            base_pi,
            coarse,
            bitmask: bitmask.to_vec(),
        }
    }

    /// Evaluates pi(v) for low <= v < high in O(1) L1D-locked memory
    #[inline(always)]
    pub fn pi(&self, v: u64) -> u64 {
        if v < self.low {
            return 0;
        }
        if v >= self.high {
            return self.base_pi + (*self.coarse.last().unwrap_or(&0) as u64);
        }

        let offset = (v - self.low) as usize;
        let coarse_idx = offset / COARSE_STRIDE;
        let base = self.base_pi + (self.coarse[coarse_idx] as u64);

        let coarse_int = coarse_idx * COARSE_STRIDE;
        let byte_start = coarse_int / 30;
        let byte_target = offset / 30;
        let rem_target = offset % 30;

        let mut fine_cnt = 0u64;

        #[cfg(target_arch = "aarch64")]
        unsafe {
            use core::arch::aarch64::*;
            let len = byte_target.saturating_sub(byte_start);
            let full_16 = len & !15;
            let ptr = self.bitmask.as_ptr().add(byte_start);

            for i in (0..full_16).step_by(16) {
                let q = vld1q_u8(ptr.add(i));
                fine_cnt += vaddlvq_u16(vpaddlq_u8(vcntq_u8(q))) as u64;
            }

            for i in full_16..len {
                fine_cnt += (*ptr.add(i)).count_ones() as u64;
            }
        }

        #[cfg(not(target_arch = "aarch64"))]
        {
            for b in byte_start..byte_target {
                fine_cnt += self.bitmask[b].count_ones() as u64;
            }
        }

        // Mask final residue byte
        if byte_target < self.bitmask.len() {
            let last_byte = self.bitmask[byte_target];
            let bit_limit = RESIDUE_TO_BIT[rem_target];
            let mask = if bit_limit == 0xFF {
                let mut m = 0u8;
                for (idx, &res) in WHEEL_RESIDUES.iter().enumerate() {
                    if (res as usize) <= rem_target {
                        m |= 1 << idx;
                    }
                }
                m
            } else {
                (1u8 << (bit_limit + 1)).wrapping_sub(1)
            };
            fine_cnt += (last_byte & mask).count_ones() as u64;
        }

        base + fine_cnt
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pi_table_l1_basic() {
        let bitmask = vec![0xFFu8; 16016]; // all prime candidates set
        let table = PiTableL1::new(1000, 1000 + 480480, 168, &bitmask);

        let p1 = table.pi(1000);
        assert!(p1 >= 168);

        let p_mid = table.pi(1000 + 2048);
        assert!(p_mid > p1);
    }
}
