//! PiTable: Ultra-fast O(1) prime counting table using Wheel-30 bits and block prefix sums.
//!
//! Representation:
//!   - Wheel-30 bit-array where 1 indicates a prime candidate
//!   - u32 prefix sums per 64-byte block (covers 1,920 numbers per block)
//!   - Lookup: block_prefix + word_popcounts + intra-byte residue mask

use titan_core::wheel::RESIDUES;
use titan_sieve::arena::SieveArena;
const BLOCK_BYTES: usize = 64;

pub struct PiTable {
    pub max_y: u64,
    pub bytes: Vec<u8>,
    pub prefix_counts: Vec<u32>,
}

impl PiTable {
    pub fn new(max_y: u64) -> Self {
        let max_y = max_y.max(30);
        let num_bytes = ((max_y / 30) + 2) as usize;
        let num_blocks = (num_bytes + BLOCK_BYTES - 1) / BLOCK_BYTES;
        let mut bytes = vec![0u8; num_blocks * BLOCK_BYTES];

        let seg_sz = 65536.min(bytes.len()).max(256);
        let mut arena = SieveArena::new(max_y, seg_sz);
        arena.reset();

        let s = seg_sz;
        let seg_span = (s as u64) * 30;
        let num_segments = ((max_y / seg_span) + 1) as usize;

        let mut byte_offset = 0;

        for seg_idx in 0..num_segments {
            let seg_low = (seg_idx as u64) * seg_span;
            let seg_high = seg_low + seg_span - 1;

            arena.presieve.init_segment(seg_idx, &mut arena.segment_buf);

            while arena.base_frontier_idx < arena.base_primes.len() {
                let p = arena.base_primes[arena.base_frontier_idx];
                let p2 = p * p;
                if p2 > seg_high {
                    break;
                }
                let p2_byte = (p2 / 30) - (seg_low / 30);
                let p_res_bit = titan_core::wheel::RESIDUE_TO_BIT[(p % 30) as usize];
                if p <= arena.small_threshold {
                    arena.small_primes.push(titan_sieve::erat_small::SmallPrime::new(p, p2_byte as usize, p_res_bit));
                } else if p <= arena.medium_threshold || arena.bucket_ring.is_none() {
                    arena.medium_primes.push(titan_sieve::erat_medium::MediumPrime::new(p, p2_byte as usize, p_res_bit));
                } else {
                    let ring = arena.bucket_ring.as_mut().unwrap();
                    let w = ring.window_size;
                    let cur_slot = seg_idx % w;
                    let p2_bit = titan_core::wheel::WHEEL_NEXT[p_res_bit as usize][p_res_bit as usize];
                    let entry = titan_sieve::erat_big::BucketEntry::pack(p as u32, p2_byte as u32, p_res_bit, p_res_bit, p2_bit, 0);
                    ring.push_ring(cur_slot, entry);
                }
                arena.base_frontier_idx += 1;
            }

            if seg_idx == 0 {
                arena.segment_buf[0] &= !(1 << 0);
                for &p in &[7u64, 11, 13] {
                    if p <= max_y {
                        let (b, bit) = titan_core::wheel::number_to_slot(p).unwrap();
                        arena.segment_buf[b] |= 1 << bit;
                    }
                }
                for &p in &arena.base_primes {
                    if p <= seg_high && p <= max_y {
                        let (b, bit) = titan_core::wheel::number_to_slot(p).unwrap();
                        arena.segment_buf[b] |= 1 << bit;
                    } else {
                        break;
                    }
                }
            }

            for p in arena.small_primes.iter_mut() {
                p.cross_off(&mut arena.segment_buf);
            }
            for p in arena.medium_primes.iter_mut() {
                p.cross_off(&mut arena.segment_buf);
            }
            if let Some(ref mut ring) = arena.bucket_ring {
                let w = ring.window_size;
                let slot = seg_idx % w;
                if seg_idx > 0 && slot == 0 {
                    ring.advance_window();
                }
                ring.drain_segment(slot, &mut arena.segment_buf, s);
            }

            // Copy sieved bytes
            if byte_offset < bytes.len() {
                let copy_len = s.min(bytes.len() - byte_offset);
                bytes[byte_offset..byte_offset + copy_len].copy_from_slice(&arena.segment_buf[..copy_len]);
                byte_offset += copy_len;
            }

            for p in arena.small_primes.iter_mut() {
                p.byte = p.byte.saturating_sub(s);
            }
            for p in arena.medium_primes.iter_mut() {
                p.byte = p.byte.saturating_sub(s as u32);
            }
        }

        // 2. Compute block prefix counts
        let mut prefix_counts = Vec::with_capacity(num_blocks);
        let mut running_sum = 3u32; // Primes 2, 3, 5

        for b in 0..num_blocks {
            prefix_counts.push(running_sum);
            let start = b * BLOCK_BYTES;
            let end = start + BLOCK_BYTES;
            let block_bits: u32 = bytes[start..end]
                .iter()
                .map(|&byte| byte.count_ones())
                .sum();
            running_sum += block_bits;
        }

        Self {
            max_y,
            bytes,
            prefix_counts,
        }
    }

    /// Fast O(1) query of pi(y)
    #[inline(always)]
    pub fn pi(&self, y: u64) -> u64 {
        if y < 2 { return 0; }
        if y == 2 { return 1; }
        if y < 5 { return 2; }
        if y < 7 { return 3; }
        if y > self.max_y {
            panic!("Query y = {} exceeds PiTable max_y = {}", y, self.max_y);
        }

        let byte_idx = (y / 30) as usize;
        let block_idx = byte_idx / BLOCK_BYTES;
        let mut count = self.prefix_counts[block_idx] as u64;

        let block_byte_start = block_idx * BLOCK_BYTES;

        // Popcount complete words in this block before byte_idx
        let mut b = block_byte_start;
        while b + 8 <= byte_idx {
            let chunk = u64::from_le_bytes(self.bytes[b..b + 8].try_into().unwrap());
            count += chunk.count_ones() as u64;
            b += 8;
        }
        while b < byte_idx {
            count += self.bytes[b].count_ones() as u64;
            b += 1;
        }

        // Final byte: mask out residues > y % 30
        let rem = (y % 30) as u8;
        let mut mask = 0u8;
        for i in 0..8 {
            if RESIDUES[i] <= rem {
                mask |= 1 << i;
            }
        }
        count += (self.bytes[byte_idx] & mask).count_ones() as u64;

        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pi_table_milestones() {
        let table = PiTable::new(100_000);
        assert_eq!(table.pi(10), 4);
        assert_eq!(table.pi(100), 25);
        assert_eq!(table.pi(1000), 168);
        assert_eq!(table.pi(10000), 1229);
        assert_eq!(table.pi(100000), 9592);
    }
}
