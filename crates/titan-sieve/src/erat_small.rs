//! EratSmall: high-frequency prime crossing loops (p <= S / 4).
//!
//! Driven by register-resident 8-cycle unrolled loops with raw pointer writes.

use titan_core::wheel::{RESIDUES, RESIDUE_TO_BIT, WHEEL_INC, WHEEL_NEXT};

#[derive(Clone, Debug)]
pub struct SmallPrime {
    pub p: usize,
    pub byte: usize,
    pub j: u8,
    pub row: u8,
    pub deltas: [usize; 8],
    pub masks: [u8; 8],
}

impl SmallPrime {
    pub fn new(p: u64, byte: usize, j: u8) -> Self {
        let row = RESIDUE_TO_BIT[(p % 30) as usize];
        let mut deltas = [0usize; 8];
        let mut masks = [0u8; 8];

        for step in 0..8 {
            let bit_idx = WHEEL_NEXT[row as usize][step];
            let res = RESIDUES[bit_idx as usize] as u64;
            let inc = WHEEL_INC[step] as u64;
            deltas[step] = ((res + p * inc) / 30) as usize;
            masks[step] = !(1 << bit_idx);
        }

        Self {
            p: p as usize,
            byte,
            j,
            row,
            deltas,
            masks,
        }
    }

    /// Cross off all multiples of this small prime within the segment buffer.
    #[inline(always)]
    pub fn cross_off(&mut self, segment: &mut [u8]) {
        let seg_len = segment.len();
        let ptr = segment.as_mut_ptr();

        let mut cur_byte = self.byte;
        let mut cur_j = self.j as usize;
        let p = self.p;

        // Scalar single-step head until cur_j is aligned to 0
        while cur_j != 0 && cur_byte < seg_len {
            unsafe {
                *ptr.add(cur_byte) &= self.masks[cur_j];
            }
            cur_byte += self.deltas[cur_j];
            cur_j = (cur_j + 1) & 7;
        }

        // Unrolled 8-cycle loop (advances exactly p bytes per 8 steps)
        let (m0, m1, m2, m3, m4, m5, m6, m7) = (
            self.masks[0], self.masks[1], self.masks[2], self.masks[3],
            self.masks[4], self.masks[5], self.masks[6], self.masks[7],
        );
        let (d0, d1, d2, d3, d4, d5, d6, d7) = (
            self.deltas[0], self.deltas[1], self.deltas[2], self.deltas[3],
            self.deltas[4], self.deltas[5], self.deltas[6], self.deltas[7],
        );

        while cur_byte + p <= seg_len {
            unsafe {
                *ptr.add(cur_byte) &= m0;
                cur_byte += d0;
                *ptr.add(cur_byte) &= m1;
                cur_byte += d1;
                *ptr.add(cur_byte) &= m2;
                cur_byte += d2;
                *ptr.add(cur_byte) &= m3;
                cur_byte += d3;
                *ptr.add(cur_byte) &= m4;
                cur_byte += d4;
                *ptr.add(cur_byte) &= m5;
                cur_byte += d5;
                *ptr.add(cur_byte) &= m6;
                cur_byte += d6;
                *ptr.add(cur_byte) &= m7;
                cur_byte += d7;
            }
        }

        // Tail loop
        while cur_byte < seg_len {
            unsafe {
                *ptr.add(cur_byte) &= self.masks[cur_j];
            }
            cur_byte += self.deltas[cur_j];
            cur_j = (cur_j + 1) & 7;
        }

        self.byte = cur_byte;
        self.j = cur_j as u8;
    }
}
