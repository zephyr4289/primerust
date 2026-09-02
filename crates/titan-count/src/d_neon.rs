//! Phase 31: D-Term NEON Acceleration — Vectorized μ=0 Kill Switch & Leaf Filter.
//!
//! Exploits the 32-bit compact Ftd entry layout: [lpf_idx: 14 | sign: 1 | nz: 1 | mpf: 16]
//! where bit 15 (NZB = 1 << 15) is the square factor kill switch (nz=1 => mu(n)=0).
//! Uses NEON 128-bit SIMD to process 4 entries per cycle with zero branch penalties.

#[cfg(target_arch = "aarch64")]
use core::arch::aarch64::*;

use crate::factortable::{Ftd, NZB, SGN};

/// Represents an active (non-zero mu) leaf candidate for the D-term walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DLeaf {
    pub d: u32,
    pub mu: i8,    // +1 or -1
    pub mpf: u32,  // largest prime factor
}

/// NEON-vectorized counter for alive (squarefree) entries in a slice of Ftd table.
///
/// Returns the number of indices where mu(n) != 0.
#[inline(always)]
pub fn count_alive_entries(ft: &Ftd, start: usize, len: usize) -> usize {
    #[cfg(target_arch = "aarch64")]
    unsafe {
        count_alive_entries_neon(&ft.e, start, len)
    }
    #[cfg(not(target_arch = "aarch64"))]
    {
        count_alive_entries_scalar(&ft.e, start, len)
    }
}

#[cfg(target_arch = "aarch64")]
#[inline(always)]
unsafe fn count_alive_entries_neon(e: &[u32], start: usize, len: usize) -> usize {
    let mut alive = 0usize;
    let mut i = start;
    let end = start + len;
    let nzb_vec = vdupq_n_u32(NZB);

    // Unroll 4x4 = 16 elements per iteration
    while i + 16 <= end {
        let v0 = vld1q_u32(e.as_ptr().add(i));
        let v1 = vld1q_u32(e.as_ptr().add(i + 4));
        let v2 = vld1q_u32(e.as_ptr().add(i + 8));
        let v3 = vld1q_u32(e.as_ptr().add(i + 12));

        let nz0 = vandq_u32(v0, nzb_vec);
        let nz1 = vandq_u32(v1, nzb_vec);
        let nz2 = vandq_u32(v2, nzb_vec);
        let nz3 = vandq_u32(v3, nzb_vec);

        // vceqzq returns 0xFFFF_FFFF if nz == 0 (alive), else 0
        let eq0 = vceqzq_u32(nz0);
        let eq1 = vceqzq_u32(nz1);
        let eq2 = vceqzq_u32(nz2);
        let eq3 = vceqzq_u32(nz3);

        // Subtraction from 0: -(-1) = 1 in 2's complement
        let sum01 = vaddq_u32(eq0, eq1);
        let sum23 = vaddq_u32(eq2, eq3);
        let total_vec = vaddq_u32(sum01, sum23);

        // Each true element in eq contributes 0xFFFF_FFFF = -1 as signed or 2^32 - 1 as unsigned.
        // Bit-negating or negating: -vaddvq_s32
        let chunk_alive = (-vaddvq_s32(vreinterpretq_s32_u32(total_vec))) as usize;
        alive += chunk_alive;
        i += 16;
    }

    // Scalar tail
    while i < end {
        if e[i] & NZB == 0 {
            alive += 1;
        }
        i += 1;
    }

    alive
}

#[allow(dead_code)]
#[inline(always)]
fn count_alive_entries_scalar(e: &[u32], start: usize, len: usize) -> usize {
    let mut alive = 0usize;
    let end = start + len;
    for i in start..end {
        if e[i] & NZB == 0 {
            alive += 1;
        }
    }
    alive
}

/// Extracts active D-leaves in batch into `out` buffer using SIMD mask filtering.
///
/// Returns number of leaves populated into `out`.
pub fn collect_d_leaves(ft: &Ftd, start: usize, len: usize, out: &mut Vec<DLeaf>) -> usize {
    let initial_len = out.len();
    let end = start + len;
    let e = &ft.e;

    // Pre-reserve capacity based on expected squarefree density 6/pi^2 ~ 60.8%
    out.reserve((len * 62) / 100);

    let mut i = start;

    #[cfg(target_arch = "aarch64")]
    unsafe {
        let nzb_vec = vdupq_n_u32(NZB);
        while i + 4 <= end {
            let v = vld1q_u32(e.as_ptr().add(i));
            let nz = vandq_u32(v, nzb_vec);
            let eq = vceqzq_u32(nz); // 0xFFFF_FFFF if alive, 0 if dead

            // Quick check: if all 4 are dead (0), skip the entire chunk
            if vaddvq_u32(eq) == 0 {
                i += 4;
                continue;
            }

            // Extract alive elements
            for lane in 0..4 {
                let entry = *e.as_ptr().add(i + lane);
                if entry & NZB == 0 {
                    let d = (i + lane) as u32;
                    let mu = if entry & SGN != 0 { -1 } else { 1 };
                    let mpf = entry >> 16;
                    out.push(DLeaf { d, mu, mpf });
                }
            }
            i += 4;
        }
    }

    while i < end {
        let entry = e[i];
        if entry & NZB == 0 {
            let d = i as u32;
            let mu = if entry & SGN != 0 { -1 } else { 1 };
            let mpf = entry >> 16;
            out.push(DLeaf { d, mu, mpf });
        }
        i += 1;
    }

    out.len() - initial_len
}

#[cfg(test)]
mod tests {
    use super::*;
    use titan_sieve::base::generate_base_primes;

    #[test]
    fn test_d_neon_count_and_collect_against_scalar() {
        let z = 50_000u64;
        let base_primes = generate_base_primes(titan_core::roots::isqrt(z) + 10);
        let primes_u32: Vec<u32> = base_primes.iter().map(|&p| p as u32).collect();
        let ft = Ftd::build(z, &primes_u32);

        let start = 2usize;
        let len = (z - 1) as usize;

        // Scalar baseline
        let scalar_alive = count_alive_entries_scalar(&ft.e, start, len);
        let neon_alive = count_alive_entries(&ft, start, len);

        assert_eq!(neon_alive, scalar_alive, "NEON alive count must match scalar");

        let mut leaves = Vec::new();
        let collected = collect_d_leaves(&ft, start, len, &mut leaves);
        assert_eq!(collected, scalar_alive, "Collected leaves count must match alive count");

        // Verify each leaf
        for leaf in &leaves {
            let n = leaf.d as u64;
            assert_ne!(ft.mu(n), 0, "Collected leaf {} must not have mu=0", n);
            assert_eq!(leaf.mu as i32, ft.mu(n), "mu mismatch at {}", n);
            assert_eq!(leaf.mpf, ft.mpf(n), "mpf mismatch at {}", n);
        }
    }
}
