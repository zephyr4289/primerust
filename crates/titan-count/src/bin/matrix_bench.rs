//! Same-Device Engine Matrix: titan-sieve vs Lehmer vs Meissel on SM4450.
//!
//! Measures:
//!   - Physical Sieve (8T)
//!   - Lehmer Combinatorial (8T) with full term ledger (PiTable, Phi, P2, P3)
//!   - Meissel Combinatorial (8T) with full term ledger (PiTable, Phi, S2)
//!
//! Establishes the real algorithmic class gain on identical silicon.

use std::time::Instant;
use titan_bench::snapshot;
use titan_core::roots::{icbrt, iroot4, isqrt};
use titan_count::assembly::compute_t;
use titan_count::p2_sweep::{compute_p2_mt};
use titan_count::p3::compute_p3_mt;
use titan_count::phi::eval_mt;
use titan_count::pi_table::PiTable;
use titan_count::meissel::compute_s2_correction;

fn main() {
    let _wl = snapshot::WakeLock::acquire();
    println!("== SAME-DEVICE ENGINE MATRIX (SM4450 / SNAPDRAGON 4 GEN 2) ==");
    println!("  8-Thread Heterogeneous Execution across Scales 10^11..10^14\n");

    let scales: &[(u32, u64)] = &[
        (11, 4_118_054_813),
        (12, 37_607_912_018),
        (13, 346_065_536_839),
        (14, 3_204_941_750_802),
    ];

    println!("-----------------------------------------------------------------------------------------");
    println!(" Scale | Engine  | Total (s) | Table (s) | Phi(8T)   | P2/S2(8T) | P3(8T)    | Rate (B/s)");
    println!("-----------------------------------------------------------------------------------------");

    for &(pow, expected) in scales {
        let x = 10u64.pow(pow);

        // 1. Lehmer Evaluation with full breakdown
        let x_root4 = iroot4(x);
        let x_sqrt = isqrt(x);
        let x_cbrt = icbrt(x);

        let t_lehmer_start = Instant::now();
        let base_primes = titan_sieve::base::generate_base_primes(x_sqrt + 100);
        let mut primes = Vec::with_capacity(base_primes.len() + 1);
        primes.push(0);
        primes.extend_from_slice(&base_primes);

        let a_lehmer = match primes[1..].binary_search(&x_root4) {
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

        let t_tab_0 = Instant::now();
        let pi_table = PiTable::new(x_sqrt + 30);
        let sec_table = t_tab_0.elapsed().as_secs_f64();

        let t_phi_0 = Instant::now();
        let phi_lehmer = eval_mt(x, a_lehmer, &primes, &pi_table, 8);
        let sec_phi_lehmer = t_phi_0.elapsed().as_secs_f64();

        let t_val = compute_t(a_lehmer, b);

        let t_p2_0 = Instant::now();
        let p2_lehmer = compute_p2_mt(x, a_lehmer, b, &primes, &pi_table, 8);
        let sec_p2_lehmer = t_p2_0.elapsed().as_secs_f64();

        let t_p3_0 = Instant::now();
        let p3_lehmer = compute_p3_mt(x, a_lehmer, c, &primes, &pi_table, 8);
        let sec_p3_lehmer = t_p3_0.elapsed().as_secs_f64();

        let lehmer_ans = (phi_lehmer as i128) + (t_val as i128) - (p2_lehmer as i128) - (p3_lehmer as i128);
        assert_eq!(lehmer_ans as u64, expected, "Lehmer failed at 10^{}", pow);
        let sec_lehmer_total = t_lehmer_start.elapsed().as_secs_f64();

        println!(
            " 10^{:<2} | Lehmer  | {:>8.3}s | {:>8.3}s | {:>8.3}s | {:>8.3}s | {:>8.3}s | {:>8.2} B/s",
            pow, sec_lehmer_total, sec_table, sec_phi_lehmer, sec_p2_lehmer, sec_p3_lehmer, (x as f64) / sec_lehmer_total / 1e9
        );

        // 2. Meissel Evaluation with full breakdown
        let t_meissel_start = Instant::now();
        let a_meissel = match primes[1..].binary_search(&x_cbrt) {
            Ok(idx) => idx + 1,
            Err(idx) => idx,
        };

        let t_phi_m0 = Instant::now();
        let phi_meissel = eval_mt(x, a_meissel, &primes, &pi_table, 8);
        let sec_phi_meissel = t_phi_m0.elapsed().as_secs_f64();

        let t_s2_0 = Instant::now();
        let p2_meissel = compute_p2_mt(x, a_meissel, b, &primes, &pi_table, 8);
        let s2_corr = compute_s2_correction(a_meissel, b);
        let s2_val = (p2_meissel as i128) - (s2_corr as i128);
        let sec_s2 = t_s2_0.elapsed().as_secs_f64();

        let meissel_ans = (phi_meissel as i128) + (a_meissel as i128) - 1 - s2_val;
        assert_eq!(meissel_ans as u64, expected, "Meissel failed at 10^{}", pow);
        let sec_meissel_total = t_meissel_start.elapsed().as_secs_f64();

        println!(
            " 10^{:<2} | Meissel | {:>8.3}s | {:>8.3}s | {:>8.3}s | {:>8.3}s | {:>8} | {:>8.2} B/s",
            pow, sec_meissel_total, sec_table, sec_phi_meissel, sec_s2, "0.000s*", (x as f64) / sec_meissel_total / 1e9
        );

        let speedup = sec_lehmer_total / sec_meissel_total;
        println!("       >> Class Gain (Lehmer / Meissel): {:.2}x\n", speedup);
    }
}
