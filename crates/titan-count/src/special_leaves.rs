//! Special Leaves (S2') evaluation for LMO / Gourdon-class engine.
//!
//! Evaluates the special leaf sum:
//!   S2'(x, a) = sum_{j=k+1}^a sum_{m} mu(m) * [ pi(floor(x / (m * p_j))) - j + 1 ]
//! where m is squarefree with least prime factor P^-(m) > p_j.

use crate::magic::MagicDivTable;
use crate::pi_table::PiTable;

#[derive(Default, Debug, Clone)]
pub struct SpecialLeafSummary {
    pub s2_prime_val: i64,
    pub special_leaves_count: u64,
}

pub struct SpecialLeafEngine {
    stack: Vec<(u64, usize, i8)>,
}

impl SpecialLeafEngine {
    pub fn new() -> Self {
        Self {
            stack: Vec::with_capacity(4096),
        }
    }

    /// Evaluates special leaves S2' using the prefix pi-table
    pub fn eval_special_leaves(
        &mut self,
        x: u64,
        a: usize,
        primes: &[u64],
        pi_table: &PiTable,
    ) -> SpecialLeafSummary {
        let magic = MagicDivTable::new(primes);
        self.stack.clear();
        let mut summary = SpecialLeafSummary::default();

        if a <= 6 || x == 0 {
            return summary;
        }

        // For each attachment level j from 7..=a:
        for j in 7..=a {
            let p_j = primes[j];
            let y = x / p_j;
            if y < p_j {
                continue;
            }

            // Subtree starting at (y, j-1) with sign -1
            self.stack.push((y, j - 1, -1));

            while let Some((cur_y, i, sign)) = self.stack.pop() {
                if i <= 6 {
                    continue;
                }

                let next_p = primes[i + 1];
                if cur_y < next_p * next_p {
                    if cur_y <= pi_table.max_y {
                        let pi_y = pi_table.pi(cur_y) as i64;
                        let term = (sign as i64) * (pi_y - (i as i64) + 1);
                        summary.s2_prime_val += term;
                        summary.special_leaves_count += 1;
                    }
                    continue;
                }

                for k in 7..=i {
                    let next_y = magic.div(cur_y, k);
                    if next_y >= primes[k] {
                        self.stack.push((next_y, k - 1, -sign));
                    }
                }
            }
        }

        summary
    }
}
