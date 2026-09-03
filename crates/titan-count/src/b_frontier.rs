//! Phase 6.13: In-Flight Frontier B(x, y) Streaming Evaluator (b_frontier.rs).
//!
//! Features Fast-Forward Segment Skipping: when quotient v = floor(x/p) >= high,
//! the segment contains ZERO prime quotients for B. Bypasses byte inspections
//! and NEON popcounts, advancing running_pi += popcount in O(1).

use std::sync::Arc;
use titan_sieve::frontier_ring::FrontierRingBuffer;
use titan_sieve::wheel30::{RESIDUE_TO_BIT, WHEEL_RESIDUES};
use crate::delta_prime_stream::DeltaPrimeStream;

pub fn compute_b_frontier_stream_fast(
    x: u64,
    y: u64,
    pi_z: u64,
    stream: &DeltaPrimeStream,
    ring: Arc<FrontierRingBuffer>,
) -> i64 {
    let sqrt_x = (x as f64).sqrt() as u64;
    if y >= sqrt_x {
        return 0;
    }

    let p_start_idx = stream.binary_search(y);
    let p_end_idx = stream.binary_search(sqrt_x);
    if p_start_idx >= p_end_idx {
        return 0;
    }

    // Gauss closed-form arithmetic progression for sum_{k=a}^b k
    let a = (p_start_idx + 1) as i64;
    let b = p_end_idx as i64;
    let n = b - a + 1;
    let sum_pi_p = (a + b) * n / 2;

    let mut sum_pi_quotients: i64 = 0;
    let mut running_pi = pi_z;

    // Start p at sqrt(x) and step backwards to y
    let mut curr_p_idx = p_end_idx;
    let mut curr_p = stream.get(curr_p_idx);

    while curr_p_idx > p_start_idx {
        let v = x / (curr_p as u64);

        if let Some((_seg_idx, low, high, popcount, buf_ptr, slot_idx)) = ring.try_acquire_committed() {
            if v >= high {
                // FAST-FORWARD: Segment contains 0 quotients for B!
                // O(1) advance of running prefix prime count
                running_pi += popcount;
                ring.release_committed(slot_idx);
                continue;
            }

            if v >= low {
                // Segment contains active quotients; drain all primes in [low, high)
                while curr_p_idx > p_start_idx {
                    let p = curr_p as u64;
                    let quot = x / p;
                    if quot >= high {
                        break;
                    }

                    let target_byte = ((quot - low) / 30) as usize;
                    let target_rem = ((quot - low) % 30) as usize;

                    let mut local_cnt = 0u64;

                    // 16-byte aligned vector popcounts
                    let full_16 = target_byte & !15;

                    #[cfg(target_arch = "aarch64")]
                    {
                        use core::arch::aarch64::*;
                        for i in (0..full_16).step_by(16) {
                            unsafe {
                                let q = vld1q_u8(buf_ptr.add(i));
                                local_cnt += vaddlvq_u16(
                                    vpaddlq_u8(vcntq_u8(q))
                                ) as u64;
                            }
                        }
                    }

                    #[cfg(not(target_arch = "aarch64"))]
                    {
                        for i in 0..full_16 {
                            unsafe { local_cnt += (*buf_ptr.add(i)).count_ones() as u64; }
                        }
                    }

                    for i in full_16..target_byte {
                        unsafe { local_cnt += (*buf_ptr.add(i)).count_ones() as u64; }
                    }

                    let last_byte = unsafe { *buf_ptr.add(target_byte) };
                    let bit_limit = RESIDUE_TO_BIT[target_rem];
                    let mask = if bit_limit == 0xFF {
                        let mut m = 0u8;
                        for (idx, &res) in WHEEL_RESIDUES.iter().enumerate() {
                            if (res as usize) <= target_rem { m |= 1 << idx; }
                        }
                        m
                    } else {
                        (1u8 << (bit_limit + 1)).wrapping_sub(1)
                    };
                    local_cnt += (last_byte & mask).count_ones() as u64;

                    sum_pi_quotients += (running_pi + local_cnt) as i64;

                    curr_p_idx -= 1;
                    curr_p = stream.get(curr_p_idx);
                }

                running_pi += popcount;
                ring.release_committed(slot_idx);
            }
        } else {
            std::hint::spin_loop();
        }
    }

    sum_pi_quotients - sum_pi_p + n
}

#[inline(always)]
pub fn compute_b_frontier_stream(
    x: u64,
    y: u64,
    pi_z: u64,
    stream: &DeltaPrimeStream,
    ring: Arc<FrontierRingBuffer>,
) -> i64 {
    compute_b_frontier_stream_fast(x, y, pi_z, stream, ring)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_b_frontier_empty() {
        let stream = DeltaPrimeStream::encode_from_slice(&[2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31]);
        let ring = FrontierRingBuffer::new(100, 491520, 1);
        let res = compute_b_frontier_stream_fast(100, 10, 4, &stream, ring);
        assert_eq!(res, 0); // y >= sqrt(x)
    }
}
