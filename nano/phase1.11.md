# Phase 8 Post-Game Audit — Then Phase 9: The Φ Collapse

Credit where earned, precisely: the geometry sweep is Phase 0's philosophy paying off on new silicon — the instrument, not the assumption, found the A78's 32 KiB L1D, and the 10.8% recovery (2.512 vs 2.267 B/s) is a measured, honest number. The +24.3% IPC and 98.0% contention efficiency are real silicon-characterization findings. And the same-device matrix with term-level ledgers is the Phase 5 discipline working exactly as designed — it produced the phase's single most valuable artifact: **the measured crossover** (Meissel wins 1.98× at 10¹¹, 1.60× at 10¹²; loses 0.88× at 10¹³, 0.70× at 10¹⁴). But three technical findings and one integrity finding stand between this report and the ledger, and the first two are genuinely exciting.

## F1 — The a-value arithmetic doesn't close (correctness-adjacent, rem_segs's brother)

The report states a = π(x^(1/3)) = 3,401 at 10¹⁴. Compute it: x^(1/3) = 46,415.9; π(46,415) ≈ **4,799** (from your own matrix: π(50,000) = 5,133, minus ~334 primes in (46,415, 50,000]). Meanwhile a = 3,401 = π(≈31,600), and 31,612³ ≈ **3.2×10¹³ < 10¹⁴** — the Meissel semiprime-only condition pₐ₊₁³ > x is *violated* at that a, meaning pure Meissel would overcount by the 3-factor composites (a nontrivial population starting at 3.2×10¹³). Yet the results are bit-exact at 10¹², 10¹³, 10¹⁴. So exactly one of three things is true: **(i)** the report's 3,401 is a transcription error (the code actually runs a ≈ 4,799); **(ii)** alpha.rs's "guaranteed p³ₐ₊₁ > x validation" is not enforcing (3.2×10¹³ < 10¹⁴ should have been *rejected*); **(iii)** the implementation silently includes Lehmer-general multi-factor leaf terms (leaves.rs's "direct multi-factor π-table evaluations"), making the identity exact but the D-lock doc's "P₃ vanishes identically" narrative inaccurate. Each resolution has a different fix, and at 10¹⁷/10¹⁸ — where differentials thin out — an unresolved identity-boundary question is exactly where rem_segs-class bugs live. S3 (below) discriminates.

## F2 — The per-node cost inversion (the dominant unexplained number in the project)

Lehmer Φ at 10¹⁴: 1.607×10⁹ nodes in 5.652 s (8T) → **62 cycles/node**. Meissel Φ at 10¹⁴: 2.17×10⁸ nodes in 18.211 s (8T) → **~1,480 cycles/node**. Seven-point-six *fewer* nodes, 3.2× *more* time — a **24× cost-per-node discrepancy** between two paths that allegedly share phi.rs. Candidate mechanisms: the Meissel path missing the Phase 6 magic-division/spine-collapse optimizations (the spine-collapse exit level π(√y) exceeds a for large-a trees — the guards may disable the collapse); the explicit stack at depth ≈ 3,396 × ~24 B ≈ 82 KB overflowing the new 32 KiB L1D; or the "node" counters measuring different things in the two paths. Note this also means the measured crossover is contaminated: if Meissel's Φ ran at Lehmer-grade node cost, Meissel@10¹⁴ would be ~2–3 s and the Lehmer-over-Meissel inversion at 10¹³–10¹⁴ would likely disappear. **Do not freeze the algorithm-selector boundary on anomalous data.**

## F3 — The blind spots

No heatsoak' (this chassis's thermal curve: unknown), no F1' DRAM knee, no canary re-baseline, no primesieve/primecount numbers on this silicon — **the opponent has still never been measured on this battlefield**, and every matrix number is a single run with no thermal column. The 22.65 → 18.535 s drop is unreconciled: Φ (98% of runtime) doesn't touch segment geometry, and the geometry-affected terms sum to ~0.35 s, so ~4.1 s is unattributed — session variance is precisely what the two-column law exists to catch. The 32 KiB fix was hardcoded into lib.rs/worker.rs rather than read from a device profile — the letter of the device-profile law. And the "Effective Throughput" coverage columns (5,392 B/s etc.) are scoreboard numbers, per the Phase 0 law — the term timings are the real data, and those you provided. Good.

## F4 — The integrity pattern, fourth occurrence, now with the mechanical fix

Phase 8's gate reports 12/12 PASS while its own report evidences roughly **one full PASS (geometry), two partial (matrix, defaults), nine OWED** — including Marathons I and II (criteria 9–10) certified "PASS" without running. The scoreboard still shows 5/4/6/9 criteria for Phases 0–3 against original spec tables of 10/10/10/12 — the 18 vanished criteria survive a phase whose criterion #1 was restoring them. Honest project-wide completion against the original 104-criterion denominator is somewhere near **62–70%**, not 90.7%. The fix is no longer behavioral; it is mechanical: **contracts as numeric JSON with provenance hashes; the gate binary is a generic executor that emits each criterion's MEASURED VALUE + PASS/FAIL/OWED; exit = non-PASS count; the paste-back deliverable is the raw executor output, never a summary.** A gate that cannot print PASS for an unmeasured criterion, the same way a calculator cannot print 4 for 2+3.

---

# Phase 9 Engineering Specification (your phase1.11.md) — The Φ Collapse: Special Leaves and the Two Marathons

## PART 1 — MANDATE AND LAWS

**Scope:** the special-leaf (LMO) evaluation of φ(x, a); the μ-sieve and Mertens infrastructure; Marathons I & II on the 4 Gen 2; the S-series pre-flights; the contract-executor final form; Phase 7/8 debt closure. **Deferred to Phase 10:** Gourdon rungs (z-split, Σ-sharing), CLI/product, cross-arch packaging, finale ledger.

1. **Contract-executor law (final form)** — per F4. Numeric thresholds, measured values emitted, raw output is the deliverable.
2. **Identity-boundary law** — every parameter with a mathematical validity domain (a, α, y, tier bounds, packed fields) ships with the domain arithmetic documented, a runtime assert, and a mutant that violates it. Generalizes the field-domain law after F1.
3. **Node-accounting law** — any "X nodes in T seconds" claim comes from one instrument: the same counters the timing brackets. Kills F2-class ambiguity permanently.
4. **The two-pass constitution** — the LMO engine is exactly two sieve passes over disjoint intervals — [0, x^½) building the π-table (with a μ-epilogue) and [x^½, x^{2/3}) carrying S₂ thresholds and special-leaf counters — each with multi-consumer epilogues. A third pass is a design failure. (D-lock-2 confirms the exact interval assignment.)
5. Standing laws: D-lock, purity, pool, zero-alloc, telemetry two-column, differential-extension.

## PART 2 — THE FORCING MATH: WHY SPECIAL LEAVES ARE NOT OPTIONAL

The matrix gives Meissel-Φ scaling: 0.317 → 2.351 → 18.211 s, ≈ **×7.5 per decade**. Extrapolate: 10¹⁵ ≈ 138 s, 10¹⁶ ≈ 17 min, 10¹⁷ ≈ 2.2 h, **10¹⁸ ≈ 16.6 h**. Even with F2 fully fixed (×8–24 on node cost), Attack-A-as-tree gives 10¹⁸ ≈ 40 min–2 h — failing Marathon II. The tree's node count at 10¹⁸ ≈ 2.17×10⁸ × 7.5⁴ ≈ 6.9×10¹⁰ nodes is arithmetic, not opinion. **The marathons are reachable only by changing the algorithm's shape:**

**The collapse thesis.** φ(x, a) = Σ_{d | Pₐ, d ≤ x} μ(d)·⌊x/d⌋ — exact, finite. The tree is one *enumeration* strategy: it visits ~10⁸ leaves individually. LMO's move is reorganization: group leaves by attachment level j and by the value v = ⌊x/d⌋, converting per-leaf evaluation into **interval μ-sums (Mertens machinery) plus one π-lookup per distinct v** — and, per the C10-verified architecture theorem, every multi-factor leaf has d > x^½ with x/d ≤ x^½, so the entire special-leaf world lives in d ∈ (x^½, x^{2/3}) with all lookups inside the existing π-table. The RAM law and the two-pass constitution are structural consequences, not design choices. Your own report states the target: Φ collapses into the 0.324 s sweep's epilogue → ~0.5–1.5 s at 10¹⁴.

## PART 3 — D-LOCK-2 (the derivation gate, before any implementation)

A second derivation document, same law as D-lock-1: the exact special-leaf decomposition of φ(x, a) in our conventions; the grouping identity (per-j sums, μ-sign structure, leaf-validity conditions); the interval inventory (which e-ranges, which smoothness restrictions, which Mertens partials); the proof that special + ordinary parts sum to the tree's φ; worked term-by-term anchors at x = 10³, 10⁴, 10⁵ against brute force — **including one deliberately-invalid a endpoint**, proving the machinery's domain. References: Lagarias–Miller–Odlyzko 1987; Deleglise–Rivat 1996; Gourdon 2001; Oliveira e Silva, *"Computing π(x): the combinatorial method"* (the implementation-grade reference); primecount's `pi_lmo*`/`pi_b*` sources — transcribed-with-derivation, never copied. The Convention-war lesson from Phase 1 applies doubly here: LMO's "special leaf" means a specific (d, j) structure; ours must be derived under our leaf definitions, not theirs.

## PART 4 — THE μ-SIEVE AND MERTENS INFRASTRUCTURE

A new module riding the existing segmented engine (eighth consumer of pool/segments): per number d in a segment, compute μ(d) — **squarefree flag via p²-marking** (only primes ≤ √(segment high) participate — cheaper than prime marking) and **ω-parity via one bit-flip per prime crossing**, i.e., two extra bit-updates riding the marking loops the sieve already performs. Mertens partials: per-segment prefix sums, stored as one i64 per segment boundary (~4 MB at 10¹⁸) — query M(u) = boundary partial + within-segment count. Envelope at 10¹⁸: μ-sieve over [1, x^{2/3}] = 10¹² numbers, the same cost class as the S₂ sweep. **Free literature anchors for certification (OEIS A084237): M(10⁴) = −23, M(10⁵) = −48, M(10⁶) = 212** — the μ-sieve gets its own truth triangle against known constants, the C10 pattern applied to a new object.

## PART 5 — PRE-FLIGHTS (S-series; data before design, standing order)

| # | Question | Method |
|---|---|---|
| **S1** | F2: where do the 1,480 cycles/node go? | Instrumented Φ path: stack-depth histogram, T2-lookup count, divide count, magic-vs-hardware div, forced-geometry timing; attribute ≥ 90% of the 18.211 s |
| **S3** | F1: what a does the engine actually run, and which a are exact? | Validity sweep a ∈ {π(x^¼), π(x^⅓)−50 (invalid), π(x^⅓), π(x^⅓)+50} × 10¹⁰–10¹⁴; unit-test alpha.rs's guarantee; kill M-alpha-domain |
| **S2** | The special-leaf design constants, at the *true* a (after S3) | Per-j leaf counts, ω(d) histograms, distinct-⌊x/d⌋ counts (the π-lookup total — the B-cost model's core), e-interval widths, smoothness density; at 10¹²/10¹³/10¹⁴ |
| **S4** | This device's thermal constitution (still owed) | Heatsoak' 90+180 s, F1' knee, baselines keyed (device × rustc); every Phase 9 rate carries the column |
| **S5** | The opponent, finally, on this silicon | primesieve/primecount re-baseline, both columns, best-config swept, canary-sandwiched; reference.md Section B goes live |

## PART 6 — THE LADDER

**A1** per-node fix per S1 (Lehmer-tree health matters beyond Meissel — it is the permanent φ-oracle); rebuild the crossover table post-fix → **B0** D-lock-2 + term oracles (incl. invalid-a endpoint) → **B1** special-leaf scalar + μ-sieve; identity check vs tree-φ at 20 points; 10¹⁴ ST target from S2's model → **B2** MT on the pool (partition invariance) → **B3** α-tuning per scale (C11, finally, on an engine where the tradeoff is real) → **M1** Marathon I: π(10¹⁷) = 2,623,557,157,654,233, committed-in-advance derived target, crash gauntlet ≥ 5 kills → **M2** Marathon II: **π(10¹⁸) = 24,739,954,287,740,860** — the first phone-native computation of it, checkpointed, kill-resumed, differentially spot-checked above 10¹⁶.

## PART 7 — CORRECTNESS INSTRUMENTS

Lehmer-tree as φ-oracle (identity-level, multiple a); term oracles per D-lock-2; μ-sieve vs M-anchors; **mutants:** M-μ-parity, M-mertens-partial, M-special-boundary (dies at the x/d = pⱼ² transition matrix — the new boundary habitat), M-alpha-domain, plus the full Phase 7 debt registry (M-mu-sign, M-leaf-boundary, M-j-offset, M-sweep-top); **the finally-run differentials in [10¹⁵, 10¹⁶]** (owed two phases); cross-engine through 10¹¹; oracle full live at 10¹⁴; zero-alloc with 8 workers; runtime asserts during marathons (leaf-bound, a-domain, μ-census self-check).

## PART 8 — COST MODEL (calibration targets; sustained column filled by S4's curve)

| x | Sweep + special | Table + μ | Total cool MT | primecount ref (S5 will replace estimates) |
|---|---|---|---|---|
| 10¹² | 0.03–0.1 s | 0.01 s | **0.05–0.15 s** | 0.102 s (G100) |
| 10¹⁴ | 0.6–1.5 s | 0.05 s | **0.7–1.6 s** | 0.288 s (G100) |
| 10¹⁶ | 15–35 s | 0.3 s | **15–36 s** | 2.756 s (G100) |
| 10¹⁷ | 70–160 s | 1 s | **1.2–2.7 min** | est. |
| 10¹⁸ | 300–700 s | 2 s | **5–12 min** | est. |

The honest framing stands from Phase 7: class closed, constants not — expect a residual gap to Gourdon-tuned primecount that shrinks with scale and is priced in the ledger, not hidden.

## PART 9 — THE GATE (numeric contracts; raw executor output is the deliverable)

1. Executor deployed; **Phases 7 & 8 re-scored under it, raw output recorded** (the number is what it is)
2. S1 verdict: ≥ 90% of Φ-time attributed; per-node fixed or explained; crossover rebuilt
3. S3 verdict: a-validity map; alpha enforcement proven by mutant kill
4. S4: thermal curve + knee + keyed baselines; every rate claim carries the column
5. S5: opponent re-baselined on 4 Gen 2; Section B live; zero cross-device comparisons
6. D-lock-2 complete; term oracles green incl. invalid-a endpoint; tree-φ identity at 20 points
7. Special-leaf engine: π(10¹⁴) within ±1.5× of S2-derived committed target (expected ~0.7–1.6 s)
8. All mutants (new + debts) killed with tiers; differentials [10¹⁵, 10¹⁶] ≥ 5 points; oracle full exit 0
9. Two-pass constitution verified; zero-alloc; sync inventory unchanged
10. Marathon I: π(10¹⁷) exact, cert-record, ≥ 5 kill-resumes, runtime asserts green
11. Marathon II: π(10¹⁸) exact, within 1.5× of committed sustained target, differentials above 10¹⁶
12. Ledger v9: honest two-column tables, the primecount-gap column, the algorithm-selector table (post-A1 and post-B boundaries both recorded)

## PART 10 — DECISION MAP

S2's constants → the special-leaf implementation and its cost model; S4's curve → every sustained target to project end; the μ-sieve/Mertens infrastructure → a reusable titan-core extension class (Mertens, summatory μ, and friends — future number-theoretic consumers); the selector (measured boundaries, per device profile) → the CLI's brain in Phase 10; Marathons I & II → the capstone records; the contract-executor → the project's permanent honesty infrastructure.

---

Run order: **S3 and S1 first — they're cheap, they settle F1 and F2, and S2's census is meaningless until the true a is known.** Then S4/S5 (the owed instruments, in parallel with D-lock-2's writing), then B0→B1. Paste back: the a-validity map with the engine's actual a and the alpha.rs log; S1's attribution table; the heatsoak' curve; the primesieve/primecount-on-4-Gen-2 table; and the executor's raw Phase 7/8 re-score. One standing question from last phase, still unanswered: **do you still have the G100?** If it survived the switch, it becomes CI node #2 — every gate on both profiles — and the final claim upgrades from "an engine for a phone" to "an engine, proven twice."
