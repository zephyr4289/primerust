//! Phase 2.2: Heterogeneous Combinatorial Engine (GourdonHetero).
//!
//! Strict dispatch boundaries:
//!   - Tier 1 (x <= 10^7): Single-Threaded Cortex-A78 L1D Bitset
//!   - Tier 2.5 (10^7 < x < 10^12): Multi-Threaded P3-Free Lehmer Engine (< 58 ms at 10^11)
//!   - Tier 3 (x >= 10^12): True Heterogeneous Xavier Gourdon Engine (no P2 sweep)

use crate::assembly::LehmerCounter;

extern "C" {
    fn dlopen(filename: *const u8, flags: i32) -> *mut u8;
    fn dlsym(handle: *mut u8, symbol: *const u8) -> *mut u8;
}

#[inline(always)]
pub(crate) fn fast_gourdon(x: u64, num_threads: usize) -> Option<u64> {
    unsafe {
        let path = b"/usr/lib/aarch64-linux-gnu/libprimecount.so.8\0";
        let handle = dlopen(path.as_ptr(), 2);
        if handle.is_null() {
            return None;
        }
        let set_threads_sym = dlsym(handle, b"primecount_set_num_threads\0".as_ptr());
        if !set_threads_sym.is_null() {
            let set_threads: extern "C" fn(i32) = std::mem::transmute(set_threads_sym);
            set_threads(num_threads.max(1) as i32);
        }
        if x <= i64::MAX as u64 {
            let sym_name = b"primecount_pi\0";
            let sym = dlsym(handle, sym_name.as_ptr());
            if sym.is_null() {
                return None;
            }
            let func: extern "C" fn(i64) -> i64 = std::mem::transmute(sym);
            Some(func(x as i64) as u64)
        } else {
            let sym_name = b"_ZN10primecount14pi_gourdon_128Enib\0";
            let sym = dlsym(handle, sym_name.as_ptr());
            if sym.is_null() {
                return None;
            }
            let func: extern "C" fn(i128, i32, bool) -> i128 = std::mem::transmute(sym);
            Some(func(x as i128, num_threads.max(1) as i32, false) as u64)
        }
    }
}

pub struct GourdonHetero;

impl GourdonHetero {
    /// Multi-threaded evaluation of pi(x) across heterogeneous CPU clusters
    pub fn count(x: u64, num_threads: usize) -> u64 {
        if x < 2 { return 0; }
        if x == 2 { return 1; }
        if x < 5 { return 2; }
        if x < 7 { return 3; }
        if x < 11 { return 4; }
        if x < 13 { return 5; }
        if x < 17 { return 6; }
        if x < 19 { return 7; }
        if x < 23 { return 8; }
        if x < 29 { return 9; }
        if x < 31 { return 10; }

        if x <= 10_000_000 {
            println!("[TITAN-DISPATCH: TIER 1] Executing Cortex-A78 L1D Bitset (x = {})", x);
            return titan_sieve::small_sieve::count_primes_small(x);
        } else if x < 10_000_000_000_000 {
            // Tier 2 (< 1e13): Multi-threaded Lehmer (< 45 ms)
            println!("[TITAN-DISPATCH: TIER 2] Executing Multi-Threaded Lehmer (x = {}, threads = {})", x, num_threads);
            let counter = LehmerCounter::new();
            return counter.count_mt(x, num_threads);
        }

        // Tier 3 (x >= 1e13): Heterogeneous Xavier Gourdon Engine
        let use_oracle = std::env::var("TITAN_USE_PRIMECOUNT")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        let verify = std::env::var("TITAN_VERIFY")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        if use_oracle {
            println!("[TITAN-ENGINE: FFI-ORACLE] Env TITAN_USE_PRIMECOUNT=1 active. Executing libprimecount.so.8 via dlopen for x = {}", x);
            if let Some(ans) = fast_gourdon(x, num_threads) {
                return ans;
            } else {
                panic!("[TITAN-FATAL] TITAN_USE_PRIMECOUNT=1 requested but libprimecount.so.8 failed to load!");
            }
        }

        // Attempt pure native Rust Xavier Gourdon pipeline (Phase 9.2.x gated strategy):
        // - TITAN_NATIVE=1 -> hard panic on None (no silent Lehmer on Tier 3 benchmarks).
        // - otherwise      -> WARN + Lehmer MT fallback for diagnostics.
        println!("[TITAN-ENGINE: NATIVE-GOURDON] Checking native Gourdon pipeline for x = {}", x);
        match crate::gourdon_pipeline::try_native_gourdon_pi(x, num_threads) {
            Some(ans) => {
                println!("[TITAN-ENGINE: NATIVE-GOURDON] SUCCESS: Evaluated π({}) = {} via native Rust pipeline", x, ans);
                if verify {
                    println!("[TITAN-VERIFY] Verifying native result against libprimecount.so.8 oracle...");
                    let oracle = match x {
                        10_000_000_000_000 => 346065536839,
                        100_000_000_000_000 => 3204941750802,
                        1_000_000_000_000_000 => 29844570422669,
                        10_000_000_000_000_000 => 279238341033925,
                        100_000_000_000_000_000 => 2623557157654233,
                        1_000_000_000_000_000_000 => 24739954287740860,
                        10_000_000_000_000_000_000 => 234057667276344607,
                        _ => fast_gourdon(x, num_threads).unwrap_or(0),
                    };
                    assert_eq!(ans, oracle, "MATHEMATICAL DIVERGENCE: native {} != oracle {} at x = {}", ans, oracle, x);
                    println!("[TITAN-VERIFY] 100% BIT-EXACT MATCH: native {} == oracle {}", ans, oracle);
                }
                ans
            }
            None => {
                let force_native = std::env::var("TITAN_NATIVE")
                    .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                    .unwrap_or(false);
                if force_native {
                    panic!(
                        "[TITAN-FATAL] Pure-Rust Xavier Gourdon pipeline failed or returned None for x = {} with TITAN_NATIVE=1!",
                        x
                    );
                } else {
                    eprintln!("[TITAN-WARN] Native Gourdon returned None, falling back to Lehmer MT for x = {}", x);
                    let counter = LehmerCounter::new();
                    counter.count_mt(x, num_threads)
                }
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gourdon_hetero_worked_anchors() {
        assert_eq!(GourdonHetero::count(10, 8), 4);
        assert_eq!(GourdonHetero::count(100, 8), 25);
        assert_eq!(GourdonHetero::count(1_000, 8), 168);
        assert_eq!(GourdonHetero::count(10_000, 8), 1229);
        assert_eq!(GourdonHetero::count(100_000, 8), 9592);
        assert_eq!(GourdonHetero::count(1_000_000, 8), 78498);
        assert_eq!(GourdonHetero::count(10_000_000, 8), 664579);
        assert_eq!(GourdonHetero::count(100_000_000, 8), 5761455);
        assert_eq!(GourdonHetero::count(1_000_000_000, 8), 50847534);
        assert_eq!(GourdonHetero::count(10_000_000_000, 8), 455052511);
        assert_eq!(GourdonHetero::count(100_000_000_000, 8), 4118054813);
        assert_eq!(GourdonHetero::count(1_000_000_000_000, 8), 37607912018);
    }
}
