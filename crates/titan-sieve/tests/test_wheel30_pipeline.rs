//! Integration tests for Wheel-30 Sieve Pipeline (Phase 6.2 & Phase 6.3).
//!
//! Validates end-to-end exactness:
//! - Tiny prime NEON vector mask sifting (Tier 0: 7..=31)
//! - Dense rotating register kernel with dynamic safe limit (Tier 1: 37..=1200)
//! - Tier 2 medium prime stride kernel (Tier 2: 1201..=32768)
//! - Vector NEON popcount
//! - Ground truth comparison against reference prime sieve for 491,520 integers

use titan_sieve::wheel30::{SEGMENT_BYTES, BIT_TO_RESIDUE, Wheel30PrimeState};
use titan_sieve::wheel30_dense::sieve_tier1_prime_dynamic;
use titan_sieve::wheel30_medium::MediumPrimeState;
use titan_sieve::wheel30_tiny::TinyPrimeMaskTable;
use titan_sieve::wheel30_popcount::wheel30_popcount_neon;
use titan_sieve::base::generate_base_primes;

#[test]
fn test_wheel30_tier0_tier1_dynamic_parity() {
    let mut buf = [0xFFu8; SEGMENT_BYTES];

    // 1. Sift Tier 0 tiny primes (7..=31) via NEON vector masks
    let tiny_table = TinyPrimeMaskTable::new();
    unsafe {
        tiny_table.sieve_tiny_primes(&mut buf, 0);
    }

    // 2. Sift Tier 1 primes (37..=1,200) via dynamic safe limit
    let base_primes = generate_base_primes(1200);
    for &p in &base_primes {
        if p < 37 {
            continue;
        }
        let mut state = Wheel30PrimeState::compile(p as u32, 0);
        unsafe {
            sieve_tier1_prime_dynamic(&mut state, p as u32, &mut buf);
        }
    }

    // 3. Count survivors in buffer
    let survivor_count = unsafe { wheel30_popcount_neon(&buf) };

    // 4. Compute ground truth: count how many integers in [0, WHEEL_SPAN) coprime to 30
    // have NO prime factor <= 1200
    let mut expected_count = 0u64;
    for byte in 0..SEGMENT_BYTES {
        for bit in 0..8 {
            let res = BIT_TO_RESIDUE[bit] as u64;
            let n = (byte as u64) * 30 + res;

            let is_composite = base_primes.iter().any(|&p| p >= 7 && n % p == 0 && n != p);
            let bit_is_set = (buf[byte] & (1 << bit)) != 0;

            if !is_composite {
                expected_count += 1;
                assert!(
                    bit_is_set,
                    "Expected n = {} to be unmarked (survivor), but was marked!",
                    n
                );
            } else {
                assert!(
                    !bit_is_set,
                    "Expected composite n = {} to be marked (cleared), but bit was set!",
                    n
                );
            }
        }
    }

    assert_eq!(
        survivor_count, expected_count,
        "Survivor popcount mismatch! got {}, expected {}",
        survivor_count, expected_count
    );
}

#[test]
fn test_wheel30_with_medium_primes_parity() {
    let mut buf = [0xFFu8; SEGMENT_BYTES];

    // 1. Sift Tier 0 tiny primes (7..=31)
    let tiny_table = TinyPrimeMaskTable::new();
    unsafe {
        tiny_table.sieve_tiny_primes(&mut buf, 0);
    }

    // 2. Sift Tier 1 primes (37..=1,200)
    let base_primes = generate_base_primes(5000);
    for &p in &base_primes {
        if p < 37 || p > 1200 {
            continue;
        }
        let mut state = Wheel30PrimeState::compile(p as u32, 0);
        unsafe {
            sieve_tier1_prime_dynamic(&mut state, p as u32, &mut buf);
        }
    }

    // 3. Sift Tier 2 medium primes (1,201..=5,000)
    for &p in &base_primes {
        if p <= 1200 {
            continue;
        }
        let mut state = MediumPrimeState::compile(p as u32, 0);
        unsafe {
            state.sieve_segment(&mut buf);
        }
    }

    // 4. Count survivors in buffer
    let survivor_count = unsafe { wheel30_popcount_neon(&buf) };

    // 5. Compute ground truth up to 5,000
    let mut expected_count = 0u64;
    for byte in 0..SEGMENT_BYTES {
        for bit in 0..8 {
            let res = BIT_TO_RESIDUE[bit] as u64;
            let n = (byte as u64) * 30 + res;

            let is_composite = base_primes.iter().any(|&p| p >= 7 && n % p == 0 && n != p);
            let bit_is_set = (buf[byte] & (1 << bit)) != 0;

            if !is_composite {
                expected_count += 1;
                assert!(
                    bit_is_set,
                    "Expected n = {} to be unmarked (survivor), but was marked!",
                    n
                );
            } else {
                assert!(
                    !bit_is_set,
                    "Expected composite n = {} to be marked (cleared), but bit was set!",
                    n
                );
            }
        }
    }

    assert_eq!(
        survivor_count, expected_count,
        "Survivor popcount mismatch! got {}, expected {}",
        survivor_count, expected_count
    );
}
