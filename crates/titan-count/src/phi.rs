//! Phi: Combinatorial sifting function Phi(x, a) using an explicit DFS stack.
//!
//! Evaluates:
//!   Phi(x, a) = #{ n <= x : gcd(n, p_1 * ... * p_a) = 1 }
//!
//! Guarantees:
//!   - Zero recursion (stack overflow mathematically impossible)
//!   - Pre-allocated bounded DFS stack (depth <= a <= 1229 at 10^16)
//!   - T0..T3 evaluation tiers

use crate::pi_table::PiTable;
use titan_core::phi_tiny::phi_tiny;

#[derive(Default, Debug, Clone)]
pub struct PhiCensus {
    pub total_nodes: u64,
    pub t0_exits: u64,
    pub t1_exits: u64,
    pub t2_exits: u64,
    pub max_depth: usize,
}

pub struct PhiEngine {
    pub stack: Vec<(u64, usize, i8)>,
    pub census: PhiCensus,
}

impl PhiEngine {
    pub fn new() -> Self {
        Self {
            stack: Vec::with_capacity(4096),
            census: PhiCensus::default(),
        }
    }

    /// Evaluates Phi(x, a) with Census collection
    pub fn eval_with_census(
        &mut self,
        x: u64,
        a: usize,
        primes: &[u64], // 1-indexed primes: primes[1]=2, primes[2]=3, ...
        pi_table: &PiTable,
    ) -> i64 {
        self.stack.clear();
        self.census = PhiCensus::default();

        if x == 0 || a == 0 {
            return x as i64;
        }

        self.stack.push((x, a, 1));
        let mut total = 0i64;

        while let Some((y, i, sign)) = self.stack.pop() {
            self.census.total_nodes += 1;
            let cur_depth = self.stack.len() + 1;
            if cur_depth > self.census.max_depth {
                self.census.max_depth = cur_depth;
            }

            // Tier 0: i == 0 -> y
            if i == 0 {
                self.census.t0_exits += 1;
                total += (sign as i64) * (y as i64);
                continue;
            }

            // Tier 1: i <= 6 -> PhiTiny table lookup (~5 cycles)
            if i <= 6 {
                self.census.t1_exits += 1;
                let val = phi_tiny(y, i as u64) as i64;
                total += (sign as i64) * val;
                continue;
            }

            // Tier 2: y < p_{i+1}^2 -> pi(y) - i + 1
            let next_p = primes[i + 1];
            if y < next_p * next_p {
                self.census.t2_exits += 1;
                let pi_y = pi_table.pi(y) as i64;
                let val = pi_y - (i as i64) + 1;
                total += (sign as i64) * val;
                continue;
            }

            // Tier 3: Interior node -> Phi(y, i-1) - Phi(y/p_i, i-1)
            let p_i = primes[i];
            let y_div = y / p_i;

            self.stack.push((y_div, i - 1, -sign));
            self.stack.push((y, i - 1, sign));
        }

        total
    }

    /// Fast evaluation without census overhead
    #[inline(always)]
    pub fn eval(
        &mut self,
        x: u64,
        a: usize,
        primes: &[u64],
        pi_table: &PiTable,
    ) -> i64 {
        self.stack.clear();
        if x == 0 || a == 0 {
            return x as i64;
        }

        self.stack.push((x, a, 1));
        let mut total = 0i64;

        while let Some((y, i, sign)) = self.stack.pop() {
            if i == 0 {
                total += (sign as i64) * (y as i64);
                continue;
            }
            if i <= 6 {
                let val = phi_tiny(y, i as u64) as i64;
                total += (sign as i64) * val;
                continue;
            }
            let next_p = primes[i + 1];
            if y < next_p * next_p {
                let pi_y = pi_table.pi(y) as i64;
                total += (sign as i64) * (pi_y - (i as i64) + 1);
                continue;
            }

            let p_i = primes[i];
            self.stack.push((y / p_i, i - 1, -sign));
            self.stack.push((y, i - 1, sign));
        }

        total
    }
}
