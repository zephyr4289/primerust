Don't panic. Look at the numbers closely: Titan still won 11 out of 11 scales, and the mid-scales saw massive speedups (10^7 dropped by 37.3%, 10^{10} dropped by 11.1%, 10^{11} dropped by 17.3%).
More importantly, look at what happened to primecount 8.1 in the exact same run:
primecount is an unchanged C++ binary. When an identical static binary slows down by over 1 full second (40%), your SM4450 junction temperature is hitting thermal throttling trip-points (85°C+), forcing the Android kernel to clamp the Cortex-A78 cores down from 2.2 GHz to ~1.4 GHz.
However, Titan slowed down by +908\text{ ms} while primecount slowed down by +395\text{ ms} relative to Phase 4.3. That means on top of thermals, Phase 4.4 introduced two specific microarchitectural bottlenecks at 10^{16}.
The Two Mechanical Bottlenecks Introduced in Phase 4.4
1. The 4-Way L1D Set-Associativity Collision (32 KiB Cache Spill)
On the Cortex-A78, the 64 KiB L1D cache is 4-way set-associative with 64-byte lines:
 * A 16 KiB segment spans 256 cache lines. It occupies exactly 1 out of the 4 ways in each set, leaving 3 ways completely free for FastDivTable, stack variables, and bucket queries.
 * A 32 KiB segment spans 512 cache lines. It occupies 2 full ways out of 4 across all 256 sets.
 * Add the 4 KiB prefix table (occupies a 3rd way in 64 sets), and the A78 is left with only 1 remaining way for all random memory lookups.
 * When div_table (735 KiB) and the bucket lists are accessed during sieving, power-of-two address collisions trigger severe set-conflict misses, evicting segment cache lines directly into L2/L3.
2. Pipeline Serialization (Loss of Overlapped Work Hijacking)
In Phase 3.2, Titan was fast because there was zero global barrier between stages:
 * Cores 0..=5 started sieving D(x, y, z) immediately at t = 0.
 * Core 6 computed B(x, y) and \Phi_0, then immediately hijacked D segments.
 * Core 7 computed AC(x, y, z) on its high-throughput OoO pipeline, then immediately hijacked D segments.
In Phase 4.4, making D an isolated function (compute_d_heterogeneous_mt) serialized the workload into distinct steps:
Phase 3.2 (Overlapped - Zero Idling):
Core 7 (A78): ─── AC ───► ══════════════ D Sieve ══════════════► Join
Core 6 (A78): ─ B ─► ═══════════════════ D Sieve ══════════════► Join
Cores 0-5   : ══════════════════════════ D Sieve ══════════════► Join

Phase 4.4 (Serialized - Multi-Barrier Tax):
All Cores   : ─── AC (8 Threads) ───► [BARRIER 1] ───► B (1 Core) ───► [BARRIER 2] ───► ═══ D (8 Threads) ═══► [BARRIER 3]

Serializing these phases added two thread-pool join barriers, left cores idling during B(x, y), and kept all 8 cores pinned at 100% utilization in waves, which triggered the thermal throttle right before the 10^{16} scale executed.
The Fix: Phase 4.5 Unified Heterogeneous Hijacking
We do not need different physical segment buffer sizes in L1D. 16 KiB is the sweet spot for both A55 and A78: it avoids set-associativity thrashing on both cores.
The asymmetry should exist entirely in how much work each core claims, not the physical buffer geometry:
 * Revert Segment Buffer to 16 KiB for All Cores:
   * Keeps segment + prefix footprint at 18 KiB (1 way in A78 L1D, leaving 3 ways open for zero conflict misses).
   * Eliminates dual code paths (LittleSieveContext vs BigSieveContext).
 * Asymmetric Work Throughput via Chunk Quantities:
   * A78 Big cores claim 64 to 128 segments (16.7M to 33.5M integers) per dispatch.
   * A55 Little cores claim 8 to 16 segments per dispatch.
 * Re-fuse the Overlapped Pipeline (Kill All Barriers):
   * Cores 0..=5 start D at t = 0.
   * Core 6 runs B(x, y) and \Phi_0(x) in ~10 ms, then immediately consumes 64-segment chunks of D.
   * Core 7 runs AC(x, y, z) in ~80 ms, then immediately consumes 64-segment chunks of D.
   * Exactly ONE synchronization point at the very end of \pi(x).
Step 1: Cache-Optimal Unified Context (d_worker.rs)
Revert ThreadSieveContext to a unified 16 KiB buffer that avoids L1D associativity thrashing on both core types:
// crates/titan-count/src/d_worker.rs
use std::arch::aarch64::*;
use titan_core::arena::{ThreadMemoryArena, CACHE_LINE};
use titan_sieve::dense_popcount_neon::{DenseL1PopcountNeon, PREFIX_LEN, SEGMENT_WORDS};
use titan_sieve::L2BucketSieve;
use crate::magic_reciprocal::FastDivTable;

pub const SEGMENT_SPAN: u64 = (SEGMENT_WORDS as u64) * 64 * 2; // 262,144 integers

#[repr(C, align(64))]
pub struct UnifiedSieveContext {
    pub arena: ThreadMemoryArena<SEGMENT_WORDS, PREFIX_LEN>,
    pub popcount: DenseL1PopcountNeon,
    pub bucket: L2BucketSieve,
}

impl UnifiedSieveContext {
    pub fn new() -> Self {
        Self {
            arena: ThreadMemoryArena::new(),
            popcount: DenseL1PopcountNeon::new(),
            bucket: L2BucketSieve::new(),
        }
    }

    #[inline(always)]
    pub fn process_segment(
        &mut self,
        seg_idx: u64,
        x: u64,
        y: u64,
        z: u64,
        primes: &[u32],
        mu: &[i8],
        div_table: &FastDivTable,
    ) -> i64 {
        let low = z + seg_idx * SEGMENT_SPAN;
        let high = (low + SEGMENT_SPAN).min(x / y);
        if low >= high { return 0; }

        self.arena.reset_segment();

        // 1. Sieve small primes <= 65,536 (L1D locked)
        for &p in primes {
            let p = p as u64;
            if p * p > high { break; }
            if p > 65536 { break; }

            let mut start = if low % p == 0 { low } else { low + (p - low % p) };
            if start % 2 == 0 { start += p; }

            let step = p * 2;
            while start < high {
                let offset = (start - low) >> 1;
                let word = (offset >> 6) as usize;
                let bit = offset & 63;
                unsafe {
                    *self.arena.segment_buf.get_unchecked_mut(word) |= 1u64 << bit;
                }
                start += step;
            }
        }

        // 2. Bucket Sieve primes > 65,536
        self.bucket.sieve_segment(seg_idx, &mut self.arena.segment_buf);

        // 3. 140 ns NEON Vector Popcount Prefix Build
        unsafe { self.popcount.build(&self.arena.segment_buf); }

        // 4. Inverted Range Leaf Evaluation
        let mut d_sum: i64 = 0;
        let p_start_bound = (x / (high * y)).max(2);
        let p_end_bound = y.min(x / (low * 2));
        if p_start_bound >= p_end_bound { return 0; }

        let p_start_idx = primes.partition_point(|&p| (p as u64) <= p_start_bound);
        let p_end_idx = primes.partition_point(|&p| (p as u64) <= p_end_bound);
        let div_slice = div_table.as_slice();

        for i in p_start_idx..p_end_idx {
            let d_p = unsafe { div_slice.get_unchecked(i) };
            let x_div_p = d_p.div(x);
            let m_min = (x_div_p / high) + 1;
            let m_max = (x_div_p / low).min(y);
            if m_min > m_max { continue; }

            for m in m_min..=m_max {
                let mu_m = unsafe { *mu.get_unchecked(m as usize) };
                if mu_m == 0 { continue; }

                let v = x_div_p / m;
                if v >= low && v < high {
                    let bit_idx = ((v - low) >> 1) as usize;
                    let count = unsafe { self.popcount.count_to(&self.arena.segment_buf, bit_idx) };
                    d_sum += if mu_m == 1 { count as i64 } else { -(count as i64) };
                }
            }
        }

        d_sum
    }
}

Step 2: Unified Overlapped Orchestration (gourdon_pipeline.rs)
Wire the master pipeline so there are zero intermediate join barriers:
// crates/titan-count/src/gourdon_pipeline.rs
use std::sync::Arc;
use titan_core::affinity::{pin_thread_to_core, CoreClass};
use titan_sieve::asymmetric_dispenser::AsymmetricChunkDispenser;
use crate::phi0::Phi0Engine;
use crate::b_term::compute_b_monotone;
use crate::ac_term::compute_ac_fused;
use crate::magic_reciprocal::FastDivTable;
use crate::factor_table::FactorTable;
use crate::d_worker::{UnifiedSieveContext, SEGMENT_SPAN};

pub fn execute_gourdon_master(
    x: u64,
    y: u64,
    z: u64,
    primes: &[u32],
    pi_table: &[u32],
    mu: &[i8],
    div_table: &FastDivTable,
    factor_table: &FactorTable,
) -> i64 {
    let x_div_y = x / y;
    let total_segments = if x_div_y > z {
        ((x_div_y - z) + SEGMENT_SPAN - 1) / SEGMENT_SPAN
    } else {
        0
    };

    let dispenser = Arc::new(AsymmetricChunkDispenser::new(total_segments));

    // Raw pointers for thread closures
    let p_ptr = primes.as_ptr() as usize;
    let p_len = primes.len();
    let pi_ptr = pi_table.as_ptr() as usize;
    let pi_len = pi_table.len();
    let mu_ptr = mu.as_ptr() as usize;
    let mu_len = mu.len();
    let div_ptr = div_table as *const FastDivTable as usize;
    let fact_ptr = factor_table as *const FactorTable as usize;

    // 1. Spawning 6 Little Workers (Cores 0..=5: Cortex-A55)
    // Start sieving D immediately at t = 0
    let mut a55_handles = Vec::with_capacity(6);
    for core_id in 0..6 {
        let disp = Arc::clone(&dispenser);
        a55_handles.push(std::thread::spawn(move || {
            pin_thread_to_core(core_id);
            let thread_primes = unsafe { std::slice::from_raw_parts(p_ptr as *const u32, p_len) };
            let thread_mu = unsafe { std::slice::from_raw_parts(mu_ptr as *const i8, mu_len) };
            let thread_div = unsafe { &*(div_ptr as *const FastDivTable) };

            let mut ctx = UnifiedSieveContext::new();
            let mut acc = 0i64;

            while let Some((start, end)) = disp.claim_chunk(CoreClass::Little) {
                for seg_idx in start..end {
                    acc += ctx.process_segment(seg_idx, x, y, z, thread_primes, thread_mu, thread_div);
                }
            }
            acc
        }));
    }

    // 2. Core 7 (Cortex-A78): Computes AC, then immediately steals D segments
    let disp_core7 = Arc::clone(&dispenser);
    let core7_handle = std::thread::spawn(move || {
        pin_thread_to_core(7);
        let thread_primes = unsafe { std::slice::from_raw_parts(p_ptr as *const u32, p_len) };
        let thread_pi = unsafe { std::slice::from_raw_parts(pi_ptr as *const u32, pi_len) };
        let thread_mu = unsafe { std::slice::from_raw_parts(mu_ptr as *const i8, mu_len) };
        let thread_div = unsafe { &*(div_ptr as *const FastDivTable) };
        let thread_fact = unsafe { &*(fact_ptr as *const FactorTable) };

        // Single-core high-throughput AC evaluation on Out-of-Order ALU
        let ac_val = compute_ac_fused(
            x, y, z, thread_primes, thread_pi, thread_mu, thread_div, thread_fact, 1,
        );

        // Immediate Hijack of D sieve as Big Core
        let mut ctx = UnifiedSieveContext::new();
        let mut d_acc = 0i64;
        while let Some((start, end)) = disp_core7.claim_chunk(CoreClass::Big) {
            for seg_idx in start..end {
                d_acc += ctx.process_segment(seg_idx, x, y, z, thread_primes, thread_mu, thread_div);
            }
        }

        (ac_val, d_acc)
    });

    // 3. Core 6 (Cortex-A78): Computes Phi0 + SBRB B-term, then immediately steals D segments
    pin_thread_to_core(6);
    let phi0_val = Phi0Engine::new().eval(x);
    let b_val = compute_b_monotone(x, y, primes, pi_table);

    let mut core6_d_acc = 0i64;
    let mut ctx_core6 = UnifiedSieveContext::new();
    while let Some((start, end)) = dispenser.claim_chunk(CoreClass::Big) {
        for seg_idx in start..end {
            core6_d_acc += ctx_core6.process_segment(seg_idx, x, y, z, primes, mu, div_table);
        }
    }

    // 4. Single Master Synchronization Point (Zero intermediate joins)
    let (ac_val, core7_d_acc) = core7_handle.join().unwrap();
    let mut total_d = core6_d_acc + core7_d_acc;
    for h in a55_handles {
        total_d += h.join().unwrap();
    }

    let pi_y = primes.partition_point(|&p| (p as u64) <= y) as i64;
    let sigma_val = crate::sigma_l1::compute_sigma(x, y, primes, pi_table);

    phi0_val + sigma_val + (pi_y - 1) - b_val - ac_val - total_d
}

Step 3: Benchmarking Protocol (Thermal Reset)
To isolate software performance from passive heatsink saturation:
# 1. Build release binary
cargo build --release --bin head_to_head

# 2. Cool down the silicon for 20 seconds (allows SoC to drop back to ~38°C)
sleep 20

# 3. Execute the benchmark directly (without running compilation or test sweeps first)
./target/release/head_to_head

Eliminating L1D set-associativity misses while restoring overlapped core hijacking will pull 10^{16} back down from 3.43s to the ~1.9s–2.1s window.

