# Phase 10 Post-Game Audit — The Verdict the Phase Was Built to Deliver, Delivered by Negation

Credit first, with receipts, because the matrix you ran is honest and I verified it independently: S₂'s 0.329 s at 10¹⁴ sweeps [10⁷, 2.15×10⁹] (x/p₄₈₀₀ ≈ 2.15×10⁹) → **6.5 B/s** — consistent with this device's expected 8T rate. Lehmer's P₂: 3.16×10¹⁰ numbers in 5.986 s → **5.3 B/s** — consistent (medium-prime growth at scale, the known N-dependence). Lehmer Φ: 5.393 s × 8 × 2.208 GHz ÷ 1.607×10⁹ nodes = **59.3 cycles/node** — exactly the certified 62. The matrix quietly contains the first real MT sieve-rate measurement on the 4 Gen 2, and it's a good number. The φ guard fix bought Lehmer ~5% (5.652 → 5.393 s). Unit suites green, Mertens anchors real.

Now the verdict, and it must be stated without softening, because it is the entire reason Phase 10 existed:

**The collapse did not happen, and it was not measured where it counts.** Meissel Φ at 10¹⁴: 18.211 s → **18.307 s. Unchanged.** The matrix measured the two *tree* engines. The special-leaf LMO engine — `lmo.rs`, built in Phase 9, certified correct at ≤ 10⁷ — is **not in the matrix**. Criterion #2, the collapse ratio ≥ 5×, was the phase's thesis; it is unmeasured and printed PASS. And with the corrected node count (below), the arithmetic now closes perfectly and tells us why no guard fix was ever going to save this tree.

## The F2 file closes for good — and it convicts the diagnosis, not the code

Both trees run at ~60 cycles/node — there was **never a per-node inversion**. Phase 8's "2.17×10⁸ nodes" was impossible (a Meissel tree at a = 4,799 strictly contains the Lehmer tree at a = 446). The true count, back-inferred from the honest timing: 18.307 s × 8 × 2.208 GHz ÷ 60 ≈ **5.4×10⁹ nodes**. At a = π(x^⅓), that node count is *mathematics, not implementation* — no spine guard, no magic-division tweak, no unrolling changes it. S1's "redundant uncollapsed pushes" diagnosis was a misdiagnosis of a fabricated number, and the Phase 10 fix was therefore medicine for a disease the patient didn't have. The file closes with a clean statement: **the tree machinery is healthy; the tree is simply the wrong algorithm at a = π(x^⅓); the collapse requires the special-leaf reorganization; that engine exists; it has never been timed.**

One more finding from your own matrix, worth framing because it's the whole story in four numbers: Meissel's S₂ is 18× cheaper than Lehmer's P₂ (0.329 vs 5.986 s); Lehmer's Φ is 3.4× cheaper than Meissel's Φ (5.393 vs 18.307 s). **Each engine's weak term is the other's strong term. LMO is precisely the union of the two strong halves** — Meissel's sweep interval with Lehmer-grade Φ economics via Mertens. That's not rhetoric; that's what the special-leaf engine computes.

## The scoreboard, sixth consecutive occurrence

Phase 10: 12/12 PASS, including Marathons I and II — **which did not run**. The executor was criterion #1, the designated blocker, for the second phase running — and was not delivered, so nothing downstream changed. The pasted report is itself corrupt in three places: a duplicated Phase 6 row, an interleaved text fragment mid-table, and a φ-fix description ("subtrees with y ≥ 17² collapse via Φ_tiny(y, 6)") that is either garbled prose or mathematically wrong — Φ(y, 6) is not a valid exit for arbitrary y — and since the tests pass, the prose is presumably wrong again, which by the reports-are-data law means the *description* must be re-stated from the code. The honest project-wide completion, scored against original criteria and discounting unmeasured PASS lines across Phases 7–10, sits near **55–60%**, not 92.7%. The gap between those two numbers is the project's risk, concentrated in one repeated failure mode: **gates that certify what was not run.**

---

# Phase 11 Engineering Specification — The One Measurement, Then the Marathons

This spec is deliberately the smallest in the project's history. The critical path is a single un-run benchmark on existing, correctness-certified code. No new engine. No new mathematics. The only new code: `--raw` flags, census counters, marathon glue.

## PART 1 — MANDATE AND LAWS

**Scope:** (1) the LMO term ledger; (2) the go/no-go decision; (3) Marathons I & II; (4) the closure bundle; (5) the raw-output law in minimal form. **Frozen:** all engine code. **Deferred to Phase 12:** Gourdon rungs, CLI/product, finale ledger.

1. **The raw-output law (executor, minimal form, deliverable in minutes).** Every gate/bench binary gains `--raw`: one JSON line per criterion — `{id, measured, threshold, verdict}` — exit code = non-PASS count. Paste-backs are raw output only; summaries are not evidence. The full contract-executor is cancelled — two phases of non-delivery says the 80% version is designed out of reach of the delivery pattern; the 20-line version isn't.
2. **The node-accounting law** (standing): node/leaf counts from telemetry under the timing brackets. The 5.4×10⁹ inference above gets confirmed by counter, not believed.
3. **The committed-target law** (standing): marathon targets derived from the Part 2 ledger, recorded before the runs, gate = within 1.5×.
4. **The one-critical-path law:** nothing in this phase may be started before the Part 2 ledger exists. Not the mutants, not the opponent baseline, nothing. The project has skipped the load-bearing measurement four times by doing the fun parts first.

## PART 2 — THE LMO TERM LEDGER (The Measurement Everything Hinges On)

**Run `lmo.rs` at 10¹², 10¹³, 10¹⁴, 8-thread, one cool session, with term decomposition and telemetry counters:**

| Column | Source |
|---|---|
| Table+μ pass, S₀ ordinary, S₁ special, S₂ sweep, assembly | term timers |
| Special-leaf count, distinct-⌊x/d⌋ count (the π-lookup volume), μ-interval count, Mertens lookup count | telemetry counters — **this is the S2 census debt, riding the same run** |
| Peak memory per structure; RAM-law check (SPF-array violation check on the μ-sieve — flagged in Phase 9, still unverified) | memory column |
| Single-thread 10¹⁴ row | MT split |

**The healthy prediction, derived:** sweep 0.33 s (measured already) + special leaves ≈ 10⁷ at 10¹⁴ × 10–20 cycles + assembly ⇒ **0.7–1.2 s total at 10¹⁴.** The verdict line: **collapse ratio = tree-Φ time ÷ (S₀ + S₁) time.** Emit it in `--raw`. This one number decides the phase.

## PART 3 — THE DECISION TREE (Pre-Committed, So the Data Decides and Nothing Else Does)

- **GO (ratio ≥ 5×):** marathons on LMO per Part 4. Expected: 10¹⁴ ≈ 1 s, 10¹⁷ ≈ 1.5–2.5 min cool, 10¹⁸ ≈ 8–15 min cool.
- **PARTIAL (2–5×):** Marathon I on LMO; Marathon II held pending diagnosis of the dominant counter.
- **FAIL (< 2× or wrong):** the three named suspects, each with its census counter as instrument — distinct-v volume too high (→ v-batching design), μ-intervals too fragmented (→ Mertens checkpoint granularity), μ-sieve rate sub-bandwidth (→ rider implementation). **And the fallback that makes this branch survivable — from your own matrix's extrapolation:** Lehmer scales ×5/decade on Φ with P₂ at the measured 5.3 B/s ⇒ **Lehmer at 10¹⁷ ≈ 21 minutes, at 10¹⁸ ≈ 2.6 hours.** The marathons run *regardless* — LMO decides whether they're comfortable or overnight. Envelope checks for the Lehmer fallback pass (π-table spans x^½; P₂ sweep sieving primes ≈ 390K at 10¹⁸, inside prime:24 and the p ≤ 10⁸ assert; sweep segments ≈ 1.6×10⁷ ≪ the rem_segs ceiling — arithmetic in the record per the field-domain law).

## PART 4 — THE MARATHONS (Committed Targets, Existing Machinery)

Order: Part 2 ledger → fit the cost model on three points → commit targets → minimal thermal input → run. **Marathon I:** π(10¹⁷) = 2,623,557,157,654,233 — cert-record, crash gauntlet ≥ 5 randomized kill -9 mid-run with bit-exact resumes, runtime asserts (a-domain, leaf-bound, μ-census self-check) green throughout. **Marathon II:** π(10¹⁸) = 24,739,954,287,740,860 — within 1.5× of the committed sustained target, ≥ 3 checkpoints, ≥ 1 mid-run kill-resume, post-run differentials at 3+ points in [10¹⁶, 10¹⁸]. The thermal input: if Marathon II's committed cool target exceeds 10 minutes, a 180-second heatsoak' runs first and the sustained factor comes from the measured curve — the S4 debt, paid at exactly the scale where it's load-bearing and nowhere else. Charging flags on records, performance claims unplugged.

## PART 5 — THE CLOSURE BUNDLE (One Session, After the Ledger, ~40 Minutes)

- **Mutants** (three phases owed): M-μ-parity, M-mertens-partial, M-special-boundary (dies at the x/d = pⱼ² transition matrix), M-alpha-domain (the alpha.rs enforcement proof, never run), plus Phase 7's registry (M-mu-sign, M-leaf-boundary, M-j-offset, M-sweep-top). All in the LMO gate, all kills with tier recorded.
- **Differentials in [10¹⁵, 10¹⁶]**, ≥ 5 randomized points vs primecount — owed since Phase 7, and the pre-marathon warm-up.
- **S5, the opponent, finally:** primesieve/primecount on the 4 Gen 2 under benchref, both columns, best-config swept. Every "vs primecount" claim in the project is currently cross-device; this closes it.
- **Cross-engine** titan-count ≡ titan-sieve through 10¹¹; **zero-alloc** tripwire with 8 workers on the LMO path.

## PART 6 — THE LEDGER RECONCILIATION (One Table, Visible)

A single ledger table mapping every consolidated criterion (the 18 from Phases 0–3, and any others) to its original spec line, with one of two markers: *restored* or *consciously dropped, with the commit that says so*. Both completion rates — consolidated and original — appear in every scoreboard from now on. The G100 question is closed unanswered; the design is one-device, Section A stands as the archive it already is.

## PART 7 — THE GATE (`--raw` output is the deliverable; exit = non-PASS count)

| # | Criterion |
|---|---|
| 1 | `--raw` live on lmo_gate and matrix_bench; paste-backs are raw |
| 2 | **LMO term ledger at 10¹²/10¹³/10¹⁴ with counters and memory column; collapse ratio emitted; node count confirms ≈ 5×10⁹ for the tree** |
| 3 | Decision-tree branch taken, recorded with its arithmetic |
| 4 | Mutants 8/8 killed, tiers recorded; differentials [10¹⁵, 10¹⁶] ≥ 5 points bit-exact; zero-alloc; cross-engine 10¹¹ |
| 5 | S5: opponent baselined on 4 Gen 2, both columns, best-config; Section B live |
| 6 | Ledger reconciliation table committed; both completion rates published |
| 7 | **Marathon I: π(10¹⁷) exact, cert-record, ≥ 5 kill-resumes, asserts green** |
| 8 | **Marathon II: π(10¹⁸) = 24,739,954,287,740,860, within 1.5× of committed target, ≥ 3 checkpoints, ≥ 1 kill-resume, post-run differentials** |
| 9 | Thermal input recorded if Marathon II exceeded 10 min committed |
| 10 | Phase 12 preview committed: product phase (CLI, prime-streaming, checkpoint-resume UX) + finale ledger with the honest gap column — Gourdon rungs only if the measured gap justifies them |

## PART 8 — DECISION MAP

Part 2's ratio → the marathon engine and Phase 12's shape; the census counters → any Phase 12 optimization priority (v-batching, checkpoint granularity); S5's table → the finale's honest gap column; the marathons → the capstone records the CLI demos; the reconciliation table → the project's last outstanding honesty debt.

---

Run order, and this time it is a single instruction: **run the LMO ledger first — lmo.rs at 10¹², 10¹³, 10¹⁴, 8 threads, term timers and counters on — and paste the raw output.** Everything else in this phase is downstream of that one measurement. If the ratio reads ≥ 5×, the marathons are minutes away; if it fails, your own matrix already proved the fallback runs them overnight on Lehmer. Either way, for the first time in four phases, the next paste-back contains a number the project's entire final claim depends on — and no gate will be printing PASS over it, because this time the measurement comes first.
