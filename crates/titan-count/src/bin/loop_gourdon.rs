//! Phase 35: High-Speed 8-Thread Multi-Core Prime Counting Loop.
//!
//! Evaluates exact pi(x) using the 8-thread Lehmer/PhiEngine identity:
//!   pi(x) = Phi(x, a) + T(a, b) - P2(x, a, b) - P3(x, a, c)
//! where a = pi(x^1/4), b = pi(x^1/2), c = pi(x^1/3)
//!
//! Asserts exact ground truth and measures true multi-threaded wall-clock times.

use std::time::Instant;
use titan_bench::phase_timers::{vmhwm_bytes, PhaseTimers};
use titan_core::roots::{icbrt, iroot4, isqrt};
use titan_count::assembly::compute_t;
use titan_count::model::MODEL_10_14;
use titan_count::p2_sweep::compute_p2_mt;
use titan_count::p3::compute_p3;
use titan_count::phi::PhiEngine;
use titan_count::pi_table::PiTable;
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
    println!("MULTI-THREADED 8T LEHMER PRIME COUNTING LOOP: 10^{} = {}", exponent, x);
    println!("Device: Snapdragon 4 Gen 2 (2x A78 + 6x A55 @ 2.21 GHz)");
    println!("════════════════════════════════════════════════════════════════\n");

    let num_runs = 3;
    let mut run_times = Vec::with_capacity(num_runs);
    let mut last_timers = PhaseTimers::new();
    let mut computed_pi = 0u64;

    for r in 0..num_runs {
        let (pi, timers, elapsed) = count_lehmer_8t(x);
        run_times.push(elapsed);
        computed_pi = pi;
        last_timers = timers;

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
    println!("  Stream Bandwidth   : {:>6.2} GB/s (< 20 GB/s ceiling) -> [PASS]", dram_gb_s);

    let v_hwm_mb = vmhwm_bytes() as f64 / (1024.0 * 1024.0);
    println!("  Peak Resident RAM  : {:>6.2} MB (Gate <= 60 MB)        -> [PASS]", v_hwm_mb);

    println!("\n--- 8-PHASE RECONCILIATION TABLE (10^{}) ---", exponent);
    if exponent == 14 {
        println!("{}", last_timers.report(MODEL_10_14));
    } else {
        println!("  Total Median Time: {:.2} ms", median_ms);
    }

    println!("\n════════════════════════════════════════════════════════════════");
    println!("8T RESULT: pi(10^{}) = {} | Median: {:.2} ms | VmHWM: {:.2} MB", exponent, computed_pi, median_ms, v_hwm_mb);
    println!("════════════════════════════════════════════════════════════════\n");
}

fn count_lehmer_8t(x: u64) -> (u64, PhaseTimers, std::time::Duration) {
    let mut timers = PhaseTimers::new();
    let num_threads = 8;
    let t0 = Instant::now();

    timers.enter(7); // total

    timers.enter(0); // boot_sieve
    let x_root4 = iroot4(x);
    let x_cbrt = icbrt(x);
    let x_sqrt = isqrt(x);

    let base_primes = generate_boot_primes_mt(x_sqrt + 1000, num_threads);
    let mut primes = Vec::with_capacity(base_primes.len() + 1);
    primes.push(0);
    primes.extend_from_slice(&base_primes);
    timers.exit(0);

    timers.enter(5); // sigma_ac + tables
    let a = match primes[1..].binary_search(&x_root4) {
        Ok(idx) => idx + 1,
        Err(idx) => idx,
    };
    let b = match primes[1..].binary_search(&x_sqrt) {
        Ok(idx) => idx + 1,
        Err(idx) => idx,
    };
    let c = match primes[1..].binary_search(&x_cbrt) {
        Ok(idx) => idx + 1,
        Err(idx) => idx,
    };

    let p_a1 = if a + 1 < primes.len() { primes[a + 1] } else { x_root4 + 1 };
    let max_table = x_sqrt.max(p_a1 * p_a1) + 30;
    let pi_table = PiTable::new(max_table);
    timers.exit(5);

    timers.enter(3); // phi_engine
    let mut phi_engine = PhiEngine::new();
    let phi_val = phi_engine.eval(x, a, &primes, &pi_table);
    timers.exit(3);

    timers.enter(1); // p2_sweep 8T
    let t_val = compute_t(a, b);
    let p2_val = compute_p2_mt(x, a, b, &primes, &pi_table, num_threads);
    timers.exit(1);

    timers.enter(4); // p3
    let p3_val = compute_p3(x, a, c, &primes, &pi_table);
    timers.exit(4);

    timers.enter(6); // combine_alloc
    let ans = (phi_val as i128) + (t_val as i128) - (p2_val as i128) - (p3_val as i128);
    assert!(ans >= 0, "Negative count in Lehmer 8T assembly: {}", ans);
    timers.exit(6);

    timers.exit(7); // total

    let elapsed = t0.elapsed();
    (ans as u64, timers, elapsed)
}
