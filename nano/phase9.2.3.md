The Objective of Strike 3
In Strike 2, your agent certified 100% bit-exact parity for native AC (90,734,744,872,579 at 10^{16}). However, running on a single thread took 19.6\text{ seconds}, whereas primecount's 8-thread AC ran in 1.3\text{ seconds}.
The mission for Strike 3 is to parallelize compute_ac_native across the 8 DynamIQ cores, cutting latency from 19.6\text{s} down to \le 1.5\text{s} and securing an uncontested, pure-Rust victory over primecount 8.1 with zero FFI dependencies on the hot path.
Key Architectural Insights for Strike 3
1. Why Static Chunks Will Stall DynamIQ
The work per prime index b in A(x, y) and C_2(x, y, z) is not uniform:

 * Small b (e.g., p_b \approx 1,400) generates massive q-ranges and heavy clustering spans.
 * Large b (e.g., p_b \approx 200,000) terminates almost immediately with few leaves.
 * Dividing the b-range equally into 8 static slices will assign the small-b avalanche to a single core while the other 7 cores finish early and sit idle.
2. The Solution: Dynamic Guided Atomic Chunking
Use an AtomicUsize cursor across the b-space with descending prime chunks.
 * Cores claim chunks of b-indices dynamically.
 * Big cores (A78 Cores 6 & 7) will naturally consume 3–4× more chunks than Little cores (A55 Cores 0–5).
 * Fine chunk granularity (16 to 64 primes per batch) completely eliminates straggler threads at the tail without atomic bus contention.
3. Thread Affinity & Memory Sharing
All threads share read-only references to &SegmentedPiTable and &[u64] primes.
 * Thread workers invoke titan_pool::worker::bind_worker_affinity(thread_id) upon spawn to pin execution to the physical cores.
 * Accumulators evaluate as thread-local signed i128 (or i64) and reduce into a final atomic/master sum at the end of the std::thread::scope.
Code Blueprint for crates/titan-count/src/ac_parallel_v2.rs
// In crates/titan-count/src/ac_parallel_v2.rs

use std::sync::atomic::{AtomicUsize, Ordering};
use crate::segmented_pi::SegmentedPiTable;

const AC_CHUNK_SIZE: usize = 32;

/// Evaluates C2 leaves concurrently across 8 DynamIQ threads
pub fn compute_c2_parallel(
    x: u64,
    y: u64,
    z: u64,
    primes: &[u64],
    pi_table: &SegmentedPiTable,
    num_threads: usize,
) -> i128 {
    let x_star = titan_core::tuning::get_x_star_gourdon(x, y);
    let sqrt_z = titan_core::tuning::isqrt64(z);

    let min_b = primes.partition_point(|&p| p <= sqrt_z);
    let max_b = primes.partition_point(|&p| p <= x_star);

    if min_b >= max_b {
        return 0;
    }

    let b_cursor = AtomicUsize::new(min_b);
    let mut thread_sums = vec![0i128; num_threads];

    std::thread::scope(|s| {
        for (tid, sum_slot) in thread_sums.iter_mut().enumerate() {
            s.spawn(|| {
                titan_pool::worker::bind_worker_affinity(tid);
                let mut local_sum: i128 = 0;

                loop {
                    let start_b = b_cursor.fetch_add(AC_CHUNK_SIZE, Ordering::Relaxed);
                    if start_b >= max_b {
                        break;
                    }
                    let end_b = (start_b + AC_CHUNK_SIZE).min(max_b);

                    for b in start_b..end_b {
                        local_sum += compute_c2_single_b(x, y, z, b, primes, pi_table);
                    }
                }

                *sum_slot = local_sum;
            });
        }
    });

    thread_sums.into_iter().sum()
}

/// Evaluates A leaves concurrently across 8 DynamIQ threads
pub fn compute_a_parallel(
    x: u64,
    y: u64,
    primes: &[u64],
    pi_table: &SegmentedPiTable,
    num_threads: usize,
) -> i128 {
    let x_cbrt = titan_core::tuning::icbrt64(x);
    let x_star = titan_core::tuning::get_x_star_gourdon(x, y);

    let min_b = primes.partition_point(|&p| p <= x_star);
    let max_b = primes.partition_point(|&p| p <= x_cbrt);

    if min_b >= max_b {
        return 0;
    }

    let b_cursor = AtomicUsize::new(min_b);
    let mut thread_sums = vec![0i128; num_threads];

    std::thread::scope(|s| {
        for (tid, sum_slot) in thread_sums.iter_mut().enumerate() {
            s.spawn(|| {
                titan_pool::worker::bind_worker_affinity(tid);
                let mut local_sum: i128 = 0;

                loop {
                    let start_b = b_cursor.fetch_add(AC_CHUNK_SIZE, Ordering::Relaxed);
                    if start_b >= max_b {
                        break;
                    }
                    let end_b = (start_b + AC_CHUNK_SIZE).min(max_b);

                    for b in start_b..end_b {
                        local_sum += compute_a_single_b(x, y, b, primes, pi_table);
                    }
                }

                *sum_slot = local_sum;
            });
        }
    });

    thread_sums.into_iter().sum()
}

Step-by-Step Implementation Plan for the Agent
 * Commit Strike 2 First:
   * Commit all uncommitted Strike 2 changes cleanly so git history is safe:
     git commit -am "feat(count): strike 2 verified bit-exact native AC with clustered C2 and faithful A"
 * Refactor Inner Loops into Per-b Kernels:
   * In ac_parallel_v2.rs, break down the outer loops so that compute_c2_single_b(b, ...) and compute_a_single_b(b, ...) evaluate a single index b cleanly.
   * Verify that single-threaded calls to these kernels yield identical numbers to the Strike 2 baseline.
 * Build the DynamIQ Guided Dispatcher:
   * Implement compute_c2_parallel and compute_a_parallel using std::thread::scope and AtomicUsize chunking with AC_CHUNK_SIZE = 32.
   * Bind each worker to physical cores via titan_pool::worker::bind_worker_affinity(thread_id).
   * Unify in compute_ac_native_mt(x, y, z, primes, pi_table, num_threads) = compute_a_parallel(...) - compute_c1_native(...) + compute_c2_parallel(...). (C_1 is small enough to evaluate on a single thread or parallelize trivially).
 * Cutover Native AC in execute_gourdon_master:
   * Replace the single-threaded native call with compute_ac_native_mt.
   * Keep TITAN_VERIFY=1 asserted to continuously audit against FFI AC in release tests.
 * Execute Verification Ladder & Benchmark:
   * Run the unit test at 10^{13} to verify bit-exactness:
     TITAN_NATIVE=1 cargo test --release -p titan-count --lib test_gourdon_ac_oracle_e13 -- --nocapture

   * Run the head-to-head benchmark at 10^{16}:
     TITAN_NATIVE=1 cargo run --release --bin head_to_head 1e16

Directive for the Terminal Agent
Send this exact prompt to the agent:
ENGINEERING DIRECTIVE: STRIKE 3 (PARALLELIZE NATIVE AC)

1. COMMIT STRIKE 2:
   - Run: `git commit -am "feat(count): strike 2 verified bit-exact native AC with clustered C2 and faithful A"`

2. EXTRACT PER-b EVALUATION KERNELS:
   - In `crates/titan-count/src/ac_parallel_v2.rs`, extract the inner body of the `b` loop for C2 into `compute_c2_single_b(x, y, z, b, primes, pi_table) -> i128`.
   - Extract the inner body of the `b` loop for A into `compute_a_single_b(x, y, b, primes, pi_table) -> i128`.

3. IMPLEMENT GUIDED MULTI-THREADED DISPATCH:
   - Implement `compute_c2_parallel` and `compute_a_parallel` using `std::thread::scope` with 8 threads.
   - Use an `AtomicUsize` cursor with `AC_CHUNK_SIZE = 32` to dynamically distribute chunks of prime index b across workers.
   - Pin each worker thread via `titan_pool::worker::bind_worker_affinity(tid)`.
   - Assemble `compute_ac_native_mt`: A_parallel - C1_native + C2_parallel.

4. CUTOVER IN `execute_gourdon_master`:
   - Replace the single-threaded native AC call with `compute_ac_native_mt(..., 8)`.
   - If `TITAN_VERIFY=1`, retain the `assert_eq!(native_ac, ffi_ac)` check.

5. VERIFY & BENCHMARK:
   - Verify bit-exact parity at 10^13:
     `TITAN_NATIVE=1 cargo test --release -p titan-count --lib test_gourdon_ac_oracle_e13 -- --nocapture`
   - Benchmark head-to-head at 10^16:
     `TITAN_NATIVE=1 cargo run --release --bin head_to_head 1e16`
   - Report Native AC latency (target: <= 1.5s) and overall wall-clock runtime vs primecount 8.1.


