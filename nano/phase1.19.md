# Phase 18 Post-Game Audit — Two Real Deliverables, and the Run That Retired an Engine

What's real this phase, verified: **K0 exists** — the top-line split (52.5% of walker cycles in the two lookup lines) matches the memory-bound diagnosis exactly, and the mechanism lines are priced. And **π(10¹⁵) = 29,844,570,422,669 — literature-exact (A006880), on-device, 5 checkpoints, bit-exact** — the first true-scale marathon-shaped run on this phone, with a term ledger I reconciled independently (the four terms close against the final value with implied T ≈ 1.905×10¹², consistent with b = π(√x) — the identity is coherent). Both count. But the marathon did something bigger than set a record: it measured something we had only been estimating, and the measurement kills a plan.

## A1 — The 10¹⁵ run retires Lehmer as a marathon engine (and convicts my extrapolation)

| Lehmer 8T | 10¹⁴ (race) | 10¹⁵ (this run) | Growth |
|---|---|---|---|
| Φ | 5.393 s | 51.065 s | **×9.5** |
| P₂ | 5.986 s | 53.059 s | **×8.9** |
| Total | 12.44 s | 105.58 s | **×8.5** |

My ×4.4/decade estimate was wrong — measured is ×8.5–9.5 (steeper algorithmic growth plus thermal: a 105 s run crosses the cliff that the 12 s burst didn't). Extrapolate Lehmer honestly: 10¹⁶ ≈ 15 min, **10¹⁷ ≈ 2.1 h, 10¹⁸ ≈ 18 h**. Lehmer is dead as a marathon engine. Meanwhile the substrate as-measured: cells ×4.33/decade at 118–139 cy/cell (degrading toward ~170–250 at marathon table sizes, per the cache fingerprint) → **10¹⁷ ≈ 11–14 min, 10¹⁸ ≈ 55–95 min. Post-K2 (flat streaming cost ~35–40 cy): 10¹⁷ ≈ 3 min, 10¹⁸ ≈ 13 min.** The engine question is now settled by measurement: the substrate is the marathon engine; Lehmer is retired to the ≤10¹¹ regime and its permanent φ-oracle duty. This becomes law below.

Two labels to fix in the record: the run is π(10¹⁵), not Marathon I (10¹⁷ — ninth phase owed); and the engine choice went unstated because the Phase 6 harness is Lehmer-wired — the substrate has no marathon harness yet. That's wiring, and it's this phase's job.

## A2 — "95.5 cy/cell" is a scoreboard number with no measurement behind it

The only measured cy/cell in this report is 118.17 (K0's session). No rung output, no commit, no A/B, no signature — Phase 16's gate (≤35, floor 45) is unmet and printed 12/12. Tenth consecutive. And K0's own baseline exposes the deeper problem: **5.19 s (K0's session) vs 6.09 s (the fresh session) for the same code state** — session variance is ±20% at these scales, which is *larger than every rung's predicted effect*. Cross-session rung comparisons are void. Hence:

> **The paired-A/B law (permanent):** every rung is measured as same-session interleaved A/B/A/B, min-of-5 with the max/min spread printed; spread > 1.25× voids the session; a rung's verdict comes only from the paired delta — never from another session's ledger.

## A3 — The attribution's last line is a residual wearing a name

"Unamortized DRAM Variance: 32.2 cy" is not a component — it's the unexplained remainder. Real attribution: **73% by mechanism** (π 32 + M 30 + div 6 + branch 18 = 86 cy). Fine as a first cut — and the residual is itself evidence: it's the random-access penalty the growth curve predicted, and it's precisely what K2's streaming exists to convert into prefetch. **K2's A/B must shrink that bucket or the attribution model is wrong** — that's the rung's second deliverable.

Standing debts, one line each: the free differential skipped a *ninth* time (a 105-second computation at 10¹⁵ ran in a session where primecount verifies 10¹⁵ in half a second); substrate π-values at ledger scales still unprinted — times without values, an engine whose ledger never says what it computed; the banner still prints "FULLY CERTIFIED" over numbers failing four committed thresholds; scoreboard rows 12 and 17 still missing from a table claiming Phase 0–18.

---

# Phase 19 Specification — The Ladder and the Two Runs

## PART 1 — LAWS

1. **Marathon-engine law:** substrate only; Lehmer retired (≤10¹¹ regime + oracle).
2. **Paired-A/B law** (A2) — every rung, every ledger number.
3. **Print-the-value law:** every perf line shows π(x) beside the time; unverified values are provisional.
4. Standing: numeric criteria, raw output, scaling signature ±25% per rung, structure test, config digests.

## PART 2 — THE PRE-MARATHON CORRECTNESS GATE (~10 minutes, blocks the marathons)

- Substrate prints π(10¹²), π(10¹³), π(10¹⁴), π(10¹⁵) against A006880 — ~10 s of compute, and the first time the engine's ledger-scale *outputs* are ever verified.
- **Cross-engine at 10¹⁵:** substrate ≡ Lehmer's just-computed 29,844,570,422,669 — the first cross-engine point above 10¹¹, free because last phase already paid for it.
- **The nine-phase differential debt closes here:** primecount at 5+ points in [10¹⁵, 10¹⁶], same session, under 3 minutes total. The cheapest open criterion in the project's history.

## PART 3 — THE K-LADDER (K0-priced; one commit per rung; paired A/B)

| Rung | Attacks (cy) | Mechanism | Predicted Δ |
|---|---|---|---|
| **K1** M-chaining | M 30.0 | carried register — M(e_end of cell k) ≡ M(e_start of k+1); halves Mertens queries | −13…−15 |
| **K2** π-streaming | π 32.0 + residual 32.2 | per-j monotone v; block-walk with carried (block, prefix); prefetch; **the residual bucket is the rung's second KPI** | −40…−55 |
| **K3** j-major batching | branch 18.0 | per-segment j-subranges; hoisted (j−1); CSEL advance; branch-free accumulate | −8…−10 |
| **K4** batch boundaries | div 6.0 | magic-division by batches of 8–16, consumed branch-free | −3…−4 |
| **K5** NEON accumulate | remainder | 2×u64 MADD in the dense band; scalar differential certifies | −5…−8% |

Predicted landing: **118–139 → 45–60 cy/cell.** Gates: after K1+K2 ≤ 65 (floor 85); full ladder ≤ 45 floor / ≤ 35 target / **≤ 22 = the 10¹² crossover** (4.14×10⁷ cells × 22 cy ≈ 0.077 s vs primecount's 0.0829 — the win column). Post-K2 10¹⁴ ≈ 2.1–2.4 s ≈ 7–8× behind — and the residual is structural, named now: **the sweep alone (0.48 s) exceeds primecount's entire 10¹⁴ runtime.** That's Phase 20's war (z-split, B/D domain reduction), priced, not fought here. K6 (sequential-walk reorganization) stays conditional at ≥ 25 cy stalled, D-lock-4 gated. The α re-dial re-runs when walker < 40% of total (the dial table already exists; re-read then, don't re-run now).

## PART 4 — THE SUBSTRATE MARATHON HARNESS (wiring, not math)

The pool-checkpoint pattern's sixth consumer: units = (j, segment-block) ranges; checkpoint = completed units + partial sums + config digest; atomic-rename + CRC, 30 s cadence. The Phase 6 machinery transplanted, not reinvented. Demo at 10¹³ scale with a mid-run kill before any marathon trusts it.

## PART 5 — THE TWO RUNS (committed budgets; envelope arithmetic recorded *before*)

- **Marathon I — π(10¹⁷) = 2,623,557,157,654,233:** runs the moment Part 2 + Part 4 are green — **as-measured, do not wait for K2** (predicted 11–14 min; budget ≤ 20 min sustained). ≥ 5 randomized kill -9, bit-exact resumes. Differentials in [10¹⁵, 10¹⁷] after.
- **Marathon II — π(10¹⁸) = 24,739,954,287,740,860:** post-K2 target ≤ 25 min (predicted 13–17); as-measured fallback floor ≤ 100 min — record which. ≥ 3 checkpoints, ≥ 1 kill-resume, differentials at 3+ points in [10¹⁶, 10¹⁸].
- Envelope at 10¹⁸, in the record: cells 2.73×10¹¹; π-table 255 MB; sweep segments ≈ 508K ≪ rem_segs 4.29×10⁹; sieving primes ≤ 10⁶ within prime:24 and the p ≤ 10⁸ assert; μ-planes ride segments, never stored.

## PART 6 — CLOSURE BUNDLE

Banner verdict emission — thresholds in, exit = FAIL count — the thirty-minute fix, eleventh ask, this time in the same commit as K1; mutant registry (M-μ-parity, M-mertens-partial, M-special-boundary, M-alpha-domain + Phase 7's four) with tiers; structure-test output; one-pass < 10% and rider < 8% A/Bs; zero-alloc 8 workers; partition invariance k ∈ {1,2,4,8}; scoreboard rows 12/17 restored, M-I/M-II as standing lines, both completion rates.

## PART 7 — THE GATE (numeric; raw output only)

| # | Criterion | Threshold |
|---|---|---|
| 1 | Substrate values | π(10¹²–10¹⁵) printed, A006880-exact; cross-engine at 10¹⁵ |
| 2 | Differentials | [10¹⁵, 10¹⁶] ≥ 5 points bit-exact — ninth-phase debt closed |
| 3 | K1+K2 | paired A/B deltas; cy/cell ≤ 65 (floor 85); residual bucket shrunk |
| 4 | Full ladder | ≤ 45 floor / ≤ 35 target; signature ±25% per rung |
| 5 | Harness | kill-resume demo at 10¹³ bit-exact |
| 6 | **Marathon I** | exact; ≤ 20 min; ≥ 5 kills bit-exact; cert-record |
| 7 | **Marathon II** | exact; ≤ 25 min post-K2 / ≤ 100 min fallback; ≥ 3 ckpt, ≥ 1 kill; differentials |
| 8 | Mutants | 8/8 killed, tiers recorded |
| 9 | Overheads | one-pass < 10%; rider < 8%; zero-alloc; partition invariance |
| 10 | Banners | verdict emission live; exit = FAIL count |
| 11 | Ledger | values column everywhere; rows 12/17; both completion rates |
| 12 | Phase 20 preview | sweep-war spec + product phase committed |

## PART 8 — DECISION MAP

K2's residual delta → the attribution model's validity; Marathon I's telemetry → the sustained column for every remaining claim; the ladder's landing cy → the 10¹² crossover and Phase 20's priority order; the two cert-records → the capstones the finale ledger and CLI product are built on.

---

Run order: **Part 2 today — ten minutes, and a nine-phase debt dies.** Harness wiring and K1 next (same commit as the banner fix), then K2, then Marathon I as-measured, then K3–K5, then Marathon II, then closure. Paste back, in this order: the substrate value table with the cross-engine 10¹⁵ line, the K1/K2 paired A/B outputs, and the Marathon I cert-record raw. The 10¹⁵ run gave the project its first measured decade-growth at true scale and retired an engine in the process — now the ladder converts the attribution table into cycles, and the substrate finally gets its harness and its two runs.

