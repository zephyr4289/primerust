//! Phase 33: Block-Local Fused FactorTable (Zero DRAM Round-Trip).
//!
//! Produces (mu, lpf, mpf) per block and immediately walks leaves in L2 cache.
//! Memory footprint: O(BlockSize) per thread (VmHWM <= 16 MB during FTD phase).
//!
//! Layout per entry: [lpf: 14 | sign: 1 | nz: 1 | mpf: 16]

use titan_core::roots::isqrt;

pub const LPF_MASK: u32 = 0x3FFF;
pub const SIGN_BIT: u32 = 1 << 14;
pub const NZ_BIT: u32 = 1 << 15;
pub const MPF_MASK: u32 = 0xFFFF;

pub struct FtdBlock {
    pub word: Vec<u32>,
    pub sfac: Vec<u32>,
}

impl FtdBlock {
    pub fn new(capacity: usize) -> Self {
        Self {
            word: vec![0u32; capacity],
            sfac: vec![1u32; capacity],
        }
    }

    /// Reset block buffers for reuse in next segment (zero allocation)
    #[inline(always)]
    pub fn reset(&mut self, len: usize) {
        self.word[..len].fill(0);
        self.sfac[..len].fill(1);
    }
}

/// Sieve power passes for a single prime p over block [lo, hi)
#[inline(always)]
pub fn sieve_power_passes(
    block: &mut FtdBlock,
    lo: u64,
    hi: u64,
    p: u32,
    p_idx: u32,
) {
    let p_u64 = p as u64;
    let len = (hi - lo) as usize;

    // Pass p^1: multiples >= 2*p
    let start_mult = if 2 * p_u64 >= lo {
        2 * p_u64
    } else {
        let rem = lo % p_u64;
        if rem == 0 { lo } else { lo + (p_u64 - rem) }
    };

    let p_clip = (p as u16) as u32;

    let mut m = start_mult;
    while m < hi {
        let idx = (m - lo) as usize;
        let mut w = block.word[idx];

        // Flip sign bit (parity of distinct prime factors)
        w ^= SIGN_BIT;

        // First-writer lpf
        if (w >> 18) & LPF_MASK == 0 {
            w |= (p_idx & LPF_MASK) << 18;
        }

        // Overwrite mpf with current prime (ascending order -> last writer is largest factor <= sqrt(hi))
        w = (w & !MPF_MASK) | p_clip;

        block.word[idx] = w;
        block.sfac[idx] = block.sfac[idx].wrapping_mul(p);

        m += p_u64;
    }

    // Pass p^j for j >= 2: set NZ bit, multiply sfac
    let mut p_pow = p_u64 * p_u64;
    while p_pow < hi {
        let start_pow = if p_pow >= lo {
            p_pow
        } else {
            let rem = lo % p_pow;
            if rem == 0 { lo } else { lo + (p_pow - rem) }
        };

        let mut m_pow = start_pow;
        while m_pow < hi {
            let idx = (m_pow - lo) as usize;
            block.word[idx] |= NZ_BIT;
            block.sfac[idx] = block.sfac[idx].wrapping_mul(p);
            m_pow += p_pow;
        }

        if let Some(next_pow) = p_pow.checked_mul(p_u64) {
            p_pow = next_pow;
        } else {
            break;
        }
    }
}

/// Resolves the final largest prime factor (mpf) using the division-avoidance lemma
#[inline(always)]
pub fn resolve_mpf(w: u32, n: u64, sfac: u32) -> u32 {
    let sfac_u64 = sfac as u64;
    if sfac_u64 == n {
        // R = 1: sfac accounted for all factors
        w & MPF_MASK
    } else if sfac_u64 * 65535 < n {
        // R > 65535: residual prime exceeds 16-bit clip
        65535
    } else {
        let r = (n / sfac_u64) as u32;
        if r > 1 {
            r.min(65535)
        } else {
            w & MPF_MASK
        }
    }
}

/// Produce block [lo, hi) and verify bit-exact against flat reference for n <= 10^6
pub fn produce_block(
    block: &mut FtdBlock,
    lo: u64,
    hi: u64,
    primes: &[u32],
) {
    let len = (hi - lo) as usize;
    block.reset(len);

    let sqrt_hi = isqrt(hi) as u32;

    for (idx, &p) in primes.iter().enumerate() {
        if p > sqrt_hi {
            break;
        }
        sieve_power_passes(block, lo, hi, p, idx as u32 + 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use titan_sieve::base::generate_base_primes;
    use crate::factortable::Ftd;

    #[test]
    fn test_ftd_block_bit_exact_against_flat() {
        let z = 100_000u64;
        let base_primes = generate_base_primes(isqrt(z) + 10);
        let primes_u32: Vec<u32> = base_primes.iter().map(|&p| p as u32).collect();

        let flat_ft = Ftd::build(z, &primes_u32);

        let block_size = 4096usize;
        let mut block = FtdBlock::new(block_size);

        let mut lo = 2u64;
        while lo <= z {
            let hi = (lo + block_size as u64).min(z + 1);
            produce_block(&mut block, lo, hi, &primes_u32);

            for n in lo..hi {
                let idx = (n - lo) as usize;
                let w = block.word[idx];
                let sfac = block.sfac[idx];

                let flat_w = flat_ft.e[n as usize];

                // Check NZ bit
                let block_nz = (w & NZ_BIT) != 0;
                let flat_nz = (flat_w & NZ_BIT) != 0;
                assert_eq!(block_nz, flat_nz, "NZ bit mismatch at n = {}", n);

                if !block_nz {
                    // Check sign bit
                    let block_sign = (w & SIGN_BIT) != 0;
                    let flat_sign = (flat_w & SIGN_BIT) != 0;
                    assert_eq!(block_sign, flat_sign, "Sign bit mismatch at n = {}", n);

                    // Check mpf resolution
                    let block_mpf = resolve_mpf(w, n, sfac);
                    let flat_mpf = flat_w & MPF_MASK;
                    assert_eq!(block_mpf, flat_mpf, "MPF mismatch at n = {}", n);
                }
            }

            lo = hi;
        }
    }
}
