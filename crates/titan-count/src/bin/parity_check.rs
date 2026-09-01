use titan_core::roots::{icbrt, iroot4, isqrt};
use titan_count::arena25::Arena25Engine;
use titan_count::leaves::LeafEngine;
use titan_count::mertens_struct::MertensStructure;
use titan_count::pi_table::PiTable;

fn main() {
    let x = 10_000_000u64; // 10^7
    let x_cbrt = icbrt(x);
    let x_sqrt = isqrt(x);
    let x_root4 = iroot4(x);

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

    let mut leaf_engine = LeafEngine::new();
    let leaf_res = leaf_engine.eval_leaves(x, a, &primes, &pi_table);
    let s0_leaf = leaf_res.s0_val;
    let s1_leaf = leaf_res.s1_val;

    let phi_c = titan_count::phi::eval_mt(x, c, &primes, &pi_table, 1) as i64;
    let d_arena = Arena25Engine::evaluate_special_leaves_arena_mt(x, c, a, &primes, &pi_table, &mertens, 1);

    println!("x = {}", x);
    println!("S0 LeafEngine: {}", s0_leaf);
    println!("Phi_c MT     : {}", phi_c);
    println!("S1 LeafEngine: {}", s1_leaf);
    println!("D Arena25    : {}", d_arena);
    println!("Phi via LeafEngine: {}", s0_leaf + s1_leaf);
    println!("Phi via Arena25   : {}", phi_c - d_arena);
}
