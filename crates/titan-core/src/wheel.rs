//! Wheel-30 Factorization: Convention A single source of truth and Phase 30 state machine.
//!
//! Convention A:
//!   - Byte k covers integers [30k, 30k + 29].
//!   - Bit i corresponds to residue UNITS[i], where
//!     UNITS = [1, 7, 11, 13, 17, 19, 23, 29] in ascending order.
//!   - Exactly 8 bits per byte, representing 30 consecutive integers.

pub const UNITS: [u8; 8] = [1, 7, 11, 13, 17, 19, 23, 29];
pub const RESIDUES: [u8; 8] = UNITS;
pub const GAP30: [u8; 8] = [6, 4, 2, 4, 2, 4, 6, 2];
pub const WHEEL_INC: [u8; 8] = GAP30;
pub const NON_COPRIME: u8 = 8;

pub const HIGH_MASK: [u8; 8] = [
    0b0000_0001,
    0b0000_0011,
    0b0000_0111,
    0b0000_1111,
    0b0001_1111,
    0b0011_1111,
    0b0111_1111,
    0b1111_1111,
];

/// R30[r] = slot of residue r if unit, else 8 (sentinel).
const fn build_r30() -> [u8; 30] {
    let mut r = [8u8; 30];
    let mut s = 0;
    while s < 8 {
        r[UNITS[s] as usize] = s as u8;
        s += 1;
    }
    r
}
pub const R30: [u8; 30] = build_r30();
pub const RESIDUE_TO_BIT: [u8; 30] = R30;

#[inline(always)]
pub const fn cand_idx(n: u64) -> u64 {
    8 * (n / 30) + R30[(n % 30) as usize] as u64
}

/// WHEEL_ROT[i][s][j]: j-th bit-index delta for prime p = UNITS[i] (mod 30),
/// first multiple's cofactor = UNITS[s] (mod 30). 8*8*8*u32 = 2 KiB rodata.
pub const WHEEL_ROT: [[[u32; 8]; 8]; 8] = build_wheel_rot();

const fn build_wheel_rot() -> [[[u32; 8]; 8]; 8] {
    let mut w = [[[0u32; 8]; 8]; 8];
    let mut i = 0;
    while i < 8 {
        let p = UNITS[i] as u64;
        let mut s = 0;
        while s < 8 {
            let mut k = UNITS[s] as u64;
            let mut j = 0;
            while j < 8 {
                let g = GAP30[R30[(k % 30) as usize] as usize] as u64;
                w[i][s][j] = (cand_idx(p * (k + g)) - cand_idx(p * k)) as u32;
                k += g;
                j += 1;
            }
            s += 1;
        }
        i += 1;
    }
    w
}

/// Compile-time proofs - the load-bearing theorems enforced by rustc:
const _: () = {
    assert!(GAP30[0] + GAP30[1] + GAP30[2] + GAP30[3] + GAP30[4] + GAP30[5] + GAP30[6] + GAP30[7] == 30, "unit cycle must close");
    let mut i = 0;
    while i < 8 {
        let mut s = 0;
        while s < 8 {
            let mut sum = 0u32;
            let mut j = 0;
            while j < 8 {
                if i > 0 {
                    assert!(WHEEL_ROT[i][s][j] >= 3, "min-delta theorem (Lemma §2)");
                }
                sum += WHEEL_ROT[i][s][j];
                j += 1;
            }
            assert!(sum == 8 * UNITS[i] as u32, "periodicity theorem");
            s += 1;
        }
        i += 1;
    }
};

/// SKIP[r]: 0 if r is a unit, else distance to the next unit (<= 5).
const fn build_skip() -> [u8; 30] {
    let mut t = [1u8; 30];
    let mut r = 0;
    while r < 30 {
        let mut d = 1u8;
        while R30[((r + d as usize) % 30) as usize] == 8 {
            d += 1;
        }
        t[r] = d;
        r += 1;
    }
    let mut s = 0;
    while s < 8 {
        t[UNITS[s] as usize] = 0;
        s += 1;
    }
    t
}
pub const SKIP: [u8; 30] = build_skip();

/// MASK_LE[r]: bits of the candidate slots with unit value <= r (both cases).
const fn build_mask_le() -> [u8; 30] {
    let mut m = [0u8; 30];
    let mut r = 0;
    while r < 30 {
        let mut s = 0;
        while s < 8 {
            if UNITS[s] as usize <= r {
                m[r] |= 1 << s;
            }
            s += 1;
        }
        r += 1;
    }
    m
}
pub const MASK_LE: [u8; 30] = build_mask_le();

/// PREV[r]: r minus largest unit <= r (for boundary n that is not a unit).
const fn build_prev() -> [u8; 30] {
    let mut t = [0u8; 30];
    let mut r = 0;
    while r < 30 {
        let mut d = 0u8;
        while R30[(r - d as usize) as usize] == 8 && d < r as u8 {
            d += 1;
        }
        t[r] = d;
        r += 1;
    }
    t
}
pub const PREV: [u8; 30] = build_prev();

pub const NEXT_COPRIME: [u8; 30] = {
    let mut table = [0u8; 30];
    let mut r = 0;
    while r < 30 {
        let mut found = 29;
        let mut i = 0;
        while i < 8 {
            if RESIDUES[i] >= r as u8 {
                found = RESIDUES[i];
                break;
            }
            i += 1;
        }
        table[r] = found;
        r += 1;
    }
    table
};

/// Permutation table for stepping wheel multiples:
pub const WHEEL_NEXT: [[u8; 8]; 8] = {
    let mut table = [[0u8; 8]; 8];
    let mut i = 0;
    while i < 8 {
        let p = RESIDUES[i] as usize;
        let mut j = 0;
        while j < 8 {
            let m = RESIDUES[j] as usize;
            let prod_rem = (p * m) % 30;
            table[i][j] = RESIDUE_TO_BIT[prod_rem];
            j += 1;
        }
        i += 1;
    }
    table
};

#[inline(always)]
pub const fn is_coprime_30(n: u64) -> bool {
    R30[(n % 30) as usize] != 8
}

#[inline(always)]
pub const fn byte_and_bit_index(n: u64) -> (usize, usize) {
    let byte_idx = (n / 30) as usize;
    let bit_idx = R30[(n % 30) as usize] as usize;
    (byte_idx, bit_idx)
}

#[inline(always)]
pub const fn int_from_byte_and_bit(byte_idx: usize, bit_idx: usize) -> u64 {
    30 * (byte_idx as u64) + UNITS[bit_idx] as u64
}

#[inline(always)]
pub const fn slot_to_number(byte_idx: usize, bit_idx: usize) -> u64 {
    30 * (byte_idx as u64) + UNITS[bit_idx] as u64
}

#[inline(always)]
pub fn number_to_slot(n: u64) -> Option<(usize, usize)> {
    let r = (n % 30) as usize;
    let bit = R30[r];
    if bit != 8 {
        Some(((n / 30) as usize, bit as usize))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wheel_units_and_periodicity() {
        assert_eq!(GAP30.iter().sum::<u8>(), 30);
        for i in 0..8 {
            for s in 0..8 {
                let sum: u32 = WHEEL_ROT[i][s].iter().sum();
                assert_eq!(sum, 8 * UNITS[i] as u32);
                if i > 0 {
                    for &d in &WHEEL_ROT[i][s] {
                        assert!(d >= 3);
                    }
                }
            }
        }
    }

    #[test]
    fn test_cand_idx_monotone() {
        let mut prev = 0u64;
        for k in 0..100 {
            for &u in &UNITS {
                let n = 30 * k + u as u64;
                let idx = cand_idx(n);
                if n > 1 {
                    assert_eq!(idx, prev + 1);
                }
                prev = idx;
            }
        }
    }
}
