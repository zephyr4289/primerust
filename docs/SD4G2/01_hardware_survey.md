# SD4G2 Hardware Survey — Baseline

**Date:** 2026-09-01  
**Device:** Snapdragon 4 Gen 2 (SM4450)  
**OS:** Termux on Android / Linux  
**Branch:** SD4G2 (diverged from main at 4333ec7)

---

## CPU Topology

| CPU | Cluster | Max Freq | Architecture |
|-----|---------|----------|--------------|
| cpu0 | Little (A55) | 1958 MHz | Cortex-A55 |
| cpu1 | Little (A55) | 1958 MHz | Cortex-A55 |
| cpu2 | Little (A55) | 1958 MHz | Cortex-A55 |
| cpu3 | Little (A55) | 1958 MHz | Cortex-A55 |
| cpu4 | Little (A55) | 1958 MHz | Cortex-A55 |
| cpu5 | Little (A55) | 1958 MHz | Cortex-A55 |
| cpu6 | Big (A78) | 2208 MHz | Cortex-A78 (part-0xd41) |
| cpu7 | Big (A78) | 2208 MHz | Cortex-A78 (part-0xd41) |

**L1D Cache:** 32 KiB per core (both A78 and A55) — **critical difference from Helio G100 (A76: 64 KiB)**

---

## Solo Pass Results (3 probes × 10 samples each)

| CPU | Cluster | Canary (ep/s) | Mem (ep/s) | Proxy Sieve (M n/s) |
|-----|---------|---------------|------------|---------------------|
| cpu0 | A55 | 127,109 | 588 | 305.69 |
| cpu1 | A55 | 166,245 | 699 | 313.32 |
| cpu2 | A55 | 170,763 | 731 | 319.75 |
| cpu3 | A55 | 168,891 | 722 | 316.14 |
| cpu4 | A55 | 170,996 | 733 | 320.21 |
| cpu5 | A55 | 170,953 | 726 | 320.12 |
| **cpu6** | **A78** | **152,434** | **1,582** | **996.67** |
| **cpu7** | **A78** | **152,289** | **1,602** | **996.91** |

---

## Cluster Inference

- **Max Gap:** 3.11x (between A55 and A78 proxy sieve)
- **Big:Little Proxy Ratio:** **3.16x** (A78 ~1.0B n/s vs A55 ~0.32B n/s)
- **Big:Little Memory Ratio:** 2.27x
- **Big:Little Canary Ratio:** 0.94x (A55 slightly higher canary — interesting)

---

## All-Core Contention Pass

| Metric | Value |
|--------|-------|
| Solo Sum Throughput | 3,888.81 M n/s |
| All-Core Aggregate | 3,414.93 M n/s |
| **Contention Efficiency** | **87.8%** |

### Per-Core Retention

| CPU | Cluster | Solo (M n/s) | All-Core (M n/s) | Retention |
|-----|---------|--------------|------------------|-----------|
| cpu0 | little | 305.69 | 306.63 | 100.3% |
| cpu1 | little | 313.32 | 316.68 | 101.1% |
| cpu2 | little | 319.75 | 322.00 | 100.7% |
| cpu3 | little | 316.14 | 316.62 | 100.2% |
| cpu4 | little | 320.21 | 320.03 | 99.9% |
| cpu5 | little | 320.12 | 318.96 | 99.6% |
| cpu6 | **big** | 996.67 | 785.72 | 78.8% |
| cpu7 | **big** | 996.91 | 728.29 | 73.1% |

**Key Insight:** Little cores show **near-zero contention** (99-101% retention). Big cores suffer **22-27% contention loss** — likely DRAM bus saturation.

---

## Comparison vs Helio G100 (Previous Platform)

| Metric | Helio G100 (A76) | SD4G2 (A78) | Delta |
|--------|------------------|-------------|-------|
| Big Core Proxy (solo) | 738.5 M n/s | **996.7 M n/s** | **+35%** |
| Little Core Proxy (solo) | 340.2 M n/s | **318.5 M n/s** | -6% |
| Big Core Memory BW | 1,496 ep/s | **1,592 ep/s** | +6% |
| All-Core Contention | 80.1% | **87.8%** | **+7.7pp** |
| Big Core Retention | 36-76% | **73-79%** | **Significant improvement** |

**Conclusion:** A78 on SD4G2 has **higher single-core throughput** but **same 32 KiB L1D** as A55. This changes optimal segment sizing — **32 KiB is now optimal for ALL cores**.