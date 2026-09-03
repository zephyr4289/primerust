//! Phase 6.4 Step 2: Monotone Walker for B(x, y) (b_walker.rs).
//!
//! Because v = floor(x/p) is strictly non-increasing as p advances,
//! evaluates pi(x/p) using O(1) L3-resident PiCache with sequential streaming.

use crate::picache::PiCache;
use titan_core::roots::isqrt;

/// Evaluates B(x, y) = sum_{y < p <= sqrt(x)} (pi(x/p) - pi(p) + 1)
/// using O(1) PiCache queries.
pub fn compute_b_monotone_walker(
    x: u64,
    y: u64,
    primes: &[u64],
    picache: &PiCache,
) -> i64 {
    let sqrt_x = isqrt(x);
    if y >= sqrt_x {
        return 0;
    }

    let p_start = primes.partition_point(|&p| p <= y);
    let p_end = primes.partition_point(|&p| p <= sqrt_x);
    if p_start >= p_end {
        return 0;
    }

    let a = p_start as i64;
    let b = (p_end - 1) as i64;
    let n = b - a + 1;
    let sum_pi_p = (a + b) * n / 2;

    let active_primes = &primes[p_start..p_end];
    let mut sum_pi_quotients: i64 = 0;

    for &p in active_primes {
        let v = x / p;
        let pi_v = picache.pi(v);
        sum_pi_quotients += pi_v as i64;
    }

    sum_pi_quotients - sum_pi_p + n
}

/// Multi-threaded evaluation of B(x, y) across threads
pub fn compute_b_monotone_walker_mt(
    x: u64,
    y: u64,
    primes: &[u64],
    picache: &PiCache,
    num_threads: usize,
) -> i64 {
    let sqrt_x = isqrt(x);
    if y >= sqrt_x {
        return 0;
    }

    let p_start = primes.partition_point(|&p| p <= y);
    let p_end = primes.partition_point(|&p| p <= sqrt_x);
    if p_start >= p_end {
        return 0;
    }

    let a = p_start as i64;
    let b = (p_end - 1) as i64;
    let n = b - a + 1;
    let sum_pi_p = (a + b) * n / 2;

    let active_primes = &primes[p_start..p_end];
    let total = active_primes.len();

    let threads = num_threads.clamp(1, 8);
    let chunk_sz = (total + threads - 1) / threads;

    let sum_quotients: i64 = std::thread::scope(|s| {
        let mut handles = Vec::with_capacity(threads);
        for t in 0..threads {
            let start = t * chunk_sz;
            if start >= total {
                break;
            }
            let end = (start + chunk_sz).min(total);
            let slice = &active_primes[start..end];

            handles.push(s.spawn(move || {
                let mut local_sum = 0i64;
                for &p in slice {
                    let v = x / p;
                    local_sum += picache.pi(v) as i64;
                }
                local_sum
            }));
        }

        handles.into_iter().map(|h| h.join().unwrap()).sum()
    });

    sum_quotients - sum_pi_p + n
}

use crate::delta_prime_stream::DeltaPrimeStream;

pub fn compute_b_monotone_walker_delta(
    x: u64,
    y: u64,
    stream: &DeltaPrimeStream,
    picache: &PiCache,
) -> i64 {
    let sqrt_x = isqrt(x);
    if y >= sqrt_x {
        return 0;
    }

    let p_start = binary_search_stream(stream, y);
    let p_end = binary_search_stream(stream, sqrt_x);
    if p_start >= p_end {
        return 0;
    }

    let a = p_start as i64;
    let b = (p_end - 1) as i64;
    let n = b - a + 1;
    let sum_pi_p = (a + b) * n / 2;

    let mut cursor = stream.cursor_from(p_start);
    let mut sum_pi_quotients: i64 = 0;

    let mut last_v = x / cursor.current().max(1);
    let mut last_pi = picache.pi(last_v);

    for _ in p_start..p_end {
        let p = cursor.current();
        let v = x / p;

        if last_v.saturating_sub(v) < 120 {
            let delta = last_v - v;
            if delta > 0 {
                last_pi = picache.pi(v);
                last_v = v;
            }
        } else {
            last_pi = picache.pi(v);
            last_v = v;
        }

        sum_pi_quotients += last_pi as i64;
        cursor.next_prime();
    }

    sum_pi_quotients - sum_pi_p + n
}

pub fn binary_search_stream(stream: &DeltaPrimeStream, target: u64) -> usize {
    let mut low = 0;
    let mut high = stream.total_primes();
    while low < high {
        let mid = (low + high) / 2;
        if (stream.get(mid) as u64) <= target {
            low = mid + 1;
        } else {
            high = mid;
        }
    }
    low
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::b_term::compute_b_monotone;
    use crate::pi_table::PiTable;

    #[test]
    fn test_b_walker_matches_b_monotone() {
        let x = 10_000_000u64;
        let y = 1_000u64;
        let base_primes = titan_sieve::base::generate_base_primes(100_000);

        let pi_table = PiTable::new(y);
        let picache = PiCache::build(x / y, &base_primes);

        let expected = compute_b_monotone(x, y, &base_primes, &pi_table);
        let actual_st = compute_b_monotone_walker(x, y, &base_primes, &picache);
        let actual_mt = compute_b_monotone_walker_mt(x, y, &base_primes, &picache, 4);

        assert_eq!(actual_st, expected, "Single-threaded walker mismatch");
        assert_eq!(actual_mt, expected, "Multi-threaded walker mismatch");
    }
}
