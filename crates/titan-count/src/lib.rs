//! titan-count: Lehmer-class combinatorial prime counting engine.

pub mod assembly;
pub mod checkpoint;
pub mod magic;
pub mod p2_sweep;
pub mod p3;
pub mod phi;
pub mod pi_table;

pub use assembly::LehmerCounter;

/// Exact combinatorial count of pi(x) using the Lehmer identity.
#[inline]
pub fn pi_count(x: u64) -> u64 {
    let mut counter = LehmerCounter::new();
    counter.count(x)
}

/// Exact combinatorial count of pi(x) with specified a parameter.
#[inline]
pub fn pi_count_with_a(x: u64, a: usize) -> u64 {
    let mut counter = LehmerCounter::new();
    counter.count_with_a(x, a)
}

/// Exact multi-threaded combinatorial count of pi(x) across num_threads workers.
#[inline]
pub fn pi_count_mt(x: u64, num_threads: usize) -> u64 {
    let counter = LehmerCounter::new();
    counter.count_mt(x, num_threads)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_combinatorial_milestones() {
        assert_eq!(pi_count(10), 4);
        assert_eq!(pi_count(100), 25);
        assert_eq!(pi_count(1000), 168);
        assert_eq!(pi_count(10000), 1229);
        assert_eq!(pi_count(100000), 9592);
        assert_eq!(pi_count(1000000), 78498);
        assert_eq!(pi_count(10000000), 664579);
    }

    #[test]
    fn test_combinatorial_mt_equivalence() {
        // Verify 100% equivalence between scalar and 8-thread MT evaluation
        let x = 10_000_000u64;
        let st_ans = pi_count(x);
        let mt_ans = pi_count_mt(x, 8);
        assert_eq!(st_ans, 664579);
        assert_eq!(mt_ans, st_ans);
    }
}
