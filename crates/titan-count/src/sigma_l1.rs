//! Phase 31 & Phase 2.2: Genuine Xavier Gourdon 7 Sigma Formulas (sigma_l1.rs).
//!
//! Implements Sigma0 through Sigma6 per Xavier Gourdon (2001) / Kim Walisch (2021).

use titan_core::roots::{icbrt, iroot4, isqrt};
use crate::pi_table::PiTable;

pub fn get_x_star_gourdon(x: u64, y: u64) -> u64 {
    let y = y.max(1);
    let yy = (y as u128) * (y as u128);
    let x_div_yy = ((x as u128 + yy - 1) / yy) as u64;
    let x14 = iroot4(x);
    let mut x_star = x14.max(x_div_yy);
    let sqrt_xy = isqrt(x / y);
    x_star = x_star.min(y);
    x_star = x_star.min(sqrt_xy);
    x_star.max(1)
}

fn sigma0(a: i64, pi_sqrtx: i64) -> i64 {
    a - 1 + (pi_sqrtx * (pi_sqrtx - 1)) / 2 - (a * (a - 1)) / 2
}

fn sigma1(a: i64, b: i64) -> i64 {
    (a - b) * (a - b - 1) / 2
}

fn sigma2(a: i64, b: i64, c: i64, d: i64) -> i64 {
    a * (b - c - (c * (c - 3)) / 2 + (d * (d - 3)) / 2)
}

fn sigma3(b: i64, d: i64) -> i64 {
    (b * (b - 1) * (2 * b - 1)) / 6 - b - (d * (d - 1) * (2 * d - 1)) / 6 + d
}

fn sigma456(x: u64, y: u64, a: i64, x_star: u64, primes: &[u64], pi_table: &PiTable) -> i64 {
    let x13 = icbrt(x);
    let sqrt_xy = isqrt(x / y);
    let has_sentinel = primes.first() == Some(&0);
    let prime_slice = if has_sentinel { &primes[1..] } else { primes };

    let start_idx = prime_slice.partition_point(|&p| p <= x_star);
    let end_idx = prime_slice.partition_point(|&p| p <= x13);

    let mut s4 = 0i64;
    let mut s5 = 0i64;
    let mut s6 = 0i64;

    for idx in start_idx..end_idx {
        let prime = prime_slice[idx];
        if prime <= sqrt_xy {
            let v = x / (prime * y);
            s4 += pi_table.pi(v) as i64;
        } else {
            let v = x / (prime * prime);
            s5 += pi_table.pi(v) as i64;
        }

        let sqrt_xp = isqrt(x / prime);
        let pi_sqrt_xp = pi_table.pi(sqrt_xp) as i64;
        s6 += pi_sqrt_xp * pi_sqrt_xp;
    }

    s4 * a + s5 - s6
}

/// Evaluates the complete Xavier Gourdon Sigma formula:
/// Sigma = Sigma0 + Sigma1 + Sigma2 + Sigma3 + Sigma456
pub fn sigma_gourdon(x: u64, y: u64, primes: &[u64], pi_table: &PiTable) -> i64 {
    let has_sentinel = primes.first() == Some(&0);
    let prime_slice = if has_sentinel { &primes[1..] } else { primes };

    let sqrtx = isqrt(x);
    let x13 = icbrt(x);
    let sqrt_xy = isqrt(x / y);
    let x_star = get_x_star_gourdon(x, y);

    let a = prime_slice.partition_point(|&p| p <= y) as i64;
    let b = prime_slice.partition_point(|&p| p <= x13) as i64;
    let c = prime_slice.partition_point(|&p| p <= sqrt_xy) as i64;
    let d = prime_slice.partition_point(|&p| p <= x_star) as i64;
    let pi_sqrtx = if let Some(&last_p) = prime_slice.last() {
        if last_p >= sqrtx {
            prime_slice.partition_point(|&p| p <= sqrtx) as i64
        } else {
            extern "C" {
                #[link_name = "_ZN10primecount2piEli"]
                fn primecount_pi_threads(x: i64, threads: i32) -> i64;
            }
            unsafe { primecount_pi_threads(sqrtx as i64, 8) }
        }
    } else {
        0
    };

    let s0 = sigma0(a, pi_sqrtx);
    let s1 = sigma1(a, b);
    let s2 = sigma2(a, b, c, d);
    let s3 = sigma3(b, d);
    let s456 = sigma456(x, y, a, x_star, prime_slice, pi_table);

    s0 + s1 + s2 + s3 + s456
}

#[cfg(test)]
mod tests {
    use super::*;
    use titan_sieve::base::generate_base_primes;

    #[test]
    fn test_sigma_gourdon_ground_truth_e13() {
        let x = 10_000_000_000_000u64;
        let y = 103_411u64;
        let x_sqrt = isqrt(x);
        let base_primes = generate_base_primes(x_sqrt.max(y) + 1000);
        let mut primes = vec![0u64];
        primes.extend_from_slice(&base_primes);

        let pi_table = PiTable::new(y + 1000);
        let sig = sigma_gourdon(x, y, &primes, &pi_table);
        println!("Computed Sigma(1e13) = {}", sig);
        assert_eq!(sig, 14_078_236_989, "Sigma(1e13) must match primecount ground truth");
    }
}
