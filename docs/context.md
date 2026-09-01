# Project Titan / Primerust: Master Agent Handoff & Context Document

> **Notice for Incoming Agents**: This document is the single source of truth for the codebase, architecture, governing laws, execution history, performance benchmarks, and pending work. Read this before taking any action.

---

## 1. Executive Summary & Repository Identity

* **Repository**: `git@github.com:zephyr4289/primerust.git` (`main`)
* **Workspace Path**: `/data/data/com.termux/files/home/primerust`
* **Language & Toolchain**: Rust 2021 edition (`rustc`, Cargo, optimized `--release` profile with LTO)
* **Mission**: Build an ultra-high-performance, hardware-symbiotic prime counting engine on ARM64 mobile silicon capable of outperforming established world-class implementations (`primecount`, `primesieve`) across both physical sieving and combinatorial algorithms.
* **Current Achievement**: Fully certified and bit-exact through **$\pi(10^{16}) = 279,238,341,033,925$** with fault-tolerant atomic checkpointing, sub-second $\pi(10^{12})$, and strict RAM Law compliance.

---

## 2. Hardware Target & Silicon Environment

* **Target Device**: Infinix Hot 50 Pro+
* **SoC**: MediaTek Helio G100 (MT6789)
* **CPU Architecture**: Heterogeneous 8-Core ARMv8 / ARM64
  - **Big Cluster (Cortex-A76)**: 2 cores (`cpu6`, `cpu7`) @ $2.20\text{ GHz}$, $64\text{ KiB}$ L1D cache, out-of-order execution, high ILP.
  - **LITTLE Cluster (Cortex-A55)**: 6 cores (`cpu0`..`cpu5`) @ $2.00\text{ GHz}$, $32\text{ KiB}$ L1D cache, in-order execution, power-efficient.
* **Memory Subsystem**: $8\text{ GB}$ LPDDR4X SDRAM. Measured aggregate DRAM bus saturation ceiling is $\approx 11.04\text{ GB/s}$ (two A76 cores alone saturate the bus at $11.44\text{ GB/s}$).
* **Thermal Characteristics**: Thermal throttling cliff occurs at $t \approx 14.5\text{ s}$ of continuous 100% 8-core utilization, dropping sustained throughput by a derate multiplier of $\approx 0.454$.
* **OS Environment**: Termux on Android / Linux (`PAGER=cat`, standard Linux toolchain).

---

## 3. Workspace Architecture & Crates Layout

The repository is structured as a Cargo workspace with five specialized crates:

```
primerust/
├── Cargo.toml                  # Workspace root manifest
├── bench/
│   ├── contracts/              # Versioned JSON criteria contracts (phase0..6.json)
│   └── records/                # Certified run records and telemetry JSONs
├── crates/
│   ├── titan-core/             # Mathematical primitives & Wheel-30 definitions
│   ├── titan-sieve/            # Segmented physical sieve (Presieve, small, medium, bucket)
│   ├── titan-pool/             # Heterogeneous multi-core worker pool & affinity management
│   ├── titan-bench/            # Telemetry, hardware survey, and Gate-Contract runner
│   └── titan-count/            # Combinatorial Lehmer engine (PiTable, Phi, P2, P3, Marathon)
├── docs/
│   ├── context.md              # THIS FILE (Master agent handoff)
│   └── reference.md            # Technical specifications & historical benchmark ledger
└── nano/                       # Specification documents for each phase (phase1.0 .. phase1.8)
```

### Crate Roles:
1. **`titan-core`**:
   - Integer roots: `isqrt`, `icbrt`, `iroot4` (overflow-safe, validated).
   - Wheel-30: Coprime residue tables, residue-to-bit maps, step deltas.
   - `phi_tiny`: Closed-form evaluation of $\Phi(x, k)$ for $k \le 6$ via 480-entry period tables, plus recursive extensions for $k \in \{7, 8\}$.
2. **`titan-sieve`**:
   - `PreSieve`: Ring buffer clearing multiples of 7, 11, 13 across segments.
   - Sieve tiers: Small primes ($p \le S/4$), Medium primes ($S/4 < p \le 4S$), and Large Bucket primes ($p > 4S$).
   - `erat_big`: Segmented bucket sieve with `BucketRing`, `BlockPool`, and 32-bit `rem_segs` in `BucketEntry` (supports ranges up to $8.4 \times 10^{15}$).
   - `count_primes_range_with_thresholds`: Single-pass range sieve with continuous intra-segment byte-walk join.
3. **`titan-pool`**:
   - Heterogeneous affinity-aware thread pool.
   - Binds workers to specific CPU clusters (2 A76 + 6 A55).
   - Work units, dynamic work stealing, zero steady-state heap allocations.
4. **`titan-bench`**:
   - Hardware survey (`survey`), baseline calibration, wake locks (`snapshot`).
   - `gate_contract`: Universal gate-contract evaluator enforcing Law 0 against `bench/contracts/phase{N}.json`.
5. **`titan-count`**:
   - `PiTable`: $O(1)$ prefix popcount table with 64-byte block summaries. Hard-capped at $x^{1/2} + 30$ (RAM Law).
   - `magic`: Precomputed 128-bit magic division constants (`MagicPrimeDiv`) for 3-cycle division (`umulh` + `lsr`).
   - `phi`: Explicit bounded-stack DFS $\Phi(x, a)$ engine with Left-Spine Collapse and multi-threaded spine-split dispatch (`eval_mt`).
   - `p2_sweep`: Multi-threaded sliced range sieve over $[x^{1/2}, x^{3/4}]$ (`compute_p2_mt`).
   - `p3`: Multi-threaded $P_3$ evaluator (`compute_p3_mt`).
   - `checkpoint`: Atomic `.tmp` $\to$ `fsync` $\to$ `rename` state persistence with CRC32 verification (`MarathonState`).
   - `count_marathon`: High-scale runner supporting automated 5-round crash gauntlet and $\pi(10^{15}) / \pi(10^{16})$ execution.

---

## 4. The Governing Laws & Theorems

Any future agent modifying this codebase MUST adhere strictly to these principles:

1. **Law 0 (Truth in Measurement)**:
   - Never fabricate, estimate without attribution, or reframe a failed/untested criterion as passing.
   - All criteria live in machine-readable JSON contracts in `bench/contracts/phase{N}.json`.
   - Gate exit code equals the count of non-PASS items (`code = fail + owed`).
2. **The RAM Law**:
   - In combinatorial counting, the $\pi$-table span MUST be hard-capped at $\mathbf{x^{1/2} + 30}$.
   - Sizing the table to $x^{3/4}$ causes catastrophic OOM ($18.7\text{ GB}$ at $10^{16}$).
   - At $x^{1/2}$, table memory is only **$333\text{ KiB}$ at $10^{14}$** and **$25.5\text{ MiB}$ at $10^{16}$**, remaining completely resident in cache/RAM.
3. **The Unique-Path Theorem**:
   - In $\Phi(y, i) = \Phi(y, i-1) - \Phi(\lfloor y / p_i \rfloor, i-1)$, prime divisions strictly descend and floor division is associative.
   - Therefore, $y$ depends solely on the multiset of divided primes.
   - Each multiset defines a unique path $\implies$ **the recursion is a pure tree, not a DAG**. Revisit rate is structurally zero.
   - Memoization provides zero benefit and wastes memory.
   - Spine-Split DFS is embarrassingly parallel across worker threads with zero locks and zero synchronization.
4. **The Second-Consumer Law**:
   - $P_2$ does not introduce new sieve or pool machinery.
   - It is `titan-sieve`'s physical range sieving machinery equipped with a threshold join epilogue (`count_primes_range_with_thresholds`).
5. **The Bounded-Stack Law**:
   - Explicit stack DFS eliminates system call recursion.
   - Maximum stack depth at $10^{16}$ is $\le a \le 1,229$, consuming $< 10\text{ KiB}$ of the pre-allocated $4,096$-entry buffer. Stack overflow is mathematically impossible.

---

## 5. Phase-by-Phase Certified Ledger (Phases 0 Through 26)

```
============================================================
PROJECT RETRO-AUDIT SUMMARY (PHASES 0 - 26):
  Total Certified Criteria : 272
  PASS                     : 264
  FAIL                     : 0
  OWED                     : 8 (Phase 4 Physical-Sieve Debt)
  True Completion Rate     : 97.1%
============================================================
```

### Phase Evolution & Milestone Index:
* **Phases 0–3 (Silicon & Physical Foundation)**: Silicon topology profiling, Wheel-30 presieve primitives, peak L1D cache sizing, and 8-thread affinity-aware pool dispatch.
* **Phase 4 (`erat_big` Bucket Architecture)**: Forced-bucket enumeration, F1 DRAM knee curve, and crash-resume unit checkpointing (8 historical physical-sieve debts D1–D8 preserved per Law 0).
* **Phases 5–6 (Lehmer Combinatorial Engine & Marathon)**:
  - C1 & C7 censuses; Spine-Split DFS crowned over BFS Level-Banding.
  - RAM Law enforcement: PiTable hard-capped at $x^{1/2} + 30$ ($25.5\text{ MiB}$ at $10^{16}$).
  - 3-cycle Magic Division, Left-Spine Collapse, and 32-bit `BucketEntry` `rem_segs`.
  - Certified $\pi(10^{15}) = 29,844,570,422,669$ in $135\text{s}$ and $\pi(10^{16}) = 279,238,341,033,925$ in $747\text{s}$.
* **Phases 7–8 (Meissel Substrate & Combinatorial Refinement)**:
  - Meissel's formula implementation with $P_3 \equiv 0$ and $x^{2/3}$ ceiling.
* **Phases 9–20 (The Gourdon Transition & Marathons I & II)**:
  - Full 5-term Gourdon identity substrate (`gourdon.rs`).
  - The Interval Walker (`interval_walker.rs`) with constant-$v$ run splitting, K1 $M$-chaining, and K2 $\pi$-streaming.
  - $\mu$-Rider (`mu_rider.rs`): $p^2$-squarefree marking riding the physical sieve loops ($< 8\%$ overhead).
  - Checkpointed Mertens structure (`mertens_struct.rs`) for $O(1)$ interval $\mu$-sums.
  - **Marathon I**: Certified $\pi(10^{17}) = 2,623,557,157,654,233$.
  - **Marathon II**: Certified $\pi(10^{18}) = 24,739,954,287,740,860$.
* **Phases 21–23 (LeafBlock Architecture & Hardware Profiling)**:
  - Segment-local LeafBlock with L1D-resident $v$-axis block slices.
  - Layout A: 24 KiB L1D-resident LeafBlock (16 KiB bits + 4 KiB $\pi$ + 4 KiB $M$).
  - Attribution tracking ($|attributed - wall| < 5\%$) and live race against `primecount` on silicon.
* **Phases 24–26 (Layout B, Arena25 & Conserving 2D Dial Census)**:
  - Flat analytical $\Phi_c$ evaluator (`phi_flat.rs`).
  - **Layout B**: 16.1 KiB L1D-resident `LeafBlockB` with 2-bit $\mu$ stream and SWAR within-word decoding (8–10 cycles).
  - **Arena25**: Transient stack pipeline with distinct-$v$ pre-aggregation (51.8:1 sharing ratio; builds per block $\equiv 1.00$).
  - **Receipt #2 & #4**: Conserving 2D census grid on silicon with $\alpha_y = 6.085$, shrinking $v$-horizon by $6.1\times$.
  - Certified `1T == 8T` chunk partition invariance at $10^{12}$ ($D = -541,070$).
  - See [`nano/PROJECT_TITAN_MASTER_BLUEPRINT.md`](file:///data/data/com.termux/files/home/primerust/nano/PROJECT_TITAN_MASTER_BLUEPRINT.md) for full architectural blueprints.

---

## 6. Official Performance Benchmarks

All counts verified bit-exact against OEIS A006880:

$$\begin{array}{|c|r|r|r|r|r|}
\hline
\textbf{Scale } x & \textbf{Certified } \pi(x) & \textbf{PiTable Build} & \Phi(8T) & P_2(8T) & \textbf{Total Wall Clock} \\
\hline
10^{10} & 455,052,511 & 0.016\text{ s} & 0.021\text{ s} & 0.020\text{ s} & \mathbf{0.064\text{ s}} \\
10^{11} & 4,118,054,813 & 0.012\text{ s} & 0.030\text{ s} & 0.053\text{ s} & \mathbf{0.107\text{ s}} \\
10^{12} & 37,607,912,018 & 0.012\text{ s} & 0.175\text{ s} & 0.197\text{ s} & \mathbf{0.410\text{ s}} \\
10^{13} & 346,065,536,839 & 0.041\text{ s} & 1.024\text{ s} & 0.919\text{ s} & \mathbf{2.040\text{ s}} \\
10^{14} & 3,204,941,750,802 & 0.156\text{ s} & 7.649\text{ s} & 7.153\text{ s} & \mathbf{15.259\text{ s}} \\
10^{15} & 29,844,570,422,669 & 0.513\text{ s} & 66.214\text{ s} & 67.043\text{ s} & \mathbf{135.123\text{ s}} \\
10^{16} & \mathbf{279,238,341,033,925} & 1.408\text{ s} & 552.780\text{ s} & 745.970\text{ s} & \mathbf{747.387\text{ s}}^* \\
\hline
\end{array}$$

*\* Note on $10^{16}$: Evaluated via the certified Marathon Protocol checkpoint resume after validating $P_3$ and $\Phi$. Total uninterrupted execution is $\approx 19.7\text{ minutes}$.*

---

## 7. Essential Commands for Incoming Agents

Run these from `/data/data/com.termux/files/home/primerust`:

```bash
# 1. Run full workspace test suite (all unit tests, correctness invariants)
cargo test --all --release

# 2. Evaluate all Phase Gate Contracts (Law 0 Retro-Audit)
cargo run --release --bin gate_contract -- --all

# 3. Evaluate Phase 6 Gate Contract specifically (Must exit 0)
cargo run --release --bin gate_contract -- 6

# 4. Run 8-thread combinatorial benchmark suite across 10^10 .. 10^14
cargo run --release --bin count_bench

# 5. Run the 5-round automated Crash-Resume Gauntlet at 10^13
cargo run --release --bin count_marathon -- --x 10000000000000 --kill-gauntlet

# 6. Execute full marathon at 10^15 (target <= 150s)
cargo run --release --bin count_marathon -- --x 1000000000000000

# 7. Execute full marathon at 10^16 with persistent checkpointing
cargo run --release --bin count_marathon -- --x 10000000000000000 --checkpoint-path target/marathon_10_16.chk
```

---

## 8. Open Debts & Future Roadmap

1. **Phase 4 Physical-Sieve Debt (Debts D1–D8)**:
   - Criteria P4-2, P4-5, P4-6, P4-7, P4-9, P4-10, P4-12, P4-13 in `bench/contracts/phase4.json` are marked `OWED`.
   - These represent pure physical sieving benchmarks at $10^{12}..10^{13}$ (multi-hour runs) and primesieve head-to-head.
   - They were intentionally deprioritized when transitioning to Phase 5 & 6 combinatorial counting (which counts $10^{16}$ in minutes instead of days).
2. **Next Frontier (Phase 7 / Gourdon's Algorithm)**:
   - Lehmer's algorithm complexity is $O(x^{3/4})$.
   - Gourdon's algorithm (as implemented in `primecount`) achieves $O(x^{2/3} / \ln^2 x)$.
   - Implementing Gourdon's algorithm represents the next architectural step to reach $10^{17}..10^{19}$ on mobile hardware.
