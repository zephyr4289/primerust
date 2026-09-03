//! Phase 6.13: Asymmetric Wheel-210 Engine (wheel210.rs).
//!
//! Tracks the 48 residues coprime to 210 (phi(210) = 48), pre-filtering
//! multiples of 2, 3, 5, and 7. Density drops to 22.86% (vs 26.67% for Wheel-30),
//! slashing marking instructions by -14.3% on Cortex-A78 cores with 64 KiB L1I.

pub const WHEEL210_MODULO: u64 = 210;
pub const WHEEL210_RESIDUES_COUNT: usize = 48;

pub const RESIDUES_210: [u8; 48] = [
    1, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83, 89, 97,
    101, 103, 107, 109, 113, 121, 127, 131, 137, 139, 143, 149, 151, 157, 163, 167, 169,
    173, 179, 181, 187, 191, 193, 197, 199, 209,
];

/// Maps residue mod 210 to coprime index (0..47), or 0xFF if composite
pub const RESIDUE_210_TO_INDEX: [u8; 210] = {
    let mut table = [0xFFu8; 210];
    let mut i = 0;
    while i < 48 {
        table[RESIDUES_210[i] as usize] = i as u8;
        i += 1;
    }
    table
};

/// 48 residue differences mod 210 (Gaps sum to 210)
pub const WHEEL210_GAPS: [u8; 48] = {
    let mut gaps = [0u8; 48];
    let mut i = 0;
    while i < 47 {
        gaps[i] = RESIDUES_210[i + 1] - RESIDUES_210[i];
        i += 1;
    }
    gaps[47] = (210 + RESIDUES_210[0]) - RESIDUES_210[47];
    gaps
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wheel210_gaps_sum() {
        let sum: u32 = WHEEL210_GAPS.iter().map(|&g| g as u32).sum();
        assert_eq!(sum, 210);
    }

    #[test]
    fn test_wheel210_residues_coprime() {
        for &r in &RESIDUES_210 {
            assert!(r % 2 != 0);
            assert!(r % 3 != 0);
            assert!(r % 5 != 0);
            assert!(r % 7 != 0);
            assert_eq!(RESIDUE_210_TO_INDEX[r as usize], RESIDUES_210.iter().position(|&x| x == r).unwrap() as u8);
        }
    }
}
