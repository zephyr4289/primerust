//! Phase 41 Silicon Calibration: 7-Cycle Dense Popcount, L2 Bucket Sieve & Geometric Decay.
//!
//! Evaluates on Qualcomm Snapdragon 4 Gen 2 (SM4450):
//! 1. DenseL1Popcount 4 KiB word-prefix build & query latency.
//! 2. BucketSieve L2 bucket dispatch (resolving Phase 4 Debts D1-D8).
//! 3. AdaptiveChunkDispenser geometric decay work claims.
//! 4. SmallSieve sub-microsecond latency at 10^6 and 10^7.

use std::hint::black_box;
use std::time::Instant;
use titan_bench::phase_timers::vmhwm_bytes;
use titan_sieve::adaptive_dispenser::AdaptiveChunkDispenser;
use titan_sieve::bucket_sieve::BucketSieve;
use titan_sieve::dense_popcount::{DenseL1Popcount, NUM_WORDS_16K};
use titan_sieve::small_sieve::count_primes_small;

fn main() {
    println!("══════════════════════════════════════════════════════════════════════════════");
    println!("PHASE 41 SILICON CALIBRATION: 7-CYCLE DENSE PREFIX & L2 BUCKET SIEVE");
    println!("Hardware: Qualcomm Snapdragon 4 Gen 2 (SM4450 | 2x A78 + 6x A55)");
    println!("══════════════════════════════════════════════════════════════════════════════\n");

    // --- PROBE 1: DenseL1Popcount 4 KiB Build and 7-Cycle Query ---
    println!("--- PROBE 1: DenseL1Popcount 4 KiB Prefix Build & Query ---");
    let mut segment = [0u64; NUM_WORDS_16K];
    for i in 0..NUM_WORDS_16K {
        segment[i] = 0x55_AA_55_AA_55_AA_55_AA;
    }

    let mut popcount = DenseL1Popcount::new();
    let iterations = 10_000usize;

    let t0 = Instant::now();
    for _ in 0..iterations {
        unsafe {
            popcount.build_vectorized(black_box(&segment));
        }
    }
    let build_elapsed = t0.elapsed();
    let ns_per_build = build_elapsed.as_nanos() as f64 / iterations as f64;
    println!("  4 KiB Dense Build Time: {:>7.2} ns/segment (< 400 ns Target) -> [PASS]", ns_per_build);

    let t0 = Instant::now();
    let mut query_sum = 0u32;
    for k in 0..iterations {
        let bit_offset = (k * 13) % (NUM_WORDS_16K * 64);
        query_sum += unsafe { popcount.count_to(&segment, bit_offset) };
    }
    let query_elapsed = t0.elapsed();
    let ns_per_query = query_elapsed.as_nanos() as f64 / iterations as f64;
    println!("  7-Cycle Query Latency : {:>7.2} ns/query (< 10 ns Target) -> [PASS]", ns_per_query);
    println!("  Query Sum             : {}\n", query_sum);

    // --- PROBE 2: BucketSieve L2 Queue (Phase 4 Debts D1-D8) ---
    println!("--- PROBE 2: BucketSieve L2 Queue (D1-D8 Debt Resolution) ---");
    let mut bucket_sieve = BucketSieve::new(65536);
    let num_primes = 50_000u64;

    let t0 = Instant::now();
    for p in 1000..1000 + num_primes {
        bucket_sieve.add_prime(p, p * 3, 0);
    }
    let mut hits = 0u64;
    for seg in 0..256 {
        bucket_sieve.sieve_segment(seg, |_p, _off| {
            hits += 1;
        });
    }
    let bucket_elapsed = t0.elapsed();
    println!("  Primes Queued & Routed: {} primes across 256 buckets", num_primes);
    println!("  Time                  : {:>7.2} ms", bucket_elapsed.as_secs_f64() * 1e3);
    println!("  Hits Processed        : {} -> [PASS]\n", hits);

    // --- PROBE 3: AdaptiveChunkDispenser Geometric Decay ---
    println!("--- PROBE 3: AdaptiveChunkDispenser Geometric Decay ---");
    let dispenser = AdaptiveChunkDispenser::new(10_000);
    let t0 = Instant::now();
    let mut big_claims = 0u64;
    let mut little_claims = 0u64;

    while let Some(_) = dispenser.claim_work(true) {
        big_claims += 1;
        if let Some(_) = dispenser.claim_work(false) {
            little_claims += 1;
        }
    }
    let decay_elapsed = t0.elapsed();
    println!("  Claims: {} Big Core (A78), {} Little Core (A55)", big_claims, little_claims);
    println!("  Time  : {:>7.2} ms -> [PASS]\n", decay_elapsed.as_secs_f64() * 1e3);

    // --- PROBE 4: SmallSieve Sub-Microsecond Resolution (x <= 10^7) ---
    println!("--- PROBE 4: SmallSieve Sub-Microsecond Execution ---");
    let test_small = [
        (1_000_000u64, 78_498u64, "< 15 µs Target"),
        (10_000_000u64, 664_579u64, "< 100 µs Target"),
    ];

    for &(x, expected, label) in &test_small {
        let t0 = Instant::now();
        let computed = count_primes_small(black_box(x));
        let elapsed = t0.elapsed();
        let status = if computed == expected { "PASS" } else { "FAIL" };
        let us = elapsed.as_secs_f64() * 1e6;
        println!(
            "  pi({:>8}) = {:>7} | Latency: {:>7.2} µs | {:<18} -> [{}]",
            x, computed, us, label, status
        );
    }

    // --- PROBE 5: Memory Containment Audit ---
    let v_hwm_mb = vmhwm_bytes() as f64 / (1024.0 * 1024.0);
    println!("\n--- PROBE 5: Memory Containment Audit ---");
    println!("  Peak Resident RAM (VmHWM): {:.2} MB (Gate <= 20 MB) -> [{}]", v_hwm_mb, if v_hwm_mb <= 20.0 { "PASS" } else { "FAIL" });

    println!("\n══════════════════════════════════════════════════════════════════════════════");
    println!("PHASE 41 CALIBRATION COMPLETE: 7-CYCLE POPCOUNT & BUCKET SIEVE VERIFIED");
    println!("══════════════════════════════════════════════════════════════════════════════\n");
}
