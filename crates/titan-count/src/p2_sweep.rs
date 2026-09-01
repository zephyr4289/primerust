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

    if num_threads <= 1 || (hi - lo) < 10_000_000 {
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
        return p2_total;
    }

    // Partition [lo, hi] into num_threads slices aligned to SEG_SPAN
    const SEG_SPAN: u64 = 65536 * 30; // 1,966,080
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
                let mut arena = SieveArena::new(s_hi, 65536);
                *total_ref = count_primes_range_with_thresholds(
                    s_lo,
                    s_hi,
                    65536,
                    &mut arena,
                    th_slice,
                    counts_ref,
                    0, // relative offset starting from 0
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
