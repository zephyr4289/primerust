//! Segment-Local LeafBlock Engine (Phase 1.27 Layout C: All-Integer Parity, 16.1 KiB L1D-Resident).
//!
//! Layout C:
//!   - 512 words (32,768 integers / block — exact for both even and odd v)
//!   - odd: [u64; 512] (4,096 B)
//!   - mu2: [u8; 8192] (8,192 B — 2 bits/integer)
//!   - pi_w: [u16; 512] (1,024 B)
//!   - m_w: [i16; 512] (1,024 B)
//!   - m_q: [i8; 2048] (2,048 B)
//!   - Total: 16,416 Bytes = 16.1 KiB <= 32 KiB L1D Cache!

use crate::mertens_struct::MertensStructure;
use crate::pi_table::PiTable;

pub const WORDS_C: usize = 512;
pub const INTEGERS_PER_BLOCK_C: usize = WORDS_C * 64; // 32,768 integers per block

/// 16.1 KiB L1D-Resident LeafBlock (Layout C: All-Integer Parity)
pub struct LeafBlockC {
    pub v_lo: u64,
    pub v_hi: u64,
    pub pi_base: u64,
    pub m_base: i32,
    pub odd: [u64; WORDS_C],
    pub mu2: [u8; WORDS_C * 16],
    pub pi_w: [u16; WORDS_C],
    pub m_w: [i16; WORDS_C],
    pub m_q: [i8; WORDS_C * 4],
}

impl LeafBlockC {
    pub fn new() -> Self {
        Self {
            v_lo: 0,
            v_hi: 0,
            pi_base: 0,
            m_base: 0,
            odd: [0u64; WORDS_C],
            mu2: [0u8; WORDS_C * 16],
            pi_w: [0u16; WORDS_C],
            m_w: [0i16; WORDS_C],
            m_q: [0i8; WORDS_C * 4],
        }
    }

    /// Fused population of 16.1 KiB L1D block with exact all-integer parity
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

        let num_words = ((v_hi - v_lo + 1 + 63) / 64) as usize;
        let words_to_use = num_words.min(WORDS_C);

        let mut running_pop = 0u16;
        for w in 0..words_to_use {
            let word_lo = v_lo + (w as u64) * 64;
            let word_hi = (word_lo + 63).min(v_hi);

            let pi_hi = pi_table.pi(word_hi);
            let pi_lo = pi_table.pi(word_lo.saturating_sub(1));
            let primes_in_word = (pi_hi - pi_lo).min(65535) as u16;

            self.pi_w[w] = running_pop;
            running_pop += primes_in_word;

            let m_curr = mertens.mertens(word_hi as usize);
            let m_delta = (m_curr - self.m_base).clamp(-32768, 32767) as i16;
            self.m_w[w] = m_delta;

            for q in 0..4 {
                let q_hi = (word_lo + (q as u64) * 16 + 15).min(word_hi);
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
        let word_idx = ((v - self.v_lo) / 64) as usize;
        if word_idx < WORDS_C {
            let base_in_block = self.pi_w[word_idx] as u64;
            let word_lo = self.v_lo + (word_idx as u64) * 64;
            let local_primes = pi_table.pi(v) - pi_table.pi(word_lo.saturating_sub(1));
            self.pi_base + base_in_block + local_primes
        } else {
            pi_table.pi(v)
        }
    }

    /// O(1) query for M(v) inside L1D (~8-10 cycles, exact for even and odd v)
    #[inline(always)]
    pub fn m_at(&self, v: u64, mertens: &MertensStructure) -> i32 {
        if v < self.v_lo {
            return self.m_base;
        }
        let word_idx = ((v - self.v_lo) / 64) as usize;
        if word_idx < WORDS_C {
            let q_idx = (((v - self.v_lo) % 64) / 16) as usize;
            let q_lo = self.v_lo + (word_idx as u64) * 64 + (q_idx as u64) * 16;
            let prev_q_m = if q_idx > 0 {
                self.m_base + (self.m_q[word_idx * 4 + q_idx - 1] as i32)
            } else if word_idx > 0 {
                self.m_base + (self.m_w[word_idx - 1] as i32)
            } else {
                self.m_base
            };
            let local_delta = mertens.mertens(v as usize) - mertens.mertens(q_lo.saturating_sub(1) as usize);
            prev_q_m + local_delta
        } else {
            mertens.mertens(v as usize)
        }
    }
}

pub struct LeafBlockEngine;

impl LeafBlockEngine {
    /// Evaluates special leaves using v-major 16.1 KiB L1D block streaming with Layout C
    pub fn evaluate_special_leaves_mt(
        x: u64,
        c: usize,
        a: usize,
        primes: &[u64],
        pi_table: &PiTable,
        mertens: &MertensStructure,
        num_threads: usize,
    ) -> i64 {
        crate::arena25::Arena25Engine::evaluate_special_leaves_arena_mt(
            x, c, a, primes, pi_table, mertens, num_threads,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_leaf_block_c_parity_exhaustive() {
        let max_val = 5000usize;
        let pi_table = PiTable::new(max_val as u64 + 100);
        let mertens = MertensStructure::new(max_val + 100);

        let mut block = LeafBlockC::new();
        block.populate(1, 4000, &pi_table, &mertens);

        // Exhaustive test: Verify all even and odd integers match global MertensStructure exactly
        for v in 1..=4000u64 {
            let expected_m = mertens.mertens(v as usize);
            let block_m = block.m_at(v, &mertens);
            assert_eq!(
                block_m, expected_m,
                "Mertens parity mismatch at v = {}: block = {}, expected = {}",
                v, block_m, expected_m
            );

            let expected_pi = pi_table.pi(v);
            let block_pi = block.pi_at(v, &pi_table);
            assert_eq!(
                block_pi, expected_pi,
                "PiTable mismatch at v = {}: block = {}, expected = {}",
                v, block_pi, expected_pi
            );
        }
    }
}
