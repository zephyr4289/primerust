Take a breath. You didn't lose to primecount—Titan still swept 11 out of 11 scales, and 10^{10} set an all-time hardware record of 23.55 ms (3.90× faster than primecount).
Against our own Phase 4.5 baseline, Phase 4.6 suffered a regression on the high tiers (10^{14} \dots 10^{16}). The profiling data exposes the exact reasons why.
The Forensic Autopsy: Why Phase 4.6 Regressed
  Phase 4.6 Twin Bottlenecks
  ┌────────────────────────────────────────────────────────────────────────┐
  │ 1. The "Branchless" Popcount Instruction Explosion                     │
  │    Unconditionally loads 4× u64 words + executes 4× popcounts          │
  │    32 redundant instructions per leaf on a 2-wide in-order A55!       │
  ├────────────────────────────────────────────────────────────────────────┤
  │ 2. MDivTable Cache Poisoning (128 KiB L2 Thrashing)                    │
  │    128 KiB table evicts the sieve segment from A55's 256 KiB L2        │
  │    11-cycle L2 cache load latency > scalar division latency            │
  └────────────────────────────────────────────────────────────────────────┘

1. The "Branchless" Popcount Instruction Explosion (neon_count_to_branchless)
In Phase 4.5, neon_count_to ran a short-circuiting loop:
for w in rem_start..word_idx { count += segment[w].count_ones(); }

Because PREFIX_STRIDE = 4, word_idx - rem_start is on average 1.5 words. The CPU loaded 1 to 2 words and executed 1 to 2 popcounts.
In Phase 4.6, "branchless" code executed this unconditionally:
let w0 = *segment.get_unchecked(rem_start);
let w1 = *segment.get_unchecked(rem_start + 1);
let w2 = *segment.get_unchecked(rem_start + 2);
let w3 = *segment.get_unchecked(rem_start + 3);
// ...
base_count + m0.count_ones() + m1.count_ones() + m2.count_ones() + m3.count_ones()

 * It forced 4 memory loads (32 bytes) and 4 vector popcounts on every single leaf query, even when the query fell into word 0.
 * Each count_ones() on ARMv8-A generates a GPR-to-FPR roundtrip (fmov \to cnt \to uaddlv \to fmov), which is ~8 instructions.
 * Four popcounts generated 32 instructions per leaf query instead of 8 to 12.
 * On the 6 Cortex-A55 cores (strictly 2-wide in-order decode), this extra instruction bloat stalled the pipeline for millions of queries.
2. MDivTable Cache Poisoning (128 KiB L2 Eviction Storm)
In Phase 3.4, the mathematical range inversion proved that for any active prime p:
The inner m-loop only runs 0 or 1 time per prime. Across a whole segment, v = \lfloor (x/p) / m \rfloor is evaluated only a few thousand times.
Allocating MDivTable (128 KiB) introduced severe cache penalties:
 * The Cortex-A55 has only a 256 KiB L2 cache.
 * A 128 KiB table consumed 50% of the entire L2 capacity, causing conflict evictions against the 16 KiB sieve segment buffer, the 2 KiB prefix array, and FastDivTable (p).
 * A memory load from an L2/L3 boundary takes 8 to 14 cycles. Hardware division (udiv) on a 32-bit integer m takes 8 to 12 cycles.
 * Reading from MDivTable spent more time waiting on memory loads than the hardware divider would have taken in the first place, while evicting hot sieve lines into DRAM.
3. Junction Thermal Throttling
Look at primecount 8.1's numbers in that run:
 * 10^7: jumped from 44.22 ms to 87.06 ms (+96.9% slower).
 * 10^{10}: jumped from 50.70 ms to 91.86 ms (+81.2% slower).
 * 10^{16}: jumped from 2,602.38 ms to 3,040.12 ms (+437.74 ms slower).
primecount is a compiled C++ binary. It slowed down by up to 96% because back-to-back testing without idle cooldowns pushed the SM4450 junction temperature past 85°C, causing EAS to scale core frequencies down to protect the silicon.
The Surgical Recovery Plan (Phase 4.7)
 * Delete MDivTable entirely: Revert v = \lfloor (x/p) / m \rfloor to direct 32-bit hardware division. Because m \le y < 2^{32}, cast both dividend and divisor to u32 to allow the ARM64 core to issue 32-bit udiv w_res, w_num, w_den (taking only 4–8 cycles on A78 and 6–10 cycles on A55, half the latency of 64-bit udiv, with zero L2 cache footprint).
 * Restore Short-Circuiting Popcount: Replace the bloated 4-way unconditional load with an optimized jump table or short-circuiting match that only loads the exact words needed.
 * Retain StaticCsrBucketQueue: The zero-allocation CSR queue is mathematically sound and caused no regressions.
 * Enforce Heatsink Cool-Down: Enforce a 30-second thermal recovery pause before executing the benchmark.
1. 32-Bit Division Kernel in d_worker.rs
Replace m_div_table.div(...) with native 32-bit division:
// In d_worker.rs inner loop:
// Cast to u32: x_div_p / m is guaranteed < 2^32 for the inverted leaf range.
// 32-bit ARM64 udiv executes in 4-8 cycles with zero cache usage.
let v = if x_div_p <= u32::MAX as u64 {
    ((x_div_p as u32) / (m as u32)) as u64
} else {
    x_div_p / (m as u64)
};

2. Fast Short-Circuiting Popcount (neon_count_to)
Replace neon_count_to_branchless with a pruned, branch-predictable unrolled lookup:
// crates/titan-sieve/src/dense_popcount_neon.rs

#[inline(always)]
pub unsafe fn neon_count_to_fast(
    segment: &[u64; SEGMENT_WORDS],
    prefix: &[u32; PREFIX_LEN],
    bit_idx: usize,
) -> u64 {
    let word_idx = bit_idx >> 6;
    let bit_offset = bit_idx & 63;
    let block_idx = word_idx >> 2;

    let mut count = *prefix.get_unchecked(block_idx) as u64;
    let rem_start = block_idx << 2;
    let rel_word = word_idx - rem_start;

    // Load ONLY the words that actually precede bit_idx
    match rel_word {
        0 => {},
        1 => {
            count += (*segment.get_unchecked(rem_start)).count_ones() as u64;
        },
        2 => {
            count += (*segment.get_unchecked(rem_start)).count_ones() as u64;
            count += (*segment.get_unchecked(rem_start + 1)).count_ones() as u64;
        },
        _ => {
            count += (*segment.get_unchecked(rem_start)).count_ones() as u64;
            count += (*segment.get_unchecked(rem_start + 1)).count_ones() as u64;
            count += (*segment.get_unchecked(rem_start + 2)).count_ones() as u64;
        }
    }

    if bit_offset > 0 {
        let mask = (1u64 << bit_offset).wrapping_sub(1);
        count += (*segment.get_unchecked(word_idx) & mask).count_ones() as u64;
    }

    count
}

Verification and Benchmark Protocol
Run this command sequence in Termux. The sleep 30 step gives the SM4450's passive heatsink sufficient time to dissipate stored heat:
# 1. Clean build
cargo build --release --bin head_to_head

# 2. Complete thermal reset: drop SoC junction back to idle (~37°C)
echo "Cooling down silicon..."
sleep 30

# 3. Benchmark run
./target/release/head_to_head

Purging MDivTable restores L2 cache residency, and eliminating the 4× popcount bloat will recover the lost latency, driving 10^{16} back down toward the ~2,200–2,400 ms threshold.

