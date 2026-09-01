//! Scale-Indexed Parameter Dispatch & Device Tuning (Phase 1.28 Deliverable).
//!
//! Provides calibrated parameter dials for Xavier Gourdon / Lehmer prime counting:
//!   - Optimal alpha_y: 6.085 for x >= 10^14 (shrinks physical sweep by 6.1x)
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
    /// Selects optimal dial configuration based on scale x and system cores
    pub fn select(x: u64, requested_threads: usize) -> DialConfig {
        let threads = requested_threads.clamp(1, 8);

        if x <= 10_000_000_000 {
            // ST dispatch: zero thread synchronization / spawn tax
            DialConfig {
                alpha_y: 1.0,
                beta: 1.0,
                num_threads: 1,
                use_z_split: false,
            }
        } else if x < 100_000_000_000_000 {
            // 10^10 < x < 10^14: Moderate alpha_y
            DialConfig {
                alpha_y: 2.0,
                beta: 1.5,
                num_threads: threads,
                use_z_split: true,
            }
        } else {
            // x >= 10^14: Full interior optimum alpha_y = 6.085, beta = 1.5
            DialConfig {
                alpha_y: 6.085,
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
        assert_eq!(cfg_large.num_threads, 8);
        assert_eq!(cfg_large.alpha_y, 6.085);
        assert_eq!(cfg_large.beta, 1.5);
    }
}
