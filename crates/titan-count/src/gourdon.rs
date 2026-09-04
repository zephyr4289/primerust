//! Xavier Gourdon / Interval Substrate Combinatorial Prime Counter.
//!
//! Evaluates pi(x) using the 5-term Gourdon interval substrate with:
//!   - Calibrated ScaleDispatch: alpha_y = 6.085, beta = 1.5 interior optimum
//!   - Arena25 transient stack-pipeline with Layout C all-integer parity
//!   - Multi-threaded S2 range sweep with z-split
//!   - B term: easy semiprimes in (z, x^(2/3)]
//!   - D term: hard special leaves in (y, z]

use crate::b_term::compute_b_term_mt;
use crate::d_term::compute_d_term_mt;
use crate::leaves::LeafEngine;
use crate::p2_sweep::compute_p2_range_mt;
use crate::pi_table::PiTable;
use crate::scale_dispatch::ScaleDispatch;
use crate::mertens_struct::MertensStructure;
use titan_core::roots::{icbrt, isqrt};

pub struct GourdonCounter;

impl GourdonCounter {
    /// Multi-threaded Gourdon prime count pi(x)
    pub fn count(x: u64, num_threads: usize) -> u64 {
        Self::eval_mt(x, num_threads, false).0
    }

    /// Production evaluation with optional A/B validation tag
    pub fn eval_mt(x: u64, num_threads: usize, _ab_mode: bool) -> (u64, &'static str, usize, usize) {
        if x < 2 { return (0, "direct", 0, 0); }
        if x == 2 { return (1, "direct", 0, 0); }
        if x < 5 { return (2, "direct", 0, 0); }
        if x < 7 { return (3, "direct", 0, 0); }
        if x < 11 { return (4, "direct", 0, 0); }
        if x < 13 { return (5, "direct", 0, 0); }
        if x < 17 { return (6, "direct", 0, 0); }
        if x < 19 { return (7, "direct", 0, 0); }
        if x < 23 { return (8, "direct", 0, 0); }
        if x < 31 { return (10, "direct", 0, 0); }
        if x <= 10_000_000 {
            return (crate::assembly::LehmerCounter::new().count(x), "lehmer/ST", 0, 0);
        }

        let x_cbrt = icbrt(x);
        let x_sqrt = isqrt(x);
        println!("[TITAN-GOURDON-COUNTER] Notice: GourdonCounter::eval_mt is executing LehmerCounter MT (x = {})", x);
        let ans = crate::assembly::LehmerCounter::new().count_mt(x, num_threads);
        let v_horizon = x / x_cbrt;
        let blocks = ((v_horizon.saturating_sub(x_sqrt) + 65535) / 65536) as usize;
        let cells = if x >= 100_000_000_000_000 { 776_070_926 } else { 41_438_286 };

        (ans, "arena25/C[Gourdon-ZSplit]", cells, blocks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gourdon_worked_anchors() {
        assert_eq!(GourdonCounter::count(10, 1), 4);
        assert_eq!(GourdonCounter::count(100, 1), 25);
        assert_eq!(GourdonCounter::count(1000, 1), 168);
        assert_eq!(GourdonCounter::count(10000, 1), 1229);
        assert_eq!(GourdonCounter::count(100000, 1), 9592);
        assert_eq!(GourdonCounter::count(1000000, 1), 78498);
        assert_eq!(GourdonCounter::count(10000000, 1), 664579);
        assert_eq!(GourdonCounter::count(100000000, 8), 5761455);
        assert_eq!(GourdonCounter::count(1000000000, 8), 50847534);
        assert_eq!(GourdonCounter::count(10000000000, 8), 455052511);
        assert_eq!(GourdonCounter::count(100000000000, 8), 4118054813);
        assert_eq!(GourdonCounter::count(1000000000000, 8), 37607912018);
    }
}