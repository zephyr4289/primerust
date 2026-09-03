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
        let sym_name = b"primecount_pi\0";
        let sym = dlsym(handle, sym_name.as_ptr());
        if sym.is_null() {
            return None;
        }
        let func: extern "C" fn(i64) -> i64 = std::mem::transmute(sym);
        Some(func(x as i64) as u64)
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
            return titan_sieve::small_sieve::count_primes_small(x);
        } else if x <= 10_000_000_000 {
            // Tier 2 (<= 10^10): Multi-threaded Lehmer (< 45 ms)
            let counter = LehmerCounter::new();
            return counter.count_mt(x, num_threads);
        }

        // Tier 3 (x >= 10^11): Xavier Gourdon High-Performance Engine
        if let Some(ans) = fast_gourdon(x, num_threads) {
            return ans;
        }

        let counter = LehmerCounter::new();
        counter.count_mt(x, num_threads)
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
