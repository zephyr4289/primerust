//! Phase 6.5 Step 1: Chained Hyperbola State Continuity (ac_hyperbola_picache.rs).
//!
//! Exploits the mathematical endpoint identity:
//!   p_low(v) == p_high(v + 1)
//! Iterates quotient v downwards from v_max down to v_min, maintaining a rolling
//! register pair (next_p, next_idx), eliminating 50% of 64-bit hardware divisions
//! and 50% of pi lookups across AC leaves. Connects O(1) L3-locked PiCache.

use crate::picache::PiCache;
use crate::pi_table::PiTable;

#[inline(always)]
pub fn evaluate_ac_hyperbola_chained(
    x_div_m: u64,
    p_min: u64,
    p_max: u64,
    pi_table: &PiTable,
    picache: &PiCache,
) -> i64 {
    if p_min >= p_max {
        return 0;
    }

    let pi_max = pi_table.max_y;
    let v_min = x_div_m / p_max;
    let v_max = x_div_m / (p_min + 1);

    if v_min > v_max {
        return 0;
    }

    let mut sum: i64 = 0;

    // Seed the chain at v_max + 1
    let mut next_p = (x_div_m / (v_max + 1)).clamp(p_min, p_max);
    let mut next_idx = if next_p <= pi_max {
        pi_table.pi(next_p) as i64
    } else {
        picache.pi(next_p) as i64
    };

    // Iterate downwards: step v reuses next_p and next_idx as its low boundary
    for v in (v_min..=v_max).rev() {
        let p_high = (x_div_m / v).clamp(p_min, p_max);

        let idx_high = if p_high <= pi_max {
            pi_table.pi(p_high) as i64
        } else {
            picache.pi(p_high) as i64
        };

        let p_low = next_p;
        let idx_low = next_idx;

        // Shift register state for next iteration (v - 1)
        next_p = p_high;
        next_idx = idx_high;

        if p_low >= p_high {
            continue;
        }

        let delta_pi = idx_high - idx_low;
        if delta_pi <= 0 {
            continue;
        }

        let pi_v = if v <= pi_max {
            pi_table.pi(v) as i64
        } else {
            picache.pi(v) as i64
        };

        // Gauss closed-form summation over prime indices
        let i_a = idx_low + 1;
        let i_b = idx_high;
        let sum_pi = (i_a + i_b) * delta_pi / 2;

        sum += delta_pi * (pi_v + 1) - sum_pi;
    }

    sum
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ac_hyperbola::evaluate_ac_hyperbola_m;

    #[test]
    fn test_chained_hyperbola_exactness() {
        let x = 10_000_000u64;
        let m = 30u64;
        let x_div_m = x / m;
        let p_min = 20u64;
        let p_max = 500u64;

        let base_primes = titan_sieve::base::generate_base_primes(100_000);
        let pi_table = PiTable::new(10_000);
        let picache = PiCache::build(x_div_m, &base_primes);

        let expected = evaluate_ac_hyperbola_m(x_div_m, p_min, p_max, &base_primes, &pi_table);
        let actual = evaluate_ac_hyperbola_chained(x_div_m, p_min, p_max, &pi_table, &picache);

        assert_eq!(
            actual, expected,
            "Chained hyperbola mismatch: got {}, expected {}",
            actual, expected
        );
    }
}
