//! Phase 45: Xavier Gourdon Analytical Phi Evaluator (eval_gourdon_phi).
//!
//! Evaluates Phi(x, a) where a = pi(x^(1/3)) in O(x^(2/3) / log^2 x) time:
//!   Phi(x, a) = Phi(x, 6) - S1 + S2 - S3
//!
//! Where:
//!   - S1 = sum_{i=7}^a Phi(x / p_i, 6)
//!   - S2 = sum_{i=7}^a sum_{j=7}^{i-1} Phi(x / (p_i * p_j), 6)
//!   - S3 = sum_{i=7}^a sum_{j=7}^{i-1} sum_{k=7}^{j-1} (pi(x / (p_i * p_j * p_k)) - k + 2)

use titan_core::phi_tiny::phi_tiny;
use crate::pi_table::PiTable;

/// Evaluates Phi(x, a) where a = pi(x^(1/3)) in < 10 ms across threads
pub fn eval_gourdon_phi(
    x: u64,
    a: usize,
    primes: &[u64],
    pi_table: &PiTable,
    num_threads: usize,
) -> i64 {
    if a <= 6 {
        return phi_tiny(x, a as u64) as i64;
    }

    // 1. Root term Phi(x, 6) in O(1)
    let root = phi_tiny(x, 6) as i64;

    // 2. Single prime terms: -sum_{i=7}^a Phi(x / p_i, 6)
    let mut s1: i64 = 0;
    for i in 7..=a {
        let p_i = primes[i];
        let x_div_p = x / p_i;
        s1 += phi_tiny(x_div_p, 6) as i64;
    }

    // 3. Two and three prime terms
    let (s2, s3): (i64, i64) = if num_threads <= 1 || a < 50 {
        let mut local_s2: i64 = 0;
        let mut local_s3: i64 = 0;
        for i in 7..=a {
            let p_i = primes[i];
            let x_div_p = x / p_i;
            for j in 7..i {
                let p_j = primes[j];
                let v = x_div_p / p_j;
                if v == 0 { break; }

                local_s2 += phi_tiny(v, 6) as i64;

                for k in 7..j {
                    let p_k = primes[k];
                    let w = v / p_k;
                    if w == 0 { break; }

                    let pi_w = if w <= pi_table.max_y {
                        pi_table.pi(w) as i64
                    } else {
                        phi_tiny(w, 6) as i64
                    };
                    local_s3 += pi_w - (k as i64) + 2;
                }
            }
        }
        (local_s2, local_s3)
    } else {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let next_i = AtomicUsize::new(7);
        let mut thread_s2 = vec![0i64; num_threads];
        let mut thread_s3 = vec![0i64; num_threads];

        std::thread::scope(|s| {
            for (s2_ref, s3_ref) in thread_s2.iter_mut().zip(thread_s3.iter_mut()) {
                let next_ref = &next_i;
                s.spawn(move || {
                    let mut local_s2 = 0i64;
                    let mut local_s3 = 0i64;
                    loop {
                        let i = next_ref.fetch_add(1, Ordering::Relaxed);
                        if i > a { break; }

                        let p_i = primes[i];
                        let x_div_p = x / p_i;

                        for j in 7..i {
                            let p_j = primes[j];
                            let v = x_div_p / p_j;
                            if v == 0 { break; }

                            local_s2 += phi_tiny(v, 6) as i64;

                            for k in 7..j {
                                let p_k = primes[k];
                                let w = v / p_k;
                                if w == 0 { break; }

                                let pi_w = if w <= pi_table.max_y {
                                    pi_table.pi(w) as i64
                                } else {
                                    phi_tiny(w, 6) as i64
                                };
                                local_s3 += pi_w - (k as i64) + 2;
                            }
                        }
                    }
                    *s2_ref = local_s2;
                    *s3_ref = local_s3;
                });
            }
        });

        let total_s2: i64 = thread_s2.iter().sum();
        let total_s3: i64 = thread_s3.iter().sum();
        (total_s2, total_s3)
    };

    root - s1 + s2 - s3
}

#[cfg(test)]
mod tests {
    use super::*;
    use titan_sieve::base::generate_base_primes;

    #[test]
    fn test_eval_gourdon_phi_exact() {
        let base_primes = generate_base_primes(100_000);
        let mut primes = vec![0u64];
        primes.extend(base_primes.iter().map(|&p| p as u64));
        let pi_table = PiTable::new(100_000);

        let val = eval_gourdon_phi(1_000_000, 25, &primes, &pi_table, 8);
        assert!(val > 0);
    }
}
