//! Scale-Indexed Parameter Dispatch & Device Tuning (Phase 1.28 & Phase 1.42 Deliverable).
//!
//! Provides calibrated dynamic parameter dials for Xavier Gourdon:
//!   - Dynamic alpha(x) = alpha_0 * (1 + k / ln(x)) with alpha_0 = 1.15, k = 2.0
//!   - Optimal beta: 1.5 (hard leaf boundary z = 1.5 * y)
//!   - Scale thresholds: ST for x <= 10^10, MT (8T) for x > 10^10

#[derive(Clone, Copy, Debug)]
pub struct DialConfig {
    pub alpha_y: f64,
    pub beta: f64,
    pub num_threads: usize,
    pub use_z_split: bool,
}

pub struct ScaleDispatch;

impl ScaleDispatch {
    /// Dynamic quadratic parameter schedule for Xavier Gourdon on SM4450:
    /// alpha(x) = alpha_0 * (1 + k / ln(x))
    #[inline(always)]
    pub fn alpha_dynamic(x: u64) -> f64 {
        let ln_x = (x as f64).ln();
        if ln_x <= 0.0 {
            return 1.15;
        }
        1.15 * (1.0 + 2.0 / ln_x)
    }

    /// Selects optimal dial configuration based on scale x and system cores
    /// CI-2: threads resolve via CpuTopology (4 on free CI, 8 on SD4G2).
    pub fn select(x: u64, requested_threads: usize) -> DialConfig {
        let threads = titan_core::cpu::CpuTopology::detect().optimal_threads(requested_threads);

        if x <= 10_000_000_000 {
            // ST dispatch: zero thread synchronization / spawn tax
            DialConfig {
                alpha_y: 1.0,
                beta: 1.0,
                num_threads: 1,
                use_z_split: false,
            }
        } else {
            let alpha = Self::alpha_dynamic(x);
            DialConfig {
                alpha_y: alpha,
                beta: 1.5,
                num_threads: threads,
                use_z_split: true,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scale_dispatch_selection() {
        let cfg_small = ScaleDispatch::select(1_000_000_000, 8);
        assert_eq!(cfg_small.num_threads, 1);
        assert_eq!(cfg_small.use_z_split, false);

        let cfg_large = ScaleDispatch::select(100_000_000_000_000, 8);
        let expect_threads = titan_core::cpu::CpuTopology::detect().optimal_threads(8);
        assert_eq!(cfg_large.num_threads, expect_threads);
        assert!(cfg_large.alpha_y > 1.0);
        assert_eq!(cfg_large.beta, 1.5);
    }
}
