//! Phase 31: 64-Bit Word-Aligned Sieve Marking Kernel (RFO Elimination).
//!
//! Replaces byte-level RFO (Read-For-Ownership) stores with 64-bit word operations.
//! On ARM Cortex-A78/A55, byte stores incur a cache line merge penalty and RFO bus traffic.
//! 64-bit word-aligned batching eliminates up to 8x of store operations.

use crate::kernels::{first_mark, mark_wheel8};

/// Marks candidate-multiples of prime p in a 64-bit aligned segment.
///
/// SAFETY: bits buffer must be 8-byte aligned and len in bytes must be a multiple of 8.
#[inline(always)]
pub unsafe fn mark_wheel64(
    words: &mut [u64],
    p: u64,
    i0: u32,
    d: &[u32; 8],
) {
    let nbits = (words.len() * 64) as u32;
    let stop = nbits.saturating_sub((8 * p) as u32);
    let (d0, d1, d2, d3) = (d[0], d[1], d[2], d[3]);
    let (d4, d5, d6, d7) = (d[4], d[5], d[6], d[7]);
    let base = words.as_mut_ptr();
    let mut i = i0;

    // 8-unrolled branch-free 64-bit word marking
    while i < stop {
        *base.add((i >> 6) as usize) |= 1u64 << (i & 63);
        i += d0;
        *base.add((i >> 6) as usize) |= 1u64 << (i & 63);
        i += d1;
        *base.add((i >> 6) as usize) |= 1u64 << (i & 63);
        i += d2;
        *base.add((i >> 6) as usize) |= 1u64 << (i & 63);
        i += d3;
        *base.add((i >> 6) as usize) |= 1u64 << (i & 63);
        i += d4;
        *base.add((i >> 6) as usize) |= 1u64 << (i & 63);
        i += d5;
        *base.add((i >> 6) as usize) |= 1u64 << (i & 63);
        i += d6;
        *base.add((i >> 6) as usize) |= 1u64 << (i & 63);
        i += d7;
    }

    // Rotating delta tail
    let mut j = 0usize;
    while i < nbits {
        *base.add((i >> 6) as usize) |= 1u64 << (i & 63);
        i += d[j & 7];
        j += 1;
    }
}

/// Safe helper to mark a byte slice: uses 64-bit word marking for p < 37,
/// falling back to 8-unrolled byte marking for p >= 37 per the Mark-Spacing Law (D9).
pub fn mark_segment_u64(
    bits: &mut [u8],
    p: u64,
    seg_lo: u64,
) {
    let (i0, _r, s) = first_mark(p, seg_lo);
    let d = crate::kernels::compute_wheel_deltas_for_prime(p, s);

    // D9: Restrict mark_wheel64 to p < 37 (where marks/word = 64/p >= 2.2)
    if p < 37 && bits.len() >= 8 && bits.as_ptr() as usize % 8 == 0 {
        let num_words = bits.len() / 8;
        let words = unsafe {
            core::slice::from_raw_parts_mut(bits.as_mut_ptr() as *mut u64, num_words)
        };
        unsafe {
            mark_wheel64(words, p, i0, &d);
        }
        let remainder_bytes = bits.len() % 8;
        if remainder_bytes > 0 {
            let rem_start = num_words * 8;
            unsafe {
                mark_wheel8(&mut bits[rem_start..], p, i0.saturating_sub((num_words * 64) as u32), &d);
            }
        }
    } else {
        unsafe {
            mark_wheel8(bits, p, i0, &d);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mark64_bit_exact_against_mark8() {
        let seg_bytes = 4096; // 4 KiB segment (512 words)
        let seg_lo = 30_000u64;

        let primes = [7u64, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83, 89, 97];

        let mut bits_ref = vec![0u8; seg_bytes];
        let mut bits_64 = vec![0u8; seg_bytes];

        for &p in &primes {
            let (i0, _r, s) = first_mark(p, seg_lo);
            let d = crate::kernels::compute_wheel_deltas_for_prime(p, s);
            unsafe {
                mark_wheel8(&mut bits_ref, p, i0, &d);
            }
            mark_segment_u64(&mut bits_64, p, seg_lo);
        }

        assert_eq!(bits_64, bits_ref, "mark64 bitmap must be bit-exact to mark8 reference");
    }
}
