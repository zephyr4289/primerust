# Titan-Prime: Ground-Truth Reference & Oracle Benchmark Ledger

```
Document:   docs/reference.md
Status:     FROZEN & CERTIFIED (Phase 0 Audit Complete)
Authority:  Measured on MediaTek Helio G100 Ultra (Termux ARM64)
Reference:  Kim Walisch's primesieve v12.6 & primecount v7.14
Environment: MediaTek Helio G100 (2x Cortex-A76 @ 2.2GHz + 6x Cortex-A55 @ 2.0GHz, 8GB RAM)
```

---

## 1. Silicon Architecture & Partitioning Weights

From `target/release/survey` (persisted in `bench/baselines.json`):

* **Cluster Topology**:
  - `cpu0..cpu5`: **6× ARM Cortex-A55** @ 2.0 GHz (Little Cluster, 32 KiB L1D)
  - `cpu6..cpu7`: **2× ARM Cortex-A76** @ 2.2 GHz (Big Cluster, 64 KiB L1D)
* **Measured Workload Capacity**:
  - **Solo Sieve Throughput**: Little cores = $340.2\text{M n/s}$ each ($2,041\text{M}$ total); Big cores = $738.5\text{M n/s}$ each ($1,477\text{M}$ total).
  - **Workload Split**: **$58.0\%$** of total sieve capacity lives on the 6 little cores; **$42.0\%$** lives on the 2 big cores.
  - **Memory Extraction Asymmetry**: $1,496$ vs $582\text{ ep/s}$ ($2.57\times$ in favor of OoO A76).
  - **All-Core Contention Efficiency**: **$80.1\%$** retained under all-core load.
  - **Per-Cluster Contention Split**: Little cores retain **$91.2\% - 100.4\%$**; Big cores drop to **$36.6\% - 76.3\%$**.
* **Thermal Envelope & Cliff**:
  - Peak Throttle (Min Derate): **$0.166$** ($16.6\%$ of clock)
  - Thermal Cliff Timestamp: **$t \approx 14.5\text{ s}$** (Burst deadline before MediaTek DVFS thermal clamp)
  - Sustained Multiplier (End Derate): **$0.454$** ($45.4\%$ of clock)

---

## 2. Best-Config Sweep & The Summit Targets

### 🎯 Physical Sieving Target (`primesieve` Sieve-Size Sweep at $10^{10}$)

| Sieve Segment Size | Execution Time ($10^{10}$) | Physical Throughput | Microarchitectural Match |
|---|:---:|:---:|---|
| 16 KiB | $1.449\text{ s}$ | $6.90\text{B n/s}$ | L1D Under-utilized |
| 32 KiB | $1.135\text{ s}$ | $8.81\text{B n/s}$ | Fits A55 L1D (32 KiB) |
| **64 KiB (SUMMIT)** | **$1.093\text{ s}$** | **$9.15\text{ Billion n/s}$** | **100% L1D Resident on A76 (64 KiB)** |
| 128 KiB | $1.274\text{ s}$ | $7.85\text{B n/s}$ | L1D Spill to L2 |
| 256 KiB | $1.330\text{ s}$ | $7.52\text{B n/s}$ | L2 Cache Thrashing |

* **Phase 2 Single-Thread Target**: $\ge 1.5\text{B n/s}$ ($70\%$ of primesieve's $2.225\text{B/s}$).
* **Phase 3 Heterogeneous Target**: Beat **$9.15\text{ Billion numbers/s}$** burst and **$4.235\text{ Billion numbers/s}$** sustained.

---

## 3. Algorithm Ladder Ground Truth (`primecount`)

Measured pure compute times (excluding Android dynamic linking and process fork overhead):

### At $10^{12}$ ($1\text{ Trillion}$):
* **Lehmer**: $0.186\text{ s}$
* **LMO**: $0.098\text{ s}$
* **Deleglise-Rivat**: $0.099\text{ s}$
* **Gourdon**: $0.102\text{ s}$

### At $10^{13}$ ($10\text{ Trillion}$):
* **Lehmer**: $0.786\text{ s}$
* **LMO**: $0.197\text{ s}$ ($4.0\times$ faster than Lehmer)
* **Deleglise-Rivat**: $0.161\text{ s}$ ($4.9\times$ faster than Lehmer)
* **Gourdon (SUMMIT)**: **$0.127\text{ s}$** (**$6.2\times$ faster than Lehmer**, $1.27\times$ faster than DR)

### Extended Gourdon Scaling Floor:
* $\pi(10^{10}) = 455,052,511$ in **$0.056\text{ s}$** ($178.6\text{B n/s}$)
* $\pi(10^{12}) = 37,607,912,018$ in **$0.102\text{ s}$** ($9.80\text{T n/s}$)
* $\pi(10^{14}) = 3,204,941,750,802$ in **$0.288\text{ s}$** ($347.2\text{T n/s}$)
* $\pi(10^{15}) = 29,844,570,422,669$ in **$0.689\text{ s}$** ($1.451\text{Q n/s}$)
* $\pi(10^{16}) = 279,238,341,033,925$ in **$2.756\text{ s}$** ($3.628\text{Q n/s}$)

---

## 4. Standalone Truth Triangle & Mutant Kills (6 / 6)

From `target/release/oracle --full`:

* **Truth Triangle Interlock**: Bit-exact across `Trial ⟷ OEIS A006880 ⟷ primecount` from $10^1$ to $10^7$.
* **Deep Literature Binchecks**:
  - `[bincheck] x = 10^12 : π(x) = 37,607,912,018 [Lit=PASS, primecount=PASS]`
  - `[bincheck] x = 10^13 : π(x) = 346,065,536,839 [Lit=PASS, primecount=PASS]`
  - `[bincheck] x = 10^14 : π(x) = 3,204,941,750,802 [Lit=PASS, primecount=PASS]`
  - `[bincheck] x = 10^15 : π(x) = 29,844,570,422,669 [Lit=PASS, primecount=PASS]`
  - `[bincheck] x = 10^16 : π(x) = 279,238,341,033,925 [Lit=PASS, primecount=PASS]`
* **Mutant Corpus Kills (6 / 6)**:
  - **M1** (Sqrt Boundary `<`): Caught at $x = 25$ by T1-small.
  - **M2** (Missing 2): Caught at $x = 2$ by T1-small.
  - **M3** (Square Numbers): Caught at $x = 25$ by T1-small.
  - **M4** (Scale Deviation): Caught at $x = 10,000,000$ by Tier 3 literature constants.
  - **M5** (Domain Off-By-One): Caught at $x = 2$ by T1-small.
  - **M6** (Wheel Residue Drop $11 \bmod 30$): Caught at $x = 11$ by T1-small (expected 5, got 4).
* **Oracle Gate**: **PASS (EXIT CODE 0)**.

---

## 5. Phase 2 & Phase 3 Physical Engine Certified Ledgers

### 🏆 Phase 2 Single-Core Physical Sieve (`titan-sieve`)
* **Single Big Core Burst ($10^{10}$)**: **$4.26\text{ s}$** ($2.346\text{ Billion numbers/s}$) — **Beats `primesieve` single-thread ($4.49\text{s}$, $2.225\text{ B/s}$)**.
* **Single Core Sustained ($10^{11}$)**: $64.004\text{ s}$ ($1.562\text{ Billion numbers/s}$ raw).
* **L1D Cache Summit**:
  - $16\text{ KiB}$: $2,374.9\text{M n/s}$
  - $32\text{ KiB}$: $2,588.4\text{M n/s}$
  - **$64\text{ KiB}$**: **$2,827.1\text{M n/s}$ (Summit on A76 64 KiB L1D)**
  - $128\text{ KiB}$: $2,026.4\text{M n/s}$ ($-28\%$ due to L2 spills)

### ⚡ Phase 3 Heterogeneous Multi-Core Pool (`titan-pool`)
* **Pre-Flight E1 Weight Vector**:
  - Cortex-A55 Little Cores (`cpu0..cpu5`, 32 KiB segment): $0.77\text{ B/s}$ each ($4.54\text{ B/s}$ aggregate, $49.3\%$ of capacity).
  - Cortex-A76 Big Cores (`cpu6..cpu7`, 64 KiB segment): $2.33\text{ B/s}$ each ($4.66\text{ B/s}$ aggregate, $50.7\%$ of capacity).
  - Normalized Weights: `[0.084, 0.084, 0.084, 0.084, 0.084, 0.084, 0.253, 0.253]`.
* **8-Core Burst Throughput ($10^{10}$)**: **$1.620\text{ s}$** (**$6.172\text{ Billion numbers/s}$**).
* **8-Core Sustained Execution ($10^{11}$)**: **$41.186\text{ s}$** (**$2.428\text{ Billion numbers/s}$**).
  - Load balancing across heterogeneous clusters: **$\le 3\%$ deviation** across all 8 workers.
* **Correctness & Seam Invariance**:
  - Partition Invariance across $k \in \{1, 2, 4, 8\}$: Bit-identical $\pi(10^8) = 5,761,455$.
  - Mutants Killed: `M-mask`, `M-restore`, `M-seam` (boundary overlap/gap).
  - Steady-State Allocations: **EXACTLY 0 heap allocations**.

---

## 6. Phase 4 Deep Domain Bucket Engine & Crash Gauntlet Ledger

### 🛰️ Pre-Flight F1: DRAM Bandwidth Knee Curve
* **Measured Bandwidth vs Worker Count**:
  - $k = 1\text{ worker}$: **$7.44\text{ GB/s}$**
  - $k = 2\text{ workers}$ (2x A76 Big): **$11.44\text{ GB/s}$** (Bus saturated by dual-core out-of-order execution)
  - $k = 4\text{ workers}$: **$10.28\text{ GB/s}$**
  - $k = 6\text{ workers}$: **$10.31\text{ GB/s}$**
  - $k = 8\text{ workers}$ (Full SoC): **$11.04\text{ GB/s}$** plateau
* **DRAM Ceiling Law**: Helio G100 LPDDR4X bandwidth is physical bus-capped at $\sim 11.0\text{ GB/s}$. At full 8-worker occupancy, individual worker bandwidth is budgeted at $\sim 1.38\text{ GB/s}$.

### ⚙️ G0 Forced-Bucket Certification Suite (`erat_big`)
* **Shrunken Geometry ($S = 256\text{ B}$, $W = 4$ & $W = 2$)**:
  - Activated 274 bucket primes at small scale ($N = 10^7$).
  - Full element-wise enumeration: Bit-exact **$\pi(10^7) = 664,579$** under both $W = 4$ and $W = 2$ window edge stress.
  - Range Invariance: $[0, 5\times 10^6] + [5\times 10^6+1, 10^7] = 348,513 + 316,066 = \mathbf{664,579}$.

### 🛡️ Crash Gauntlet & Mutant Corpus
* **Mutants Killed**:
  - **`M-bucket`**: Deliberately dropped bucket prime crossings caught immediately via overcount ($665,604 > 664,579$).
  - **`M-checkpoint`**: Corrupted checkpoint byte rejected by checksum validation.
* **Crash Gauntlet**: Interrupted at arbitrary unit boundaries, saved via atomic-rename (`.tmp` $\to$ `fsync` $\to$ `rename`), resumed with **100% bit-exact count $\pi(10^8) = 5,761,455$**.

### 🚀 Phase 4 Performance
* **8-Core $10^{11}$ Execution**: **$37.154\text{ s}$** (**$2.691\text{ Billion numbers/s}$**), improving upon Phase 3's $41.186\text{ s}$.
* **Gate Record**: Persisted in `bench/records/titan_deep_gate.json` (**ALL CRITERIA GREEN**).

---

## 7. Phase 5 Combinatorial Crown Ledger (`titan-count`)

### 🛰️ Pre-Flight Experiment C1: $\Phi$-Tree Census
* **Node Count & Stack Depth Distribution**:
  - $10^{10}$ ($a=65$): $312,833\text{ nodes}$, max depth $60$, **$0.009\text{ s}$**
  - $10^{11}$ ($a=102$): $2,565,665\text{ nodes}$, max depth $97$, **$0.054\text{ s}$**
  - $10^{12}$ ($a=168$): $22,230,535\text{ nodes}$, max depth $163$, **$0.602\text{ s}$**
  - $10^{13}$ ($a=275$): $190,648,025\text{ nodes}$, max depth $270$, **$5.510\text{ s}$**
  - $10^{14}$ ($a=446$): $1,607,709,569\text{ nodes}$, max depth $441$, **$28.945\text{ s}$**
* **The Explicit Stack Law**:
  - Pre-allocated bounded DFS stack ($4,096$ entries) consumed only $10.7\%$ of allocated capacity at $10^{14}$.
  - Zero recursion across all scales: the stack-overflow failure class is permanently eradicated.

### ⚡ Combinatorial Milestones & Attributions
* **Sub-Second $\pi(10^{12})$**: **$\pi(10^{12}) = 37,607,912,018$** in **$0.842\text{ s}$** (vs physical sieve $150+\text{ s}$).
* **$\pi(10^{13})$**: **$\pi(10^{13}) = 346,065,536,839$** in **$6.035\text{ s}$**.
* **$\pi(10^{14})$**: **$\pi(10^{14}) = 3,204,941,750,802$** in **$45.494\text{ s}$** (Table $19.3\text{s}$, $\Phi$ $25.4\text{s}$, $P_2$ $0.033\text{s}$, $P_3$ $0.76\text{s}$).

### 🛡️ Correctness & Truth Stack
* **Fourth-Power Boundary Matrix**: $\{2^4, 3^4, 5^4, 7^4, 11^4\} \pm 1$ evaluated bit-exact.
* **Cross-Engine Differential**: 12 points spanning $[10^6, 10^{10}]$ verified bit-exact between `titan-count` (combinatorial) and `titan-sieve` (physical bit sieve).
* **Full Oracle**: Passed candidate streaming protocol in **$23.04\text{ s}$** with 6/6 mutants killed.
* **Gate Record**: Persisted in `bench/records/titan_count_gate.json` (**EXIT 0**).

