//! P2: Second partial sifting term P2(x, a, b) via RAM-Law compliant sweep.
//!
//! Evaluates:
//!   P2(x, a, b) = sum_{i=a+1}^b pi(x / p_i)
//!
//! Guarantees:
//!   - PiTable hard-capped at x^1/2 (RAM Law)
//!   - Queries <= pi_table.max_y evaluated in O(1)
//!   - Queries > pi_table.max_y evaluated via single-pass segmented sweep
//!     with continuous intra-segment byte-walk join (walk-not-lookup).

use crate::pi_table::PiTable;
use titan_sieve::arena::SieveArena;
use titan_sieve::segment::count_primes_range_with_thresholds;

pub fn compute_p2(
    x: u64,
    a: usize,
    b: usize,
    primes: &[u64],
    pi_table: &PiTable,
) -> u128 {
    if a >= b {
        return 0;
    }

    let mut p2_total = 0u128;
    let mut high_queries = Vec::new();

    let active_slice = &primes[(a + 1)..=b];
    let mut sbrb = crate::b_term::StreamingReciprocalBuffer::new();
    let mut chunk_start = 0;

    while chunk_start < active_slice.len() {
        let chunk_end = (chunk_start + crate::b_term::RECIPROCAL_BLOCK_SIZE).min(active_slice.len());
        let slice = &active_slice[chunk_start..chunk_end];
        sbrb.fill_block(slice, x);

        let mut i = 0;
        while i + 4 <= slice.len() {
            let y0 = sbrb.table[i].div(x);
            let y1 = sbrb.table[i + 1].div(x);
            let y2 = sbrb.table[i + 2].div(x);
            let y3 = sbrb.table[i + 3].div(x);

            if y0 <= pi_table.max_y { p2_total += pi_table.pi(y0) as u128; } else { high_queries.push(y0); }
            if y1 <= pi_table.max_y { p2_total += pi_table.pi(y1) as u128; } else { high_queries.push(y1); }
            if y2 <= pi_table.max_y { p2_total += pi_table.pi(y2) as u128; } else { high_queries.push(y2); }
            if y3 <= pi_table.max_y { p2_total += pi_table.pi(y3) as u128; } else { high_queries.push(y3); }
            i += 4;
        }

        while i < slice.len() {
            let y = sbrb.table[i].div(x);
            if y <= pi_table.max_y {
                p2_total += pi_table.pi(y) as u128;
            } else {
                high_queries.push(y);
            }
            i += 1;
        }

        chunk_start = chunk_end;
    }

    if high_queries.is_empty() {
        return p2_total;
    }

    // Sort ascending for monotonic sweep
    high_queries.sort_unstable();
    let hi = *high_queries.last().unwrap();
    let lo = pi_table.max_y + 1;

    let mut threshold_counts = vec![0u64; high_queries.len()];
    let mut arena = SieveArena::new(hi, 65536);
    let initial_pi = pi_table.pi(pi_table.max_y);

    count_primes_range_with_thresholds(
        lo,
        hi,
        65536,
        &mut arena,
        &high_queries,
        &mut threshold_counts,
        initial_pi,
    );

    for cnt in threshold_counts {
        p2_total += cnt as u128;
    }

    p2_total
}

/// Multi-threaded evaluation of P2(x, a, b) with sliced range sieving across threads
pub fn compute_p2_mt(
    x: u64,
    a: usize,
    b: usize,
    primes: &[u64],
    pi_table: &PiTable,
    num_threads: usize,
) -> u128 {
    if a >= b {
        return 0;
    }

    let mut p2_total = 0u128;
    let mut high_queries = Vec::new();

    for i in (a + 1)..=b {
        let p_i = primes[i];
        let y = x / p_i;

        if y <= pi_table.max_y {
            p2_total += pi_table.pi(y) as u128;
        } else {
            high_queries.push(y);
        }
    }

    if high_queries.is_empty() {
        return p2_total;
    }

    high_queries.sort_unstable();
    let hi = *high_queries.last().unwrap();
    let lo = pi_table.max_y + 1;

    if num_threads <= 1 || (hi - lo) < 200_000 {
        let mut threshold_counts = vec![0u64; high_queries.len()];
        let mut arena = SieveArena::new(hi, 32768);
        let initial_pi = pi_table.pi(pi_table.max_y);

        count_primes_range_with_thresholds(
            lo,
            hi,
            32768,
            &mut arena,
            &high_queries,
            &mut threshold_counts,
            initial_pi,
        );

        for cnt in threshold_counts {
            p2_total += cnt as u128;
        }
        return p2_total;
    }

    // Heterogeneous DynamIQ Partitioning for Snapdragon 4 Gen 2 (2x A78 Big + 6x A55 Little)
    const SEG_SPAN: u64 = 32768 * 30; // 32 KiB segments for Cortex-A55 L1D
    let start_seg = lo / SEG_SPAN;
    let end_seg = hi / SEG_SPAN;
    let total_segs = end_seg - start_seg + 1;

    // Weights: Core 7 (3.0), Core 6 (3.0), Cores 5..0 (1.0 each)
    let mut thread_core_map = Vec::with_capacity(num_threads);
    let mut weights = Vec::with_capacity(num_threads);

    if num_threads >= 8 {
        // Cores 7, 6 are Big (A78); Cores 5..0 are Little (A55)
        thread_core_map = vec![7, 6, 5, 4, 3, 2, 1, 0];
        weights = vec![3.0f64, 3.0f64, 1.0f64, 1.0f64, 1.0f64, 1.0f64, 1.0f64, 1.0f64];
    } else {
        for t in 0..num_threads {
            thread_core_map.push(7 - t % 8);
            weights.push(if (7 - t % 8) >= 6 { 2.5f64 } else { 1.0f64 });
        }
    }

    let total_weight: f64 = weights.iter().sum();
    let mut thread_ranges = Vec::new();
    let mut cur_seg = start_seg;

    for t in 0..num_threads {
        let segs_for_thread = ((total_segs as f64) * weights[t] / total_weight).round() as u64;
        let segs_for_thread = segs_for_thread.max(1);
        let seg_end = if t == num_threads - 1 {
            end_seg
        } else {
            (cur_seg + segs_for_thread - 1).min(end_seg)
        };

        let slice_lo = if cur_seg == start_seg { lo } else { cur_seg * SEG_SPAN };
        let slice_hi = if seg_end >= end_seg { hi } else { (seg_end + 1) * SEG_SPAN - 1 };

        if slice_lo <= slice_hi && cur_seg <= end_seg {
            thread_ranges.push((slice_lo, slice_hi, thread_core_map[t]));
        }
        cur_seg = seg_end + 1;
        if cur_seg > end_seg {
            break;
        }
    }

    let actual_threads = thread_ranges.len();
    let mut thread_thresholds = Vec::with_capacity(actual_threads);
    for &(s_lo, s_hi, _) in &thread_ranges {
        let start_idx = high_queries.partition_point(|&y| y < s_lo);
        let end_idx = high_queries.partition_point(|&y| y <= s_hi);
        thread_thresholds.push(&high_queries[start_idx..end_idx]);
    }

    let mut thread_counts: Vec<Vec<u64>> = (0..actual_threads)
        .map(|i| vec![0u64; thread_thresholds[i].len()])
        .collect();
    let mut thread_totals = vec![0u64; actual_threads];

    std::thread::scope(|s| {
        for (t, (counts_ref, total_ref)) in thread_counts.iter_mut().zip(thread_totals.iter_mut()).enumerate() {
            let (s_lo, s_hi, core_id) = thread_ranges[t];
            let th_slice = thread_thresholds[t];

            s.spawn(move || {
                let _ = titan_bench::affinity::pin_thread_to_core(core_id);
                let mut arena = SieveArena::new(s_hi, 32768);
                *total_ref = count_primes_range_with_thresholds(
                    s_lo,
                    s_hi,
                    32768,
                    &mut arena,
                    th_slice,
                    counts_ref,
                    0,
                );
            });
        }
    });

    // Prefix sum base counts
    let mut running_base = pi_table.pi(pi_table.max_y);
    for t in 0..actual_threads {
        for &rel_cnt in &thread_counts[t] {
            p2_total += (running_base + rel_cnt) as u128;
        }
        running_base += thread_totals[t];
    }

    p2_total
}

/// Evaluates P2 over an explicit range [lo, hi] for z-split support.
/// This allows the Gourdon algorithm to restrict the physical sweep to [x^(1/2), z]
/// and delegate (z, x^(2/3)] to B/D terms.
pub fn compute_p2_range(
    x: u64,
    a: usize,
    b: usize,
    primes: &[u64],
    pi_table: &PiTable,
    range_lo: u64,
    range_hi: u64,
) -> u128 {
    if a >= b {
        return 0;
    }

    let mut p2_total = 0u128;
    let mut high_queries = Vec::new();

    for i in (a + 1)..=b {
        let p_i = primes[i];
        let y = x / p_i;

        // Only include queries that fall within the specified range
        if y < range_lo || y > range_hi {
            continue;
        }

        if y <= pi_table.max_y {
            p2_total += pi_table.pi(y) as u128;
        } else {
            high_queries.push(y);
        }
    }

    if high_queries.is_empty() {
        return p2_total;
    }

    high_queries.sort_unstable();
    let hi = *high_queries.last().unwrap().min(&range_hi);
    let lo = range_lo.max(pi_table.max_y + 1);

    let mut threshold_counts = vec![0u64; high_queries.len()];
    let mut arena = SieveArena::new(hi, 32768); // 32 KiB for SD4G2
    let initial_pi = pi_table.pi(pi_table.max_y);

    count_primes_range_with_thresholds(
        lo,
        hi,
        32768,
        &mut arena,
        &high_queries,
        &mut threshold_counts,
        initial_pi,
    );

    for cnt in threshold_counts {
        p2_total += cnt as u128;
    }

    p2_total
}

/// Multi-threaded P2 evaluation over explicit range [range_lo, range_hi]
pub fn compute_p2_range_mt(
    x: u64,
    a: usize,
    b: usize,
    primes: &[u64],
    pi_table: &PiTable,
    range_lo: u64,
    range_hi: u64,
    num_threads: usize,
) -> u128 {
    if a >= b || range_lo > range_hi {
        return 0;
    }
    if num_threads <= 1 || (range_hi - range_lo) < 10_000_000 {
        return compute_p2_range(x, a, b, primes, pi_table, range_lo, range_hi);
    }

    let mut p2_total = 0u128;
    let mut high_queries = Vec::new();

    for i in (a + 1)..=b {
        let p_i = primes[i];
        let y = x / p_i;

        if y < range_lo || y > range_hi {
            continue;
        }

        if y <= pi_table.max_y {
            p2_total += pi_table.pi(y) as u128;
        } else {
            high_queries.push(y);
        }
    }

    if high_queries.is_empty() {
        return p2_total;
    }

    high_queries.sort_unstable();
    let hi = *high_queries.last().unwrap().min(&range_hi);
    let lo = range_lo.max(pi_table.max_y + 1);

    const SEG_SPAN: u64 = 32768 * 30; // 32 KiB segments for SD4G2
    let start_seg = lo / SEG_SPAN;
    let end_seg = hi / SEG_SPAN;
    let total_segs = end_seg - start_seg + 1;
    let segs_per_thread = (total_segs + (num_threads as u64) - 1) / (num_threads as u64);

    let mut thread_ranges = Vec::new();
    let mut cur_seg = start_seg;
    for t in 0..num_threads {
        let seg_end = (cur_seg + segs_per_thread - 1).min(end_seg);
        let slice_lo = if t == 0 { lo } else { cur_seg * SEG_SPAN };
        let slice_hi = if t == num_threads - 1 || seg_end >= end_seg {
            hi
        } else {
            (seg_end + 1) * SEG_SPAN - 1
        };

        if slice_lo <= slice_hi {
            thread_ranges.push((slice_lo, slice_hi));
        }
        cur_seg = seg_end + 1;
        if cur_seg > end_seg {
            break;
        }
    }

    let actual_threads = thread_ranges.len();
    let mut thread_thresholds = Vec::with_capacity(actual_threads);
    for &(s_lo, s_hi) in &thread_ranges {
        let start_idx = high_queries.partition_point(|&y| y < s_lo);
        let end_idx = high_queries.partition_point(|&y| y <= s_hi);
        thread_thresholds.push(&high_queries[start_idx..end_idx]);
    }

    let mut thread_counts: Vec<Vec<u64>> = (0..actual_threads)
        .map(|i| vec![0u64; thread_thresholds[i].len()])
        .collect();
    let mut thread_totals = vec![0u64; actual_threads];

    std::thread::scope(|s| {
        for (t, (counts_ref, total_ref)) in thread_counts.iter_mut().zip(thread_totals.iter_mut()).enumerate() {
            let (s_lo, s_hi) = thread_ranges[t];
            let th_slice = thread_thresholds[t];

            s.spawn(move || {
                let mut arena = SieveArena::new(s_hi, 32768);
                *total_ref = count_primes_range_with_thresholds(
                    s_lo,
                    s_hi,
                    32768,
                    &mut arena,
                    th_slice,
                    counts_ref,
                    0,
                );
            });
        }
    });

    let mut running_base = pi_table.pi(pi_table.max_y);
    for t in 0..actual_threads {
        for &rel_cnt in &thread_counts[t] {
            p2_total += (running_base + rel_cnt) as u128;
        }
        running_base += thread_totals[t];
    }

    p2_total
}

#[cfg(test)]
mod tests {
    use super::*;
    use titan_sieve::base::generate_base_primes;
    use titan_core::roots::isqrt;

    #[test]
    fn test_compute_p2_exact() {
        let x = 100_000u64;
        let x_sqrt = isqrt(x);
        let base_primes = generate_base_primes(x_sqrt + 100);
        let mut primes = vec![0u64];
        primes.extend_from_slice(&base_primes);
        let pi_table = PiTable::new(x_sqrt + 30);
        let a = 5;
        let b = 15;
        let p2 = compute_p2(x, a, b, &primes, &pi_table);
        assert!(p2 > 0);
    }
}
