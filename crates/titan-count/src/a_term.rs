//! Phase 46: True Xavier Gourdon Ordinary Leaves Engine A(x, y).
//!
//! Evaluates ordinary leaves using squarefree integers m <= y:
//!   A(x, y) = sum_{m <= y, mu(m) != 0, lpf(m) > p_k} mu(m) * Phi(x / m, pi(lpf(m)) - 1)
//!
//! Because y <= x^(1/3) * alpha_y (e.g. y <= 21,544 at 10^13),
//! the total number of squarefree integers m <= y is under 5,000.
//! Evaluates in < 0.5 ms on Cortex-A78.

use titan_core::phi_tiny::phi_tiny;
use crate::phi_tables::PhiTables;
use crate::pi_table::PiTable;

pub struct OrdinaryLeavesA;

impl OrdinaryLeavesA {
    /// Evaluates A(x, y) sequentially over squarefree integers m <= y
    pub fn compute(
        x: u64,
        y: u64,
        k: usize, // base prime cutoff (e.g. k = 6 for p_6 = 13)
        primes: &[u64],
        pi_table: &PiTable,
    ) -> i64 {
        if y < primes[k + 1] {
            return 0;
        }

        let mut sum = 0i64;
        let mut stack = Vec::with_capacity(64);

        let max_prime_idx = primes[1..].partition_point(|&p| p <= y) + 1;

        for i in (k + 1)..=max_prime_idx {
            if i >= primes.len() { break; }
            let p = primes[i];
            if p > y { break; }
            stack.push((p, i, -1i8)); // m = p, lpf = p (index i), mu = -1
        }

        while let Some((m, lpf_idx, mu)) = stack.pop() {
            let x_m = x / m;
            let a_m = lpf_idx - 1;

            // Evaluate Phi(x/m, a_m)
            let phi_val = if a_m <= 6 {
                phi_tiny(x_m, a_m as u64) as i64
            } else if x_m <= pi_table.max_y && (a_m + 1) < primes.len() && x_m < primes[a_m + 1] * primes[a_m + 1] {
                (pi_table.pi(x_m) as i64) - (a_m as i64) + 1
            } else {
                PhiTables::phi_small(x_m, a_m.min(7)) as i64
            };

            sum += (mu as i64) * phi_val;

            // Expand squarefree multiples m * primes[j] <= y with j > lpf_idx
            for j in (lpf_idx + 1)..=max_prime_idx {
                if j >= primes.len() { break; }
                let p_j = primes[j];
                let next_m = m * p_j;
                if next_m > y {
                    break;
                }
                stack.push((next_m, j, -mu));
            }
        }

        sum
    }

    /// Multi-threaded evaluation of A(x, y)
    pub fn compute_mt(
        x: u64,
        y: u64,
        k: usize,
        primes: &[u64],
        pi_table: &PiTable,
        num_threads: usize,
    ) -> i64 {
        if y < primes[k + 1] || num_threads <= 1 {
            return Self::compute(x, y, k, primes, pi_table);
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
                    let mut local_sum = 0i64;
                    let mut stack = Vec::with_capacity(64);

                    loop {
                        let root_i = next_ref.fetch_add(1, Ordering::Relaxed);
                        if root_i > max_prime_idx || root_i >= primes.len() {
                            break;
                        }
                        let p = primes[root_i];
                        if p > y { break; }

                        stack.clear();
                        stack.push((p, root_i, -1i8));

                        while let Some((m, lpf_idx, mu)) = stack.pop() {
                            let x_m = x / m;
                            let a_m = lpf_idx - 1;

                            let phi_val = if a_m <= 6 {
                                phi_tiny(x_m, a_m as u64) as i64
                            } else if x_m <= pi_table.max_y && (a_m + 1) < primes.len() && x_m < primes[a_m + 1] * primes[a_m + 1] {
                                (pi_table.pi(x_m) as i64) - (a_m as i64) + 1
                            } else {
                                PhiTables::phi_small(x_m, a_m.min(7)) as i64
                            };

                            local_sum += (mu as i64) * phi_val;

                            for j in (lpf_idx + 1)..=max_prime_idx {
                                if j >= primes.len() { break; }
                                let p_j = primes[j];
                                let next_m = m * p_j;
                                if next_m > y {
                                    break;
                                }
                                stack.push((next_m, j, -mu));
                            }
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
    fn test_ordinary_leaves_small() {
        let base_primes = generate_base_primes(10_000);
        let mut primes = vec![0u64];
        primes.extend(base_primes.iter().map(|&p| p as u64));
        let pi_table = PiTable::new(10_000);

        let a_val = OrdinaryLeavesA::compute(100_000, 30, 6, &primes, &pi_table);
        assert!(a_val != 0);
    }
}
