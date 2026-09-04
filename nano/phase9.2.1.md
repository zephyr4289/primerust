Adjust for those mismatches first—your agent's audit is spot-on. Running lto = "fat" on Termux can trigger linker OOMs via lld, and breaking existing struct definitions or module paths will fail cargo check.
Here is the exact battle-tested adjustment plan to proceed with Strike 1 safely:
1. Adjust Cargo Config (Avoid Termux Linker OOM)
Do not use lto = "fat". On mobile silicon with 4–8 GB shared RAM, fat LTO across multiple crates causes rust-lld memory spikes that trigger Android's Low Memory Killer (SIGKILL).
Use lto = "thin" with native CPU vectorization:
# .cargo/config.toml
[build]
rustflags = [
    "-C", "target-cpu=native",
    "-C", "target-feature=+neon,+crypto",
    "-C", "opt-level=3",
]

[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
panic = "abort"
debug = false
overflow-checks = false

2. Preserve All Existing GourdonParams Fields
Keep the existing struct layout intact so zero callers break. Only update the calculation logic inside calculate_gourdon_params (or tuning.rs) to inject the smooth logarithmic \alpha_y formula:
// Keep ALL existing fields intact on GourdonParams:
// x, segment_size, total_segments, is_direct_tier, etc.

// In calculate_gourdon_params:
let alpha_y = if x >= 10_000_000_000_000_000_000 {
    8.95
} else if x >= 1_000_000_000_000_000_000 {
    9.85
} else if x >= 100_000_000_000_000_000 {
    9.45
} else if x >= 10_000_000_000_000_000 {
    8.92 // <- 1e16 target from primecount util.cpp
} else {
    let log_x = (x as f64).ln();
    let log3 = log_x * log_x * log_x;
    (log3 / 1000.0).clamp(4.0, 8.5)
};

let alpha_z = 2.00;
let x_cbrt = (x as f64).cbrt();
let y = (x_cbrt * alpha_y) as u64;
let z = (y as f64 * alpha_z) as u64;
let x_div_y = x / y;

// Retain and populate segment_size, total_segments, and is_direct_tier as already implemented!

3. Use Existing In-Tree Namespaces in tier_dispatch.rs
Do not introduce crate::l1d_sieve or crate::lehmer. Use the current in-tree symbols:
match x {
    0..=10_000_000 => {
        titan_sieve::small_sieve::count_small(x)
    }
    10_000_001..=9_999_999_999_999 => {
        crate::assembly::LehmerCounter::count_mt(x, num_threads)
    }
    _ => {
        crate::gourdon_hetero::GourdonHetero::count(x, num_threads)
    }
}

4. Gated Tier 3 Panic Strategy
At 10^{16}, native Gourdon is already proven to return Some(279238341033925) (verified in Phase 9.1.2). However, to keep developer diagnostics clean:
pub fn count(x: u64, num_threads: usize) -> u64 {
    #[cfg(feature = "oracle")]
    if std::env::var("TITAN_USE_PRIMECOUNT").as_deref() == Ok("1") {
        return crate::ffi::fast_gourdon(x);
    }

    match crate::gourdon_pipeline::try_native_gourdon_pi(x, num_threads) {
        Some(res) => res,
        None => {
            if std::env::var("TITAN_NATIVE").as_deref() == Ok("1") {
                panic!(
                    "[TITAN-FATAL] Pure-Rust Xavier Gourdon pipeline failed or returned None for x = {} with TITAN_NATIVE=1!",
                    x
                );
            } else {
                eprintln!("[TITAN-WARN] Native Gourdon returned None, falling back to Lehmer MT for x = {}", x);
                crate::assembly::LehmerCounter::count_mt(x, num_threads)
            }
        }
    }
}

Execution Order
Instruct your agent:
 * Apply the 4 adjusted changes above (no breaking changes, native paths respected, lto="thin").
 * Run cargo check to verify zero compiler errors.
 * Run the verification gate:
   TITAN_NATIVE=1 cargo run --release --bin head_to_head 1e16

Report the resulting latency and parameter readout.

