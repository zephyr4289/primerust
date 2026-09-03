use titan_count::sampled_index::SampledPrimeIndex;
use titan_sieve::base::{generate_base_primes, generate_base_primes_u32};

#[test]
fn test_sampled_index_exact_parity() {
    // Generate 2,000,000 primes (~32.4 million limit)
    let primes_u32 = generate_base_primes_u32(32_452_843);
    assert!(primes_u32.len() >= 2_000_000);

    let index = SampledPrimeIndex::build_u32(&primes_u32);
    println!("Sample table footprint: {} bytes", index.table_bytes());
    assert!(index.table_bytes() <= 64 * 1024); // Must fit in 64 KiB L1D

    // Boundary cases
    assert_eq!(index.pi_u32(&primes_u32, 0), 0);
    assert_eq!(index.pi_u32(&primes_u32, 1), 0);
    assert_eq!(index.pi_u32(&primes_u32, 2), 1);
    assert_eq!(index.pi_u32(&primes_u32, 3), 2);
    assert_eq!(index.pi_u32(&primes_u32, 4), 2);
    assert_eq!(index.pi_u32(&primes_u32, 5), 3);

    let max_p = *primes_u32.last().unwrap() as u64;
    assert_eq!(index.pi_u32(&primes_u32, max_p), primes_u32.len() as u64);
    assert_eq!(index.pi_u32(&primes_u32, max_p + 100), primes_u32.len() as u64);

    // Test exact parity against primes.partition_point across 200,000 random queries
    let mut rng_state: u64 = 0x853c49e6748fea9b;
    let mut xorshift = || {
        rng_state ^= rng_state << 13;
        rng_state ^= rng_state >> 7;
        rng_state ^= rng_state << 17;
        rng_state
    };

    for _ in 0..200_000 {
        let query_v = xorshift() % (max_p + 500);
        let expected = primes_u32.partition_point(|&p| (p as u64) <= query_v) as u64;
        let actual = index.pi_u32(&primes_u32, query_v);
        assert_eq!(
            actual, expected,
            "Parity mismatch at v = {}: expected {}, got {}",
            query_v, expected, actual
        );
    }
}

#[test]
fn test_sampled_index_u64_parity() {
    let primes_u64 = generate_base_primes(500_000);
    let index = SampledPrimeIndex::build(&primes_u64);

    let max_p = *primes_u64.last().unwrap();
    let mut rng_state: u64 = 0x123456789abcdef0;
    let mut xorshift = || {
        rng_state ^= rng_state << 13;
        rng_state ^= rng_state >> 7;
        rng_state ^= rng_state << 17;
        rng_state
    };

    for _ in 0..50_000 {
        let query_v = xorshift() % (max_p + 200);
        let expected = primes_u64.partition_point(|&p| p <= query_v) as u64;
        let actual = index.pi(&primes_u64, query_v);
        assert_eq!(actual, expected, "u64 parity mismatch at v = {}", query_v);
    }
}
