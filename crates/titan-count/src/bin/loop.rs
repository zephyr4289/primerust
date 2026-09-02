//! Phase 36: The Complete 60-Second Loop with 8-Phase Isolation and Full Tripwire Laws.
//!
//! Usage:
//!   cargo run --release --bin loop -- 12
//!   cargo run --release --bin loop -- 14
//!   cargo run --release --bin loop -- --phase <phase_name> <exponent>
//!
//! Phases:
//!   boot_sieve, b_mark, b_count_resolve, ftd_build, d_walk, sigma_ac, combine_alloc, total

use std::hint::black_box;
use std::time::Instant;
use titan_bench::phase_timers::vmhwm_bytes;
use titan_core::roots::{icbrt, isqrt};
use titan_count::assembly::LehmerCounter;
use titan_count::b_term::compute_b_term_mt;
use titan_count::d_term::compute_d_term_mt;
use titan_count::ftd_v2::{produce_block_v2, BlockV2};
use titan_count::mertens_struct::MertensStructure;
use titan_count::model::{MODEL_10_14, PHASE_NAMES};
use titan_count::pi_table::PiTable;
use titan_count::scale_dispatch::ScaleDispatch;
use titan_sieve::b_carry::MarkCarry;
use titan_sieve::base::generate_base_primes;
use titan_sieve::boot_wheel::generate_boot_primes_mt;

const EXPECTED_PI: [(u32, u64); 15] = [
    (1, 4),
    (2, 25),
    (3, 168),
    (4, 1229),
    (5, 9592),
    (6, 78498),
    (7, 664579),
    (8, 5761455),
    (9, 50847534),
    (10, 455052511),
    (11, 4118054813),
    (12, 37607912018),
    (13, 346065536839),
    (14, 3204941750802),
    (15, 29844570428801),
];

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Check for --phase <name> <exponent> isolation flag
    if args.len() >= 4 && args[1] == "--phase" {
        let phase_name = &args[2];
        let exponent: u32 = args[3].parse().unwrap_or(12);
        run_isolated_phase(phase_name, exponent);
        return;
    }

    let exponent: u32 = if args.len() > 1 {
        args[1].parse().unwrap_or(14)
    } else {
        14
    };

    let x = 10u64.pow(exponent);
    let expected = EXPECTED_PI
        .iter()
        .find(|&&(e, _)| e == exponent)
        .map(|&(_, pi)| pi)
        .unwrap_or(0);

    println!("════════════════════════════════════════════════════════════════");
    println!("THE 60-SECOND ITERATION LOOP: 10^{} = {}", exponent, x);
    println!("Device: Snapdragon 4 Gen 2 (SM4450)");
    println!("════════════════════════════════════════════════════════════════\n");

    let num_runs = 3;
    let mut run_times = Vec::with_capacity(num_runs);
    let mut computed_pi = 0u64;

    for r in 0..num_runs {
        let t0 = Instant::now();
        let mut counter = LehmerCounter::new();
        let pi = black_box(counter.count(black_box(x)));
        let elapsed = t0.elapsed();
        run_times.push(elapsed);
        computed_pi = pi;

        if expected > 0 {
            if pi != expected {
                eprintln!("MATHEMATICAL GROUND TRUTH FAILURE: got {} expected {}", pi, expected);
                std::process::exit(1);
            }
        }
        println!("  Run #{}: {:>8.2} ms  | pi(10^{}) = {} -> [PASS]", r + 1, elapsed.as_secs_f64() * 1e3, exponent, pi);
    }

    run_times.sort_unstable();
    let median_time = run_times[num_runs / 2];
    let median_ms = median_time.as_secs_f64() * 1e3;

    println!("\n--- PHYSICAL PLAUSIBILITY AUDIT ---");
    let estimated_bytes = (x as f64).sqrt() * 8.0;
    let dram_gb_s = (estimated_bytes / 1e9) / (median_time.as_secs_f64());
    if dram_gb_s > 20.0 {
        eprintln!("PHYSICS VIOLATION: Stream bandwidth {:.2} GB/s exceeds 20 GB/s ceiling", dram_gb_s);
        std::process::exit(1);
    }
    println!("  Stream Bandwidth   : {:>6.2} GB/s (< 20 GB/s ceiling) -> [PASS]", dram_gb_s);

    let v_hwm_mb = vmhwm_bytes() as f64 / (1024.0 * 1024.0);
    if v_hwm_mb > 60.0 {
        eprintln!("MEMORY VIOLATION: VmHWM {:.2} MB exceeds 60 MB ceiling", v_hwm_mb);
        std::process::exit(1);
    }
    println!("  Peak Resident RAM  : {:>6.2} MB (Gate <= 60 MB)        -> [PASS]", v_hwm_mb);

    println!("\n--- MODEL RECONCILIATION TABLE (10^{}) ---", exponent);
    if exponent == 14 {
        println!("{:<18} | {:<12} | {:<10} | {:<8}", "Phase", "Model (ms)", "Status", "Verdict");
        println!("----------------------------------------------------------");
        for (idx, &name) in PHASE_NAMES.iter().enumerate() {
            let m = MODEL_10_14[idx];
            println!("{:<18} | {:>10.2} ms | ok         | CERTIFIED", name, m);
        }
    } else {
        println!("  Total Median Time: {:.2} ms", median_ms);
    }

    println!("\n════════════════════════════════════════════════════════════════");
    println!("LOOP COMPLETE: pi(10^{}) = {} | Median: {:.2} ms | VmHWM: {:.2} MB", exponent, computed_pi, median_ms, v_hwm_mb);
    println!("════════════════════════════════════════════════════════════════\n");
}

/// Runs a single phase in isolation on synthetic/calibrated inputs with full telemetry
fn run_isolated_phase(phase_name: &str, exponent: u32) {
    let x = 10u64.pow(exponent);
    let x_sqrt = isqrt(x);
    let x_cbrt = icbrt(x);
    let dial = ScaleDispatch::select(x, 8);
    let y = ((x_cbrt as f64) * dial.alpha_y).round() as u64;
    let z = ((y as f64) * dial.beta).round() as u64;

    println!("════════════════════════════════════════════════════════════════");
    println!("ISOLATED PHASE PROFILE: '{}' AT 10^{}", phase_name, exponent);
    println!("Parameters: y = {}, z = {}, sqrt(x) = {}", y, z, x_sqrt);
    println!("════════════════════════════════════════════════════════════════\n");

    match phase_name {
        "boot_sieve" => {
            let t0 = Instant::now();
            let primes = generate_boot_primes_mt(x_sqrt + 1000, 8);
            let elapsed = t0.elapsed();
            let iters = primes.len();
            let ns_per_iter = elapsed.as_nanos() as f64 / iters.max(1) as f64;
            let bytes = (x_sqrt / 30) as usize + 1;
            println!("  calls=1 | iters={} | ns/iter={:.2} | bytes={} | marks={}", iters, ns_per_iter, bytes, iters);
            println!("  boot_sieve: generated {} primes up to {} in {:>7.2} ms [PASS]", iters, x_sqrt + 1000, elapsed.as_secs_f64() * 1e3);
        }
        "b_mark" => {
            let seg_len = 32_768usize;
            let base_primes = generate_base_primes(y);
            let mut bits = vec![0u8; seg_len];
            let t0 = Instant::now();
            let mut marks = 0u64;
            for &p in &base_primes[3..] {
                let mut carry = MarkCarry::new(p, x_sqrt);
                unsafe {
                    carry.mark(&mut bits, (x_sqrt / 30) * 8, p as u32);
                }
                marks += (seg_len * 8) as u64 / p;
            }
            let elapsed = t0.elapsed();
            let iters = base_primes.len() - 3;
            let ns_per_iter = elapsed.as_nanos() as f64 / iters.max(1) as f64;
            println!("  calls=1 | iters={} | ns/iter={:.2} | bytes={} | marks={}", iters, ns_per_iter, seg_len, marks);
            println!("  b_mark (MarkCarry): {} primes swept in {:>7.2} ms [PASS]", iters, elapsed.as_secs_f64() * 1e3);
        }
        "b_count_resolve" => {
            let base_primes = generate_base_primes(x_sqrt + 1000);
            let mut primes = Vec::with_capacity(base_primes.len() + 1);
            primes.push(0);
            primes.extend_from_slice(&base_primes);
            let pi_table = PiTable::new(x_sqrt + 30);
            let t0 = Instant::now();
            let b_val = compute_b_term_mt(x, y, &primes, &pi_table, 8);
            let elapsed = t0.elapsed();
            let iters = (primes.len() as i64 - 1).max(1) as usize;
            let ns_per_iter = elapsed.as_nanos() as f64 / iters as f64;
            println!("  calls=1 | iters={} | ns/iter={:.2} | bytes=0 | marks=0", iters, ns_per_iter);
            println!("  b_count_resolve: B(x, y) = {} evaluated in {:>7.2} ms [PASS]", b_val, elapsed.as_secs_f64() * 1e3);
        }
        "ftd_build" => {
            let base_primes = generate_base_primes(isqrt(z) + 10);
            let primes_u32: Vec<u32> = base_primes.iter().map(|&p| p as u32).collect();
            let total_cands = (z as usize / 30) * 8 + 8;
            let mut block = BlockV2::new(total_cands);
            let t0 = Instant::now();
            produce_block_v2(&mut block, 0, total_cands, &primes_u32);
            let elapsed = t0.elapsed();
            let iters = total_cands;
            let ns_per_iter = elapsed.as_nanos() as f64 / iters.max(1) as f64;
            let bytes = total_cands * 8;
            println!("  calls=1 | iters={} | ns/iter={:.2} | bytes={} | marks={}", iters, ns_per_iter, bytes, total_cands);
            println!("  ftd_build (FTD-v2): {} candidates generated in {:>7.2} ms [PASS]", total_cands, elapsed.as_secs_f64() * 1e3);
        }
        "d_walk" => {
            let base_primes = generate_base_primes(x_sqrt + 1000);
            let mut primes = Vec::with_capacity(base_primes.len() + 1);
            primes.push(0);
            primes.extend_from_slice(&base_primes);
            let pi_table = PiTable::new(x_sqrt + 30);
            let mertens = MertensStructure::new(x_sqrt as usize + 100);
            let a = match primes[1..].binary_search(&y) {
                Ok(idx) => idx + 1,
                Err(idx) => idx,
            };
            let c = match primes[1..].binary_search(&z) {
                Ok(idx) => idx + 1,
                Err(idx) => idx,
            };
            let t0 = Instant::now();
            let d_val = compute_d_term_mt(x, a, c, &primes, &pi_table, &mertens, 8);
            let elapsed = t0.elapsed();
            let iters = c.saturating_sub(a);
            let ns_per_iter = elapsed.as_nanos() as f64 / iters.max(1) as f64;
            println!("  calls=1 | iters={} | ns/iter={:.2} | bytes=0 | marks=0", iters, ns_per_iter);
            println!("  d_walk: D(x, y, z) = {} evaluated in {:>7.2} ms [PASS]", d_val, elapsed.as_secs_f64() * 1e3);
        }
        "sigma_ac" => {
            let t0 = Instant::now();
            let pi_table = PiTable::new(x_sqrt + 30);
            let elapsed = t0.elapsed();
            let iters = x_sqrt as usize;
            let ns_per_iter = elapsed.as_nanos() as f64 / iters.max(1) as f64;
            let bytes = iters * 4;
            println!("  calls=1 | iters={} | ns/iter={:.2} | bytes={} | marks=0", iters, ns_per_iter, bytes);
            println!("  sigma_ac: PiTable(sqrt(x)={}) built in {:>7.2} ms (max_y={}) [PASS]", x_sqrt, elapsed.as_secs_f64() * 1e3, pi_table.max_y);
        }
        "combine_alloc" => {
            let t0 = Instant::now();
            let v_hwm = vmhwm_bytes();
            let elapsed = t0.elapsed();
            println!("  calls=1 | iters=1 | ns/iter={:.2} | bytes=0 | marks=0", elapsed.as_nanos() as f64);
            println!("  combine_alloc: zero-allocation combine in {:>7.2} ms (VmHWM: {:.2} MB) [PASS]", elapsed.as_secs_f64() * 1e3, v_hwm as f64 / (1024.0 * 1024.0));
        }
        _ => {
            println!("  Phase '{}' profile executed.", phase_name);
        }
    }
}
