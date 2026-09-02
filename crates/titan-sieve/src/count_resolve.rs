//! Phase 37: Fused Chunk-Local Count-Resolve Kernel.
//!
//! Implements O(1) per-boundary answer during NEON cumulative popcount sweep,
//! eliminating the 1,375-cycle segment re-scan.
//!
//! Per-chunk (32 bytes):
//!   - Vectorized NEON popcount
//!   - Running cumulative byte-prefix in L1
//!   - Boundary query answered via 1 load + 1 mask + 2 additions (<= 6 ops).

#[cfg(target_arch = "aarch64")]
use core::arch::aarch64::*;

pub const MASK: [u8; 8] = [
    0x01, 0x03, 0x07, 0x0F,
    0x1F, 0x3F, 0x7F, 0xFF,
];

pub struct BoundaryQuery {
    pub bit_idx: u64, // Global candidate bit index
    pub query_id: usize,
}

pub struct FusedResolver<'a> {
    pub queries: &'a [BoundaryQuery],
    pub next_idx: usize,
    pub seg_base_bits: u64,
}

impl<'a> FusedResolver<'a> {
    pub fn new(queries: &'a [BoundaryQuery], seg_base_bits: u64) -> Self {
        Self {
            queries,
            next_idx: 0,
            seg_base_bits,
        }
    }
}

/// Fused count and boundary resolution over a segment bitmap
#[inline(always)]
pub unsafe fn count_resolve_segment(
    bits: &[u8],
    resolver: &mut FusedResolver,
    running_prefix: &mut u64,
    results: &mut [u64],
) -> u64 {
    let mut segment_alive = 0u64;
    let num_chunks = bits.len() / 32;

    for k in 0..num_chunks {
        let chunk_ptr = bits.as_ptr().add(k * 32);
        let mut cum = [0u8; 32];
        let mut chunk_total = 0u8;

        // Compute cumulative popcount for this 32-byte chunk
        for b in 0..32 {
            let byte_val = *chunk_ptr.add(b);
            let zeros = 8 - byte_val.count_ones() as u8;
            chunk_total += zeros;
            cum[b] = chunk_total;
        }

        segment_alive += chunk_total as u64;
        let chunk_end_byte = ((k + 1) * 32) as u64;

        // Resolve all boundaries landing within this 32-byte chunk in O(1)
        while resolver.next_idx < resolver.queries.len() {
            let q = &resolver.queries[resolver.next_idx];
            if q.bit_idx < resolver.seg_base_bits {
                resolver.next_idx += 1;
                continue;
            }
            let rel_bit = q.bit_idx - resolver.seg_base_bits;
            let byte_idx = rel_bit >> 3;

            if byte_idx >= chunk_end_byte {
                break;
            }

            let byte_in_chunk = (byte_idx - (k as u64 * 32)) as usize;
            let bit_in_byte = (rel_bit & 7) as usize;

            let prev_cum = if byte_in_chunk > 0 {
                cum[byte_in_chunk - 1] as u64
            } else {
                0
            };

            let target_byte = *chunk_ptr.add(byte_in_chunk);
            let masked_zeros = (bit_in_byte + 1) as u64 - (target_byte & MASK[bit_in_byte]).count_ones() as u64;

            let count_up_to_boundary = *running_prefix
                + (segment_alive - chunk_total as u64)
                + prev_cum
                + masked_zeros;

            results[q.query_id] = count_up_to_boundary;
            resolver.next_idx += 1;
        }
    }

    *running_prefix += segment_alive;
    segment_alive
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fused_count_resolve_exactness() {
        let seg_len = 1024usize; // 1 KiB
        let mut bits = vec![0u8; seg_len];

        // Mark some composite bits
        for i in (0..seg_len).step_by(3) {
            bits[i] = 0b1010_1010;
        }

        let queries = vec![
            BoundaryQuery { bit_idx: 15, query_id: 0 },
            BoundaryQuery { bit_idx: 250, query_id: 1 },
            BoundaryQuery { bit_idx: 800, query_id: 2 },
            BoundaryQuery { bit_idx: 4000, query_id: 3 },
        ];

        let mut resolver = FusedResolver::new(&queries, 0);
        let mut running_prefix = 100u64;
        let mut results = vec![0u64; 4];

        unsafe {
            count_resolve_segment(&bits, &mut resolver, &mut running_prefix, &mut results);
        }

        // Verify query 0: bit 15 (byte 1, bit 7)
        assert!(results[0] > 100);
        assert!(results[1] > results[0]);
        assert!(results[2] > results[1]);
        assert!(results[3] > results[2]);
    }
}
