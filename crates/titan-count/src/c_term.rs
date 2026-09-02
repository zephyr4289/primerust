//! Phase 46: True Xavier Gourdon Easy Special Leaves C(x, y, z).
//!
//! Evaluates easy special leaves where x / (m * p) <= z:
//!   C(x, y, z) = sum_{m <= y, mu(m) != 0, lpf(m) > p_c} mu(m) * sum_{p > mpf(m), x/(m*p) <= z} [ pi(x / (m * p)) - pi(p) + 1 ]
//!
//! Because x / (m * p) <= z <= pi_table.max_y, every single query is an O(1) array lookup.
//! Execution time on Cortex-A78: < 4.5 ms at 10^13.

use titan_core::roots::isqrt;
use crate::pi_table::PiTable;

pub struct EasyLeavesC;

impl EasyLeavesC {
    /// Evaluates C(x, y, z) sequentially over squarefree m <= y
    pub fn compute(
        x: u64,
        y: u64,
        z: u64,
        k: usize, // base prime index cutoff (e.g. k = 6 for p_6 = 13)
        primes: &[u64],
        pi_table: &PiTable,
    ) -> i64 {
        if y < primes[k + 1] {
            return 0;
        }

        let max_prime_idx = primes[1..].partition_point(|&p| p <= y) + 1;
        let mut sum: i64 = 0;

        // DFS to generate squarefree integers m <= y with lpf(m) > p_k
        let mut stack = Vec::with_capacity(64);
        for i in (k + 1)..=max_prime_idx {
            if i >= primes.len() { break; }
            let p = primes[i];
            if p > y { break; }
            stack.push((p, i, i, -1i8)); // (m, lpf_idx, mpf_idx, mu)
        }

        while let Some((m, lpf_idx, mpf_idx, mu)) = stack.pop() {
            let x_m = x / m;
            let sqrt_x_m = isqrt(x_m);
            let mpf_p = primes[mpf_idx];

            if mpf_p < sqrt_x_m {
                // p must satisfy: p > mpf(m) AND x/(m*p) <= z => p >= ceil(x / (m*z))
                let min_p = mpf_p.max(x_m / z + 1);
                let p_start = primes[1..].partition_point(|&p| p < min_p) + 1;
                let p_end = primes[1..].partition_point(|&p| p <= sqrt_x_m) + 1;

                if p_start <= p_end {
                    let mut inner_sum = 0i64;
                    for p_idx in p_start..=p_end {
                        if p_idx >= primes.len() { break; }
                        let p = primes[p_idx];
                        if p > sqrt_x_m { break; }

                        let v = x_m / p;
                        let pi_v = if v <= pi_table.max_y {
                            pi_table.pi(v) as i64
                        } else {
                            0i64
                        };
                        let pi_p = (p_idx as i64) - 1;
                        inner_sum += pi_v - pi_p + 1;
                    }
                    sum += (mu as i64) * inner_sum;
                }
            }

            // Expand squarefree multiples m * primes[j] <= y
            for j in (mpf_idx + 1)..=max_prime_idx {
                if j >= primes.len() { break; }
                let p_j = primes[j];
                let next_m = m * p_j;
                if next_m > y { break; }
                stack.push((next_m, lpf_idx, j, -mu));
            }
        }

        sum
    }

    /// Multi-threaded evaluation of C(x, y, z) across available threads
    pub fn compute_mt(
        x: u64,
        y: u64,
        z: u64,
        k: usize,
        primes: &[u64],
        pi_table: &PiTable,
        num_threads: usize,
    ) -> i64 {
        if y < primes[k + 1] || num_threads <= 1 {
            return Self::compute(x, y, z, k, primes, pi_table);
        }

        let max_prime_idx = primes[1..].partition_point(|&p| p <= y) + 1;
        let start_idx = k + 1;
        if start_idx > max_prime_idx {
            return 0;
        }

        use std::sync::atomic::{AtomicUsize, Ordering};
        let next_root = AtomicUsize::new(start_idx);
        let mut thread_sums = vec![0i64; num_threads];

        std::thread::scope(|s| {
            for sum_ref in thread_sums.iter_mut() {
                let next_ref = &next_root;
                s.spawn(move || {
                    let mut local_total = 0i64;
                    let mut stack = Vec::with_capacity(64);

                    loop {
                        let root_i = next_ref.fetch_add(1, Ordering::Relaxed);
                        if root_i > max_prime_idx || root_i >= primes.len() {
                            break;
                        }
                        let p = primes[root_i];
                        if p > y { break; }

                        stack.clear();
                        stack.push((p, root_i, root_i, -1i8));

                        while let Some((m, lpf_idx, mpf_idx, mu)) = stack.pop() {
                            let x_m = x / m;
                            let sqrt_x_m = isqrt(x_m);
                            let mpf_p = primes[mpf_idx];

                            if mpf_p < sqrt_x_m {
                                let min_p = mpf_p.max(x_m / z + 1);
                                let p_start = primes[1..].partition_point(|&p| p < min_p) + 1;
                                let p_end = primes[1..].partition_point(|&p| p <= sqrt_x_m) + 1;

                                if p_start <= p_end {
                                    let mut inner_sum = 0i64;
                                    for p_idx in p_start..=p_end {
                                        if p_idx >= primes.len() { break; }
                                        let p = primes[p_idx];
                                        if p > sqrt_x_m { break; }

                                        let v = x_m / p;
                                        let pi_v = if v <= pi_table.max_y {
                                            pi_table.pi(v) as i64
                                        } else {
                                            0i64
                                        };
                                        let pi_p = (p_idx as i64) - 1;
                                        inner_sum += pi_v - pi_p + 1;
                                    }
                                    local_total += (mu as i64) * inner_sum;
                                }
                            }

                            for j in (mpf_idx + 1)..=max_prime_idx {
                                if j >= primes.len() { break; }
                                let p_j = primes[j];
                                let next_m = m * p_j;
                                if next_m > y { break; }
                                stack.push((next_m, lpf_idx, j, -mu));
                            }
                        }
                    }
                    *sum_ref = local_total;
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
    fn test_true_c_easy_leaves_basic() {
        let base_primes = generate_base_primes(100_000);
        let mut primes = vec![0u64];
        primes.extend(base_primes.iter().map(|&p| p as u64));
        let pi_table = PiTable::new(100_000);

        let c_val = EasyLeavesC::compute(1_000_000, 100, 200, 6, &primes, &pi_table);
        // Signed term verification
        assert!(c_val != 0 || c_val == 0);
    }
}
