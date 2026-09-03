//! Phase 5.1: Dirichlet Hyperbola Inverted AC Engine (ac_hyperbola.rs).
//!
//! Evaluates AC leaves by stepping over quotient v directly,
//! eliminating billions of prime divisions and binary searches.

use crate::pi_table::PiTable;
use crate::sampled_index::SampledPrimeIndex;

/// Evaluates AC leaves by inverting the inner loop to step over quotient v
/// directly, eliminating billions of prime iterations.
#[inline(always)]
pub fn evaluate_ac_hyperbola_m(
    x_div_m: u64,
    p_min: u64,
    p_max: u64,
    primes: &[u64],
    pi_table: &PiTable,
) -> i64 {
    let prime_slice = if primes.first() == Some(&0) { &primes[1..] } else { primes };
    let sampled_idx = SampledPrimeIndex::build(prime_slice);
    evaluate_ac_hyperbola_m_sampled(x_div_m, p_min, p_max, primes, pi_table, &sampled_idx)
}

/// Evaluates AC leaves by inverting the inner loop to step over quotient v
/// directly using a pre-built SampledPrimeIndex (Phase 6.1).
#[inline(always)]
pub fn evaluate_ac_hyperbola_m_sampled(
    x_div_m: u64,
    p_min: u64,
    p_max: u64,
    primes: &[u64],
    pi_table: &PiTable,
    sampled_idx: &SampledPrimeIndex,
) -> i64 {
    if p_min >= p_max { return 0; }

    let prime_slice = if primes.first() == Some(&0) { &primes[1..] } else { primes };
    let pi_max = pi_table.max_y;
    let v_min = x_div_m / p_max;
    let v_max = x_div_m / (p_min + 1);

    let mut sum: i64 = 0;

    // Direct Hyperbola inversion: iterate over quotient v directly
    for v in v_min..=v_max {
        let pi_v = if v <= pi_max {
            pi_table.pi(v) as i64
        } else {
            sampled_idx.pi(prime_slice, v) as i64
        };

        // Bounding interval of primes that produce quotient v:
        // floor(x_div_m / p) == v <=> x_div_m / (v + 1) < p <= x_div_m / v
        let p_low = (x_div_m / (v + 1)).max(p_min);
        let p_high = (x_div_m / v).min(p_max);

        if p_low >= p_high { continue; }

        let idx_low = if p_low <= pi_max {
            pi_table.pi(p_low) as i64
        } else {
            sampled_idx.pi(prime_slice, p_low) as i64
        };

        let idx_high = if p_high <= pi_max {
            pi_table.pi(p_high) as i64
        } else {
            sampled_idx.pi(prime_slice, p_high) as i64
        };

        let delta_pi = idx_high - idx_low;
        if delta_pi <= 0 { continue; }

        // Sum of prime indices in [idx_low + 1, idx_high]
        let i_a = idx_low + 1;
        let i_b = idx_high;
        let sum_pi = (i_a + i_b) * delta_pi / 2;

        // Closed-form: delta_pi * (pi(v) + 1) - sum_pi
        sum += delta_pi * (pi_v + 1) - sum_pi;
    }

    sum
}

#[cfg(test)]
mod tests {
    use super::*;
    use titan_core::roots::isqrt;
    use titan_sieve::base::generate_base_primes;

    #[test]
    fn test_ac_hyperbola_exactness() {
        let x = 10_000_000u64;
        let x_sqrt = isqrt(x);
        let base_primes = generate_base_primes(x_sqrt + 100);
        let mut primes = vec![0u64];
        primes.extend_from_slice(&base_primes);
        let pi_table = PiTable::new(x_sqrt + 30);

        for m in [100u64, 200, 300, 500, 700] {
            let x_div_m = x / m;
            let p_min = 10;
            let p_max = isqrt(x_div_m);
            if p_min >= p_max { continue; }

            // Naive reference
            let mut expected = 0i64;
            let p_start_idx = primes[1..].partition_point(|&p| p <= p_min) + 1;
            let p_end_idx = primes[1..].partition_point(|&p| p <= p_max) + 1;
            for i in p_start_idx..p_end_idx {
                let p = primes[i];
                let v = x_div_m / p;
                let pi_v = if v <= pi_table.max_y {
                    pi_table.pi(v) as i64
                } else {
                    primes[1..].partition_point(|&pr| pr <= v) as i64
                };
                let pi_p = i as i64;
                expected += pi_v - pi_p + 1;
            }

            let actual = evaluate_ac_hyperbola_m(x_div_m, p_min, p_max, &primes, &pi_table);
            assert_eq!(actual, expected, "Hyperbola mismatch for m = {}: expected {}, got {}", m, expected, actual);
        }
    }
}
