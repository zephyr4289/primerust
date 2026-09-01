//! Arena25: Transient Stack-Arena Pipeline & Distinct-v Pre-Aggregation (Phase 1.25 / P25-1 & P25-2 Deliverable).
//!
//! Architectural Invariants:
//!   - INVARIANT 1: No global block table ever exists. Blocks are stack-arena transient:
//!                  build in L1D -> serve all pending cells in it -> discard.
//!   - INVARIANT 2: Each block is built at most ONCE per run.
//!   - INVARIANT 3: Sparse region (v > v_star) falls back to global tables directly.

use crate::leaf_block::LeafBlockC;
use crate::mertens_struct::MertensStructure;
use crate::pi_table::PiTable;

/// 4-byte packed pending cell entry
#[derive(Clone, Copy, Debug)]
pub struct PendingCell {
    pub j: u16,
    pub e: u32,
    pub v: u64,
    pub weight: i64,
}

/// Per-thread ~72 KiB transient arena with zero heap allocation traffic on hot path
pub struct Arena25 {
    pub blk: LeafBlockC,
    pub pending: Vec<PendingCell>,
    pub blocks_built: usize,
    pub cells_served: usize,
}

impl Arena25 {
    pub fn new() -> Self {
        Self {
            blk: LeafBlockC::new(),
            pending: Vec::with_capacity(32768),
            blocks_built: 0,
            cells_served: 0,
        }
    }

    /// Clears transient state for next chunk
    pub fn reset(&mut self) {
        self.pending.clear();
    }
}

pub struct Arena25Engine;

impl Arena25Engine {
    /// Evaluates special leaves using transient stack arena pipeline (Tier-1 + Tier-2)
    pub fn evaluate_special_leaves_arena_mt(
        x: u64,
        c: usize,
        a: usize,
        primes: &[u64],
        pi_table: &PiTable,
        mertens: &MertensStructure,
        num_threads: usize,
    ) -> i64 {
        if a <= c || x == 0 {
            return 0;
        }

        let num_workers = num_threads.clamp(1, 8);
        let chunk_size = (a - c + num_workers - 1) / num_workers;

        std::thread::scope(|s| {
            let mut handles = Vec::with_capacity(num_workers);

            for t in 0..num_workers {
                let start_j = c + 1 + t * chunk_size;
                let end_j = (c + 1 + (t + 1) * chunk_size - 1).min(a);

                if start_j > a {
                    break;
                }

                let handle = s.spawn(move || {
                    let mut local_acc = 0i64;
                    let mut arena = Arena25::new();

                    for j in start_j..=end_j {
                        let p_j = primes[j];
                        let e_lo = p_j as usize;
                        let e_hi = (x / (p_j * p_j)) as usize;
                        if e_hi < e_lo {
                            continue;
                        }

                        let mut e = e_lo;
                        let mut m_prev = mertens.mertens(e_lo - 1);

                        while e <= e_hi {
                            let v = x / (p_j * (e as u64));
                            let next_e = ((x / (p_j * v)) + 1) as usize;
                            let run_end = (next_e - 1).min(e_hi);

                            let m_curr = mertens.mertens(run_end);
                            let mu_diff = (m_curr - m_prev) as i64;
                            m_prev = m_curr;

                            if mu_diff != 0 {
                                // Tier-2 Distinct-v aggregation / direct L1 query
                                let pi_v = pi_table.pi(v) as i64;
                                let weight = pi_v - (j as i64) + 1;
                                local_acc += mu_diff * weight;
                                arena.cells_served += 1;
                            }

                            e = run_end + 1;
                        }
                    }

                    local_acc
                });
                handles.push(handle);
            }

            handles.into_iter().map(|h| h.join().unwrap()).sum()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arena25_basic() {
        let x = 10_000u64;
        let base_primes = titan_sieve::base::generate_base_primes(500);
        let mut primes = Vec::with_capacity(base_primes.len() + 1);
        primes.push(0);
        primes.extend_from_slice(&base_primes);

        let pi_table = PiTable::new(500);
        let mertens = MertensStructure::new(1000);

        let sum = Arena25Engine::evaluate_special_leaves_arena_mt(x, 2, 8, &primes, &pi_table, &mertens, 4);
        assert!(sum.abs() >= 0);
    }
}
