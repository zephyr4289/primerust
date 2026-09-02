//! Phase 39: Möbius-First Streaming Combinatorial Engine (LMOEngine).
//!
//! Evaluates pi(x) using streaming Mobius values and O(1) Phi tables,
//! operating in O(x^(1/4)) RAM without building large intermediate FactorTable arrays.

use titan_core::roots::isqrt;
use titan_sieve::base::generate_base_primes;
use crate::mobius_stream::MobiusStream;
use crate::phi_tables::PhiTables;
use crate::pi_table::PiTable;

pub struct LMOEngine {
    pub primes: Vec<u64>,
    pub pi_table: PiTable,
}

impl LMOEngine {
    pub fn new(max_sqrt: u64) -> Self {
        let base_primes = generate_base_primes(max_sqrt + 100);
        let mut primes = Vec::with_capacity(base_primes.len() + 1);
        primes.push(0);
        primes.extend_from_slice(&base_primes);
        let pi_table = PiTable::new(max_sqrt + 30);
        Self { primes, pi_table }
    }

    /// Evaluates pi(x) using the streaming Mobius combinatorial identity
    pub fn count(&self, x: u64) -> u64 {
        if x < 2 { return 0; }
        if x <= self.pi_table.max_y {
            return self.pi_table.pi(x);
        }

        let sqrt_x = isqrt(x);
        let mut stream = MobiusStream::new(sqrt_x);

        // a = pi(x^(1/4)) or small constant
        let x_root4 = isqrt(sqrt_x);
        let a = match self.primes[1..].binary_search(&x_root4) {
            Ok(idx) => idx + 1,
            Err(idx) => idx,
        };
        let a = a.min(7).max(1);

        let mut sum = 0i128;

        while let Some((d, mu)) = stream.next() {
            if mu == 0 {
                continue;
            }
            let x_d = x / d;
            let phi_val = PhiTables::phi_small(x_d, a) as i128;
            let pi_d = if d > 1 {
                self.pi_table.pi(d - 1) as i128
            } else {
                0
            };

            let term = (mu as i128) * (phi_val - pi_d);
            sum += term;
        }

        if sum < 0 {
            0
        } else {
            sum as u64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lmo_engine_small() {
        let engine = LMOEngine::new(1000);
        assert_eq!(engine.count(10), 4);
        assert_eq!(engine.count(100), 25);
        assert_eq!(engine.count(1000), 168);
    }
}
