//! Leaves: Ordinary and combinatorial leaf enumerator for LMO / Gourdon engine.
//!
//! Evaluates:
//!   - Ordinary leaves (S0): direct evaluation via PhiTiny
//!   - T2 multi-factor leaves: direct evaluation via PiTable
//!   - Census metrics on leaf distributions and depth bounds

use crate::magic::MagicDivTable;
use crate::pi_table::PiTable;
use titan_core::phi_tiny::phi_tiny;

#[derive(Debug, Default, Clone)]
pub struct LeafSummary {
    pub s0_val: i64,
    pub s1_val: i64,
    pub ordinary_leaves_count: u64,
    pub multi_factor_leaves_count: u64,
}

pub struct LeafEngine {
    stack: Vec<(u64, usize, i8)>,
}

impl LeafEngine {
    pub fn new() -> Self {
        Self {
            stack: Vec::with_capacity(4096),
        }
    }

    /// Evaluates ordinary leaves and multi-factor T2 leaves
    pub fn eval_leaves(
        &mut self,
        x: u64,
        a: usize,
        primes: &[u64],
        pi_table: &PiTable,
    ) -> LeafSummary {
        let magic = MagicDivTable::new(primes);
        self.stack.clear();

        if x == 0 || a == 0 {
            return LeafSummary {
                s0_val: x as i64,
                s1_val: 0,
                ordinary_leaves_count: 1,
                multi_factor_leaves_count: 0,
            };
        }

        self.stack.push((x, a, 1));
        let mut summary = LeafSummary::default();

        while let Some((y, i, sign)) = self.stack.pop() {
            if i <= 6 {
                let val = phi_tiny(y, i as u64) as i64;
                summary.s0_val += (sign as i64) * val;
                summary.ordinary_leaves_count += 1;
                continue;
            }

            let next_p = primes[i + 1];
            if y < next_p * next_p {
                let pi_y = pi_table.pi(y) as i64;
                let term = (sign as i64) * (pi_y - (i as i64) + 1);
                summary.s1_val += term;
                summary.multi_factor_leaves_count += 1;
                continue;
            }

            // Left-spine collapse:
            let val = phi_tiny(y, 6) as i64;
            summary.s0_val += (sign as i64) * val;
            summary.ordinary_leaves_count += 1;

            for k in 7..=i {
                let y_div = magic.div(y, k);
                self.stack.push((y_div, k - 1, -sign));
            }
        }

        summary
    }
}
