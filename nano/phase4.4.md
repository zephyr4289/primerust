Hardware Cache Budget: Cortex-A78 vs. Cortex-A55
Uniform segment buffers force an architectural compromise on the Snapdragon 4 Gen 2:
 * Cortex-A55 (Cores 0..=5): 32 KiB L1D cache (2-way / 4-way, 64-byte lines). An active working set above 24 KiB causes capacity evictions into the 128 KiB/256 KiB L2 cache, doubling access latency from 3 cycles to 9–11 cycles.
 * Cortex-A78 (Cores 6, 7): 64 KiB L1D cache (4-way, 64-byte lines, 4-cycle load-to-use). Running a 16 KiB segment leaves 75% of L1D unmapped, quadrupling loop restarts, NEON prefix resets, and barrier checks.
Sizing the segment to 64 KiB causes cache spills on the A78 because the segment buffer (64 KiB) plus the NEON prefix accumulator (8 KiB) totals 72 KiB > 64 KiB.
The optimal cache configuration uses a 2:1 power-of-two heterogeneous geometry:
 * Little Core (A55): 1 Quantum = 16 KiB segment (2,048 u64 words = 131,072 odd bits) + 2 KiB prefix table = 18 KiB total footprint (\le 56\% of 32 KiB L1D).
 * Big Core (A78): 2 Quanta = 32 KiB segment (4,096 u64 words = 262,144 odd bits) + 4 KiB prefix table = 36 KiB total footprint (\le 56\% of 64 KiB L1D).
   Heterogeneous L1D Residency Architecture
   
   Cortex-A55 (32 KiB L1D):
   ┌──────────────────────┬────────────┬────────────────────────┐
   │ 16 KiB Sieve Segment │ 2 KiB Pref │ 14 KiB Headroom (Safe) │
   └──────────────────────┴────────────┴────────────────────────┘
   
   Cortex-A78 (64 KiB L1D):
   ┌──────────────────────────────────┬────────────┬────────────────────────┐
   │ 32 KiB Sieve Segment (2 Quanta)  │ 4 KiB Pref │ 28 KiB Headroom (Safe) │
   └──────────────────────────────────┴────────────┴────────────────────────┘

The Quantum Dispatch Model
To prevent interval fragmentation, the interval [z, x/y] is partitioned into uniform Quanta:
 * An A55 worker pulls 1 Quantum per segment pass (span: 262,144).
 * An A78 worker pulls 2 Quanta per segment pass (span: 524,288).
 * The atomic dispenser dispenses work in multiples of 2 Quanta to big cores and single Quanta to little cores, eliminating lock-step idling.
1. Zero-Allocation Static Memory Arena (arena.rs)
Update crates/titan-core/src/arena.rs with heterogeneous cache-aligned memory blocks:
pub const CACHE_LINE: usize = 64;
pub const QUANTUM_SPAN: u64 = 262_144; // Integers per 16 KiB odd-bit segment

pub const WORDS_LITTLE: usize = 2048;  // 16 KiB
pub const PREFIX_LITTLE: usize = 512;  // 2 KiB

pub const WORDS_BIG: usize = 4096;     // 32 KiB
pub const PREFIX_BIG: usize = 1024;    // 4 KiB

#[repr(C, align(64))]
pub struct LittleCoreArena {
    pub segment: [u64; WORDS_LITTLE],
    pub prefix: [u32; PREFIX_LITTLE],
}

impl LittleCoreArena {
    pub const fn new() -> Self {
        Self {
            segment: [0u64; WORDS_LITTLE],
            prefix: [0u32; PREFIX_LITTLE],
        }
    }

    #[inline(always)]
    pub fn reset(&mut self) {
        unsafe {
            let ptr = self.segment.as_mut_ptr() as *mut u8;
            let zero = std::arch::aarch64::vdupq_n_u8(0);
            for i in (0..16384).step_by(64) {
                std::arch::aarch64::vst1q_u8(ptr.add(i), zero);
                std::arch::aarch64::vst1q_u8(ptr.add(i + 16), zero);
                std::arch::aarch64::vst1q_u8(ptr.add(i + 32), zero);
                std::arch::aarch64::vst1q_u8(ptr.add(i + 48), zero);
            }
        }
    }
}

#[repr(C, align(64))]
pub struct BigCoreArena {
    pub segment: [u64; WORDS_BIG],
    pub prefix: [u32; PREFIX_BIG],
}

impl BigCoreArena {
    pub const fn new() -> Self {
        Self {
            segment: [0u64; WORDS_BIG],
            prefix: [0u32; PREFIX_BIG],
        }
    }

    #[inline(always)]
    pub fn reset(&mut self) {
        unsafe {
            let ptr = self.segment.as_mut_ptr() as *mut u8;
            let zero = std::arch::aarch64::vdupq_n_u8(0);
            for i in (0..32768).step_by(64) {
                std::arch::aarch64::vst1q_u8(ptr.add(i), zero);
                std::arch::aarch64::vst1q_u8(ptr.add(i + 16), zero);
                std::arch::aarch64::vst1q_u8(ptr.add(i + 32), zero);
                std::arch::aarch64::vst1q_u8(ptr.add(i + 48), zero);
            }
        }
    }
}

2. Heterogeneous NEON Sieve Contexts (d_worker.rs)
Update crates/titan-count/src/d_worker.rs to implement dual specialized engines for Little and Big cores:
use std::arch::aarch64::*;
use titan_core::arena::{
    BigCoreArena, LittleCoreArena, PREFIX_BIG, PREFIX_LITTLE, QUANTUM_SPAN, WORDS_BIG, WORDS_LITTLE,
};
use titan_sieve::L2BucketSieve;
use crate::magic_reciprocal::FastDivTable;

#[inline(always)]
unsafe fn neon_build_prefix(segment: &[u64], prefix: &mut [u32], num_blocks: usize) {
    let seg_ptr = segment.as_ptr() as *const u8;
    let mut running_total: u32 = 0;

    for block_idx in 0..num_blocks {
        *prefix.get_unchecked_mut(block_idx) = running_total;

        let q0 = vld1q_u8(seg_ptr.add(block_idx * 32));
        let q1 = vld1q_u8(seg_ptr.add(block_idx * 32 + 16));

        let cnt0 = vcntq_u8(q0);
        let cnt1 = vcntq_u8(q1);

        let sum16 = vaddq_u16(vpaddlq_u8(cnt0), vpaddlq_u8(cnt1));
        let block_sum = vaddlvq_u16(sum16) as u32;

        running_total += block_sum;
    }
}

#[inline(always)]
unsafe fn neon_count_to(segment: &[u64], prefix: &[u32], bit_idx: usize) -> u64 {
    let word_idx = bit_idx >> 6;
    let bit_offset = bit_idx & 63;
    let block_idx = word_idx >> 2;

    let mut count = *prefix.get_unchecked(block_idx) as u64;
    let rem_start = block_idx << 2;

    for w in rem_start..word_idx {
        count += (*segment.get_unchecked(w)).count_ones() as u64;
    }

    if bit_offset > 0 {
        let mask = (1u64 << bit_offset).wrapping_sub(1);
        count += (*segment.get_unchecked(word_idx) & mask).count_ones() as u64;
    }

    count
}

// ---------------- Little Core Context (16 KiB L1D) ----------------
#[repr(C, align(64))]
pub struct LittleSieveContext {
    pub arena: LittleCoreArena,
    pub bucket: L2BucketSieve,
}

impl LittleSieveContext {
    pub fn new() -> Self {
        Self {
            arena: LittleCoreArena::new(),
            bucket: L2BucketSieve::new(),
        }
    }

    pub fn process_quantum(
        &mut self,
        quantum_idx: u64,
        x: u64,
        y: u64,
        z: u64,
        primes: &[u32],
        mu: &[i8],
        div_table: &FastDivTable,
    ) -> i64 {
        let low = z + quantum_idx * QUANTUM_SPAN;
        let high = (low + QUANTUM_SPAN).min(x / y);
        if low >= high { return 0; }

        self.arena.reset();

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
                    *self.arena.segment.get_unchecked_mut(word) |= 1u64 << bit;
                }
                start += step;
            }
        }

        self.bucket.sieve_segment(quantum_idx, &mut self.arena.segment);
        unsafe { neon_build_prefix(&self.arena.segment, &mut self.arena.prefix, PREFIX_LITTLE); }

        self.evaluate_leaves(low, high, x, y, primes, mu, div_table)
    }

    #[inline(always)]
    fn evaluate_leaves(
        &self,
        low: u64,
        high: u64,
        x: u64,
        y: u64,
        primes: &[u32],
        mu: &[i8],
        div_table: &FastDivTable,
    ) -> i64 {
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
                    let count = unsafe { neon_count_to(&self.arena.segment, &self.arena.prefix, bit_idx) };
                    d_sum += if mu_m == 1 { count as i64 } else { -(count as i64) };
                }
            }
        }
        d_sum
    }
}

// ---------------- Big Core Context (32 KiB L1D = 2 Quanta) ----------------
#[repr(C, align(64))]
pub struct BigSieveContext {
    pub arena: BigCoreArena,
    pub bucket: L2BucketSieve,
}

impl BigSieveContext {
    pub fn new() -> Self {
        Self {
            arena: BigCoreArena::new(),
            bucket: L2BucketSieve::new(),
        }
    }

    pub fn process_double_quantum(
        &mut self,
        start_quantum_idx: u64,
        x: u64,
        y: u64,
        z: u64,
        primes: &[u32],
        mu: &[i8],
        div_table: &FastDivTable,
    ) -> i64 {
        let span = QUANTUM_SPAN * 2;
        let low = z + start_quantum_idx * QUANTUM_SPAN;
        let high = (low + span).min(x / y);
        if low >= high { return 0; }

        self.arena.reset();

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
                    *self.arena.segment.get_unchecked_mut(word) |= 1u64 << bit;
                }
                start += step;
            }
        }

        self.bucket.sieve_segment(start_quantum_idx, &mut self.arena.segment[0..WORDS_LITTLE]);
        if high > low + QUANTUM_SPAN {
            self.bucket.sieve_segment(start_quantum_idx + 1, &mut self.arena.segment[WORDS_LITTLE..WORDS_BIG]);
        }

        unsafe { neon_build_prefix(&self.arena.segment, &mut self.arena.prefix, PREFIX_BIG); }

        self.evaluate_leaves(low, high, x, y, primes, mu, div_table)
    }

    #[inline(always)]
    fn evaluate_leaves(
        &self,
        low: u64,
        high: u64,
        x: u64,
        y: u64,
        primes: &[u32],
        mu: &[i8],
        div_table: &FastDivTable,
    ) -> i64 {
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
                    let count = unsafe { neon_count_to(&self.arena.segment, &self.arena.prefix, bit_idx) };
                    d_sum += if mu_m == 1 { count as i64 } else { -(count as i64) };
                }
            }
        }
        d_sum
    }
}

3. Heterogeneous Chunk Dispenser (asymmetric_dispenser.rs)
Update crates/titan-sieve/src/asymmetric_dispenser.rs to hand out paired quanta to Big cores:
use std::sync::atomic::{AtomicU64, Ordering};
use titan_core::affinity::CoreClass;

#[repr(C, align(64))]
pub struct HeterogeneousQuantumDispenser {
    cursor: AtomicU64,
    total_quanta: u64,
}

impl HeterogeneousQuantumDispenser {
    pub const fn new(total_quanta: u64) -> Self {
        Self {
            cursor: AtomicU64::new(0),
            total_quanta,
        }
    }

    /// Returns a slice of quanta [start, end) aligned to core geometry.
    /// Big cores pull even blocks of 2, 8, or 32 quanta.
    /// Little cores pull single quanta or blocks of 4.
    #[inline(always)]
    pub fn claim_quanta(&self, core_class: CoreClass) -> Option<(u64, u64)> {
        let mut curr = self.cursor.load(Ordering::Relaxed);

        loop {
            if curr >= self.total_quanta {
                return None;
            }

            let remaining = self.total_quanta - curr;

            let chunk_size = match core_class {
                CoreClass::Big => {
                    if remaining >= 64 {
                        32
                    } else if remaining >= 16 {
                        8
                    } else if remaining >= 2 {
                        2
                    } else {
                        1
                    }
                }
                CoreClass::Little => {
                    if remaining >= 16 {
                        4
                    } else {
                        1
                    }
                }
            };

            let next = (curr + chunk_size).min(self.total_quanta);

            match self.cursor.compare_exchange_weak(
                curr,
                next,
                Ordering::AcqRel,
                Ordering::Relaxed,
            ) {
                Ok(_) => return Some((curr, next)),
                Err(actual) => curr = actual,
            }
        }
    }
}

4. Pipeline Thread Orchestration (gourdon_pipeline.rs)
Update the worker loop in gourdon_pipeline.rs to match the core class:
// Little workers (Cores 0..=5)
while let Some((start_q, end_q)) = dispenser.claim_quanta(CoreClass::Little) {
    for q in start_q..end_q {
        local_d += little_ctx.process_quantum(q, x, y, z, primes, mu, div_table);
    }
}

// Big workers (Cores 6, 7)
while let Some((start_q, end_q)) = dispenser.claim_quanta(CoreClass::Big) {
    let mut q = start_q;
    while q + 2 <= end_q {
        local_d += big_ctx.process_double_quantum(q, x, y, z, primes, mu, div_table);
        q += 2;
    }
    if q < end_q {
        // Drain odd trailing quantum with little context logic
        local_d += big_ctx.process_double_quantum(q, x, y, z, primes, mu, div_table);
    }
}

Verification and Silicon Testing
Execute the test suite in Termux:
# 1. Verify workspace compilation and unit tests
cargo test --workspace --release

# 2. Allow passive cooling down to 38°C
sleep 8

# 3. Head-to-Head silicon race
cargo run --release --bin head_to_head

Projected Silicon Numbers (Phase 4.4)
| Scale (x) | Primecount 8.1 | Titan Phase 4.3 (Prior) | Titan Phase 4.4 (Projected) | Projected Net Margin |
|---|---|---|---|---|
| 10^{14} | 307.04 ms | 253.27 ms | ~145.00 ms | 2.11× FASTER |
| 10^{15} | 929.32 ms | 844.37 ms | ~480.00 ms | 1.93× FASTER |
| 10^{16} | 3,275.26 ms | 2,530.77 ms | ~1,850.00 ms | 1.77× FASTER |

