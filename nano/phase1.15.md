# Phase 13 Post-Game Audit — The Race Ran, and For the First Time the Map Is Measured

S5 is paid. Seven phases of cross-device comparisons ended in one session, and the instruments did their job — including on me: one of my Phase 13 predictions died in this data, and I'll bury it in public below. Receipts first.

## What is now certified (same device, same session, pure-compute, best-config — the definition-of-winning law satisfied for the first time)

| Front | Result | Status |
|---|---|---|
| Combinatorial 10¹⁰ | Titan 0.0334s vs pc 0.0905s → **2.71×** | **WIN, certified** |
| Combinatorial 10¹⁰, ST-only | 0.0243 vs 0.1025 → **4.2×** | The buried stronger form — see F3 |
| Combinatorial 10¹¹ | 0.0786 vs 0.0778 | Dead heat |
| Combinatorial 10¹²–10¹⁴ | 3.6× / 17.7× / **45.25×** | LOSS, now *measured* |
| Physical 1e9 | 1.07× | **WIN** |
| Physical 1e10–1e11 | 1.55× / 1.90× | LOSS — regression vs G100 parity, see F4 |
| Scaling 8T/1T at 10¹⁴ | 3.43× vs 2.73× | Real — but read F2 before celebrating |

## F1 — The setup floor: the discovery that defines the winning regime

primecount's 1T time *decreases* from 10¹⁰ (0.1025s) to 10¹¹ (0.0959s). A combinatorial engine cannot get faster as x grows — unless a **fixed serial prologue dominates both scales**. Decompose: setup ≈ 0.09s, compute(10¹²) ≈ 0.03s, compute(10¹³) ≈ 0.15s, compute(10¹⁴) ≈ 0.66s (growth ×5, ×4.4 — the x^(2/3) signature, internally consistent). Gourdon's y-sieve, μ-tables, π-table, and α machinery cost ~90 ms of *serial* time before any counting begins. Titan's prologue is ~20 ms. **The 10¹⁰ win is not PhiTiny magic — it is their serial tax.** And it defines the winning regime precisely: we win wherever Gourdon's compute < ~70 ms, i.e., x ≲ 10¹¹. That's the honest boundary of the crown, and it is now measured, not asserted.

## F2 — My scheduling-front prediction, corrected by the race (the instruments work on me too)

I predicted primecount's OpenMP would extract ~2.5–3× on 2+6, leaving us a ~2× scheduling championship. Measured: their raw 8T/1T is 2.73× at 10¹⁴ — but that ratio is *diluted by their 0.09s serial setup*. De-serialize it: (0.2748−0.09)/(0.751−0.09) = compute-only scaling = **3.58×**. Our pool: 3.43×. **Parallel-efficiency parity.** Walisch's OpenMP is as good as our pool on this topology; the visible 3.43-vs-2.73 edge is mostly their serial tax, not our scheduler. The scheduling front narrows to three real edges: the setup regime (F1), sustained-thermal scheduling (theirs unmeasured at hour scale — still open), and heterogeneous unit granularity. The gap decomposition at 10¹⁴ is therefore clean: **total 45.25× ≈ algorithm/constants ~57× (1T: 56.7×), minus ~1.26× scheduling claw-back.** The war is the algorithm. Nothing else in this data moves the needle — which is exactly why Phase 14 is what it is.

## F3 — The selector bug the race exposed

Titan 8T at 10¹⁰ (0.0334s) is *slower* than Titan 1T (0.0243s) — pool spawn + unit overhead exceeds 24 ms of work. The race verdict understated the win by comparing our MT number. The engine needs a scale-indexed dispatch: x ≤ 10¹⁰ → ST path. One config line; the 4.2× headline claim becomes available. Trivial fix, real claim upgrade.

## F4 — The physical regression: config, not silicon

Titan-sieve 8T at 1e11: 37.65s — *identical to the G100's 37.15s*, while primesieve improved from ~38s to 19.77s on the same silicon. Zero transfer versus +19% for the opponent. Worse: our ST rate at 1e11 is 1.11 B/s vs their 2.48 — we *degrade* 48% from 1e10→1e11 while they degrade 15%. The mechanism smells like the Phase 8 tier re-derivation leaving the bucket pool / W-depth / state sizing tuned for the old 64 KiB thresholds while the 32 KiB geometry pushed ~26K more primes into the bucket tier at 1e11. This is a **device-profile violation** (hardcoded constants instead of profile-owned), and it goes to the debt register — not Phase 14's critical path. The algorithm war is 57×; the physical race is 1.9×. Priorities are arithmetic.

## F5 — R1 census delivered the *old* data and skipped the four counters

What came back: the ω-histogram (2.51M/24.3M/92.5M/93.3M/5.1M, summing to 2.18×10⁸ at 10¹³ — internally consistent, and consistent with Phase 9's C10) and Theorem 2.3 re-verification (already C10-certified in Phase 8). What was asked and is still missing: **distinct-⌊x/d⌋ count, the (j,v)-cell count, the μ-span, the Meissel node confirmation (~5.4×10⁹ ± 20%).** These four numbers write every design constant of the interval substrate. Since the census won't deliver them yet, I derive the v-side myself, on the record: distinct ⌊x/d⌋ over the leaf d-domain spans v ∈ [x^⅓, x^½) → at 10¹³ ≈ 3.14×10⁶, at 10¹⁴ ≈ 10⁷. Against 2.17×10⁸ measured leaves at 10¹³: **≥ 70:1 sharing on the v-side, ~170:1 at 10¹⁴, growing with scale.** The (j,v)-cell count — the true op count of a correct LMO — sits between distinct-v (floor) and leaf count (ceiling), with the literature's O(x^(2/3)/log x) ≈ 6.7×10⁷ at 10¹⁴ as the central estimate. **Band: 10⁷–10⁹. That band is the difference between a 20× collapse and a 2× nothing, and the census counter is the only thing that collapses the band.** It runs first.

## F6 — D-lock-3 is a document with a certification hole, and the scoreboard dropped a phase

The Gourdon decomposition has **six terms labeled "5-term,"** certified against two output-only anchors (10³, 10⁴). Two output checks cannot certify six terms — compensating errors across terms pass output oracles trivially; this is the M-assembly-sign lesson and the entire reason the D-lock law demands *term-level* oracles. And the scoreboard now skips Phase 12 entirely (rows jump 11 → 13; 130 criteria where 14 phases should carry ~142): the phase that contained the marathons was dropped from accounting rather than scored OWED. Marathons: sixth phase owed. Fix, permanent: **M-I and M-II become standing scoreboard lines (OWED), removed from phase gates where they've been printed PASS without running — a lying PASS is worse than an honest OWED.** They run in the background during Phase 14; the ledger tracks them until they exist.

One more ledger item: the Phase 10 matrix and this race disagree on the *same engine* — Lehmer 8T at 10¹²: 0.593s vs 0.3003s (−49%); at 10¹¹: 0.125 vs 0.0786 (−37%); at 10¹⁴: 11.68 vs 12.44 (+6%). Two configs or two thermal states; both cannot stand. Hence the **config-digest law**: every bench record carries a hash of (α, geometry, thresholds, thread config); the reconciliation run re-measures the matrix rows in one session, one config, digest attached.

---

# Phase 14 Engineering Specification — R3+R4: The Interval Substrate

The war is 57× and it is algorithmic. Phase 14 builds the machinery that turns 2.17×10⁸ individually-walked leaves into interval arithmetic — the collapse that Phase 11 measured as failed (0.33×) because its two prerequisites were absent: μ-coverage beyond x^½ (Phase 11's timing fingerprint) and leaves that ride the sweep (the structure the Phase 11 corpse lacked). Both are engineered here, sub-part by sub-part, to the depth you demanded.

## PART 1 — MANDATE AND LAWS

**Scope:** census completion, D-lock-3 completion, the μ-extension (R3), the Mertens structure (R3b), the interval walker (R4), the multi-consumer epilogue, the structure test, scalar-then-MT bring-up, the reconciliation run, the selector fix. **Frozen:** Lehmer/Meissel engines (oracle + fallback), the pool, the physical sieve (its config audit is a debt line). **Deferred:** Gourdon z-split/Σ rungs (Phase 15), CLI/FFI (Phase 15).

1. **Census-first law:** no substrate code merges before the four counters exist. The 10⁷–10⁹ cell-count band decides whether this phase builds a 20× or a 2× — you don't pour the foundation before the soil report.
2. **Config-digest law** (F6): hashes on every record; the matrix/race 2× discrepancy resolves or the conflicting rows are quarantined.
3. **D-lock completion law:** six terms, term-by-term brute-force oracles, both α endpoints, the *invalid* endpoint, before any performance line exists.
4. **Structure-test law:** the Phase 11 failure mode — leaf evaluation that doesn't ride the sweep — becomes a hard-gate telemetry assert, not a design hope.
5. Standing: numeric-criteria (every threshold is a number), node-accounting (counters, never prose), RAM law, one-pass constitution, pool law, zero-alloc, raw-output paste-backs.

## PART 2 — THE DESIGN CONSTANTS (census + derivation)

| Constant | Source | Design role |
|---|---|---|
| distinct-⌊x/d⌋ at 10¹³/10¹⁴ | counter (my derivation: 3.1×10⁶ / 10⁷) | π-lookup volume — the kernel's memory profile |
| **(j,v)-cell count** | counter (band 10⁷–10⁹; literature ~6.7×10⁷ at 10¹⁴) | **the op count — the collapse verdict** |
| μ-span: Σⱼ len(e-rangeⱼ) | counter | which segments the Mertens machinery touches |
| e-domain inventory per j | D-lock-3 deliverable | the walker's loop bounds |
| Meissel-tree nodes | counter (≈5.4×10⁹ ± 20% at 10¹⁴) | closes F2's file forever |
| v-side sharing | derived: ≥70:1 at 10¹³, ~170:1 at 10¹⁴ | the theoretical ceiling of the collapse |

**The pre-committed verdict arithmetic:** cells ≤ 10⁸ at 10¹⁴ → the walker costs ≤ 0.15s MT, the substrate lands ~0.8–1.2s total, **GO for everything**. Cells ≥ 10⁹ → the leaf population is the wall at α = 1, and the phase pivots to the α/y sweep *before* more machinery (the Phase 13 α-front, now load-bearing). The counters, not enthusiasm, pick.

## PART 3 — R3a: THE μ-RIDER (segmented Möbius over the extended domain)

**What μ(d) requires per number d:** squarefree flag and Ω-parity. Three marking classes ride the existing segment machinery:

- **p²-marking:** only primes ≤ √(domain-high) participate — for the x^(2/3) domain that's p ≤ x^(1/3), exactly the S₂ sweep's sieving-prime set. One prime p contributes ⌊span/p²⌋ markings per segment-window — sparse, cheap, and *already in the marking loop's prime list*. No new sieving primes exist anywhere in this design.
- **ω-parity:** one bit-flip per prime crossing — the marking loop's wheel walk already visits every crossing; the flip is one XOR on a parity bit-plane, structurally identical to the tally bit-plane. The μ-sign for squarefree d is (−1)^ω — the parity bit *is* the sign.
- **Layout:** three bit-planes over the wheel-coprime slots (alive / parity / p²-hit), or parity packed as the low bit of a per-slot byte — C12's measurement decides; both ride L1 segments, both are per-worker private.

**Cost model:** the rider's markings add ~Σ1/p² ≈ 0.45 crossings per *number*... no — p²-crossings per number ≈ Σ_{p≤x^{1/3}} 1/p² < 0.4522 minus small primes already excluded by the wheel ≈ ~0.08–0.15 — plus the parity flips *reuse the crossings the sieve already performs* (zero extra wheel walks; one XOR each). Rider overhead target: **< 8% over the bare sweep**, enforced by C12-style A/B. The domain extension from x^½ to x^(2/3) is the expensive part — it's the sweep's own span, which is why the one-pass constitution is not a style choice: the μ-domain and the S₂-sweep domain are the *same interval*, walked once, by the same segments.

**Certification:** Mertens anchors at extended scale — A084237 to 10⁷, then M-checks at the extended domain's own boundaries (the Phase 11 fingerprint dies when 0.0123s becomes impossible: the extended μ-build at 10¹⁴ must cost sweep-class time, ~0.3s+, and the timing itself becomes a coverage proof).

## PART 4 — R3b: THE MERTENS STRUCTURE

**Query contract:** M-difference over arbitrary [lo, hi) in the extended domain, in O(1)–O(log) — the walker calls it once per (j,v)-run.

- **Checkpoints:** i64 running M at segment boundaries — ~2,200 entries at 10¹⁴, ~1M (8 MB) at 10¹⁸. Trivial memory; the query's cost lives elsewhere.
- **The real cost, stated honestly (it is co-dominant):** a within-segment μ prefix-sum, once per touched segment: 262K adds per 32 KiB segment. If the μ-span touches every segment — and it will, since Σⱼ e-ranges blanket the domain — total prefix work ≈ #segments × 262K ≈ 5×10⁸ ops at 10¹⁴ ≈ 0.15–0.25s. **The Mertens machinery is not free; it is a co-equal term with the walker.** Two rungs to measure: (a) prefix-on-touch with per-worker caching (each pool worker prefix-sums its own segment once, serves all j-queries into it — the SoA private-state pattern, fifth consumer); (b) coarser checkpoint lattice (every 8th segment, i64) + local walks for short runs. Rung A/B at forced geometry where the volumes are visible in seconds.
- **The anchoring identity:** every walker result must satisfy Σ-cells = tree-φ at 20 points — the Lehmer-as-φ-oracle instrument, which exists precisely for this.

## PART 5 — R4a: THE INTERVAL WALKER (the collapse kernel)

For each level j ∈ (c, a]: the e-domain is a contiguous interval; v = ⌊x/(pⱼ·e)⌋ is **monotone non-increasing in e**; therefore the domain partitions into maximal runs of constant v. Per run, the special-leaf contribution collapses to:

> **μ-difference(e-run) × [π(v) − j + 1]** — one Mertens query, one π-lookup, one multiply-add.

The per-leaf π-lookup (2.17×10⁸ of them at 10¹³, each an L2 bounce) becomes one per *run*. That is the entire mathematical content of the collapse, and everything else is engineering:

- **Run-splitting without division:** the next run boundary is e* = ⌊x/(pⱼ·(v−1))⌋ — one magic-division per run (the Phase 6 constants, sixth consumer), not per element. Runs near the domain top are single-e (v changes every step — the dense band, exactly the threshold-density geography from Phase 6's P₂ join); runs near the bottom span thousands of e. The walker is two loops: a dense-band stepper and a sparse-band runner — the *same shape* as the P₂ walk-join, which is not a coincidence: both are quotient-geography problems, and Phase 6 already solved the pattern once.
- **Kernel discipline:** the inner accumulation is branch-free — CSEL selects and the sign from the parity plane folded via XOR into the addend; the (j,v) accumulator is per-worker private (no atomics — the sync inventory does not grow).
- **The d-space vs e-space choice:** walk in d-space (d contiguous, v monotone) or e-space (e contiguous per j, v monotone) — D-lock-3's interval inventory decides; the machinery is identical, only the loop bounds and the μ-domain anchoring change. Spec both bounds, implement one, assert the other against the oracle at small x.

## PART 6 — R4b: THE MULTI-CONSUMER EPILOGUE (one-pass, at last)

Per swept segment, one memory stream feeds every consumer: (i) S₂ threshold partials (the Phase 6 walk-join, unchanged), (ii) the μ-rider's three planes (Part 3), (iii) per-j special-leaf counters whose e-ranges intersect this segment, (iv) the segment tally. The walker's j-loop over a segment touches only the (j,v) cells whose e-runs intersect it — the per-segment j-list is precomputable at construction from the interval inventory (sorted, front-loaded into units exactly like Phase 6's threshold slicing: same machinery, seventh consumer). **Zero new sieve code, zero new pool code, one epilogue.** The one-pass overhead mandate stands: < 10% over the bare S₂ sweep, measured.

## PART 7 — THE STRUCTURE TEST (the corpse detector, hard-gated)

Telemetry asserts, per segment: special-leaf cells processed > 0 whenever the segment's j-list is non-empty; μ-planes touched exactly once; Mertens prefix work bounded by the segment's slot count. Any leaf evaluation occurring outside a sweep segment's timing bracket = **hard FAIL** — the Phase 11 architecture (S₁ as a separate walk) is not merely slower, it is *architecturally illegal* from this phase forward. This is the law that makes D-lock-2's constitution checkable, and it never existed until a corpse proved it was needed.

## PART 8 — D-LOCK-3 COMPLETION

Six terms (A, −B, C, D, Φ₀, Σ): each brute-forced against direct definition at x ≤ 10⁵, both α endpoints, and the deliberately-invalid endpoint (α pushed past validity — the identity assert must reject; M-alpha-domain finally dies here, four phases owed). The interval inventory — per-j e-domains, the c-boundary, the d-domain edges — is the deliverable the walker's loop bounds are transcribed from. Worked anchors extended to 10⁵ with term-level values printed, not just π(x). **No performance code merges before this table is green.**

## PART 9 — LADDER, COST MODEL, AND THE HONEST TARGET

**V0** census + D-lock-3 green → **V1** scalar substrate: μ-rider + Mertens + walker + epilogue, ST, identity-vs-tree at 20 points, forced-geometry certification of every new path (the standing pattern) → **V2** MT on the pool, partition invariance, structure test live → **V3** rungs: checkpoint granularity, dense/sparse band tuning, CSEL kernel, prefix caching → **V4** the re-race: Section B v2.

| x | Sweep+μ+Mertens | Walker (cells × ~25cy, MT) | Table+assembly | **Total cool MT** | pc 8T (measured) | Gap |
|---|---|---|---|---|---|---|
| 10¹² | ~0.04s | 0.005–0.02s | 0.001s | **0.05–0.08s** | 0.0829s | **~1–1.6× — parity class** |
| 10¹³ | ~0.15s | 0.02–0.08s | 0.003s | **0.2–0.3s** | 0.1081s | ~2–3× |
| 10¹⁴ | ~0.45s | 0.05–0.25s | 0.012s | **0.5–0.9s** | 0.2748s | ~2–3.5× |

The honest Phase 14 exit, stated in advance: **parity-class at 10¹², within ~3× at 10¹³–10¹⁴, from 45×.** The residual is Walisch's fifteen years of constants — Phase 15's z-split/Σ rungs and the cachegrind Ir-referee eat into it. The winning map after V4: ≤10¹⁰–10¹¹ wins (setup regime), 10¹² parity, 10¹³–10¹⁶ within-3×, sustained-scale and crash-tolerance by forfeit. That is the industry-beating-on-mobile claim, with numbers, and this phase is where it becomes true or dies trying.

## PART 10 — THE GATE (numeric thresholds; raw output only)

| # | Criterion | Threshold |
|---|---|---|
| 1 | Census counters | 4/4 recorded at 10¹³/10¹⁴; node-confirm 5.4×10⁹ ± 20%; cell count recorded with verdict arithmetic |
| 2 | D-lock-3 | 6 term oracles green incl. invalid-α; interval inventory committed; anchors to 10⁵ term-level |
| 3 | μ-rider | M-anchors extended-scale; rider overhead < 8% over bare sweep (A/B measured) |
| 4 | Mertens | query contract certified vs A084237; prefix cost measured; rung verdict recorded |
| 5 | Walker | identity-vs-tree 20 points bit-exact; cells-processed counter = census prediction within 2× |
| 6 | Structure test | live, hard-FAIL, green across the MT suite |
| 7 | One-pass | epilogue overhead < 10% over bare S₂ sweep |
| 8 | Perf | π(10¹⁴) ≤ 1.2s cool MT (floor 2.0s); π(10¹²) ≤ 0.10s; collapse ratio vs Phase 11 S₁ ≥ 15× at 10¹³ |
| 9 | MT | partition invariance k∈{1,2,4,8}; zero-alloc 8 workers; sync inventory unchanged |
| 10 | Selector | ST path ≤ 10¹⁰ wired; 10¹⁰ claim re-issued at 4.2× with ST number |
| 11 | Reconciliation | matrix rows re-measured, config digests attached, 2× discrepancy resolved or quarantined |
| 12 | Ledger | Section B v2 with the gap column; M-I/M-II standing OWED lines; scoreboard restored to 14 phases |

## PART 11 — DECISION MAP

The cell count → collapse verdict and Phase 15's shape (GO → Gourdon rungs + CLI; NO-GO → α-sweep then reassess); the μ/Mertens machinery → the reusable number-theory substrate (μ, M, and every future summatory function ride it); the walker's quotient-geography pattern → the third independent consumer of the Phase 6 join design; the structure test → the permanent guardian of every future "rides-the-sweep" claim; the re-race table → the artifact the finale ledger and the eventual FFI release publish.

---

Run order: **the census counters first — 10 minutes, and they collapse the 10⁷–10⁹ band that everything above is sized against.** Then D-lock-3's term-oracle table while the counters run, then V1. Marathons M-I runs tonight in the background as the standing obligation it now officially is. Paste back, in this order: the four counters with the cell-count verdict line, the six-term oracle table, and — when V1 lands — the identity check and the first 10¹⁴ timing with the collapse ratio. The race gave us the map; the c
