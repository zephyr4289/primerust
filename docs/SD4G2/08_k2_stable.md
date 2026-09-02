# SD4G2 K2 π-Streaming Results (Stable)

**Date:** 2026-09-01  
**Branch:** SD4G2  
**Status:** K2 implemented, K3 reverted (hurts Lehmer path)

---

## Current Performance (K2 Only)

### count_bench (8T)
| Scale | Total | Table | Phi(8T) | P2(8T) | P3(8T) |
|-------|-------|-------|---------|--------|--------|
| 10^10 | 0.032s | 0.001s | 0.010s | 0.016s | 0.005s |
| 10^11 | 0.160s | 0.001s | 0.061s | 0.082s | 0.015s |
| 10^12 | 0.369s | 0.018s | 0.138s | 0.199s | 0.014s |
| 10^13 | 1.602s | 0.023s | 0.654s | 0.883s | 0.043s |
| 10^14 | **12.142s** | 0.083s | **5.866s** | **6.012s** | 0.181s |

### count_gate (Lehmer Assembly)
| Scale | Time | Status |
|-------|------|--------|
| 10^12 | 0.643s | PASS |
| 10^13 | 4.458s | PASS |
| 10^14 | 33.626s | PASS |

---

## K2 Impact Summary

| Metric | Before K2 | After K2 | Improvement |
|--------|-----------|----------|-------------|
| count_bench 10^14 Total | 20.38s | 12.14s | **40% faster** |
| count_bench 10^14 Phi | 6.49s | 5.87s | 9.5% faster |
| count_bench 10^14 P2 | 13.47s | 6.01s | **55% faster** |
| count_bench 10^14 P3 | 0.34s | 0.18s | 47% faster |
| count_gate 10^14 | 36.6s | 33.6s | 8% faster |

**Key Insight:** K2 π-streaming provides massive P2/P3 speedups in the count_bench path (which uses direct P2/P3 calls), but more modest gains in the full Lehmer assembly (count_gate).

---

## Next Optimization Targets

### K4: Batched Magic Division (magic.rs)
- Added `div_batch8()` function for 8-wide umulh divisions
- Need to integrate into PhiEngine, IntervalWalker, P2 sweep

### K5: NEON Vectorization
- tally.rs already uses NEON vcntq_u8 for popcount
- Extend to P2 sweep inner loops, magic division batching

### Z-Split (Algorithmic)
- Restrict P2 sweep to [sqrt(x), z] where z = y * beta
- Add B term for (z, x^(2/3)]
- **Expected 60% P2 reduction** → 12s → ~5s at 10^14

### Full Gourdon Implementation
- Requires correct B + D term integration
- Current attempt had formula issues

---

## Immediate Next Steps

1. **Integrate K4 batched magic division** into PhiEngine and interval walker
2. **Implement K5 NEON** for P2 sweep inner loops
3. **Add z-split to LMO/Lehmer** path (algorithmic 60% P2 reduction)
4. **Debug Gourdon formula** for full algorithmic win