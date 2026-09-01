//! Pre-Flight Experiment C10: Leaf Census via the Lehmer Tree.
//!
//! Analyzes T2 leaf distribution, factor counts omega(d), y = floor(x/d) values,
//! and verifies the Architecture Theorem (all multi-factor leaves satisfy y <= sqrt(x)).

use std::time::Instant;
use titan_bench::snapshot;
use titan_core::roots::{icbrt, iroot4, isqrt};
use titan_count::magic::MagicDivTable;
use titan_count::pi_table::PiTable;

#[derive(Default, Debug)]
struct LeafCensus {
    total_t2_leaves: u64,
    total_t1_leaves: u64,
    single_factor_leaves: u64,
    multi_factor_leaves: u64,
    multi_factor_exceeding_sqrt: u64,
    max_y_multi_factor: u64,
    factor_histogram: [u64; 16],
    y_le_sqrt: u64,
    y_gt_sqrt: u64,
}

fn run_leaf_census(x: u64, a: usize, primes: &[u64], pi_table: &PiTable) -> LeafCensus {
    let magic = MagicDivTable::new(primes);
    let x_sqrt = isqrt(x);

    let mut census = LeafCensus::default();
    // Stack entry: (y, i, sign, omega)
    let mut stack: Vec<(u64, usize, i8, usize)> = Vec::with_capacity(4096);
    stack.push((x, a, 1, 0));

    while let Some((y, i, _sign, omega)) = stack.pop() {
        if i <= 6 {
            census.total_t1_leaves += 1;
            continue;
        }

        let next_p = primes[i + 1];
        if y < next_p * next_p {
            census.total_t2_leaves += 1;
            let om = omega.min(15);
            census.factor_histogram[om] += 1;

            if omega == 1 {
                census.single_factor_leaves += 1;
            } else if omega > 1 {
                census.multi_factor_leaves += 1;
                if y > census.max_y_multi_factor {
                    census.max_y_multi_factor = y;
                }
                if y > x_sqrt {
                    census.multi_factor_exceeding_sqrt += 1;
                }
            }

            if y <= x_sqrt {
                census.y_le_sqrt += 1;
            } else {
                census.y_gt_sqrt += 1;
            }
            continue;
        }

        // Left child: (y, i - 1, sign, omega)
        stack.push((y, i - 1, 1, omega));

        // Right child: (y / p_i, i - 1, -sign, omega + 1)
        let y_div = magic.div(y, i);
        stack.push((y_div, i - 1, -1, omega + 1));
    }

    census
}

fn main() {
    let _wl = snapshot::WakeLock::acquire();
    println!("== PRE-FLIGHT EXPERIMENT C10: LEAF CENSUS (LMO FOUNDATIONS) ==");

    for &pow in &[10, 11, 12, 13] {
        let x = 10u64.pow(pow);
        let x_root4 = iroot4(x);
        let x_cbrt = icbrt(x);
        let x_sqrt = isqrt(x);

        let base_primes = titan_sieve::base::generate_base_primes(x_sqrt + 100);
        let mut primes = Vec::with_capacity(base_primes.len() + 1);
        primes.push(0);
        primes.extend_from_slice(&base_primes);

        let a_root4 = match primes[1..].binary_search(&x_root4) {
            Ok(idx) => idx + 1,
            Err(idx) => idx,
        };
        let a_cbrt = match primes[1..].binary_search(&x_cbrt) {
            Ok(idx) => idx + 1,
            Err(idx) => idx,
        };

        let pi_table = PiTable::new(x_sqrt + 30);

        println!("\n============================================================");
        println!("  Scale 10^{} (x = {}, sqrt(x) = {})", pow, x, x_sqrt);
        println!("============================================================");

        for &(name, a_val) in &[("Lehmer (a = pi(x^1/4))", a_root4), ("Meissel (a = pi(x^1/3))", a_cbrt)] {
            let t0 = Instant::now();
            let c = run_leaf_census(x, a_val, &primes, &pi_table);
            let elapsed = t0.elapsed().as_secs_f64();

            println!("  [{}] a = {} (p_a = {}) | Elapsed: {:.3}s", name, a_val, primes[a_val], elapsed);
            println!("    Total T2 Leaves               : {:>10}", c.total_t2_leaves);
            println!("    Total T1 (PhiTiny) Leaves      : {:>10}", c.total_t1_leaves);
            println!("    Single-Factor Leaves (omega=1) : {:>10}", c.single_factor_leaves);
            println!("    Multi-Factor Leaves (omega>=2) : {:>10}", c.multi_factor_leaves);
            println!("    Max y in Multi-Factor Leaves   : {:>10} (sqrt(x) = {})", c.max_y_multi_factor, x_sqrt);
            println!("    Multi-Factor with y > sqrt(x)  : {:>10} {}", c.multi_factor_exceeding_sqrt, if c.multi_factor_exceeding_sqrt == 0 { "[THEOREM 2.3 HOLDS]" } else { "[VIOLATION!]" });
            println!("    Leaves with y <= sqrt(x)       : {:>10} ({:.2}%)", c.y_le_sqrt, (c.y_le_sqrt as f64) / (c.total_t2_leaves.max(1) as f64) * 100.0);
            println!("    Leaves with y > sqrt(x)        : {:>10} ({:.2}%)", c.y_gt_sqrt, (c.y_gt_sqrt as f64) / (c.total_t2_leaves.max(1) as f64) * 100.0);

            print!("    Factor count distribution      : [");
            for om in 1..=6 {
                print!("omega={}: {}{}", om, c.factor_histogram[om], if om < 6 { ", " } else { "" });
            }
            println!("]");
        }
    }
}
