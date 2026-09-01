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
            } else {
                arena.medium_primes.push(MediumPrime::new(p, p2_byte as usize, p_res_bit));
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
        } else {
            arena.medium_primes.push(MediumPrime::new(p, p_byte as usize, j));
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
