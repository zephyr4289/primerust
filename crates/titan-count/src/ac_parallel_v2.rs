//! Phase 8.3 & Phase 8.4.4: Dual-A78 Concurrent ILP-4 AC Engine + A(x, y) formula.
//!
//! Re-wires the lock-free AcWorkQueue so both Cortex-A78 cores run the 4-way ILP
//! unrolled reciprocal loop in parallel, plus evaluates the missing A(x, y) analytical leaves.

use std::sync::atomic::{AtomicU64, Ordering};
use crate::magic_reciprocal::FastDiv64;
use crate::pi_table::PiTable;
use crate::segmented_pi::SegmentedPiTable;
use titan_core::roots::{icbrt, isqrt};
use crate::sigma_l1::get_x_star_gourdon;

pub struct AcWorkQueue {
    current_m: AtomicU64,
    y: u64,
}

impl AcWorkQueue {
    pub fn new(y: u64) -> Self {
        Self {
            current_m: AtomicU64::new(1),
            y,
        }
    }

    #[inline(always)]
    pub fn claim_chunk(&self) -> Option<(u64, u64)> {
        let mut curr = self.current_m.load(Ordering::Relaxed);
        loop {
            if curr > self.y {
                return None;
            }
            let remaining = self.y - curr + 1;
            let step = (remaining / 64).clamp(16, 4096);
            let next = (curr + step).min(self.y + 1);

            match self.current_m.compare_exchange_weak(
                curr,
                next,
                Ordering::AcqRel,
                Ordering::Relaxed,
            ) {
                Ok(_) => return Some((curr, next)),
                Err(actual) => curr = actual,
            }
        }
    }
}

pub fn compute_ac_range_ilp4(
    m_start: u64,
    m_end: u64,
    x: u64,
    z: u64,
    mu: &[i8],
    primes: &[u64],
    reciprocals: &[FastDiv64],
    pi_table: &SegmentedPiTable,
) -> i64 {
    let mut chunk_sum: i64 = 0;

    for m in m_start..m_end {
        let mu_m = if (m as usize) < mu.len() {
            unsafe { *mu.get_unchecked(m as usize) }
        } else {
            0
        };
        if mu_m == 0 {
            continue;
        }

        let x_div_m = x / m;
        let p_min_bound = x / (m * z);
        let p_max = isqrt(x_div_m);

        if p_min_bound >= p_max {
            continue;
        }

        let p_start_idx = primes.partition_point(|&p| p <= p_min_bound);
        let p_end_idx = primes.partition_point(|&p| p <= p_max);

        if p_start_idx >= p_end_idx {
            continue;
        }

        let mut sub_sum: i64 = 0;
        let mut idx = p_start_idx;

        while idx + 4 <= p_end_idx {
            let r0 = unsafe { reciprocals.get_unchecked(idx) };
            let r1 = unsafe { reciprocals.get_unchecked(idx + 1) };
            let r2 = unsafe { reciprocals.get_unchecked(idx + 2) };
            let r3 = unsafe { reciprocals.get_unchecked(idx + 3) };

            let v0 = r0.div(x_div_m);
            let v1 = r1.div(x_div_m);
            let v2 = r2.div(x_div_m);
            let v3 = r3.div(x_div_m);

            let pi_v0 = pi_table.pi(v0) as i64;
            let pi_v1 = pi_table.pi(v1) as i64;
            let pi_v2 = pi_table.pi(v2) as i64;
            let pi_v3 = pi_table.pi(v3) as i64;

            let p0 = (idx + 1) as i64;
            let p1 = (idx + 2) as i64;
            let p2 = (idx + 3) as i64;
            let p3 = (idx + 4) as i64;

            sub_sum += (pi_v0 - p0 + 1)
                     + (pi_v1 - p1 + 1)
                     + (pi_v2 - p2 + 1)
                     + (pi_v3 - p3 + 1);

            idx += 4;
        }

        while idx < p_end_idx {
            let v = unsafe { reciprocals.get_unchecked(idx).div(x_div_m) };
            let pi_v = pi_table.pi(v) as i64;
            let pi_p = (idx + 1) as i64;
            sub_sum += pi_v - pi_p + 1;
            idx += 1;
        }

        chunk_sum += (mu_m as i64) * sub_sum;
    }

    chunk_sum
}

/// Evaluates the genuine Xavier Gourdon A(x, y) formula across range b in (pi(x_star), pi(x^(1/3))]
pub fn compute_a_formula(
    x: u64,
    y: u64,
    primes: &[u64],
    pi_table: &PiTable,
) -> i64 {
    let x_cbrt = icbrt(x);
    let x_star = get_x_star_gourdon(x, y);
    let has_sentinel = primes.first() == Some(&0);
    let prime_slice = if has_sentinel { &primes[1..] } else { primes };

    let min_b = prime_slice.partition_point(|&p| p <= x_star);
    let max_b = prime_slice.partition_point(|&p| p <= x_cbrt);

    if min_b >= max_b {
        return 0;
    }

    let mut sum: i64 = 0;

    for b in min_b..max_b {
        let prime = prime_slice[b];
        let xp = x / prime;
        let sqrt_xp = isqrt(xp);

        let min_q = prime;
        let max_q = sqrt_xp;

        if min_q >= max_q {
            continue;
        }

        let mut i = prime_slice.partition_point(|&p| p <= min_q);
        let max_i1 = prime_slice.partition_point(|&p| p <= (xp / y).min(max_q));
        let max_i2 = prime_slice.partition_point(|&p| p <= max_q);

        // Leaves where x / (p * q) >= y (Weight 1)
        while i < max_i1 {
            let q = prime_slice[i];
            let xpq = xp / q;
            sum += pi_table.pi(xpq) as i64;
            i += 1;
        }

        // Leaves where x / (p * q) < y (Weight 2 — Symmetry Multiplier)
        while i < max_i2 {
            let q = prime_slice[i];
            let xpq = xp / q;
            sum += (pi_table.pi(xpq) as i64) * 2;
            i += 1;
        }
    }

    sum
}

/// Evaluates C2 leaves for b in (pi(sqrt(z)), pi(x_star)]
pub fn compute_c2_formula(
    x: u64,
    y: u64,
    z: u64,
    primes: &[u64],
    pi_table: &PiTable,
) -> i64 {
    let x_star = isqrt(x / y);
    let sqrt_z = isqrt(z);
    let has_sentinel = primes.first() == Some(&0);
    let prime_slice = if has_sentinel { &primes[1..] } else { primes };

    let min_b = prime_slice.partition_point(|&p| p <= sqrt_z);
    let max_b = prime_slice.partition_point(|&p| p <= x_star);

    if min_b >= max_b {
        return 0;
    }

    let mut sum: i64 = 0;

    for b in min_b..max_b {
        let prime = prime_slice[b];
        let xp = x / prime;
        let sqrt_xp = isqrt(xp);

        let min_q = prime.max(z / prime);
        let max_q = (xp / prime).min(y);

        if min_q >= max_q {
            continue;
        }

        let mut i = prime_slice.partition_point(|&p| p <= min_q);
        let max_i = prime_slice.partition_point(|&p| p <= max_q);

        let b_1based = (b + 1) as i64;
        while i < max_i {
            let q = prime_slice[i];
            let xpq = xp / q;
            let phi_xpq = (pi_table.pi(xpq) as i64) - b_1based + 2;
            sum += phi_xpq;
            i += 1;
        }
    }

    sum
}

#[cfg(test)]
mod tests {
    use super::*;
    use titan_sieve::base::generate_base_primes;

    #[test]
    fn test_a_formula_e13() {
        let x = 10_000_000_000_000u64;
        let y = 103_411u64;
        let max_v = x / y;
        let base_primes = generate_base_primes(max_v.min(10_000_000));
        let mut primes = vec![0u64];
        primes.extend_from_slice(&base_primes);

        let pi_table = PiTable::new(max_v);
        let a_val = compute_a_formula(x, y, &primes, &pi_table);
        println!("Computed A(1e13) = {}", a_val);
        assert!(a_val > 0, "A(1e13) must be positive");

        let z = 170_628u64;
        let c2_val = compute_c2_formula(x, y, z, &primes, &pi_table);
        println!("Computed C2(1e13) = {}", c2_val);
    }
}
