//! Phase 6.2 Step 4: Vector Popcount for Wheel-30 Sieve Engine (wheel30_popcount.rs).
//!
//! Evaluates the number of survivors (1s) in the 16 KiB buffer using
//! 32-byte 2-way dual-issue ARM64 NEON instructions (`vcntq_u8`, `vpaddlq_u8`, `vaddlvq_u16`).

use crate::wheel30::{SEGMENT_BYTES, RESIDUE_TO_BIT};

/// Computes the total number of survivor bits (1s) in the entire 16 KiB segment.
#[inline(always)]
pub unsafe fn wheel30_popcount_neon(sieve_buf: &[u8; SEGMENT_BYTES]) -> u64 {
    #[cfg(target_arch = "aarch64")]
    {
        use core::arch::aarch64::*;
        let ptr = sieve_buf.as_ptr();
        let mut acc = vdupq_n_u16(0);

        for i in (0..SEGMENT_BYTES).step_by(32) {
            let q0 = vld1q_u8(ptr.add(i));
            let q1 = vld1q_u8(ptr.add(i + 16));

            let cnt0 = vcntq_u8(q0);
            let cnt1 = vcntq_u8(q1);

            let sum0 = vpaddlq_u8(cnt0);
            let sum1 = vpaddlq_u8(cnt1);

            acc = vaddq_u16(acc, vaddq_u16(sum0, sum1));
        }

        vaddlvq_u16(acc) as u64
    }

    #[cfg(not(target_arch = "aarch64"))]
    {
        sieve_buf.iter().map(|&b| b.count_ones() as u64).sum()
    }
}

/// Computes the number of survivor bits (1s) up to integer `v` within `[low, high)`.
/// `low` must be aligned to a multiple of 30.
#[inline(always)]
pub unsafe fn wheel30_count_to(
    sieve_buf: &[u8; SEGMENT_BYTES],
    _prefix_table: Option<&[u32]>,
    low: u64,
    v: u64,
) -> u64 {
    if v < low {
        return 0;
    }
    let offset = v - low;
    let byte_limit = (offset / 30) as usize;
    if byte_limit >= SEGMENT_BYTES {
        return wheel30_popcount_neon(sieve_buf);
    }

    let ptr = sieve_buf.as_ptr();
    let mut total = 0u64;

    // Vectorized chunks of 16 bytes
    let vec_chunks = byte_limit / 16;
    #[cfg(target_arch = "aarch64")]
    {
        use core::arch::aarch64::*;
        let mut acc = vdupq_n_u16(0);
        let mut i = 0;
        while i + 2 <= vec_chunks {
            let q0 = vld1q_u8(ptr.add(i * 16));
            let q1 = vld1q_u8(ptr.add((i + 1) * 16));
            let cnt0 = vcntq_u8(q0);
            let cnt1 = vcntq_u8(q1);
            acc = vaddq_u16(acc, vaddq_u16(vpaddlq_u8(cnt0), vpaddlq_u8(cnt1)));
            i += 2;
        }
        if i < vec_chunks {
            let q = vld1q_u8(ptr.add(i * 16));
            acc = vaddq_u16(acc, vpaddlq_u8(vcntq_u8(q)));
        }
        total += vaddlvq_u16(acc) as u64;
    }

    #[cfg(not(target_arch = "aarch64"))]
    {
        for b in 0..(vec_chunks * 16) {
            total += (*ptr.add(b)).count_ones() as u64;
        }
    }

    // Scalar bytes up to byte_limit
    for b in (vec_chunks * 16)..byte_limit {
        total += (*ptr.add(b)).count_ones() as u64;
    }

    // Partial byte at byte_limit
    let res = (offset % 30) as usize;
    let bit_limit = RESIDUE_TO_BIT[res];
    if bit_limit != 0xFF {
        let mask = (1u8 << (bit_limit + 1)) - 1;
        total += ((*ptr.add(byte_limit)) & mask).count_ones() as u64;
    } else {
        // If v % 30 is not coprime to 30, count all coprime bits <= v % 30
        let mut mask = 0u8;
        for (b, &r) in crate::wheel30::BIT_TO_RESIDUE.iter().enumerate() {
            if (r as usize) <= res {
                mask |= 1 << b;
            }
        }
        total += ((*ptr.add(byte_limit)) & mask).count_ones() as u64;
    }

    total
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wheel30_popcount_empty_and_full() {
        let empty = [0u8; SEGMENT_BYTES];
        assert_eq!(unsafe { wheel30_popcount_neon(&empty) }, 0);

        let full = [0xFFu8; SEGMENT_BYTES];
        assert_eq!(unsafe { wheel30_popcount_neon(&full) }, (SEGMENT_BYTES * 8) as u64);
    }

    #[test]
    fn test_wheel30_count_to_exactness() {
        let mut buf = [0u8; SEGMENT_BYTES];
        // Set bit 0 in byte 0 (corresponds to integer low + 1)
        buf[0] = 1;
        // Set bit 1 in byte 0 (corresponds to integer low + 7)
        buf[0] |= 2;
        // Set bit 0 in byte 1 (corresponds to integer low + 31)
        buf[1] = 1;

        let low = 0u64;
        assert_eq!(unsafe { wheel30_count_to(&buf, None, low, 0) }, 0);
        assert_eq!(unsafe { wheel30_count_to(&buf, None, low, 1) }, 1);
        assert_eq!(unsafe { wheel30_count_to(&buf, None, low, 6) }, 1);
        assert_eq!(unsafe { wheel30_count_to(&buf, None, low, 7) }, 2);
        assert_eq!(unsafe { wheel30_count_to(&buf, None, low, 30) }, 2);
        assert_eq!(unsafe { wheel30_count_to(&buf, None, low, 31) }, 3);
    }
}
