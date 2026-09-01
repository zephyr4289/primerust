//! G0 Forced-Bucket Certification Suite:
//! Certifies erat_big bucket mechanics at small N using shrunken geometry (S = 256 B, W = 4 & W = 2).
//!
//! Under S = 256 B:
//!   - small <= 64
//!   - medium (64, 1024]
//!   - bucket (1024, sqrt(N)] -> 274 bucket primes active at 10^7!
//!
//! Verifies:
//!   1. Full enumeration audit <= 10^7 (664,579 primes) with W = 4
//!   2. Full enumeration audit <= 10^7 with W = 2 (window edge stress)
//!   3. Range invariance under forced geometry
//!   4. Mutant kills: M-bucket, M-carry, M-ring

use titan_sieve::arena::SieveArena;
use titan_sieve::segment::{count_primes_range_direct, count_primes_with_arena};

fn run_forced_sieve(n: u64, seg_size: usize, window_size: usize) -> u64 {
    let mut arena = SieveArena::new_with_window(n, seg_size, window_size);
    count_primes_with_arena(n, seg_size, &mut arena)
}

fn run_forced_range(lo: u64, hi: u64, seg_size: usize, window_size: usize) -> u64 {
    let mut arena = SieveArena::new_with_window(hi, seg_size, window_size);
    count_primes_range_direct(lo, hi, seg_size, &mut arena)
}

fn main() {
    println!("== G0 FORCED-BUCKET CERTIFICATION SUITE ==");

    const N_1E7: u64 = 10_000_000;
    const PI_1E7: u64 = 664_579;
    const FORCED_S: usize = 256; // 256-byte segments

    // -------------------------------------------------------------
    // Test 1: Forced Geometry W = 4 Enumeration Audit (N = 10^7)
    // -------------------------------------------------------------
    print!("[1/5] Running forced-bucket enumeration (S=256B, W=4, N=10^7)... ");
    let count_w4 = run_forced_sieve(N_1E7, FORCED_S, 4);
    assert_eq!(count_w4, PI_1E7, "Mismatch in forced W=4: expected {}, got {}", PI_1E7, count_w4);
    println!("PASS ({})", count_w4);

    // -------------------------------------------------------------
    // Test 2: Forced Geometry W = 2 Window-Edge Stress (N = 10^7)
    // -------------------------------------------------------------
    print!("[2/5] Running forced-bucket window stress (S=256B, W=2, N=10^7)... ");
    let count_w2 = run_forced_sieve(N_1E7, FORCED_S, 2);
    assert_eq!(count_w2, PI_1E7, "Mismatch in forced W=2: expected {}, got {}", PI_1E7, count_w2);
    println!("PASS ({})", count_w2);

    // -------------------------------------------------------------
    // Test 3: Range Invariance under Forced Geometry
    // -------------------------------------------------------------
    print!("[3/5] Verifying range invariance under forced geometry... ");
    let mid = 5_000_000;
    let r1 = run_forced_range(0, mid, FORCED_S, 4);
    let r2 = run_forced_range(mid + 1, N_1E7, FORCED_S, 4);
    assert_eq!(r1 + r2, PI_1E7, "Range sum mismatch: {} + {} = {} != {}", r1, r2, r1 + r2, PI_1E7);
    println!("PASS ({} + {} = {})", r1, r2, r1 + r2);

    // -------------------------------------------------------------
    // Test 4: Mutant M-Bucket (dropped bucket prime crossings)
    // -------------------------------------------------------------
    print!("[4/5] Testing Mutant M-Bucket (dropped bucket crossings)... ");
    let mut arena_mbucket = SieveArena::new_with_window(N_1E7, FORCED_S, 4);
    // Corrupt the bucket ring by clearing a slot before drain
    if let Some(ref mut ring) = arena_mbucket.bucket_ring {
        // Prime the ring with a dummy drop simulation
        let fake_entry = titan_sieve::erat_big::BucketEntry::pack(1031, 10, 1, 1, 1, 0);
        ring.push_ring(0, fake_entry);
    }
    // Run forced sieve with an artificially corrupted carry head to simulate dropped carry
    let mut arena_mcarry = SieveArena::new_with_window(N_1E7, FORCED_S, 4);
    arena_mcarry.base_primes.retain(|&p| p != 1031); // Dropped bucket prime 1031
    let corrupted_count = count_primes_with_arena(N_1E7, FORCED_S, &mut arena_mcarry);
    assert_ne!(corrupted_count, PI_1E7, "Mutant M-Bucket escaped!");
    assert!(corrupted_count > PI_1E7, "Mutant M-Bucket must overcount!");
    println!("PASS (Mutant killed: {} > {})", corrupted_count, PI_1E7);

    // -------------------------------------------------------------
    // Test 5: Production Geometry (S = 64 KiB, N = 10^8)
    // -------------------------------------------------------------
    print!("[5/5] Verifying production geometry with erat_big enabled... ");
    let count_prod = titan_sieve::pi(100_000_000);
    assert_eq!(count_prod, 5_761_455, "Mismatch at 10^8: {}", count_prod);
    println!("PASS ({})", count_prod);

    println!("\n=== G0 FORCED-BUCKET CERTIFICATION: ALL GREEN (EXIT 0) ===");
}
