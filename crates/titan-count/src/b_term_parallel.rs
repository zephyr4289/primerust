//! Phase 5.0: Dual-Core Cortex-A78 Parallel B(x, y) Sweep (b_term_parallel.rs).
//!
//! Equi-work harmonic split across Cortex-A78 big cores (Cores 6 and 7)
//! with streaming SBRB reciprocal blocks and 4-way ILP pipelined division.

use crate::b_term::{StreamingReciprocalBuffer, RECIPROCAL_BLOCK_SIZE};
use crate::pi_table::PiTable;
use crate::sampled_index::SampledPrimeIndex;
use titan_core::affinity::pin_thread_to_core;
use titan_core::roots::isqrt;

pub fn compute_b_parallel_a78(
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

    let p_start = if primes.first() == Some(&0) {
        sampled_idx.pi(prime_slice, y) as usize + 1
    } else {
        sampled_idx.pi(prime_slice, y) as usize
    };
    let p_end = if primes.first() == Some(&0) {
        sampled_idx.pi(prime_slice, sqrt_x) as usize + 1
    } else {
        sampled_idx.pi(prime_slice, sqrt_x) as usize
    };
    if p_start >= p_end {
        return 0;
    }

    let total = p_end - p_start;
    // Harmonic split: lower primes have wider spans in pi(x/p)
    let split = p_start + (total * 38 / 100);

    // Run parallel on Cores 6 and 7 (DynamIQ Cortex-A78 big cores)
    std::thread::scope(|s| {
        let h7 = s.spawn(|| {
            pin_thread_to_core(7);
            sweep_range(x, split, p_end, primes, pi_table, &sampled_idx, prime_slice)
        });

        pin_thread_to_core(6);
        let sum6 = sweep_range(x, p_start, split, primes, pi_table, &sampled_idx, prime_slice);
        let sum7 = h7.join().unwrap();

        sum6 + sum7
    })
}

fn sweep_range(
    x: u64,
    start_idx: usize,
    end_idx: usize,
    primes: &[u64],
    pi_table: &PiTable,
    sampled_idx: &SampledPrimeIndex,
    prime_slice: &[u64],
) -> i64 {
    if start_idx >= end_idx {
        return 0;
    }
    let active_slice = &primes[start_idx..end_idx];
    let total = active_slice.len();

    // 1. Gauss Closed-Form Arithmetic Progression in O(1)
    let a = start_idx as i64;
    let b = (end_idx - 1) as i64;
    let n = b - a + 1;
    let sum_pi_p = (a + b) * n / 2;
    let sum_ones = n;

    let mut sbrb = StreamingReciprocalBuffer::new();
    let mut sum_pi_quotients: i64 = 0;
    let pi_max = pi_table.max_y;

    let mut chunk_start = 0;
    while chunk_start < total {
        let chunk_end = (chunk_start + RECIPROCAL_BLOCK_SIZE).min(total);
        let slice = &active_slice[chunk_start..chunk_end];
        let len = slice.len();

        sbrb.fill_block(slice, x);

        let mut i = 0;
        while i + 4 <= len {
            let d0 = unsafe { sbrb.table.get_unchecked(i) };
            let d1 = unsafe { sbrb.table.get_unchecked(i + 1) };
            let d2 = unsafe { sbrb.table.get_unchecked(i + 2) };
            let d3 = unsafe { sbrb.table.get_unchecked(i + 3) };

            let q0 = d0.div(x);
            let q1 = d1.div(x);
            let q2 = d2.div(x);
            let q3 = d3.div(x);

            let pi0 = if q0 <= pi_max {
                pi_table.pi(q0) as i64
            } else {
                sampled_idx.pi(prime_slice, q0) as i64
            };
            let pi1 = if q1 <= pi_max {
                pi_table.pi(q1) as i64
            } else {
                sampled_idx.pi(prime_slice, q1) as i64
            };
            let pi2 = if q2 <= pi_max {
                pi_table.pi(q2) as i64
            } else {
                sampled_idx.pi(prime_slice, q2) as i64
            };
            let pi3 = if q3 <= pi_max {
                pi_table.pi(q3) as i64
            } else {
                sampled_idx.pi(prime_slice, q3) as i64
            };

            sum_pi_quotients += pi0 + pi1 + pi2 + pi3;
            i += 4;
        }

        while i < len {
            let d = unsafe { sbrb.table.get_unchecked(i) };
            let q = d.div(x);
            let pi_q = if q <= pi_max {
                pi_table.pi(q) as i64
            } else {
                sampled_idx.pi(prime_slice, q) as i64
            };
            sum_pi_quotients += pi_q;
            i += 1;
        }

        chunk_start = chunk_end;
    }

    sum_pi_quotients - sum_pi_p + sum_ones
}

#[cfg(test)]
mod tests {
    use super::*;
    use titan_core::roots::{icbrt, isqrt};
    use titan_sieve::base::generate_base_primes;
    use crate::b_term::compute_b_monotone;

    #[test]
    fn test_b_parallel_a78_exactness() {
        let x = 10_000_000u64;
        let x_sqrt = isqrt(x);
        let y = icbrt(x);

        let base_primes = generate_base_primes(x_sqrt + 100);
        let mut primes = vec![0u64];
        primes.extend_from_slice(&base_primes);

        let pi_table = PiTable::new(x_sqrt + 30);
        let expected = compute_b_monotone(x, y, &primes, &pi_table);
        let actual = compute_b_parallel_a78(x, y, &primes, &pi_table);

        assert_eq!(actual, expected, "B-term parallel A78 mismatch: expected {}, got {}", expected, actual);
    }
}
