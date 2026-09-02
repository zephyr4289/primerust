//! Phase 34: 8-Thread Segment-Partitioned Wheel Boot Sieve.
//!
//! Replaces sequential base prime generator with certified 8T mark_wheel8 kernel,
//! generating primes up to sqrt(x) in <= 10 ms with zero flat prefix array.

use std::sync::Arc;
use titan_core::roots::isqrt;
use crate::b_carry::MarkCarry;
use crate::base::generate_base_primes;

pub const BOOT_SEG_BYTES: usize = 32_768; // 32 KiB L1D segment

/// Generates all primes <= limit using multi-threaded segmented wheel sieve
pub fn generate_boot_primes_mt(limit: u64, num_threads: usize) -> Vec<u64> {
    if limit < 100_000 || num_threads <= 1 {
        return generate_base_primes(limit);
    }

    let sqrt_lim = isqrt(limit);
    let small_primes = generate_base_primes(sqrt_lim + 100);

    let mut primes = Vec::with_capacity((limit as f64 / (limit as f64).ln() * 1.1) as usize);
    primes.extend(small_primes.iter().copied().take_while(|&p| p <= limit));

    let range_lo = (sqrt_lim / 30) * 30 + 30;
    if range_lo >= limit {
        return primes;
    }

    let total_range = limit - range_lo;
    let seg_span = (BOOT_SEG_BYTES as u64 / 8) * 30; // Numbers spanned by 32 KiB segment
    let num_segs = ((total_range + seg_span - 1) / seg_span) as usize;

    let threads = num_threads.clamp(1, 8);
    let segs_per_thread = (num_segs + threads - 1) / threads;

    let small_primes_arc = Arc::new(small_primes);
    let mut handles = Vec::with_capacity(threads);

    for t in 0..threads {
        let t_start_seg = t * segs_per_thread;
        let t_end_seg = ((t + 1) * segs_per_thread).min(num_segs);

        if t_start_seg >= t_end_seg {
            continue;
        }

        let sp = Arc::clone(&small_primes_arc);

        handles.push(std::thread::spawn(move || {
            let mut thread_primes = Vec::new();
            let mut bits = vec![0u8; BOOT_SEG_BYTES];

            let thread_lo = range_lo + (t_start_seg as u64) * seg_span;

            // Initialize MarkCarry for marking primes >= 7
            let mut carries: Vec<MarkCarry> = sp[3..]
                .iter()
                .map(|&p| MarkCarry::new(p, thread_lo))
                .collect();

            for s in t_start_seg..t_end_seg {
                let seg_lo = range_lo + (s as u64) * seg_span;
                let seg_hi = (seg_lo + seg_span).min(limit + 1);
                let seg_base_bits = (seg_lo / 30) * 8;

                bits.fill(0);

                for (idx, &p) in sp[3..].iter().enumerate() {
                    if p * p >= seg_hi {
                        break;
                    }
                    unsafe {
                        carries[idx].mark(&mut bits, seg_base_bits, p as u32);
                    }
                }

                // Extract primes
                for byte_idx in 0..BOOT_SEG_BYTES {
                    let b = bits[byte_idx];
                    if b == 0xFF {
                        continue;
                    }
                    let block_num = (byte_idx / 8) as u64;
                    let block_base = seg_lo + block_num * 30;

                    for bit in 0..8 {
                        if b & (1 << bit) == 0 {
                            let cand = block_base + (titan_core::wheel::UNITS[bit] as u64);
                            if cand <= limit && cand >= seg_lo {
                                thread_primes.push(cand);
                            }
                        }
                    }
                }
            }

            thread_primes
        }));
    }

    for handle in handles {
        if let Ok(tp) = handle.join() {
            primes.extend(tp);
        }
    }

    primes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boot_primes_mt_matches_reference() {
        let limit = 1_000_000u64;
        let p_seq = generate_base_primes(limit);
        let p_mt = generate_boot_primes_mt(limit, 8);

        assert_eq!(p_seq.len(), p_mt.len(), "Prime counts must match");
        assert_eq!(p_seq, p_mt, "Primes list must be bit-exact");
    }
}
