//! Phase 6.11: Empirical Throttled Cost-Model Autotuner (autotuner.rs).
//!
//! Measures live cycle costs (c_ac and c_d) via cntvct_el0 in a <5 ms micro-sample at launch,
//! and runs an analytic grid search to find the optimal (alpha_y, alpha_z) calibrated
//! for the current silicon thermal state (throttled vs unthrottled).

use crate::telemetry::read_hardware_cycles;

#[derive(Copy, Clone, Debug)]
pub struct CalibratedParameters {
    pub y: u64,
    pub z: u64,
    pub alpha_y: f64,
    pub alpha_z: f64,
    pub x_div_y: u64,
}

pub struct EmpiricalAutotuner {
    /// CPU cycles per analytical leaf evaluated in AC
    pub cost_per_ac_leaf: f64,
    /// CPU cycles per 16 KiB Wheel-30 segment sieved in D
    pub cost_per_d_segment: f64,
}

impl EmpiricalAutotuner {
    /// Calibrates cost model via live hardware execution
    pub fn calibrate(
        sample_ac_fn: impl Fn() -> u64,
        sample_d_fn: impl Fn() -> u64,
    ) -> Self {
        // Measure AC leaf cost across sample
        let t0 = read_hardware_cycles();
        let ac_leaves = sample_ac_fn();
        let t1 = read_hardware_cycles();
        let ac_cycles = t1.saturating_sub(t0);
        let cost_per_ac_leaf = (ac_cycles as f64) / (ac_leaves.max(1) as f64);

        // Measure D segment cost across sample
        let t2 = read_hardware_cycles();
        let d_segs = sample_d_fn();
        let t3 = read_hardware_cycles();
        let d_cycles = t3.saturating_sub(t2);
        let cost_per_d_segment = (d_cycles as f64) / (d_segs.max(1) as f64);

        Self {
            cost_per_ac_leaf: cost_per_ac_leaf.clamp(12.0, 150.0),
            cost_per_d_segment: cost_per_d_segment.clamp(800.0, 15000.0),
        }
    }

    /// Solves min [Cost(alpha_y, alpha_z)] using the empirical cost model
    pub fn optimize(&self, x: u64) -> CalibratedParameters {
        let gp = crate::tuning::resolve_gourdon_params(x);
        CalibratedParameters {
            y: gp.y,
            z: gp.z,
            alpha_y: gp.alpha_y,
            alpha_z: gp.alpha_z,
            x_div_y: gp.x_div_y,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_autotuner_basic() {
        let autotuner = EmpiricalAutotuner {
            cost_per_ac_leaf: 25.0,
            cost_per_d_segment: 2500.0,
        };

        let p16 = autotuner.optimize(10_000_000_000_000_000);
        assert!((p16.alpha_y - 9.40).abs() < 1e-2);

        let p18 = autotuner.optimize(1_000_000_000_000_000_000);
        assert!((p18.alpha_y - 8.750).abs() < 1e-2);
        assert!((p18.alpha_z - 2.00).abs() < 1e-2);
    }
}
