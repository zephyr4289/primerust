//! Phase 39: Streaming Möbius Generator (MobiusStream).
//!
//! Generates mu(d) on-the-fly in 32 KiB L1D-resident blocks up to sqrt(x),
//! using only O(x^(1/4)) base primes and zero heap allocations.
//!
//! Eliminates the 40 MB FactorTableD entirely.

use titan_core::roots::isqrt;
use titan_sieve::base::generate_base_primes;

pub const MOBIUS_BLOCK_SIZE: usize = 32768; // 32 KiB L1D cache block

pub struct MobiusBlock {
    pub mu: Vec<i8>,
    pub sfac: Vec<u32>,
    pub base_primes: Vec<u64>,
}

impl MobiusBlock {
    pub fn new(max_limit: u64) -> Self {
        let sqrt_limit = isqrt(max_limit);
        let base_primes = generate_base_primes(sqrt_limit + 100);
        Self {
            mu: vec![1i8; MOBIUS_BLOCK_SIZE],
            sfac: vec![1u32; MOBIUS_BLOCK_SIZE],
            base_primes,
        }
    }

    /// Sieves mu(d) for d in [lo, hi) in L1D cache
    pub fn sieve_block(&mut self, lo: u64, hi: u64) {
        let len = (hi - lo) as usize;
        self.mu[..len].fill(1);
        self.sfac[..len].fill(1);

        if lo == 0 && len > 0 {
            self.mu[0] = 0; // mu(0) = 0
        }

        let sqrt_hi = isqrt(hi);

        for &p in &self.base_primes {
            if p > sqrt_hi {
                break;
            }
            let p_u32 = p as u32;

            // Pass p^1: flip sign
            let start = if p >= lo {
                p
            } else {
                let rem = lo % p;
                if rem == 0 { lo } else { lo + (p - rem) }
            };

            let mut m = start;
            while m < hi {
                let idx = (m - lo) as usize;
                self.mu[idx] = -self.mu[idx];
                self.sfac[idx] = self.sfac[idx].wrapping_mul(p_u32);
                m += p;
            }

            // Pass p^2: set mu = 0 (square kill)
            let p2 = p * p;
            let start2 = if p2 >= lo {
                p2
            } else {
                let rem = lo % p2;
                if rem == 0 { lo } else { lo + (p2 - rem) }
            };

            let mut m2 = start2;
            while m2 < hi {
                let idx = (m2 - lo) as usize;
                self.mu[idx] = 0;
                m2 += p2;
            }
        }

        // Finalize residual prime factors
        for i in 0..len {
            let n = lo + i as u64;
            if n <= 1 {
                continue;
            }
            if self.mu[i] != 0 {
                let s = self.sfac[i] as u64;
                if s < n {
                    let r = n / s;
                    if r > 1 {
                        self.mu[i] = -self.mu[i];
                    }
                }
            }
        }
    }
}

pub struct MobiusStream {
    block: MobiusBlock,
    limit: u64,
    cur_d: u64,
    block_lo: u64,
    block_hi: u64,
}

impl MobiusStream {
    pub fn new(limit: u64) -> Self {
        let mut stream = Self {
            block: MobiusBlock::new(limit),
            limit,
            cur_d: 1,
            block_lo: 1,
            block_hi: 1,
        };
        stream.load_next_block();
        stream
    }

    fn load_next_block(&mut self) {
        if self.block_lo > self.limit {
            return;
        }
        self.block_hi = (self.block_lo + MOBIUS_BLOCK_SIZE as u64).min(self.limit + 1);
        self.block.sieve_block(self.block_lo, self.block_hi);
    }
}

impl Iterator for MobiusStream {
    type Item = (u64, i8);

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.cur_d > self.limit {
            return None;
        }

        if self.cur_d >= self.block_hi {
            self.block_lo = self.block_hi;
            self.load_next_block();
        }

        let d = self.cur_d;
        let idx = (d - self.block_lo) as usize;
        let mu = self.block.mu[idx];
        self.cur_d += 1;

        Some((d, mu))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mobius_stream_exactness() {
        let stream = MobiusStream::new(30);
        let results: Vec<(u64, i8)> = stream.collect();

        // Check first 10 values of mu(n)
        assert_eq!(results[0], (1, 1));
        assert_eq!(results[1], (2, -1));
        assert_eq!(results[2], (3, -1));
        assert_eq!(results[3], (4, 0)); // 4 = 2^2
        assert_eq!(results[4], (5, -1));
        assert_eq!(results[5], (6, 1));  // 6 = 2 * 3
        assert_eq!(results[6], (7, -1));
        assert_eq!(results[7], (8, 0));  // 8 = 2^3
        assert_eq!(results[8], (9, 0));  // 9 = 3^2
        assert_eq!(results[9], (10, 1)); // 10 = 2 * 5
        assert_eq!(results[29], (30, -1)); // 30 = 2 * 3 * 5
    }
}
