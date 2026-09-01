//! Alpha-Sweep: Evaluates cell reduction and wall time across alpha in [1.0, 3.0]
//!
//! Measures cells(alpha), sweep-span(alpha), and total wall time at 10^13 and 10^14.

use std::time::Instant;
use titan_core::roots::{icbrt, iroot4, isqrt};
use titan_count::mertens_struct::MertensStructure;
use titan_count::pi_table::PiTable;

fn run_alpha_benchmark(x: u64, alpha: f64) -> (usize, usize, f64, f64, f64) {
    let x_cbrt = icbrt(x);
    let x_sqrt = isqrt(x);
    let x_root4 = iroot4(x);

    let y = ((x_cbrt as f64) * alpha).round() as u64;

    let base_primes = titan_sieve::base::generate_base_primes((x_sqrt + 100).max(y + 100));
    let mut primes = Vec::with_capacity(base_primes.len() + 1);
    primes.push(0);
    primes.extend_from_slice(&base_primes);

    let a = match primes[1..].binary_search(&y) {
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

    let pi_table = PiTable::new(x_sqrt + 30);
    let mertens = MertensStructure::new(x_sqrt as usize + 100);

    let t0_w = Instant::now();
    let _ = titan_count::interval_walker::IntervalWalker::walk_intervals_mt(
        x, c, a, &primes, &pi_table, &mertens, 8,
    );
    let sec_w = t0_w.elapsed().as_secs_f64();

    let t0_s2 = Instant::now();
    let _ = titan_count::p2_sweep::compute_p2_mt(x, a, b, &primes, &pi_table, 8);
    let sec_s2 = t0_s2.elapsed().as_secs_f64();

    let total_sec = sec_w + sec_s2;
    let sweep_span = if b >= a { b - a } else { 0 };

    (a - c, sweep_span, sec_w, sec_s2, total_sec)
}

fn main() {
    println!("=========================================================================================");
    println!("                  ALPHA-SWEEP: CELL REDUCTION & WALL-TIME DIAL                          ");
    println!("=========================================================================================");
    println!(" Scale |  alpha  | Levels (a - c) | Sweep Primes | Walker MT (s) | S2 Sweep (s) | Total (s)");
    println!("-----------------------------------------------------------------------------------------");

    let alphas = [1.0, 1.5, 2.0, 2.5, 3.0];
    let scales = [(13, 10_000_000_000_000u64), (14, 100_000_000_000_000u64)];

    for &(pow, x) in &scales {
        for &alpha in &alphas {
            let (levels, sweep_primes, sec_w, sec_s2, total) = run_alpha_benchmark(x, alpha);
            println!(
                " 10^{:<2} |  {:<5.1}  | {:>14} | {:>12} | {:>12.4}s | {:>11.4}s | {:>8.4}s",
                pow, alpha, levels, sweep_primes, sec_w, sec_s2, total
            );
        }
        println!("-----------------------------------------------------------------------------------------");
    }
    println!("=========================================================================================");
}
