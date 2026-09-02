//! titan-sieve: High-performance segmented wheel sieve of Eratosthenes.

pub mod arena;
pub mod b_carry;
pub mod base;
pub mod boot_wheel;
pub mod count;
pub mod erat_big;
pub mod erat_medium;
pub mod erat_small;
pub mod kernels;
pub mod mark64;
pub mod presieve;
pub mod segment;
pub mod tally;

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
