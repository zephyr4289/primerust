//! Phase 7.2: 3-Instruction O(1) Segmented Pi Table (segmented_pi.rs).
//!
//! Stores exact prime counts and Wheel-30 coprime bitmasks for every 240 integers.
//! 240 integers / 30 = 8 bytes * 8 coprime residues = exactly 64 bits (1 x u64).
//!
//! Replaces vector popcount loops with a 3-instruction sequence:
//!   LDP (count, bits) -> AND (bits & mask) -> POPCNT -> ADD

use std::alloc::{alloc_zeroed, dealloc, Layout};

pub const INTEGERS_PER_WORD: usize = 240;

const PI_TINY: [u64; 6] = [0, 0, 1, 2, 2, 3];

#[repr(C, align(16))]
#[derive(Clone, Copy, Default)]
pub struct PiWord {
    pub count: u64,
    pub bits: u64,
}

pub struct SegmentedPiTable {
    pub low: u64,
    pub high: u64,
    words: *mut PiWord,
    word_count: usize,
    layout: Layout,
    unset_larger: [u64; INTEGERS_PER_WORD],
}

unsafe impl Send for SegmentedPiTable {}
unsafe impl Sync for SegmentedPiTable {}

impl SegmentedPiTable {
    pub fn new(low: u64, high: u64, primes: &[u64]) -> Self {
        let low_aligned = (low / 240) * 240;
        let high_aligned = ((high + 239) / 240) * 240;
        let range = (high_aligned - low_aligned) as usize;
        let word_count = (range / INTEGERS_PER_WORD).max(1);

        let layout = Layout::array::<PiWord>(word_count)
            .unwrap()
            .align_to(16)
            .unwrap();
        let words = unsafe { alloc_zeroed(layout) as *mut PiWord };
        assert!(!words.is_null(), "SegmentedPiTable allocation failed");

        // 1. Build precomputed UNSET_LARGER bitmask table (240 entries)
        let mut unset_larger = [0u64; INTEGERS_PER_WORD];
        for n in 0..INTEGERS_PER_WORD {
            let byte_idx = n / 30;
            let res = n % 30;
            let bits_in_byte = match res {
                r if r >= 29 => 8,
                r if r >= 23 => 7,
                r if r >= 19 => 6,
                r if r >= 17 => 5,
                r if r >= 13 => 4,
                r if r >= 11 => 3,
                r if r >= 7 => 2,
                r if r >= 1 => 1,
                _ => 0,
            };
            let total_bits = byte_idx * 8 + bits_in_byte;
            unset_larger[n] = if total_bits >= 64 {
                !0u64
            } else if total_bits == 0 {
                0u64
            } else {
                (1u64 << total_bits) - 1
            };
        }

        // 2. Set bits for prime residues
        for &p in primes {
            if p < low_aligned || p >= high_aligned {
                continue;
            }
            if p <= 5 {
                continue; // 2, 3, 5 pre-filtered by Wheel-30
            }

            let p_offset = (p - low_aligned) as usize;
            let word_idx = p_offset / INTEGERS_PER_WORD;
            let rem = p_offset % INTEGERS_PER_WORD;

            let byte_idx = rem / 30;
            let res = rem % 30;
            let bit_in_byte = match res {
                1 => 0,
                7 => 1,
                11 => 2,
                13 => 3,
                17 => 4,
                19 => 5,
                23 => 6,
                29 => 7,
                _ => continue,
            };
            let total_bit = (byte_idx * 8) + bit_in_byte;
            unsafe {
                (*words.add(word_idx)).bits |= 1u64 << total_bit;
            }
        }

        // 3. Compute running prefix counts across 240-integer words
        let initial_count = if low_aligned <= 5 {
            3
        } else {
            primes.partition_point(|&p| p < low_aligned) as u64
        };
        let mut running_count = initial_count;

        for w in 0..word_count {
            unsafe {
                let entry = &mut *words.add(w);
                entry.count = running_count;
                running_count += entry.bits.count_ones() as u64;
            }
        }

        Self {
            low: low_aligned,
            high: high_aligned,
            words,
            word_count,
            layout,
            unset_larger,
        }
    }

    pub fn from_u32(low: u64, high: u64, primes: &[u32]) -> Self {
        let p64: Vec<u64> = primes.iter().map(|&p| p as u64).collect();
        Self::new(low, high, &p64)
    }

    /// Evaluates π(x) in exactly 3-4 cycles via 1 table load, 1 mask, 1 popcnt, 1 add.
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
        let word_idx = offset / INTEGERS_PER_WORD;
        let rem = offset % INTEGERS_PER_WORD;

        unsafe {
            let entry = &*self.words.add(word_idx);
            let mask = *self.unset_larger.get_unchecked(rem);
            entry.count + (entry.bits & mask).count_ones() as u64
        }
    }
}

impl Drop for SegmentedPiTable {
    fn drop(&mut self) {
        unsafe {
            dealloc(self.words as *mut u8, self.layout);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_segmented_pi_milestones() {
        let primes = titan_sieve::base::generate_base_primes(100_000);
        let table = SegmentedPiTable::new(0, 100_000, &primes);

        assert_eq!(table.pi(10), 4);
        assert_eq!(table.pi(100), 25);
        assert_eq!(table.pi(1_000), 168);
        assert_eq!(table.pi(10_000), 1229);
        assert_eq!(table.pi(99_991), 9592);
    }
}
