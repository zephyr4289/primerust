//! Phase 4.1: Linear Factor Table for Greatest Prime Factor (gpf).
//!
//! Euler's Linear Sieve precomputes gpf(m) for all m <= max_y in O(max_y) time
//! with zero hardware divisions.

#[repr(C, align(64))]
pub struct FactorTable {
    gpf: Vec<u32>,
    max_y: usize,
}

impl FactorTable {
    /// Builds the greatest prime factor table for all m <= max_y in O(max_y) time
    /// using Euler's linear sieve. Zero hardware divisions.
    pub fn new(max_y: usize) -> Self {
        let mut gpf = vec![0u32; max_y + 1];
        let mut lpf = vec![0u32; max_y + 1];
        let mut primes = Vec::with_capacity(max_y / 8);

        for i in 2..=max_y {
            if lpf[i] == 0 {
                lpf[i] = i as u32;
                gpf[i] = i as u32;
                primes.push(i as u32);
            }

            let lpf_i = lpf[i];
            let gpf_i = gpf[i];

            for &p in &primes {
                if p > lpf_i {
                    break;
                }
                let next = i * (p as usize);
                if next > max_y {
                    break;
                }
                lpf[next] = p;
                // Invariant: p <= lpf[i] <= gpf[i], so largest factor is unchanged
                gpf[next] = gpf_i;
            }
        }

        // Drop lpf and primes immediately; only gpf is retained in memory
        Self { gpf, max_y }
    }

    /// O(1) lookup of greatest prime factor. Compiles to a single AArch64 LDR.
    #[inline(always)]
    pub fn gpf(&self, m: u64) -> u64 {
        if m <= 1 {
            return 0;
        }
        debug_assert!((m as usize) <= self.max_y, "m exceeds precomputed max_y");
        unsafe { *self.gpf.get_unchecked(m as usize) as u64 }
    }

    #[inline(always)]
    pub fn max_y(&self) -> usize {
        self.max_y
    }
}

pub const MAX_Y_18: usize = 1_000_000;

/// Compressed factor table: stores GPF for odd integers only, halving L3 memory footprint.
#[repr(C, align(64))]
pub struct CompressedFactorTable {
    // Stores GPF for odd integers only: index = m >> 1
    // Size at y = 1,000,000: 500,000 * 4 bytes = 1.90 MiB (100% fits in 2 MiB L3!)
    odd_gpf: Vec<u32>,
    max_y: usize,
}

impl CompressedFactorTable {
    pub fn new(max_y: usize) -> Self {
        let mut gpf = vec![0u32; max_y + 1];
        let mut lpf = vec![0u32; max_y + 1];
        let mut primes = Vec::with_capacity(max_y / 8);

        for i in 2..=max_y {
            if lpf[i] == 0 {
                lpf[i] = i as u32;
                gpf[i] = i as u32;
                primes.push(i as u32);
            }

            let lpf_i = lpf[i];
            let gpf_i = gpf[i];

            for &p in &primes {
                if p > lpf_i {
                    break;
                }
                let next = i * (p as usize);
                if next > max_y {
                    break;
                }
                lpf[next] = p;
                gpf[next] = gpf_i;
            }
        }

        let odd_len = (max_y >> 1) + 1;
        let mut odd_gpf = vec![0u32; odd_len];
        for i in (1..=max_y).step_by(2) {
            odd_gpf[i >> 1] = gpf[i];
        }

        Self { odd_gpf, max_y }
    }

    #[inline(always)]
    pub fn gpf(&self, m: u64) -> u64 {
        if m <= 1 {
            return 0;
        }
        if m & 1 == 1 {
            unsafe { *self.odd_gpf.get_unchecked((m >> 1) as usize) as u64 }
        } else {
            // For even m: gpf(2 * k) = max(2, gpf(odd_part(k)))
            let odd_part = m >> m.trailing_zeros();
            if odd_part <= 1 {
                2
            } else {
                unsafe { (*self.odd_gpf.get_unchecked((odd_part >> 1) as usize) as u64).max(2) }
            }
        }
    }

    #[inline(always)]
    pub fn max_y(&self) -> usize {
        self.max_y
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn naive_gpf(mut n: u64) -> u64 {
        if n <= 1 { return 0; }
        let mut max_p = 0;
        let mut d = 2;
        while d * d <= n {
            if n % d == 0 {
                max_p = max_p.max(d);
                while n % d == 0 { n /= d; }
            }
            d += 1;
        }
        if n > 1 { max_p = max_p.max(n); }
        max_p
    }

    #[test]
    fn test_factor_table_correctness() {
        let limit = 10_000;
        let table = FactorTable::new(limit);
        let compressed = CompressedFactorTable::new(limit);

        assert_eq!(table.gpf(0), 0);
        assert_eq!(table.gpf(1), 0);
        assert_eq!(compressed.gpf(0), 0);
        assert_eq!(compressed.gpf(1), 0);

        for m in 2..=limit as u64 {
            let expected = naive_gpf(m);
            let actual = table.gpf(m);
            let actual_compressed = compressed.gpf(m);
            assert_eq!(
                actual, expected,
                "GPF mismatch for m = {}: expected {}, got {}",
                m, expected, actual
            );
            assert_eq!(
                actual_compressed, expected,
                "Compressed GPF mismatch for m = {}: expected {}, got {}",
                m, expected, actual_compressed
            );
        }
    }
}
