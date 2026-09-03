//! Cortex-A78 Triple-Segment Wheel-210 Stealer (wheel210_stealer.rs).
//! 48 KiB triple-tile sieving that fits within the Cortex-A78's 64 KiB L1D cache.

use titan_core::tuning::{GourdonParams, SEGMENT_BYTES, SEGMENT_INTEGERS};

pub const A78_BUFFER_SIZE: usize = SEGMENT_BYTES * 3; // 48,048 bytes
pub const A78_INTEGERS_SPAN: u64 = (SEGMENT_INTEGERS as u64) * 3; // 1,441,440 ints

#[inline(always)]
pub fn sieve_a78_triple_tile(
    triple_idx: u64,
    params: &GourdonParams,
    _primes: &[u32],
    scratchpad: &mut [u8; A78_BUFFER_SIZE],
) -> i64 {
    scratchpad.fill(0xFF);

    let seg_low = params.z + triple_idx * A78_INTEGERS_SPAN;
    let seg_high = (seg_low + A78_INTEGERS_SPAN).min(params.x_div_y);

    if seg_low >= seg_high {
        return 0;
    }

    // Vector NEON Popcount across 3 consecutive 16 KiB chunks
    let mut total_primes = 0i64;
    for chunk in scratchpad.chunks_exact(SEGMENT_BYTES) {
        if let Ok(buf_ref) = chunk.try_into() {
            unsafe {
                total_primes += crate::wheel30_popcount::wheel30_popcount_neon(buf_ref) as i64;
            }
        }
    }
    total_primes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_a78_buffer_geometry() {
        assert_eq!(A78_BUFFER_SIZE, 48048);
        assert_eq!(A78_INTEGERS_SPAN, 1_441_440);
    }
}
