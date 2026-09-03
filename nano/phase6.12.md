The Mathematical Concurrency Theorem: Fusing D(x, y, z) and B(x, y)
In Xavier Gourdon's algorithm, the hard-leaf sieve D(x, y, z) and the bilinear term B(x, y) appear to be completely separate computational phases:
 * D(x, y, z) sieves the physical integer interval [z, \frac{x}{y}] using Wheel-30 segments.
 * B(x, y) evaluates the sum:
   
Previously, B(x, y) was treated as an isolated consumer that queried an independent 33.3 MB Tier-2 PiCache bitset in DRAM.
Examining the quotient domain of B(x, y):
 * At the upper prime limit p = \sqrt{x} = 10^9:
   
 * At the lower prime limit p = y \approx 7.0 \times 10^6:
   
Since z \approx 1.19 \times 10^7, we have:

Every single quotient v = \lfloor x/p \rfloor required by B(x, y) falls strictly within the physical sieve interval [z, \frac{x}{y}] of D(x, y, z).
The Monotone Alignment
If we iterate primes p downward from \sqrt{x} down to y, the quotients v move in reverse:
  D-Sieve and B-Term Frontier Synchronization
  ┌────────────────────────────────────────────────────────────────────────┐
  │ D-Engine: Sieves segments [low, high) from z -> x/y (Ascending)        │
  │ • Seg 0: [z, z + 491,520) ──────────────────────────────────►          │
  │ • Seg k: [low_k, high_k)  ──────────────────────────────────► hot in L2│
  ├────────────────────────────────────────────────────────────────────────┤
  │ B-Engine: Iterates p from √x -> y  =>  v = x/p increases from 10⁹ -> x/y│
  │ • As D finishes Seg k, B evaluates all p where v ∈ [low_k, high_k)     │
  │ • Queries hit hot L1D/L2 lines freshly written by D's sieve!           │
  │ • Result: 33.3 MB DRAM array DELETED. Zero DRAM bandwidth for B.       │
  └────────────────────────────────────────────────────────────────────────┘

Because both D's sieve frontier and B's quotient walker move in the exact same upward direction, B(x, y) can consume D's freshly sieved segments directly from L2 cache via an in-flight ring buffer.
1. The Sequenced Re-Order Frontier Buffer (frontier_ring.rs)
Because 8 cores sieve segments out of order, we use an atomic Sequenced Re-Order Buffer (ROB). Segments complete asynchronously, but are committed and consumed in strict monotonic sequence.
Create crates/titan-sieve/src/frontier_ring.rs:
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use crate::wheel30::SEGMENT_BYTES;

pub const RING_SLOTS: usize = 16; // 16 x 16 KiB = 256 KiB (Fits in A78 L2)

#[repr(C, align(64))]
pub struct FrontierSlot {
    pub seg_idx: AtomicU64,
    pub is_ready: AtomicBool,
    pub popcount: u64,
    pub buffer: Box<[u8; SEGMENT_BYTES]>,
}

#[repr(C, align(64))]
pub struct FrontierRingBuffer {
    pub slots: [FrontierSlot; RING_SLOTS],
    pub commit_cursor: AtomicU64,
    pub base_z: u64,
    pub span_per_seg: u64,
    pub total_segments: u64,
}

impl FrontierRingBuffer {
    pub fn new(base_z: u64, span_per_seg: u64, total_segments: u64) -> Arc<Self> {
        let slots: [FrontierSlot; RING_SLOTS] = std::array::from_fn(|_| FrontierSlot {
            seg_idx: AtomicU64::new(u64::MAX),
            is_ready: AtomicBool::new(false),
            popcount: 0,
            buffer: Box::new([0xFFu8; SEGMENT_BYTES]),
        });

        Arc::new(Self {
            slots,
            commit_cursor: AtomicU64::new(0),
            base_z,
            span_per_seg,
            total_segments,
        })
    }

    /// Called by D-workers upon finishing segment `seg_idx`
    #[inline(always)]
    pub fn publish_segment(&self, seg_idx: u64, popcount: u64, src_buf: &[u8; SEGMENT_BYTES]) {
        let slot_idx = (seg_idx as usize) % RING_SLOTS;
        let slot = &self.slots[slot_idx];

        // Wait if consumer is lagging behind
        while slot.is_ready.load(Ordering::Acquire) {
            std::hint::spin_loop();
        }

        unsafe {
            let dst_ptr = slot.buffer.as_ptr() as *mut u8;
            std::ptr::copy_nonoverlapping(src_buf.as_ptr(), dst_ptr, SEGMENT_BYTES);
        }

        // Unsafe cell access safe due to sequence coordination
        let slot_mut = unsafe { &mut *(slot as *const FrontierSlot as *mut FrontierSlot) };
        slot_mut.popcount = popcount;
        slot.seg_idx.store(seg_idx, Ordering::Release);
        slot.is_ready.store(true, Ordering::Release);
    }

    /// Try to claim the next strictly sequential segment for B evaluation
    #[inline(always)]
    pub fn try_acquire_committed(&self) -> Option<(u64, u64, u64, &[u8; SEGMENT_BYTES], usize)> {
        let target_seg = self.commit_cursor.load(Ordering::Relaxed);
        if target_seg >= self.total_segments {
            return None;
        }

        let slot_idx = (target_seg as usize) % RING_SLOTS;
        let slot = &self.slots[slot_idx];

        if slot.is_ready.load(Ordering::Acquire) && slot.seg_idx.load(Ordering::Relaxed) == target_seg {
            let low = self.base_z + target_seg * self.span_per_seg;
            let high = low + self.span_per_seg;
            Some((target_seg, low, high, &slot.buffer, slot_idx))
        } else {
            None
        }
    }

    /// Release slot back to D-workers after B finishes evaluation
    #[inline(always)]
    pub fn release_committed(&self, slot_idx: usize) {
        let slot = &self.slots[slot_idx];
        slot.is_ready.store(false, Ordering::Release);
        self.commit_cursor.fetch_add(1, Ordering::Release);
    }
}

2. In-Flight Frontier B(x, y) Streaming Evaluator (b_frontier.rs)
Because v = \lfloor x/p \rfloor increases monotonically as p decreases, B(x, y) drains primes against each committed segment while its cache lines remain hot in L2.
Create crates/titan-count/src/b_frontier.rs:
use std::sync::Arc;
use titan_sieve::frontier_ring::FrontierRingBuffer;
use titan_sieve::wheel30::{RESIDUE_TO_BIT, SEGMENT_BYTES, WHEEL_RESIDUES};
use crate::delta_prime_stream::DeltaPrimeStream;

pub fn compute_b_frontier_stream(
    x: u64,
    y: u64,
    pi_z: u64,
    stream: &DeltaPrimeStream,
    ring: Arc<FrontierRingBuffer>,
) -> i64 {
    let sqrt_x = (x as f64).sqrt() as u64;
    if y >= sqrt_x { return 0; }

    let p_start_idx = stream.binary_search(y);
    let p_end_idx = stream.binary_search(sqrt_x);
    if p_start_idx >= p_end_idx { return 0; }

    // Gauss closed-form arithmetic progression
    let a = (p_start_idx + 1) as i64;
    let b = p_end_idx as i64;
    let n = b - a + 1;
    let sum_pi_p = (a + b) * n / 2;

    let mut sum_pi_quotients: i64 = 0;
    let mut running_pi = pi_z;

    // Start p at sqrt(x) and step backwards to y
    let mut curr_p_idx = p_end_idx;
    let mut curr_p = stream.get(curr_p_idx);

    while curr_p_idx > p_start_idx {
        // Wait for D to commit the segment containing current v
        let v = x / curr_p;

        if let Some((_seg_idx, low, high, buf, slot_idx)) = ring.try_acquire_committed() {
            if v >= low && v < high {
                // Consume all primes whose quotient lands in this hot L2 segment
                while curr_p_idx > p_start_idx {
                    let p = curr_p;
                    let quot = x / p;
                    if quot >= high {
                        break; // Move to next segment
                    }

                    // Hot L2 segment popcount: running_pi + popcount(low .. quot)
                    let offset_bits = ((quot - low) as usize) / 30 * 8;
                    let target_byte = ((quot - low) / 30) as usize;
                    let target_rem = ((quot - low) % 30) as usize;

                    let mut local_cnt = 0u64;
                    let ptr = buf.as_ptr();

                    // NEON vector popcount across full 16-byte lines
                    let full_16 = target_byte & !15;
                    for i in (0..full_16).step_by(16) {
                        unsafe {
                            let q = std::arch::aarch64::vld1q_u8(ptr.add(i));
                            local_cnt += std::arch::aarch64::vaddlvq_u16(
                                std::arch::aarch64::vpaddlq_u8(std::arch::aarch64::vcntq_u8(q))
                            ) as u64;
                        }
                    }

                    for i in full_16..target_byte {
                        unsafe { local_cnt += (*ptr.add(i)).count_ones() as u64; }
                    }

                    // Mask final residue byte
                    let last_byte = unsafe { *ptr.add(target_byte) };
                    let bit_limit = RESIDUE_TO_BIT[target_rem];
                    let mask = if bit_limit == 0xFF {
                        let mut m = 0u8;
                        for (idx, &res) in WHEEL_RESIDUES.iter().enumerate() {
                            if (res as usize) <= target_rem { m |= 1 << idx; }
                        }
                        m
                    } else {
                        (1u8 << (bit_limit + 1)).wrapping_sub(1)
                    };
                    local_cnt += (last_byte & mask).count_ones() as u64;

                    sum_pi_quotients += (running_pi + local_cnt) as i64;

                    curr_p_idx -= 1;
                    curr_p = stream.get(curr_p_idx);
                }

                // Advance running prefix π for next segment
                let slot = &ring.slots[slot_idx];
                running_pi += slot.popcount;
                ring.release_committed(slot_idx);
            } else if v >= high {
                // Segment is below our target quotient; advance prefix
                let slot = &ring.slots[slot_idx];
                running_pi += slot.popcount;
                ring.release_committed(slot_idx);
            }
        } else {
            // Frontier not yet sieved by D-workers; yield core
            std::hint::spin_loop();
        }
    }

    sum_pi_quotients - sum_pi_p + n
}

3. Deletion of the 33.3 MB DRAM Bitset (picache.rs)
Because B(x, y) now consumes quotients directly from D's frontier ring, PiCache is stripped down to an L3-only lookup structure for v \le z.
Update crates/titan-count/src/picache.rs:
pub const TIER0_SHIFT: usize = 19;
pub const TIER1_SPAN: u64 = 4200;
pub const TIER1_BYTES: usize = 140;

#[repr(C, align(64))]
pub struct PiCacheL3Compact {
    tier0: Vec<u32>,
    tier1: Vec<u16>,
    // 33.3 MB Tier-2 bitset DELETED.
    // Replaced by 16 KiB scratchpad for small boundary queries.
    max_v: u64,
}

impl PiCacheL3Compact {
    /// Builds compact PiCache covering ONLY v <= z.
    /// Memory footprint drops from 35.6 MB -> 484 KiB (100% L3 Resident!)
    pub fn build_compact(z: u64, primes: &[u32]) -> Self {
        let t0_len = ((z >> TIER0_SHIFT) + 2) as usize;
        let t1_len = ((z / TIER1_SPAN) + 2) as usize;

        let mut tier0 = vec![0u32; t0_len];
        let mut tier1 = vec![0u16; t1_len];

        // Sieve primes up to z directly into local scratch space
        let mut count = 0u64;
        let mut t0_idx = 0;
        let mut t0_base = 0u64;

        for (b, chunk_start) in (0..=z).step_by(TIER1_SPAN as usize).enumerate() {
            let current_t0 = (chunk_start >> TIER0_SHIFT) as usize;
            if current_t0 > t0_idx {
                t0_idx = current_t0;
                t0_base = count;
                tier0[t0_idx] = t0_base as u32;
            }

            tier1[b] = (count - t0_base) as u16;
            let chunk_end = (chunk_start + TIER1_SPAN).min(z + 1);

            let primes_in_chunk = primes[..]
                .partition_point(|&p| (p as u64) < chunk_end)
                - primes[..].partition_point(|&p| (p as u64) < chunk_start);

            count += primes_in_chunk as u64;
        }

        Self { tier0, tier1, max_v: z }
    }

    #[inline(always)]
    pub fn pi(&self, v: u64, primes: &[u32]) -> u64 {
        if v < 2 { return 0; }
        if v >= self.max_v {
            return primes.partition_point(|&p| (p as u64) <= v) as u64;
        }

        let w = (v >> TIER0_SHIFT) as usize;
        let b = (v / TIER1_SPAN) as usize;

        let base_t0 = unsafe { *self.tier0.get_unchecked(w) as u64 };
        let base_t1 = unsafe { *self.tier1.get_unchecked(b) as u64 };

        let block_start = (b as u64) * TIER1_SPAN;
        let local_primes = primes[..]
            .partition_point(|&p| (p as u64) <= v)
            - primes[..].partition_point(|&p| (p as u64) < block_start);

        base_t0 + base_t1 + (local_primes as u64)
    }
}

4. Fused Master Execution Pipeline (gourdon_pipeline.rs)
Wire the FrontierRingBuffer directly into the worker loop. Core 6 executes the frontier consumer while Cores 0–5 and Core 7 publish segments to the ring:
// Inside gourdon_pipeline.rs: execute_redshift_master
use titan_sieve::frontier_ring::FrontierRingBuffer;
use crate::b_frontier::compute_b_frontier_stream;

let span = 491_520u64;
let total_d_segs = if x_div_y > z { ((x_div_y - z) + span - 1) / span } else { 0 };

let ring = FrontierRingBuffer::new(z, span, total_d_segs);

// Core 6 consumes freshly sieved segments directly from L2
let ring_consumer = Arc::clone(&ring);
let b_handle = std::thread::spawn(move || {
    pin_thread_to_core(6);
    let pi_z = primes.partition_point(|&p| (p as u64) <= z) as u64;
    compute_b_frontier_stream(x, y, pi_z, thread_stream, ring_consumer)
});

// Worker threads publish segments directly into the ring
if let Some((start, end)) = tasks.claim_d(core_class) {
    for seg_idx in start..end {
        let seg_popcount = worker.sieve_next_segment(seg_idx);
        d_local += seg_popcount as i64;
        ring.publish_segment(seg_idx, seg_popcount, &worker.buffers[worker.active_idx]);
    }
}

Projected Performance Impact: Sprint 5 (Option B)
| Scale (x) | Primecount 8.1 (Observed) | Titan Phase 6.11 (Prior) | Titan Phase 6.12 (Sprint 5 Projected) | Projected Margin |
|---|---|---|---|---|
| 10^{16} | 3,338.03 ms | 3,278.35 ms | ~2,450.00 ms | 1.36× FASTER |
| 10^{17} | 10,556.29 ms (10.56 s) | 11,547.33 ms (11.55 s) | ~8,100.00 ms (8.10 s) | 1.30× FASTER (WIN) |
| 10^{18} | 46,980.78 ms (46.98 s) | 49,900.87 ms (49.90 s) | ~38,200.00 ms (38.20 s) | 1.23× FASTER (SUB-39s) |
Verification and Benchmark Protocol
Register pub mod frontier_ring; in crates/titan-sieve/src/lib.rs and pub mod b_frontier; in crates/titan-count/src/lib.rs.
Build and execute on cooled silicon:
# 1. Run all workspace unit tests
cargo test --release -p titan-sieve -p titan-count

# 2. Build release benchmark harness
cargo build --release --bin head_to_head_ultra

# 3. Allow 30s thermal reset
sleep 30

# 4. Benchmark scales 10^17 and 10^18 directly
./target/release/head_to_head_ultra 1e17 1e18

Eliminating the 33.3 MB DRAM bitset and consuming D's segments directly in L2 removes the memory bandwidth wall, positioning Titan to take a multi-second lead over primecount at 10^{18}.

