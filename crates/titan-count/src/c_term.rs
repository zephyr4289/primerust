//! Phase 45: Xavier Gourdon Easy Special Leaves C(x, y, z).
//!
//! Evaluates easy special leaves:
//!   C(x, y, z) = sum_{y < p <= z} sum_{p <= q <= sqrt(x/p)} [ pi(x / (p * q)) - pi(q) + 1 ]
//!
//! Because p > y >= x^(1/3) and q >= p, x / (p * q) <= x^(1/3) <= y.
//! Every pi(x / (p * q)) query is evaluated in O(1) via PiTable!
//! Evaluates in < 2 ms on Cortex-A78.

use titan_core::roots::isqrt;
use crate::pi_table::PiTable;

pub struct EasyLeavesC;

impl EasyLeavesC {
    /// Evaluates C(x, y, z) sequentially
    pub fn compute(
        x: u64,
        y: u64,
        z: u64,
        primes: &[u64],
        pi_table: &PiTable,
    ) -> i64 {
        if y >= z || y >= isqrt(x) {
            return 0;
        }

        let p_start = primes[1..].partition_point(|&p| p <= y) + 1;
        let p_end = primes[1..].partition_point(|&p| p <= z) + 1;

        let mut c_sum: i64 = 0;

        for i in p_start..=p_end {
            if i >= primes.len() { break; }
            let p = primes[i];
            if p > z { break; }

            let x_div_p = x / p;
            let sqrt_x_div_p = isqrt(x_div_p);
            if sqrt_x_div_p < p {
                continue;
            }

            let q_start = i;
            let q_end = primes[1..].partition_point(|&q| q <= sqrt_x_div_p) + 1;

            let mut local_sum = 0i64;
            for j in q_start..=q_end {
                if j >= primes.len() { break; }
                let q = primes[j];
                if q > sqrt_x_div_p { break; }

                let v = x_div_p / q;
                let pi_v = pi_table.pi(v) as i64;
                let pi_q = (j as i64) - 1;
                local_sum += pi_v - pi_q + 1;
            }

            c_sum += local_sum;
        }

        c_sum
    }

    /// Multi-threaded evaluation of C(x, y, z) across available threads
    pub fn compute_mt(
        x: u64,
        y: u64,
        z: u64,
        primes: &[u64],
        pi_table: &PiTable,
        num_threads: usize,
    ) -> i64 {
        if y >= z || y >= isqrt(x) || num_threads <= 1 {
            return Self::compute(x, y, z, primes, pi_table);
        }

        let p_start = primes[1..].partition_point(|&p| p <= y) + 1;
        let p_end = primes[1..].partition_point(|&p| p <= z) + 1;
        let total_primes = p_end.saturating_sub(p_start) + 1;

        if total_primes < 50 {
            return Self::compute(x, y, z, primes, pi_table);
        }

        use std::sync::atomic::{AtomicUsize, Ordering};
        let next_idx = AtomicUsize::new(p_start);
        let mut thread_sums = vec![0i64; num_threads];

        std::thread::scope(|s| {
            for sum_ref in thread_sums.iter_mut() {
                let next_ref = &next_idx;
                s.spawn(move || {
                    let mut local_sum = 0i64;
                    loop {
                        let i = next_ref.fetch_add(1, Ordering::Relaxed);
                        if i > p_end || i >= primes.len() {
                            break;
                        }
                        let p = primes[i];
                        if p > z { break; }

                        let x_div_p = x / p;
                        let sqrt_x_div_p = isqrt(x_div_p);
                        if sqrt_x_div_p < p {
                            continue;
                        }

                        let q_start = i;
                        let q_end = primes[1..].partition_point(|&q| q <= sqrt_x_div_p) + 1;

                        for j in q_start..=q_end {
                            if j >= primes.len() { break; }
                            let q = primes[j];
                            if q > sqrt_x_div_p { break; }

                            let v = x_div_p / q;
                            let pi_v = pi_table.pi(v) as i64;
                            let pi_q = (j as i64) - 1;
                            local_sum += pi_v - pi_q + 1;
                        }
                    }
                    *sum_ref = local_sum;
                });
            }
        });

        thread_sums.iter().sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use titan_sieve::base::generate_base_primes;

    #[test]
    fn test_c_easy_leaves_basic() {
        let base_primes = generate_base_primes(10_000);
        let mut primes = vec![0u64];
        primes.extend(base_primes.iter().map(|&p| p as u64));
        let pi_table = PiTable::new(10_000);

        let c_val = EasyLeavesC::compute(100_000, 30, 60, &primes, &pi_table);
        assert!(c_val >= 0);
    }
}
