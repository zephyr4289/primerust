//! Phase 2.1: Xavier Gourdon Ordinary Leaves Phi_0(x, y, z, k) (phi0.rs).
//!
//! Evaluates the contribution of ordinary leaves:
//! Phi0(x, y, z, k) = sum_{n <= z, P+(n) <= y} mu(n) * phi_tiny(x / n, k)

use titan_core::phi_tiny::phi_tiny;

/// Recursive square-free tree traversal for Phi0 ordinary leaves.
fn phi0_thread(
    x: u64,
    z: u64,
    mut b: usize,
    k: usize,
    square_free: u64,
    mu: i64,
    primes: &[u64],
) -> i64 {
    let mut phi0 = 0i64;
    b += 1;
    while b < primes.len() {
        let p = primes[b];
        let next = match square_free.checked_mul(p) {
            Some(v) if v <= z => v,
            _ => break,
        };
        phi0 += mu * (phi_tiny(x / next, k as u64) as i64);
        phi0 += phi0_thread(x, z, b, k, next, -mu, primes);
        b += 1;
    }
    phi0
}

/// Computes the genuine Xavier Gourdon Phi0 ordinary leaves:
/// Phi0(x, y, z, k) = phi_tiny(x, k) - sum_{b=k+1}^{pi(y)} phi_tiny(x / p_b, k)
///                  + sum_{square_free <= z} mu(n) * phi_tiny(x / n, k)
pub fn compute_phi0(x: u64, y: u64, z: u64, k: usize, primes: &[u64]) -> i64 {
    let has_sentinel = primes.first() == Some(&0);
    let prime_slice = if has_sentinel { &primes[1..] } else { primes };
    let pi_y = prime_slice.partition_point(|&p| p <= y);

    let mut phi0 = phi_tiny(x, k as u64) as i64;

    // 1-indexed primes b = k + 1 ..= pi_y
    for b in (k + 1)..=pi_y {
        let p = prime_slice[b - 1];
        phi0 -= phi_tiny(x / p, k as u64) as i64;
        phi0 += phi0_thread(x, z, b - 1, k, p, 1, prime_slice);
    }

    phi0
}

#[derive(Default, Debug, Clone, Copy)]
pub struct Phi0Engine;

impl Phi0Engine {
    pub fn new() -> Self {
        Self
    }

    #[inline(always)]
    pub fn eval(&self, x: u64) -> i64 {
        phi_tiny(x, 6) as i64
    }

    #[inline(always)]
    pub fn eval_gourdon(&self, x: u64, y: u64, z: u64, k: usize, primes: &[u64]) -> i64 {
        compute_phi0(x, y, z, k, primes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use titan_sieve::base::generate_base_primes;

    #[test]
    fn test_phi0_gourdon_ground_truth_e13() {
        let x = 10_000_000_000_000u64;
        let y = 103_411u64;
        let z = 170_628u64;
        let k = 8usize;
        let base_primes = generate_base_primes(y.max(z) + 1000);
        let mut primes = vec![0u64];
        primes.extend_from_slice(&base_primes);

        let phi0 = compute_phi0(x, y, z, k, &primes);
        println!("Computed Phi0(1e13) = {}", phi0);
        assert_eq!(phi0, 99_778_753_004, "Phi0(1e13) must match primecount ground truth");
    }
}
