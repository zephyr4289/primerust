//! Phase 30: Fused In-Flight Popcount & Boundary Resolution (NEON-Accelerated).
//!
//! Evaluates segment popcounts in a single DRAM streaming pass, resolving
//! boundary values pi(floor(x / p)) in-flight in O(1) amortized time per boundary.

#[cfg(target_arch = "aarch64")]
use core::arch::aarch64::*;

#[inline(always)]
#[cfg(target_arch = "aarch64")]
unsafe fn pop16(p: *const u8) -> u32 {
    vaddvq_u8(vcntq_u8(vld1q_u8(p))) as u32
}

#[inline(always)]
#[cfg(not(target_arch = "aarch64"))]
unsafe fn pop16(p: *const u8) -> u32 {
    let mut sum = 0u32;
    for i in 0..16 {
        sum += (*p.add(i)).count_ones();
    }
    sum
}

#[derive(Clone, Copy, Debug)]
pub struct BndItem {
    pub p_idx: u64,
    pub c_global: u64, // sorted by c_global ascending
}

pub struct Bnd<'a> {
    pub items: &'a [BndItem],
    pub next: usize,
    pub seg_base: u64, // cand_idx(seg_lo) = 8 * seg_lo / 30
    pub i0: u64,
    pub i1: u64,       // global pi-index range of this slice
}

impl<'a> Bnd<'a> {
    pub fn new(items: &'a [BndItem], seg_base: u64, i0: u64, i1: u64) -> Self {
        Self {
            items,
            next: 0,
            seg_base,
            i0,
            i1,
        }
    }
}

/// Single fused pass over one segment: returns full-segment coprime count,
/// adds pi(floor(x / p)) of every boundary landing inside to pi_sum.
///
/// Note: bits is a COMPOSITE bitmap (1 = composite, 0 = prime candidate).
/// Thus, prime candidates are counted by counting zeros (!bits).
pub unsafe fn count_resolve(
    bits: &[u8],
    seg_prefix: u64,
    b: &mut Bnd,
    pi_sum: &mut u64,
) -> u32 {
    let mut primes_in_seg = 0u32;
    let mut k32 = 0usize;
    let chunks = bits.chunks_exact(32);

    for c in chunks {
        let chunk_offset = k32 * 32;

        // Resolve boundaries whose byte lies in [chunk_offset, chunk_offset + 32)
        while b.next < b.items.len() {
            let it = &b.items[b.next];
            if it.c_global < b.seg_base {
                b.next += 1;
                continue;
            }
            let i = (it.c_global - b.seg_base) as u32;
            let byte = (i >> 3) as usize;
            if byte >= chunk_offset + 32 {
                break;
            }
            b.next += 1;

            // Partial zeros count in bytes [chunk_offset, byte)
            let mut part_zeros = 0u32;
            let mut t = chunk_offset;
            while t + 16 <= byte {
                let ones = pop16(bits.as_ptr().add(t));
                part_zeros += 128 - ones;
                t += 16;
            }
            while t < byte {
                part_zeros += (bits[t].count_zeros()) as u32;
                t += 1;
            }

            // In the boundary byte, count zeros in bits <= (i & 7)
            let bit_in_byte = (i & 7) as u8;
            let mask = ((1u16 << (bit_in_byte + 1)) - 1) as u8;
            let target_byte = bits[byte];
            let byte_zeros = (!target_byte & mask).count_ones() as u64;

            let total_pi = seg_prefix + primes_in_seg as u64 + part_zeros as u64 + byte_zeros;
            *pi_sum = pi_sum.wrapping_add(total_pi);
        }

        // Count prime candidates (zeros) in the 32-byte chunk
        let ones = pop16(c.as_ptr()) + pop16(c.as_ptr().add(16));
        let zeros = 256 - ones;
        primes_in_seg += zeros;
        k32 += 1;
    }

    let remainder_offset = k32 * 32;
    for (rem_idx, &byte_val) in bits[remainder_offset..].iter().enumerate() {
        let current_byte = remainder_offset + rem_idx;

        while b.next < b.items.len() {
            let it = &b.items[b.next];
            if it.c_global < b.seg_base {
                b.next += 1;
                continue;
            }
            let i = (it.c_global - b.seg_base) as u32;
            let byte = (i >> 3) as usize;
            if byte > current_byte {
                break;
            }
            b.next += 1;

            let bit_in_byte = (i & 7) as u8;
            let mask = ((1u16 << (bit_in_byte + 1)) - 1) as u8;
            let byte_zeros = (!byte_val & mask).count_ones() as u64;

            let total_pi = seg_prefix + primes_in_seg as u64 + byte_zeros;
            *pi_sum = pi_sum.wrapping_add(total_pi);
        }

        primes_in_seg += byte_val.count_zeros() as u32;
    }

    primes_in_seg
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_resolve_basic() {
        let mut bits = vec![0u8; 64]; // 64 bytes all zeros (512 candidate primes)
        // Mark byte 0 bit 0 as composite
        bits[0] = 1;

        let bnd_items = [
            BndItem { p_idx: 1, c_global: 0 },
            BndItem { p_idx: 2, c_global: 8 }, // byte 1 bit 0
        ];
        let mut b = Bnd::new(&bnd_items, 0, 1, 3);
        let mut pi_sum = 0u64;

        let cnt = unsafe { count_resolve(&bits, 100, &mut b, &mut pi_sum) };
        assert_eq!(cnt, 511); // 512 - 1 composite = 511
        // boundary at bit 0: zero prime bits <= 0 in byte 0 (since bit 0 is 1) -> 100 + 0 = 100
        // boundary at bit 8: byte 0 has 7 zeros + byte 1 bit 0 has 1 zero = 8 -> 100 + 8 = 108
        assert_eq!(pi_sum, 100 + 108);
    }
}
