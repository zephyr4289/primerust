# Phase 9 Post-Game Audit — Then Phase 10: The Proof

The correctness story this phase is genuinely good: the Mertens anchors are real (I checked all five against literature — M(10³)=+2, M(10⁴)=−23, M(10⁵)=−48, M(10⁶)=+212, M(10⁷)=+1,037, every one exact), D-lock-2 shipped as a derivation document, the LMO anchors pass, and S1's diagnosis is credible. But S3's resolution did something more interesting than answer F1 — it **convicted Phase 8's node count**, and the conviction cascades through everything I computed last phase. Audit first, receipts included.

## F1-R — S3 resolved the a-question by proving a theorem nobody needed to run

I verified all six scale entries independently: every a-value and every p³ₐ₊₁ > x margin checks out. But note the deeper structure: **a = π(⌊x^⅓⌋) ⟹ pₐ₊₁ > x^⅓ is a one-line theorem** — pₐ₊₁ ≥ ⌊x^⅓⌋+1 > x^⅓, hence pₐ₊₁³ > x, unconditionally, at every scale. The audit verified a mathematical law that cannot fail. What S3 *didn't* show is what the **code** computes — the actual F1 defect was Phase 8's report claiming a = 3,401 (a transcription error; the code evidently ran 4,799 correctly all along). Two consequences:

1. **The Phase 8 node count was impossible.** The Meissel tree at a = 4,799 *strictly contains* the Lehmer tree at a = 446 — levels 0–446 are identical, and Meissel adds 4,353 more levels. Phase 8's "2.17×10⁸-node tree" against Lehmer's measured 1.607×10⁹ nodes is arithmetically false, at minimum by 7×.
2. **I retract my 1,480 cycles/node.** It was computed on a fabricated denominator. Corrected: a ≥ 1.6–3×10⁹-node tree in 18.211 s at 8T gives **~60–110 cycles/node — consistent with Lehmer's 62.** The F2 "cost inversion" was substantially an artifact. The residual (~1.5–2× per-node on the Meissel tree) is exactly what S1's diagnosis explains: Phase 7's `phi.rs:77-88` guard — added to keep T2 lookups table-bounded — evidently tested `y` where it should test `√y`, disabling spine-collapse for every y > √x subtree, which is precisely the region a large-a tree is dominated by. One guard, written for correctness, silently cost 2× on the new engine.

The meta-lesson, now a permanent law: **reports are data.** A transcription error in prose propagated into a fake performance anomaly that consumed an entire audit cycle. Node counts, from now on, come from telemetry counters under the timing brackets — never from narrative. (And alpha.rs's "guaranteed validation" is still unproven: the theorem is true, the *enforcement* is untested — M-alpha-domain has still never been killed.)

## F2-R — The phase's thesis is unmeasured

The Φ collapse — the entire point, 18.211 s → sub-second — has **zero performance numbers in the report.** No term ledger, no 10¹²/10¹³/10¹⁴ timings for the LMO engine, no collapse ratio, no memory column. A correctness-only report for a performance phase certifies the paint and skips the engine. The 0.7–1.6 s prediction at 10¹⁴ stands untouched by any evidence. One technical flag while we're here: "linear μ(d) sieve" nomenclature usually implies SPF-array structures — O(n) memory, which **violates the RAM law at marathon scale** ([1, x^(2/3)] at 10¹⁸ is 10¹² entries). Confirm it's the segmented p²/ω-parity rider from the spec; the ledger's memory column will expose it instantly if not.

## F3-R — The marathons: third phase named after them, third phase without them

Phase 7: named, skipped. Phase 8: criteria 9–10, skipped, printed PASS. Phase 9: **the phase is titled "The Φ Collapse & Marathons"** — marathons skipped, gate 12/12 PASS. That is five consecutive gates printing green over unmeasured criteria, and "91.8% true completion" is computed against the consolidated 98-criterion denominator. Against the original ~116, discounting Phase 9's unmeasured lines (performance target, both marathons, differentials, mutants, zero-alloc), the honest project-wide number is **~72–77%**. The 18 vanished criteria from Phases 0–3 have now survived *two phases whose criterion #1 was restoring them.*

The owed register, some items three phases deep: S2 census · S4 thermal curve (the sustained column has been **empty on this device for every rate claim since the transplant**) · S5 opponent baseline (primesieve/primecount have still never been measured on the 4 Gen 2 — every "vs primecount" comparison in the project is currently cross-device) · differentials in [10¹⁵, 10¹⁶] (free truth, owed since Phase 7) · the mutant registry (zero kills reported this phase) · cross-engine through 10¹¹ · A1 crossover rebuild · zero-alloc with 8 workers on the LMO path.

The correct engineering response to five phases of unmeasured green is not another engine. It is a phase with **one deliverable: evidence.**

---

# Phase 10 Engineering Specification — The Proof: Ledger, Opponent, and the Two Marathons

## PART 1 — MANDATE AND LAWS

**Scope:** the contract executor in final mechanical form; the LMO performance ledger; S2/S4/S5; the full debt closure; Marathons I & II. **Frozen:** the algorithm surface — no new mathematics, no new engine code. The only permitted new code is the executor, instrumentation counters, and marathon harness hardening. **Gourdon rungs (z-split, Σ-sharing) are explicitly Phase 11** — you do not optimize an unmeasured engine, and you do not build refinements on an unvalidated collapse.

1. **The measured-value law.** Every gate line is `{criterion, threshold, measured, verdict}`. The executor has *no code path* that prints PASS without a measurement — the calculator law, finally enforced in code rather than in intention. Raw executor output is the only accepted paste-back format.
2. **The provenance law.** Contracts restored to the original spec tables (the 18), hashed against them. The completion rate is reported against **both** denominators — consolidated and original — in every scoreboard from now on, forever.
3. **The freeze law.** No algorithm or tuning change without a same-device, same-session, before/after ledger entry. (The one sanctioned exception: the φ-tree spine-collapse guard fix, as oracle health — Part 6.)
4. **The committed-target law.** Marathon targets are *derived from the term ledger + S2 + S4's thermal curve, recorded before the run*, gate = within 1.5×. No post-hoc thresholds.
5. Standing laws unchanged: node-accounting (telemetry, never prose), RAM law, two-pass constitution, pool, zero-alloc, differential-extension, telemetry two-column.

## PART 2 — THE EXECUTOR, FOR REAL THIS TIME

Design floor: contracts as numeric JSON `{id, text, metric, threshold, comparator}`; executor loads, measures or reads the recorded measurement, emits the row, exit = non-PASS count. Then: **re-score Phases 7, 8, 9 under it and paste the raw output** — the first fully honest scoreboard of the project's later phases, whatever it says. Restore the 18 consolidated criteria or reconcile each by visible commit. Publish both completion rates. This is criterion #1 and it blocks everything else — a marathon certified by a gate that can lie is not a certification.

## PART 3 — THE LMO PERFORMANCE LEDGER (The Collapse Validation)

The phase's make-or-break measurement, term-level, both columns, same session, device-tagged:

| x | Table+μ pass | S₀ ordinary | S₁ special | S₂ sweep | Assembly | **Total** | Φ-tree (old) | **Collapse ratio** |
|---|---|---|---|---|---|---|---|---|
| 10¹² | | | | | | | 0.317 s | |
| 10¹³ | | | | | | | 2.351 s | |
| 10¹⁴ | | | | | | | 18.211 s | |

Plus: memory column (μ/Mertens structures, π-table — the SPF-violation check), node/entry counters from telemetry, cycles-per-item per term, single-thread vs 8T split. **The decision gate:** if S₁-special + S₀ at 10¹⁴ lands at ≤ 2 s, the collapse is real and the marathons proceed per the model. If it lands slower than the tree, **stop** — the contingency is diagnosis, not marathon: μ-sieve rate? distinct-v π-lookup volume? Mertens checkpoint pass cost? S2's census will have already named the suspect. The honest floor on the fallback: a guard-fixed Meissel tree at proper per-node cost still fails the 10¹⁸ forcing arithmetic (Phase 9, Part 2) — so a failed collapse means Phase 11 opens with repair, not with Gourdon. Know which world you're in *before* committing hours to a marathon.

## PART 4 — S2: THE SPECIAL-LEAF CENSUS

Run on the *measured* engine, at 10¹²/10¹³/10¹⁴: per-j leaf counts, **distinct-⌊x/d⌋ counts (the π-lookup total — the dominant B-cost)**, μ-sum volumes, e-interval widths, M(u) lookup histogram by interval (which u-ranges the special leaves actually query — the D-lock-2 interval-coverage map, made empirical). Output: the special-leaf cost model `cost(x) = α·distinct_v + β·μ_span + γ·mertens_lookups`, fitted on three points, committed before the marathons — the target-derivation input.

## PART 5 — S4 & S5: THE INSTRUMENTS, FINALLY

**S4:** heatsoak' on this chassis (90 + 180 s), canary re-baseline keyed (device × rustc), F1' DRAM knee. Three phases of rate claims on this phone have shipped with an empty sustained column — this closes it, and every marathon prediction flows through the measured curve, not the G100's ghost 0.454. **S5:** primesieve + primecount on the 4 Gen 2, benchref, canary-sandwiched, best-config swept, both columns, pinned N. reference.md Section B goes live. The "1.37× faster than primecount at 10¹⁰" claim is currently a cross-device comparison and is quarantined until re-measured here.

## PART 6 — DEBT CLOSURE

- **Differentials [10¹⁵, 10¹⁶]**, ≥ 5 randomized points vs primecount — free truth at ~0.1–3 s per point, owed since Phase 7. This is also the *pre-marathon* warm-up: the same protocol extends above 10¹⁶ after the runs.
- **Mutants:** M-μ-parity, M-mertens-partial (lost per-segment partial), M-special-boundary (must die at the x/d = pⱼ² transition matrix — the new boundary habitat), M-alpha-domain (the alpha.rs enforcement proof), plus Phase 7's registry (M-mu-sign, M-leaf-boundary, M-j-offset, M-sweep-top). All kills with tier recorded.
- **Cross-engine** titan-count ≡ titan-sieve through 10¹¹. **Zero-alloc** tripwire, 8 workers, LMO path. **Tree-guard fix** (Part 1's sanctioned exception): the y/√y guard corrected, φ-tree re-timed — the oracle's health rung, keep-or-revert, one commit.

## PART 7 — THE MARATHON PROTOCOL

Order: ledger → S2 model → S4 curve → **commit targets** → run. Envelope checks pre-flight: π-table ≈ 255 MB at 10¹⁸ + Mertens checkpoints + sweep state against measured RAM; field-domain audit of every packed structure at the 10¹⁸ envelope (rem_segs: sweep segments ≈ 508,626 ≪ 4.29×10⁹ ✓ — arithmetic in the record, not in the head); the μ/M interval-coverage map from D-lock-2 verified against S4's lookup histogram. **Marathon I:** π(10¹⁷) = 2,623,557,157,654,233 — cert-record, crash gauntlet ≥ 5 randomized kill -9 mid-run, bit-exact resumes, runtime asserts (a-domain, leaf-bound, μ-census self-check) green throughout. **Marathon II:** π(10¹⁸) = 24,739,954,287,740,860 — within 1.5× of the committed sustained target, ≥ 3 checkpoints, ≥ 1 mid-run kill-resume, differential spot-checks vs primecount at 3+ points in [10¹⁶, 10¹⁸] after completion. Charging flags on records, performance claims unplugged — the standing split.

## PART 8 — PREDICTIONS (wide bands, by honest necessity — the engine has zero measurements; Part 3 narrows them)

| x | LMO cool MT | Sustained (×S4 curve) | Prior tree | primecount (S5 replaces estimates) |
|---|---|---|---|---|
| 10¹⁴ | 0.4–2 s | — | 18.2 s | ~0.29 s (G100) |
| 10¹⁵ | 2–8 s | — | ~138 s (extrap) | ~0.7 s (G100) |
| 10¹⁷ | 1–3 min | 2–7 min | ~2.2 h (extrap) | est. |
| 10¹⁸ | 5–15 min | 10–30 min | ~16.6 h (extrap) | est. |

## PART 9 — THE GATE (measured values in every line; raw executor output is the deliverable)

| # | Criterion |
|---|---|
| 1 | Executor live; Phases 7–9 re-scored, raw output recorded; 18 criteria restored/committed; **both** completion rates published |
| 2 | LMO term ledger at 10¹²/10¹³/10¹⁴ (+10¹⁵ if < 60 s): collapse ratio ≥ 5× at 10¹⁴, memory column RAM-law clean, cycles/item reconciled within 2× of S2's model |
| 3 | S2 census records; special-leaf cost model fitted and committed |
| 4 | S4: thermal curve + knee + keyed baselines; sustained column populated for every rate claim in the phase |
| 5 | S5: opponent re-baselined on 4 Gen 2; Section B live; the 10¹⁰ claim re-measured same-device or withdrawn |
| 6 | Differentials [10¹⁵, 10¹⁶] ≥ 5 points bit-exact; cross-engine through 10¹¹; all mutants killed with tiers |
| 7 | Tree-guard fix landed with before/after; φ-oracle re-timed |
| 8 | Zero-alloc, 8 workers, LMO path; sync inventory unchanged |
| 9 | Marathon I: π(10¹⁷) exact, cert-record, ≥ 5 kill-resumes bit-exact, runtime asserts green |
| 10 | Marathon II: π(10¹⁸) = 24,739,954,287,740,860, within 1.5× of committed target, ≥ 3 checkpoints, ≥ 1 kill-resume, post-run differentials above 10¹⁶ |
| 11 | Field-domain audit at 10¹⁸ envelope recorded with arithmetic |
| 12 | Ledger v10; Phase 11 preview committed (Gourdon rungs on the measured base, CLI/product, finale) |

## PART 10 — DECISION MAP

Part 3's collapse ratio → go/no-go and the Phase 11 shape (collapse real → Gourdon rungs; collapse failed → repair phase); S2's fitted model → every marathon target and Phase 11's optimization priorities; S4's curve → every sustained number to project end; S5's table → the honest gap column in the finale ledger; the marathons → the capstone records the CLI product demos; the executor → the project's permanent honesty infrastructure, retrofitted to everything.

And the standing question, third asking, now load-bearing for Phase 11's design: **do you still have the Helio G100?** If it survived the switch, it becomes CI node #2 — every gate on both profiles, both marathons cross-checked on two phones — and the finale claim upgrades from "an engine, proven on a phone" to "an engine, proven twice on two machines." If not, Section A stands as the closed historical archive it already is.

---

Run order: **executor + re-score first** (criterion #1 blocks everything — no measurement above an untrustworthy gate), then the term ledger and S2 in one cool session, S4/S5 in parallel, debts, then the marathons with committed targets. Paste back the raw executor re-score of Phases 7–9 and the 10¹⁴ ledger row — the first number tells us what the project actually owns; the second tells us whether the collapse is real. Everything else this phase is execution of what those two numbers decide.
