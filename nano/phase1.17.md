# Phase 15 Post-Game Audit — The Substrate Is Real, Class-Correct, and the Entire War Is Now One Number

Credit first, with receipts, because this phase crossed a line the project has been approaching for four phases: **the interval substrate is measured, identity-certified, and — for the first time in this project's history — an engine whose measured growth curve matches the opponent's complexity class.** Walker time tracks cells (×4.33/decade = x^(2/3)), not μ-span (×10/decade = O(x)) — the scaling signature holds at 118/94/118 cy/cell, which is the *instrument* proving the walker does interval arithmetic rather than element-walking. Identity: 20/20 bit-exact against the Lehmer oracle. Mertens: all five anchors exact. The term ledger is honest. And the substrate is already the project's best engine at scale: 1.03 s vs Lehmer's 1.91 s at 10¹³, 5.54 vs 12.44 at 10¹⁴. The selector flips to substrate at ≥ 10¹³, as-measured, today.

Now the audit table, because the banner lied again:

| Phase 15 criterion | Threshold | Measured | Verdict |
|---|---|---|---|
| V1 suite, 9 instruments | all green | **4 of 9 evidenced** (identity, anchors, ledger, signature) | FAIL, printed "FULLY CERTIFIED" |
| Collapse ratio | ≥ 15× | **3.53×** | FAIL |
| π(10¹⁴) | ≤ 1.5 s | **5.54 s** | FAIL |
| D-lock-3 term oracles, structure test, one-pass A/B, rider A/B, zero-alloc, partition invariance | green | no output | OWED |
| α-sweep, mutants, differentials [10¹⁵, 10¹⁶] | recorded | no output | OWED (differentials: **eighth** phase) |
| Marathons I & II | run | not run | **Ninth** phase. Scoreboard still missing Phase 12's row; M-I/M-II still not standing lines |

Eighth consecutive gate printing green over failed and unmeasured criteria. The honest Phase 15 score is ~2 of 12. But here is why this phase's failure is *productive* failure — the diagnostic the ledger handed us is the cleanest position the project has ever held:

## The one-number diagnosis

Decompose the 5.5414 s at 10¹⁴: **walker 5.1928 s (94%) + sweep 0.3486 s + table/assembly ~0.015 s.** Now delete the walker entirely: 0.36 s against primecount's 0.2748 s — **1.31×. Near parity.** The entire 20.2× gap at 10¹⁴ is one kernel: the walker's measured **118 cy/cell**, against the ~7–8 cy/cell the Phase 15 kernel design targeted. 118 is not a mystery — it is the fingerprint of **three unamortized random L2 lookups per cell** (π(v) at ~28–32 cy, plus two independent Mertens points at ~28–32 each ≈ 90 cy) plus magic-division, plus branch/state overhead ≈ 118. The report labels the walker "monotone-v streaming" — the *label* is doing work the *code* isn't. What shipped is interval arithmetic at scalar-random-lookup grade; the streaming kernel (per-j monotone v, block-walked π-table, chained Mertens, CSEL advance) was specified in Part 5 and not built.

And one identity, stated once, worth ~25 free cycles: **within a fixed j, the e-runs partition the domain — so M(e_end of cell k) IS M(e_start of cell k+1), exactly, by construction.** One of every cell's two Mertens lookups is the previous cell's other endpoint. A carried register, not a memory bounce. The kernel's own mathematics is handing us the first optimization for free.

Two more receipts: the substrate's own π(x) values at 10¹²–10¹⁴ (its ledger scales!) are not printed anywhere — identity was certified on φ terms at [10⁶, 10¹⁰], the totals show *times* but not *values*; print them. And the cy/cell sawtooth (118/94/118 — dip at 10¹³) is a cache-geometry signature worth one flag in the K0 attribution, not a finding.

## The marathon arithmetic, from measured numbers only — the excuse chain dies here

| Run | Substrate as-measured (cool, 8T) | Lehmer (measured extrapolation) | primecount (est) |
|---|---|---|---|
| π(10¹⁷) | walker 6.3×10¹⁰ cells × 118 cy = **~7.1 min** + sweep 35 s ≈ **~8 min** | ~21 min | ~1 min |
| π(10¹⁸) | walker 2.73×10¹¹ cells × 118 = **~30 min** + sweep 163 s ≈ **~33–35 min** | ~2.6 h | ~70 s |

The substrate *as it ran last night* beats the Lehmer fallback by 4.5× at 10¹⁸ and delivers both marathons at budgets cheaper than the Lehmer plan I proposed three phases ago. There is no remaining engineering reason the marathons haven't run. None. They are a standing OWED line, nine phases deep, and Phase 16 executes them *before* the kernel war lands — because the records exist to be beaten, and a 33-minute 10¹⁸ that later becomes a 5-minute 10¹⁸ is a *stronger* ledger than one number with no baseline.

---

# Phase 16 Engineering Specification — The Kernel War: 118 → 20

One metric rules the phase. Every rung's deliverable is its measured cy/cell delta; every other number is downstream.

## PART 1 — MANDATE AND LAWS

**Scope:** the K-series kernel rungs, the α-dial, Marathons I & II, the closure bundle, re-race v3. **Frozen:** sweep, pool, engines' mathematical surface. **Deferred to Phase 17:** the sweep war (z-split, B/D domain reduction, wheel-210, presieve extension — see Part 8) and the product phase (CLI/FFI/finale).

1. **The kernel-metric law:** cy/cell is the phase's single gate metric, measured by the standing node-accounting counters under the timing brackets, at three scales, scaling signature ±25% enforced at *every* rung — a rung that buys cy/cell by regressing toward μ-span walking is reverted on its own signature.
2. **The marathons-first law:** M-I runs before any kernel rung merges. The substrate as-measured is the marathon engine; every K-rung that lands before M-II multiplies its margin, but none *blocks* it.
3. **Numeric-criteria, raw-output, config-digest, evidence-first** — standing, non-negotiable.
4. **The structure test stays hard-gate** — j-major batching (Part 3, K3) must still ride the sweep as per-segment j-subranges; if K6's deeper reorganization supersedes the structure, that's a D-lock-4 decision with term oracles first, not a silent relaxation.

## PART 2 — K0: THE ATTRIBUTION (one instrumented run, 10 minutes)

Instrument the walker with per-cell counters: π-lookup count, M-lookup count, divisions, run-boundary branches, accumulator ops. Deliverable: a table attributing ≥ 90% of the 118 cy. Predicted split (to be confirmed or refuted by measurement): lookups ~90 (3 × ~30), division ~5, state/branch ~20. **No K-rung merges before this table exists** — the war shoots at the attributed components, not at vibes.

## PART 3 — THE RUNGS (one change per commit; ±3% keep-or-revert; oracle-quick between; predicted cy/cell)

| Rung | Change | The mechanism, precisely | Predicted cy/cell |
|---|---|---|---|
| **K1** | M-chaining | Carried register: M(e_end)→M(e_start) of the next cell (the identity above); the second Mertens point becomes free; checkpoint positions stream monotonically per j | 118 → 85–95 |
| **K2** | π-streaming per-j | v descends monotonically per j → block-walk the π-table with carried (block, prefix) state instead of random lookups; the census's 51.8:1 v-sharing becomes locality instead of waste | → 55–70 |
| **K3** | j-major segment batching | Per-segment per-j contiguous e-subranges (the Phase 6 threshold-slicing pattern, seventh consumer); hoist (j−1) and the sign; CSEL advance, branch-free accumulate | → 35–50 |
| **K4** | Batch run-boundaries | Precompute next-run boundaries in batches of 8–16 via magic division (sixth consumer), consume branch-free | → 25–40 |
| **K5** | NEON accumulate | 2×u64/4×u32 multiply-add on cell contributions in the dense band; scalar differential certifies (the standing SIMD-swap law) | −10–20% |
| **K6** | *Conditional* — the sequential-walk reorganization | Only if stalled ≥ 25 cy: per-j, the cell stream is a *merge of two monotone streams* (quotients and μ-checkpoints) — restructure as a single walk with running counters, the form primecount actually runs. **D-lock-4 gate: term oracles before any code** | ≤ 10 |

## PART 4 — THE α-DIAL (finally, fourth phase asked)

Census-owned: measure **cells(α)** at 10¹³/10¹⁴ for α ∈ {1.0, 1.5, 2.0, 3.0} — the cell count at α=1 is 11.6× the literature ideal, and α is the knob that shaves cells while the K-rungs shave per-cell cost; the two multiply. Term oracles re-run at each α; **M-alpha-domain is killed here** (fifth phase owed). Selection rule, committed now: the α minimizing *measured total* at 10¹⁴, per scale, into the device profile.

## PART 5 — THE MARATHONS (measured budgets, committed targets)

**Marathon I — tonight, substrate as-measured:** π(10¹⁷) = 2,623,557,157,654,233. Committed budget: **≤ 12 min sustained** (as-measured arithmetic: ~8 min cool × thermal factor). Cert-record: term ledger, telemetry curve, ≥ 5 randomized kill -9 with bit-exact resumes, runtime asserts (a-domain, field-domain, scaling-signature live), charging flags. Differentials: primecount spot-checks at 3+ points in [10¹⁵, 10¹⁷].

**Marathon II — mid-phase, post-K1/K2 (every landed rung multiplies the margin; do not wait for the full ladder):** π(10¹⁸) = 24,739,954,287,740,860. Committed budget: **≤ 75 min sustained as-measured / ≤ 30 min if K1–K3 land first** — record which. ≥ 3 checkpoints, ≥ 1 kill-resume, post-run differentials at 3+ points in [10¹⁶, 10¹⁸] — and the five owed [10¹⁵, 10¹⁶] points, same session, eighth-phase debt paid.

Envelope arithmetic recorded *before* both runs, per the field-domain law: cells(10¹⁸) = 2.73×10¹¹ → 3.2×10¹³ cycles ≈ 30 min wall ✓; μ-planes ride segments, never stored ✓; π-table 255 MB ✓; sweep segments ≪ rem_segs ceiling ✓.

## PART 6 — THE CLOSURE BUNDLE (one session, the full standing debt)

D-lock-3 term oracles (six terms, brute-forced, invalid-α rejection — **third phase owed**); structure-test output; one-pass A/B (< 10%) and rider A/B (< 8%); the mutant registry (M-μ-parity, M-mertens-partial, M-special-boundary, M-alpha-domain + Phase 7's four) — all kills with tiers; zero-alloc with 8 workers; partition invariance k ∈ {1,2,4,8}; **the substrate's own π values printed and verified at 10¹²/10¹³/10¹⁴**; the matrix/race reconciliation with config digests (Phase 13 F6, still open); scoreboard: Phase 12's row restored, M-I/M-II as lines, both completion rates.

## PART 7 — RE-RACE v3 AND THE CROSSOVER COMMITMENT

Same-session, definition-of-winning law, config digests. The committed crossover arithmetic — this is what the kernel war buys, stated before it's fought:

| cy/cell achieved | π(10¹²) | π(10¹³) | π(10¹⁴) |
|---|---|---|---|
| 118 (now) | 0.306 s — 3.7× behind | 1.03 s — 9.5× | 5.54 s — 20.2× |
| **≤ 20 (gate)** | **~0.06 s — WIN ~1.4×** | ~0.28 s — ~2.6× | ~1.9 s — ~7× |
| ≤ 10 (stretch) | ~0.05 s — WIN ~1.7× | ~0.19 s — ~1.8× | ~1.2 s — ~4.4× |
| ≤ 7 (ideal) | 0.046 s — WIN 1.8× | 0.16 s — 1.5× | 0.67 s — 2.4× |

Selector, updated per rung: Lehmer holds 10¹⁰–10¹¹ (the setup-regime crown: 0.0334 s at 10¹⁰, certified 2.71×, ST-dispatched), substrate takes ≥ 10¹² the moment cy/cell ≤ 20 flips 10¹² to a win. **Even at the ideal kernel, 10¹⁴ lands ~2.4× — and the reason is structural, named now:** our *sweep alone* (0.35 s) exceeds primecount's entire 10¹⁴ runtime. Their Gourdon B/D terms do not pay for the full [x^½, x^(2/3)] interval the way we do. That residual is Phase 17's war, not this one's.

## PART 8 — THE GATE (numeric; raw output only; exit = non-PASS count)

| # | Criterion | Threshold |
|---|---|---|
| 1 | **Marathon I** | π(10¹⁷) exact; ≥ 5 kill-resumes bit-exact; ≤ 12 min sustained; differentials recorded |
| 2 | **Marathon II** | π(10¹⁸) = 24,739,954,287,740,860 exact; ≥ 3 checkpoints, ≥ 1 kill-resume; ≤ committed budget; differentials above 10¹⁶ |
| 3 | K0 attribution | ≥ 90% of 118 cy attributed by counter |
| 4 | **Kernel gate** | cy/cell ≤ 35 (floor 45); **stretch ≤ 20 → 10¹² win certified in re-race** |
| 5 | Scaling signature | ±25% at every merged rung, three scales |
| 6 | α-sweep | cells(α) table at 2 scales; α in profile; M-alpha-domain killed |
| 7 | D-lock-3 | six term oracles green incl. invalid-α; third-phase debt closed |
| 8 | Mutants + differentials | registry killed with tiers; [10¹⁵, 10¹⁶] ≥ 5 points bit-exact |
| 9 | Overheads + hygiene | one-pass < 10%, rider < 8%; zero-alloc 8 workers; partition invariance |
| 10 | Substrate outputs | π values printed & verified at 10¹²/10¹³/10¹⁴ |
| 11 | Re-race v3 | crossover table with config digests; both completion rates; Phase 12 row restored |
| 12 | Phase 17 preview | sweep-war spec committed (z-split, B/D domains, wheel-210) + product phase |

## PART 9 — DECISION MAP

The K0 attribution table → which rungs are real vs. imagined waste; the cy/cell landing point → the 10¹²/10¹³ crossover and the honest gap column; the α table → the cell-count side of the product; the two cert-records → the capstone artifacts the CLI demos and the finale ledger is built on; K6's conditional trigger → whether Phase 17 opens with the sequential-walk reorganization or the sweep war. And the standing truth this phase's ledger made unavoidable: **the algorithm war is over — class-correct, measured, identity-certified. What remains is 94% one kernel's constants, then one sweep's structure, then the product.** Three fights, in that order, each with a number on it.

---

Run order: **Marathon I tonight on the substrate as-measured — nothing precedes it.** K0 and the α-sweep ride the idle little cores during the run. Then K1→K4 one commit at a time, Marathon II after K2, closure session, re-race. Paste back, in this order: the M-I cert-record raw, the K0 attribution table, the α table, and — as the rungs land — each rung's cy/cell line. The number we're all waiting for is 20. When the walker prints it, this project holds a win column at 10¹² against the best engine ever written, on the same phone, in the same session — and the marathons, finally run, will already be in the ledger waiting to be beaten.
