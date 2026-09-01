//! EratMedium: mid-frequency prime crossing loops (S / 4 < p <= 8S).
//!
//! Multiples repeat 1 to 32 times per segment.
//! Driven by compact state, precomputed masks, and translation invariance (byte -= S).

use titan_core::wheel::{RESIDUES, RESIDUE_TO_BIT, WHEEL_INC, WHEEL_NEXT};

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MediumPrime {
    pub byte: u32,
    pub deltas: [u32; 8],
    pub masks: [u8; 8],
    pub p: u32,
    pub j: u8,
    pub row: u8,
    pub _pad: [u8; 2],
}

impl MediumPrime {
    pub fn new(p: u64, byte: usize, j: u8) -> Self {
        let row = RESIDUE_TO_BIT[(p % 30) as usize];
        let mut deltas = [0u32; 8];
        let mut masks = [0u8; 8];

        for step in 0..8 {
            let bit_idx = WHEEL_NEXT[row as usize][step];
            let res = RESIDUES[bit_idx as usize] as u64;
            let inc = WHEEL_INC[step] as u64;
            deltas[step] = ((res + p * inc) / 30) as u32;
            masks[step] = !(1 << bit_idx);
        }

        Self {
            byte: byte as u32,
            deltas,
            masks,
            p: p as u32,
            j,
            row,
            _pad: [0; 2],
        }
    }

    /// Cross off multiples within current segment.
    #[inline(always)]
    pub fn cross_off(&mut self, segment: &mut [u8]) {
        let seg_len = segment.len() as u32;
        let ptr = segment.as_mut_ptr();
        let mut cur_byte = self.byte;
        let mut cur_j = self.j as usize;

        while cur_byte < seg_len {
            unsafe {
                *ptr.add(cur_byte as usize) &= self.masks[cur_j];
            }
            cur_byte += self.deltas[cur_j];
            cur_j = (cur_j + 1) & 7;
        }

        self.byte = cur_byte;
        self.j = cur_j as u8;
    }
}
