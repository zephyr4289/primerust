//! Magic Division: Ultra-fast 3-cycle division by primes for y <= 10^16.
//!
//! Theorem:
//!   For any p >= 7 and y <= 10^16, with shift = 64 + floor(log2(p)) and
//!   M = ceil(2^shift / p), floor(y * M / 2^shift) == floor(y / p) identically.
//!
//! Replaces 64-bit hardware integer division with umulh + lsr (~3-4 cycles).

#[derive(Copy, Clone, Debug)]
pub struct MagicPrimeDiv {
    pub prime: u64,
    pub magic: u64,
    pub shift: u8,
}

impl MagicPrimeDiv {
    pub fn new(p: u64) -> Self {
        assert!(p >= 7);
        let log2_p = 63 - p.leading_zeros();
        let shift = 64 + (log2_p as u8);
        let num = 1u128 << shift;
        let magic = (num / (p as u128) + 1) as u64;

        Self {
            prime: p,
            magic,
            shift,
        }
    }

    #[inline(always)]
    pub fn div(&self, y: u64) -> u64 {
        ((y as u128 * self.magic as u128) >> self.shift) as u64
    }
}

pub struct MagicDivTable {
    divisors: Vec<MagicPrimeDiv>,
}

impl MagicDivTable {
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.divisors.len()
    }

    pub fn new(primes: &[u64]) -> Self {
        let mut divisors = Vec::with_capacity(primes.len());
        for &p in primes {
            if p >= 7 {
                divisors.push(MagicPrimeDiv::new(p));
            } else {
                divisors.push(MagicPrimeDiv { prime: p, magic: 0, shift: 0 });
            }
        }
        Self { divisors }
    }

    #[inline(always)]
    pub fn div(&self, y: u64, prime_idx: usize) -> u64 {
        let entry = &self.divisors[prime_idx];
        if entry.prime >= 7 {
            entry.div(y)
        } else {
            y / entry.prime
        }
    }

    /// K4: Batched magic division - processes 8 divisions at once.
    /// Manual loop unrolling for ILP and reduced instruction overhead.
    #[inline(always)]
    pub fn div_batch8(&self, ys: &[u64; 8], prime_indices: &[usize; 8], results: &mut [u64; 8]) {
        for i in 0..8 {
            let entry = &self.divisors[prime_indices[i]];
            if entry.prime >= 7 {
                results[i] = ((ys[i] as u128 * entry.magic as u128) >> entry.shift) as u64;
            } else {
                results[i] = ys[i] / entry.prime;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_magic_div_correctness() {
        let test_primes = [7u64, 11, 13, 17, 19, 23, 29, 31, 97, 1009, 9973];
        for &p in &test_primes {
            let m = MagicPrimeDiv::new(p);
            // Test boundaries
            for &y in &[0u64, 1, p - 1, p, p + 1, p * p, 10_000_000, 1_000_000_000_000, 10_000_000_000_000_000] {
                assert_eq!(m.div(y), y / p, "Failed for y={}, p={}", y, p);
            }
            // Pseudo-random sampling
            let mut y = 123456789u64;
            for _ in 0..10_000 {
                y = y.wrapping_mul(6364136223846793005).wrapping_add(1) % 10_000_000_000_000_000;
                assert_eq!(m.div(y), y / p, "Failed random for y={}, p={}", y, p);
            }
        }
    }
}
