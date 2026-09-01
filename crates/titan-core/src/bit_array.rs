//! BitWindow: zero-allocation borrowed view over a segment byte slice.
//!
//! Provides bit-level get/set/clear, masked boundary updates,
//! and word-wise popcount counting (the SIMD swap point).

/// Borrowed mutable bit window over an external byte slice.
pub struct BitWindow<'a> {
    data: &'a mut [u8],
    num_bits: u32,
}

impl<'a> BitWindow<'a> {
    /// Create a new BitWindow over a mutable byte slice.
    #[inline]
    pub fn new(data: &'a mut [u8]) -> Self {
        let num_bits = (data.len() as u64 * 8).min(u32::MAX as u64) as u32;
        Self { data, num_bits }
    }

    /// Total number of bits in the window.
    #[inline(always)]
    pub fn len_bits(&self) -> u32 {
        self.num_bits
    }

    /// Access underlying byte slice.
    #[inline(always)]
    pub fn as_bytes(&self) -> &[u8] {
        self.data
    }

    /// Access underlying mutable byte slice.
    #[inline(always)]
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        self.data
    }

    /// Get bit value at bit_idx.
    #[inline(always)]
    pub fn get(&self, bit_idx: u32) -> bool {
        debug_assert!(bit_idx < self.num_bits);
        let byte = (bit_idx >> 3) as usize;
        let bit = (bit_idx & 7) as u8;
        (self.data[byte] & (1 << bit)) != 0
    }

    /// Set bit at bit_idx (1).
    #[inline(always)]
    pub fn set(&mut self, bit_idx: u32) {
        debug_assert!(bit_idx < self.num_bits);
        let byte = (bit_idx >> 3) as usize;
        let bit = (bit_idx & 7) as u8;
        self.data[byte] |= 1 << bit;
    }

    /// Clear bit at bit_idx (0).
    #[inline(always)]
    pub fn clear(&mut self, bit_idx: u32) {
        debug_assert!(bit_idx < self.num_bits);
        let byte = (bit_idx >> 3) as usize;
        let bit = (bit_idx & 7) as u8;
        self.data[byte] &= !(1 << bit);
    }

    /// Fill all bytes with val.
    #[inline]
    pub fn fill(&mut self, val: u8) {
        self.data.fill(val);
    }

    /// Mask out all bits strictly above last_valid_bit in the window.
    pub fn mask_above(&mut self, last_valid_bit: u32) {
        if last_valid_bit >= self.num_bits {
            return;
        }
        let byte = (last_valid_bit >> 3) as usize;
        let bit_in_byte = (last_valid_bit & 7) as usize;
        let mask = crate::wheel::HIGH_MASK[bit_in_byte];
        self.data[byte] &= mask;
        if byte + 1 < self.data.len() {
            self.data[byte + 1..].fill(0);
        }
    }

    /// Count the number of set bits in [lo, hi).
    ///
    /// Evaluates using aligned 64-bit word popcounts over the interior,
    /// with exact bit-masking on unaligned head and tail boundaries.
    /// This is the exact reference contract for Phase 2's SIMD swap point.
    pub fn count_range(&self, lo: u32, hi: u32) -> u64 {
        if lo >= hi || lo >= self.num_bits {
            return 0;
        }
        let hi = hi.min(self.num_bits);

        let start_byte = (lo >> 3) as usize;
        let _end_byte = ((hi + 7) >> 3) as usize;

        if start_byte == ((hi - 1) >> 3) as usize {
            // Same byte range
            let lo_bit = lo & 7;
            let hi_bit = (hi - 1) & 7;
            let mask = (crate::wheel::HIGH_MASK[hi_bit as usize])
                & !(crate::wheel::HIGH_MASK[lo_bit as usize] >> 1);
            return (self.data[start_byte] & mask).count_ones() as u64;
        }

        let mut count = 0u64;

        // Head byte (unaligned lo)
        let head_rem = lo & 7;
        let mut cur_byte = start_byte;
        if head_rem != 0 {
            let mask = !(crate::wheel::HIGH_MASK[(head_rem - 1) as usize]);
            count += (self.data[cur_byte] & mask).count_ones() as u64;
            cur_byte += 1;
        }

        // Tail byte (unaligned hi)
        let last_byte = (hi >> 3) as usize;
        let tail_rem = hi & 7;

        // Aligned 64-bit word interior
        let interior_end = last_byte;
        while cur_byte + 8 <= interior_end {
            let chunk = u64::from_ne_bytes(
                self.data[cur_byte..cur_byte + 8].try_into().unwrap()
            );
            count += chunk.count_ones() as u64;
            cur_byte += 8;
        }

        // Leftover interior bytes
        while cur_byte < interior_end {
            count += self.data[cur_byte].count_ones() as u64;
            cur_byte += 1;
        }

        // Tail byte
        if tail_rem != 0 && cur_byte < self.data.len() {
            let mask = crate::wheel::HIGH_MASK[(tail_rem - 1) as usize];
            count += (self.data[cur_byte] & mask).count_ones() as u64;
        }

        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bit_get_set_clear_roundtrip() {
        let mut storage = vec![0u8; 128];
        let mut window = BitWindow::new(&mut storage);

        assert_eq!(window.len_bits(), 1024);
        assert!(!window.get(42));

        window.set(42);
        assert!(window.get(42));

        window.clear(42);
        assert!(!window.get(42));
    }

    #[test]
    fn test_count_range_differential_against_scalar_oracle() {
        // Scalar bit-by-bit truth oracle
        let scalar_count = |slice: &[u8], lo: u32, hi: u32| -> u64 {
            let mut c = 0u64;
            for i in lo..hi {
                let byte = (i >> 3) as usize;
                let bit = (i & 7) as u8;
                if (slice[byte] & (1 << bit)) != 0 {
                    c += 1;
                }
            }
            c
        };

        // Pseudo-random bit pattern
        let mut storage = vec![0u8; 256];
        let mut seed = 0x9E37_79B9_7F4A_7C15u64;
        for b in storage.iter_mut() {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            *b = (seed >> 32) as u8;
        }

        let window = BitWindow::new(&mut storage);

        // Sweep all combinations of lo and hi alignments
        for lo in 0..64 {
            for hi in lo..256 {
                let expected = scalar_count(window.as_bytes(), lo, hi);
                let actual = window.count_range(lo, hi);
                assert_eq!(
                    actual, expected,
                    "count_range({}, {}) failed! Expected {}, got {}",
                    lo, hi, expected, actual
                );
            }
        }
    }

    #[test]
    fn test_mask_above() {
        let mut storage = vec![0xFFu8; 4];
        let mut window = BitWindow::new(&mut storage);

        window.mask_above(11); // keep bits 0..=11 (12 bits total: byte 0 + lower 4 bits of byte 1)
        assert_eq!(window.as_bytes()[0], 0xFF);
        assert_eq!(window.as_bytes()[1], 0x0F);
        assert_eq!(window.as_bytes()[2], 0x00);
        assert_eq!(window.as_bytes()[3], 0x00);
    }
}
