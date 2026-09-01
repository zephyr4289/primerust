//! Alpha: Tuning parameters for y and a = pi(y) selection.
//!
//! Evaluates:
//!   y = alpha * x^(1/3)
//!
//! As proven in the Meissel identity, the formula is exact for any a >= pi(x^(1/3)).
//! Increasing alpha shrinks the S2 sweep span at the cost of a deeper Phi leaf tree.

use titan_core::roots::icbrt;

#[derive(Debug, Clone, Copy)]
pub struct AlphaConfig {
    pub alpha: f64,
}

impl Default for AlphaConfig {
    fn default() -> Self {
        Self { alpha: 1.0 }
    }
}

impl AlphaConfig {
    pub fn for_scale(x: u64) -> Self {
        // Optimal alpha tuning defaults derived from C11 experiments
        let alpha = if x <= 10_000_000_000 {
            1.0
        } else if x <= 1_000_000_000_000 {
            1.1
        } else if x <= 100_000_000_000_000 {
            1.25
        } else {
            1.5
        };
        Self { alpha }
    }

    /// Computes the recommended a parameter for a given x
    pub fn select_a(&self, x: u64, primes: &[u64]) -> usize {
        let x_cbrt = icbrt(x);
        let target_y = ((x_cbrt as f64) * self.alpha).round() as u64;

        let mut a = match primes[1..].binary_search(&target_y) {
            Ok(idx) => idx + 1,
            Err(idx) => idx,
        };

        // Invariant guard: ensure p_a^3 > x
        while (primes[a] as u128) * (primes[a] as u128) * (primes[a] as u128) <= (x as u128) {
            a += 1;
        }

        a
    }
}
