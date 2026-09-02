# SD4G2 Race Session — REAL Head-to-Head (Primecount v7.14, Primesieve v12.12)

**Date:** 2026-09-01  
**Branch:** SD4G2  
**Opponents:** primecount v7.14 (Gourdon), primesieve v12.12  
**Device:** Snapdragon 4 Gen 2 (SM4450) — 2×A78 @ 2.208 GHz + 6×A55 @ 1.958 GHz

---

## Combinatorial Race: Titan Lehmer vs Primecount Gourdon

| Scale | PC 1T | PC 8T | PC 8T/1T | Titan 1T | Titan 8T | Titan 8T/1T | Verdict |
|-------|-------|-------|----------|----------|----------|-------------|---------|
| 10^10 | 0.0368s | **0.0269s** | 1.37x | **0.0282s** | 0.0287s | 0.98x | **PC 1.07x** |
| 10^11 | 0.0381s | 0.0415s | 0.92x | 0.1433s | **0.0843s** | 1.70x | **PC 2.03x** |
| 10^12 | 0.0615s | 0.0855s | 0.72x | 0.9352s | **0.3823s** | 2.45x | **PC 4.47x** |
| 10^13 | 0.1990s | **0.1146s** | 1.74x | 6.5562s | 2.4857s | 2.64x | **PC 21.7x** |
| 10^14 | 0.7423s | **0.3273s** | 2.27x | 47.8920s | 19.8146s | 2.42x | **PC 60.5x** |

### Key Observations

1. **At 10^10: Titan 1T (0.0282s) BEATS Primecount 1T (0.0368s) and nearly matches 8T (0.0269s)** — This is the Lehmer/PhiTiny advantage at small scale!

2. **Primecount 8T scaling is WEIRD**: 1.37x at 10^10, then 0.92x (SLOWER at 8T!) at 10^11, then 0.72x at 10^12, then 1.74x at 10^13, 2.27x at 10^14. This suggests primecount's OpenMP scheduling has overhead at medium scales.

3. **Titan 8T scaling is CONSISTENT**: 0.98x → 1.70x → 2.45x → 2.64x → 2.42x. Our heterogeneous pool scales properly.

4. **THE GAP IS ALGORITHMIC**: At 10^14, Primecount Gourdon (0.327s) vs Titan Lehmer (19.8s) = **60.5x**. This is Gourdon's O(x^(2/3)/log^2 x) vs Lehmer's O(x^(3/4)).

---

## Physical Sieve Race: Titan-Sieve vs Primesieve

| Limit | PS 1T | PS 8T | PS 8T/1T | Titan 1T | Titan 8T | Titan 8T/1T | Verdict |
|-------|-------|-------|----------|----------|----------|-------------|---------|
| 1e9 | 0.3538s | **0.1404s** | 2.52x | 0.4730s | 0.1716s | 2.76x | **PS 1.22x** |
| 1e10 | 4.2962s | **1.6827s** | 2.55x | 5.8024s | 1.9712s | 2.94x | **PS 1.17x** |
| 1e11 | 50.6087s | **25.4575s** | 1.99x | 124.9171s | 51.8811s | 2.41x | **PS 2.04x** |

### Key Observations

1. **Primesieve is 1.2-2× faster at physical sieving** — highly optimized C++, decades of tuning, better cache utilization.

2. **Titan 8T scaling (2.4-2.9x) slightly BETTER than Primesieve (2.0-2.6x)** at 1e11 — our pool handles heterogeneity well.

3. **But absolute performance gap remains** — Primesieve's segmented sieve is more optimized.

---

## Comparison vs Helio G100 (Previous Platform)

| Metric | Helio G100 (A76) | SD4G2 (A78) | Delta |
|--------|------------------|-------------|-------|
| Primecount 8T @ 10^14 | 0.2748s | **0.3273s** | +19% (slower) |
| Titan 8T @ 10^14 (Lehmer) | 12.6s (race) / 11.9s (bench) | **19.8s** | +65% (slower) |
| Primesieve 8T @ 1e11 | 19.77s | **25.46s** | +29% (slower) |
| Titan 8T @ 1e11 (sieve) | 37.65s | **51.88s** | +38% (slower) |

**Wait — SD4G2 is SLOWER than Helio G100 across the board?** That's concerning. Possible causes:
- Thermal throttling more aggressive on SD4G2
- Different memory subsystem (LPDDR4X vs LPDDR5?)
- 32 KiB L1D on A78 vs 64 KiB on A76 hurting cache performance
- Background Android processes

**But the survey showed A78 single-core proxy at 996 M n/s vs A76 at 738 M n/s (+35%).** So single-core compute is better, but memory-bound workloads suffer.

---

## THE STRATEGIC IMPERATIVE

**We must implement Gourdon's algorithm (z-split + B/D terms) to close the 60x gap at 10^14.**

Current Titan uses Lehmer (O(x^3/4)). Primecount uses Gourdon (O(x^2/3/log^2 x)).

The optimization roadmap is clear:

| Phase | Optimization | Target 10^14 Time | vs Primecount |
|-------|--------------|-------------------|---------------|
| Current | Lehmer 8T | 19.8s | 60.5× behind |
| + Gourdon z-split | ~5s | 15× behind |
| + K2 π-streaming | ~3s | 9× behind |
| + K3 batching + K4 magic | ~2s | 6× behind |
| + K5 NEON | ~1.5s | 4.5× behind |
| + Full Gourdon (A term optimized) | **< 0.3s** | **WIN** |

**Next: Start implementing Gourdon z-split + B/D terms (Track B)**