//! PreSieve: 1,001-byte cyclic bit-pattern for {7, 11, 13}.
//!
//! Replicated 67x (~66 KiB) so that any segment buffer of size S <= 64 KiB
//! can be initialized in a single linear slice copy from L2 cache.

use titan_core::wheel::{self, RESIDUES, WHEEL_INC};

pub const PATTERN_BYTES: usize = 1001; // 30,030 / 30
pub const REPLICA_COUNT: usize = 132;  // ceil((131072 + 1000) / 1001) covers up to 128 KiB segments
pub const REPLICA_BYTES: usize = PATTERN_BYTES * REPLICA_COUNT; // 132,132 bytes (~129 KiB)

pub struct PreSieve {
    replica: Box<[u8]>,
}

impl PreSieve {
    pub fn new() -> Self {
        let mut base_pattern = vec![0xFFu8; PATTERN_BYTES];

        // Sieve multiples of 7, 11, 13 over the 30,030 number span
        let presieve_primes = [7u64, 11, 13];

        for &p in &presieve_primes {
            let mut m_idx = 0usize; // starts at multiplier 1 (RESIDUES[0])
            let mut multiple = p * (RESIDUES[0] as u64); // p * 1 = p

            while multiple < 30_030 {
                if let Some((b, bit)) = wheel::number_to_slot(multiple) {
                    if b < PATTERN_BYTES {
                        base_pattern[b] &= !(1 << bit);
                    }
                }
                let gap = WHEEL_INC[m_idx] as u64;
                multiple += p * gap;
                m_idx = (m_idx + 1) % 8;
            }
        }

        // Replicate base pattern 67 times
        let mut replica = vec![0u8; REPLICA_BYTES];
        for rep in 0..REPLICA_COUNT {
            let start = rep * PATTERN_BYTES;
            replica[start..start + PATTERN_BYTES].copy_from_slice(&base_pattern);
        }

        Self {
            replica: replica.into_boxed_slice(),
        }
    }

    /// Linear copy into segment buffer of size S bytes.
    /// Offset in replica is `(seg_index * S) % 1001`.
    #[inline(always)]
    pub fn init_segment(&self, seg_index: usize, segment_buf: &mut [u8]) {
        let s = segment_buf.len();
        let offset = (seg_index * s) % PATTERN_BYTES;
        debug_assert!(offset + s <= REPLICA_BYTES);
        segment_buf.copy_from_slice(&self.replica[offset..offset + s]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_presieve_clears_7_11_13_multiples() {
        let presieve = PreSieve::new();
        let mut buf = vec![0u8; PATTERN_BYTES];
        presieve.init_segment(0, &mut buf);

        // Multiples of 7, 11, 13 should be cleared
        for mult in [7u64, 11, 13, 49, 77, 91, 119, 121, 143, 169] {
            let (b, bit) = wheel::number_to_slot(mult).unwrap();
            assert_eq!(
                (buf[b] & (1 << bit)), 0,
                "Multiple {} was not cleared by presieve pattern!",
                mult
            );
        }

        // Numbers not divisible by 2, 3, 5, 7, 11, 13 must remain set (1)
        for cand in [17u64, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61] {
            let (b, bit) = wheel::number_to_slot(cand).unwrap();
            assert_ne!(
                (buf[b] & (1 << bit)), 0,
                "Candidate {} was falsely cleared by presieve!",
                cand
            );
        }
    }

    #[test]
    fn test_replica_window_invariant() {
        let presieve = PreSieve::new();
        let mut seg = vec![0u8; 65536];

        // Test segments 0 through 100 to ensure offsets never overflow replica
        for seg_idx in 0..100 {
            presieve.init_segment(seg_idx, &mut seg);
            assert_eq!(seg.len(), 65536);
        }
    }
}
