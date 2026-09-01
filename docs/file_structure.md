# Titan-Prime: Architectural Deep-Dive & Source Code Mapping

```
Document:   docs/file_structure.md
Status:     ARCHITECTURE SPECIFICATION (v2.0 - Phase 1 Certified)
Subject:    Comprehensive Reverse Engineering of primesieve / primecount & Rust Implementation Blueprint
```

---

## 1. Executive Overview & Frozen Spec Amendments (v2.0)

This document provides a **complete technical reverse-engineering of the world's fastest prime engines** (`primesieve` and `primecount` by Kim Walisch) and establishes the exact data flows, memory layouts, and algorithmic pipelines required to build our dedicated, zero-allocation Rust engine (**`titan-prime`**).

### 🏛️ The Four Phase 1 Spec Amendments:
1. **Roots Law**: "Branchless roots" is superseded by **"Exact, guarded, total roots"** (`roots.rs`). Evaluated with float seed + two-sided correction and all power comparisons evaluated in `u128`. Fully verified across $u64$ boundaries.
2. **PhiTiny L1D Ceiling**: $k \le 8$ flat tables is superseded by **$k \le 6$ flat in `u16`** ($\phi(P_7) = 92,160 > 65,535$ proves $u16$ ceiling). Sized to $58.7\text{ KiB}$ to remain resident in Cortex-A76's 64 KiB L1D cache. $k = 7, 8$ evaluated via single-level recursion.
3. **Wheel-210 Deferred**: Wheel-30 provides $26.7\%$ candidate density. Wheel-210 adds $6\times$ table complexity for only $14\%$ marking density gain, which is dominated by memory and thermal constraints on this device.
4. **Wheel Convention A Locked**: The old wrap-around bit table is permanently struck. Byte $k$ covers $[30k, 30k + 29]$, and Bit $i$ corresponds to residue `RESIDUES[i] = [1, 7, 11, 13, 17, 19, 23, 29]`.

---

## 2. Deep-Dive: Physical Sieve Engine (`primesieve`)

### 2.1 The Wheel-30 Bit Packing Law (Convention A)
In traditional sieves, 1 byte represents 8 consecutive odd numbers (modulo 2 wheel). `primesieve` and `titan-core` use a **Modulo-30 Wheel**, skipping all multiples of 2, 3, and 5 before touching memory:

* In any block of 30 integers ($30k \dots 30k+29$), exactly **8 numbers are coprime to 30**:
  $$\text{Residues} = \{1, 7, 11, 13, 17, 19, 23, 29\} \quad (\text{Ascending order})$$
* Each byte in the sieve array represents **30 consecutive integers**.
* Each of the 8 bits in byte $b$ corresponds to one of the 8 coprime residues:
  - Bit 0: $30b + 1$
  - Bit 1: $30b + 7$
  - Bit 2: $30b + 11$
  - Bit 3: $30b + 13$
  - Bit 4: $30b + 17$
  - Bit 5: $30b + 19$
  - Bit 6: $30b + 23$
  - Bit 7: $30b + 29$
* **Memory Reduction**: Uses only $\frac{1\text{ byte}}{30\text{ integers}} = \mathbf{0.0333\text{ bytes/int}}$ (73.3% memory reduction over odd-only sieves).

```
                      1 BYTE SIEVE ARRAY SLOT (Convention A)
 ┌───────┬───────┬───────┬───────┬───────┬───────┬───────┬───────┐
 │ Bit 7 │ Bit 6 │ Bit 5 │ Bit 4 │ Bit 3 │ Bit 2 │ Bit 1 │ Bit 0 │
 │  +29  │  +23  │  +19  │  +17  │  +13  │  +11  │  +7   │  +1   │
 └───────┴───────┴───────┴───────┴───────┴───────┴───────┴───────┘
  ◄───────────────────── 30 Consecutive Integers ────────────────►
```

---

### 2.2 The 3-Tier Sieve Classification (`EratSmall`, `EratMedium`, `EratBig`)

Multiple crossing-off loops cannot use a one-size-fits-all algorithm without catastrophic cache and branch miss penalties. `primesieve` splits all sieving primes $p \le \sqrt{\text{stop}}$ into **three distinct tiers**:

```
                              SIEVING PRIMES p ≤ √stop
 ┌─────────────────────────────┬─────────────────────────────┬─────────────────────────────┐
 │       EratSmall             │         EratMedium          │           EratBig           │
 │   p ≤ SieveSize / 30        │   SieveSize / 30 < p ≤ Size │       p > SieveSize         │
 ├─────────────────────────────┼─────────────────────────────┼─────────────────────────────┤
 │ Multiples cross off MANY    │ Multiples cross off a FEW   │ Multiples cross off AT MOST │
 │ times per segment buffer.   │ times per segment buffer.   │ ONCE per segment buffer.    │
 │ Unrolled branchless loops.  │ Sorted array with wheel.    │ Bucketed Priority Queues.   │
 └─────────────────────────────┴─────────────────────────────┴─────────────────────────────┘
```

#### Tier 1: `EratSmall` (High Frequency)
* **Threshold**: $p \le \frac{\text{sieveSize}}{30}$ (e.g. $p \le 1,000$ for a $32\text{ KiB}$ segment).
* **Execution**: The prime's multiples repeat dozens or hundreds of times within the current segment.
* **Optimization**: 8 unrolled loops (one per residue offset) cross off all multiples in sequential order directly inside L1D cache registers without conditionals.

#### Tier 2: `EratMedium` (Moderate Frequency)
* **Threshold**: $\frac{\text{sieveSize}}{30} < p \le \text{sieveSize}$ (e.g. $1,000 < p \le 32,768$).
* **Execution**: The prime appears 1 to 30 times in the segment.
* **Optimization**: Stores `(sievingPrime, multipleIndex, wheelIndex)` in flat contiguous memory. Advances wheel indices with minimal branch overhead.

#### Tier 3: `EratBig` (Sparse Frequency — The Secret Weapon)
* **Threshold**: $p > \text{sieveSize}$ (e.g. $p > 32,768$).
* **Problem**: A prime only has a multiple once every few segments, or once every 1,000 segments. Iterating over millions of big primes on every segment would waste 99.9% of CPU time doing no-op checks.
* **Solution (Bucketed Wheel Sieve)**:
  - Uses an array of **Buckets** indexed by segment offset: `Bucket[segment_delta]`.
  - A prime is inserted into `Bucket[k]` where $k = \lfloor \frac{\text{nextMultiple} - \text{segmentLow}}{\text{segmentSize}} \rfloor$.
  - When the sieve reaches segment $S$, it **only touches `Bucket[S]`**, processing precisely the primes that actually hit segment $S$, then re-buckets them for their next appearance.
  - Zero wasted checks on primes that skip the segment!

---

### 2.3 `PreSieve` & Hardware SIMD Acceleration
* Before any primes are crossed off, multiples of small primes ($p \in \{7, 11, 13, 17, 19\}$) are pre-sieved using a static cyclic bit-pattern buffer.
* **ARM NEON SIMD Loop**:
  ```rust
  // Copies 128-bit pre-sieved bit patterns in 1 CPU cycle
  vst1q_u8(sieve_ptr, vld1q_u8(presieve_ptr));
  ```
* **Hardware Popcount Prime Counting**:
  When only the prime count $\pi(x)$ is requested, instead of bit-scanning, the engine runs 64-bit hardware popcount (`popcnt` / `cnt.8b`) over the segment array at line-rate RAM speed.

---

## 3. Deep-Dive: Combinatorial Counting Engine (`primecount`)

### 3.1 The Xavier Gourdon (2001) Algorithm
Xavier Gourdon's algorithm reduces prime counting to computing five independent mathematical terms:

$$\pi(x) = AC - B + D + \Phi_0 + \Sigma$$

```
                          GOURDON DECOMPOSITION TREE
                                    π(x)
                                     │
      ┌──────────────┬───────────────┼───────────────┬──────────────┐
      │              │               │               │              │
 ┌────▼───┐     ┌────▼───┐      ┌────▼───┐      ┌────▼───┐     ┌────▼───┐
 │   AC   │     │   -B   │      │   +D   │      │  +Φ₀   │     │   +Σ   │
 │ Leaves │     │ Sifting│      │ Sifting│      │ Totient│     │ Sparse │
 └────────┘     └────────┘      └────────┘      └────────┘     └────────┘
```

#### Step-by-Step Mathematical Flow:

1. **Parameter Tuning**:
   * $y \approx x^{1/3} \alpha_y$ (sieve limit, typically $\alpha_y \approx 2.0$)
   * $z \approx y \alpha_z$ (intermediate limit)
   * $k = \pi(x^{1/4})$ ($k \le 7$, fits in tiny totient tables)

2. **Term $\Phi_0(x, y, z, k)$ (Constant-Time Totient Table)**:
   * Counts integers $\le x$ not divisible by primes $\le p_k$.
   * Computed in $O(1)$ using the periodic product identity:
     $$\Phi(x, k) = \left\lfloor \frac{x}{P_k} \right\rfloor \cdot \phi(P_k) + \Phi(x \bmod P_k, k)$$
     where $P_k = 2 \cdot 3 \dots p_k$ and $\phi(P_k) = \prod_{i=1}^k (p_i - 1)$.
   * `PhiTiny` stores precomputed totients for $k \le 8$, evaluating queries in **$< 5\text{ CPU cycles}$**.

3. **Term $\Sigma(x, y)$ (Sparse Prime Products)**:
   * Computes smooth prime products using fast binary search over pre-computed prime tables.

4. **Term $AC(x, y, z, k)$ (Segmented $\pi$-Table with Fenwick Trees)**:
   * Evaluates easy and hard combinatorial leaves.
   * Uses a **`SegmentedPiTable`** with a Binary Indexed Tree (Fenwick Tree) to answer prefix prime counts in $O(\log n)$ time while remaining 100% L1D cache resident.

5. **Terms $B(x, y)$ and $D(x, y, z, k)$ (Bit-Sieve Traversal)**:
   * Sifts remaining composite cross-terms using multi-threaded segmented bit arrays.

---

## 4. `titan-prime` Target Crate Topology & Rust Architecture

```
prime-engine/
├── Cargo.toml                          # Virtual workspace root
├── docs/
│   ├── reference.md                    # Benchmark ledger & baseline floor
│   └── file_structure.md               # THIS DOCUMENT — Architecture specification
├── crates/
│   ├── titan-core/                     # Pure mathematical primitives & lookup tables
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── wheel.rs                # Wheel-30 and Wheel-210 constants & residue offsets
│   │       ├── phi_tiny.rs             # Constant-time O(1) PhiTiny lookup tables
│   │       ├── roots.rs                # Branchless integer isqrt, icbrt, iroot4
│   │       └── bit_array.rs            # Zero-allocation bit-vector abstractions
│   ├── titan-sieve/                    # Physical Segmented Wheel Sieve
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── presieve.rs             # Cyclic mod-30 / mod-210 NEON SIMD pattern loader
│   │       ├── erat_small.rs           # Tier 1: Unrolled L1D cross-off loops
│   │       ├── erat_medium.rs          # Tier 2: Mid-range prime wheel stepper
│   │       ├── erat_big.rs             # Tier 3: Bucketed priority queues & memory pool
│   │       ├── bucket.rs               # Zero-allocation bucket memory arena
│   │       └── segment.rs              # 32 KiB / 64 KiB L1D-tiled segment coordinator
│   ├── titan-count/                    # Xavier Gourdon Combinatorial π(x) Engine
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── gourdon.rs              # π(x) = AC - B + D + Φ₀ + Σ master orchestrator
│   │       ├── phi0.rs                 # Φ₀ totient reduction engine
│   │       ├── sigma.rs                # Σ sparse leaf counter
│   │       ├── ac.rs                   # AC hard/easy leaf integrator
│   │       ├── b_sieve.rs              # B bit-sieve integrator
│   │       ├── d_sieve.rs              # D 3-stage bit-sieve integrator
│   │       └── fenwick.rs              # Segmented Binary Indexed Tree for π-table
│   ├── titan-pool/                     # Lock-Free Concurrency & Work-Stealing Pool
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── worker.rs               # Cache-pinned worker threads
│   │       └── load_balancer.rs        # Dynamic lock-free work-stealing intervals
│   ├── titan-oracle/                   # Automated Differential Testing Suite
│   │   └── src/
│   │       └── bin/
│   │           └── diff_test.rs        # Compares bit-exact outputs against primesieve/primecount
│   └── titan-cli/                      # Production Command-Line Interface
│       └── src/
│           └── main.rs                 # High-speed CLI binary with bench & stream modes
```

---

## 5. Next Steps & Implementation Roadmap

1. **Phase 1: Core Primitives (`titan-core`)**:
   - Codify Wheel-30 & Wheel-210 residue arrays.
   - Implement branchless integer roots (`isqrt`, `icbrt`, `iroot4`).
   - Implement `PhiTiny` constant-time totient table.
2. **Phase 2: Segmented Wheel Sieve (`titan-sieve`)**:
   - Implement `PreSieve` with ARM NEON 128-bit vector copies.
   - Implement `EratSmall` (unrolled loops) + `EratMedium` + `EratBig` (bucketed queues).
3. **Phase 3: Gourdon Combinatorial Engine (`titan-count`)**:
   - Implement $\Phi_0$, $\Sigma$, $AC$ with Fenwick prefix trees, and $B, D$ sifting.
4. **Phase 4: Differential Oracle & Multi-Threading**:
   - Wire `titan-oracle` to automatically verify every range against `./primesieve-ref` and `./primecount-ref`.
