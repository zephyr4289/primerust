//! Mu-Sieve and Mertens Function Infrastructure (Phase 9 Deliverable).
//!
//! Evaluates the Mobius function mu(d) and Mertens sum M(u) = sum_{d <= u} mu(d).
//!
//! Certification Anchors (OEIS A084237):
//!   - M(10^3) = 2
//!   - M(10^4) = -23
//!   - M(10^5) = -48
//!   - M(10^6) = 212
//!   - M(10^7) = -1037

use titan_core::roots::isqrt;

pub struct MertensTable {
    pub max_n: usize,
    pub mu: Vec<i8>,
    pub m: Vec<i32>,
}

impl MertensTable {
    /// Generates mu(d) and Mertens prefix array M(u) up to max_n using a linear sieve
    pub fn new(max_n: usize) -> Self {
        let mut mu = vec![0i8; max_n + 1];
        let mut primes = Vec::with_capacity(max_n / 10);
        let mut is_prime = vec![true; max_n + 1];

        if max_n >= 1 {
            mu[1] = 1;
        }

        for i in 2..=max_n {
            if is_prime[i] {
                primes.push(i);
                mu[i] = -1;
            }
            for &p in &primes {
                if i * p > max_n {
                    break;
                }
                is_prime[i * p] = false;
                if i % p == 0 {
                    mu[i * p] = 0; // p^2 divides i*p
                    break;
                } else {
                    mu[i * p] = -mu[i];
                }
            }
        }

        let mut m = vec![0i32; max_n + 1];
        let mut running_sum = 0i32;
        for i in 1..=max_n {
            running_sum += mu[i] as i32;
            m[i] = running_sum;
        }

        Self { max_n, mu, m }
    }

    #[inline(always)]
    pub fn mu(&self, n: usize) -> i8 {
        if n <= self.max_n {
            self.mu[n]
        } else {
            panic!("mu index {} exceeds max_n {}", n, self.max_n);
        }
    }

    #[inline(always)]
    pub fn mertens(&self, n: usize) -> i32 {
        if n <= self.max_n {
            self.m[n]
        } else {
            panic!("mertens index {} exceeds max_n {}", n, self.max_n);
        }
    }
}

/// Sieve-based segmented mu-evaluator for range [lo, hi]
pub fn count_mertens_range(lo: u64, hi: u64, base_primes: &[u64]) -> i64 {
    if hi < lo || hi == 0 {
        return 0;
    }
    let span = (hi - lo + 1) as usize;
    let mut mu = vec![1i8; span];
    let mut remaining = Vec::with_capacity(span);
    for n in lo..=hi {
        remaining.push(n);
    }

    let sqrt_hi = isqrt(hi);
    for &p in base_primes {
        if p > sqrt_hi {
            break;
        }
        let p_sq = p * p;
        // Mark square multiples
        let start_sq = if lo % p_sq == 0 {
            lo
        } else {
            lo + (p_sq - lo % p_sq)
        };
        let mut idx_sq = (start_sq - lo) as usize;
        while idx_sq < span {
            mu[idx_sq] = 0;
            idx_sq += p_sq as usize;
        }

        // Divide out prime factors and flip sign
        let start_p = if lo % p == 0 {
            lo
        } else {
            lo + (p - lo % p)
        };
        let mut idx_p = (start_p - lo) as usize;
        while idx_p < span {
            if mu[idx_p] != 0 {
                mu[idx_p] = -mu[idx_p];
                while remaining[idx_p] % p == 0 {
                    remaining[idx_p] /= p;
                }
            }
            idx_p += p as usize;
        }
    }

    // Numbers with remaining prime factor > sqrt(hi)
    for i in 0..span {
        if mu[i] != 0 && remaining[i] > 1 {
            mu[i] = -mu[i];
        }
    }

    mu.iter().map(|&v| v as i64).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oeis_a084237_anchors() {
        let max_n = 10_000_000;
        let table = MertensTable::new(max_n);

        assert_eq!(table.mertens(1_000), 2, "M(10^3) anchor mismatch");
        assert_eq!(table.mertens(10_000), -23, "M(10^4) anchor mismatch");
        assert_eq!(table.mertens(100_000), -48, "M(10^5) anchor mismatch");
        assert_eq!(table.mertens(1_000_000), 212, "M(10^6) anchor mismatch");
        assert_eq!(table.mertens(10_000_000), 1037, "M(10^7) anchor mismatch");
    }
}
