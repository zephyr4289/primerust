//! P3: Third partial sifting term P3(x, a, c) via prefetch-friendly pi_table walks.
//!
//! Evaluates:
//!   P3(x, a, c) = sum_{i=a+1}^c sum_{j=i}^{pi(sqrt(x/p_i))} [ pi(x / (p_i * p_j)) - (j - 1) ]

use crate::pi_table::PiTable;
use titan_core::roots::isqrt;

pub fn compute_p3(
    x: u64,
    a: usize,
    c: usize,
    primes: &[u64],
    pi_table: &PiTable,
) -> u64 {
    if a >= c {
        return 0; // P3 vanishes when a >= c (Meissel's condition)
    }

    let mut p3_sum = 0u64;

    for i in (a + 1)..=c {
        let p_i = primes[i];
        let x_div_pi = x / p_i;
        let sqrt_x_div_pi = isqrt(x_div_pi);
        let j_max = pi_table.pi(sqrt_x_div_pi) as usize;

        if j_max < i {
            continue;
        }

        let mut inner_sum = 0u64;
        for j in i..=j_max {
            let p_j = primes[j];
            let y = x_div_pi / p_j;
            let pi_y = pi_table.pi(y);
            let term = pi_y - (j as u64 - 1);
            inner_sum += term;
        }
        p3_sum += inner_sum;
    }

    p3_sum
}

/// Multi-threaded evaluation of P3(x, a, c) partitioned over prime index i
pub fn compute_p3_mt(
    x: u64,
    a: usize,
    c: usize,
    primes: &[u64],
    pi_table: &PiTable,
    num_threads: usize,
) -> u64 {
    if a >= c {
        return 0;
    }
    if num_threads <= 1 || (c - a) < 50 {
        return compute_p3(x, a, c, primes, pi_table);
    }

    use std::sync::atomic::{AtomicUsize, Ordering};
    let next_i = AtomicUsize::new(a + 1);
    let mut thread_sums = vec![0u64; num_threads];

    std::thread::scope(|s| {
        for sum_ref in thread_sums.iter_mut() {
            let next_i_ref = &next_i;
            s.spawn(move || {
                let mut local_sum = 0u64;
                loop {
                    let i = next_i_ref.fetch_add(1, Ordering::Relaxed);
                    if i > c {
                        break;
                    }
                    let p_i = primes[i];
                    let x_div_pi = x / p_i;
                    let sqrt_x_div_pi = isqrt(x_div_pi);
                    let j_max = pi_table.pi(sqrt_x_div_pi) as usize;

                    if j_max < i {
                        continue;
                    }

                    let mut inner_sum = 0u64;
                    for j in i..=j_max {
                        let p_j = primes[j];
                        let y = x_div_pi / p_j;
                        let pi_y = pi_table.pi(y);
                        let term = pi_y - (j as u64 - 1);
                        inner_sum += term;
                    }
                    local_sum += inner_sum;
                }
                *sum_ref = local_sum;
            });
        }
    });

    thread_sums.into_iter().sum()
}
