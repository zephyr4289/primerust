//! Term Breakdown & Per-Phase Diagnostic Binary (Phase 1.23 Deliverable).
//!
//! Measures exact millisecond-level per-phase timing breakdown at 10^14:
//!   - Base Primes generation
//!   - PiTable construction
//!   - MertensStructure initialization
//!   - Phi0 calculation (Phi(x, 6))
//!   - S2 / P2 physical sweep
//!   - D Special-Leaf Walker MT
//!   - Final Assembly

use std::time::Instant;
use titan_core::phi_tiny::phi_tiny;
use titan_core::roots::{icbrt, iroot4, isqrt};
use titan_count::mertens_struct::MertensStructure;
use titan_count::p2_sweep::compute_p2_mt;
use titan_count::pi_table::PiTable;

fn main() {
    println!("=========================================================================================");
    println!("          PHASE 1.23: 10^14 PER-PHASE PROFILING & TIMING BREAKDOWN (8 THREADS)           ");
    println!("=========================================================================================");

    let x = 100_000_000_000_000u64; // 10^14
    let x_cbrt = icbrt(x);
    let x_sqrt = isqrt(x);
    let x_root4 = iroot4(x);

    let t_total_start = Instant::now();

    // Phase 1: Base primes
    let t0 = Instant::now();
    let base_primes = titan_sieve::base::generate_base_primes(x_sqrt + 100);
    let mut primes = Vec::with_capacity(base_primes.len() + 1);
    primes.push(0);
    primes.extend_from_slice(&base_primes);
    let t_primes = t0.elapsed().as_secs_f64();

    let a = match primes[1..].binary_search(&x_cbrt) {
        Ok(idx) => idx + 1,
        Err(idx) => idx,
    };
    let b = match primes[1..].binary_search(&x_sqrt) {
        Ok(idx) => idx + 1,
        Err(idx) => idx,
    };
    let c = match primes[1..].binary_search(&x_root4) {
        Ok(idx) => idx + 1,
        Err(idx) => idx,
    };

    // Phase 2: PiTable
    let t0 = Instant::now();
    let pi_table = PiTable::new(x_sqrt + 30);
    let t_pi_table = t0.elapsed().as_secs_f64();

    // Phase 3: Mertens Structure
    let t0 = Instant::now();
    let mertens = MertensStructure::new(x_sqrt as usize + 100);
    let t_mertens = t0.elapsed().as_secs_f64();

    // Phase 4: Phi_c (Phi(x, c) where c = pi(x^1/4))
    let t0 = Instant::now();
    let phi_c = titan_count::phi::eval_mt(x, c, &primes, &pi_table, 8) as i64;
    let t_phic = t0.elapsed().as_secs_f64();

    // Phase 5: S2 / P2 Physical Range Sweep
    let t0 = Instant::now();
    let p2_val = compute_p2_mt(x, a, b, &primes, &pi_table, 8);
    let s2_corr = titan_count::meissel::compute_s2_correction(a, b);
    let s2_val = (p2_val as i128) - (s2_corr as i128);
    let t_s2 = t0.elapsed().as_secs_f64();

    // Phase 6: D Special-Leaf Interval Walker MT
    let t0 = Instant::now();
    let d_special = titan_count::interval_walker::IntervalWalker::walk_intervals_mt(
        x, c, a, &primes, &pi_table, &mertens, 8,
    );
    let t_walker = t0.elapsed().as_secs_f64();

    let total_elapsed = t_total_start.elapsed().as_secs_f64();

    // Assembly: Phi(x, a) = Phi_c(x, c) - D_special
    let phi_val = (phi_c as i128) - (d_special as i128);
    let count = phi_val + (a as i128) - 1 - s2_val;

    println!(" Input x                     : 10^14 (100,000,000,000,000)");
    println!(" Expected pi(10^14)          : 3,204,941,750,802");
    println!(" Computed pi(10^14)          : {}", count);
    println!(" Verification                : {}", if count == 3_204_941_750_802 { "BIT-EXACT MATCH (PASS)" } else { "FAIL" });
    println!("-----------------------------------------------------------------------------------------");
    println!(" Phase / Component           | Wall Time (s) | Fraction (%) | Optimization Vector");
    println!("-----------------------------------------------------------------------------------------");
    println!(" 1. Base Primes Generation   | {:>11.4}s  | {:>10.2}% | Wheel-30 Presieved", t_primes, (t_primes / total_elapsed) * 100.0);
    println!(" 2. PiTable Construction     | {:>11.4}s  | {:>10.2}% | 64-byte Blocks", t_pi_table, (t_pi_table / total_elapsed) * 100.0);
    println!(" 3. Mertens Prefix Structure | {:>11.4}s  | {:>10.2}% | Checkpointed M(u)", t_mertens, (t_mertens / total_elapsed) * 100.0);
    println!(" 4. Phi_c (Phi(x, x^1/4))    | {:>11.6}s  | {:>10.4}% | Fast MT Phi(x, c)", t_phic, (t_phic / total_elapsed) * 100.0);
    println!(" 5. S2 Range Sweep MT (P2)   | {:>11.4}s  | {:>10.2}% | Multi-Core Segment Sieve", t_s2, (t_s2 / total_elapsed) * 100.0);
    println!(" 6. D Special Leaves MT      | {:>11.4}s  | {:>10.2}% | Interval Substrate / K-Ladder", t_walker, (t_walker / total_elapsed) * 100.0);
    println!("-----------------------------------------------------------------------------------------");
    println!(" TOTAL END-TO-END RUNTIME    | {:>11.4}s  |    100.00% | (True Substrate Speed)", total_elapsed);
    println!("=========================================================================================");
}
