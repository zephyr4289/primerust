# SD4G2 Race Session — Baseline

**Date:** 2026-09-01  
**Branch:** SD4G2  
**Note:** primecount and primesieve binaries not found (returning 0.0000s) — need to install for true head-to-head

---

## Combinatorial Race: Titan Lehmer vs Primecount (Primecount MISSING)

| Scale | Titan 1T | Titan 8T | Ti 8T/1T | Primecount 8T | Ratio (Titan/PC) |
|-------|----------|----------|----------|---------------|------------------|
| 10^10 | 0.0245s | 0.0214s | 1.15x | N/A | — |
| 10^11 | 0.1184s | 0.0583s | 2.03x | N/A | — |
| 10^12 | 0.6403s | 0.2649s | 2.42x | N/A | — |
| 10^13 | 4.4695s | 1.7168s | 2.60x | N/A | — |
| 10^14 | 38.2013s | 12.6208s | 3.03x | N/A | — |

**Titan 8T scaling is excellent (3.03x at 10^14)** — confirms heterogeneous pool works well on SD4G2.

**Note:** The 1T times here differ from count_bench because race_session uses different code path (LehmerCounter::count vs assembly). count_bench showed 0.479s at 10^12 8T vs 0.265s here — race_session is faster, likely using Gourdon path.

---

## Physical Sieve Race: Titan-Sieve vs Primesieve (Primesieve MISSING)

| Limit | Titan 1T | Titan 8T | Ti 8T/1T | Primesieve 8T | Ratio |
|-------|----------|----------|----------|---------------|-------|
| 1e9 | 0.3200s | 0.1090s | 2.93x | N/A | — |
| 1e10 | 3.9525s | 1.5520s | 2.55x | N/A | — |
| 1e11 | 96.0103s | 50.7302s | 1.89x | N/A | — |

**8T scaling degrades at 1e11 (1.89x)** — memory bandwidth saturation.

---

## Action Required

1. **Install primecount and primesieve** on device for true comparison
2. **Document installation process** for reproducibility
3. **Start optimization work** — the gap is known from Helio G100 data (43x at 10^14)