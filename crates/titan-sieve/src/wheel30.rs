//! Phase 6.2: Constants, Wheel Mapping, and State Compilation for Wheel-30 (wheel30.rs).
//!
//! Wheel-30 factors out multiples of 2, 3, and 5 before sieving begins,
//! tracking strictly the 8 coprime residues modulo 30:
//!   Residues = {1, 7, 11, 13, 17, 19, 23, 29}
//!
//! Buffer Geometry:
//!   - 1 byte represents 30 integers (8 bits <-> 8 coprime residues)
//!   - 16 KiB L1D segment contains 16,384 bytes = 131,072 bits
//!   - Segment span: 16,384 * 30 = 491,520 integers (1.875x wider cache horizon than Mod-2)

pub const WHEEL_RESIDUES: [u8; 8] = [1, 7, 11, 13, 17, 19, 23, 29];
pub const WHEEL_GAPS: [u8; 8] = [6, 4, 2, 4, 2, 4, 6, 2];

pub const SEGMENT_BYTES: usize = 16384; // 16 KiB = fits Cortex-A55 L1D
pub const SEGMENT_BITS: usize = SEGMENT_BYTES * 8; // 131,072 bits
pub const WHEEL_SPAN: u64 = (SEGMENT_BYTES as u64) * 30; // 491,520 integers

/// Maps residue mod 30 to bit index 0..7, or 0xFF if composite
pub const RESIDUE_TO_BIT: [u8; 30] = {
    let mut table = [0xFFu8; 30];
    table[1] = 0;
    table[7] = 1;
    table[11] = 2;
    table[13] = 3;
    table[17] = 4;
    table[19] = 5;
    table[23] = 6;
    table[29] = 7;
    table
};

pub const BIT_TO_RESIDUE: [u8; 8] = [1, 7, 11, 13, 17, 19, 23, 29];

#[repr(C, align(64))]
#[derive(Clone, Copy, Debug)]
pub struct Wheel30PrimeState {
    pub next_byte: u32,
    pub phase: u8,
    pub _pad: [u8; 3],
    pub adv_strip: u64,  // 8 packed byte advances for p <= 1200
    pub mask_strip: u64, // 8 packed clearing masks (1 << bit)
}

impl Wheel30PrimeState {
    /// Compiles a sieving prime p >= 7 into a dual-register rotational state
    /// starting at segment boundary `low` (which must be a multiple of 30).
    pub fn compile(p: u32, low: u64) -> Self {
        let p_u64 = p as u64;

        // Find first multiple >= low coprime to 30, starting at least from p^2
        let mut m = if low % p_u64 == 0 {
            low
        } else {
            low + (p_u64 - low % p_u64)
        };
        if m < p_u64 * p_u64 {
            m = p_u64 * p_u64;
        }

        let mut r = (m % 30) as usize;
        let mut k = (m / p_u64) % 30;

        // Advance to the first coprime multiple (at most 8 steps)
        while RESIDUE_TO_BIT[r] == 0xFF {
            m += p_u64;
            r = (m % 30) as usize;
            k = (m / p_u64) % 30;
        }

        let next_byte = ((m - low) / 30) as u32;

        // Build 8-step rotational masks and advances
        let mut mask_bytes = [0u8; 8];
        let mut adv_bytes = [0u8; 8];

        let mut curr_m = m;
        let mut k_idx = RESIDUE_TO_BIT[k as usize] as usize;

        for step in 0..8 {
            let res = (curr_m % 30) as usize;
            let bit = RESIDUE_TO_BIT[res];
            mask_bytes[step] = 1u8 << bit;

            let gap = WHEEL_GAPS[k_idx] as u64;
            let next_m = curr_m + p_u64 * gap;
            let byte_adv = ((next_m / 30) - (curr_m / 30)) as u8;
            adv_bytes[step] = byte_adv;

            curr_m = next_m;
            k_idx = (k_idx + 1) & 7;
        }

        Self {
            next_byte,
            phase: 0,
            _pad: [0; 3],
            adv_strip: u64::from_le_bytes(adv_bytes),
            mask_strip: u64::from_le_bytes(mask_bytes),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wheel_constants() {
        assert_eq!(SEGMENT_BYTES, 16384);
        assert_eq!(SEGMENT_BITS, 131072);
        assert_eq!(WHEEL_SPAN, 491520);
        assert_eq!(WHEEL_RESIDUES.len(), 8);
        assert_eq!(WHEEL_GAPS.len(), 8);
        assert_eq!(WHEEL_GAPS.iter().map(|&g| g as u64).sum::<u64>(), 30);
    }

    #[test]
    fn test_residue_mapping_bijection() {
        for (bit, &res) in BIT_TO_RESIDUE.iter().enumerate() {
            assert_eq!(RESIDUE_TO_BIT[res as usize], bit as u8);
        }
        for r in 0..30 {
            if !WHEEL_RESIDUES.contains(&(r as u8)) {
                assert_eq!(RESIDUE_TO_BIT[r], 0xFF);
            }
        }
    }

    #[test]
    fn test_wheel30_state_compile() {
        let state = Wheel30PrimeState::compile(7, 0);
        // 7^2 = 49. 49 % 30 = 19. 19 is coprime to 30.
        // Byte: 49 / 30 = 1.
        assert_eq!(state.next_byte, 1);
        assert_eq!(state.phase, 0);

        let adv = state.adv_strip.to_le_bytes();
        let total_adv: u32 = adv.iter().map(|&b| b as u32).sum();
        // Over 8 wheel steps, the advance in byte coordinate must be exactly p = 7!
        assert_eq!(total_adv, 7);
    }
}
