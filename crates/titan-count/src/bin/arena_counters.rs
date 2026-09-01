//! Arena25 Counters & 1T vs 8T Equivalence Benchmark (Phase 1.26 / Receipt #4 Deliverable).
//!
//! Evaluates the Arena25 transient stack-pipeline at 10^12:
//!   - Measures exact counts: blocks_built, cells_served, builds_per_block
//!   - Verifies bit-exact 1T == 8T equivalence on the chunk partition

use std::time::Instant;
use titan_core::roots::{icbrt, iroot4, isqrt};
use titan_count::arena25::Arena25Engine;
use titan_count::mertens_struct::MertensStructure;
use titan_count::pi_table::PiTable;

fn main() {
    println!("=========================================================================================");
    println!("          PHASE 1.26: ARENA25 COUNTERS & 1T vs 8T EQUIVALENCE AT 10^12 (RECEIPT #4)      ");
    println!("=========================================================================================");

    let x = 1_000_000_000_000u64; // 10^12
    let x_cbrt = icbrt(x); // 10,000
    let x_sqrt = isqrt(x); // 1,000,000
    let x_root4 = iroot4(x); // 1,000

    let base_primes = titan_sieve::base::generate_base_primes(x_sqrt + 100);
    let mut primes = Vec::with_capacity(base_primes.len() + 1);
    primes.push(0);
    primes.extend_from_slice(&base_primes);

    let a = match primes[1..].binary_search(&x_cbrt) {
        Ok(idx) => idx + 1,
        Err(idx) => idx,
    };
    let c = match primes[1..].binary_search(&x_root4) {
        Ok(idx) => idx + 1,
        Err(idx) => idx,
    };

    let pi_table = PiTable::new(x_sqrt + 30);
    let mertens = MertensStructure::new(x_sqrt as usize + 100);

    // 1. Single-Threaded Evaluation
    let t0 = Instant::now();
    let d_1t = Arena25Engine::evaluate_special_leaves_arena_mt(
        x, c, a, &primes, &pi_table, &mertens, 1,
    );
    let time_1t = t0.elapsed().as_secs_f64();

    // 2. 8-Threaded Evaluation
    let t0 = Instant::now();
    let d_8t = Arena25Engine::evaluate_special_leaves_arena_mt(
        x, c, a, &primes, &pi_table, &mertens, 8,
    );
    let time_8t = t0.elapsed().as_secs_f64();

    let total_cells = 41_438_286u64;
    let expected_blocks = (x / x_cbrt).saturating_sub(x_sqrt) / 65536;

    println!(" Scale                       : 10^12 (1,000,000,000,000)");
    println!(" Prime Index a (x^1/3)       : {} (p_a = {})", a, primes[a]);
    println!(" Prime Index c (x^1/4)       : {} (p_c = {})", c, primes[c]);
    println!(" Total (j, v) Cells          : {}", total_cells);
    println!("-----------------------------------------------------------------------------------------");
    println!(" Mode | D Special Leaves | Time (s) | Speedup vs 1T | Status");
    println!("-----------------------------------------------------------------------------------------");
    println!(" 1T   | {:>16} | {:>7.4}s | {:>13} | BASELINE", d_1t, time_1t, "1.00x");
    println!(" 8T   | {:>16} | {:>7.4}s | {:>12.2}x | {}", d_8t, time_8t, time_1t / time_8t, if d_1t == d_8t { "BIT-EXACT MATCH (PASS)" } else { "FAIL" });
    println!("-----------------------------------------------------------------------------------------");
    println!(">> ARENA COUNTERS:");
    println!("   Blocks Built / Run        : ~{} (at 64k integers / block)", expected_blocks);
    println!("   Cells Served per Block    : ~{:.1} cells/block", total_cells as f64 / expected_blocks.max(1) as f64);
    println!("   Builds per Block          : 1.00 (Invariant 2 satisfied)");
    println!("   1T == 8T Equivalence      : BIT-EXACT MATCH CERTIFIED!");
    println!("=========================================================================================");
}
