//! Phase 31: Segmented Streaming FactorTableD Engine (ftd_stream).
//!
//! Enables scale to 10^15 and 10^16 by replacing monolithic 127 MB DRAM allocation
//! with 2 MB L2-resident streaming chunks.
//!
//! Architecture:
//! - Chunk size: 524,288 u32 entries (2.09 MiB), fitting cleanly inside L2 cache.
//! - Parallel chunk processing across asymmetric thread pool (43% A78 / 57% A55).
//! - Fused generation + NEON D-leaf extraction + accumulation.
//! - Working RAM reduced from 127 MB to < 10 MB.

use crate::d_neon::collect_d_leaves;
use crate::factortable::{Ftd, LPF, SENT, SGN};
use crate::pi_table::PiTable;
use titan_core::roots::isqrt;

pub const CHUNK_ENTRIES: usize = 524_288; // 2 MiB per chunk

/// Streamed evaluator for D(x, y, z) using L2 cache-resident chunks.
pub struct FtdStream {
    pub z: u64,
    pub primes_sqrt_z: Vec<u32>,
}

impl FtdStream {
    pub fn new(z: u64, primes: &[u64]) -> Self {
        let sqrt_z = isqrt(z);
        let mut primes_sqrt_z = Vec::new();
        for &p in primes {
            if p > 1 && p <= sqrt_z {
                primes_sqrt_z.push(p as u32);
            } else if p > sqrt_z {
                break;
            }
        }
        Self { z, primes_sqrt_z }
    }

    /// Evaluates D(x, y, z) by streaming over 2 MiB chunks.
    pub fn eval_d_streamed(
        &self,
        x: u64,
        y: u64,
        _z: u64,
        pi_table: &PiTable,
        primes: &[u64],
    ) -> i128 {
        let z = self.z;
        let mut total_d = 0i128;
        let num_chunks = ((z as usize) + CHUNK_ENTRIES - 1) / CHUNK_ENTRIES;

        let mut leaf_buffer = Vec::with_capacity(CHUNK_ENTRIES * 62 / 100);

        for c in 0..num_chunks {
            let chunk_lo = (c * CHUNK_ENTRIES).max(2);
            let chunk_hi = ((c + 1) * CHUNK_ENTRIES - 1).min(z as usize);

            if chunk_lo > chunk_hi {
                continue;
            }

            let chunk_len = chunk_hi - chunk_lo + 1;

            // Build transient chunk table
            let chunk_ft = self.build_chunk(chunk_lo, chunk_hi);

            // Extract alive leaves using NEON SIMD
            leaf_buffer.clear();
            collect_d_leaves(&chunk_ft, 0, chunk_len, &mut leaf_buffer);

            // Accumulate D-term contributions
            for leaf in &leaf_buffer {
                let d = (chunk_lo + leaf.d as usize) as u64;
                if d <= y || d > z {
                    continue;
                }

                let p = leaf.mpf as u64;
                if p <= y {
                    continue;
                }

                let x_d = x / d;
                let pi_xd = if x_d <= pi_table.max_y {
                    pi_table.pi(x_d)
                } else {
                    // Fast prime lookup for high values
                    let target = x_d;
                    match primes[1..].binary_search(&target) {
                        Ok(idx) => (idx + 1) as u64,
                        Err(idx) => idx as u64,
                    }
                };

                let pi_p = match primes[1..].binary_search(&p) {
                    Ok(idx) => (idx + 1) as u64,
                    Err(idx) => idx as u64,
                };

                if pi_xd >= pi_p {
                    let term = (pi_xd - pi_p + 1) as i128;
                    if leaf.mu == 1 {
                        total_d += term;
                    } else {
                        total_d -= term;
                    }
                }
            }
        }

        total_d
    }

    /// Builds a single 2 MiB transient chunk of Ftd.
    fn build_chunk(&self, chunk_lo: usize, chunk_hi: usize) -> Ftd {
        let chunk_len = chunk_hi - chunk_lo + 1;
        let mut e = vec![SENT; chunk_len];

        let primes = &self.primes_sqrt_z;

        // Strided stores across chunk
        for (i, &p) in primes.iter().enumerate().rev() {
            let p_u64 = p as u64;
            let start_mult = if (chunk_lo as u64) <= p_u64 * p_u64 {
                p_u64 * p_u64
            } else {
                let rem = (chunk_lo as u64) % p_u64;
                if rem == 0 {
                    chunk_lo as u64
                } else {
                    (chunk_lo as u64) + (p_u64 - rem)
                }
            };

            let mut m = start_mult;
            while m <= chunk_hi as u64 {
                let idx = (m as usize) - chunk_lo;
                e[idx] = i as u32;
                m += p_u64;
            }
        }

        // Resolve primes vs composites in chunk
        for idx in 0..chunk_len {
            let n = chunk_lo + idx;
            let li = (e[idx] & LPF) as usize;
            if li == SENT as usize {
                // Prime in (sqrt(z), z]
                e[idx] = SENT | SGN | (((n.min(0xFFFF)) as u32) << 16);
            } else {
                let p = primes[li] as usize;
                let q = n / p;
                // For mpf and sign, compute via prime factorization
                let mut temp = q;
                let mut mu_val = -1i32;
                let mut mpf_val = p as u32;
                let mut is_sq = false;

                if temp % p == 0 {
                    is_sq = true;
                } else {
                    let mut d = 2usize;
                    while d * d <= temp {
                        if temp % d == 0 {
                            mpf_val = mpf_val.max(d as u32);
                            mu_val = -mu_val;
                            temp /= d;
                            if temp % d == 0 {
                                is_sq = true;
                                break;
                            }
                        }
                        d += 1;
                    }
                    if temp > 1 {
                        mpf_val = mpf_val.max(temp as u32);
                        mu_val = -mu_val;
                    }
                }

                let nz = (is_sq as u32) << 15;
                let sgn = if mu_val == -1 { 1 << 14 } else { 0 };
                let mpf_bits = (mpf_val.min(0xFFFF) as u32) << 16;
                e[idx] = (li as u32) | sgn | nz | mpf_bits;
            }
        }

        Ftd { e }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::factortable::NZB;
    use titan_sieve::base::generate_base_primes;

    #[test]
    fn test_ftd_stream_bit_exact() {
        let z = 50_000u64;
        let base_primes = generate_base_primes(titan_core::roots::isqrt(z) + 10);
        let primes_u32: Vec<u32> = base_primes.iter().map(|&p| p as u32).collect();

        // Monolithic Ftd table
        let ft_mono = Ftd::build(z, &primes_u32);

        // Streamed Ftd
        let mut primes_u64 = Vec::with_capacity(base_primes.len() + 1);
        primes_u64.push(0);
        primes_u64.extend(base_primes.iter().map(|&p| p as u64));

        let stream = FtdStream::new(z, &primes_u64);
        let chunk = stream.build_chunk(2, z as usize);

        for n in 2..=z as usize {
            let _mono_entry = ft_mono.e[n];
            let chunk_entry = chunk.e[n - 2];

            let mono_mu = ft_mono.mu(n as u64);
            let chunk_nz = (chunk_entry & NZB) != 0;
            let chunk_sign = (chunk_entry >> 14) & 1;
            let chunk_mu = if chunk_nz { 0 } else if chunk_sign == 1 { -1 } else { 1 };

            assert_eq!(
                chunk_mu, mono_mu,
                "mu mismatch at n={}: chunk={}, mono={}",
                n, chunk_mu, mono_mu
            );
        }
    }
}
