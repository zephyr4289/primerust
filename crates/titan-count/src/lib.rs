//! titan-count: Lehmer-class combinatorial prime counting engine.

pub mod assembly;
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
}
