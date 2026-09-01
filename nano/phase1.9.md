# Phase 6 Post-Game Audit — Then Phase 7: The Algorithm War

The phase delivered its crown — π(10¹⁶) bit-exact, crash-proof, phone-native — and the forensic story inside it is the best thing that happened all phase, because it *teaches a law*. Three findings first, then the war.

## Finding 1 — The retro-audit needs an audit of its own

Your scoreboard reads Phase 0: 5 criteria, Phase 1: 4, Phase 2: 6, Phase 3: 9, Phase 4: 14 (6+8), Phase 5: 12, Phase 6: 12. The original spec tables read 10 / 10 / 10 / 12 / 12 / 12 / 12. Phases 5–6 match exactly. **Phases 0–4 do not** — the contract files were evidently written at retrofit time to summarize what was measured, not transcribed from the specs. That is the quiet-reframe pattern surviving one level up: the gate-contract law's entire value was *diffability against the original tables*. Fix is mechanical, not accusatory: re-diff every contract file against its phase spec; every consolidation is either a visible commit with a one-line justification or gets restored; re-score. The law is visibility, not the count.

## Finding 2 — The rem_segs forensics: the system worked, now extract the law

A 14-bit field wrapped at 16,383 segments, silently misplacing bucket primes above 3.22 × 10¹⁰, producing a **+0.5% overcount** — caught only by the truth triangle at 10¹⁶. Study the geography: the bug was invisible to the cross-engine differential ([10⁶, 10¹⁰] — P₂ sweeps there top at 10⁷·⁵, far below the wrap), invisible to every forced-geometry test, invisible to T3 up to 10¹¹. It lived *exactly* above the instruments' reach — the M4 geometry, but real. Two permanent laws fall out:

> **Field-domain law:** every packed bitfield ships with (a) a documented maximum domain *with the arithmetic that proves it*, (b) a debug-build range assert at every write, (c) at least one differential test point that exercises the field's ceiling. The Phase 1 "guarded powers" instinct was right; this was the one packed structure that escaped it.

> **Differential-extension law:** the randomized-differential range vs primecount extends with the certified domain — primecount answers any x in ~0.1–3 s, so [10¹⁵, 10¹⁶] points are *free truth* and would have caught rem_segs without knowing any constant. Differential ranges are domain-scoped config, never constants.

Retrofit both into Phase 4's entry packing (audit every field: at 10¹⁸, sweep segments ≈ 5 × 10⁵ ✓, sieving primes ≤ 10⁶ ✓ within prime:24 — the audit is cheap, skipping it is how rem_segs happened).

## Finding 3 — Housekeeping

Section 1's marathon table didn't survive your paste — the term-level timings for 10¹⁵/10¹⁶ exist somewhere (the gate passed 12/12 against ≤ 150 s / ≤ 12 min criteria) but I can't audit what I can't read; paste them into the ledger properly. C7's verdicts land exactly on the spec's kill-lines: BFS frontier 3.3 GB against the ~800 MB ceiling (rejected, correctly), spine-split with 0.29% top subtree (work-stealing viable — and note the irony: the tree being *violently* left-heavy is what makes spine-split *and* nothing else work). The 25.47 MiB table at 10¹⁶ extrapolates to ~255 MiB at 10¹⁸ — fits, and Phase 7's memory plan accounts for it.

---

# Phase 7 Engineering Specification — `titan-count` v2: LMO / Gourdon-Class π(x)

Phase 6 won the engineering war (terminal Lehmer, 12 min at 10¹⁶, crash-proof). Phase 7 fights the **algorithm** war: replace the O(x^(3/4)) Lehmer wedge with the O(x^(2/3)) combinatorial class — Lagarias–Miller–Odlyzko structure, then Deleglise-Rivat/Gourdon refinements. The honest scoreboard we're attacking: primecount's Gourdon does 10¹⁴ in 0.288 s, 10¹⁶ in 2.756 s. We will not beat a two-decade-tuned reference engine's constants in one phase. **The mission: close the algorithm class (12 min → ~30 s at 10¹⁶, a 20–40× structural gain), certify it under the full instrument stack, and take the badge no engine has ever taken: exact π(10¹⁸) computed on a phone.**

---

## PART 1 — MANDATE AND LAWS

**Scope:** LMO identity implementation (`meissel.rs`, `leaves.rs`, `special_leaves.rs`, `alpha.rs`), the shared-sweep extension of P₂ machinery, 10¹⁷/10¹⁸ marathons, retro-audit re-diff, field-domain retrofit. **Deferred to Phase 8:** CLI/streaming product, cross-arch packaging, final docs.

1. **The D-lock law.** LMO's special-leaf mechanism is *derived, never transcribed*. Before any implementation: a derivation document (docs/lmo_derivation.md) built from LMO 1987 §2–3, Oliveira e Silva's "Computing π(x): the combinatorial method" (the best modern reference), and primecount's `pi_lmo*` source — each term transcribed **with its derivation**, our conventions, our symbols. Term oracles brute-force-certify every term at x ≤ 10⁷ *before* the first performance line of code exists. This is the law that made Lehmer trustworthy; LMO's sieve mechanics are precisely the artifact class where transcription kills.
2. **The field-domain law and differential-extension law** (Finding 2) — retroactive and permanent.
3. **The one-pass law.** One sweep over [x^(1/2), x^(2/3)] serves *every* consumer: S₂ thresholds, special-leaf counters, (at Gourdon rungs) the Σ-term. Segment memory is read once; the epilogue accumulates all consumers. A second sieve pass over the same interval is a design failure.
4. **The contract-integrity law** (Finding 1): contracts = original spec tables, deviations = visible commits.
5. **Pool law unchanged.** The sweep is the same unit pool, the same partition invariance, the same telemetry. Sixth consumer. The sync inventory does not grow.

## PART 2 — THE MATHEMATICS (derived; D-lock completes)

### 2.1 The Meissel identity (the foundation — derived and numerically verified)

For a = π(y) with **p_a³ > x** (y slightly above x^(1/3)), b = π(⌊√x⌋):

**π(x) = φ(x, a) + a − 1 − Σᵢ₌ₐ₊₁ᵇ [π(⌊x/pᵢ⌋) − i + 1]**

*Derivation sketch:* numbers with least prime factor > p_a are (by p_a³ > x) primes or semiprimes pq with p_a < p ≤ q; so primes > p_a = (φ(x,a) − 1) − S₂ where S₂ counts those semiprimes = Σ over p ∈ (p_a, √x] of #primes q ∈ [p, x/p]. *Worked anchor (the term oracles generalize this):* x = 100: a = 3, b = 4, φ(100,3) = 26, Σ-term = π(100/7) − 3 = 3 → π(100) = 3 + 26 − 1 − 3 = **25** ✓.

Note what this identity *is*: it is our Phase 5 Lehmer pushed to a = π(x^(1/3)) — where **P₃ vanishes identically** (the semiprime-only decomposition) and the T-term vanishes. The whole phase is: evaluate φ(x, a) for the much larger a without the tree exploding.

### 2.2 The leaf reorganization (the tree, re-summed)

φ(x, a) = Σ over the φ-tree's T2 leaves of μ(d)·[π(⌊x/d⌋) − π(P⁻(d)) + 1] + Σ over T1 leaves of μ(d)·Φtiny(⌊x/d⌋, i).

A T2 leaf product d = pⱼ·e: e squarefree, all factors in (pⱼ, p_a], leaf condition ⌊x/d⌋ < pⱼ², μ(d) = (−1)^ω(d) — the sign structure *is* the tree's subtract-on-division, and by the Phase 6 unique-path theorem each leaf appears exactly once. (Exact ancestor-validity conditions: D-lock deliverable; the size theorem below is unconditional.)

### 2.3 The architecture theorem (the result that shapes everything — derived here)

**Every multi-factor leaf satisfies ⌊x/d⌋ < x^(1/2); every S₂ threshold lies in [x^(1/2), x^(2/3)).** Proof of the first: d = pⱼ·e with e ≥ p_{j+1}. Case pⱼ ≤ x^(1/4): leaf condition gives x/d < pⱼ² ≤ x^(1/2). Case pⱼ > x^(1/4): then d > pⱼ·p_{j+1} > x^(1/2), so x/d < x^(1/2). ∎ Single-factor leaves (d = pⱼ, existing only for pⱼ ∈ (x^(1/3), p_a]) land in [x^(2/3)-boundary territory — the *top of the same sweep range as the S₂ thresholds*. Consequences, frozen:

- **The π-table (span x^(1/2)) serves every leaf lookup.** No structure above x^(1/2) is ever queried except the sweep itself.
- **One sweep domain: [x^(1/2), x/p_{a+1}) ≈ span x^(2/3)** — serving S₂ thresholds, single-factor leaf points, and the special-leaf counters (one-pass law). At 10¹⁶ this is [10⁸, 4.6 × 10¹⁰] — **21× smaller than Lehmer's P₂ span**, with *fewer* sieving primes (√(x^(2/3)) = x^(1/3) = 2.15 × 10⁵ vs Lehmer's 10⁶).
- At the canonical a = π(x^(1/3)): the formula's validity domain is a ∈ (π(x^(1/3)), b] — **the α-sweep is reborn and legitimate**: larger a shrinks the sweep top (a = π(x^(2/5)) cuts the span another ~4×) at the price of deeper leaf machinery. This is exactly Gourdon's y-parameter (your own file_structure.md: y ≈ x^(1/3)·α_y). α becomes a *measured constant per scale* (C11) — the Phase 5 discipline, now at the level where the tradeoff is real.

### 2.4 The Gourdon term map (onto our modules)

Your file_structure.md's Gourdon decomposition maps cleanly onto what exists: **Φ₀ ↔ titan-core's PhiTiny** (k ≤ 8 — our Phase 1 tables, exactly); **Σ ↔ the Phase 6 threshold sweep** (mechanism owned, span shrunk); **B, D ↔ the special-leaf sieve terms** (the new module, D-locked); **AC ↔ ordinary-leaf enumeration + assembly**. The "Fenwick tree" line in your doc remains wrong (flagged in message 1) — direct prefix-count tables, O(1), which is what we built. **Gourdon is one new module on a substrate we already own.** The special/ordinary split and its sieve realization — *which counter rides which loop, and the exact leaf-validity conditions* — is D-lock deliverable #1, with a hard constraint: it consumes our segment machinery; no second sieve exists.

## PART 3 — ARCHITECTURE

| Module | Role | Reuses |
|---|---|---|
| `meissel.rs` | Identity orchestration, a/b/α selection, assembly with identity assert | titan-core roots, checked arithmetic |
| `leaves.rs` | Ordinary-leaf enumeration (small-d leaves, direct) | π-table, PhiTiny, wheel tables |
| `special_leaves.rs` | The D-locked special-leaf sum, riding the sweep | titan-sieve segments, bucket engine, pool |
| `alpha.rs` | y/a parameterization + C11-fitted defaults per scale | config |
| P₂ sweep (Phase 6, extended) | S₂ thresholds + single-factor points + special counters — one pass | everything |

**Multi-consumer segment epilogue:** per swept segment, one memory pass accumulates (i) segment prime tally, (ii) S₂ threshold-slice partials (the Phase 6 walk-join — LMO's thresholds are *denser* in a *smaller* span, deep in walk territory), (iii) special-leaf counter partials. Per-worker private, pool-joined, u128 where Phase 5's overflow audit says so. **Memory at 10¹⁸:** π-table ~255 MiB (x^(1/2) span), sweep state as Phase 4 sized, Mertens-style per-j partials small — total well inside the device; the geometry sweep (C2, still owed) pins the table constants.

## PART 4 — COST MODEL (with the honest uncertainty band)

| x | Sweep [x^½, x^⅔] @ 6.17 B/s cool | π-table build | Leaf machinery (band — C10 sizes it) | **Total cool MT** | Sustained (~0.45) | primecount ref |
|---|---|---|---|---|---|---|
| 10¹² | 0.02 s | 0.01 s | 0.1–0.3 s | **0.15–0.4 s** | — | 0.102 s |
| 10¹³ | 0.08 s | 0.01 s | 0.3–0.8 s | **0.4–1 s** | — | 0.127 s |
| 10¹⁴ | 0.75 s | 0.02 s | 1–2.5 s | **2–3.5 s** | 4–6 s | 0.288 s |
| 10¹⁵ | 1.6 s | 0.05 s | 2–5 s | **4–7 s** | 9–15 s | 0.689 s |
| 10¹⁶ | 7.5 s | 0.05 s | 8–20 s | **15–30 s** | 35–60 s | 2.756 s |
| 10¹⁷ | 35 s | 0.2 s | 30–90 s | **1–2 min** | 2.5–5 min | (est ~6 s) |
| 10¹⁸ | 162 s | 0.5 s | 100–250 s | **4.5–7 min** | 10–15 min | (est ~25 s) |

The sweep and table lines are certain (certified rates, proven spans). The leaf-machinery band is the phase's genuine unknown — LMO theory says O(x^(2/3)/log), C10 measures the actual constant *before implementation*. Against Phase 6 Lehmer: 10¹⁶ goes 12 min → ~30 s. Against primecount: **class closed, constants not** — expect 5–10× residual at 10¹⁴, growing slowly with scale; that residual is their S2-sieve micro-tuning and cache hierarchy, priced and owned in the ledger, not hidden.

## PART 5 — CORRECTNESS INSTRUMENTS: The Truth Tetrahedron

We now own three engines with pairwise-disjoint mathematics. The interlock becomes four-sided:

```
titan-sieve (physical) ⟷ Lehmer (Phase 6) ⟷ LMO/Gourdon (new) ⟷ primecount/A006880
```

- **Lehmer-as-φ-oracle (the star instrument):** φ_LMO(x, a) ≡ φ_Lehmer-tree(x, a) at 20+ points — two independent evaluations of the *same mathematical object* by *different algorithms*. No reference binary offers this. Phase 6's engine stays in the codebase permanently as a certification instrument (the sieve's role, one level up).
- **Term oracles:** φ-special, φ-ordinary, Φtiny-part, S₂ — each brute-force-locked at x ≤ 10⁷, both α endpoints, p³-boundary points (the M-leaf-boundary habitat: x = p³ ± 1, mandatory matrix).
- **Cross-engine** vs titan-sieve extended through 10¹¹; **oracle full mode** live to 10¹⁴; **randomized differentials vs primecount extended into [10¹⁵, 10¹⁶]** and [10¹⁶, 10¹⁸] post-marathon (the differential-extension law — the rem_segs lesson, wired in).
- **Mutant registry:** M-mu-sign (flip μ on k-factor leaves — the leaf sum's sign class), M-leaf-boundary (x/d < pⱼ² off-by-one — dies at p³ points), M-j-offset (π(P⁻(d)) ± 1), M-sweep-top (drop single-factor leaf points / top-of-sweep boundary), M-mer tens-partial (lost per-j partial), M-alpha-domain (α pushed below validity — dies at the identity assert). All killed, tiers recorded.
- **Field-domain audit** of every packed structure at the 10¹⁸ envelope, per the new law. **Zero-alloc tripwire** across full π(10¹⁵) MT. **Crash gauntlet** at 10¹⁷ scale (≥ 5 kills, bit-exact resumes) — checkpoints extend to the leaf machinery at subtree/j-slice granularity.

## PART 6 — PRE-FLIGHT (C-series; data before design, the standing order)

| # | Experiment | Question | Method |
|---|---|---|---|
| **C10** | **Leaf census via the Lehmer tree** — per-j leaf counts, factor-count histogram, x/d value distribution, μ-partials per j (the Mⱼ design constants), special/ordinary split sizes | What is the Phase 7 workload, *before Phase 7 code exists*? | Instrument the certified Phase 6 engine's T2 exits at 10¹²/10¹³ (runs in seconds) — the old engine is the census instrument for the new one |
| **C11** | α/y-sweep: a ∈ {π(x^⅓), π(1.5·x^⅓), π(2·x^⅓), π(x^⅖)} at 10¹³/10¹⁴ | Sweep span vs leaf machinery — the measured tradeoff curve; per-scale α defaults | LMO engine post-L7.2, D-lock re-certification at each α |
| **C12** | Shared-sweep rate: S₂ thresholds + special counters in one pass vs the Phase 6 sweep alone | The one-pass law's overhead — must be < 10% over bare sweep | deep_bench pattern |
| **C13** | primecount ladder at 10¹⁵/10¹⁶ per algorithm (lehmer/lmo/DR/gourdon) | Their per-rung constants at the marathon scales — our target geometry | benchref, canary sandwich |
| **C2-debt** | π-table geometry (block size, count width) at the 10¹⁸ envelope | Table constants for the marathon | lookup microbench both core types |

## PART 7 — THE LADDER

**L7.0** Retro-audit re-diff (contracts vs original spec tables, visible reconciliation commits) + field-domain retrofit of all packed structures — *honesty before speed, second time* → **L7.1** **D-lock**: derivation doc + term oracles green at x ≤ 10⁷, both α endpoints, p³ matrix → **L7.2** LMO scalar: Meissel assembly + ordinary leaves + S₂ via the existing sweep (single-thread; Lehmer-as-φ-oracle green) — 10¹⁴ ≈ 4–6 s ST → **L7.3** special-leaf machinery on the shared sweep (the D-locked module; one-pass law enforced) — 10¹⁴ ≈ 2–3.5 s MT, **the algorithm-class gate** → **L7.4** MT + pool integration (sixth consumer, partition invariance) → **L7.5** α-tuning from C11 → **L7.6** DR/Gourdon refinements (z-split, k-parameter, Σ-sharing — the B/D/Σ one-pass unification) → **L7.7** micro-rungs (NEON leaf counters, P₃-free epilogue... vector walk where it survives) → **Marathon I**: π(10¹⁷) = 2,623,557,157,654,233, cert-record + crash gauntlet → **Marathon II**: **π(10¹⁸) = 24,739,954,287,740,860 — first phone-native computation of it, checkpointed, kill-resumed, differentially spot-checked in [10¹⁶, 10¹⁸].**

## PART 8 — THE GATE (Law 0 format: PASS/FAIL/OWED, exit = non-PASS count)

| # | Criterion |
|---|---|
| 1 | Retro-audit re-diffed: every contract file enumerates its original spec table; deviations are visible commits |
| 2 | D-lock: derivation doc complete; term oracles green (all terms, both α endpoints, p³±1 matrix) |
| 3 | Lehmer-as-φ-oracle: φ_LMO ≡ φ_Lehmer at 20+ points; cross-engine through 10¹¹; oracle full live 10¹⁴ exit 0 |
| 4 | Randomized differentials vs primecount in [10¹⁵, 10¹⁶] bit-exact (≥ 5 points) |
| 5 | All six mutants killed, tiers recorded; field-domain audit green at the 10¹⁸ envelope |
| 6 | One-pass law: shared-sweep overhead < 10% over bare sweep (C12); sync inventory unchanged; zero-alloc across π(10¹⁵) MT |
| 7 | **π(10¹⁴) ≤ 3.5 s cool MT** (floor 5 s); two-column telemetry records |
| 8 | **π(10¹⁶) ≤ 60 s sustained**, ≥ 20× faster than Phase 6 Lehmer, term ledger reconciled |
| 9 | **Marathon I: π(10¹⁷) exact, cert-record, ≥ 5 kill-resumes bit-exact** |
| 10 | **Marathon II: π(10¹⁸) = 24,739,954,287,740,860, cert-record, ≤ 15 min sustained, differential spot-checks above 10¹⁶** |
| 11 | C10–C13 + C2-debt records exist; C10 leaf census reconciles with the cost-model band; C11's α table recorded per scale |
| 12 | reference.md v7: honest two-column table at every scale including the primecount-gap column (priced, owned); file_structure v2.5 amendments committed |

## PART 9 — AMENDMENTS AND DECISION MAP

**Amendments (v2.5):** the Meissel identity with its worked verification and validity domain; the architecture theorem (all leaf lookups ≤ x^(1/2); one sweep domain [x^(1/2), x^(2/3))]) with proof; the D-lock law; field-domain and differential-extension laws; the one-pass law; α/y as census-owned measured constants; Lehmer reclassified from "engine" to "engine + permanent certification oracle."

**Decision map:** C10's leaf census → the special/ordinary design constants and the cost-model band closure; C11's α table → per-scale defaults (and Phase 8's auto-tuner if the curve is interesting); the truth tetrahedron → every future algorithm claims certification through it; the shared-sweep multi-consumer epilogue → the standing pattern for any future interval-sum machinery; Marathons I/II → the ledger's capstone rows and the CLI product's demo. **Phase 8 preview:** titan-cli (count/stream/bench/marathon subcommands with checkpoint-resume as the phone-native differentiator), cross-arch packaging (x86_64 backend, CI via cross/QEMU), prime-streaming mode (the enumeration audit, productized), and the finale ledger — every number in the repo traceable to a record, every record to a gate, every gate to a contract.

---

Run order: **L7.0 and C10 first, in parallel** — the re-diff restores the ledger's honesty, and the leaf census (running on the engine you already trust) hands Phase 7 its workload statistics before a single new line exists. Paste back the reconciled contract table and C10's histograms — one tells us what we own, the other sizes the war.
