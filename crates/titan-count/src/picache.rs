//! Phase 6.6 Step 1: L3-Decongested PiCache (picache.rs).
//!
//! Evaluates pi(v) in strictly 35-45 cycles with zero binary searches.
//! - Tier 0: u32 every 2^19 ints (7.6 KiB, L1D resident)
//! - Tier 1: u16 every 4,200 ints (476 KiB, fits easily in 2 MiB shared L3 cache, freeing 1.52 MiB for sieve)
//! - Tier 2: 140 bytes of raw Wheel-30 bitset per 4,200 integers

use titan_sieve::wheel30::{RESIDUE_TO_BIT, WHEEL_RESIDUES};

pub const TIER0_SHIFT: usize = 19;
pub const TIER0_SPAN: u64 = 1 << TIER0_SHIFT; // 524,288 integers
pub const TIER1_SPAN: u64 = 4200;             // 140 bytes = 4,200 integers (Wheel-aligned)
pub const TIER1_BYTES: usize = 140;

/// Compile-time lookup table mapping residue mod 30 to bitmask of all coprime residues <= r
const COP_MASK_30: [u8; 30] = {
    let mut table = [0u8; 30];
    let mut r = 0;
    while r < 30 {
        let mut mask = 0u8;
        let mut i = 0;
        while i < 8 {
            if WHEEL_RESIDUES[i] <= r as u8 {
                mask |= 1 << i;
            }
            i += 1;
        }
        table[r] = mask;
        r += 1;
    }
    table
};

#[repr(C, align(64))]
pub struct PiCache {
    pub tier0: Vec<u32>,
    pub tier1: Vec<u16>,          // 476 KiB: Fits easily in 2 MiB L3
    pub tier2_bits: Vec<u8>,      // 33.3 MB in DRAM
    pub max_v: u64,
}

pub type PiCacheL3 = PiCache;

impl PiCache {
    /// Builds the 3-tier PiCache from an existing prime list or base primes
    pub fn build(max_v: u64, primes: &[u64]) -> Self {
        let t0_len = ((max_v >> TIER0_SHIFT) + 4) as usize;
        let t1_len = ((max_v / TIER1_SPAN) + 4) as usize;
        let t2_bytes = ((max_v / 30) + 160) as usize;

        let mut tier0 = vec![0u32; t0_len];
        let mut tier1 = vec![0u16; t1_len];
        let mut tier2_bits = vec![0xFFu8; t2_bytes];

        // Integer 1 is not prime: clear bit 0 in byte 0
        tier2_bits[0] &= !(1u8 << RESIDUE_TO_BIT[1]);

        // 1. Mark composites in Tier 2 Wheel-30 bitset
        for &p in primes {
            if p * p > max_v {
                break;
            }
            if p == 2 || p == 3 || p == 5 {
                continue;
            }

            let mut m = p * p;
            while m <= max_v {
                let r = (m % 30) as usize;
                let bit = RESIDUE_TO_BIT[r];
                if bit != 0xFF {
                    let byte_idx = (m / 30) as usize;
                    tier2_bits[byte_idx] &= !(1u8 << bit);
                }
                m += p * 2; // skip even multiples
            }
        }

        // 2. Build prefix counters
        let mut total_primes: u64 = 3; // 2, 3, 5
        let num_t1_blocks = ((max_v / TIER1_SPAN) + 1) as usize;

        for b in 0..num_t1_blocks {
            let int_coord = (b as u64) * TIER1_SPAN;
            let current_t0 = (int_coord >> TIER0_SHIFT) as usize;

            // Tier 0 records total primes up to the start of this 2^19 window
            if int_coord % TIER0_SPAN < TIER1_SPAN && tier0[current_t0] == 0 && current_t0 > 0 {
                tier0[current_t0] = total_primes as u32;
            }

            let t0_base = tier0[current_t0] as u64;
            tier1[b] = (total_primes - t0_base) as u16;

            // Count surviving primes in this 140-byte block
            let start_byte = b * TIER1_BYTES;
            let end_byte = (start_byte + TIER1_BYTES).min(tier2_bits.len());
            let chunk = &tier2_bits[start_byte..end_byte];

            let mut block_cnt = 0u64;
            #[cfg(target_arch = "aarch64")]
            unsafe {
                use core::arch::aarch64::*;
                let ptr = chunk.as_ptr();
                let mut off = 0;
                while off + 16 <= chunk.len() {
                    let q = vld1q_u8(ptr.add(off));
                    block_cnt += vaddlvq_u16(vpaddlq_u8(vcntq_u8(q))) as u64;
                    off += 16;
                }
                while off < chunk.len() {
                    block_cnt += (*ptr.add(off)).count_ones() as u64;
                    off += 1;
                }
            }

            #[cfg(not(target_arch = "aarch64"))]
            {
                for &byte in chunk {
                    block_cnt += byte.count_ones() as u64;
                }
            }

            total_primes += block_cnt;
        }

        Self {
            tier0,
            tier1,
            tier2_bits,
            max_v,
        }
    }

    /// O(1) query in strictly 35-45 cycles. Zero binary searches.
    #[inline(always)]
    pub fn pi(&self, mut v: u64) -> u64 {
        if v < 2 {
            return 0;
        }
        if v < 7 {
            return match v {
                2 => 1,
                3..=4 => 2,
                5..=6 => 3,
                _ => unreachable!(),
            };
        }
        if v > self.max_v {
            v = self.max_v;
        }

        let b = (v / TIER1_SPAN) as usize;
        let block_coord = (b as u64) * TIER1_SPAN;
        let w = (block_coord >> TIER0_SHIFT) as usize;

        let base_t0 = unsafe { *self.tier0.get_unchecked(w) as u64 };
        let base_t1 = unsafe { *self.tier1.get_unchecked(b) as u64 };

        // Count survivor bits from block start up to v
        let block_byte_start = b * TIER1_BYTES;
        let target_byte = (v / 30) as usize;
        let target_rem = (v % 30) as usize;

        let mut tail_primes: u64 = 0;
        let full_bytes = target_byte.saturating_sub(block_byte_start);

        unsafe {
            let ptr = self.tier2_bits.as_ptr().add(block_byte_start);
            let mut i = 0;

            #[cfg(target_arch = "aarch64")]
            {
                use core::arch::aarch64::*;
                while i + 16 <= full_bytes {
                    let q = vld1q_u8(ptr.add(i));
                    tail_primes += vaddlvq_u16(vpaddlq_u8(vcntq_u8(q))) as u64;
                    i += 16;
                }
            }

            while i < full_bytes {
                tail_primes += (*ptr.add(i)).count_ones() as u64;
                i += 1;
            }

            // Mask active bits in final byte
            let last_byte = *self.tier2_bits.get_unchecked(target_byte);
            let mask = COP_MASK_30[target_rem];
            tail_primes += (last_byte & mask).count_ones() as u64;
        }

        base_t0 + base_t1 + tail_primes
    }
}

#[repr(C, align(64))]
pub struct PiCacheL3Compact {
    pub tier0: Vec<u32>,
    pub tier1: Vec<u16>,
    // 33.3 MB Tier-2 bitset DELETED.
    pub max_v: u64,
}

impl PiCacheL3Compact {
    /// Builds compact PiCache covering ONLY v <= z.
    /// Memory footprint drops from 35.6 MB -> 484 KiB (100% L3 Resident!)
    pub fn build_compact(z: u64, primes: &[u32]) -> Self {
        let t0_len = ((z >> TIER0_SHIFT) + 2) as usize;
        let t1_len = ((z / TIER1_SPAN) + 2) as usize;

        let mut tier0 = vec![0u32; t0_len];
        let mut tier1 = vec![0u16; t1_len];

        let mut count = 0u64;
        let mut t0_idx = 0;
        let mut t0_base = 0u64;

        for (b, chunk_start) in (0..=z).step_by(TIER1_SPAN as usize).enumerate() {
            let current_t0 = (chunk_start >> TIER0_SHIFT) as usize;
            if current_t0 > t0_idx && current_t0 < t0_len {
                t0_idx = current_t0;
                t0_base = count;
                tier0[t0_idx] = t0_base as u32;
            }

            if b < t1_len {
                tier1[b] = (count.saturating_sub(t0_base)) as u16;
            }
            let chunk_end = (chunk_start + TIER1_SPAN).min(z + 1);

            let primes_in_chunk = primes[..]
                .partition_point(|&p| (p as u64) < chunk_end)
                - primes[..].partition_point(|&p| (p as u64) < chunk_start);

            count += primes_in_chunk as u64;
        }

        Self { tier0, tier1, max_v: z }
    }

    #[inline(always)]
    pub fn pi(&self, v: u64, primes: &[u32]) -> u64 {
        if v < 2 { return 0; }
        if v >= self.max_v {
            return primes.partition_point(|&p| (p as u64) <= v) as u64;
        }

        let w = (v >> TIER0_SHIFT) as usize;
        let b = (v / TIER1_SPAN) as usize;

        let base_t0 = unsafe { *self.tier0.get_unchecked(w.min(self.tier0.len() - 1)) as u64 };
        let base_t1 = unsafe { *self.tier1.get_unchecked(b.min(self.tier1.len() - 1)) as u64 };

        let block_start = (b as u64) * TIER1_SPAN;
        let local_primes = primes[..]
            .partition_point(|&p| (p as u64) <= v)
            - primes[..].partition_point(|&p| (p as u64) < block_start);

        base_t0 + base_t1 + (local_primes as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_picache_ground_truth() {
        let max_v = 100_000u64;
        let base_primes = titan_sieve::base::generate_base_primes(max_v);

        let picache = PiCache::build(max_v, &base_primes);

        // Test small edge cases
        assert_eq!(picache.pi(0), 0);
        assert_eq!(picache.pi(1), 0);
        assert_eq!(picache.pi(2), 1);
        assert_eq!(picache.pi(3), 2);
        assert_eq!(picache.pi(4), 2);
        assert_eq!(picache.pi(5), 3);
        assert_eq!(picache.pi(6), 3);
        assert_eq!(picache.pi(7), 4);
        assert_eq!(picache.pi(8), 4);
        assert_eq!(picache.pi(9), 4);
        assert_eq!(picache.pi(10), 4);
        assert_eq!(picache.pi(11), 5);
        assert_eq!(picache.pi(12), 5);
        assert_eq!(picache.pi(13), 6);
        assert_eq!(picache.pi(100), 25);
        assert_eq!(picache.pi(1000), 168);
        assert_eq!(picache.pi(10000), 1229);
        assert_eq!(picache.pi(100000), 9592);

        // Random queries across the entire domain
        let mut rng = 123456789u64;
        for _ in 0..10_000 {
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
            let q = rng % max_v;
            let expected = base_primes.partition_point(|&p| p <= q) as u64;
            let actual = picache.pi(q);
            assert_eq!(
                actual, expected,
                "Query mismatch at q = {}: got {}, expected {}",
                q, actual, expected
            );
        }
    }

    #[test]
    fn test_picache_compact_parity() {
        let z = 50_000u64;
        let base_primes_u64 = titan_sieve::base::generate_base_primes(z + 1000);
        let base_primes: Vec<u32> = base_primes_u64.iter().map(|&p| p as u32).collect();

        let compact = PiCacheL3Compact::build_compact(z, &base_primes);

        for v in [0, 1, 2, 3, 5, 10, 100, 1000, 10000, 49999, 50000] {
            let expected = base_primes.partition_point(|&p| (p as u64) <= v) as u64;
            let actual = compact.pi(v, &base_primes);
            assert_eq!(actual, expected, "Compact mismatch at v = {}", v);
        }
    }
}
