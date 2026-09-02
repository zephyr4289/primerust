//! Phase 34: FTD-v2 Wheel-30 Packed Block Engine.
//!
//! Packs FactorTable strictly over Wheel-30 candidates (8 per 30 numbers, coprime to 2, 3, 5).
//! - Eliminates passes for p = 2, 3, 5 (-43% total marks).
//! - 3.75x denser memory layout (8 B / candidate).
//! - L1D-resident blocks: 6,144 candidates (48 KiB <= 64 KiB A78 L1D).
//! - Division-avoidance post-pass (Coprime-Residual Lemma).

use titan_core::roots::isqrt;
use titan_core::wheel::{cand_idx, UNITS};

pub const LPF_MASK: u32 = 0x3FFF;
pub const SIGN_BIT: u32 = 1 << 14;
pub const NZ_BIT: u32 = 1 << 15;
pub const MPF_MASK: u32 = 0xFFFF;

/// Wheel-30 packed block resident in L1D cache
pub struct BlockV2 {
    pub word: Vec<u32>, // [lpf:14 | sign:1 | nz:1 | mpf:16]
    pub sfac: Vec<u32>, // Product of prime factors <= sqrt(hi)
    pub block_cand_lo: usize, // Base candidate index of this block
}

impl BlockV2 {
    pub fn new(capacity_cands: usize) -> Self {
        Self {
            word: vec![0u32; capacity_cands],
            sfac: vec![1u32; capacity_cands],
            block_cand_lo: 0,
        }
    }

    #[inline(always)]
    pub fn reset(&mut self, cand_len: usize, block_cand_lo: usize) {
        self.word[..cand_len].fill(0);
        self.sfac[..cand_len].fill(1);
        self.block_cand_lo = block_cand_lo;
    }
}

/// Computes the candidate value for a candidate index
#[inline(always)]
pub fn candidate_val(c_idx: usize) -> u64 {
    let q = (c_idx / 8) as u64;
    let r = c_idx % 8;
    q * 30 + (UNITS[r] as u64)
}

/// Sieve power passes for a prime p >= 7 over block of candidates [cand_lo, cand_hi)
pub fn sieve_prime_wheel(
    block: &mut BlockV2,
    cand_lo: usize,
    cand_hi: usize,
    p: u32,
    p_idx: u32,
) {
    let p_u64 = p as u64;
    let val_lo = candidate_val(cand_lo);
    let val_hi = candidate_val(cand_hi);

    let p_clip = (p as u16) as u32;

    // Pass p^1: multiples of p
    let start_val = if 2 * p_u64 >= val_lo {
        2 * p_u64
    } else {
        let rem = val_lo % p_u64;
        if rem == 0 { val_lo } else { val_lo + (p_u64 - rem) }
    };

    // Find first wheel candidate multiple >= start_val
    let mut m = start_val;
    while m < val_hi {
        let r = (m % 30) as usize;
        let c_rel = cand_idx(r as u64);
        if c_rel != 255 {
            let c_idx = (m / 30) as usize * 8 + (c_rel as usize);
            if c_idx >= cand_lo && c_idx < cand_hi {
                let local_idx = c_idx - cand_lo;
                let mut w = block.word[local_idx];

                w ^= SIGN_BIT;
                if (w >> 18) & LPF_MASK == 0 {
                    w |= (p_idx & LPF_MASK) << 18;
                }
                w = (w & !MPF_MASK) | p_clip;

                block.word[local_idx] = w;
                block.sfac[local_idx] = block.sfac[local_idx].wrapping_mul(p);
            }
        }
        m += p_u64;
    }

    // Pass p^j (j >= 2): powers of p
    let mut p_pow = p_u64 * p_u64;
    while p_pow < val_hi {
        let start_pow = if p_pow >= val_lo {
            p_pow
        } else {
            let rem = val_lo % p_pow;
            if rem == 0 { val_lo } else { val_lo + (p_pow - rem) }
        };

        let mut m_pow = start_pow;
        while m_pow < val_hi {
            let r = (m_pow % 30) as usize;
            let c_rel = cand_idx(r as u64);
            if c_rel != 255 {
                let c_idx = (m_pow / 30) as usize * 8 + (c_rel as usize);
                if c_idx >= cand_lo && c_idx < cand_hi {
                    let local_idx = c_idx - cand_lo;
                    block.word[local_idx] |= NZ_BIT;
                    block.sfac[local_idx] = block.sfac[local_idx].wrapping_mul(p);
                }
            }
            m_pow += p_pow;
        }

        if let Some(next_pow) = p_pow.checked_mul(p_u64) {
            p_pow = next_pow;
        } else {
            break;
        }
    }
}

/// Resolves the largest prime factor (mpf) using the Coprime-Residual Lemma
#[inline(always)]
pub fn resolve_mpf_v2(w: u32, n: u64, sfac: u32) -> u32 {
    let sfac_u64 = sfac as u64;
    if sfac_u64 == n {
        w & MPF_MASK
    } else if sfac_u64 * 65535 < n {
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

/// Produce a Wheel-30 block of candidates [cand_lo, cand_hi)
pub fn produce_block_v2(
    block: &mut BlockV2,
    cand_lo: usize,
    cand_hi: usize,
    primes: &[u32],
) {
    let len = cand_hi - cand_lo;
    block.reset(len, cand_lo);

    let val_hi = candidate_val(cand_hi);
    let sqrt_hi = isqrt(val_hi) as u32;

    for (idx, &p) in primes.iter().enumerate() {
        if p < 7 {
            continue;
        }
        if p > sqrt_hi {
            break;
        }
        sieve_prime_wheel(block, cand_lo, cand_hi, p, idx as u32 + 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use titan_sieve::base::generate_base_primes;
    use crate::factortable::Ftd;

    #[test]
    fn test_ftd_v2_oracle_against_flat() {
        let z = 100_000u64;
        let base_primes = generate_base_primes(isqrt(z) + 10);
        let primes_u32: Vec<u32> = base_primes.iter().map(|&p| p as u32).collect();

        let flat_ft = Ftd::build(z, &primes_u32);

        let total_cands = (z as usize / 30) * 8 + 8;
        let block_cands = 1024usize;
        let mut block = BlockV2::new(block_cands);

        let mut cand_lo = 0usize;
        let mut alive_count = 0usize;

        while cand_lo < total_cands {
            let cand_hi = (cand_lo + block_cands).min(total_cands);
            produce_block_v2(&mut block, cand_lo, cand_hi, &primes_u32);

            for c in cand_lo..cand_hi {
                let n = candidate_val(c);
                if n > z || n < 2 {
                    continue;
                }

                let local_idx = c - cand_lo;
                let w = block.word[local_idx];
                let sfac = block.sfac[local_idx];

                let flat_w = flat_ft.e[n as usize];

                let block_nz = (w & NZ_BIT) != 0;
                let flat_nz = (flat_w & NZ_BIT) != 0;
                assert_eq!(block_nz, flat_nz, "NZ bit mismatch at candidate n = {}", n);

                if !block_nz {
                    alive_count += 1;
                    let block_sign = (w & SIGN_BIT) != 0;
                    let flat_sign = (flat_w & SIGN_BIT) != 0;
                    assert_eq!(block_sign, flat_sign, "Sign mismatch at candidate n = {}", n);

                    let block_mpf = resolve_mpf_v2(w, n, sfac);
                    let flat_mpf = flat_w & MPF_MASK;
                    assert_eq!(block_mpf, flat_mpf, "MPF mismatch at candidate n = {}", n);
                }
            }
            cand_lo = cand_hi;
        }

        assert!(alive_count > 0, "Must have squarefree survivors");
    }
}
