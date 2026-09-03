use titan_count::delta_prime_stream::DeltaPrimeStream;
use titan_sieve::base::generate_base_primes;

#[test]
fn test_delta_prime_stream_parity() {
    let raw_primes = generate_base_primes(10_000_000);
    let primes_u32: Vec<u32> = raw_primes.iter().map(|&p| p as u32).collect();
    let stream = DeltaPrimeStream::encode_from_slice(&primes_u32);

    println!("Raw Primes Size : {} MB", (primes_u32.len() * 4) / (1024 * 1024));
    println!("Delta Stream Size: {} MB", stream.memory_bytes() / (1024 * 1024));

    // Verify exact parity on first 10,000 primes
    for (i, &expected) in primes_u32.iter().enumerate().take(10_000) {
        let actual = stream.get(i);
        assert_eq!(actual, expected, "Mismatch at prime index {}", i);
    }

    // Verify cursor sequential iteration
    let mut cursor = stream.cursor_from(0);
    assert_eq!(cursor.current(), 2);
    for (i, &expected) in primes_u32.iter().enumerate().skip(1).take(10_000) {
        let p = cursor.next_prime() as u32;
        assert_eq!(p, expected, "Cursor mismatch at prime index {}", i);
    }
}
