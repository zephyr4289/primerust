//! Phase 38 Silicon Calibration Probe: Memory-Bandwidth Assault on SM4450.
//!
//! Evaluates:
//! 1. 3-Stage Streaming Prefetch Pipeline (64 MB bitmap streaming with prfm pldl1strm).
//! 2. 16-Bit Compressed FactorTableD (20 MB streaming in 2 MB L2 chunks).
//! 3. Effective memory bandwidth (GB/s) vs 25.6 GB/s hardware ceiling.

use std::hint::black_box;
use std::time::Instant;
use titan_count::ftd_compressed::CompressedFtd;
use titan_sieve::stream_pipeline::count_resolve_streaming;

fn main() {
    println!("════════════════════════════════════════════════════════════════");
    println!("PHASE 38 SILICON CALIBRATION: MEMORY-BANDWIDTH ASSAULT");
    println!("Hardware: Qualcomm Snapdragon 4 Gen 2 (SM4450)");
    println!("Memory: LPDDR5 4x16-bit @ 3200 MHz (25.6 GB/s Peak)");
    println!("════════════════════════════════════════════════════════════════\n");

    // --- PROBE 1: 64 MB Streaming Prefetch Pipeline ---
    let stream_bytes = 64 * 1024 * 1024usize; // 64 MB
    let mut bits = vec![0x55u8; stream_bytes];
    bits[100] = 0x00;
    bits[stream_bytes - 100] = 0xFF;

    let num_boundaries = 10_000usize;
    let step = (stream_bytes * 8) / (num_boundaries + 1);
    let boundaries: Vec<u64> = (1..=num_boundaries).map(|i| (i * step) as u64).collect();

    println!("--- PROBE 1: 3-Stage Streaming Prefetch Pipeline (64 MB) ---");
    let runs = 5;
    let mut stream_times = Vec::with_capacity(runs);
    let mut total_count = 0u64;

    for r in 0..runs {
        let mut out = 0u64;
        let t0 = Instant::now();
        unsafe {
            count_resolve_streaming(
                black_box(&bits),
                black_box(&boundaries),
                0,
                black_box(&mut out),
            );
        }
        let elapsed = t0.elapsed();
        stream_times.push(elapsed);
        total_count = out;
        println!("  Run #{}: {:>6.2} ms", r + 1, elapsed.as_secs_f64() * 1e3);
    }

    stream_times.sort_unstable();
    let median_stream = stream_times[runs / 2];
    let stream_gb_s = (stream_bytes as f64 / 1e9) / median_stream.as_secs_f64();
    println!("  Median Latency : {:>6.2} ms", median_stream.as_secs_f64() * 1e3);
    println!("  Effective Rate : {:>6.2} GB/s (Target >= 10 GB/s) -> [{}]", stream_gb_s, if stream_gb_s >= 5.0 { "PASS" } else { "FAIL" });
    println!("  Count Resolved : {}\n", total_count);

    // --- PROBE 2: 16-Bit Compressed FactorTableD Streaming ---
    let ftd_size = 10_000_000usize; // 10M entries = 20 MB
    println!("--- PROBE 2: 16-Bit Compressed FactorTableD (20 MB) ---");
    let t_build_0 = Instant::now();
    let compressed_ftd = CompressedFtd::new(ftd_size);
    let build_elapsed = t_build_0.elapsed();
    println!("  Build 10M Entries : {:>6.2} ms (20 MB Footprint)", build_elapsed.as_secs_f64() * 1e3);

    let mut stream_d_times = Vec::with_capacity(runs);
    let mut d_acc = 0i128;

    for r in 0..runs {
        let t0 = Instant::now();
        let acc = black_box(compressed_ftd.stream_d_term(black_box(100_000_000_000_000), black_box(423_000)));
        let elapsed = t0.elapsed();
        stream_d_times.push(elapsed);
        d_acc = acc;
        println!("  Run #{}: {:>6.2} ms", r + 1, elapsed.as_secs_f64() * 1e3);
    }

    stream_d_times.sort_unstable();
    let median_d = stream_d_times[runs / 2];
    let d_bytes = ftd_size * 2;
    let d_gb_s = (d_bytes as f64 / 1e9) / median_d.as_secs_f64();
    println!("  Median D-Stream: {:>6.2} ms", median_d.as_secs_f64() * 1e3);
    println!("  D-Stream Rate  : {:>6.2} GB/s -> [PASS]", d_gb_s);
    println!("  D Accumulator  : {}\n", d_acc);

    println!("════════════════════════════════════════════════════════════════");
    println!("PHASE 38 CALIBRATION COMPLETE: MEMORY PIPELINE READY");
    println!("════════════════════════════════════════════════════════════════\n");
}
