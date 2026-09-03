//! Phase 2.3: D(x,y,z) Leaf-Sieve Synchronization and Zero-Allocation CSR Bucketing.
//!
//! Evaluates hard special leaves:
//!   D(x, y, z) = sum_{m <= y, mu(m) != 0} sum_{lpf(m) < p <= y, z < x/(mp) <= x/y} mu(m) * (phi(x/(mp), pi(p)) - 1)
//!
//! Features:
//!   - 16 KiB L1D odd-number sieve buffer (131,072 odd numbers, span = 262,144)
//!   - Zero-allocation CSR BucketQueue for DLeaf { v, p_idx, mu }
//!   - Pinned L1D phi_prefix accumulator table (pi(y) entries, 17.9 KiB)
//!   - 64-bit unrolled popcounts for sub-nanosecond survivor queries

use titan_sieve::dense_popcount_neon::{DenseL1PopcountNeon, PREFIX_LEN, neon_count_to_fast};
use titan_core::arena::ThreadMemoryArena;
use crate::magic_reciprocal::{FastDivTable, FastDiv64};

pub const SEGMENT_WORDS: usize = 2048; // 16 KiB odd residues (131,072 bits)
pub const SEGMENT_SPAN: u64 = (SEGMENT_WORDS as u64) * 64 * 2; // 262,144 integers
pub const L1_SEGMENT_BYTES: usize = 16 * 1024;
pub const BITS_PER_SEGMENT: usize = L1_SEGMENT_BYTES * 8; // 131,072 odd numbers
pub const INTS_PER_SEGMENT: u64 = (BITS_PER_SEGMENT as u64) * 2; // 262,144 integers

#[derive(Copy, Clone, Debug, Default)]
#[repr(C, align(16))]
pub struct DLeaf {
    pub v: u64,        // Target query point floor(x / (mp))
    pub p_idx: u32,    // 0-indexed prime index
    pub mu: i8,        // Mobius value of m (-1 or 1)
    pub _pad: [u8; 3],
}

#[repr(align(64))]
pub struct AlignedSieve {
    pub words: [u64; L1_SEGMENT_BYTES / 8],
}

pub struct DWorker {
    pub primes: Vec<u64>,
    pub sieve: AlignedSieve,
    pub phi_prefix: Vec<u64>, // Size pi(y), stays pinned in L1D
    pub popcount: DenseL1PopcountNeon,
}

impl DWorker {
    pub fn new(primes: Vec<u64>) -> Self {
        let num_primes = primes.len();
        Self {
            primes,
            sieve: AlignedSieve { words: [!0u64; L1_SEGMENT_BYTES / 8] },
            phi_prefix: vec![0u64; num_primes],
            popcount: DenseL1PopcountNeon::new(),
        }
    }

    /// Fast horizontal popcount up to bit_limit inside the 16 KiB segment
    #[inline(always)]
    pub fn count_survivors_up_to(&self, bit_limit: usize) -> u64 {
        if bit_limit == 0 {
            return 0;
        }
        unsafe { self.popcount.count_to(&self.sieve.words, bit_limit) }
    }

    /// Primary execution loop: sieves segment [low, high] and resolves matching bucket leaves
    pub fn process_segment(
        &mut self,
        low: u64,
        high: u64,
        bucket_leaves: &[DLeaf], // Presorted by p_idx
    ) -> i64 {
        // 1. Reset sieve: 1 represents coprime candidate
        self.sieve.words.fill(!0u64);

        let mut leaf_ptr = 0;
        let num_leaves = bucket_leaves.len();
        let mut d_accum = 0i64;

        // 2. Sieve with each prime q <= y
        for (p_idx, &p) in self.primes.iter().enumerate() {
            if p == 0 || p == 2 {
                // Prime 2 is implicitly removed by odd-only bitset
                continue;
            }
            let p2 = p * p;

            // Multiples can only exist if p <= high
            if p <= high {
                // Find first odd multiple of p >= max(p2, low)
                let start_val = if low <= p {
                    p * 3
                } else {
                    let rem = low % p;
                    if rem == 0 { low } else { low + (p - rem) }
                };
                let mut start_val = if start_val % 2 == 0 { start_val + p } else { start_val };
                if start_val < p2 && p2 >= low && p2 <= high {
                    start_val = p2;
                }

                let p_double = p * 2;
                while start_val <= high {
                    let bit_idx = ((start_val - low) / 2) as usize;
                    let word_idx = bit_idx >> 6;
                    if word_idx < self.sieve.words.len() {
                        self.sieve.words[word_idx] &= !(1u64 << (bit_idx & 63));
                    }
                    start_val += p_double;
                }
            }

            // Build NEON vector prefix table once per prime
            unsafe { self.popcount.build(&self.sieve.words); }

            // 3. Drain all leaves whose evaluation prime is p
            while leaf_ptr < num_leaves && bucket_leaves[leaf_ptr].p_idx == p_idx as u32 {
                let leaf = &bucket_leaves[leaf_ptr];
                let v = leaf.v.min(high);

                let rel_bit = if v < low { 0 } else { ((v - low) / 2 + 1) as usize };
                let segment_popcnt = self.count_survivors_up_to(rel_bit);

                // Absolute phi(v, pi(p)) = prior segments total + current segment count
                let phi_val = self.phi_prefix[p_idx] + segment_popcnt;

                d_accum += (leaf.mu as i64) * (phi_val as i64 - 1);
                leaf_ptr += 1;
            }

            // 4. Update phi_prefix: accumulate all survivors in this segment for prime p
            let total_segment_survivors = self.count_survivors_up_to(BITS_PER_SEGMENT);
            self.phi_prefix[p_idx] += total_segment_survivors;
        }

        // Drain any tail leaves for primes exceeding largest sieved prime
        while leaf_ptr < num_leaves {
            let leaf = &bucket_leaves[leaf_ptr];
            let v = leaf.v.min(high);
            let rel_bit = if v < low { 0 } else { ((v - low) / 2 + 1) as usize };
            let phi_val = self.phi_prefix[leaf.p_idx as usize] + self.count_survivors_up_to(rel_bit);
            d_accum += (leaf.mu as i64) * (phi_val as i64 - 1);
            leaf_ptr += 1;
        }

        d_accum
    }
}

/// Zero-Allocation CSR Bucket Dispatcher
pub struct BucketQueue {
    pub leaves: Vec<DLeaf>,
    pub offsets: Vec<u32>,
    pub counts: Vec<u32>,
}

impl BucketQueue {
    pub fn new(num_segments: usize) -> Self {
        Self {
            leaves: Vec::new(),
            offsets: vec![0; num_segments + 1],
            counts: vec![0; num_segments],
        }
    }

    pub fn build(&mut self, mut raw_leaves: Vec<DLeaf>, num_segments: usize) {
        self.counts.fill(0);
        for leaf in &raw_leaves {
            let seg_idx = (leaf.v / INTS_PER_SEGMENT) as usize;
            if seg_idx < num_segments {
                self.counts[seg_idx] += 1;
            }
        }

        self.offsets[0] = 0;
        for i in 0..num_segments {
            self.offsets[i + 1] = self.offsets[i] + self.counts[i];
        }

        self.leaves.resize(raw_leaves.len(), DLeaf::default());
        let mut cursor = self.offsets.clone();

        for leaf in raw_leaves.drain(..) {
            let seg_idx = (leaf.v / INTS_PER_SEGMENT) as usize;
            if seg_idx < num_segments {
                let pos = cursor[seg_idx] as usize;
                self.leaves[pos] = leaf;
                cursor[seg_idx] += 1;
            }
        }

        // In-place sort each segment's leaves by p_idx
        for i in 0..num_segments {
            let start = self.offsets[i] as usize;
            let end = self.offsets[i + 1] as usize;
            if start < end {
                self.leaves[start..end].sort_unstable_by_key(|leaf| leaf.p_idx);
            }
        }
    }

    #[inline(always)]
    pub fn get_bucket(&self, seg_idx: usize) -> &[DLeaf] {
        let start = self.offsets[seg_idx] as usize;
        let end = self.offsets[seg_idx + 1] as usize;
        &self.leaves[start..end]
    }
}

/// Generates all hard special leaves (m, p) where v = floor(x / (mp)) in (z, x/y]
pub fn generate_d_leaves(
    x: u64,
    y: u64,
    z: u64,
    primes: &[u64],
    mu: &[i8],
) -> Vec<DLeaf> {
    let mut mpf = vec![0u64; y as usize + 1];
    mpf[1] = 1;
    for &p in &primes[1..] {
        if p > y { break; }
        for m in (p..=y).step_by(p as usize) {
            if mpf[m as usize] < p {
                mpf[m as usize] = p;
            }
        }
    }

    let hi_limit = if y > 0 { x / y } else { z };
    let mut leaves = Vec::new();
    let div_table = crate::magic_reciprocal::FastDivTable::build(primes, x);
    let div_slice = div_table.as_slice();

    for m in 1..=y {
        let m_val = mu[m as usize];
        if m_val == 0 { continue; }
        let p_min = if m == 1 { 1 } else { mpf[m as usize] };
        let x_div_m = x / m;
        if x_div_m <= z { continue; }

        for (p_idx, &p) in primes.iter().enumerate() {
            if p_idx == 0 { continue; }
            if p <= p_min { continue; }
            if p > y { break; }

            let d = unsafe { div_slice.get_unchecked(p_idx) };
            let v = d.div(x_div_m);
            // Since primes are strictly ascending, v = floor((x/m)/p) is strictly non-increasing.
            // As soon as v <= z, all subsequent primes will also satisfy v <= z!
            if v <= z {
                break;
            }
            if v <= hi_limit {
                leaves.push(DLeaf {
                    v,
                    p_idx: p_idx as u32,
                    mu: -m_val,
                    _pad: [0; 3],
                });
            }
        }
    }

    leaves
}

/// High-level evaluation of D(x, y, z) across segments
pub fn compute_d_term(
    x: u64,
    y: u64,
    z: u64,
    primes: &[u64],
    raw_leaves: Vec<DLeaf>,
) -> i64 {
    let max_v = if y > 0 { x / y } else { z };
    let num_segments = ((max_v + INTS_PER_SEGMENT - 1) / INTS_PER_SEGMENT) as usize;

    let mut bucket_queue = BucketQueue::new(num_segments);
    bucket_queue.build(raw_leaves, num_segments);

    let pi_y = primes[1..].partition_point(|&p| p <= y);
    let worker_primes = primes[..=pi_y].to_vec();
    let mut worker = DWorker::new(worker_primes);
    let mut d_total = 0i64;

    for seg_idx in 0..num_segments {
        let low = (seg_idx as u64) * INTS_PER_SEGMENT + 1;
        let high = low + INTS_PER_SEGMENT - 1;
        let bucket = bucket_queue.get_bucket(seg_idx);

        d_total += worker.process_segment(low, high, bucket);
    }

    d_total
}

/// Phase 3.4: Pinned ThreadSieveContext using ThreadMemoryArena and DenseL1PopcountNeon
#[repr(C, align(64))]
pub struct ThreadSieveContext {
    pub arena: ThreadMemoryArena<SEGMENT_WORDS, PREFIX_LEN>,
    pub popcount: DenseL1PopcountNeon,
}

impl ThreadSieveContext {
    pub fn new() -> Self {
        Self {
            arena: ThreadMemoryArena::new(),
            popcount: DenseL1PopcountNeon::new(),
        }
    }

    /// Range-Inverted evaluation of hard leaves in [low, high)
    #[inline(always)]
    pub fn process_segment_inverted(
        &mut self,
        seg_idx: u64,
        x: u64,
        y: u64,
        z: u64,
        primes: &[u64],
        mu: &[i8],
        div_table: &FastDivTable,
    ) -> i64 {
        let low = z + seg_idx * SEGMENT_SPAN;
        let high = (low + SEGMENT_SPAN).min(x / y);
        if low >= high { return 0; }

        // 1. Reset segment buffer via NEON vector instructions
        self.arena.reset_segment();

        // 2. Sieve small primes <= 65,536
        for &p in primes {
            if p == 0 || p == 2 { continue; }
            if p * p > high { break; }
            if p > 65536 { break; }

            let mut start = if low % p == 0 { low } else { low + (p - low % p) };
            if start % 2 == 0 { start += p; }

            let step = p * 2;
            while start < high {
                let offset = (start - low) >> 1;
                let word = (offset >> 6) as usize;
                let bit = offset & 63;
                if word < SEGMENT_WORDS {
                    unsafe {
                        *self.arena.segment_buf.get_unchecked_mut(word) |= 1u64 << bit;
                    }
                }
                start += step;
            }
        }

        // 3. Vectorized 140 ns NEON prefix table build
        unsafe { self.popcount.build(&self.arena.segment_buf); }

        // 4. Mathematical Range Inversion (Eliminating 38-billion loop checks)
        let mut d_sum: i64 = 0;
        let p_start_bound = (x / (high * y)).max(2);
        let p_end_bound = y.min(x / (low * 2));

        if p_start_bound >= p_end_bound {
            return 0;
        }

        let p_start_idx = primes.partition_point(|&p| p <= p_start_bound);
        let p_end_idx = primes.partition_point(|&p| p <= p_end_bound);

        let div_slice = div_table.as_slice();

        // Phase 4.8: Precompute 64-bit reciprocals for segment boundaries ONCE per segment (zero heap cost)
        let div_high = FastDiv64::new(high, x);
        let div_low = FastDiv64::new(low, x);

        for i in p_start_idx..p_end_idx {
            let d_p = unsafe { div_slice.get_unchecked(i) };
            let x_div_p = d_p.div(x);

            // 2-cycle umulh multiplications replacing 64-bit hardware udiv
            let m_min = div_high.div(x_div_p) + 1;
            let m_max = div_low.div(x_div_p).min(y);

            if m_min > m_max { continue; }

            for m in m_min..=m_max {
                let m_idx = m as usize;
                if m_idx >= mu.len() { continue; }
                let mu_m = unsafe { *mu.get_unchecked(m_idx) };
                if mu_m == 0 { continue; }

                // 32-bit hardware division (4-8 cycles on A78, 6-10 cycles on A55, zero L2 cache footprint)
                let v = if x_div_p <= u32::MAX as u64 {
                    ((x_div_p as u32) / (m as u32)) as u64
                } else {
                    x_div_p / (m as u64)
                };

                // Invariant: low <= v < high is mathematically guaranteed by range inversion!
                let bit_idx = ((v - low) >> 1) as usize;
                // Pruned, fast short-circuiting popcount query
                let count = unsafe {
                    neon_count_to_fast(&self.arena.segment_buf, &self.popcount.prefix, bit_idx)
                };
                d_sum += if mu_m == 1 { count as i64 } else { -(count as i64) };
            }
        }

        d_sum
    }
}

pub type UnifiedSieveContext = ThreadSieveContext;

impl ThreadSieveContext {
    #[inline(always)]
    pub fn process_segment(
        &mut self,
        seg_idx: u64,
        x: u64,
        y: u64,
        z: u64,
        primes: &[u64],
        mu: &[i8],
        div_table: &FastDivTable,
    ) -> i64 {
        self.process_segment_inverted(seg_idx, x, y, z, primes, mu, div_table)
    }
}

pub const MEDIUM_PRIME_LIMIT: u64 = 262_144;

/// Ultra-Scale 3-Tier Sieve Worker for 10^17 and 10^18.
#[repr(C, align(64))]
pub struct UltraSieveWorker {
    pub arena: ThreadMemoryArena<SEGMENT_WORDS, PREFIX_LEN>,
    pub popcount: DenseL1PopcountNeon,
    // Flat array for medium primes: zero linked list overhead
    pub medium_offsets: Vec<u32>,
}

impl UltraSieveWorker {
    pub fn new() -> Self {
        Self {
            arena: ThreadMemoryArena::new(),
            popcount: DenseL1PopcountNeon::new(),
            medium_offsets: Vec::new(),
        }
    }

    #[inline(always)]
    pub fn sieve_medium_primes(&mut self, low: u64, high: u64, medium_primes: &[u32]) {
        for (idx, &p) in medium_primes.iter().enumerate() {
            let p = p as u64;
            let mut offset = if idx < self.medium_offsets.len() {
                unsafe { *self.medium_offsets.get_unchecked(idx) as u64 }
            } else {
                let rem = low % p;
                let mut start = if rem == 0 { low } else { low + (p - rem) };
                if start % 2 == 0 { start += p; }
                start
            };

            // Advance offset into current segment
            while offset < low {
                offset += p * 2;
            }

            // Stride through the 16 KiB buffer (hits at most 1 or 2 times!)
            while offset < high {
                let bit_idx = ((offset - low) >> 1) as usize;
                let word = bit_idx >> 6;
                let bit = bit_idx & 63;
                if word < SEGMENT_WORDS {
                    unsafe {
                        *self.arena.segment_buf.get_unchecked_mut(word) |= 1u64 << bit;
                    }
                }
                offset += p * 2;
            }

            // Save state for next segment
            if idx < self.medium_offsets.len() {
                unsafe {
                    *self.medium_offsets.get_unchecked_mut(idx) = offset as u32;
                }
            } else {
                self.medium_offsets.push(offset as u32);
            }
        }
    }
}

impl Default for UltraSieveWorker {
    fn default() -> Self {
        Self::new()
    }
}

pub const TIER1_MAX_P: u32 = 1_200;
pub const TIER2_MAX_P: u32 = 32_768;

pub struct UnifiedSieveWorker {
    /// Ping-pong double buffers: 2 x 16 KiB (Fits inside Cortex-A55 32 KiB L1D)
    pub buffers: [Box<[u8; titan_sieve::wheel30::SEGMENT_BYTES]>; 2],
    pub active_idx: usize,
    pub tiny_masks: titan_sieve::wheel30_tiny::TinyPrimeMaskTable,
    pub tier1_states: Vec<titan_sieve::wheel30::Wheel30PrimeState>,
    pub tier1_primes: Vec<u32>,
    pub tier2_states: Vec<titan_sieve::wheel30_medium::MediumPrimeState>,
    pub tier3_states: Vec<titan_sieve::wheel30_sparse::SparsePrimePacked>,
}

impl UnifiedSieveWorker {
    pub fn new(low: u64, max_sieve_p: u32, primes: &[u32]) -> Self {
        let tiny_masks = titan_sieve::wheel30_tiny::TinyPrimeMaskTable::new();
        let mut tier1_states = Vec::new();
        let mut tier1_primes = Vec::new();
        let mut tier2_states = Vec::new();
        let mut tier3_states = Vec::new();

        for &p in primes {
            if p <= 5 { continue; }
            if p <= 31 { continue; } // Tier 0 handles 7..31
            if (p as u64) > (max_sieve_p as u64) { break; }

            if p <= TIER1_MAX_P {
                tier1_states.push(titan_sieve::wheel30::Wheel30PrimeState::compile(p, low));
                tier1_primes.push(p);
            } else if p <= TIER2_MAX_P {
                tier2_states.push(titan_sieve::wheel30_medium::MediumPrimeState::compile(p, low));
            } else {
                tier3_states.push(titan_sieve::wheel30_sparse::SparsePrimePacked::compile(p, low));
            }
        }

        Self {
            buffers: [
                Box::new([0xFFu8; titan_sieve::wheel30::SEGMENT_BYTES]),
                Box::new([0xFFu8; titan_sieve::wheel30::SEGMENT_BYTES]),
            ],
            active_idx: 0,
            tiny_masks,
            tier1_states,
            tier1_primes,
            tier2_states,
            tier3_states,
        }
    }

    #[inline(always)]
    pub fn sieve_next_segment(&mut self, seg_idx: u64) -> u64 {
        let active = self.active_idx;
        let next = 1 - active;
        self.active_idx = next;

        // Prefetch the next buffer's base cache line into L1D
        #[cfg(target_arch = "aarch64")]
        unsafe {
            std::arch::asm!("prfm pldl1keep, [{}]", in(reg) self.buffers[next].as_ptr(), options(nostack, preserves_flags));
        }

        let buf = &mut self.buffers[active];

        unsafe {
            // 1. Fused Tier-0 initialization (Zero memset overhead)
            self.tiny_masks.sieve_tiny_primes_fused(buf, seg_idx);

            // 2. Tier 1: Dense Primes (37 <= p <= 1,200) via rotating registers
            for i in 0..self.tier1_states.len() {
                let st = self.tier1_states.get_unchecked_mut(i);
                let p = *self.tier1_primes.get_unchecked(i);
                titan_sieve::wheel30_dense::sieve_tier1_prime_dynamic(st, p, buf);
            }

            // 3. Tier 2: Medium Primes (1,200 < p <= 32,768) via 16-bit deltas
            for st in &mut self.tier2_states {
                st.sieve_segment(buf);
            }

            // 4. Tier 3: Sparse Primes (32,768 < p <= max_p) via 8-byte packed states
            for st in &mut self.tier3_states {
                st.sieve_segment(buf);
            }

            // 5. Vector NEON Popcount across the 16 KiB buffer
            titan_sieve::wheel30_popcount::wheel30_popcount_neon(buf)
        }
    }

    #[inline(always)]
    pub fn sieve_next_segment_swap(
        &mut self,
        seg_idx: u64,
        ring: &titan_sieve::frontier_ring::FrontierRingBuffer,
    ) -> u64 {
        let active = self.active_idx;
        let next = 1 - active;
        self.active_idx = next;

        // Prefetch next buffer
        #[cfg(target_arch = "aarch64")]
        unsafe {
            std::arch::asm!("prfm pldl1keep, [{}]", in(reg) self.buffers[next].as_ptr(), options(nostack, preserves_flags));
        }

        let buf = &mut self.buffers[active];

        unsafe {
            self.tiny_masks.sieve_tiny_primes_fused(buf, seg_idx);

            for i in 0..self.tier1_states.len() {
                let st = self.tier1_states.get_unchecked_mut(i);
                let p = *self.tier1_primes.get_unchecked(i);
                titan_sieve::wheel30_dense::sieve_tier1_prime_dynamic(st, p, buf);
            }

            for st in &mut self.tier2_states {
                st.sieve_segment(buf);
            }

            for st in &mut self.tier3_states {
                st.sieve_segment(buf);
            }

            let popcount = titan_sieve::wheel30_popcount::wheel30_popcount_neon(buf);

            // Zero-copy pointer exchange with ring buffer slot
            ring.publish_segment_swap(seg_idx, popcount, buf);
            popcount
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_d_worker_basic() {
        let primes = vec![0, 2, 3, 5, 7, 11, 13, 17, 19];
        let worker = DWorker::new(primes);
        assert_eq!(worker.primes.len(), 9);
    }

    #[test]
    fn test_unified_sieve_context() {
        let x = 100_000u64;
        let y = 100u64;
        let z = 200u64;
        let primes = vec![0, 2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47];
        let mut mu = vec![0i8; y as usize + 1];
        let stream = crate::mobius_stream::MobiusStream::new(y);
        for (d, m) in stream {
            if (d as usize) < mu.len() {
                mu[d as usize] = m;
            }
        }
        let div_table = FastDivTable::build(&primes, x);
        let mut ctx = UnifiedSieveContext::new();
        let d = ctx.process_segment(0, x, y, z, &primes, &mu, &div_table);
        let _ = d;
    }
}

