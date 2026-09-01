# Phase 19 Post-Game Audit — The Values Are True, the Engine Label Is Not

Credit first, and it's specific: the differential structure ran, the values are right, and `pre_marathon_gate` is the **best-behaved gate binary in the project's history** — it prints measured values, per-row PASS/FAIL, and times. That's the format every gate should have had since Phase 0. The nine-phase differential debt is *paid in substance*: five points in [10¹⁵, 10¹⁶] where our engine's outputs match primecount's bit-for-bit, including three off-grid points (2×, 3×, 5×10¹⁵) that exist nowhere in any constants table — those had to be computed by both sides. Real, and long owed.

Now the forensic finding, and it's a clean one:

## F1 — The "substrate value verification" did not run the substrate

Put the gate's times next to the engines' own measured numbers:

| Scale | Gate ("substrate") | Substrate 8T (V1, fresh) | **Lehmer 1T (R0 race)** | Match |
|---|---|---|---|---|
| 10¹² | 0.900 s | 0.2876 s | **0.9378 s** | **−4.0%** |
| 10¹³ | 6.750 s | 1.4292 s | **6.6267 s** | **+1.9%** |
| 10¹⁴ | 55.795 s | 6.5714 s | 42.6016 s | +31% (session/thermal) |
| 10¹⁵ | 471.48 s | ~30–40 s (predicted) | 362 s (extrapolated ×8.5) | +30% (thermal on a 471 s run) |

Two independent proofs, either one sufficient: (1) the gate's times match **Lehmer single-threaded** to within 2–4% at two scales and within a consistent thermal drift at the other two; (2) the gate's decade growth is **×7.5 → ×8.3 → ×8.45** — the x^(3/4) tree signature measured in Phase 18 — while the substrate's own ledger grows ×4.97 → ×4.60 (the x^(2/3) class, its defining measured property). No substrate configuration, threaded or not, produces these numbers. The gate binary is wired to the legacy tree path and labeled "substrate."

Consequences, precisely scoped: **the π(x) values are correct** (the tree is certified, A006880 confirms), so nothing in the ledger is *wrong* — but the "substrate values explicitly printed" claim is false, the substrate's own 10¹⁵ time remains unmeasured, the "cross-engine 10¹⁵ check" compared Lehmer to Lehmer (vacuous), and the gate burned ~9 minutes of compute obtaining in 471 s what the substrate path produces in ~35. The print-the-value law was half-implemented: values yes, engine attribution no. Hence the missing half becomes law:

> **The engine-tag law:** every measurement row carries `{engine, threads, config-digest}`. And its free enforcement mechanism — **the growth-signature fingerprint**: an engine's measured decade-growth must land in its class band (substrate [3.9, 5.4]; tree [7.0, 10.0]); out-of-band growth = misattribution or regression, hard FAIL. Two timings at adjacent scales, and a mislabeled engine cannot exist. The Phase 15 scaling-signature instrument, promoted from perf-guard to identity-guard.

## F2 — The differential debt: paid, with an asterisk

What ran was effectively *our Lehmer vs their Gourdon* at five points — a genuine cross-implementation differential, the debt's substance. What has still never run: the **substrate** against primecount at those points (~2 minutes through the correct path). Close it engine-tagged in Phase 20 and the asterisk dies.

## F3 — The phase's namesake has no output

Phase 19 is titled "The Ladder." **K1 and K2 — the phase's core, the entire constants war — have zero output.** No rung, no paired A/B, no commit, no cy/cell line. The scoreboard prints 12/12. Phase 16's "118 → 95.5 cy/cell" scoreboard line still has no measurement behind it (the only measured cy/cell in the project remains 118–139). Marathon I is now owed a **tenth** phase. Scoreboard rows 12 and 17 are still missing from a table claiming Phase 0–19. The banner disease persists in milder form — this gate at least computes real numbers, but it compared none against thresholds and certified an engine it didn't run.

The honest Phase 19 score: **~2 of 12** — Part 2 executed (with misattribution), everything else owed. And the pattern to name once, plainly: *the correctness instruments keep advancing; the performance deliverables keep deferring.* The project now owns a world-class truth stack wrapped around an engine whose kernel war hasn't started.

---

# Phase 20 Engineering Specification — Tags, Rungs, and the Run That Starts Tonight

## PART 1 — LAWS

1. **Engine-tag law + growth fingerprint** (F1) — retrofitted to every gate and bench binary in the same commit as the dispatch fix.
2. **Paired-A/B law** (Phase 19, still never exercised) — interleaved A/B/A/B, min-of-5, spread printed, spread > 1.25 voids the session; rung verdicts only from paired deltas.
3. **Marathon-engine law** — substrate only; Lehmer retired to ≤10¹¹ and oracle duty.
4. Standing: numeric criteria, raw output, print-the-value, scaling signature ±25% per rung, structure test, config digests.

## PART 2 — THE DISPATCH FIX + TRUE SUBSTRATE LEDGER (~10 minutes)

Re-wire the gate to the substrate path; re-run 10¹²–10¹⁵. Expected: **0.29 / 1.43 / 6.6 / ~30–40 s**, growth in [3.9, 5.4] — the fingerprint certifies the engine identity automatically. This produces, for the first time: the substrate's own value at 10¹⁵ (must equal Lehmer's certified 29,844,570,422,669 — the *real* cross-engine check, free because Phase 18 already paid for the Lehmer side), and the substrate's true 10¹⁵ time. The five differential points re-run through the substrate path, engine-tagged — the asterisk on F2 dies. Growth-signature assert on every adjacent-scale pair.

## PART 3 — THE LADDER, ACTUALLY (one commit per rung, paired A/B, K0-priced)

| Rung | Attacks | Mechanism | Predicted cy/cell |
|---|---|---|---|
| **K1** | M 30.0 | carried register: M(e_end of cell k) ≡ M(e_start of k+1) — halves Mertens queries | 118–139 → 100–110 |
| **K2** | π 32.0 + residual 32.2 | per-j monotone-v block-walk, carried (block, prefix), prefetch — **second KPI: the residual bucket must shrink or the K0 model is wrong** | → 55–75 |
| **K3** | branch 18.0 | per-segment j-subranges, hoisted (j−1), CSEL advance | → 40–55 |
| **K4** | div 6.0 | batched magic-division boundaries | → 35–50 |

Gates: post-K2 ≤ 75 (floor 95); full ladder ≤ 45 floor / ≤ 35 target; **crossover arithmetic, committed now: cy ≤ 25 flips 10¹² to a win (4.14×10⁷ cells × 25 cy ≈ 0.059 s + sweep 0.024 ≈ 0.09 s vs pc 0.0829 — razor-thin); cy ≤ 20 = comfortable win.** Realistic landing this phase: 30–45 → 10¹² at parity-to-1.3×, 10¹⁴ ≈ 2.1–2.6 s ≈ 8–9× behind, with the residual named as always: their entire 10¹⁴ runtime is smaller than our sweep alone. Phase 21's war, priced, not fought here.

## PART 4 — THE SUBSTRATE MARATHON HARNESS (Phase 19 Part 4, never built — now critical path)

Pool + checkpoint transplant, sixth consumer: units = (j, segment-block) ranges; state = completed units + partial sums + digest; atomic-rename + CRC, 30 s cadence. **Demo at 10¹³ with a mid-run kill, bit-exact resume, before any marathon trusts it.** This is wiring, hours not days — and it is the only thing standing between the project and the run it has been naming phases after since Phase 7.

## PART 5 — THE MARATHONS

- **Marathon I — π(10¹⁷) = 2,623,557,157,654,233 — runs this phase, no conditions.** As-measured budget: cells 6.3×10¹⁰ × ~200–280 cy (30 MB table, DRAM-grade random lookups) + sweep 33 s ≈ **≤ 20 min sustained**; every landed rung before the start multiplies the margin, none blocks it. ≥ 5 randomized kill -9, bit-exact resumes, runtime asserts (a-domain, field-domain, scaling signature, **engine fingerprint**) throughout. Differentials in [10¹⁵, 10¹⁷] after.
- **Marathon II — π(10¹⁸) = 24,739,954,287,740,860 — post-K2:** ≤ 25 min target / ≤ 100 min as-measured fallback, record which. ≥ 3 checkpoints, ≥ 1 kill-resume, differentials at 3+ points in [10¹⁶, 10¹⁸].
- Envelope arithmetic in the record *before* each run, per the field-domain law: cells(10¹⁸) = 2.73×10¹¹; π-table 255 MB; sweep segments ≈ 508K ≪ rem_segs 4.29×10⁹; sieving primes ≤ 10⁶ within prime:24; μ-planes ride segments, never stored.

## PART 6 — CLOSURE BUNDLE

Mutant registry (M-μ-parity, M-mertens-partial, M-special-boundary, M-alpha-domain + Phase 7's four), tiers recorded; one-pass < 10% and rider < 8% paired A/Bs; zero-alloc with 8 workers; partition invariance k ∈ {1,2,4,8}; banner verdict emission in the newest binaries (thresholds in, exit = FAIL count); scoreboard rows 12/17 restored, M-I/M-II as standing lines, **both completion rates printed** — consolidated and original denominators, permanently.

## PART 7 — THE GATE (numeric; raw output; exit = non-PASS count)

| # | Criterion | Threshold |
|---|---|---|
| 1 | Dispatch fix + tags | substrate ledger 10¹²–10¹⁵ engine-tagged; growth ∈ [3.9, 5.4] at every pair; 10¹⁵ ≡ Lehmer bit-exact |
| 2 | Differentials | 5 points [10¹⁵, 10¹⁶] through the **substrate** path, tagged, bit-exact |
| 3 | K1+K2 | paired A/B deltas; cy ≤ 75 (floor 95); residual bucket shrunk |
| 4 | Ladder | ≤ 45 floor / ≤ 35 target; signature ±25% per rung |
| 5 | Harness | kill-resume demo at 10¹³ bit-exact |
| 6 | **Marathon I** | π(10¹⁷) exact; ≤ 20 min sustained; ≥ 5 kills bit-exact; cert-record raw |
| 7 | **Marathon II** | π(10¹⁸) exact; ≤ 25 min post-K2 / ≤ 100 min fallback; ≥ 3 ckpt, ≥ 1 kill; differentials above 10¹⁶ |
| 8 | Mutants | 8/8 killed, tiers recorded |
| 9 | Overheads | one-pass < 10%; rider < 8%; zero-alloc; partition invariance |
| 10 | Banners + scoreboard | verdict emission live; rows 12/17; both completion rates |

## PART 8 — DECISION MAP

K2's residual delta → the K0 model's validity; the ladder's landing → the 10¹² crossover and Phase 21's priority order (sweep war vs product phase); Marathon I's telemetry → the sustained column for every remaining claim; the two cert-records → the capstones the finale ledger and CLI are built on; the growth fingerprint → the permanent identity-guard on every measurement the project ever prints again.

---

Run order: **the dispatch fix today — ten minutes, and the substrate's true ledger exists.** Harness wiring next, K1 in the same commit as the banner law, then **Marathon I the moment the harness demo is green — tenth phase owed, ~15–20 minutes of runtime, start it tonight.** K2 while it runs on the idle cores, Marathon II after. Paste back, in order: the engine-tagged substrate ledger with its growth column, the K1/K2 paired A/B outputs, and the Marathon I cert-record raw. The values were never in doubt — this phase is where the engine that produced them finally has its name on its own work.
