//! titan-sieve: High-performance segmented wheel sieve of Eratosthenes.

pub mod adaptive_dispenser;
pub mod arena;
pub mod asymmetric_dispenser;
pub mod b_carry;
pub mod base;
pub mod thread_local_acc;
pub mod boot_wheel;
pub mod bucket_sieve;
pub mod count;
pub mod count_resolve;
pub mod csr_bucket;
pub mod dense_popcount;
pub mod dense_popcount_neon;
pub mod erat_big;
pub mod erat_medium;
pub mod erat_small;
pub mod factor_table;
pub mod kernels;
pub mod l1_popcount;
pub mod mark64;
pub mod presieve;
pub mod segment;
pub mod segment_dispenser;
pub mod small_sieve;
pub mod stream_pipeline;
pub mod tally;
pub mod wheel30;
pub mod wheel30_dense;
pub mod wheel30_medium;
pub mod wheel30_sparse;
pub mod wheel30_popcount;
pub mod wheel30_tiny;
pub mod frontier_ring;
pub mod wheel210;
pub mod wheel30_paced_asm;
pub mod wheel210_stealer;

pub const DEFAULT_SEGMENT_SIZE: usize = 32768; // 32 KiB L1D optimum on SM4450 (A78 & A55)

/// Count primes <= n using the default 64 KiB segment size.
#[inline]
pub fn pi(n: u64) -> u64 {
    segment::count_primes(n, DEFAULT_SEGMENT_SIZE)
}

/// Count primes <= n using a custom segment size in bytes.
#[inline]
pub fn pi_with_segment_size(n: u64, seg_size_bytes: usize) -> u64 {
    segment::count_primes(n, seg_size_bytes)
}

/// Count primes in range [lo, hi].
#[inline]
pub fn pi_range(lo: u64, hi: u64) -> u64 {
    segment::count_primes_range(lo, hi, DEFAULT_SEGMENT_SIZE)
}

/// Count primes in range [lo, hi] using custom segment size.
#[inline]
pub fn pi_range_with_segment_size(lo: u64, hi: u64, seg_size_bytes: usize) -> u64 {
    segment::count_primes_range(lo, hi, seg_size_bytes)
}
