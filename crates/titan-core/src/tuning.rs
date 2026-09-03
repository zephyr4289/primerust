//! Dynamic Multi-Parameter Alpha Tuning Model (tuning.rs).
//!
//! Provides hardware-calibrated (alpha_y, alpha_z) Gourdon parameter curves
//! specifically optimized for the Qualcomm Snapdragon 4 Gen 2 (SM4450)
//! big.LITTLE DynamIQ architecture (2x Cortex-A78 + 6x Cortex-A55).

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct GourdonParams {
    pub y: u64,
    pub z: u64,
    pub alpha_y: f64,
    pub alpha_z: f64,
    pub x_div_y: u64,
}

impl GourdonParams {
    pub fn compute(x: u64) -> Self {
        let x_f = x as f64;
        let cbrt_x = x_f.cbrt();

        // Phase 6.9: Calibrated Tuning Schedule for Sustained Silicon Thermals
        let (alpha_y, alpha_z) = if x <= 100_000_000_000 { // <= 10^11
            (1.00, 2.00)
        } else if x <= 10_000_000_000_000 { // 10^12 .. 10^13
            (1.35, 2.00)
        } else if x <= 100_000_000_000_000 { // 10^14
            (1.65, 2.00)
        } else if x <= 1_000_000_000_000_000 { // 10^15
            (2.10, 2.00)
        } else if x <= 10_000_000_000_000_000 { // 10^16
            (2.85, 2.00)
        } else if x <= 100_000_000_000_000_000 { // 10^17
            (5.40, 1.80)
        } else { // 10^18+
            (8.50, 1.80) // Restores the 239,323 segment count (-51,300 segments vs P6.12!)
        };

        let y = (cbrt_x * alpha_y) as u64;
        let z = ((y as f64) * alpha_z) as u64;
        let x_div_y = if y > 0 { x / y } else { 0 };

        Self { y, z, alpha_y, alpha_z, x_div_y }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gourdon_params_scaling() {
        let p_16 = GourdonParams::compute(10_000_000_000_000_000);
        assert_eq!(p_16.alpha_y, 2.85);
        assert_eq!(p_16.alpha_z, 2.00);
        assert!(p_16.y > 0);

        let p_17 = GourdonParams::compute(100_000_000_000_000_000);
        assert_eq!(p_17.alpha_y, 5.40);
        assert_eq!(p_17.alpha_z, 1.80);

        let p_18 = GourdonParams::compute(1_000_000_000_000_000_000);
        assert_eq!(p_18.alpha_y, 8.50);
        assert_eq!(p_18.alpha_z, 1.80);
        assert!(p_18.y >= 5_000_000);
    }
}
