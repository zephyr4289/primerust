//! Wheel-30 Factorization: Convention A single source of truth.
//!
//! Convention A:
//!   - Byte k covers integers [30k, 30k + 29].
//!   - Bit i corresponds to residue RESIDUES[i], where
//!     RESIDUES = [1, 7, 11, 13, 17, 19, 23, 29] in ascending order.
//!   - Exactly 8 bits per byte, representing 30 consecutive integers.

/// The 8 coprime residues modulo 30 in ascending order (Convention A).
pub const RESIDUES: [u8; 8] = [1, 7, 11, 13, 17, 19, 23, 29];

/// Sentinel indicating non-coprime residue.
pub const NON_COPRIME: u8 = 0xFF;

/// Lookup table mapping residue r in 0..30 to bit index 0..8, or NON_COPRIME.
pub const RESIDUE_TO_BIT: [u8; 30] = {
    let mut table = [NON_COPRIME; 30];
    let mut i = 0;
    while i < 8 {
        table[RESIDUES[i] as usize] = i as u8;
        i += 1;
    }
    table
};

/// Additive gaps between consecutive wheel residues:
/// gap[i] = (RESIDUES[(i+1)%8] + 30 - RESIDUES[i]) % 30
/// [6, 4, 2, 4, 2, 4, 6, 2] -> sums to exactly 30!
pub const WHEEL_INC: [u8; 8] = [6, 4, 2, 4, 2, 4, 6, 2];

/// Permutation table for stepping wheel multiples:
/// For prime residue index i and multiplier residue index j:
/// WHEEL_NEXT[i][j] = bit index of (RESIDUES[i] * RESIDUES[j]) mod 30.
pub const WHEEL_NEXT: [[u8; 8]; 8] = {
    let mut table = [[0u8; 8]; 8];
    let mut i = 0;
    while i < 8 {
        let p = RESIDUES[i] as usize;
        let mut j = 0;
        while j < 8 {
            let m = RESIDUES[j] as usize;
            let prod_rem = (p * m) % 30;
            table[i][j] = RESIDUE_TO_BIT[prod_rem];
            j += 1;
        }
        i += 1;
    }
    table
};

/// Smallest coprime residue >= r for any r in 0..30.
pub const NEXT_COPRIME: [u8; 30] = {
    let mut table = [0u8; 30];
    let mut r = 0;
    while r < 30 {
        let mut found = 29;
        let mut i = 0;
        while i < 8 {
            if RESIDUES[i] >= r as u8 {
                found = RESIDUES[i];
                break;
            }
            i += 1;
        }
        table[r] = found;
        r += 1;
    }
    table
};

/// High-mask for end-of-range masking: bits <= i are set (1..=8 bits set).
pub const HIGH_MASK: [u8; 8] = [
    0x01, // bit 0
    0x03, // bits 0..1
    0x07, // bits 0..2
    0x0F, // bits 0..3
    0x1F, // bits 0..4
    0x3F, // bits 0..5
    0x7F, // bits 0..6
    0xFF, // bits 0..7
];

// -----------------------------------------------------------------------
// Compile-time Invariant Assertions
// -----------------------------------------------------------------------
const _: () = {
    // Assert 1: WHEEL_INC sums to 30
    let mut sum = 0;
    let mut i = 0;
    while i < 8 {
        sum += WHEEL_INC[i] as u32;
        i += 1;
    }
    assert!(sum == 30, "WHEEL_INC must sum to 30");

    // Assert 2: Every row of WHEEL_NEXT is a valid permutation of 0..8
    let mut row = 0;
    while row < 8 {
        let mut seen = 0u8;
        let mut col = 0;
        while col < 8 {
            let bit = WHEEL_NEXT[row][col];
            assert!(bit < 8, "WHEEL_NEXT entry out of bounds");
            seen |= 1 << bit;
            col += 1;
        }
        assert!(seen == 0xFF, "WHEEL_NEXT row must be a bijection on 0..8");
        row += 1;
    }
};

/// Convert an integer n to (byte_index, bit_index).
/// Returns None if n is divisible by 2, 3, or 5.
#[inline(always)]
pub fn number_to_slot(n: u64) -> Option<(usize, usize)> {
    let rem = (n % 30) as usize;
    let bit = RESIDUE_TO_BIT[rem];
    if bit == NON_COPRIME {
        None
    } else {
        Some(((n / 30) as usize, bit as usize))
    }
}

/// Convert (byte_index, bit_index) back to the exact integer.
#[inline(always)]
pub fn slot_to_number(byte_idx: usize, bit_idx: usize) -> u64 {
    debug_assert!(bit_idx < 8);
    (byte_idx as u64) * 30 + (RESIDUES[bit_idx] as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_trip_exhaustive() {
        // Test round trip for all numbers up to 3,000,000 (100,000 wheel blocks)
        for byte_idx in 0..100_000 {
            for bit_idx in 0..8 {
                let n = slot_to_number(byte_idx, bit_idx);
                let (b, bit) = number_to_slot(n).expect("coprime slot must map");
                assert_eq!(b, byte_idx);
                assert_eq!(bit, bit_idx);
            }
        }
    }

    #[test]
    fn test_prime_containment_invariant() {
        // Every prime > 5 must have (p % 30) in RESIDUES
        let is_prime = |n: u64| -> bool {
            if n < 2 { return false; }
            if n == 2 || n == 3 { return true; }
            if n % 2 == 0 || n % 3 == 0 { return false; }
            let mut d = 5;
            while d * d <= n {
                if n % d == 0 || n % (d + 2) == 0 { return false; }
                d += 6;
            }
            true
        };

        for p in 7..100_000 {
            if is_prime(p) {
                let rem = (p % 30) as usize;
                assert_ne!(
                    RESIDUE_TO_BIT[rem],
                    NON_COPRIME,
                    "Prime {} was falsely flagged as non-coprime to 30!",
                    p
                );
            }
        }
    }

    #[test]
    fn test_scalar_wheel_sieve_pi_7919() {
        // Standalone scalar wheel sieve up to 7919 (the 1000th prime).
        // Uses purely wheel.rs semantics to prove completeness!
        const LIMIT: usize = 7919;
        let num_bytes = (LIMIT / 30) + 1;
        let mut sieve = vec![0xFFu8; num_bytes];

        // 1 is not prime -> clear bit 0 of byte 0
        sieve[0] &= !(1 << 0);

        let sqrt_limit = (LIMIT as f64).sqrt() as usize;

        // Cross off multiples
        'sieve_primes: for byte in 0..num_bytes {
            let mut bits = sieve[byte];
            while bits != 0 {
                let bit_idx = bits.trailing_zeros() as usize;
                bits &= !(1 << bit_idx);
                let p = slot_to_number(byte, bit_idx) as usize;
                if p > sqrt_limit {
                    break 'sieve_primes;
                }
                if p < 7 {
                    continue; // 2, 3, 5 are wheel primes
                }

                // Cross off multiples p * m for m coprime to 30, starting at m = p
                let mut m_idx = RESIDUE_TO_BIT[p % 30] as usize;
                let mut multiple = p * p;

                while multiple <= LIMIT {
                    if let Some((b, bit)) = number_to_slot(multiple as u64) {
                        if b < num_bytes {
                            sieve[b] &= !(1 << bit);
                        }
                    }
                    // Advance multiple to next coprime multiplier
                    let gap = WHEEL_INC[m_idx] as usize;
                    multiple += p * gap;
                    m_idx = (m_idx + 1) % 8;
                }
            }
        }

        // Count primes: 2, 3, 5 are the 3 wheel primes
        let mut count = 3u64;
        for byte in 0..num_bytes {
            let mut bits = sieve[byte];
            while bits != 0 {
                let bit_idx = bits.trailing_zeros() as usize;
                bits &= !(1 << bit_idx);
                let p = slot_to_number(byte, bit_idx);
                if p <= LIMIT as u64 {
                    count += 1;
                }
            }
        }

        assert_eq!(count, 1000, "pi(7919) must equal exactly 1000!");
    }
}
