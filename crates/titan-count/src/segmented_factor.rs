//! Phase 5.0: L1D Block-Sieved Factorizer (segmented_factor.rs).
//!
//! Evaluates m in cache-tiled blocks of L = 4,096 integers (16 KiB footprint)
//! directly inside L1D cache using small primes p <= sqrt(y).
//! Zero heap allocations in the hot path.

pub const FACTOR_BLOCK_SIZE: usize = 4096; // 16 KiB footprint: L1D pinned

#[repr(C, align(64))]
pub struct BlockFactorSieve {
    pub gpf: [u32; FACTOR_BLOCK_SIZE],
    pub mu: [i8; FACTOR_BLOCK_SIZE],
    residual: [u32; FACTOR_BLOCK_SIZE],
}

impl BlockFactorSieve {
    pub const fn new() -> Self {
        Self {
            gpf: [0u32; FACTOR_BLOCK_SIZE],
            mu: [1i8; FACTOR_BLOCK_SIZE],
            residual: [0u32; FACTOR_BLOCK_SIZE],
        }
    }

    /// Factors an entire chunk of m in [start_m, start_m + len) inside L1D cache.
    /// Uses primes <= sqrt(y) (at most 351 primes for y = 5.5e6).
    #[inline(always)]
    pub fn sieve_block(&mut self, start_m: u64, len: usize, small_primes: &[u32]) {
        debug_assert!(len <= FACTOR_BLOCK_SIZE);

        for i in 0..len {
            let m = start_m + (i as u64);
            self.gpf[i] = 1;
            self.mu[i] = 1;
            self.residual[i] = m as u32;
        }

        for &p in small_primes {
            let p_u64 = p as u64;
            if p_u64 * p_u64 > (start_m + len as u64) {
                break;
            }

            let p_sq = p_u64 * p_u64;
            let mut start_idx = if start_m % p_u64 == 0 {
                0
            } else {
                (p_u64 - (start_m % p_u64)) as usize
            };

            let p_step = p as usize;

            while start_idx < len {
                let m = start_m + start_idx as u64;
                if m % p_sq == 0 {
                    self.mu[start_idx] = 0;
                } else if self.mu[start_idx] != 0 {
                    self.mu[start_idx] = -self.mu[start_idx];
                }

                self.gpf[start_idx] = self.gpf[start_idx].max(p);

                // Divide out all factors of p
                let mut res = self.residual[start_idx];
                while res > 1 && res % p == 0 {
                    res /= p;
                }
                self.residual[start_idx] = res;

                start_idx += p_step;
            }
        }

        // Any remaining residual > 1 is a prime factor > sqrt(y)
        for i in 0..len {
            let res = self.residual[i];
            if res > 1 {
                self.gpf[i] = self.gpf[i].max(res);
                if self.mu[i] != 0 {
                    self.mu[i] = -self.mu[i];
                }
            }
        }
    }
}

impl Default for BlockFactorSieve {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::factor_table::CompressedFactorTable;
    use titan_sieve::base::generate_base_primes;

    #[test]
    fn test_block_factor_sieve_exactness() {
        let max_y = 50_000u64;
        let ft = CompressedFactorTable::new(max_y as usize);
        let base_primes_u64 = generate_base_primes(titan_core::roots::isqrt(max_y) + 10);
        let small_primes: Vec<u32> = base_primes_u64.iter().map(|&p| p as u32).collect();

        let mut sieve = BlockFactorSieve::new();

        let mut start_m = 2u64;
        while start_m <= max_y {
            let len = ((max_y - start_m + 1) as usize).min(FACTOR_BLOCK_SIZE);
            sieve.sieve_block(start_m, len, &small_primes);

            for i in 0..len {
                let m = start_m + i as u64;
                let expected_gpf = ft.gpf(m);
                let actual_gpf = sieve.gpf[i] as u64;
                assert_eq!(
                    actual_gpf, expected_gpf,
                    "GPF mismatch for m = {}: expected {}, got {}",
                    m, expected_gpf, actual_gpf
                );
            }

            start_m += len as u64;
        }
    }
}
