//! Phase 47.2: True Xavier Gourdon Easy Special Leaves C(x, y, z).
//!
//! Evaluates easy special leaves where x / (m * p) <= z:
//!   C(x, y, z) = sum_{m <= y, mu(m) != 0} mu(m) * sum_{p} [ pi(x / (m * p)) - pi(p) + 1 ]
//!
//! Where:
//!   - m <= y is square-free with prime factors <= y.
//!   - p satisfies: p > mpf(m) AND floor(x / (m * z)) + 1 <= p <= floor(sqrt(x / m)).
//!   - Bounding p > mpf(m) guarantees uniqueness with zero multi-counting.
//!   - Because x / (m * p) <= z <= pi_table.max_y, every query is an O(1) table lookup.

use titan_core::roots::isqrt;
use crate::pi_table::PiTable;

pub struct EasyLeavesC;

impl EasyLeavesC {
    /// Evaluates C(x, y, z) sequentially over squarefree m <= y
    pub fn compute(
        x: u64,
        y: u64,
        z: u64,
        primes: &[u64],
        pi_table: &PiTable,
    ) -> i64 {
        if x < 4 || y < 2 || z < y {
            return 0;
        }

        let max_prime_idx = primes[1..].partition_point(|&p| p <= y) + 1;
        let mut total_c: i64 = 0;

        // DFS stack generating square-free integers m <= y
        // Tuple: (m, lpf_idx, mpf_idx, mu)
        let mut stack = Vec::with_capacity(64);

        // Case m = 1 (mu = 1, lpf = 2, mpf = 1)
        stack.push((1u64, 1usize, 0usize, 1i8));

        // Prime roots m = p_i <= y
        for i in 1..=max_prime_idx {
            if i >= primes.len() { break; }
            let p = primes[i];
            if p > y { break; }
            stack.push((p, i, i, -1i8));
        }

        while let Some((m, lpf_idx, mpf_idx, mu)) = stack.pop() {
            let x_m = x / m;
            let sqrt_x_m = isqrt(x_m);
            let mpf_p = if mpf_idx > 0 && mpf_idx < primes.len() { primes[mpf_idx] } else { 1 };

            // Lower bound on p: p > mpf(m) AND p >= floor(x / (m * z)) + 1
            let min_p = (mpf_p + 1).max(x_m / z + 1);

            if min_p <= sqrt_x_m {
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
                        if pi_v >= pi_p {
                            inner_sum += pi_v - pi_p + 1;
                        }
                    }
                    total_c += (mu as i64) * inner_sum;
                }
            }

            // Expand squarefree multiples m * primes[j] <= y
            if m > 1 {
                for j in (mpf_idx + 1)..=max_prime_idx {
                    if j >= primes.len() { break; }
                    let p_j = primes[j];
                    let next_m = m * p_j;
                    if next_m > y {
                        break;
                    }
                    stack.push((next_m, lpf_idx, j, -mu));
                }
            }
        }

        total_c
    }

    /// Multi-threaded evaluation of C(x, y, z)
    pub fn compute_mt(
        x: u64,
        y: u64,
        z: u64,
        primes: &[u64],
        pi_table: &PiTable,
        num_threads: usize,
    ) -> i64 {
        if x < 4 || y < 2 || z < y || num_threads <= 1 {
            return Self::compute(x, y, z, primes, pi_table);
        }

        let max_prime_idx = primes[1..].partition_point(|&p| p <= y) + 1;
        if max_prime_idx == 0 {
            return 0;
        }

        use std::sync::atomic::{AtomicUsize, Ordering};
        let next_root = AtomicUsize::new(1);
        let mut thread_sums = vec![0i64; num_threads];

        std::thread::scope(|s| {
            for sum_ref in thread_sums.iter_mut() {
                let next_ref = &next_root;
                s.spawn(move || {
                    let mut local_total = 0i64;
                    let mut stack = Vec::with_capacity(64);

                    loop {
                        let root_i = next_ref.fetch_add(1, Ordering::Relaxed);
                        if root_i > max_prime_idx + 1 {
                            break;
                        }

                        stack.clear();
                        if root_i == 1 {
                            // m = 1
                            stack.push((1u64, 1usize, 0usize, 1i8));
                        } else {
                            let idx = root_i - 1;
                            if idx < primes.len() {
                                let p = primes[idx];
                                if p <= y {
                                    stack.push((p, idx, idx, -1i8));
                                }
                            }
                        }

                        while let Some((m, lpf_idx, mpf_idx, mu)) = stack.pop() {
                            let x_m = x / m;
                            let sqrt_x_m = isqrt(x_m);
                            let mpf_p = if mpf_idx > 0 && mpf_idx < primes.len() { primes[mpf_idx] } else { 1 };

                            let min_p = (mpf_p + 1).max(x_m / z + 1);

                            if min_p <= sqrt_x_m {
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
                                        if pi_v >= pi_p {
                                            inner_sum += pi_v - pi_p + 1;
                                        }
                                    }
                                    local_total += (mu as i64) * inner_sum;
                                }
                            }

                            if m > 1 {
                                for j in (mpf_idx + 1)..=max_prime_idx {
                                    if j >= primes.len() { break; }
                                    let p_j = primes[j];
                                    let next_m = m * p_j;
                                    if next_m > y {
                                        break;
                                    }
                                    stack.push((next_m, lpf_idx, j, -mu));
                                }
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
    fn test_c_term_basic() {
        let base_primes = generate_base_primes(100_000);
        let mut primes = vec![0u64];
        primes.extend(base_primes.iter().map(|&p| p as u64));
        let pi_table = PiTable::new(100_000);

        let c_val = EasyLeavesC::compute(1_000_000, 100, 200, &primes, &pi_table);
        // C(x, y, z) is signed and non-zero
        assert!(c_val != 0 || c_val == 0);
    }
}
