//! Phase 6.2 Step 2: Dense Rotating Sieve Kernel for Tier 1 Primes (wheel30_dense.rs).
//!
//! Inner marking loop for primes 37 <= p <= 1,200 using dual-register rotation.
//! Holds all 8 byte advances in a 64-bit register (`adv_strip`) and all 8 bitmasks
//! in another 64-bit register (`mask_strip`), eliminating memory loads in the inner loop.

use crate::wheel30::{SEGMENT_BYTES, Wheel30PrimeState};

/// Inner marking loop for primes <= 1,200 with dynamic per-prime safe limit.
/// Allows 99%+ of the segment buffer to run under 4-way ILP unrolling for small primes.
#[inline(always)]
pub unsafe fn sieve_tier1_prime_dynamic(
    state: &mut Wheel30PrimeState,
    p: u32,
    sieve_buf: &mut [u8; SEGMENT_BYTES],
) {
    let mut byte_idx = state.next_byte as usize;
    if byte_idx >= SEGMENT_BYTES {
        state.next_byte -= SEGMENT_BYTES as u32;
        return;
    }

    let buf_ptr = sieve_buf.as_mut_ptr();
    let mut mask_strip = state.mask_strip.rotate_right((state.phase as u32) * 8);
    let mut adv_strip = state.adv_strip.rotate_right((state.phase as u32) * 8);
    let mut phase = state.phase;

    // Dynamic per-prime safe limit: allows 99%+ of buffer to run 4-way unrolled
    let max_4step = ((p as usize * 16) / 30) + 8;
    let safe_limit = if SEGMENT_BYTES > max_4step {
        SEGMENT_BYTES - max_4step
    } else {
        0
    };

    // 4-Way Pipelined ILP Unrolling
    while byte_idx < safe_limit {
        // Prefetch next cacheline ahead to keep store buffer flowing
        #[cfg(target_arch = "aarch64")]
        std::arch::asm!("prfm pldl1keep, [{}]", in(reg) buf_ptr.add(byte_idx + 64), options(nostack, preserves_flags));

        let m0 = mask_strip as u8;
        let a0 = adv_strip as u8;
        *buf_ptr.add(byte_idx) &= !m0;
        byte_idx += a0 as usize;

        let m1 = (mask_strip >> 8) as u8;
        let a1 = (adv_strip >> 8) as u8;
        *buf_ptr.add(byte_idx) &= !m1;
        byte_idx += a1 as usize;

        let m2 = (mask_strip >> 16) as u8;
        let a2 = (adv_strip >> 16) as u8;
        *buf_ptr.add(byte_idx) &= !m2;
        byte_idx += a2 as usize;

        let m3 = (mask_strip >> 24) as u8;
        let a3 = (adv_strip >> 24) as u8;
        *buf_ptr.add(byte_idx) &= !m3;
        byte_idx += a3 as usize;

        mask_strip = mask_strip.rotate_right(32);
        adv_strip = adv_strip.rotate_right(32);
        phase = (phase + 4) & 7;
    }

    // Residual tail handling up to the exact segment end
    while byte_idx < SEGMENT_BYTES {
        let m = mask_strip as u8;
        let a = adv_strip as u8;
        *buf_ptr.add(byte_idx) &= !m;
        byte_idx += a as usize;

        mask_strip = mask_strip.rotate_right(8);
        adv_strip = adv_strip.rotate_right(8);
        phase = (phase + 1) & 7;
    }

    state.next_byte = (byte_idx - SEGMENT_BYTES) as u32;
    state.phase = phase;
}

/// Backward-compatible wrapper using default conservative bound
#[inline(always)]
pub unsafe fn sieve_tier1_prime(
    state: &mut Wheel30PrimeState,
    sieve_buf: &mut [u8; SEGMENT_BYTES],
) {
    sieve_tier1_prime_dynamic(state, 1200, sieve_buf);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wheel30::BIT_TO_RESIDUE;

    #[test]
    fn test_tier1_prime_marking_exactness() {
        let mut buf = [0xFFu8; SEGMENT_BYTES];
        let p = 37u32;
        let mut state = Wheel30PrimeState::compile(p, 0);

        unsafe {
            sieve_tier1_prime(&mut state, &mut buf);
        }

        // Verify that for all multiples of 37 in [0, WHEEL_SPAN) coprime to 30,
        // their bit in buf is 0 (cleared), and for all other coprime integers, bit is 1.
        for byte in 0..SEGMENT_BYTES {
            for bit in 0..8 {
                let res = BIT_TO_RESIDUE[bit] as u64;
                let n = (byte as u64) * 30 + res;
                let is_marked = (buf[byte] & (1 << bit)) == 0;

                let should_be_marked = n >= (p as u64) * (p as u64) && n % (p as u64) == 0;
                assert_eq!(
                    is_marked,
                    should_be_marked,
                    "Mismatch at integer n = {} (byte {}, bit {}) for prime {}",
                    n, byte, bit, p
                );
            }
        }
    }
}
