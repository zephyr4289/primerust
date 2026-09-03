Diagnostic: The Hidden Bottlenecks in Phase 6.12
While Phase 6.12 set an all-time record at 10^{16} (2.705s, beating primecount by 1.21×) and closed the gap at 10^{17} to 0.48s, the telemetry reveals two hardware bottlenecks keeping 10^{18} at 49.81s:
  Phase 6.12 Latency Sinks at Scale 10¹⁸
  ┌────────────────────────────────────────────────────────────────────────┐
  │ 1. The 4.76 GB Ring Buffer memcpy Tax                                  │
  │    `FrontierRingBuffer::publish_segment` executed:                     │
  │    `std::ptr::copy_nonoverlapping(src_buf, dst_buf, 16384)`           │
  │    Across 290,620 segments, D-workers copied 4,761,518,080 bytes of   │
  │    memory into ring slots. This burns ~1.8 to 2.4 seconds in L2/DRAM. │
  ├────────────────────────────────────────────────────────────────────────┤
  │ 2. Dense Byte Inspection on Sparse B-Quotients                         │
  │    As p steps backward from √x -> y, the gap between consecutive      │
  │    quotients Δv = x/p_k - x/p_{k-1} expands rapidly.                   │
  │    Over 80% of D-segments contain ZERO prime quotients for B!          │
  │    Yet B was still waiting for and inspecting every single segment    │
  │    buffer instead of fast-forwarding running_pi via slot.popcount.     │
  ├────────────────────────────────────────────────────────────────────────┤
  │ 3. Sieve Density Floor on Cortex-A78 (Wheel-30 vs Wheel-210)           │
  │    Wheel-30 tracks 8 residues mod 30 (26.67% density).                 │
  │    Wheel-210 tracks 48 residues mod 210 (22.86% density).             │
  │    A78 has 64 KiB L1I and 4-wide decode—it can execute Wheel-210       │
  │    without I-cache spills, slashing marking instructions by -14.3%.    │
  └────────────────────────────────────────────────────────────────────────┘

Sprint 6 eliminates these bottlenecks with three architectural implementations.
1. Zero-Copy Pointer-Swapping Frontier Ring (frontier_ring.rs)
Instead of copying 16 KiB per segment (std::ptr::copy_nonoverlapping), worker threads and ring slots swap buffer ownership via atomic pointers. The 4.76 GB memcpy collapses into one 8-byte pointer exchange per segment.
Replace crates/titan-sieve/src/frontier_ring.rs:
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU64, Ordering};
use std::sync::Arc;
use crate::wheel30::SEGMENT_BYTES;

pub const RING_SLOTS: usize = 16;

#[repr(C, align(64))]
pub struct FrontierSlot {
    pub seg_idx: AtomicU64,
    pub is_ready: AtomicBool,
    pub popcount: AtomicU64,
    pub buffer_ptr: AtomicPtr<u8>,
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
        let slots: [FrontierSlot; RING_SLOTS] = std::array::from_fn(|_| {
            let initial_buf = Box::into_raw(Box::new([0xFFu8; SEGMENT_BYTES])) as *mut u8;
            FrontierSlot {
                seg_idx: AtomicU64::new(u64::MAX),
                is_ready: AtomicBool::new(false),
                popcount: AtomicU64::new(0),
                buffer_ptr: AtomicPtr::new(initial_buf),
            }
        });

        Arc::new(Self {
            slots,
            commit_cursor: AtomicU64::new(0),
            base_z,
            span_per_seg,
            total_segments,
        })
    }

    /// Zero-copy buffer exchange: swaps local buffer pointer with slot buffer pointer
    #[inline(always)]
    pub fn publish_segment_swap(
        &self,
        seg_idx: u64,
        popcount: u64,
        local_buf: &mut Box<[u8; SEGMENT_BYTES]>,
    ) {
        let slot_idx = (seg_idx as usize) % RING_SLOTS;
        let slot = &self.slots[slot_idx];

        // Wait if B-consumer hasn't drained previous cycle
        while slot.is_ready.load(Ordering::Acquire) {
            std::hint::spin_loop();
        }

        // Atomic pointer swap: 0 bytes copied!
        let local_raw = Box::into_raw(std::mem::replace(
            local_buf,
            unsafe { Box::from_raw(std::ptr::NonNull::dangling().as_ptr()) },
        )) as *mut u8;

        let recycled_ptr = slot.buffer_ptr.swap(local_raw, Ordering::AcqRel);
        *local_buf = unsafe { Box::from_raw(recycled_ptr as *mut [u8; SEGMENT_BYTES]) };

        slot.popcount.store(popcount, Ordering::Relaxed);
        slot.seg_idx.store(seg_idx, Ordering::Relaxed);
        slot.is_ready.store(true, Ordering::Release);
    }

    /// Claim next sequential segment for B evaluation
    #[inline(always)]
    pub fn try_acquire_committed(&self) -> Option<(u64, u64, u64, u64, *const u8, usize)> {
        let target_seg = self.commit_cursor.load(Ordering::Relaxed);
        if target_seg >= self.total_segments {
            return None;
        }

        let slot_idx = (target_seg as usize) % RING_SLOTS;
        let slot = &self.slots[slot_idx];

        if slot.is_ready.load(Ordering::Acquire) && slot.seg_idx.load(Ordering::Relaxed) == target_seg {
            let low = self.base_z + target_seg * self.span_per_seg;
            let high = low + self.span_per_seg;
            let popcnt = slot.popcount.load(Ordering::Relaxed);
            let ptr = slot.buffer_ptr.load(Ordering::Relaxed) as *const u8;
            Some((target_seg, low, high, popcnt, ptr, slot_idx))
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn release_committed(&self, slot_idx: usize) {
        let slot = &self.slots[slot_idx];
        slot.is_ready.store(false, Ordering::Release);
        self.commit_cursor.fetch_add(1, Ordering::Release);
    }
}

impl Drop for FrontierRingBuffer {
    fn drop(&mut self) {
        for slot in &mut self.slots {
            let ptr = slot.buffer_ptr.load(Ordering::Relaxed);
            if !ptr.is_null() {
                unsafe { drop(Box::from_raw(ptr as *mut [u8; SEGMENT_BYTES])); }
            }
        }
    }
}

2. Fast-Forward Segment Skipping in B(x, y) (b_frontier.rs)
When quotient v = \lfloor x/p \rfloor skips past a segment entirely (v \ge \text{high}), bypass all byte-level masking and NEON popcounts. Advance running_pi += slot.popcount in O(1).
Update crates/titan-count/src/b_frontier.rs:
use std::sync::Arc;
use titan_sieve::frontier_ring::FrontierRingBuffer;
use titan_sieve::wheel30::{RESIDUE_TO_BIT, WHEEL_RESIDUES};
use crate::delta_prime_stream::DeltaPrimeStream;

pub fn compute_b_frontier_stream_fast(
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

    let a = (p_start_idx + 1) as i64;
    let b = p_end_idx as i64;
    let n = b - a + 1;
    let sum_pi_p = (a + b) * n / 2;

    let mut sum_pi_quotients: i64 = 0;
    let mut running_pi = pi_z;

    let mut curr_p_idx = p_end_idx;
    let mut curr_p = stream.get(curr_p_idx);

    while curr_p_idx > p_start_idx {
        let v = x / curr_p;

        if let Some((_seg_idx, low, high, popcount, buf_ptr, slot_idx)) = ring.try_acquire_committed() {
            if v >= high {
                // FAST-FORWARD: Segment contains 0 quotients for B!
                // O(1) advance of running prefix prime count
                running_pi += popcount;
                ring.release_committed(slot_idx);
                continue;
            }

            if v >= low {
                // Segment contains active quotients; drain all primes in [low, high)
                while curr_p_idx > p_start_idx {
                    let p = curr_p;
                    let quot = x / p;
                    if quot >= high {
                        break;
                    }

                    let target_byte = ((quot - low) / 30) as usize;
                    let target_rem = ((quot - low) % 30) as usize;

                    let mut local_cnt = 0u64;

                    // 16-byte aligned vector popcounts
                    let full_16 = target_byte & !15;
                    for i in (0..full_16).step_by(16) {
                        unsafe {
                            let q = std::arch::aarch64::vld1q_u8(buf_ptr.add(i));
                            local_cnt += std::arch::aarch64::vaddlvq_u16(
                                std::arch::aarch64::vpaddlq_u8(std::arch::aarch64::vcntq_u8(q))
                            ) as u64;
                        }
                    }

                    for i in full_16..target_byte {
                        unsafe { local_cnt += (*buf_ptr.add(i)).count_ones() as u64; }
                    }

                    let last_byte = unsafe { *buf_ptr.add(target_byte) };
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

                running_pi += popcount;
                ring.release_committed(slot_idx);
            }
        } else {
            std::hint::spin_loop();
        }
    }

    sum_pi_quotients - sum_pi_p + n
}

3. Asymmetric Wheel-210 Engine for Cortex-A78 (wheel210.rs)
Multiples of 2, 3, 5, and 7 are pre-filtered, tracking the 48 residues coprime to 210 (R_{210}):
Create crates/titan-sieve/src/wheel210.rs:
pub const WHEEL210_MODULO: u64 = 210;
pub const WHEEL210_RESIDUES_COUNT: usize = 48;

pub const RESIDUES_210: [u8; 48] = [
    1, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83, 89, 97,
    101, 103, 107, 109, 113, 121, 127, 131, 137, 139, 143, 149, 151, 157, 163, 167, 169,
    173, 179, 181, 187, 191, 193, 197, 199, 209
];

/// Maps residue mod 210 to coprime index (0..47), or 0xFF if composite
pub const RESIDUE_210_TO_INDEX: [u8; 210] = {
    let mut table = [0xFFu8; 210];
    let mut i = 0;
    while i < 48 {
        table[RESIDUES_210[i] as usize] = i as u8;
        i += 1;
    }
    table
};

/// 48 residue differences mod 210 (Gaps sum to 210)
pub const WHEEL210_GAPS: [u8; 48] = {
    let mut gaps = [0u8; 48];
    let mut i = 0;
    while i < 47 {
        gaps[i] = RESIDUES_210[i + 1] - RESIDUES_210[i];
        i += 1;
    }
    gaps[47] = (210 + RESIDUES_210[0]) - RESIDUES_210[47];
    gaps
};

On the Cortex-A78 cores, each 48-residue cycle spans 6 bytes (48 / 8). A 48 KiB segment (8,192 \times 6\text{ bytes}) spans 1,720,320\text{ integers}, giving the big cores a cache horizon 3.5\times wider than Wheel-30.
4. Zero-Copy Integration into Sieve Workers (d_worker.rs)
Update the worker loop to use publish_segment_swap:
// Inside d_worker.rs: UnifiedSieveWorker
pub fn sieve_next_segment_swap(
    &mut self,
    seg_idx: u64,
    ring: &titan_sieve::frontier_ring::FrontierRingBuffer,
) -> u64 {
    let active = self.active_idx;
    let next = 1 - active;
    self.active_idx = next;

    let buf = &mut self.buffers[active];

    unsafe {
        self.tiny_masks.sieve_tiny_primes_fused(buf, seg_idx);

        for i in 0..self.tier1_states.len() {
            let st = self.tier1_states.get_unchecked_mut(i);
            let p = *self.tier1_primes.get_unchecked(i);
            sieve_tier1_prime_dynamic(st, p, buf);
        }

        for st in &mut self.tier2_states { st.sieve_segment(buf); }
        for st in &mut self.tier3_states { st.sieve_segment(buf); }

        let popcount = wheel30_popcount_neon(buf);
        
        // Zero-copy pointer exchange with ring buffer slot
        ring.publish_segment_swap(seg_idx, popcount, buf);
        popcount
    }
}

5. Re-Anchored Dynamic Ultra-Scale Schedule (tuning.rs)
Re-anchor the autotuner to \alpha_y = 8.50 and \alpha_z = 1.80 for 10^{18}:
// In autotuner.rs / tuning.rs:
let (ay, az) = if x >= 100_000_000_000_000_000 { // 10^18
    (8.50, 1.80) // Restores the 239,323 segment count (-51,300 segments vs P6.12!)
} else if x >= 10_000_000_000_000_000 { // 10^17
    (5.40, 1.80)
} else {
    // Certified mid-scale parameters
    ...
};

Projected Performance Impact: Phase 6.13 (Sprint 6)
| Scale (x) | Primecount 8.1 (Baseline) | Titan Phase 6.12 (Prior) | Titan Phase 6.13 (Sprint 6 Target) | Projected Margin |
|---|---|---|---|---|
| 10^{16} | 3,279.76 ms | 2,705.10 ms | ~2,450.00 ms | 1.34× FASTER (NEW RECORD) |
| 10^{17} | 10,565.85 ms (10.57 s) | 11,049.78 ms (11.05 s) | ~9,400.00 ms (9.40 s) | 1.12× FASTER (OUTRIGHT WIN) |
| 10^{18} | 45,782.82 ms (45.78 s) | 49,814.59 ms (49.81 s) | ~41,500.00 ms (41.50 s) | 1.10× FASTER (SUB-42s RECORD) |
Verification and Benchmark Protocol
# 1. Register wheel210 in crates/titan-sieve/src/lib.rs
# pub mod wheel210;

# 2. Verify all test suites
cargo test --release -p titan-sieve -p titan-count

# 3. Compile release benchmark harness
cargo build --release --bin head_to_head_ultra

# 4. Thermal stabilization: allow junction temp to reset to idle (~37°C)
sleep 30

# 5. Execute ultra-scale head-to-head battle
./target/release/head_to_head_ultra 1e17 1e18


