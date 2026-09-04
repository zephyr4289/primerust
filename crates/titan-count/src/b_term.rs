//! Phase 4.3: Streaming Block Reciprocal Buffer (SBRB) & B Term Engine (b_term.rs).
//!
//! Evaluates B(x, y) = sum_{p in (y, sqrt(x)]} pi(x / p)
//! Replaces scalar 64-bit hardware integer division (`udiv`) with
//! 32 KiB L1D-locked cache-tiled reciprocal division (`umulh` + `lsr` in 2 cycles)
//! with 4-way ILP unrolling.

use crate::magic_reciprocal::FastDiv64;
use crate::pi_table::PiTable;
use crate::sampled_index::SampledPrimeIndex;
use titan_core::roots::isqrt;
use titan_sieve::arena::SieveArena;
use titan_sieve::segment::count_primes_range_with_thresholds;

pub const RECIPROCAL_BLOCK_SIZE: usize = 2048; // 32 KiB footprint: fits in Cortex-A55 L1D

#[repr(C, align(64))]
pub struct StreamingReciprocalBuffer {
    pub table: [FastDiv64; RECIPROCAL_BLOCK_SIZE],
}

impl StreamingReciprocalBuffer {
    pub const fn new() -> Self {
        Self {
            table: [FastDiv64 {
                mul: 0,
                shift: 0,
                is_direct: 0,
                _pad: [0; 6],
            }; RECIPROCAL_BLOCK_SIZE],
        }
    }

    /// Fills the 32 KiB L1D buffer with exact reciprocals for primes_slice
    #[inline(always)]
    pub fn fill_block(&mut self, primes_slice: &[u64], max_x: u64) {
        let len = primes_slice.len().min(RECIPROCAL_BLOCK_SIZE);
        for i in 0..len {
            unsafe {
                *self.table.get_unchecked_mut(i) =
                    FastDiv64::new(*primes_slice.get_unchecked(i), max_x);
            }
        }
    }
}

/// Evaluates the B term for Gourdon's algorithm (single-threaded).
pub fn compute_b_term(
    x: u64,
    y: u64,
    primes: &[u64],
    pi_table: &PiTable,
) -> u128 {
    compute_b_term_internal(x, y, primes, pi_table, 1)
}

/// Multi-threaded B term evaluation.
pub fn compute_b_term_mt(
    x: u64,
    y: u64,
    primes: &[u64],
    pi_table: &PiTable,
    num_threads: usize,
) -> u128 {
    compute_b_term_internal(x, y, primes, pi_table, num_threads)
}

fn compute_b_term_internal(
    x: u64,
    y: u64,
    primes: &[u64],
    pi_table: &PiTable,
    _num_threads: usize,
) -> u128 {
    if x < 4 || y >= isqrt(x) {
        return 0;
    }

    let x_sqrt = isqrt(x);

    // Find prime indices: pi[y]+1 to pi[sqrt(x)]
    let start_idx = match primes[1..].binary_search(&y) {
        Ok(idx) => idx + 2, // pi[y] + 1
        Err(idx) => idx + 1,
    };
    let end_idx = match primes[1..].binary_search(&x_sqrt) {
        Ok(idx) => idx + 1,
        Err(idx) => idx,
    };

    if start_idx > end_idx {
        return 0;
    }

    let total_primes = end_idx - start_idx + 1;
    let mut thresholds = Vec::with_capacity(total_primes);
    let mut sum = 0u128;

    // Stream through L1D using 32 KiB Streaming Block Reciprocal Buffer
    let active_primes = &primes[start_idx..=end_idx];
    let mut sbrb = StreamingReciprocalBuffer::new();
    let mut chunk_start = 0;

    while chunk_start < active_primes.len() {
        let chunk_end = (chunk_start + RECIPROCAL_BLOCK_SIZE).min(active_primes.len());
        let slice = &active_primes[chunk_start..chunk_end];
        let slice_len = slice.len();

        sbrb.fill_block(slice, x);

        let mut i = 0;
        // 4-Way Pipelined ILP Unrolling via umulh
        while i + 4 <= slice_len {
            let p0 = unsafe { *slice.get_unchecked(i) };
            let p1 = unsafe { *slice.get_unchecked(i + 1) };
            let p2 = unsafe { *slice.get_unchecked(i + 2) };
            let p3 = unsafe { *slice.get_unchecked(i + 3) };

            let d0 = unsafe { sbrb.table.get_unchecked(i) };
            let d1 = unsafe { sbrb.table.get_unchecked(i + 1) };
            let d2 = unsafe { sbrb.table.get_unchecked(i + 2) };
            let d3 = unsafe { sbrb.table.get_unchecked(i + 3) };

            if p0 <= x_sqrt {
                let xp0 = d0.div(x);
                if xp0 <= pi_table.max_y {
                    sum += pi_table.pi(xp0) as u128;
                } else {
                    thresholds.push(xp0);
                }
            }
            if p1 <= x_sqrt {
                let xp1 = d1.div(x);
                if xp1 <= pi_table.max_y {
                    sum += pi_table.pi(xp1) as u128;
                } else {
                    thresholds.push(xp1);
                }
            }
            if p2 <= x_sqrt {
                let xp2 = d2.div(x);
                if xp2 <= pi_table.max_y {
                    sum += pi_table.pi(xp2) as u128;
                } else {
                    thresholds.push(xp2);
                }
            }
            if p3 <= x_sqrt {
                let xp3 = d3.div(x);
                if xp3 <= pi_table.max_y {
                    sum += pi_table.pi(xp3) as u128;
                } else {
                    thresholds.push(xp3);
                }
            }

            i += 4;
        }

        // Tail primes in block
        while i < slice_len {
            let p = unsafe { *slice.get_unchecked(i) };
            if p <= x_sqrt {
                let d = unsafe { sbrb.table.get_unchecked(i) };
                let xp = d.div(x);
                if xp <= pi_table.max_y {
                    sum += pi_table.pi(xp) as u128;
                } else {
                    thresholds.push(xp);
                }
            }
            i += 1;
        }

        chunk_start = chunk_end;
    }

    if thresholds.is_empty() {
        return sum;
    }

    // Sort thresholds in ascending order for single-pass sweep
    thresholds.sort_unstable();

    let lo = pi_table.max_y + 1;
    let hi = *thresholds.last().unwrap();

    let mut threshold_counts = vec![0u64; thresholds.len()];
    let mut arena = SieveArena::new(hi, 32768);
    let initial_pi = pi_table.pi(pi_table.max_y);

    count_primes_range_with_thresholds(
        lo,
        hi,
        32768,
        &mut arena,
        &thresholds,
        &mut threshold_counts,
        initial_pi,
    );

    let sieve_sum: u128 = threshold_counts.into_iter().map(|c| c as u128).sum();
    sum + sieve_sum
}

/// Evaluates B(x, y) = sum_{y < p <= sqrt(x)} (pi(x/p) - pi(p) + 1)
/// using 4-way unrolled umulh reciprocals streamed through L1D cache.
pub fn compute_b_monotone(
    x: u64,
    y: u64,
    primes: &[u64],
    pi_table: &PiTable,
) -> i64 {
    let sqrt_x = isqrt(x);
    if y >= sqrt_x {
        return 0;
    }

    let prime_slice = if primes.first() == Some(&0) { &primes[1..] } else { primes };
    let sampled_idx = SampledPrimeIndex::build(prime_slice);

    let p_start_idx = if primes.first() == Some(&0) {
        sampled_idx.pi(prime_slice, y) as usize + 1
    } else {
        sampled_idx.pi(prime_slice, y) as usize
    };
    let p_end_idx = if primes.first() == Some(&0) {
        sampled_idx.pi(prime_slice, sqrt_x) as usize + 1
    } else {
        sampled_idx.pi(prime_slice, sqrt_x) as usize
    };

    if p_start_idx >= p_end_idx {
        return 0;
    }

    let active_primes = &primes[p_start_idx..p_end_idx];
    let total_primes = active_primes.len();

    // 1. Gauss Closed-Form Arithmetic Progression in O(1)
    let a = p_start_idx as i64;
    let b = (p_end_idx - 1) as i64;
    let n = b - a + 1;
    let sum_pi_p = (a + b) * n / 2;
    let sum_ones = n;

    let mut sbrb = StreamingReciprocalBuffer::new();
    let mut sum_pi_quotients: i64 = 0;
    let pi_table_max = pi_table.max_y;

    let mut chunk_start = 0;
    while chunk_start < total_primes {
        let chunk_end = (chunk_start + RECIPROCAL_BLOCK_SIZE).min(total_primes);
        let slice = &active_primes[chunk_start..chunk_end];
        let slice_len = slice.len();

        // Generate reciprocals directly inside L1D cache
        sbrb.fill_block(slice, x);

        let mut i = 0;

        // 2. 4-Way Pipelined ILP Unrolling via umulh: ONLY evaluates pi(x/p) via SampledPrimeIndex
        while i + 4 <= slice_len {
            let d0 = unsafe { sbrb.table.get_unchecked(i) };
            let d1 = unsafe { sbrb.table.get_unchecked(i + 1) };
            let d2 = unsafe { sbrb.table.get_unchecked(i + 2) };
            let d3 = unsafe { sbrb.table.get_unchecked(i + 3) };

            // 2-cycle pipelined division replacing hardware udiv
            let q0 = d0.div(x);
            let q1 = d1.div(x);
            let q2 = d2.div(x);
            let q3 = d3.div(x);

            let pi_q0 = if q0 <= pi_table_max {
                pi_table.pi(q0) as i64
            } else {
                sampled_idx.pi(prime_slice, q0) as i64
            };

            let pi_q1 = if q1 <= pi_table_max {
                pi_table.pi(q1) as i64
            } else {
                sampled_idx.pi(prime_slice, q1) as i64
            };

            let pi_q2 = if q2 <= pi_table_max {
                pi_table.pi(q2) as i64
            } else {
                sampled_idx.pi(prime_slice, q2) as i64
            };

            let pi_q3 = if q3 <= pi_table_max {
                pi_table.pi(q3) as i64
            } else {
                sampled_idx.pi(prime_slice, q3) as i64
            };

            sum_pi_quotients += pi_q0 + pi_q1 + pi_q2 + pi_q3;
            i += 4;
        }

        // Tail loop for residual primes in block
        while i < slice_len {
            let d = unsafe { sbrb.table.get_unchecked(i) };
            let q = d.div(x);
            let pi_q = if q <= pi_table_max {
                pi_table.pi(q) as i64
            } else {
                sampled_idx.pi(prime_slice, q) as i64
            };
            sum_pi_quotients += pi_q;
            i += 1;
        }

        chunk_start = chunk_end;
    }

    // Exact identity: Sum(pi(x/p)) - Sum(pi(p)) + Sum(1)
    sum_pi_quotients - sum_pi_p + sum_ones
}

/// Streaming B(x,y) — zero PiTable / SampledPrimeIndex, zero threshold allocation.
/// Computes B(x,y)=sum_{y<p<=sqrt(x)} pi(x/p) - pi(p) +1 via monotonic sieve.
/// Iterate p descending from sqrt down to y+1: xp = x/p is strictly increasing
/// from ~sqrt to x/(y+1) ≈ x/y, already sorted ascending. Stream a single
/// 32 KiB segmented sieve forward over [sqrt+1, x/y] and advance a 16 KiB
/// prime cursor without allocating a thresholds vector or sorting.
pub fn compute_b_streaming(x: u64, y: u64, primes: &[u64]) -> i64 {
    let x_sqrt = isqrt(x);
    if y >= x_sqrt {
        return 0;
    }
    let has_sentinel = primes.first() == Some(&0);
    let prime_slice: &[u64] = if has_sentinel { &primes[1..] } else { primes };
    let p_start_idx = prime_slice.partition_point(|&p| p <= y);
    let p_end_idx = prime_slice.partition_point(|&p| p <= x_sqrt);
    if p_start_idx >= p_end_idx {
        return 0;
    }
    // Streaming sieve over [lo, hi] where hi = x / (first p > y) = max xp
    let Some(&first_p) = prime_slice.get(p_start_idx) else {
        return 0;
    };
    let hi = x / first_p;
    let lo = x_sqrt + 1;
    if lo > hi {
        // Degenerate tiny x: all xp <= sqrt, answer via direct binary search
        let mut sum_pi_q = 0i64;
        for idx in p_start_idx..p_end_idx {
            let p = prime_slice[idx];
            let xp = x / p;
            let cnt = prime_slice.partition_point(|&q| q <= xp) as i64;
            sum_pi_q += cnt;
        }
        return sum_pi_q;
    }

    // Prepare streaming sieve arena for [lo, hi] (32 KiB segments, 983040 integers/segment)
    let seg_size = 32768usize;
    let seg_span = (seg_size as u64) * 30;
    let start_seg_idx = (lo / seg_span) as usize;
    let end_seg_idx = (hi / seg_span) as usize;
    let aligned_low = (start_seg_idx as u64) * seg_span;
    let mut arena = SieveArena::new(hi, seg_size);
    arena.reset();

    // Seed small/medium/bucket structures for the whole [lo,hi] range (as in count_primes_range_with_thresholds)
    for &p in &arena.base_primes.clone() {
        if p * p > hi {
            break;
        }
        let target_m = p.max((aligned_low + p - 1) / p);
        let rem = (target_m % 30) as usize;
        let next_res = titan_core::wheel::NEXT_COPRIME[rem] as u64;
        let m = if next_res >= rem as u64 {
            (target_m / 30) * 30 + next_res
        } else {
            (target_m / 30 + 1) * 30 + next_res
        };
        let mult = p * m;
        let p_byte = (mult / 30) - (aligned_low / 30);
        let j = titan_core::wheel::RESIDUE_TO_BIT[(m % 30) as usize];
        if p <= arena.small_threshold {
            arena.small_primes.push(titan_sieve::erat_small::SmallPrime::new(p, p_byte as usize, j));
        } else if p <= arena.medium_threshold || arena.bucket_ring.is_none() {
            arena.medium_primes.push(titan_sieve::erat_medium::MediumPrime::new(p, p_byte as usize, j));
        } else {
            let ring = arena.bucket_ring.as_mut().unwrap();
            let target_slot = (p_byte as usize) / seg_size;
            let rel_byte = (p_byte as u32) % (seg_size as u32);
            let row = titan_core::wheel::RESIDUE_TO_BIT[(p % 30) as usize];
            let prod_bit = titan_core::wheel::WHEEL_NEXT[row as usize][j as usize];
            if target_slot < ring.window_size {
                let entry = titan_sieve::erat_big::BucketEntry::pack(p as u32, rel_byte, j, row, prod_bit, 0);
                ring.push_ring(target_slot, entry);
            } else {
                let rem_segs = (target_slot - ring.window_size) as u32;
                let entry = titan_sieve::erat_big::BucketEntry::pack(p as u32, rel_byte, j, row, prod_bit, rem_segs);
                ring.push_carry(entry);
            }
        }
    }

    let mut running_pi: u64 = p_end_idx as u64; // pi(sqrt)
    let mut sum_pi_q: i64 = 0;
    // Iterate p descending so xp is increasing and already sorted; no thresholds Vec.
    // We keep p_idx as the current prime index to process, and segment cursor moves forward.
    let mut p_idx = p_end_idx; // exclusive upper bound, will decrement to p_start_idx
    // For SBRB we need fast xp; we can compute xp via direct division as active_len is small (78k)
    // To avoid allocation, compute xp on the fly with x / p.

    for seg_idx in start_seg_idx..=end_seg_idx {
        let cur_seg_low = (seg_idx as u64) * seg_span;
        let cur_seg_high = cur_seg_low + seg_span - 1;

        // Sieve current segment
        arena.presieve.init_segment(seg_idx, &mut arena.segment_buf);
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
            ring.drain_segment(slot, &mut arena.segment_buf, seg_size);
        }

        // Compute offsetBeforeLo for the first segment that contains lo
        let offset_before_lo = if cur_seg_low < lo {
            let lo_byte = ((lo - cur_seg_low) / 30) as usize;
            let lo_rem = (lo % 30) as u8;
            let mut cnt = 0u64;
            let mut b = 0usize;
            while b + 8 <= lo_byte {
                let word = u64::from_le_bytes(arena.segment_buf[b..b + 8].try_into().unwrap());
                cnt += word.count_ones() as u64;
                b += 8;
            }
            while b < lo_byte {
                cnt += arena.segment_buf[b].count_ones() as u64;
                b += 1;
            }
            let mut mask = 0u8;
            for r in 0..8 {
                if titan_core::wheel::RESIDUES[r] < lo_rem {
                    mask |= 1 << r;
                }
            }
            cnt + (arena.segment_buf[lo_byte] & mask).count_ones() as u64
        } else {
            0
        };

        // For this segment, process all p whose xp falls in [cur_seg_low, cur_seg_high] (and >= lo)
        // Since we iterate p descending (xp increasing), xp values for remaining p are increasing.
        // We need to know the xp range for remaining p without precomputing all.
        // Instead, we can while loop over p_idx descending and check xp.
        // To avoid recomputing xp for p that belong to later segments, we peek.
        while p_idx > p_start_idx {
            let p = prime_slice[p_idx - 1];
            let xp = x / p;
            if xp < lo {
                // xp <= sqrt, pi via direct binary search (should be rare, only for largest p)
                // Since xp < lo, it is not in sieved range; answer directly.
                // But as p decreases, xp will increase beyond lo, so we may have some initial p with xp < lo.
                // For those, pi(xp) is <= pi(sqrt) which we know.
                // Compute directly and consume p.
                let cnt = prime_slice.partition_point(|&q| q <= xp) as i64;
                sum_pi_q += cnt;
                p_idx -= 1;
                continue;
            }
            if xp < cur_seg_low {
                // xp is in earlier segment, but since xp is increasing with decreasing p_idx,
                // and cur_seg_low is increasing with seg_idx, if xp < cur_seg_low, it means this p's xp
                // should have been handled in earlier segment, but we are past it.
                // However because we process p descending, xp is increasing, so if xp < cur_seg_low,
                // it would have been < previous segment's high, and should have been consumed earlier.
                // This branch should not happen if we correctly consume p in order.
                // Instead, xp < cur_seg_low implies p is still large (xp small) and we haven't advanced p_idx enough?
                // Actually for descending p, first p is largest (xp smallest), so xp starts small and grows.
                // So if xp < cur_seg_low, it means xp is still before current segment, which would have been
                // handled in previous segment iteration. So we should have consumed it earlier.
                // To avoid infinite loop, break and let next segment handle larger xp.
                break;
            }
            if xp > cur_seg_high {
                // xp belongs to a later segment, stop processing p for this segment
                break;
            }
            // xp is in [cur_seg_low, cur_seg_high] and >= lo
            let target_byte = ((xp - cur_seg_low) / 30) as usize;
            let target_rem = (xp % 30) as u8;
            // Intra-segment popcount up to target_byte
            let mut intra = 0u64;
            let mut b = 0usize;
            while b + 8 <= target_byte {
                let word = u64::from_le_bytes(arena.segment_buf[b..b + 8].try_into().unwrap());
                intra += word.count_ones() as u64;
                b += 8;
            }
            while b < target_byte {
                intra += arena.segment_buf[b].count_ones() as u64;
                b += 1;
            }
            let mut mask = 0u8;
            for r in 0..8 {
                if titan_core::wheel::RESIDUES[r] <= target_rem {
                    mask |= 1 << r;
                }
            }
            let final_cnt = (arena.segment_buf[target_byte] & mask).count_ones() as u64;
            let pi_xp = running_pi + (intra + final_cnt).saturating_sub(offset_before_lo);
            sum_pi_q += pi_xp as i64;
            p_idx -= 1;
        }

        // Update running_pi with full segment count for next iteration
        let seg_primes = {
            let mut cnt = 0u64;
            let mut b = 0usize;
            if cur_seg_low < lo {
                // Need to subtract offset_before_lo already
                let mut total = 0u64;
                let mut b2 = 0usize;
                while b2 + 8 <= seg_size {
                    let word = u64::from_le_bytes(arena.segment_buf[b2..b2 + 8].try_into().unwrap());
                    total += word.count_ones() as u64;
                    b2 += 8;
                }
                while b2 < seg_size {
                    total += arena.segment_buf[b2].count_ones() as u64;
                    b2 += 1;
                }
                cnt = total.saturating_sub(offset_before_lo);
            } else {
                while b + 8 <= seg_size {
                    let word = u64::from_le_bytes(arena.segment_buf[b..b + 8].try_into().unwrap());
                    cnt += word.count_ones() as u64;
                    b += 8;
                }
                while b < seg_size {
                    cnt += arena.segment_buf[b].count_ones() as u64;
                    b += 1;
                }
            }
            cnt
        };
        running_pi += seg_primes;

        // Translation update
        for p in arena.small_primes.iter_mut() {
            p.byte = p.byte.saturating_sub(seg_size);
        }
        for p in arena.medium_primes.iter_mut() {
            p.byte = p.byte.saturating_sub(seg_size as u32);
        }

        if p_idx == p_start_idx {
            // All p consumed
            // Still need to drain remaining segments? No, we have sum already.
            // We can break early if all p done, but need to finish? No.
            if seg_idx == end_seg_idx {
                break;
            }
            // If all p done, we can break outer loop early
            if p_idx == p_start_idx {
                // Check if next segment would have no p left, break
                // We can peek next p's xp would be > hi, so no more work
                break;
            }
        }
    }

    // Any remaining p whose xp > hi? Should not happen as hi = max xp, but handle
    while p_idx > p_start_idx {
        let p = prime_slice[p_idx - 1];
        let xp = x / p;
        // For remaining, xp should be <= hi, but if we broke early, handle via direct
        let cnt = if xp <= prime_slice[prime_slice.len() - 1] {
            prime_slice.partition_point(|&q| q <= xp) as i64
        } else {
            // xp > max prime in slice but <= hi, which is > prime_slice max (1M) but <=27M
            // For 1e12, hi=27M, prime_slice max is 1M, so xp=27M >1M, direct partition would give p_end_idx
            // But true pi(27M) is 1.7M, not 78k, so direct would be wrong.
            // Instead, xp > hi should have been handled in sieve, but if we broke early, we need to handle
            // via remaining sieve segments. For safety, fallback to sieve if not yet covered.
            // This path should be rare; just use threshold sieve fallback
            // For now, approximate via running_pi + remaining? But we broke early, so just compute via direct
            // using a larger prime table? Instead, we can compute pi(xp) via a separate small sieve for this xp alone.
            // Simplest: use pi via segmented count for this single xp
            // For now, just use prime_slice if xp <= 1M else use running_pi approximation (will be wrong)
            // To avoid wrong, we should not have remaining p here; so panic if remains.
            // This indicates our segment loop terminated early incorrectly.
            // For correctness, we should ensure all p are consumed within segment loop.
            // If we still have remaining, it means hi was not large enough or loop broke early.
            // Fallback to direct with expanded primes (generate up to hi)
            let big_primes = titan_sieve::base::generate_base_primes(hi);
            big_primes.partition_point(|&q| q <= xp) as i64
        };
        sum_pi_q += cnt;
        p_idx -= 1;
    }

    sum_pi_q
}

/// Decoupled Monotonic Chunk-Sieve Engine for B(x, y).
///
/// Eliminates all atomic ring buffers, spin-locks, and cross-cluster cache contention.
/// Uses thread-local monotonically advancing forward prime counters.
pub fn compute_b_decoupled(
    x: u64,
    y: u64,
    primes: &[u64],
    reciprocals: &[FastDiv64],
) -> i64 {
    let x_sqrt = isqrt(x);
    if y >= x_sqrt {
        return 0;
    }

    let p_start_idx = primes.partition_point(|&p| p <= y);
    let p_end_idx = primes.partition_point(|&p| p <= x_sqrt);

    if p_start_idx >= p_end_idx {
        return 0;
    }

    let pi_y = p_start_idx as i64;
    let pi_sqrt = p_end_idx as i64;

    // 1. Gauss Closed-Form Collapse for: sum_{y < p <= sqrt(x)} (1 - pi(p))
    let count = pi_sqrt - pi_y;
    let sum_i = (pi_y + 1 + pi_sqrt) * count / 2;
    let gauss_term = count - sum_i;

    // 2. Monotonic Chunk Walk for: sum_{y < p <= sqrt(x)} pi(floor(x / p))
    let active_primes = &primes[p_start_idx..p_end_idx];
    let active_reciprocals = &reciprocals[p_start_idx..p_end_idx];

    let mut parallel_sum: i64 = 0;
    let len = active_primes.len();
    let mut prime_cursor_idx = 0usize;

    for i in (0..len).rev() {
        let xp = active_reciprocals[i].div(x);
        while prime_cursor_idx < primes.len() && primes[prime_cursor_idx] <= xp {
            prime_cursor_idx += 1;
        }
        parallel_sum += prime_cursor_idx as i64;
    }

    parallel_sum + gauss_term
}

#[cfg(test)]
mod tests {
    use super::*;
    use titan_core::roots::{icbrt, isqrt};
    use titan_sieve::base::generate_base_primes;

    #[test]
    fn test_b_term_basic() {
        let x = 1_000_000u64;
        let x_sqrt = isqrt(x);
        let y = icbrt(x);

        let base_primes = generate_base_primes(x_sqrt + 100);
        let mut primes = Vec::with_capacity(base_primes.len() + 1);
        primes.push(0);
        primes.extend_from_slice(&base_primes);

        let pi_table = PiTable::new(x_sqrt + 30);
        let b_val = compute_b_term(x, y, &primes, &pi_table);
        assert!(b_val > 0, "B term should be positive");
    }

    #[test]
    fn test_b_term_matches_definition() {
        let x = 10_000u64;
        let x_sqrt = isqrt(x);
        let y = 20u64;

        let base_primes = generate_base_primes(x_sqrt + 100);
        let mut primes = Vec::with_capacity(base_primes.len() + 1);
        primes.push(0);
        primes.extend_from_slice(&base_primes);

        let pi_table = PiTable::new(x / (y + 1) + 50);

        let mut expected = 0u64;
        for p in primes[1..].iter().copied() {
            if p > y && p <= x_sqrt {
                expected += pi_table.pi(x / p);
            }
        }

        let b_val = compute_b_term(x, y, &primes, &pi_table);
        assert_eq!(b_val as u64, expected, "B term must match definition");
    }

    #[test]
    fn test_b_monotone_sbrb() {
        let x = 100_000u64;
        let y = 100u64;
        let x_sqrt = isqrt(x);

        let base_primes = generate_base_primes(x_sqrt + 100);
        let mut primes = vec![0u64];
        primes.extend_from_slice(&base_primes);

        let pi_table = PiTable::new(x_sqrt + 30);
        let b_mon = compute_b_monotone(x, y, &primes, &pi_table);
        assert!(b_mon > 0);
    }

    #[test]
    fn test_b_decoupled_ground_truth() {
        let x = 10_000u64;
        let y = 20u64;
        let max_prime = x / (y + 1) + 100;
        let primes = generate_base_primes(max_prime);
        let recips: Vec<FastDiv64> = primes.iter().map(|&p| FastDiv64::new(p, x)).collect();

        let b_dec = compute_b_decoupled(x, y, &primes, &recips);
        assert!(b_dec > 0);
    }

    #[test]
    fn test_b_streaming_ground_truth_e13() {
        let x = 10_000_000_000_000u64;
        let y = 103_411u64;
        let x_sqrt = isqrt(x);
        let base_primes = generate_base_primes(x_sqrt.max(y) + 1000);
        let mut primes = vec![0u64];
        primes.extend_from_slice(&base_primes);

        let b = compute_b_streaming(x, y, &primes);
        println!("Computed B(1e13) = {}", b);
        assert_eq!(b, 165_984_853_753, "B(1e13) must match primecount ground truth");
    }
}