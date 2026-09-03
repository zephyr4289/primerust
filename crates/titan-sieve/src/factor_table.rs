//! Phase 7.3: Fused Wheel-2310 Factor Table for Hard Special Leaves D(x, y, z).
//!
//! Encodes mu(n) != 0, mpf(n) <= y, and lpf(n) > prime into a single u16 comparison:
//!     `if prime < factor_table.get_factor(n)`
//! Reduces coprime candidate space to 20.78% using Wheel-2310.

use std::alloc::{alloc_zeroed, dealloc, Layout};

pub const WHEEL2310: usize = 2310;
pub const WHEEL2310_COPRIMES: usize = 480;

pub struct FactorTableD {
    pub z: usize,
    pub y: usize,
    table: *mut u16,
    pub table_len: usize,
    layout: Layout,
    coprime_to_idx: [u16; WHEEL2310],
    idx_to_coprime: [u16; WHEEL2310_COPRIMES],
}

unsafe impl Send for FactorTableD {}
unsafe impl Sync for FactorTableD {}

impl FactorTableD {
    pub fn new(z: usize, y: usize) -> Self {
        // 1. Build Wheel-2310 forward and backward residue maps
        let mut coprime_to_idx = [u16::MAX; WHEEL2310];
        let mut idx_to_coprime = [0u16; WHEEL2310_COPRIMES];
        let mut idx = 0;

        for r in 1..WHEEL2310 {
            if r % 2 != 0 && r % 3 != 0 && r % 5 != 0 && r % 7 != 0 && r % 11 != 0 {
                coprime_to_idx[r] = idx as u16;
                idx_to_coprime[idx] = r as u16;
                idx += 1;
            }
        }
        debug_assert_eq!(idx, WHEEL2310_COPRIMES);

        // 2. Allocate compressed factor table aligned for vector streaming
        let blocks = (z + WHEEL2310 - 1) / WHEEL2310 + 1;
        let table_len = blocks * WHEEL2310_COPRIMES + 1;
        let layout = Layout::array::<u16>(table_len)
            .unwrap()
            .align_to(64)
            .unwrap();

        let table = unsafe { alloc_zeroed(layout) as *mut u16 };
        assert!(!table.is_null(), "FactorTableD allocation failed");

        let mut ft = Self {
            z,
            y,
            table,
            table_len,
            layout,
            coprime_to_idx,
            idx_to_coprime,
        };

        ft.precompute_factors();
        ft
    }

    /// Precomputes fused factors across all numbers coprime to 2310 up to z.
    fn precompute_factors(&mut self) {
        let z = self.z;
        let y = self.y;

        // Small prime linear sieve for factorization up to z
        let mut min_prime = vec![0u32; z + 1];
        let mut max_prime = vec![0u32; z + 1];
        let mut mu = vec![1i8; z + 1];
        let mut primes = Vec::with_capacity(100_000);

        for i in 2..=z {
            if min_prime[i] == 0 {
                min_prime[i] = i as u32;
                max_prime[i] = i as u32;
                mu[i] = -1;
                primes.push(i as u32);
            }
            for &p in &primes {
                let p = p as usize;
                if p > min_prime[i] as usize || i * p > z {
                    break;
                }
                min_prime[i * p] = p as u32;
                max_prime[i * p] = max_prime[i].max(p as u32);
                mu[i * p] = if min_prime[i] as usize == p { 0 } else { -mu[i] };
            }
        }

        // Populate compressed Wheel-2310 entries
        for n in 1..=z {
            let rem = n % WHEEL2310;
            let coprime_idx = self.coprime_to_idx[rem];
            if coprime_idx == u16::MAX {
                continue; // Not coprime to 2310, handled by sieve wheel
            }

            let block = n / WHEEL2310;
            let packed_idx = block * WHEEL2310_COPRIMES + (coprime_idx as usize);

            // Condition fusion:
            // Valid leaf <=> mu[n] != 0 AND mpf[n] <= y
            // Value stored: lpf[n] (clamped to u16::MAX if square-free prime factor > u16::MAX)
            if mu[n] != 0 && (max_prime[n] as usize) <= y {
                let lpf = min_prime[n];
                unsafe {
                    *self.table.add(packed_idx) = lpf.min(u16::MAX as u32) as u16;
                }
            } else {
                unsafe {
                    *self.table.add(packed_idx) = 0;
                }
            }
        }
    }

    /// Single-branch evaluation of composite leaf validity.
    /// Returns 0 if composite n is invalid or square-full.
    #[inline(always)]
    pub fn get_factor(&self, n: usize) -> u32 {
        if n > self.z {
            return 0;
        }
        let rem = n % WHEEL2310;
        let coprime_idx = unsafe { *self.coprime_to_idx.get_unchecked(rem) };
        if coprime_idx == u16::MAX {
            return 0;
        }
        let block = n / WHEEL2310;
        let packed_idx = block * WHEEL2310_COPRIMES + (coprime_idx as usize);
        unsafe { *self.table.add(packed_idx) as u32 }
    }

    /// Traverses coprime residues across the segment and accumulates leaf count
    #[inline(always)]
    pub fn process_segment_leaves(
        &self,
        seg_low: u64,
        seg_high: u64,
        prime: u32,
        sieve_buf: &[u8],
        leaf_sum: &mut i64,
    ) {
        let mut n = seg_low as usize;
        let limit = seg_high as usize;

        while n < limit {
            let factor = self.get_factor(n);
            if prime < factor {
                let bit_offset = (n - seg_low as usize) / 30;
                if bit_offset < sieve_buf.len() {
                    let byte_val = unsafe { *sieve_buf.get_unchecked(bit_offset) };
                    if (byte_val & (1 << ((n % 30) >> 2))) != 0 {
                        *leaf_sum += 1;
                    }
                }
            }
            n += 1;
        }
    }
}

impl Drop for FactorTableD {
    fn drop(&mut self) {
        unsafe {
            dealloc(self.table as *mut u8, self.layout);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_factor_table_wheel2310_fused() {
        let z = 100_000;
        let y = 10_000;
        let ft = FactorTableD::new(z, y);

        // 13 is prime and coprime to 2310: mu != 0, mpf = 13 <= y, lpf = 13
        assert_eq!(ft.get_factor(13), 13);

        // 13 * 17 = 221: coprime to 2310, mu = 1, mpf = 17 <= y, lpf = 13
        assert_eq!(ft.get_factor(221), 13);

        // 13 * 13 = 169: square-full (mu = 0) -> factor must be 0
        assert_eq!(ft.get_factor(169), 0);

        // Multiples of 2, 3, 5, 7, 11 must return 0
        assert_eq!(ft.get_factor(2), 0);
        assert_eq!(ft.get_factor(3), 0);
        assert_eq!(ft.get_factor(5), 0);
        assert_eq!(ft.get_factor(7), 0);
        assert_eq!(ft.get_factor(11), 0);
        assert_eq!(ft.get_factor(14), 0);
        assert_eq!(ft.get_factor(15), 0);

        // Composite with mpf > y must return 0
        let ft_small_y = FactorTableD::new(z, 15);
        assert_eq!(ft_small_y.get_factor(221), 0); // mpf(221) = 17 > 15
    }
}
