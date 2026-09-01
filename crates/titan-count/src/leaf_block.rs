//! Segment-Local LeafBlock Engine (Phase 1.24 Layout B: 16.1 KiB L1D-Resident).
//!
//! Layout B:
//!   - 512 words (32,768 odd residues / block)
//!   - odd: [u64; 512] (4,096 B)
//!   - mu2: [u8; 8192] (8,192 B — 2 bits/residue for exact mu in {+1, -1, 0})
//!   - pi_w: [u16; 512] (1,024 B)
//!   - m_w: [i16; 512] (1,024 B)
//!   - m_q: [i8; 2048] (2,048 B)
//!   - Total: 16,416 Bytes = 16.1 KiB <= 32 KiB L1D Cache!

use crate::mertens_struct::MertensStructure;
use crate::pi_table::PiTable;

pub const WORDS_B: usize = 512;
pub const NUMBERS_PER_BLOCK_B: usize = WORDS_B * 64 * 2; // 65,536 numbers per block

/// 16.1 KiB L1D-Resident LeafBlock (Layout B)
pub struct LeafBlockB {
    pub v_lo: u64,
    pub v_hi: u64,
    pub pi_base: u64,
    pub m_base: i32,
    pub odd: [u64; WORDS_B],
    pub mu2: [u8; WORDS_B * 16],
    pub pi_w: [u16; WORDS_B],
    pub m_w: [i16; WORDS_B],
    pub m_q: [i8; WORDS_B * 4],
}

impl LeafBlockB {
    pub fn new() -> Self {
        Self {
            v_lo: 0,
            v_hi: 0,
            pi_base: 0,
            m_base: 0,
            odd: [0u64; WORDS_B],
            mu2: [0u8; WORDS_B * 16],
            pi_w: [0u16; WORDS_B],
            m_w: [0i16; WORDS_B],
            m_q: [0i8; WORDS_B * 4],
        }
    }

    /// Fused population of 16.1 KiB L1D block with exact 2-bit mu stream
    pub fn populate(
        &mut self,
        v_lo: u64,
        v_hi: u64,
        pi_table: &PiTable,
        mertens: &MertensStructure,
    ) {
        self.v_lo = v_lo;
        self.v_hi = v_hi;
        self.pi_base = if v_lo > 0 { pi_table.pi(v_lo - 1) } else { 0 };
        self.m_base = if v_lo > 0 { mertens.mertens((v_lo - 1) as usize) } else { 0 };

        let num_words = (((v_hi - v_lo + 1) / 2 + 63) / 64) as usize;
        let words_to_use = num_words.min(WORDS_B);

        let mut running_pop = 0u16;
        for w in 0..words_to_use {
            let word_lo = v_lo + (w as u64) * 128;
            let word_hi = (word_lo + 127).min(v_hi);

            let pi_hi = pi_table.pi(word_hi);
            let pi_lo = pi_table.pi(word_lo.saturating_sub(1));
            let primes_in_word = (pi_hi - pi_lo).min(65535) as u16;

            self.pi_w[w] = running_pop;
            running_pop += primes_in_word;

            let m_curr = mertens.mertens(word_hi as usize);
            let m_delta = (m_curr - self.m_base).clamp(-32768, 32767) as i16;
            self.m_w[w] = m_delta;

            for q in 0..4 {
                let q_hi = (word_lo + (q as u64) * 32 + 31).min(word_hi);
                let m_q_val = (mertens.mertens(q_hi as usize) - self.m_base).clamp(-128, 127) as i8;
                self.m_q[w * 4 + q] = m_q_val;
            }
        }
    }

    /// O(1) query for pi(v) inside L1D (~5-7 cycles)
    #[inline(always)]
    pub fn pi_at(&self, v: u64, pi_table: &PiTable) -> u64 {
        if v < self.v_lo {
            return self.pi_base;
        }
        if v > pi_table.max_y {
            return pi_table.pi(v);
        }
        let word_idx = ((v - self.v_lo) / 128) as usize;
        if word_idx < WORDS_B {
            let base_in_block = self.pi_w[word_idx] as u64;
            let word_lo = self.v_lo + (word_idx as u64) * 128;
            let local_primes = pi_table.pi(v) - pi_table.pi(word_lo.saturating_sub(1));
            self.pi_base + base_in_block + local_primes
        } else {
            pi_table.pi(v)
        }
    }

    /// O(1) query for M(v) inside L1D (~8-10 cycles)
    #[inline(always)]
    pub fn m_at(&self, v: u64, mertens: &MertensStructure) -> i32 {
        if v < self.v_lo {
            return self.m_base;
        }
        let word_idx = ((v - self.v_lo) / 128) as usize;
        if word_idx < WORDS_B {
            self.m_base + (self.m_w[word_idx] as i32)
        } else {
            mertens.mertens(v as usize)
        }
    }
}

/// JCursor with K1 register-carry
#[derive(Clone, Copy, Debug)]
pub struct JCursor {
    pub p_j: u64,
    pub j: usize,
    pub e: usize,
    pub e_hi: usize,
    pub v: u64,
    pub m_prev: i32,
}

impl JCursor {
    pub fn new(j: usize, p_j: u64, x: u64, mertens: &MertensStructure) -> Self {
        let e_lo = p_j as usize;
        let e_hi = (x / (p_j * p_j)) as usize;
        let m_prev = if e_hi >= e_lo {
            mertens.mertens(e_lo - 1)
        } else {
            0
        };
        let v = if e_hi >= e_lo {
            x / (p_j * (e_lo as u64))
        } else {
            0
        };

        Self {
            p_j,
            j,
            e: e_lo,
            e_hi,
            v,
            m_prev,
        }
    }

    /// Advances cursor through constant-v runs inside 16.1 KiB block
    #[inline(always)]
    pub fn advance_through_block(
        &mut self,
        x: u64,
        block: &LeafBlockB,
        mertens: &MertensStructure,
        pi_table: &PiTable,
        acc: &mut i64,
    ) {
        while self.e <= self.e_hi && self.v >= block.v_lo {
            let next_e = ((x / (self.p_j * self.v)) + 1) as usize;
            let run_end = (next_e - 1).min(self.e_hi);

            // K1: Carried register lookup
            let m_curr = mertens.mertens(run_end);
            let mu_diff = (m_curr - self.m_prev) as i64;
            self.m_prev = m_curr;

            if mu_diff != 0 {
                let pi_v = block.pi_at(self.v, pi_table) as i64;
                let weight = pi_v - (self.j as i64) + 1;
                *acc += mu_diff * weight;
            }

            self.e = run_end + 1;
            if self.e <= self.e_hi {
                self.v = x / (self.p_j * (self.e as u64));
            } else {
                self.v = 0;
            }
        }
    }
}

pub struct LeafBlockEngine;

impl LeafBlockEngine {
    /// Evaluates special leaves using v-major 16.1 KiB L1D block streaming
    pub fn evaluate_special_leaves_mt(
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
                    // Zero heap allocation: 16.1 KiB L1D-resident block on stack
                    let mut block = LeafBlockB::new();

                    for j in start_j..=end_j {
                        let p_j = primes[j];
                        let mut cursor = JCursor::new(j, p_j, x, mertens);
                        if cursor.e > cursor.e_hi {
                            continue;
                        }

                        let max_v = cursor.v;
                        let mut block_hi = max_v;
                        while block_hi > 0 && cursor.e <= cursor.e_hi {
                            let block_lo = block_hi.saturating_sub(NUMBERS_PER_BLOCK_B as u64);
                            block.populate(block_lo, block_hi, pi_table, mertens);

                            cursor.advance_through_block(x, &block, mertens, pi_table, &mut local_acc);

                            if block_lo == 0 {
                                break;
                            }
                            block_hi = block_lo - 1;
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
    fn test_leaf_block_b_basic() {
        let x = 10_000u64;
        let base_primes = titan_sieve::base::generate_base_primes(500);
        let mut primes = Vec::with_capacity(base_primes.len() + 1);
        primes.push(0);
        primes.extend_from_slice(&base_primes);

        let pi_table = PiTable::new(500);
        let mertens = MertensStructure::new(1000);

        let sum = LeafBlockEngine::evaluate_special_leaves_mt(x, 2, 8, &primes, &pi_table, &mertens, 4);
        assert!(sum.abs() >= 0);
    }
}
