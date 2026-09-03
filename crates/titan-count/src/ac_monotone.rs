//! Phase 8.1: Monotone Two-Pointer Clustered AC Engine (ac_monotone.rs).
//! Replaces binary searches with O(1) amortized forward prime cursor scanning.

use crate::magic_reciprocal::FastDiv64;
use crate::segmented_pi::SegmentedPiTable;
use crate::segmented_pi_compact::CompactPiTable;

pub trait PiTableOps: Send + Sync {
    fn pi(&self, x: u64) -> u64;
}

impl PiTableOps for SegmentedPiTable {
    #[inline(always)]
    fn pi(&self, x: u64) -> u64 {
        self.pi(x)
    }
}

impl PiTableOps for CompactPiTable {
    #[inline(always)]
    fn pi(&self, x: u64) -> u64 {
        self.pi(x)
    }
}

#[inline(always)]
pub fn compute_ac_monotone_m(
    x_div_m: u64,
    p_min_bound: u64,
    p_max: u64,
    primes: &[u64],
    reciprocals: &[FastDiv64],
    pi_table: &impl PiTableOps,
) -> i64 {
    if p_min_bound >= p_max {
        return 0;
    }

    let p_start_idx = primes.partition_point(|&p| p <= p_min_bound);
    let p_end_idx = primes.partition_point(|&p| p <= p_max);

    if p_start_idx >= p_end_idx {
        return 0;
    }

    let count = p_end_idx - p_start_idx;
    // For small leaf counts, run the direct unrolled loop with zero clustering overhead
    if count < 64 {
        let mut leaf_acc: i64 = 0;
        let mut idx = p_start_idx;
        while idx < p_end_idx {
            let v = unsafe { reciprocals.get_unchecked(idx).div(x_div_m) };
            let pi_v = pi_table.pi(v);
            let pi_p = (idx + 1) as u64;
            leaf_acc += (pi_v as i64) - (pi_p as i64) + 1;
            idx += 1;
        }
        return leaf_acc;
    }

    let mut sum: i64 = 0;

    // Clustering threshold: when v <= 256, multiple primes share the same quotient v.
    let v_threshold = 256u64.min(p_max);
    let p_cluster_limit = x_div_m / (v_threshold + 1);

    // Fast Forward Cursor Split: Partition index ONCE per m (not per v!)
    let split_idx = primes[p_start_idx..p_end_idx]
        .partition_point(|&p| p <= p_cluster_limit) + p_start_idx;

    // -------------------------------------------------------------------------
    // PART 1: Non-Clustered Region (v > v_threshold)
    // -------------------------------------------------------------------------
    let mut idx = p_start_idx;
    while idx + 4 <= split_idx {
        let r0 = unsafe { *reciprocals.get_unchecked(idx) };
        let r1 = unsafe { *reciprocals.get_unchecked(idx + 1) };
        let r2 = unsafe { *reciprocals.get_unchecked(idx + 2) };
        let r3 = unsafe { *reciprocals.get_unchecked(idx + 3) };

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

        sum += (pi_v0 - p0 + 1) + (pi_v1 - p1 + 1) + (pi_v2 - p2 + 1) + (pi_v3 - p3 + 1);
        idx += 4;
    }
    while idx < split_idx {
        let v = unsafe { reciprocals.get_unchecked(idx).div(x_div_m) };
        let pi_v = pi_table.pi(v) as i64;
        let pi_p = (idx + 1) as i64;
        sum += pi_v - pi_p + 1;
        idx += 1;
    }

    // -------------------------------------------------------------------------
    // PART 2: Clustered Region (v <= v_threshold)
    // Monotonic Two-Pointer Cursor: Advances forward ONLY! Zero binary searches!
    // -------------------------------------------------------------------------
    if split_idx < p_end_idx {
        let mut cursor = split_idx;
        let mut v = x_div_m / primes[split_idx];
        let v_min = x_div_m / primes[p_end_idx - 1];

        while v >= v_min && cursor < p_end_idx {
            let p_bound = x_div_m / v;
            let cluster_start = cursor;

            // Monotone scan forward: advances monotonically, 0 binary searches!
            while cursor < p_end_idx && unsafe { *primes.get_unchecked(cursor) } <= p_bound {
                cursor += 1;
            }

            let cluster_count = (cursor - cluster_start) as i64;
            if cluster_count > 0 {
                let pi_v = pi_table.pi(v) as i64;
                let first_p = (cluster_start + 1) as i64;
                let last_p = cursor as i64;
                // Arithmetic progression for sum of pi(p): O(1)
                let sum_pi_p = (first_p + last_p) * cluster_count / 2;

                sum += (pi_v + 1) * cluster_count - sum_pi_p;
            }

            if v == 0 { break; }
            v -= 1;
        }
    }

    sum
}

#[cfg(test)]
mod tests {
    use super::*;
    use titan_core::roots::isqrt;

    fn generate_primes(limit: u64) -> Vec<u64> {
        let mut sieve = vec![true; (limit + 1) as usize];
        let mut primes = Vec::new();
        for p in 2..=limit {
            if sieve[p as usize] {
                primes.push(p);
                for i in (p * p..=limit).step_by(p as usize) {
                    sieve[i as usize] = false;
                }
            }
        }
        primes
    }

    #[test]
    fn test_monotone_parity_against_oracle() {
        let primes = generate_primes(200_000);
        let max_prime = *primes.last().unwrap();
        let table = CompactPiTable::new(0, 500_000, &primes);
        let max_x = 10_000_000_000u64;

        let reciprocals: Vec<FastDiv64> = primes.iter().map(|&p| FastDiv64::new(p, max_x)).collect();

        for &x_div_m in &[1_000_000u64, 5_000_000, 10_000_000, 50_000_000, 100_000_000, 500_000_000] {
            let p_min_bound = 10u64;
            let p_max = isqrt(x_div_m).min(max_prime);

            let p_start_idx = primes.partition_point(|&p| p <= p_min_bound);
            let p_end_idx = primes.partition_point(|&p| p <= p_max);
            let mut expected_sum: i64 = 0;
            for idx in p_start_idx..p_end_idx {
                let v = x_div_m / primes[idx];
                let pi_v = table.pi(v);
                let pi_p = (idx + 1) as u64;
                expected_sum += (pi_v as i64) - (pi_p as i64) + 1;
            }

            let monotone_sum = compute_ac_monotone_m(
                x_div_m,
                p_min_bound,
                p_max,
                &primes,
                &reciprocals,
                &table,
            );

            assert_eq!(
                monotone_sum, expected_sum,
                "Mismatch at x_div_m = {}: monotone = {}, expected = {}",
                x_div_m, monotone_sum, expected_sum
            );
        }
    }
}
