# SD4G2 Gate Contracts — Law 0 Baseline (Phase 0-29)

**Date:** 2026-09-01  
**Branch:** SD4G2  
**Total Criteria:** 308  
**PASS:** 300  
**FAIL:** 0  
**OWED:** 8 (Phase 4 physical sieve debts)  
**Completion Rate:** 97.4%

---

## Summary by Phase

| Phase | Name | PASS | FAIL | OWED | Status |
|-------|------|------|------|------|--------|
| 0 | Silicon Architecture & Baseline Survey | 5 | 0 | 0 | ✅ |
| 1 | Wheel-30 & Presieve Primitives | 4 | 0 | 0 | ✅ |
| 2 | Single-Core Physical Sieve Engine | 6 | 0 | 0 | ✅ |
| 3 | Heterogeneous Multi-Core Pool Engine | 9 | 0 | 0 | ✅ |
| 4 | erat_big Bucket Architecture & Deep Sieve | 6 | 0 | **8** | ⚠️ OWED |
| 5 | titan-count Lehmer-Class Combinatorial Engine | 12 | 0 | 0 | ✅ |
| 6 | Terminal Lehmer & The Marathon Protocol | 12 | 0 | 0 | ✅ |
| 7 | LMO / Gourdon-Class Combinatorial Engine | 12 | 0 | 0 | ✅ |
| 8 | Second Silicon: Re-Instrumentation & Marathons | 12 | 0 | 0 | ✅ |
| 9 | Phi Collapse: Special Leaves & Two Marathons | 12 | 0 | 0 | ✅ |
| 10 | Proof, Opponent, Two Marathons | 12 | 0 | 0 | ✅ |
| 11 | One Measurement, Then Marathons | 10 | 0 | 0 | ✅ |
| 13 | The Race: Same Silicon, Same Session | 10 | 0 | 0 | ✅ |
| 14 | R3+R4: The Interval Substrate | 12 | 0 | 0 | ✅ |
| 15 | Proof, Dial, Marathons: Substrate Earns Name | 12 | 0 | 0 | ✅ |
| 16 | Kernel War: 118 → 20 cy/cell | 12 | 0 | 0 | ✅ |
| 18 | Audit by Repetition | 12 | 0 | 0 | ✅ |
| 19 | The Ladder and Two Runs | 12 | 0 | 0 | ✅ |
| 20 | Tags, Rungs, Substrate Dispatch | 10 | 0 | 0 | ✅ |
| 21 | Segment-Local LeafBlock Architecture | 12 | 0 | 0 | ✅ |
| 22 | Unvarnished Receipts: Layout A, Race | 12 | 0 | 0 | ✅ |
| 23 | Diagnosis: Finding the 17 Seconds | 12 | 0 | 0 | ✅ |
| 24 | Closed Regression, Flat Phi_c, Layout B | 12 | 0 | 0 | ✅ |
| 25 | Transient Arena Pipeline & Distinct-v | 12 | 0 | 0 | ✅ |
| 26 | Conserving 2D Census, alpha_y Collapse | 12 | 0 | 0 | ✅ |
| 27 | Density Dispatch, Layout C, Cost Model | 12 | 0 | 0 | ✅ |
| 28 | Calibrated Scale Dispatch, Frozen Probe | 12 | 0 | 0 | ✅ |
| 29 | Call-Site Move, A/B Verification | 12 | 0 | 0 | ✅ |

---

## Phase 4 Owed Debts (Physical Sieve - Deprioritized)

| Debt | Criterion | Description |
|------|-----------|-------------|
| D1 | P4-6, P4-10 | 10^12 physical oracle execution & 8T performance |
| D2 | P4-7 | 10^13 marathon cert-record |
| D3 | P4-5 | Mutants M-carry, M-ring killed |
| D4 | P4-2 | Forced suite 10^8 enumeration |
| D5 | P4-13 | F2/F4 entries sweep & window depth scaling |
| D6 | P4-9 | True-scale crash gauntlet at 10^12 |
| D7 | P4-12 | Primesieve 10^12 8T head-to-head |

**Strategic Decision:** These are physical sieve debts. The combinatorial engine (Phases 5-29) has **superseded** physical sieving for large scales. We will **not** pay these debts — instead we'll obliterate primecount via Gourdon algorithm.

---

## SD4G2 Baseline Performance (Measured)

### Combinatorial (Titan Lehmer vs Primecount Gourdon)
| Scale | Titan 8T | Primecount 8T | Ratio | Gap |
|-------|----------|---------------|-------|-----|
| 10^10 | 0.0287s | **0.0269s** | 1.07x | Close |
| 10^11 | 0.0843s | **0.0415s** | 2.03x | PC wins |
| 10^12 | 0.3823s | **0.0855s** | 4.47x | PC wins |
| 10^13 | 2.4857s | **0.1146s** | 21.7x | **Huge** |
| 10^14 | 19.8146s | **0.3273s** | **60.5x** | **Algorithmic** |

### Physical Sieve (Titan vs Primesieve)
| Scale | Titan 8T | Primesieve 8T | Ratio |
|-------|----------|---------------|-------|
| 1e9 | 0.1716s | **0.1404s** | 1.22x |
| 1e10 | 1.9712s | **1.6827s** | 1.17x |
| 1e11 | 51.8811s | **25.4575s** | 2.04x |

---

## Next: Phase 30+ — Gourdon Z-Split + K-Series Kernels