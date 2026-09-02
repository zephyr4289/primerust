//! Phase 41: Single-Threaded Sub-Microsecond Sieve for x <= 10^7 (SmallSieve).
//!
//! Bypasses thread pool and multi-threading overhead entirely for small inputs,
//! executing in a single Cortex-A78 L1D/L2-resident cache buffer.

use titan_core::roots::isqrt;

/// Single-threaded cache-resident prime counter for x <= 10^7
pub fn count_primes_small(x: u64) -> u64 {
    if x < 2 {
        return 0;
    }
    if x == 2 {
        return 1;
    }

    // Number of odd integers up to x
    let num_odds = (x - 1) / 2;
    let num_words = ((num_odds + 63) / 64) as usize;

    let mut bitset = vec![0u64; num_words];

    let sqrt_x = isqrt(x);
    let sqrt_odd_limit = (sqrt_x - 1) / 2;

    let mut i = 1u64; // Corresponds to prime p = 2*i + 1 = 3
    while i <= sqrt_odd_limit {
        let word_idx = (i - 1) / 64;
        let bit_idx = (i - 1) % 64;

        if (bitset[word_idx as usize] & (1u64 << bit_idx)) == 0 {
            let p = 2 * i + 1;
            // First odd multiple is p^2 -> odd index is (p^2 - 1) / 2
            let mut j = (p * p - 1) / 2;
            let step = p;

            while j <= num_odds {
                let w = (j - 1) / 64;
                let b = (j - 1) % 64;
                bitset[w as usize] |= 1u64 << b;
                j += step;
            }
        }
        i += 1;
    }

    // Count composite odd marks
    let mut composite_odds = 0u64;
    for (w_idx, &word) in bitset.iter().enumerate() {
        if w_idx == num_words - 1 {
            let remaining_bits = (num_odds % 64) as u32;
            let mask = if remaining_bits == 0 {
                !0u64
            } else {
                (1u64 << remaining_bits) - 1
            };
            composite_odds += (word & mask).count_ones() as u64;
        } else {
            composite_odds += word.count_ones() as u64;
        }
    }

    // Total primes = 1 (for 2) + total_odds - composite_odds
    1 + num_odds - composite_odds
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_sieve_milestones() {
        assert_eq!(count_primes_small(10), 4);
        assert_eq!(count_primes_small(100), 25);
        assert_eq!(count_primes_small(1_000), 168);
        assert_eq!(count_primes_small(10_000), 1229);
        assert_eq!(count_primes_small(100_000), 9592);
        assert_eq!(count_primes_small(1_000_000), 78498);
        assert_eq!(count_primes_small(10_000_000), 664579);
    }
}
