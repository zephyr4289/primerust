//! Phase 8.1: Compact L2-Locked Segmented Pi Table (segmented_pi_compact.rs).
//!
//! Separates counts (u32) and masks (u64).
//! For z = 27.2M, the primary count array takes exactly 453.6 KiB,
//! fitting 100% within the Cortex-A78's 512 KiB private L2 cache.

use std::alloc::{alloc_zeroed, dealloc, Layout};

pub const INTEGERS_PER_WORD: usize = 240;
const WHEEL30_RESIDUES: [u8; 8] = [1, 7, 11, 13, 17, 19, 23, 29];
const PI_TINY: [u64; 6] = [0, 0, 1, 2, 2, 3];

pub struct CompactPiTable {
    pub low: u64,
    pub high: u64,
    counts: *mut u32,
    bits: *mut u64,
    word_count: usize,
    counts_layout: Layout,
    bits_layout: Layout,
    unset_larger: [u64; INTEGERS_PER_WORD],
}

unsafe impl Send for CompactPiTable {}
unsafe impl Sync for CompactPiTable {}

impl CompactPiTable {
    pub fn new(low: u64, high: u64, primes: &[u64]) -> Self {
        let low_aligned = (low / 240) * 240;
        let high_aligned = ((high + 239) / 240) * 240;
        let range = (high_aligned - low_aligned) as usize;
        let word_count = (range / INTEGERS_PER_WORD).max(1);

        // Counts array: 4 bytes per 240 integers (453.6 KiB at z=27.2M)
        let counts_layout = Layout::array::<u32>(word_count).unwrap().align_to(64).unwrap();
        // Bits array: 8 bytes per 240 integers
        let bits_layout = Layout::array::<u64>(word_count).unwrap().align_to(64).unwrap();

        let counts = unsafe { alloc_zeroed(counts_layout) as *mut u32 };
        let bits = unsafe { alloc_zeroed(bits_layout) as *mut u64 };
        assert!(!counts.is_null() && !bits.is_null(), "Allocation failed");

        let mut unset_larger = [0u64; INTEGERS_PER_WORD];
        for rem in 0..INTEGERS_PER_WORD {
            let mut mask = 0u64;
            let mut bit_idx = 0;
            for byte_idx in 0..8 {
                let base_int = byte_idx * 30;
                for &res in &WHEEL30_RESIDUES {
                    if base_int + res as usize <= rem {
                        mask |= 1u64 << bit_idx;
                    }
                    bit_idx += 1;
                }
            }
            unset_larger[rem] = mask;
        }

        // Set prime coprime bits
        for &p in primes {
            if p < low_aligned || p >= high_aligned || p <= 5 { continue; }
            let offset = (p - low_aligned) as usize;
            let word_idx = offset / INTEGERS_PER_WORD;
            let rem = offset % INTEGERS_PER_WORD;
            let byte_idx = rem / 30;
            let res = (rem % 30) as u8;
            if let Some(bit_pos) = WHEEL30_RESIDUES.iter().position(|&r| r == res) {
                unsafe {
                    *bits.add(word_idx) |= 1u64 << ((byte_idx * 8) + bit_pos);
                }
            }
        }

        // Populate running prefix sums into 32-bit counts array
        let initial_count = if low_aligned <= 5 {
            3u32
        } else {
            primes.partition_point(|&p| p < low_aligned) as u32
        };

        let mut running = initial_count;
        for w in 0..word_count {
            unsafe {
                *counts.add(w) = running;
                running += (*bits.add(w)).count_ones();
            }
        }

        Self {
            low: low_aligned,
            high: high_aligned,
            counts,
            bits,
            word_count,
            counts_layout,
            bits_layout,
            unset_larger,
        }
    }

    #[inline(always)]
    pub fn pi(&self, mut x: u64) -> u64 {
        if x < 6 {
            return PI_TINY[x as usize];
        }
        if x < self.low {
            return 3;
        }
        if x >= self.high {
            x = self.high - 1;
        }
        let offset = (x - self.low) as usize;
        let w_idx = offset / INTEGERS_PER_WORD;
        let rem = offset % INTEGERS_PER_WORD;

        unsafe {
            let base_count = *self.counts.add(w_idx) as u64;
            let word_bits = *self.bits.add(w_idx);
            let mask = *self.unset_larger.get_unchecked(rem);
            base_count + (word_bits & mask).count_ones() as u64
        }
    }
}

impl Drop for CompactPiTable {
    fn drop(&mut self) {
        unsafe {
            dealloc(self.counts as *mut u8, self.counts_layout);
            dealloc(self.bits as *mut u8, self.bits_layout);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::segmented_pi::SegmentedPiTable;

    fn generate_primes(limit: u64) -> Vec<u64> {
        let mut sieve = vec![true; (limit + 1) as usize];
        let mut primes = Vec::new();
        for p in 2..=limit {
            if sieve[p as usize] {
                primes.push(p);
                for i in (p * p..=limit).step_by(p as usize) {
                    sieve[i as usize] = false;
                }
            }
        }
        primes
    }

    #[test]
    fn test_compact_pi_table_exact_match() {
        let primes = generate_primes(50_000);
        let standard = SegmentedPiTable::new(0, 50_000, &primes);
        let compact = CompactPiTable::new(0, 50_000, &primes);

        for x in 0..50_000 {
            assert_eq!(compact.pi(x), standard.pi(x), "Mismatch at x = {}", x);
        }
    }
}
