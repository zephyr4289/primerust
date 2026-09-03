//! Phase 2.1 / Phase 3.1 / Phase 4.2: Fused Ordinary & Easy Special Leaves AC(x, y, z) (ac_term.rs).
//!
//! Evaluates Fused Ordinary + Easy Special Leaves AC(x, y, z).
//! Accelerates inner quotient evaluation via 64-bit Granlund-Montgomery reciprocal
//! multiplication (`umulh` + `lsr`) with 4-way ILP dual-issue pipeline unrolling.
//!
//! Phase 4.2: Guided Adaptive Dynamic Work Claiming (64 dense / 256 sparse)
//! and zero-atomic thread-local register accumulation with core affinity pinning.

use crate::factor_table::CompressedFactorTable;
use crate::magic_reciprocal::FastDivTable;
use crate::pi_table::PiTable;
use crate::sampled_index::SampledPrimeIndex;
use std::sync::atomic::{AtomicU64, Ordering};
use titan_core::affinity::pin_thread_to_core;

#[repr(C, align(64))]
pub struct AcWorkDispenser {
    cursor: AtomicU64,
    y: u64,
    dense_threshold: u64,
}

impl AcWorkDispenser {
    pub fn new(y: u64) -> Self {
        Self {
            cursor: AtomicU64::new(1),
            y,
            dense_threshold: (y / 20).max(64), // First 5% of m is dense
        }
    }

    #[inline(always)]
    pub fn claim_chunk(&self) -> Option<(u64, u64)> {
        let mut curr = self.cursor.load(Ordering::Relaxed);

        loop {
            if curr > self.y {
                return None;
            }

            // Adaptive chunking: 64 for dense head, 256 for sparse tail
            let chunk_size = if curr <= self.dense_threshold {
                64
            } else {
                256
            };

            let next = (curr + chunk_size).min(self.y + 1);

            match self.cursor.compare_exchange_weak(
                curr,
                next,
                Ordering::AcqRel,
                Ordering::Relaxed,
            ) {
                Ok(_) => return Some((curr, next)),
                Err(actual) => curr = actual,
            }
        }
    }
}

/// Evaluates Fused Leaves AC(x, y, z) with zero hot-path atomics
/// and chunked work claiming across the DynamIQ cluster.
pub fn compute_ac_fused(
    x: u64,
    y: u64,
    z: u64,
    primes: &[u64],
    pi_table: &PiTable,
    mu: &[i8],
    num_threads: usize,
) -> i64 {
    let threads = num_threads.max(1);
    let dispenser = AcWorkDispenser::new(y);

    // Precompute reciprocal multiplication table for all primes <= max_prime
    let div_table = FastDivTable::build(primes, x);

    // Precompute greatest prime factor for all m <= y via Euler's linear sieve in O(y) (L3 compressed)
    let factor_table = CompressedFactorTable::new(y as usize);
    let prime_slice = if primes.first() == Some(&0) { &primes[1..] } else { primes };
    let sampled_idx = SampledPrimeIndex::build(prime_slice);

    std::thread::scope(|s| {
        let mut handles = Vec::with_capacity(threads);

        for core_id in 0..threads {
            let disp_ref = &dispenser;
            let div_slice = div_table.as_slice();
            let ft_ref = &factor_table;
            let s_idx = &sampled_idx;

            handles.push(s.spawn(move || {
                pin_thread_to_core(core_id);
                let mut thread_total: i64 = 0;

                while let Some((start_m, end_m)) = disp_ref.claim_chunk() {
                    let mut chunk_sum: i64 = 0;

                    for m in start_m..end_m {
                        let mu_m = mu[m as usize];
                        if mu_m == 0 { continue; }

                        // O(1) FactorTable lookup (Phase 4.1)
                        let gpf_m = ft_ref.gpf(m);
                        let x_div_m = x / m;
                        let p_min_bound = (x_div_m / z).max(gpf_m);
                        let p_max_bound = titan_core::roots::isqrt(x_div_m);

                        if p_min_bound >= p_max_bound { continue; }

                        let p_start_idx = s_idx.pi(prime_slice, p_min_bound) as usize + 1;
                        let p_end_idx = s_idx.pi(prime_slice, p_max_bound) as usize + 1;

                        let mut m_term: i64 = 0;
                        let mut i = p_start_idx;

                        // 4-Way Pipelined ILP Unrolling via umulh (Dual ALU pipelines on Cortex-A78)
                        while i + 4 <= p_end_idx {
                            let d0 = unsafe { div_slice.get_unchecked(i) };
                            let d1 = unsafe { div_slice.get_unchecked(i + 1) };
                            let d2 = unsafe { div_slice.get_unchecked(i + 2) };
                            let d3 = unsafe { div_slice.get_unchecked(i + 3) };

                            // Concurrently evaluate 4 reciprocal divisions
                            let v0 = d0.div(x_div_m);
                            let v1 = d1.div(x_div_m);
                            let v2 = d2.div(x_div_m);
                            let v3 = d3.div(x_div_m);

                            // Parallel pi lookups via SampledPrimeIndex
                            let pi_v0 = if v0 <= pi_table.max_y {
                                pi_table.pi(v0) as i64
                            } else {
                                s_idx.pi(prime_slice, v0) as i64
                            };
                            let pi_v1 = if v1 <= pi_table.max_y {
                                pi_table.pi(v1) as i64
                            } else {
                                s_idx.pi(prime_slice, v1) as i64
                            };
                            let pi_v2 = if v2 <= pi_table.max_y {
                                pi_table.pi(v2) as i64
                            } else {
                                s_idx.pi(prime_slice, v2) as i64
                            };
                            let pi_v3 = if v3 <= pi_table.max_y {
                                pi_table.pi(v3) as i64
                            } else {
                                s_idx.pi(prime_slice, v3) as i64
                            };

                            let pi_primes = (i + (i + 1) + (i + 2) + (i + 3)) as i64;
                            m_term += (pi_v0 + pi_v1 + pi_v2 + pi_v3) - pi_primes + 4;
                            i += 4;
                        }

                        // Tail primes
                        while i < p_end_idx {
                            let d = unsafe { div_slice.get_unchecked(i) };
                            let v = d.div(x_div_m);

                            let pi_v = if v <= pi_table.max_y {
                                pi_table.pi(v) as i64
                            } else {
                                s_idx.pi(prime_slice, v) as i64
                            };
                            let pi_p = i as i64; // 1-based prime index

                            m_term += pi_v - pi_p + 1;
                            i += 1;
                        }

                        if mu_m == 1 {
                            chunk_sum += m_term;
                        } else {
                            chunk_sum -= m_term;
                        }
                    }

                    thread_total += chunk_sum;
                }

                thread_total
            }));
        }

        let mut total_ac: i64 = 0;
        for h in handles {
            total_ac += h.join().unwrap();
        }
        total_ac
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use titan_sieve::base::generate_base_primes;

    #[test]
    fn test_ac_fused_small() {
        let x = 100_000u64;
        let y = 100u64;
        let z = 200u64;

        let base_primes = generate_base_primes(2000);
        let mut primes = vec![0u64];
        primes.extend_from_slice(&base_primes);

        let pi_table = PiTable::new(1000);
        let mut mu = vec![0i8; y as usize + 1];
        let stream = crate::mobius_stream::MobiusStream::new(y);
        for (d, m) in stream {
            if (d as usize) < mu.len() {
                mu[d as usize] = m;
            }
        }

        let ac = compute_ac_fused(x, y, z, &primes, &pi_table, &mu, 4);
        assert!(ac >= 0);
    }

    #[test]
    fn test_ac_work_dispenser_partition() {
        let y = 10_000;
        let dispenser = AcWorkDispenser::new(y);
        let mut total_covered = 0;
        let mut prev_end = 1;
        while let Some((start, end)) = dispenser.claim_chunk() {
            assert_eq!(start, prev_end);
            assert!(end > start);
            total_covered += end - start;
            prev_end = end;
        }
        assert_eq!(prev_end, y + 1);
        assert_eq!(total_covered, y);
    }
}
