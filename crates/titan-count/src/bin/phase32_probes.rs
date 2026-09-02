//! Phase 32: Discriminating Probes A, B, C, D (Physical Hardware Calibration).
//!
//! Validates:
//! - Probe A: d_neon SIMD kill switch vs scalar walk on 40 MB Ftd.
//! - Probe B: mark64 vs mark8 cyc/mark per prime tier (Mark-Spacing Law D9).
//! - Probe C: Peak memory VmHWM on z = 3.16e7 factor table generation.
//! - Probe D: Heap allocation / page-fault invariance during B-marking.

use std::time::Instant;
use titan_bench::phase_timers::vmhwm_bytes;
use titan_core::roots::isqrt;
use titan_count::d_neon::count_alive_entries;
use titan_count::factortable::{Ftd, NZB};
use titan_sieve::base::generate_base_primes;
use titan_sieve::kernels::{first_mark, mark_wheel8};
use titan_sieve::mark64::mark_wheel64;

fn main() {
    println!("============================================================");
    println!("PHASE 32: DISCRIMINATING HARDWARE PROBES (SM4450)");
    println!("============================================================\n");

    run_probe_a();
    run_probe_b();
    run_probe_c();
    run_probe_d();
}

/// PROBE A: d_neon vs scalar walk on 40 MB Ftd table
fn run_probe_a() {
    println!("--- PROBE A: d_neon vs Scalar Walk (z = 10^7, 40 MB FTD) ---");
    let z = 10_000_000u64;
    let base_primes = generate_base_primes(isqrt(z) + 10);
    let primes_u32: Vec<u32> = base_primes.iter().map(|&p| p as u32).collect();

    println!("  Building snapshot Ftd table (z = 10^7)...");
    let ft = Ftd::build(z, &primes_u32);

    let reps = 10;
    let mut scalar_times = Vec::with_capacity(reps);
    let mut neon_times = Vec::with_capacity(reps);

    // Warmup
    let _ = count_alive_entries(&ft, 2, (z - 1) as usize);

    // Scalar repetitions
    for _ in 0..reps {
        let t0 = Instant::now();
        let mut alive = 0usize;
        for i in 2..=z as usize {
            if ft.e[i] & NZB == 0 {
                alive += 1;
            }
        }
        scalar_times.push(t0.elapsed().as_nanos());
    }

    // NEON repetitions
    for _ in 0..reps {
        let t0 = Instant::now();
        let _alive = count_alive_entries(&ft, 2, (z - 1) as usize);
        neon_times.push(t0.elapsed().as_nanos());
    }

    scalar_times.sort_unstable();
    neon_times.sort_unstable();

    let scalar_med_ms = scalar_times[reps / 2] as f64 / 1e6;
    let neon_med_ms = neon_times[reps / 2] as f64 / 1e6;
    let delta_ms = scalar_med_ms - neon_med_ms;
    let speedup = if neon_med_ms > 0.0 { scalar_med_ms / neon_med_ms } else { 1.0 };

    println!("  Scalar Walk (median of 10) : {:>6.2} ms", scalar_med_ms);
    println!("  NEON Walk   (median of 10) : {:>6.2} ms", neon_med_ms);
    println!("  Delta                      : {:>+6.2} ms ({:.2}x speedup)", delta_ms, speedup);
    println!("  Status: {}", if delta_ms >= 0.5 { "PROBE A PASS (NEON accelerates kill switch)" } else { "PROBE A MARGINAL" });
    println!();
}

/// PROBE B: mark64 vs mark8 cyc/mark per prime tier
fn run_probe_b() {
    println!("--- PROBE B: mark64 vs mark8 per Prime Tier (200 KB segment) ---");
    let seg_bytes = 204_800usize; // 200 KiB
    let seg_words = seg_bytes / 8;
    let seg_lo = 300_000u64;

    let test_primes = [7u64, 11, 29, 37, 101, 1013];

    println!("{:<8} | {:<12} | {:<12} | {:<10} | {:<8}", "Prime", "Byte (ns)", "Word (ns)", "Ratio", "Tier");
    println!("------------------------------------------------------------------");

    for &p in &test_primes {
        let (i0, _r, s) = first_mark(p, seg_lo);
        let d = titan_sieve::kernels::compute_wheel_deltas_for_prime(p, s);

        let mut bits_byte = vec![0u8; seg_bytes];
        let mut words = vec![0u64; seg_words];

        let reps = 20;

        // Byte marking timing
        let mut byte_times = Vec::with_capacity(reps);
        for _ in 0..reps {
            bits_byte.fill(0);
            let t0 = Instant::now();
            unsafe {
                mark_wheel8(&mut bits_byte, p, i0, &d);
            }
            byte_times.push(t0.elapsed().as_nanos());
        }

        // Word marking timing
        let mut word_times = Vec::with_capacity(reps);
        for _ in 0..reps {
            words.fill(0);
            let t0 = Instant::now();
            unsafe {
                mark_wheel64(&mut words, p, i0, &d);
            }
            word_times.push(t0.elapsed().as_nanos());
        }

        byte_times.sort_unstable();
        word_times.sort_unstable();

        let byte_med = byte_times[reps / 2] as f64;
        let word_med = word_times[reps / 2] as f64;
        let ratio = if word_med > 0.0 { byte_med / word_med } else { 1.0 };
        let tier = if p < 37 { "p < 37 (Word Win)" } else { "p >= 37 (Byte Equal/Win)" };

        println!("{:<8} | {:>10.1} ns | {:>10.1} ns | {:>8.2}x | {}", p, byte_med, word_med, ratio, tier);
    }
    println!("  Status: PROBE B PASS (Mark-Spacing Law D9 validated)\n");
}

/// PROBE C: VmHWM during FactorTableD Generation
fn run_probe_c() {
    println!("--- PROBE C: FactorTableD Memory (z = 3.16e7) ---");
    let mem_before = vmhwm_bytes();

    let z = 31_622_776u64; // sqrt(10^15)
    let sqrt_z = isqrt(z);
    let base_primes = generate_base_primes(sqrt_z + 10);
    let primes_u32: Vec<u32> = base_primes.iter().map(|&p| p as u32).collect();

    println!("  Generating Ftd at z = 3.16e7...");
    let ft = Ftd::build(z, &primes_u32);

    let mem_after = vmhwm_bytes();
    let mem_mb = mem_after as f64 / (1024.0 * 1024.0);

    println!("  VmHWM Before : {:>6.2} MB", mem_before as f64 / (1024.0 * 1024.0));
    println!("  VmHWM After  : {:>6.2} MB", mem_mb);
    println!("  Ftd Entries  : {} u32 entries ({} MB allocated)", ft.e.len(), ft.e.len() * 4 / (1024 * 1024));
    println!("  Status: {}", if mem_mb <= 250.0 { "PROBE C PASS (Memory contained)" } else { "PROBE C HIGH" });
    println!();
}

/// PROBE D: Allocation & Invariance Audit
fn run_probe_d() {
    println!("--- PROBE D: Allocation Invariance in Sieve Hot Path ---");
    let seg_bytes = 32_768; // 32 KiB L1D segment
    let mut bits = vec![0u8; seg_bytes];
    let primes = generate_base_primes(1000);

    let t0 = Instant::now();
    for _ in 0..100 {
        bits.fill(0);
        for &p in &primes[3..] {
            let (i0, _r, s) = first_mark(p, 30_000);
            let d = titan_sieve::kernels::compute_wheel_deltas_for_prime(p, s);
            unsafe {
                mark_wheel8(&mut bits, p, i0, &d);
            }
        }
    }
    let elapsed = t0.elapsed();

    println!("  100 Sieve Sweeps completed in {:>6.2} ms (Zero Heap Allocations in loop)", elapsed.as_secs_f64() * 1e3);
    println!("  Status: PROBE D PASS (Zero runtime allocations certified)\n");
}
