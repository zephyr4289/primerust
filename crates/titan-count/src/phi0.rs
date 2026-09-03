//! Phase 2.1: Small Wheel Legendre Base Phi_0(x) (phi0.rs).
//!
//! Counts integers <= x not divisible by the first c = 6 primes (2, 3, 5, 7, 11, 13).
//! Period M = 30030, Totient phi(M) = 5760.

use titan_core::phi_tiny::phi6;

pub const WHEEL_MOD: u64 = 30030; // 2 * 3 * 5 * 7 * 11 * 13
pub const TOTIENT: u64 = 5760;

#[derive(Default, Debug, Clone, Copy)]
pub struct Phi0Engine;

impl Phi0Engine {
    pub fn new() -> Self {
        Self
    }

    #[inline(always)]
    pub fn eval(&self, x: u64) -> i64 {
        phi6(x) as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phi0_exactness() {
        let engine = Phi0Engine::new();
        assert_eq!(engine.eval(100), phi6(100) as i64);
        assert_eq!(engine.eval(1000), phi6(1000) as i64);
        assert_eq!(engine.eval(30030), 5760);
        assert_eq!(engine.eval(60060), 11520);
    }
}
