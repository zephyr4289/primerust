//! Sieve Driver: orchestrates segmented sieving and translation invariance.

use crate::arena::SieveArena;
use crate::erat_medium::MediumPrime;
use crate::erat_small::SmallPrime;
use crate::tally::count_segment;
use titan_core::wheel::{self, RESIDUES, RESIDUE_TO_BIT};

/// Counts primes <= n using a segmented wheel sieve.
pub fn count_primes(n: u64, seg_size_bytes: usize) -> u64 {
    // Exact small-n paths
    if n < 2 { return 0; }
    if n == 2 { return 1; }
    if n < 5 { return 2; }
    if n < 7 { return 3; }
    if n < 11 { return 4; }
    if n < 13 { return 5; }
    if n < 17 { return 6; }
    if n < 19 { return 7; }
    if n < 23 { return 8; }
    if n < 29 { return 9; }
    if n < 31 { return 10; }

    let mut arena = SieveArena::new(n, seg_size_bytes);
    count_primes_with_arena(n, seg_size_bytes, &mut arena)
}

pub fn count_primes_with_arena(n: u64, seg_size_bytes: usize, arena: &mut SieveArena) -> u64 {
    if n < 2 { return 0; }
    if n == 2 { return 1; }
    if n < 5 { return 2; }
    if n < 7 { return 3; }
    if n < 11 { return 4; }
    if n < 13 { return 5; }
    if n < 17 { return 6; }
    if n < 19 { return 7; }
    if n < 23 { return 8; }
    if n < 29 { return 9; }
    if n < 31 { return 10; }

    arena.reset();
    let s = seg_size_bytes;
    let seg_span = (s as u64) * 30;

    let num_segments = ((n / seg_span) + 1) as usize;
    let mut total_primes = 3u64; // 2, 3, 5

    for seg_idx in 0..num_segments {
        let seg_low = (seg_idx as u64) * seg_span;
        let seg_high = seg_low + seg_span - 1;
        let is_last = seg_high >= n;

        // 1. Presieve init
        arena.presieve.init_segment(seg_idx, &mut arena.segment_buf);

        // 2. Frontier activation: activate any base prime with p^2 <= seg_high
        while arena.base_frontier_idx < arena.base_primes.len() {
            let p = arena.base_primes[arena.base_frontier_idx];
            let p2 = p * p;
            if p2 > seg_high {
                break;
            }
            // First multiple is p^2
            let p2_byte = (p2 / 30) - (seg_low / 30);
            let p_res_bit = RESIDUE_TO_BIT[(p % 30) as usize];

            if p <= arena.small_threshold {
                arena.small_primes.push(SmallPrime::new(p, p2_byte as usize, p_res_bit));
            } else if p <= arena.medium_threshold || arena.bucket_ring.is_none() {
                arena.medium_primes.push(MediumPrime::new(p, p2_byte as usize, p_res_bit));
            } else {
                let ring = arena.bucket_ring.as_mut().unwrap();
                let w = ring.window_size;
                let cur_slot = seg_idx % w;
                let p2_bit = wheel::WHEEL_NEXT[p_res_bit as usize][p_res_bit as usize];
                let entry = crate::erat_big::BucketEntry::pack(
                    p as u32,
                    p2_byte as u32,
                    p_res_bit,
                    p_res_bit,
                    p2_bit,
                    0,
                );
                ring.push_ring(cur_slot, entry);
            }
            arena.base_frontier_idx += 1;
        }

        // 3. Segment 0 special handling
        if seg_idx == 0 {
            // n = 1 is not prime
            arena.segment_buf[0] &= !(1 << 0);

            // Re-set presieve primes 7, 11, 13 if <= n
            for &p in &[7u64, 11, 13] {
                if p <= n {
                    let (b, bit) = wheel::number_to_slot(p).unwrap();
                    arena.segment_buf[b] |= 1 << bit;
                }
            }

            // Re-set base primes 7, 11, 13... that presieve cleared
            for &p in &arena.base_primes {
                if p <= seg_high && p <= n {
                    let (b, bit) = wheel::number_to_slot(p).unwrap();
                    arena.segment_buf[b] |= 1 << bit;
                } else {
                    break;
                }
            }
        }

        // 4. Cross off small primes
        for p in arena.small_primes.iter_mut() {
            p.cross_off(&mut arena.segment_buf);
        }

        // 5. Cross off medium primes
        for p in arena.medium_primes.iter_mut() {
            p.cross_off(&mut arena.segment_buf);
        }

        // 5b. Drain bucket primes for current segment
        if let Some(ref mut ring) = arena.bucket_ring {
            let w = ring.window_size;
            let slot = seg_idx % w;
            if seg_idx > 0 && slot == 0 {
                ring.advance_window();
            }
            ring.drain_segment(slot, &mut arena.segment_buf, s);
        }

        // 6. Tally segment
        if !is_last {
            total_primes += count_segment(&mut arena.segment_buf, s, 7);
        } else {
            // Final segment: compute exact valid bytes and last valid bit
            let last_byte_in_seg = ((n - seg_low) / 30) as usize;
            let rem = (n % 30) as u8;

            if rem < 1 {
                // No candidates in last_byte_in_seg
                if last_byte_in_seg > 0 {
                    total_primes += count_segment(&mut arena.segment_buf, last_byte_in_seg, 7);
                }
            } else {
                // Find highest bit <= rem
                let mut last_valid_bit = 0u8;
                for (bit, &r) in RESIDUES.iter().enumerate() {
                    if r <= rem {
                        last_valid_bit = bit as u8;
                    } else {
                        break;
                    }
                }
                total_primes += count_segment(&mut arena.segment_buf, last_byte_in_seg + 1, last_valid_bit);
            }
            break;
        }

        // 7. Translation-invariance update for next segment: byte -= S
        for p in arena.small_primes.iter_mut() {
            p.byte = p.byte.saturating_sub(s);
        }
        for p in arena.medium_primes.iter_mut() {
            p.byte = p.byte.saturating_sub(s as u32);
        }
    }

    total_primes
}

/// Direct range sieve: counts primes in [lo, hi] by sieving only segments within [lo, hi].
pub fn count_primes_range_direct(
    lo: u64,
    hi: u64,
    seg_size_bytes: usize,
    arena: &mut SieveArena,
) -> u64 {
    if lo > hi {
        return 0;
    }
    if lo <= 2 {
        return count_primes_with_arena(hi, seg_size_bytes, arena);
    }

    arena.reset();
    let s = seg_size_bytes;
    let seg_span = (s as u64) * 30;

    let start_seg_idx = (lo / seg_span) as usize;
    let end_seg_idx = (hi / seg_span) as usize;
    let aligned_low = (start_seg_idx as u64) * seg_span;

    // Activate all base primes up to sqrt(hi)
    for &p in &arena.base_primes {
        if p * p > hi {
            break;
        }
        let target_m = p.max((aligned_low + p - 1) / p);
        let rem = (target_m % 30) as usize;
        let next_res = wheel::NEXT_COPRIME[rem] as u64;
        let m = if next_res >= (rem as u64) {
            (target_m / 30) * 30 + next_res
        } else {
            (target_m / 30 + 1) * 30 + next_res
        };

        let mult = p * m;
        let p_byte = (mult / 30) - (aligned_low / 30);
        let j = wheel::RESIDUE_TO_BIT[(m % 30) as usize];

        if p <= arena.small_threshold {
            arena.small_primes.push(SmallPrime::new(p, p_byte as usize, j));
        } else if p <= arena.medium_threshold || arena.bucket_ring.is_none() {
            arena.medium_primes.push(MediumPrime::new(p, p_byte as usize, j));
        } else {
            let ring = arena.bucket_ring.as_mut().unwrap();
            let target_slot = (p_byte as usize) / s;
            let rel_byte = (p_byte as u32) % (s as u32);
            let row = wheel::RESIDUE_TO_BIT[(p % 30) as usize];
            let prod_bit = wheel::WHEEL_NEXT[row as usize][j as usize];
            if target_slot < ring.window_size {
                let entry = crate::erat_big::BucketEntry::pack(p as u32, rel_byte, j, row, prod_bit, 0);
                ring.push_ring(target_slot, entry);
            } else {
                let rem_segs = (target_slot - ring.window_size) as u32;
                let entry = crate::erat_big::BucketEntry::pack(p as u32, rel_byte, j, row, prod_bit, rem_segs);
                ring.push_carry(entry);
            }
        }
    }

    let mut total_primes = 0u64;

    for seg_idx in start_seg_idx..=end_seg_idx {
        let cur_seg_low = (seg_idx as u64) * seg_span;
        let cur_seg_high = cur_seg_low + seg_span - 1;

        // 1. Presieve
        arena.presieve.init_segment(seg_idx, &mut arena.segment_buf);

        // 2. Cross off small & medium primes
        for p in arena.small_primes.iter_mut() {
            p.cross_off(&mut arena.segment_buf);
        }
        for p in arena.medium_primes.iter_mut() {
            p.cross_off(&mut arena.segment_buf);
        }

        // 2b. Drain bucket primes for current segment
        if let Some(ref mut ring) = arena.bucket_ring {
            let local_idx = seg_idx - start_seg_idx;
            let w = ring.window_size;
            let slot = local_idx % w;
            if local_idx > 0 && slot == 0 {
                ring.advance_window();
            }
            ring.drain_segment(slot, &mut arena.segment_buf, s);
        }

        // 3. Tally with head/tail masking if boundary segment
        let seg_start_n = cur_seg_low.max(lo);
        let seg_end_n = cur_seg_high.min(hi);

        if seg_start_n == cur_seg_low && seg_end_n == cur_seg_high {
            // Completely full segment: full popcount
            total_primes += count_segment(&mut arena.segment_buf, s, 7);
        } else {
            // Partial segment: iterate over valid bytes
            let first_byte = ((seg_start_n - cur_seg_low) / 30) as usize;
            let last_byte = ((seg_end_n - cur_seg_low) / 30) as usize;

            for b in first_byte..=last_byte {
                let mut bits = arena.segment_buf[b];
                while bits != 0 {
                    let bit_idx = bits.trailing_zeros() as usize;
                    bits &= !(1 << bit_idx);
                    let cand = cur_seg_low + (b as u64) * 30 + (RESIDUES[bit_idx] as u64);
                    if cand >= seg_start_n && cand <= seg_end_n {
                        total_primes += 1;
                    }
                }
            }
        }

        // 4. Translation-invariance update for next segment: byte -= S
        for p in arena.small_primes.iter_mut() {
            p.byte = p.byte.saturating_sub(s);
        }
        for p in arena.medium_primes.iter_mut() {
            p.byte = p.byte.saturating_sub(s as u32);
        }
    }

    total_primes
}

/// Range count: pi(hi) - pi(lo - 1)
pub fn count_primes_range(lo: u64, hi: u64, seg_size_bytes: usize) -> u64 {
    if lo > hi {
        return 0;
    }
    let mut arena = SieveArena::new(hi, seg_size_bytes);
    count_primes_range_direct(lo, hi, seg_size_bytes, &mut arena)
}

/// Evaluates prime counts at specific thresholds across an interval [lo, hi]
/// using a single-pass sweep with an ultra-fast intra-segment popcount walk.
pub fn count_primes_range_with_thresholds(
    lo: u64,
    hi: u64,
    seg_size_bytes: usize,
    arena: &mut SieveArena,
    thresholds: &[u64],
    threshold_counts: &mut [u64],
    initial_pi_before_lo: u64,
) -> u64 {
    if lo > hi || thresholds.is_empty() {
        return initial_pi_before_lo;
    }

    let s = seg_size_bytes;
    let seg_span = (s as u64) * 30;

    let start_seg_idx = (lo / seg_span) as usize;
    let end_seg_idx = (hi / seg_span) as usize;
    let aligned_low = (start_seg_idx as u64) * seg_span;

    arena.reset();

    for &p in &arena.base_primes {
        if p * p > hi {
            break;
        }
        let target_m = p.max((aligned_low + p - 1) / p);
        let rem = (target_m % 30) as usize;
        let next_res = wheel::NEXT_COPRIME[rem] as u64;
        let m = if next_res >= (rem as u64) {
            (target_m / 30) * 30 + next_res
        } else {
            (target_m / 30 + 1) * 30 + next_res
        };

        let mult = p * m;
        let p_byte = (mult / 30) - (aligned_low / 30);
        let j = wheel::RESIDUE_TO_BIT[(m % 30) as usize];

        if p <= arena.small_threshold {
            arena.small_primes.push(SmallPrime::new(p, p_byte as usize, j));
        } else if p <= arena.medium_threshold || arena.bucket_ring.is_none() {
            arena.medium_primes.push(MediumPrime::new(p, p_byte as usize, j));
        } else {
            let ring = arena.bucket_ring.as_mut().unwrap();
            let target_slot = (p_byte as usize) / s;
            let rel_byte = (p_byte as u32) % (s as u32);
            let row = wheel::RESIDUE_TO_BIT[(p % 30) as usize];
            let prod_bit = wheel::WHEEL_NEXT[row as usize][j as usize];
            if target_slot < ring.window_size {
                let entry = crate::erat_big::BucketEntry::pack(p as u32, rel_byte, j, row, prod_bit, 0);
                ring.push_ring(target_slot, entry);
            } else {
                let rem_segs = (target_slot - ring.window_size) as u32;
                let entry = crate::erat_big::BucketEntry::pack(p as u32, rel_byte, j, row, prod_bit, rem_segs);
                ring.push_carry(entry);
            }
        }
    }

    let mut running_pi = initial_pi_before_lo;
    let mut th_idx = 0;

    for seg_idx in start_seg_idx..=end_seg_idx {
        let cur_seg_low = (seg_idx as u64) * seg_span;
        let cur_seg_high = cur_seg_low + seg_span - 1;

        // 1. Presieve
        arena.presieve.init_segment(seg_idx, &mut arena.segment_buf);

        // 2. Cross off primes
        for p in arena.small_primes.iter_mut() {
            p.cross_off(&mut arena.segment_buf);
        }
        for p in arena.medium_primes.iter_mut() {
            p.cross_off(&mut arena.segment_buf);
        }

        if let Some(ref mut ring) = arena.bucket_ring {
            let local_idx = seg_idx - start_seg_idx;
            let w = ring.window_size;
            let slot = local_idx % w;
            if local_idx > 0 && slot == 0 {
                ring.advance_window();
            }
            ring.drain_segment(slot, &mut arena.segment_buf, s);
        }

        // 3. Process thresholds falling in this segment with a continuous byte-walk
        let offset_before_lo = if cur_seg_low < lo {
            let lo_byte = ((lo - cur_seg_low) / 30) as usize;
            let lo_rem = (lo % 30) as u8;
            let mut count_before = 0u64;
            for b in 0..lo_byte {
                count_before += arena.segment_buf[b].count_ones() as u64;
            }
            let mut mask = 0u8;
            for r in 0..8 {
                if wheel::RESIDUES[r] < lo_rem {
                    mask |= 1 << r;
                }
            }
            count_before += (arena.segment_buf[lo_byte] & mask).count_ones() as u64;
            count_before
        } else {
            0
        };

        let mut cur_walk_byte = 0usize;
        let mut intra_walk_sum = 0u64;

        while th_idx < thresholds.len() && thresholds[th_idx] <= cur_seg_high {
            let t = thresholds[th_idx];
            if t < cur_seg_low {
                threshold_counts[th_idx] = initial_pi_before_lo;
                th_idx += 1;
                continue;
            }

            let target_byte = ((t - cur_seg_low) / 30) as usize;
            let target_rem = (t % 30) as u8;

            // Advance intra_walk_sum up to target_byte
            while cur_walk_byte < target_byte {
                intra_walk_sum += arena.segment_buf[cur_walk_byte].count_ones() as u64;
                cur_walk_byte += 1;
            }

            // Final byte mask
            let mut mask = 0u8;
            for r in 0..8 {
                if wheel::RESIDUES[r] <= target_rem {
                    mask |= 1 << r;
                }
            }
            let final_byte_count = (arena.segment_buf[target_byte] & mask).count_ones() as u64;

            let primes_in_seg_to_t = (intra_walk_sum + final_byte_count).saturating_sub(offset_before_lo);
            threshold_counts[th_idx] = running_pi + primes_in_seg_to_t;
            th_idx += 1;
        }

        // 4. Full segment popcount
        let seg_primes = count_segment(&mut arena.segment_buf, s, 7);
        running_pi += seg_primes.saturating_sub(offset_before_lo);

        // 5. Translation update
        for p in arena.small_primes.iter_mut() {
            p.byte = p.byte.saturating_sub(s);
        }
        for p in arena.medium_primes.iter_mut() {
            p.byte = p.byte.saturating_sub(s as u32);
        }
    }

    running_pi
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pi_milestones() {
        assert_eq!(count_primes(10, 65536), 4);
        assert_eq!(count_primes(100, 65536), 25);
        assert_eq!(count_primes(1_000, 65536), 168);
        assert_eq!(count_primes(10_000, 65536), 1229);
        assert_eq!(count_primes(100_000, 65536), 9592);
        assert_eq!(count_primes(1_000_000, 65536), 78498);
        assert_eq!(count_primes(10_000_000, 65536), 664579);
    }

    #[test]
    fn test_range_invariance() {
        let full = count_primes(100_000, 32768);
        let range1 = count_primes_range(0, 50_000, 32768);
        let range2 = count_primes_range(50_001, 100_000, 32768);
        assert_eq!(range1 + range2, full);
    }
}
