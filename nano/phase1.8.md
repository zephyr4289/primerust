# Phase 5 Post-Game Audit — Then Phase 6: Terminal Lehler & The Marathon Protocol

The phase did its real job: the identity is certified (fourth-power boundaries, sign mutant killed with a visible wrong answer, oracle interlock to 10¹⁴, cross-engine agreement), the explicit-stack law held with 10.7% arena utilization, and the phone now computes π(10¹⁴) in 45.5 seconds where physical sieving needs 10+ hours. But the audit has three findings, and the first one is forensic.

## Finding 1 — The marathons weren't slow. They were *impossible.*

The reported π-table build times carry a signature: 0.589s at 10¹², 2.901s at 10¹³, 19.379s at 10¹⁴. Divide each by the certified engine's rate and back out the implied span: **the table covers ~x^(3/4), not x^½** — 10⁹, 5.6×10⁹, 3.16×10¹⁰ numbers. Confirmation from the other side: P₂'s timings (0.004s / 0.011s / 0.033s) divide by their threshold counts (78.3K / 228K / 664K) to a constant **~50 ns per lookup across three scales** — the fingerprint of random access into a giant table. So the implemented architecture is: *one π-table spanning [0, x^(3/4)], P₂ evaluated by table lookup.*

That was a legitimate simplification — and it has a hard wall. Bits for an x^(3/4) table: **1.05 GB at 10¹⁴ (fits — you ran it), 18.7 GB at 10¹⁵ (does not fit an 8 GB phone, ever).** Criteria 8 and 9 weren't "not yet run" — the current engine *cannot run them at any speed*. Phase 6's first act is therefore structural, not incremental: the threshold-sliced P₂ sweep from the Phase 5 spec is mandatory, and this time the forcing arithmetic goes in the spec where it can't be forgotten.

## Finding 2 — Law 0 was violated *again, by the gate itself*

The Phase 5 spec's own targets: π(10¹⁴) ≤ 8s MT cool, floor 12s → **measured 45.5s, reported as green.** π(10¹⁵) ≤ 90s → not run. 10¹⁶ marathon → not run. Five mutants specified, one reported. C2–C6 specified, C1 reported. This is the third consecutive phase where correctness is real and the performance half is quietly reframed — and this phase *had* Law 0. The gate binary either ignores its criteria or has internal relaxed copies of them. The pattern is now a project risk, so the fix becomes structural, not behavioral:

> **The gate-contract law:** every phase's criteria table lives as a *data file* in the repo (versioned, diffable in git); the gate binary is a generic executor that loads the table, emits PASS/FAIL/OWED per line, and its exit code equals the count of non-PASS lines. Targets cannot drift because drifting them requires a visible commit. Ladder rung M0 retrofits this to all five prior gates and re-runs them — I want to know what Phases 2–4 *actually* owe under honest scoring.

## Finding 3 — Two of my own Phase 5 spec items are dead, and I can prove it

**The memo rung (L3) is dead on arrival, by theorem.** In φ(y, i) = φ(y, i−1) − φ(⌊y/pᵢ⌋, i−1), every root-to-node path divides by a *strictly descending* sequence of prime indices, and floor-division is associative (⌊⌊x/p⌋/q⌋ = ⌊x/(pq)⌋). So y depends only on the *multiset* of divided primes, and each multiset determines exactly one path: **the recursion is a pure tree, not a DAG — revisit rate is structurally zero, memoization gains exactly nothing.** The census's revisit columns I asked for would have read 0 forever; strike them and the L3 rung. The flip side is the phase's biggest gift: a tree with zero shared state is *trivially partitionable* — MT-Φ needs no locks, no coordination, just subtree dispatch.

**The P₂ sweep should have been RAM-forced in the spec**, not presented as an optimization option — the 18.7 GB derivation above belongs in Part 5 of phase1.7.md where a future implementer would have hit it.

**One gem in your data worth framing:** leaves = 303.6M + 500.3M = 803.9M, interior ≈ 803.9M, total 1.607B — a binary tree satisfies leaves = interior + 1, and your census matches it to 0.01%. The census isn't just performance data; it *self-validates the tree structure*. That's a free correctness instrument — keep it as a permanent census assert.

## The honest scoreboard (from your own comparison doc)

We beat primecount at 10¹⁴ in *single-thread vs their 8-thread* (0.041s vs 0.056s — real, but narrow-domain: small-x where Gourdon's setup dominates). We dead-heat primesieve sustained at 10¹¹. We lose combinatorial ≥ 10¹² by 8×–158× — and here's the number that reframes Phase 6: primecount's own **Lehmer** at 10¹³ is 0.786s 8T. The cost model below says terminal-Lehmer lands at ~1–1.4s at 10¹³. **The engineering war at the Lehmer class ends in parity; the remaining gap to their Gourdon is purely algorithmic and belongs to Phase 7.** Phase 6's mission is therefore precise: drive Lehmer to its terminal form, and use it to deliver the thing no desktop engine has ever had — **π(10¹⁶), exact, on a phone, killable and resumable.**

---

# Phase 6 Engineering Specification — Terminal Lehmer & The Marathon Protocol

## PART 1 — MANDATE AND LAWS

**Scope:** restructured P₂ (threshold sweep), magic-division + spine-collapsed Φ, MT-Φ, MT/vector P₃, combinatorial checkpointing, the 10¹⁵/10¹⁶ marathons, the gate-contract retrofit. **Deferred:** LMO/DR/Gourdon (Phase 7 — the algorithm war), any α ≠ ¼ policy (C9 will certify its death at scale).

1. **Gate-contract law** (Finding 2's fix) — criteria as data, exit code = non-PASS count, M0 retrofits all phases.
2. **The RAM law:** π-table span is hard-capped at x^½. The 18.7 GB derivation is recorded in the spec. Any structure whose memory is O(x^(3/4)) is forbidden by arithmetic, not style.
3. **Purity law:** the Φ tree has no shared state (unique-path theorem) — MT-Φ must preserve exactly Σ-subtrees = φ(x, a), partition-invariance-tested like every pool consumer before it.
4. **One thesis per rung** — unchanged, and now enforced by gate-contract diffs.

## PART 2 — P₂ RESTRUCTURED: THE SWEEP AND THE WALK

The design from Phase 5, now with the derivations that make each piece non-optional:

**The sweep.** P₂ = Σᵢ₌ₐ₊₁ᵦ π(⌊x/pᵢ⌋) over b − a thresholds (664K at 10¹⁴ → **5.76M at 10¹⁶**). Arguments span [x^½, x^¾] — 3.16×10¹⁰ numbers at 10¹⁴, 10¹² at 10¹⁶. Sieve that span with the Phase 4 bucket engine through the pool (sieving primes reach x^⅜ = 10⁶ at 10¹⁶ — 55K bucketed primes, inside the certified envelope; zero new sieve code, third consumer). Cost = span ÷ certified MT rate: 5.2s at 10¹⁴, 30s at 10¹⁵, ~164s cool / ~415s sustained at 10¹⁶. **This is the dominant term of the 10¹⁶ run — everything else is engineering detail by comparison.**

**The threshold density derivation (this decides the join's design).** Map primes p ∈ (x¼, x½] through t = x/p: threshold density per unit of t is x/(t²·ln(x/t)). At t = x^½ this is 2/ln x ≈ 0.11 per number at 10¹⁶ — **~215K thresholds inside each 1.97M-number segment**; at t = x^¾ it is ~10⁻⁸ — zero. Integrating: *all 5.76M thresholds live in the first ~27 segments of the sweep; the remaining ~507K segments contain none.* The sweep splits into a **dense band** (≈27 segments near x^½) and a **sparse ocean** (pure tally, feeding prefix sums).

**The join — and the trap it must avoid.** Naive design: per threshold, a pi_range partial count from the unit's start. Average half-unit ≈ 15.4K words of popcount per lookup × 5.76M lookups ≈ **144 G cycles ≈ 65 seconds of death** — four orders worse than the sweep itself. Correct design: thresholds within a dense unit are ~9 numbers apart, so π(t_{k+1}) − π(t_k) counts primes in an interval of ~9 numbers — *walk each dense unit once*, accumulating word-popcounts, emitting the running total at each threshold boundary (~12 cycles/word): **~10M cycles total ≈ 5 ms.** The join is noise; the trap is real; the derivation above is why the walk is the only legal design. Sparse units: per-unit tallies (the sieve already produces them) prefix-summed in one 4 MB pass. Accumulator: u128, per the Phase 5 overflow audit.

## PART 3 — Φ TERMINAL FORM: KILL THE DIVISION, COLLAPSE THE SPINE

Current: 1.6B nodes, 25.5s, **35 cycles/node.** Decomposition: ~800M interior nodes, each performing one hardware u64 division (~10–20 cycles on A76, and *far* worse on the in-order A55s that MT-Φ will recruit), one T2 threshold compare, two pushes, one pop. Attack both costs:

**Magic division.** The recursion divides by pᵢ — and *every division at tree level i uses the same pᵢ* (the divide-child of any level-i+1 node divides by p_{i+1}). Precompute, per prime index, a 128-bit magic constant: quotient = mulhi(y, magic) + shift — ~4–5 cycles, exact for the domain (y < 2⁵⁴, p < 2¹⁴ at 10¹⁶ — generous bounds). **Certification instrument (non-negotiable):** each magic constant is verified by its mathematical validity bound *and* a randomized u128 differential at 10⁶ points per prime in debug builds; M-magicdiv (one deliberately wrong constant) must die there.

**Left-spine collapse.** The no-division child chain (y, i) → (y, i−1) → … exits T2 at level j where y < p_{j+1}² ⟺ **j = π(⌊√y⌋)** — directly computable via titan-core isqrt + the (now x^½-span) π-table. So a maximal left chain of length L collapses to: one isqrt, one π-lookup, L right-children pushes, one direct leaf evaluation π(y) − j + 1. The intermediate left-node push/pop round-trips — several hundred million of them — simply cease to exist. Savings stack with magic division; the node ledger target: **35 → 15–20 cycles/node.** (Census cross-check: depth at 10¹⁴ was 441 = a − 5; at 10¹⁶, a = 1,229 → depth ≈ 1,224. The 4,096-entry arena holds. Assert it.)

**Node scaling law (empirical, from your census):** ×8.43 per decade — 22M (10¹²), 191M (10¹³), 1.6B (10¹⁴) → **13.5B at 10¹⁵, ~114B at 10¹⁶.** Note the exponent heuristic nodes ~ a^3.7: it is also the death certificate for the α-sweep — at α = 0.30, a grows ×5 and nodes grow ×~500. C9 runs the mini-census at 10¹² anyway (cheap), because "the model killed it" deserves one confirming measurement before burial.

## PART 4 — MT-Φ: TWO STRATEGIES, THE CENSUS ARBITRATES

The tree is pure (Finding 3) — but it is *violently left-heavy*: the leftmost subtree φ(x, a−1) is ~(445/446)^3.7 ≈ **99.2% of the total tree** at 10¹⁴. Uniform depth-8 splitting is worthless (the biggest piece is the whole work). Two viable strategies, decided by **C7 (subtree/frontier census)**:

| Strategy | Mechanism | Risk | Killer feature |
|---|---|---|---|
| **Spine-split DFS** | Dispatch the right-children of the left spine as pool units + terminal φ(x, j₀); sizes follow the a^3.7 curve | Skew persists into pieces; needs deep splits | Zero new machinery — pool + explicit stacks, verbatim |
| **BFS level-banding** | Process the tree level-by-level; frontier slices are pool units; per-level barrier (≤1,229 barriers/run — µs each) | Frontier *storage*: width × 8 B must stay ≪ RAM | **Every division in a level uses the same prime** — one magic constant per level, and wide-middle y values fit u32 → **NEON 4-wide magic-division across the frontier** |

The BFS gem is where "every cycle in favor" becomes literal: a vectorized division-by-constant across a u32 frontier could push the wide bands toward 2–4 cycles/node, and the A55's NEON does the same 4-wide u32. But BFS only lives if C7's frontier-width histogram stays under ~50–100M nodes (400–800 MB). Spec both; the data picks; the loser is documented as a measured negative (the standing ladder discipline).

## PART 5 — P₃ TERMINAL

For fixed i, the arguments x/(pᵢpⱼ) descend monotonically in j — the lookups *stream down* the π-table's count array and bit blocks: descending-prefetchable, ~8–12 cycles/term with software pipelining, NEON-assisted. Terms at 10¹⁶ ≈ 3–4×10⁸ → ~1.5s scalar, <0.5s vector/MT. Split over i (19K independent i-walks at 10¹⁶) — embarrassingly parallel, partial sums per worker, partition-invariance tested. Small rung; do it because the 10¹⁶ marathon shouldn't carry a scalar P₃ out of laziness.

## PART 6 — MARATHON PROTOCOL: CHECKPOINT, RESUME, CERTIFY

The crash law extends to the combinatorial engine, and the Phase 4 machinery is the *third* consumer: **P₂ units checkpoint exactly like Phase 4 sieve units** (pool index + completed units + partial tallies). Φ checkpoints at subtree granularity (completed subtree list + partial φ sums — hundreds of bytes). P₃ is short; recompute on resume. Atomic-rename + checksum, 30 s cadence, unchanged. The cert-record protocol gains fields (term-level timings, node counts, checkpoint count, charging flag). **Kill gauntlet at 10¹⁵ scale:** ≥ 5 randomized kill -9 mid-run, every resume bit-exact. This is the phone-native crown — primecount has no equivalent because desktops never needed one.

## PART 7 — CORRECTNESS INSTRUMENTS

The restructure changes *evaluation order and leaf handling*, so the strongest new instrument is **self-differential: current certified v0 becomes the oracle for the restructured engine** — bit-identical π(x) at every scale 10⁶–10¹⁴, plus identity-level: restructured Φ(x, a) ≡ v0 Φ(x, a) at 20 points. Then: subtree partition invariance (k-sweep, jittered dispatch orderings, bit-identical); magic-div certificate (Part 3); cross-engine vs titan-sieve extended to **20+ points including 10¹²** (12 was thin); oracle full mode with the combinatorial candidate, live to 10¹⁴; 10¹⁵/10¹⁶ via cert-record. **Mutants (with the Phase 5 debts):** M-magicdiv, M-spine (exit-level off-by-one — must die at fourth-power boundaries), M-subtree (drop one dispatched subtree), M-threshold (dense-band boundary off-by-one), M-p3-partial (lost worker partial), plus the four unreported Phase 5 mutants (M-a-root, M-phi-tier, M-P2-threshold, M-P3-jstart) as closed debts. Zero-alloc tripwire across a full π(10¹⁵) with 8 workers live.

## PART 8 — PRE-FLIGHT (C-SERIES)

| # | Question | Method |
|---|---|---|
| C2 (owed) | π-table geometry: block size 32/64/128 B, count width | lookup microbench on both core types |
| C7 | **Subtree-size histogram (spine-split) + frontier-width histogram (BFS) at 10¹³/10¹⁴** | census rerun with two extra counters — decides Part 4's fork |
| C8 | Hardware div vs magic div, both core types | microbench — A55's div penalty prices MT-Φ's little-core contribution |
| C9 | α mini-census at 10¹²: a ∈ {π(x¼), π(x^0.27), π(x^0.30)} | buries or resurrects the α-sweep with data |
| C5-debt | primecount ladder at 10¹⁴/10¹⁵ | Phase 7's rung targets |

## PART 9 — THE LADDER

**M0** gate-contract retrofit + re-score of all five prior gates (honesty before speed) → **L0** P₂ restructure, single-thread, self-differential green (10¹⁴: 45.5 → ~26s; table build 19.4s → ~0.01s) → **L1** magic-div + spine collapse (Φ 25.5 → ~10–13s; 10¹⁴ ≈ 25s ST) → **L2** MT: pool Φ subtrees/levels + P₂ sweep + P₃ split — **10¹⁴ ≈ 8–10s cool, the gate target** → **L3** P₃ vectorization + Φ node micro (SoA stack, branchless T2 check) → **L4** pool overlap (Φ and P₂ units co-resident; the pool's front-load already understands heterogeneous unit costs) → **L5** 10¹⁵ first-ever + kill gauntlet → **L6** **10¹⁶ marathon cert-record — the crown: π(10¹⁶) = 279,238,341,033,925, on a phone, checkpointed, kill-resumed.**

## PART 10 — COST MODEL AND PREDICTIONS (calibrate against these)

| x | Φ nodes | Φ MT | P₂ sweep (cool) | P₃ | Total cool | Current | primecount ref |
|---|---|---|---|---|---|---|---|
| 10¹² | 22M | 0.03s | 0.16s | 0.04s | **~0.3s** | 0.842s | Gourdon 0.102s |
| 10¹³ | 191M | 0.27s | 0.92s | 0.2s | **~1.4s** | 6.0s | **Lehmer 0.786s — parity class** |
| 10¹⁴ | 1.6B | 2.3s | 5.2s | 0.7s | **~8–10s** | 45.5s | Lehmer ~4s (est) |
| 10¹⁵ | 13.5B | 20s | 30s | 2s | **~55s cool / ~90 sustained** | *impossible* | Gourdon 0.689s |
| 10¹⁶ | 114B | 165s | 164s cool / 415 sustained | 1.5s | **~5.5 min cool / ~10 min sustained** | *impossible* | Gourdon 2.756s |

Basis: nodes × 17 cycles ÷ ~12G cycles/s mixed-cluster; P₂ = span ÷ certified rates; sustained carries big-cluster derate with little-core ballast (the pool's law). At 10¹³ we land in primecount's *own Lehmer* class — the honest meaning of "terminal Lehmer." The 10¹⁴–10¹⁶ gap to Gourdon is O(x^(2/3)) vs O(x^(3/4)) — an algorithm fact, priced and owned, not hidden.

## PART 11 — THE GATE (gate-contract format, exit = non-PASS count)

1. Gate-contract retrofit complete; all six phase gates re-scored under data-file criteria; discrepancies reported (retro-audit is a deliverable, not a footnote)
2. Self-differential: restructured ≡ v0 bit-identical, 10⁶–10¹⁴, plus Φ-term identity at 20 points
3. Subtree/level partition invariance + M-subtree killed
4. Magic-div certificate green; M-magicdiv killed
5. Cross-engine 20+ points incl. 10¹²; oracle full live 10¹⁴ exit 0; all mutants (new + Phase 5 debts) killed with tiers
6. Zero-alloc + stack-depth asserts across π(10¹⁵), 8 workers
7. **π(10¹⁴) ≤ 12s cool (floor 18s)**, telemetry-recorded
8. **π(10¹⁵) exact, ≤ 150s sustained, ≥ 5 kill-resumes bit-exact**
9. **π(10¹⁶) = 279,238,341,033,925 via cert-record: ≤ 12 min sustained, ≥ 3 checkpoints, ≥ 1 mid-run kill-resume**
10. C2/C7/C8/C9 records; C7's verdict on the MT-Φ fork recorded with the losing strategy's measurements
11. Node ledger: 35 → measured cycles/node; ×8.43/decade verified at 10¹⁵/10¹⁶; census leaves=interior+1 assert permanent
12. reference.md v6; Phase 5 owed items closed or scheduled with dates

## PART 12 — AMENDMENTS AND DECISION MAP

**Amendments:** strike the memo rung (unique-path theorem, with proof, recorded); the RAM law with the 18.7 GB derivation; the threshold-density derivation and the walk-not-lookup join law; the gate-contract law; census leaves=interior assert; the α-sweep burial pending C9's confirming shot.

**Decision map:** C7's histograms → the MT-Φ architecture (and Phase 7's φ-sweep parallelization pattern); C8's A55 division penalty → little-core Φ viability; the terminal-Lehmer engine → **Phase 7's substrate** — LMO's special-leaf machinery replaces the 114B-node Φ wedge with a sievable x^(2/3) sweep, and its B/D sifters consume *this* P₂ machinery as their fourth consumer; C5's ladder → Phase 7's per-rung targets; the marathon/cert protocol → every future trillion-scale claim; the re-scored historical gates → the project's first fully honest ledger.

---

Run order: **M0 first** — I want the retro-audit's honest re-score of Phases 0–5 before a single new cycle is spent, because a 6-minute 10¹⁶ built on quietly-reframed gates is worth less than a 12-minute one built on truth. Then C7 (it decides the MT fork), then L0. Paste back the re-scored gate table and C7's histograms — the former tells us what we actually own, the latter decides how the tree falls to eight cores.
