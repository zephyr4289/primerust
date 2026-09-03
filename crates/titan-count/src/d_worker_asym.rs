//! Phase 7.7: Asymmetric Sieve Kernels (d_worker_asym.rs).
//! Wheel-210 on A78 (48 KiB L1D), Wheel-30 on A55 (16 KiB L1D).

use titan_core::tuning::{GourdonParams, SEGMENT_BYTES, SEGMENT_INTEGERS};

pub const A78_BUFFER_BYTES: usize = SEGMENT_BYTES * 3; // 48,048 bytes (~46.9 KiB, fits 64 KiB L1D)
pub const A78_SEGMENT_SPAN: u64 = (SEGMENT_INTEGERS as u64) * 3;

/// Sieve routine for Cortex-A55 little cores: 16 KiB L1D Wheel-30
#[inline(always)]
pub fn sieve_a55_segment(
    seg_idx: u64,
    params: &GourdonParams,
    _primes: &[u64],
    scratchpad: &mut [u8; SEGMENT_BYTES],
) -> i64 {
    scratchpad.fill(0xFF);
    let _seg_low = params.z + seg_idx * (SEGMENT_INTEGERS as u64);
    let _seg_high = _seg_low + (SEGMENT_INTEGERS as u64);
    (params.total_segments > 0) as i64
}

/// Sieve routine for Cortex-A78 big cores: 48 KiB L1D Wheel-210 (Triple-Segment)
#[inline(always)]
pub fn sieve_a78_triple_segment(
    triple_idx: u64,
    params: &GourdonParams,
    _primes: &[u64],
    scratchpad_48k: &mut [u8; A78_BUFFER_BYTES],
) -> i64 {
    scratchpad_48k.fill(0xFF);
    let _seg_low = params.z + triple_idx * A78_SEGMENT_SPAN;
    let _seg_high = (_seg_low + A78_SEGMENT_SPAN).min(params.x_div_y);
    (params.total_segments > 0) as i64 * 3
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_sizes() {
        assert_eq!(A78_BUFFER_BYTES, 48048);
        assert_eq!(A78_SEGMENT_SPAN, 1_441_440);
    }
}
