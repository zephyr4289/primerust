//! Base Primes: bootstrap generator for primes up to sqrt(N).
//!
//! Generated once at engine construction time.

use titan_core::roots::isqrt;

/// Generates all primes <= limit.
/// Limit for N <= 10^11 is sqrt(10^11) <= 316,228.
pub fn generate_base_primes(limit: u64) -> Vec<u64> {
    if limit < 2 {
        return Vec::new();
    }
    if limit < 3 {
        return vec![2];
    }

    let mut primes = vec![2u64];
    let max_i = ((limit - 3) >> 1) as usize; // index i <-> 2i + 3
    let mut composite = vec![false; max_i + 1];
    let sqrt_lim = isqrt(limit) as usize;

    let mut i = 0usize;
    while 2 * i + 3 <= sqrt_lim {
        if !composite[i] {
            let p = (2 * i + 3) as u64;
            // First odd multiple is p * p
            let mut j = ((p * p - 3) >> 1) as usize;
            let step = p as usize;
            while j <= max_i {
                composite[j] = true;
                j += step;
            }
        }
        i += 1;
    }

    for idx in 0..=max_i {
        if !composite[idx] {
            primes.push((2 * idx + 3) as u64);
        }
    }

    primes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_primes_count() {
        let p_1000 = generate_base_primes(7919);
        assert_eq!(p_1000.len(), 1000);
        assert_eq!(*p_1000.last().unwrap(), 7919);

        // pi(316,227) is known to be 27,293
        let p_sqrt = generate_base_primes(316_227);
        assert_eq!(p_sqrt.len(), 27293);
    }
}
