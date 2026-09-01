# Phase 4 Post-Game Audit — Then Phase 5: The Combinatorial Crown

The bucket architecture is certified where it counts: forced-geometry enumeration at both window depths, M-bucket killed with a *visible* wrong answer (665,604 — the mutant produced a countable lie, which is exactly what a discriminator should do), atomic-rename checkpointing with checksum rejection, and F1 — the most consequential measurement of the project so far. But the gate table and the report do not agree, and that gap now produces a permanent law.

## What F1 actually proved (the device's constitution)

Two A76s extract **11.44 GB/s** — the same as all 8 workers (11.04). The bus is saturated by two out-of-order cores; the A55s add compute but essentially zero bandwidth. Three consequences, frozen as the memory law of the machine: (1) any worker needing > ~1.4 GB/s at 8T is dead; (2) the engine at 10¹¹ uses ~0.67 GB/s aggregate (~6% of bus) — **the 10¹¹ shortfall is thermal and cycle-bound, not memory-bound; the byte wall is a 10¹³+ and deep-sweep phenomenon**; (3) the 6 B-entry rung (F2) is worthless at 10¹¹ and real at 10¹³+ — which sequences it *before* Phase 5's deep marathons, not before Phase 5's build. Also note the thermal closure: 6.172 B/s cool burst × 0.454 end-derate = 2.80 B/s vs measured sustained 2.691 — **4%. The Phase 0 curve now predicts Phase 4 rates through the Phase 3 telemetry chain. The accounting closes end-to-end.**

## The owed table (score: ~4.5/12)

| Criterion | Status |
|---|---|
| 1 forced suite (10⁷ **and 10⁸** enum, N mod 30) | 10⁷/W=2/W=4 ✓ — 10⁸ + mod-30 absent |
| 2 four mutants | M-bucket ✓, M-checkpoint ✓ — **M-carry, M-ring absent** |
| 3 live 10¹² oracle | absent (10¹¹ only) |
| 4 10¹³ marathon | absent |
| 5 crash gauntlet at 10¹² | ran at 10⁸ — mechanism ✓, adversarial duration ✗ |
| 6–8, 10, 11 | zero-alloc-with-buckets, 10¹² perf run, head-to-head, F2–F6, ledgers — all absent |
| 9 debts | 10¹¹ improved 41.2→37.2 s (+10.8%) — **unattributed** |

And the structural finding: **the gate binary reports EXIT 0 while half its criteria were never measured.** That is M4 wearing a gate's clothes — a gate that skips criteria is a gate that lies by omission. Hence:

> **LAW 0 (permanent, retroactive):** every gate binary enumerates the *full* criterion table and self-reports each as PASS / FAIL / **OWED**; any OWED line forces nonzero exit. Retrofit all previous gates. A green gate must be synonymous with a complete gate.

**Debt schedule (D1–D9), parallel-trackable:** D1 10¹² 8T run + live oracle (~7 min); D2 10¹³ marathon cert-record (charger-flagged, ~40–60 min); D3 M-carry/M-ring; D4 10⁸ enumeration + mod-30 matrix; D5 F2/F4 (entry size, W sweep — **required before Phase 5's 10¹⁵/10¹⁶ marathons**, they price the deep sweep); D6 true-scale crash gauntlet (20 kills during a 10¹² run); D7 primesieve 10¹² head-to-head; D8 the attribution A/B (4S vs 8S boundary, R2 on/off at 10¹¹) before the ledger freezes; D9 per-tier telemetry ledger. None block Phase 5's build; D5 blocks its marathons.

---

# Phase 5 Engineering Specification — `titan-count`: The Lehmer-Class Combinatorial Engine

The ghost to exorcise: reference.md's own table says seive.md **crashed with stack overflow at 10¹⁵** and hid behind a 30 B/s coverage fiction. Phase 5 delivers what that code aspired to: **π(10¹⁶) exact, on this phone, in minutes, killable and resumable.** The speed war against primecount's Gourdon (2.756 s) is Phase 6; Phase 5's war is exactness at scale, on our own mathematics, with every cycle accounted.

---

## PART 1 — MANDATE AND LAWS

**Scope:** `titan-count` crate: `pi_table.rs`, `phi.rs`, `p2_sweep.rs`, `p3.rs`, `assembly.rs`, census binary, gate binary. **Excluded:** LMO/DR/Gourdon (Phase 6), MT-φ (a rung, not v0).

1. **Law 0** (above) applies from this phase forward.
2. **Formulas are certified, not trusted.** The assembly identity is brute-force-locked at small x *before any performance code exists* — including by me, now, below.
3. **The explicit-stack law.** Zero recursion anywhere. The φ tree runs on a bounded arena stack. The stack-overflow class dies structurally, not by tuning.
4. **Every cycle serves a term.** In titan-count a cycle either evaluates one φ node, serves one π-table lookup, sieves one P₂ number, or adds one P₃ term. Everything else amortizes per the standing law.
5. **The second-consumer law.** P₂ is titan-sieve's machinery with a new epilogue — no new sieve code, no new pool code.

## PART 2 — THE MATHEMATICS (derived, and derivation-grade)

Let a = π(⌊x¼⌋), b = π(⌊√x⌋), c = π(⌊x⅓⌋) — titan-core roots, exact. Then:

**π(x) = Φ(x, a) + T(a, b) − P₂(x, a, b) − P₃(x, a, c)**

- **Φ(x, a)** = #{n ≤ x : gcd(n, p₁···pₐ) = 1}
- **T(a, b)** = (b + a − 2)(b − a + 1)/2
- **P₂** = Σᵢ₌ₐ₊₁ᵇ π(⌊x/pᵢ⌋)
- **P₃** = Σᵢ₌ₐ₊₁ᶜ Σⱼ₌ᵢ^{π(⌊√(x/pᵢ)⌋)} [π(⌊x/(pᵢpⱼ)⌋) − (j − 1)]

**Why this is exact (the one-line proof that justifies the whole structure):** composites ≤ x with least prime factor > pₐ decompose as pᵢ·m with m ≥ pᵢ; recursing once more, m = pⱼ·m″ requires m″ ≥ pⱼ² and m″ ≤ x/(pᵢpⱼ), so m″ composite would force **pᵢ·pⱼ³ ≤ x — impossible once pᵢ > x¼** — so every inner m″ is prime, the sums terminate, and nothing is approximated. Your original seive.md used this same sign convention (−P₃); the derivation above confirms it — and the term oracles will re-confirm it mechanically, because sign/index conventions are the single most error-prone element of Lehmer implementations and mixing conventions from different papers is the M-assembly class.

**The exactness domain (this phase's original result — the α-sweep's license):** the proof only requires p_{a+1} > x¼. So the formula family is **exact for any a ∈ [π(⌊x¼⌋), π(⌊x⅓⌋)]** — at the left end (Lehmer): minimal φ depth, P₂ span [x½, x¾]; at the right end (Meissel, a = c): **P₃ vanishes, P₂ span shrinks to [x½, x⅔]** — at 10¹⁶ that's a **21× reduction in sieved numbers** (10¹² → 4.6 × 10¹⁰) — paid for by a deeper φ tree and a larger π-table (the T2 exit bound p_{a+1}² grows from x½ toward x⅔). Three-way trade: P₂ span vs π-table size vs φ wedge. **α is a measured constant, not a formula constant** — C1/C6 decide it per scale. Boundary hazard: x = p⁴ (perfect fourth powers) is where the a-threshold bites — the certifier gets x ∈ {2⁴, 3⁴, 5⁴, 7⁴, 11⁴} ± 1 as mandatory cases.

**Overflow audit (documented, u128-cross-checked in tests):** at 10¹⁶ — T ≈ 1.66 × 10¹³ ✓; Φ ≤ x ✓; P₂ sum ≈ 10¹⁵ magnitude but its per-term accumulation approaches u64's edge by 10¹⁸ → **P₂ accumulator is u128 unconditionally** (free insurance). All roots/indices via titan-core.

**Term domains (they dictate every table bound):** Φ recursion arguments exit at y < p_{i+1}² ≤ x½; P₃ lookups live in [x⅓, x½]; P₂ arguments live in [x½, x¾] — *beyond any table, which is precisely why P₂ is a sieve job.* One table covers [0, x½]; one sweep covers [x½, x¾]; nothing else exists.

## PART 3 — THE π-TABLE

Representation (primecount-grade, ours): wheel-30 sieve bits over [0, Y_tab] (at 10¹⁶, α = ¼: 3.34 MB) + **u32 prefix counts per 64-byte block** (208 KB at 10¹⁶ — L2-resident). Lookup contract: π(y) = count[block(y)] + popcount(bits within block below y) — one L2 read + ≤ 8 word-popcounts, ~15–30 cycles. Built by the Phase 2 engine in ~0.05 s at 10¹⁶ scale. Geometry (block size 32/64/128 B) is C2's sweep. Y_tab is a φ-engine parameter (Part 4) — v0: p_{a+1}². This becomes a titan-core-style certified primitive with its own exhaustive tests (round-trip vs the enumeration audit's list, boundary y = block edges, y = pᵢ ± 1).

## PART 4 — THE Φ ENGINE (where seive.md died)

Four tiers, evaluated on an **explicit DFS stack** (depth ≤ a ≤ 1,229 at 10¹⁶ — bounded arena, asserted, zero alloc):

| Tier | Condition | Cost |
|---|---|---|
| T0 | i = 0 | return y |
| T1 | i ≤ 6 (Φ-tiny) | **Phase 1 tables — L1D, ~5 cycles. PhiTiny's entire reason to exist pays off here.** |
| T2 | y < p_{i+1}² | π-table lookup − i + 1 |
| T3 | interior | φ(y, i) = φ(y, i−1) − φ(⌊y/pᵢ⌋, i−1) — push both children |

**The census-then-memo discipline (no guessed caches ever again):** v0 ships *unmemoized* with counters — nodes per depth, tier-exit histogram, distinct-y per level, revisit rates. That census (C1) then selects the memo from measured data: dense level-1/2 arrays (y = x/p and x/pq sorted, direct-indexed by prime-index pairs — at 10¹⁶ level-2 is ~6 MB), or a sorted (y, i) array, or *no memo at all* — at 10¹⁴ the tree may be small enough that memoization is pure overhead. The 64 MB direct-mapped hash of the ancestral code is not an option on the menu; it was a guess wearing a data structure's clothes. Node-count prediction band at 10¹⁶: 10⁷–10⁹ (census arbitrates; if it exceeds 10¹⁰, the α-sweep or Phase 6's LMO becomes mathematically mandatory — the census will have *derived* why LMO must exist, which is the best possible sign the ladder is honest).

## PART 5 — P₂: THE SECOND CONSUMER

P₂ = Σ π(⌊x/pᵢ⌋) over i ∈ (a, b] is **one contiguous sweep** over [x/p_b, x/p_{a+1}] = [x½, x¾] with threshold slicing: sort the b − a thresholds descending; pre-slice them per pool unit at construction (O(1) amortized pointer — thresholds cluster near x½ where pᵢ is dense, sparse near x¾; per-unit lists vary, total memory trivial). Each pool unit sieves its number-range with the Phase 4 engine (buckets live — the sweep's top at 10¹⁶ is 10¹², inside the certified domain) and returns partial counts per contained threshold; join sums them into the u128 accumulator. **Zero new sieve code, zero new pool code, zero locks** — the sync inventory does not grow. The Φ tree and P₃ run as pool units themselves (v0: sequential phases; L5 overlaps them — the pool's heterogeneous front-load already knows how). M-P2-threshold mutant: off-by-one in threshold slicing — the term oracle kills it at small x.

## PART 6 — P₃ AND ASSEMBLY

P₃ is a pure single-thread π-table walk: for fixed i, the arguments x/(pᵢpⱼ) descend monotonically in j — **sequential, prefetch-perfect table stream**, ~10 cycles/term, ~4 s at 10¹⁶, vectorizable later (L6). The j-bounds π(⌊√(x/pᵢ)⌋) via π-table + titan-core isqrt. Assembly: checked arithmetic, u128 accumulator for P₂, one final identity assert. M-P3-jstart (j from i+1) and M-assembly-sign (+P₃) mutants must both die in the term oracle — the sign convention I derived above and the convention the code implements must be forced to agree by machine, not memory.

## PART 7 — CORRECTNESS INSTRUMENTS (three new classes)

1. **Term oracles — certify the formula, not just the code:** Φ, T, P₂, P₃ each computed *by direct definition* (brute force, slow, auditable-by-reading) at x ≤ 10⁷, compared term-by-term against the engine at both α endpoints and the boundary cases x = p⁴ ± 1. This kills derivation errors, transcription errors, and convention mixing — classes no output-only oracle can see.
2. **Cross-engine differential — the project's strongest internal instrument:** titan-count vs titan-sieve at 20+ points in [10⁶, 10⁹] plus 10¹⁰/10¹¹. Two engines, disjoint mathematics (combinatorial identity vs physical bit-sieving), disjoint failure classes, agreeing bit-exactly. No reference binary provides this; we built both sides.
3. **External:** oracle full mode with titan-count as batch-protocol candidate — T1/T2/T3 live to 10¹⁴; randomized differentials vs primecount in [10¹², 10¹⁵] (primecount answers any x in ~0.1 s — instant truth at any scale).

Mutant registry: M-a-root (isqrt where iroot4 belongs — wrong a at every x), M-phi-tier (p_i² for p_{i+1}²), M-P2-threshold, M-P3-jstart, M-assembly-sign. All must be caught with tier recorded. Zero-alloc tripwire across a full π(10¹³) evaluation. Explicit-stack depth assert. Hygiene + telemetry per standing policy — engine telemetry supersedes canary normalization for sustained runs (Phase 3 law).

## PART 8 — PRE-FLIGHT EXPERIMENTS (C-series, data before design)

| # | Experiment | Question |
|---|---|---|
| C1 | **φ-census** at 10¹²/10¹³/10¹⁴: node counts per depth, tier exits, distinct-y, revisit rates | memo design; tree-size model; the α-sweep cost curve |
| C2 | π-table geometry sweep (block size, count width) | lookup cycles vs cache residency |
| C3 | P₂ sweep rate at 10¹⁴, k = 1..8, threshold overhead measured | the sweep's real MT rate; unit sizing |
| C4 | Term cost attribution at 10¹²–10¹⁴ | the titan-count cycle ledger |
| C5 | primecount ladder extension: lehmer/lmo/dr/gourdon at 10¹⁴/10¹⁵ (we hold 10¹³) | per-rung targets; calibrates our P₂-span model against their Lehmer times |
| C6 | α-sweep dry run at 10¹² with census at a ∈ {π(x¼), π(x^0.28), π(x^0.30), π(x⅓)} | the three-way trade measured, not guessed |

## PART 9 — THE LADDER (one change per rung, ±3% keep-or-revert, oracle-quick between)

**L0** scalar-correct v0 (sequential: table → Φ → P₃ → P₂-ST → assembly) — *gate: exact at 10¹⁴, cross-engine green; predicted ~15 s single-thread at 10¹⁴* → **L1** MT P₂ via pool (second consumer) → ~5.5 s → **L2** π-table geometry from C2 → **L3** Φ memo from C1 → **L4** **α-sweep** (re-tune a from C1+C6; formula structure re-certified at the new a — P₃ shrinks or vanishes; potentially the difference between minutes and tens of seconds at 10¹⁵/10¹⁶) → **L5** Φ/P₂ overlap on the pool → **L6** micro-rungs (P₃ vector walk, Φ-node SoA layout — the R2 constant/mutable law, third consumer) → **Marathon** 10¹⁶ cert-record with checkpoints (precondition: D5 closed).

## PART 10 — COST MODEL AND PREDICTIONS (calibrate against these)

| x | P₂ span | P₂ @ ~6 B/s cool | Φ + P₃ + tables | Total cool | Reference (theirs) |
|---|---|---|---|---|---|
| 10¹² | ~10⁹ | 0.17 s | ~0.05 s | **~0.3–0.5 s** | Gourdon 0.077 s |
| 10¹⁴ | 3.2 × 10¹⁰ | 5.3 s | ~0.5 s | **~6–8 s** | Lehmer ~4 s (est), Gourdon 0.288 s |
| 10¹⁵ | 1.8 × 10¹¹ | 30 s | ~1.5 s | **~30–40 s cool / ~70 s sustained** | Gourdon 0.689 s |
| 10¹⁶ | 10¹² | 165 s cool / 370 s sustained | ~6 s | **~3–7 min** | Gourdon 2.756 s |

Phase 5 does not fight those reference numbers — Phase 6 does. Phase 5's fight: exact, phone-native, crash-proof, ours. If C4's term ledger misses these by > 2×, the census says which term lied.

## PART 11 — THE GATE (Law 0 format: PASS/FAIL/OWED, no omissions)

| # | Criterion |
|---|---|
| 1 | Term oracles: all four terms brute-force-certified at x ≤ 10⁷, both α endpoints, p⁴±1 boundaries |
| 2 | Oracle full, titan-count as candidate: exit 0, live T3 to 10¹⁴; differentials vs primecount at 5+ points in [10¹², 10¹⁵] |
| 3 | Cross-engine: titan-count ≡ titan-sieve at 20+ points [10⁶, 10⁹] + 10¹⁰/10¹¹ |
| 4 | Five mutants killed, tiers recorded |
| 5 | Explicit-stack assert + zero-alloc tripwire across full π(10¹³) |
| 6 | C1–C6 records exist; census reconciles with the node model within 2× |
| 7 | π(10¹²) ≤ 0.5 s MT cool; **π(10¹⁴) ≤ 8 s MT cool** (floor 12 s); both telemetry-recorded, two-column |
| 8 | π(10¹⁵) exact recorded; ≤ 90 s sustained stretch |
| 9 | **Marathon: π(10¹⁶) = 279,238,341,033,925 via cert-record — with ≥ 3 checkpoints and at least one kill-resume mid-run** (D6 harness, true scale) |
| 10 | Term ledger (C4) reconciled within 2× |
| 11 | D1–D9 closed or OWED-with-schedule — nothing silently dropped |
| 12 | Gate record + reference.md v5; Law 0 retrofitted to all prior gate binaries |

## PART 12 — AMENDMENTS AND DECISION MAP

**file_structure.md v2.4:** the exactness-domain theorem (a ∈ [π(x¼), π(x⅓)]) recorded with its one-line proof; α and Y_tab declared measured constants (census-owned); the Φ-tier law; π-table representation law; P₂-as-second-consumer law; Law 0; the D1–D9 schedule.

**Decision map:** C1 census → memo design and the α choice → also the *derivation* of whether Phase 6's LMO is forced; the π-table → Phase 6's π-table needs verbatim; the P₂ sweep machinery → Gourdon's B/D sifting terms (same interval-sweep, different thresholds — third consumer); term oracles → the standing pattern for certifying every future combinatorial identity; cross-engine harness → permanent two-machine truth interlock; checkpoint marathons → the CLI's defining feature.

---

Run order: **C1 census first** — it sizes the Φ memo, prices the α-sweep, and tells us whether the tree at 10¹⁶ is a 2-second job or a mathematical force pushing us toward LMO. Paste back the census histogram and L0's first π(10¹⁴) — the derivation above says the formula is exact; now the silicon gets to say how fast it's exact.
