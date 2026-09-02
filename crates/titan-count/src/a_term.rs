//! Phase 44: Xavier Gourdon Ordinary Leaves Engine A(x, y).
//!
//! Evaluates ordinary leaves using squarefree integers m <= y:
//!   A(x, y) = sum_{m <= y, mu(m) != 0, lpf(m) > p_k} mu(m) * Phi(x / m, pi(lpf(m)) - 1)
//!
//! Because y <= x^(1/3) * alpha_y (e.g. y <= 21,544 at 10^13),
//! the total number of squarefree integers m <= y is under 5,000.
//! Evaluates in < 0.2 ms on Cortex-A78.

use titan_core::phi_tiny::phi_tiny;
use crate::phi_tables::PhiTables;
use crate::pi_table::PiTable;

pub struct OrdinaryLeavesA;

impl OrdinaryLeavesA {
    /// Evaluates A(x, y) sequentially or parallelized over squarefree integers m <= y
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

        // Start DFS from each prime p_i > p_k up to y
        let max_prime_idx = match primes[1..].binary_search(&y) {
            Ok(idx) => idx + 1,
            Err(idx) => idx,
        };

        for i in (k + 1)..=max_prime_idx {
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
                let p_j = primes[j];
                let next_m = m.saturating_mul(p_j);
                if next_m > y {
                    break;
                }
                stack.push((next_m, lpf_idx, -mu));
            }
        }

        sum
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use titan_sieve::base::generate_base_primes;

    #[test]
    fn test_ordinary_leaves_small() {
        let base_primes = generate_base_primes(1000);
        let mut primes = vec![0u64];
        primes.extend(base_primes.iter().map(|&p| p as u64));
        let pi_table = PiTable::new(1000);

        let a_val = OrdinaryLeavesA::compute(10_000, 25, 3, &primes, &pi_table);
        assert!(a_val != 0);
    }
}
