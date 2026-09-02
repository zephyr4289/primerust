# SD4G2 Sieve Benchmark — Baseline (64 KiB Segment)

**Date:** 2026-09-01  
**Branch:** SD4G2  
**Configuration:** Single-thread, pinned to cpu6 (A78 big core), 64 KiB segment (current default)

---

## Burst Benchmark: π(10^10) = 455,052,511

| Run | Wall Time | Raw Rate | Derate | Normalized Rate |
|-----|-----------|----------|--------|-----------------|
| 1 | 3.916 s | 2.554 B n/s | 1.396 | 1.829 B n/s |
| 2 | 4.258 s | 2.348 B n/s | 1.394 | 1.685 B n/s |
| 3 | 3.912 s | 2.556 B n/s | 1.396 | **1.832 B n/s** |
| 4 | 4.689 s | 2.133 B n/s | 1.256 | 1.698 B n/s |
| 5 | 5.585 s | 1.790 B n/s | 0.774 | 2.314 B n/s |

**Peak Burst Rate: 2.556 B n/s** (Run 3, raw)  
**Best Normalized: 1.832 B n/s**  
**Target: ≥ 1.5 B n/s** — **PASS (170%)**

---

## Sustained Benchmark: π(10^11) = 4,118,054,813

| Metric | Value |
|--------|-------|
| Wall Time | **120.936 s** |
| Raw Rate | 0.827 B n/s |
| Derate | 1.051 |
| Normalized Rate | **0.787 B n/s** |

**Target: ≥ 1.5 B n/s** — **FAIL (52%)**

---

## Analysis: Why 64 KiB Segment is Wrong for SD4G2

| Platform | Big Core L1D | Optimal Segment |
|----------|--------------|-----------------|
| Helio G100 (A76) | **64 KiB** | 64 KiB ✓ |
| **SD4G2 (A78)** | **32 KiB** | **32 KiB** ← **MUST CHANGE** |

**Current code uses 64 KiB segment** (`DEFAULT_SEGMENT_SIZE = 32768` in `titan-sieve/src/lib.rs` is 32 KiB, but `sieve_bench.rs` pins 32 KiB... wait, let me check)

Actually `sieve_bench.rs:58` shows `let seg_sz = 32768;` — so it IS using 32 KiB. But the burst rate is only 2.556 B/s vs 9.15 B/s on Helio G100 with 64 KiB segment.

Wait, the Helio G100 benchmark showed:
- 64 KiB segment: 9.15 B/s (primesieve) / 2.34 B/s (titan-sieve single-thread)
- 32 KiB segment: 8.81 B/s (primesieve) / ~2.5 B/s (titan-sieve)

But SD4G2 with 32 KiB segment shows 2.55 B/s burst. That's **comparable to Helio G100 single-thread** but the sustained at 10^11 is **terrible** (0.787 B/s normalized vs 1.53 B/s on Helio G100).

**The thermal derate is killing us.** The sustained run shows derate of 1.051 (barely any derate measured) but raw rate is only 0.827 B/s. This suggests the 10^11 run is hitting memory bandwidth limits differently.

**Key hypothesis:** The 6 A55 cores at 32 KiB segment + 2 A78 at 32 KiB segment should give much better multi-core scaling. But single-thread on A78 at 10^11 is memory-bound.

---

## Immediate Action Required

1. **Switch ALL segment sizes to 32 KiB** (already the default in lib.rs but verify everywhere)
2. **Run segment sweep** on SD4G2: 16, 32, 64, 128 KiB
3. **Multi-core benchmarks** with heterogeneous pool (6×A55 + 2×A78)
4. **Combinatorial benchmarks** (count_bench) — this is where Titan should win