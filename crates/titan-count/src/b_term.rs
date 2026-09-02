//! B Term: Deleglise-Rivat / Gourdon B(x, y) using segmented sieve.
//!
//! B(x, y) = sum_{p in (y, sqrt(x)]} pi(x / p)

use crate::pi_table::PiTable;
use titan_core::roots::isqrt;
use titan_sieve::arena::SieveArena;
use titan_sieve::segment::count_primes_range_with_thresholds;

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
    num_threads: usize,
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

    if num_threads <= 1 || (end_idx - start_idx) < 100 {
        return compute_b_term_thread(x, primes, pi_table, start_idx, end_idx);
    }

    let total_primes = end_idx - start_idx + 1;
    let chunk_size = (total_primes + num_threads - 1) / num_threads;
    let mut thread_sums = vec![0u128; num_threads];

    std::thread::scope(|s| {
        for (t, sum_out) in thread_sums.iter_mut().enumerate() {
            let s_start = start_idx + t * chunk_size;
            let s_end = (start_idx + (t + 1) * chunk_size - 1).min(end_idx);
            if s_start <= end_idx {
                s.spawn(move || {
                    *sum_out = compute_b_term_thread(x, primes, pi_table, s_start, s_end);
                });
            }
        }
    });

    thread_sums.into_iter().sum()
}

fn compute_b_term_thread(
    x: u64,
    primes: &[u64],
    pi_table: &PiTable,
    start_idx: usize,
    end_idx: usize,
) -> u128 {
    let mut sum = 0u128;
    let x_sqrt = isqrt(x);

    for i in (start_idx..=end_idx).rev() {
        let p = primes[i];
        if p > x_sqrt {
            continue;
        }

        let xp = x / p;

        if xp <= x_sqrt {
            sum += pi_table.pi(xp) as u128;
        } else {
            let cnt = count_primes_range_with_segmented_sieve(xp, pi_table);
            sum += cnt as u128;
        }
    }

    sum
}

fn count_primes_range_with_segmented_sieve(n: u64, pi_table: &PiTable) -> u64 {
    if n <= pi_table.max_y {
        return pi_table.pi(n);
    }

    let lo = pi_table.max_y + 1;
    let hi = n;

    let mut threshold_counts = vec![0u64; 1];
    let mut arena = SieveArena::new(hi, 32768);
    let initial_pi = pi_table.pi(pi_table.max_y);

    count_primes_range_with_thresholds(
        lo,
        hi,
        32768,
        &mut arena,
        &[hi],
        &mut threshold_counts,
        initial_pi,
    );

    threshold_counts[0]
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
}