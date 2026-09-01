//! Phi: Combinatorial sifting function Phi(x, a) with Magic Division and Left-Spine Collapse.
//!
//! Evaluates:
//!   Phi(x, a) = #{ n <= x : gcd(n, p_1 * ... * p_a) = 1 }
//!
//! Features:
//!   - Left-Spine Collapse: chains (y, i) -> (y, 6) collapse directly to phi_tiny(y, 6)
//!   - Magic Division: 3-cycle division by constant primes via umulh + lsr
//!   - Zero recursion, bounded explicit DFS stack

use crate::magic::MagicDivTable;
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
    pub magic_table: Option<MagicDivTable>,
}

impl PhiEngine {
    pub fn new() -> Self {
        Self {
            stack: Vec::with_capacity(4096),
            census: PhiCensus::default(),
            magic_table: None,
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
        if self.magic_table.as_ref().map_or(true, |m| m.len() < primes.len()) {
            self.magic_table = Some(MagicDivTable::new(primes));
        }
        let magic = self.magic_table.as_ref().unwrap();

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

            if i <= 6 {
                self.census.t1_exits += 1;
                let val = phi_tiny(y, i as u64) as i64;
                total += (sign as i64) * val;
                continue;
            }

            let next_p = primes[i + 1];
            if y < next_p * next_p {
                self.census.t2_exits += 1;
                let pi_y = pi_table.pi(y) as i64;
                total += (sign as i64) * (pi_y - (i as i64) + 1);
                continue;
            }

            // Left-spine collapse:
            // Since y >= p_{i+1}^2, y >= p_k^2 for all k <= i down to 7.
            // Directly evaluate leaf (y, 6) via phi_tiny
            self.census.t1_exits += 1;
            let val = phi_tiny(y, 6) as i64;
            total += (sign as i64) * val;

            // Push all right children along the collapsed spine
            for k in 7..=i {
                let y_div = magic.div(y, k);
                self.stack.push((y_div, k - 1, -sign));
            }
        }

        total
    }

    /// Fast evaluation with Left-Spine Collapse and Magic Division
    #[inline(always)]
    pub fn eval(
        &mut self,
        x: u64,
        a: usize,
        primes: &[u64],
        pi_table: &PiTable,
    ) -> i64 {
        if self.magic_table.as_ref().map_or(true, |m| m.len() < primes.len()) {
            self.magic_table = Some(MagicDivTable::new(primes));
        }
        let magic = self.magic_table.as_ref().unwrap();

        self.stack.clear();
        if x == 0 || a == 0 {
            return x as i64;
        }

        self.stack.push((x, a, 1));
        let mut total = 0i64;

        while let Some((y, i, sign)) = self.stack.pop() {
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

            // Left-spine collapse:
            let val = phi_tiny(y, 6) as i64;
            total += (sign as i64) * val;

            for k in 7..=i {
                let y_div = magic.div(y, k);
                self.stack.push((y_div, k - 1, -sign));
            }
        }

        total
    }
}

/// Multi-threaded evaluation of Phi(x, a) using Spine-Split subtree dispatch
pub fn eval_mt(
    x: u64,
    a: usize,
    primes: &[u64],
    pi_table: &PiTable,
    num_threads: usize,
) -> i64 {
    if x == 0 || a == 0 {
        return x as i64;
    }
    if a <= 6 {
        return phi_tiny(x, a as u64) as i64;
    }
    if num_threads <= 1 || a < 20 {
        let mut eng = PhiEngine::new();
        return eng.eval(x, a, primes, pi_table);
    }

    let magic = MagicDivTable::new(primes);

    // 1. Direct evaluation of root left spine (x, 6):
    let root_val = phi_tiny(x, 6) as i64;

    // 2. Right children subtrees along the left spine:
    let mut subtrees: Vec<(u64, usize)> = Vec::with_capacity(a);
    for k in (7..=a).rev() {
        let y_div = magic.div(x, k);
        subtrees.push((y_div, k - 1));
    }

    // 3. Dynamic work distribution
    use std::sync::atomic::{AtomicUsize, Ordering};
    let next_task = AtomicUsize::new(0);
    let total_subtrees = subtrees.len();

    let mut thread_sums = vec![0i64; num_threads];

    std::thread::scope(|s| {
        for sum_ref in thread_sums.iter_mut() {
            let next_task_ref = &next_task;
            let subtrees_ref = &subtrees;
            s.spawn(move || {
                let mut eng = PhiEngine::new();
                let mut local_sum = 0i64;

                loop {
                    let idx = next_task_ref.fetch_add(1, Ordering::Relaxed);
                    if idx >= total_subtrees {
                        break;
                    }
                    let (y, k) = subtrees_ref[idx];
                    let sub_val = eng.eval(y, k, primes, pi_table);
                    local_sum -= sub_val; // Right-children have sign = -1
                }

                *sum_ref = local_sum;
            });
        }
    });

    let mut total = root_val;
    for s in thread_sums {
        total += s;
    }

    total
}

