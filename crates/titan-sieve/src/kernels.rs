//! Phase 30: 8-Unrolled Wheel-30 Sieve Marking Kernel.
//!
//! Evaluates candidate-multiples of prime p in 1 byte per 30 integers format.
//! Fully branch-free interior loop for Cortex-A55 in-order pipeline.

use titan_core::wheel::{cand_idx, GAP30, R30, SKIP, UNITS};

/// Marks candidate-multiples of p in one segment bitmap.
/// 1 byte per 30 integers: bit s of byte B <-> integer 30 * (seg_lo/30 + B) + UNITS[s].
///
/// SAFETY: bits.len() * 8 == nbits; i0 < nbits; d == &WHEEL_ROT[r][slot] for prime p.
#[inline(always)]
pub unsafe fn mark_wheel8(bits: &mut [u8], p: u64, i0: u32, d: &[u32; 8]) {
    let nbits = (bits.len() * 8) as u32;
    let stop = nbits.saturating_sub((8 * p) as u32);
    let (d0, d1, d2, d3) = (d[0], d[1], d[2], d[3]);
    let (d4, d5, d6, d7) = (d[4], d[5], d[6], d[7]);
    let base = bits.as_mut_ptr();
    let mut i = i0;

    // 8-unrolled branch-free main loop
    while i < stop {
        *base.add((i >> 3) as usize) |= 1 << (i & 7);
        i += d0;
        *base.add((i >> 3) as usize) |= 1 << (i & 7);
        i += d1;
        *base.add((i >> 3) as usize) |= 1 << (i & 7);
        i += d2;
        *base.add((i >> 3) as usize) |= 1 << (i & 7);
        i += d3;
        *base.add((i >> 3) as usize) |= 1 << (i & 7);
        i += d4;
        *base.add((i >> 3) as usize) |= 1 << (i & 7);
        i += d5;
        *base.add((i >> 3) as usize) |= 1 << (i & 7);
        i += d6;
        *base.add((i >> 3) as usize) |= 1 << (i & 7);
        i += d7;
    }

    // D2 fix: tail loop rotates the delta cycle
    let mut j = 0usize;
    while i < nbits {
        *base.add((i >> 3) as usize) |= 1 << (i & 7);
        i += d[j & 7];
        j += 1;
    }
}

/// First candidate-multiple of p at/after seg_lo (seg_lo == 0 mod 30).
#[inline(always)]
pub fn first_mark(p: u64, seg_lo: u64) -> (u32, usize, usize) {
    let k = (seg_lo + p - 1) / p; // ceil
    let k = k + SKIP[(k % 30) as usize] as u64; // first unit cofactor
    let m0 = p * k;
    debug_assert!(m0 >= seg_lo && R30[(m0 % 30) as usize] != 8);
    let i0 = (cand_idx(m0) - 8 * (seg_lo / 30)) as u32; // bit index in segment
    let (r, s) = (R30[(p % 30) as usize] as usize, R30[(k % 30) as usize] as usize);
    (i0, r, s)
}

#[inline(always)]
pub fn compute_wheel_deltas_for_prime(p: u64, start_k_slot: usize) -> [u32; 8] {
    let mut d = [0u32; 8];
    let mut k = UNITS[start_k_slot] as u64;
    let mut m = p * k;
    for j in 0..8 {
        let g = GAP30[R30[(k % 30) as usize] as usize] as u64;
        let next_m = m + p * g;
        d[j] = (cand_idx(next_m) - cand_idx(m)) as u32;
        k += g;
        m = next_m;
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mark_wheel8_against_naive_modulo() {
        let seg_bytes = 4096; // 4 KiB segment
        let seg_lo = 3000u64;
        let mut bits = vec![0u8; seg_bytes];

        let primes = [7u64, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47];

        for &p in &primes {
            let (i0, _r, s) = first_mark(p, seg_lo);
            let d = compute_wheel_deltas_for_prime(p, s);
            unsafe {
                mark_wheel8(&mut bits, p, i0, &d);
            }
        }

        // Verify marked bits match multiples
        for byte_idx in 0..seg_bytes {
            for bit_idx in 0..8 {
                let n = seg_lo + 30 * (byte_idx as u64) + UNITS[bit_idx] as u64;
                let is_marked = (bits[byte_idx] & (1 << bit_idx)) != 0;

                let has_prime_factor = primes.iter().any(|&p| n % p == 0);
                assert_eq!(
                    is_marked, has_prime_factor,
                    "Mismatch at integer {}: marked={}, expected multiple={}",
                    n, is_marked, has_prime_factor
                );
            }
        }
    }
}
