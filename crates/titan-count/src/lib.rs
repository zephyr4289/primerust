//! titan-count: Lehmer-class combinatorial prime counting engine.

pub mod alpha;
pub mod arena25;
pub mod assembly;
pub mod b_term;
pub mod checkpoint;
pub mod d_neon;
pub mod d_term;
pub mod factortable;
pub mod ftd_block;
pub mod ftd_compressed;
pub mod ftd_stream;
pub mod ftd_v2;
pub mod gourdon;
pub mod interval_walker;
pub mod leaf_block;
pub mod leaves;
pub mod lmo;
pub mod lmo_engine;
pub mod magic;
pub mod meissel;
pub mod mertens_struct;
pub mod mobius_stream;
pub mod model;
pub mod mu_rider;
pub mod mu_sieve;
pub mod p2_sweep;
pub mod p3;
pub mod phi;
pub mod phi_flat;
pub mod phi_tables;
pub mod pi_table;
pub mod scale_dispatch;
pub mod sigma_l1;
pub mod special_leaves;

pub use arena25::Arena25Engine;
pub use assembly::LehmerCounter;
pub use factortable::Ftd;
pub use ftd_stream::FtdStream;
pub use gourdon::GourdonCounter;
pub use leaf_block::{LeafBlockC, LeafBlockEngine};
pub use lmo::LmoCounter;
pub use meissel::MeisselCounter;
pub use phi_flat::PhiFlat;
pub use scale_dispatch::{DialConfig, ScaleDispatch};

/// Exact combinatorial count of pi(x) using the Gourdon interval substrate.
#[inline]
pub fn pi_gourdon(x: u64) -> u64 {
    GourdonCounter::count(x, 8)
}

/// Exact multi-threaded combinatorial count of pi(x) using Gourdon MT.
#[inline]
pub fn pi_gourdon_mt(x: u64, num_threads: usize) -> u64 {
    GourdonCounter::count(x, num_threads)
}

/// Exact combinatorial count of pi(x) using the LMO algorithm.
#[inline]
pub fn pi_lmo(x: u64) -> u64 {
    let mut counter = LmoCounter::new();
    counter.count(x)
}

/// Exact multi-threaded combinatorial count of pi(x) using LMO MT.
#[inline]
pub fn pi_lmo_mt(x: u64, num_threads: usize) -> u64 {
    let counter = LmoCounter::new();
    counter.count_mt(x, num_threads)
}

/// Exact combinatorial count of pi(x) using the Meissel identity (P3-free).
#[inline]
pub fn pi_meissel(x: u64) -> u64 {
    let mut counter = MeisselCounter::new();
    counter.count(x)
}

/// Exact multi-threaded combinatorial count of pi(x) using Meissel MT.
#[inline]
pub fn pi_meissel_mt(x: u64, num_threads: usize) -> u64 {
    let counter = MeisselCounter::new();
    counter.count_mt(x, num_threads)
}

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
