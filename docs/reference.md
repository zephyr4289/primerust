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
