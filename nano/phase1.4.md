# Phase 2 Engineering Specification — `titan-sieve`: The Physical Engine, Single-Threaded

Phase 1 sign-off first, three notes, none blocking: (1) 63.97 KiB static footprint "fits L1D" is true **only on the A76** — on the A55 (32 KiB) PhiTiny partially lives in L2; irrelevant for Phase 2 (phi and segments are never resident together) but it's a Phase 5 line-item, keep it in the record. (2) The gate log shows the icbrt full-domain sweep explicitly but not the isqrt high-range LCG zone — confirm those rows exist inside `titan_core_gate.json` when you next open it. (3) Wheel tables at exactly 0.14 KiB matches the derivation to the byte — the tables are what I specified, Convention A is locked. Clean phase. Now the engine starts, and Phase 2's character is: **the first code that produces primes, and the first code where speed is a deliverable.** Everything below exists to make those two properties independently verifiable.

---

## PART 1 — MANDATE, SCOPE, AND THE LAWS

**Files:** `segment.rs` (driver), `arena.rs` (memory), `base.rs` (bootstrap primes), `presieve.rs`, `erat_small.rs`, `erat_medium.rs`, `tally.rs`, plus a bench binary and a gate binary. **Deferred:** `erat_big.rs`/`bucket.rs` (Phase 4, with a quantitative justification in Part 5), threading (Phase 3), wheel-210 (frozen out in Phase 1 amendments).

**The five laws of this phase:**

1. **The purity boundary.** `titan-sieve` *lib* is zero-dependency, like titan-core. Only *binaries* (bench, gate) may link titan-bench for canary/hygiene/records. The engine itself stays portable and allocation-free at runtime.
2. **All allocation at construction.** One arena, sized once from (N, segment size), zero thereafter — enforced by the Phase 1 tripwire installed in the bench binary, asserted across steady-state segments.
3. **Correctness instruments precede speed work.** The oracle integration (Part 8) must be green on scalar v0 *before* the first optimization rung lands. No rung may merge with the oracle quick-tier red.
4. **One change per rung.** Each optimization is one commit, benchmarked before/after with the canary sandwich, keep-or-revert at ±3%, recorded. A rung that mixes two changes is unattributable and therefore doesn't exist.
5. **No transcription from primesieve.** Every threshold and layout below is *derived* for our wheel, our tiers, our silicon. primesieve is the oracle and the opponent, never the blueprint — its design decisions are tuned for its bucketed tier structure, which we don't have yet (Part 5).

---

## PART 2 — THE COST MODEL (Derive Before Building)

The single number that governs this phase: at primesieve's measured single-thread rate (2.225 B n/s at 2.2 GHz), the entire engine runs at **~1.0 cycles per number**. Our gate target (≥1.5 B/s = 70% of primesieve) is a budget of **~1.47 cycles/number**. That is the whole allowance — marking, bookkeeping, init, tally — averaged over every number in [0, N).

Decompose the budget per number, using Mertens (Σ1/p over primes ≤ x ≈ ln ln x + 0.2615):

| Component | Derivation | Cycles/number (S = 64 KiB, N = 10¹⁰) |
|---|---|---|
| Tier crossings | (8/30)·Σ1/p over p ∈ [17, √N] ≈ 0.267 × 1.40 ≈ **0.37 crossings/number** | 0.37 × ~3 = **~1.1** (scalar) |
| Presieve init | 128 KiB moved per 1.966M numbers ≈ 0.067 B/number | ~0.004 |
| Medium bookkeeping | 25,400 state-touches/segment ÷ 1.97M | ~0.05–0.10 |
| Tally | 8,192 u64-words popcount per segment | ~0.015 |
| **Total (scalar v0)** | | **~1.2–1.3 → predicted 1.2–1.8 B/s** |

Three consequences fall out of this table, and they set the ladder's priorities:

- **Crossing work is ~75–90% of the budget.** Tally NEON-ization is worth ~1%; it's a rung, but a small one. Presieve *extension* (Part 6) removes 8% of crossings — bigger than the entire tally cost. Attack costs in proportion to the model, not to glamour.
- **The model is the anomaly localizer.** When measured rate deviates from the model by >2×, the per-tier decomposition tells you *which* tier is lying before you profile blind.
- The model says scalar v0 with presieve may already land near 1.5 B/s. If it lands at 0.6, something structural is wrong (cache behavior, compiler de-optimization) — find it before stacking rungs on top.

All Phase 2 benchmarks pin to the canary-selected fastest big core. Code must remain *correct* on A55 (segment size is runtime config, never a hardcoded 64 KiB) — heterogeneous *tuning* is Phase 3's problem.

---

## PART 3 — THE ARENA

One construction-time allocation, four regions, all sizing from config (N, S = segment bytes):

| Region | Size (N = 10¹¹, S = 64 KiB) | Contents |
|---|---|---|
| Segment buffer | S bytes | the live sieve, bit=1 means candidate-alive |
| Presieve pattern replica | 1001 × 67 ≈ 66 KiB (Part 6) | cyclic source for segment init |
| Medium-tier state | ~36 B × π(√N) ≈ ~1.0 MiB | (prime, byte, j) + 8-entry Δ-tables |
| Small-tier state | compact, primes ≤ S/4 | same shape, fewer entries |

State lives in L2 and *streams* — it is touched once per prime per segment in ascending order, which is the access pattern L2 prefetch handles well. The segment itself is the only L1-resident object. Region order in the arena should place segment and pattern replica adjacent (different reuse classes: segment = read-modify-write hot, pattern = read-once-per-segment stream).

**The translation-invariance law (the structural heart of the design):** segments are 30-aligned (Phase 1, §3.4), so byte-space is *translation-invariant* — a prime's wheel walk state `(byte, j)` is valid in every segment without re-anchoring. Segment transition = `byte −= S`. **No per-prime per-segment modular arithmetic exists anywhere in the engine.** This single consequence of the alignment invariant is what makes medium-tier state viable.

---

## PART 4 — WHEEL WALK MECHANICS (The Core Derivation)

Everything in this Part is exact math on the Phase 1 tables; get it right and the inner loops contain no division, no modulo, no searching.

**Slot arithmetic, exact form.** For candidate n (coprime to 30): `byte(n) = ⌊n/30⌋`, `bit(n) = RESIDUE_TO_BIT[n mod 30]`. Both directions trivially invert. This is Convention A paying off: the byte index is just the number divided by 30 — no offset games.

**The walk.** A sieving prime p > 5 crosses numbers p·m where m runs through the coprime residues **in ascending order**: 1, 7, 11, 13, 17, 19, 23, 29, 31, … — so m's residue index j increments **+1 mod 8, always, for every prime**. There is no next-residue search; the walk is a fixed 8-cycle.

**The two per-step quantities:**

- *Bit cleared at state j:* bit(p·m) = `WHEEL_NEXT[row][j]`, where row = RESIDUE_TO_BIT[p mod 30]. The bits table is **8 rows × 8 entries = 64 bytes, shared by all primes, L1-resident forever** — per-prime bit storage is unnecessary.
- *Byte delta for the step j → j+1:* Δ[j] = ⌊(RESIDUES[WHEEL_NEXT[row][j]] + p·WHEEL_INC[j]) / 30⌋. Eight u32 values per prime, computed **once at activation**, stored inline with the prime's state.

**The sequentiality theorem (this drives the EratSmall design):** since m ascends, n = p·m ascends, and byte(n) = ⌊n/30⌋ is monotone in n — **the marking walk is monotonically non-decreasing in byte space.** Consequences: hardware-prefetch-friendly sequential store stream; multiple crossings can land in the *same byte* (Δ[j] = 0 happens whenever p·WHEEL_INC[j] < 30 — constant for small p, e.g. p = 7 has Δ ∈ {0, 1}); and for p ≲ 60 the density exceeds one crossing per byte, enabling the R1 optimization: accumulate a bit-mask per byte, store once on byte change, instead of read-modify-write per crossing.

**Activation — the zero-division result.** A prime p must cross from max(p², segment_low) upward (composites below p² have smaller factors and are already handled). Segments process in ascending order, so p activates at the *first* segment whose high ≥ p² — and at that moment low ≤ p² < high is guaranteed. Therefore **the first crossing at activation is always p² itself**: m₀ = p (coprime, no advance), `byte = ⌊p²/30⌋ − ⌊low/30⌋`, `bit = RESIDUE_TO_BIT[p² mod 30]`, `j = RESIDUE_TO_BIT[p mod 30]`. One u64 multiply (p ≤ 10⁶ → p² ≤ 10¹², fits), zero divisions. **The full-run engine π(N) contains no division in any steady-state path.** Division returns in exactly one place: mid-range `pi_range(lo, hi)` startup needs ceil(lo/p) once per prime (Phase 3's partition entry points pay this — acceptable, once per prime per range, never per segment).

Activation bookkeeping is a frontier pointer, not a scan: base primes are ascending, p² is monotone in p, so "activate all primes with p² < segment_high" is an O(1)-amortized advance of a split point between inactive and active lists.

---

## PART 5 — TIER BOUNDARIES AND THE N-DOMAIN (Derived, Not Transcribed)

Crossings per segment for prime p ≈ 8S/p. The tier structure falls out of *where per-crossing cost stops dominating per-prime overhead:*

| Tier | Bound (S = 64 KiB) | Rationale | # primes at N = 10¹⁰ / 10¹¹ |
|---|---|---|---|
| EratSmall | p ≤ S/4 ≈ 16,384 | ≥ 32 crossings/segment → unrolled batched loops, per-prime setup amortized to noise | ~1,900 / ~1,900 |
| EratMedium | S/4 < p ≤ 8S ≈ 524,288, capped at √N | 1–32 crossings → state arrays, per-segment load/compare/walk | ~7,700 / ~25,400 |
| (EratBig) | p > 8S | sometimes zero crossings/segment → per-segment iteration becomes waste → **buckets** | 0 at N ≤ 10¹¹ (√N < 8S) |

**The Phase 2 N-domain derivation — this is the quantitative justification for deferring buckets:** medium-tier cost ≈ π(√N) × (N / 30S) state-touches. At N = 10¹¹ (S = 64 KiB): 25,400 × 50,865 ≈ 1.3 × 10⁹ touches ≈ 5–8% of budget — acceptable. At N = 10¹²: 76,600 × 508,650 ≈ **3.9 × 10¹⁰ touches ≈ 90+ seconds of pure bookkeeping — catastrophic.** Buckets are not optional beyond ~10¹¹; below it, they're unneeded. **Phase 2's certified domain is N ≤ 10¹¹**, and the medium-overhead measurement (Part 10) becomes Phase 4's trigger data. This also means the *segment-size sweep optimum may differ from primesieve's 64 KiB*: our medium tier is compare-based where theirs is bucketed — a bigger S raises our medium prime count, a smaller S multiplies per-segment fixed costs. The sweep is ours to run.

**Spec amendment (file_structure.md v2.1):** §2.2's EratBig threshold "p > SieveSize" conflates bytes and numbers. The correct boundary is in **number space: p > 30·S_bytes** guarantees at most one multiple per segment; our bucket trigger is the looser and honest bound p > 8S_bytes (expected crossings < 1). Strike the unit-ambiguous sentence.

---

## PART 6 — PRESIEVE

**Pattern period derivation:** the crossings of {7, 11, 13} among wheel candidates repeat with period LCM(30, 7·11·13) = 30,030 in numbers = exactly **1,001 bytes** in wheel space. Build the 1,001-byte pattern once at construction by direct marking; every segment init is then a pattern copy instead of ~163,000 individual crossings for those three primes (8S/7 + 8S/11 + 8S/13 ≈ 163,000 at S = 64 KiB) — the 16.5% of all crossing work that is *also* the most byte-clustered and most RMW-heavy. This is the cheapest big win in the phase.

**The replica trick:** 1,001 doesn't divide S, so a naive cyclic copy wraps ~65 times per segment. Replicate the pattern **67× (~66 KiB, L2-resident)** and every segment becomes *one* linear copy of S bytes from offset `(segment_index · S) mod 1001`. One memcpy call, sequential stream from L2, ~0.3% of budget. (67 = ⌈(S + 1000)/1001⌉ guarantees any S-byte window fits.)

**Certification (Law 3, exhaustive-over-period):** pattern byte b must equal direct-marking byte b for **all 1,001 bytes**, plus segment-boundary invariance: init of segment k equals pattern at offset (k·S) mod 1001, checked at several k. Full period, not samples.

**The {17, 19} extension** (pattern period 323,323 bytes ≈ 316 KiB, L2-streamed): removes another ~0.03 crossings/number ≈ 8% of remaining crossing work, at a memcpy cost of ~0.8% of budget. Predicted net-positive but not certain (L2 pressure on the state arrays). **Rung, not default** — the ladder decides.

**Segment-0 traps — enumerate them now because each one is a silent wrong-answer:** (a) n = 1 is residue 1, byte 0, bit 0, crossed by nobody — must be explicitly cleared. (b) The pattern clears multiples of 7, 11, 13 *including those primes themselves* — segment 0 must re-set the bits of every base prime in [7, √N]. (c) No tier re-clears them: 11's crossings by p = 7 begin at 49, by p = 11 at 121 — below those, untouched. (d) Primes 2, 3, 5 live outside the wheel: the final tally adds 3 for them when N ≥ 5, and the small-N path (N < 30) is a separate exact branch, not a masked segment.

---

## PART 7 — TALLY, MASKS, AND THE API CONTRACTS

Tally = popcount of the segment (bit set = alive) with end-masking. The scalar path uses Phase 1's `count_range` (u64-word popcount); the NEON path (vcnt.16b + horizontal accumulate) is the **SIMD swap point** — same contract, certified by the Phase 1 differential harness pattern (fixed-seed patterns, all unaligned tail geometries, scalar = oracle). All `unsafe` lives inside this kernel and the copy kernel, nowhere else.

**End-of-range masking:** 10ᵏ ≡ 10 (mod 30) for every k ≥ 1 — never a residue — so *every* power-of-ten boundary ends mid-byte: bits for residues > N mod 30 in the final byte are masked via HIGH_MASK before counting. Unmasked, every unaligned N reports phantom primes (the M5 class, at every boundary). Tests must hit N mod 30 across all 30 classes and N ∈ {0..100} exhaustively.

**API contracts (frozen now — Phase 3 and 5 build on these signatures):**

- `pi(N) → u64` — exact, N ≤ 10¹¹ certified domain, division-free steady state.
- `pi_range(lo, hi) → u64` — exact for arbitrary unaligned lo, hi; masking at *both* ends, NEXT_COPRIME for start alignment, one division per prime at range init. **Invariant-tested: pi_range(a, b) = pi(b) − pi(a−1) at randomized a, b.** Phase 3's partition-then-sum architecture is *dead* if this contract wobbles; it is the most load-bearing function nobody will benchmark.
- Prime enumeration: forward bit-scan iterator over segments. Phase 2 uses it only as a correctness instrument (Part 8) but the API shape is fixed so the CLI/streaming phase costs nothing later.

---

## PART 8 — CORRECTNESS INSTRUMENTATION (Before Any Speed)

**The oracle extension — subprocess streaming protocol.** The isolation law (oracle shares no code with what it judges) is preserved by judging titan-sieve *across a process boundary*. But Termux spawn costs ~200 ms (Phase 0, H2) and T1-small is 2,001 cases — per-case spawning is 400 seconds of overhead. Therefore: the gate binary exposes a **batch protocol** — reads x-values from stdin, one per line, emits one count per line, in order; the oracle streams all cases through a *single* process. One spawn (~200 ms), then compute. The oracle gains a `--candidate-bin` mode implementing exactly this; the case set is unchanged, the truth sources are unchanged, isolation is unchanged.

**Deep-tier certification — the payoff moment:** with a subprocess candidate, the T3 tier is no longer capped at trial division's 10⁷. The sieve is certified at **10⁹, 10¹⁰, 10¹¹ against A006880, and at five randomized large x against a primesieve subprocess** — two independent truth sources at scales the honest-by-reading reference can never reach. This is the first titan code verified by the *full* triangle, and it's why the oracle was built before the engine.

**The enumeration audit — the strongest instrument in the project:** counts can coincide despite mislabeled bits; *lists* cannot. Enumerate every prime the engine finds ≤ 10⁷ (664,579 primes) via the bit-scan iterator and diff element-by-element against an independently-built simple sieve list in the test. A residue-class corruption that happens to preserve counts (compensating errors) still dies here. Any future optimization that breaks *which* numbers are prime, not just *how many*, dies here.

**Local mutants (the discriminator discipline continues):** M-mask (skip the end-of-range mask) must be caught by the N mod 30 matrix; M-restore (forget segment-0 base-prime re-set) must be caught by T1-small. If either survives the local suite, the suite grows until it doesn't.

**Edge matrix:** N ∈ 0..2000 exhaustive (T1-small via batch protocol); boundary triples p−1/p/p+1 near 10⁴, 10⁵, 10⁶; π-milestone x values; N ≡ each residue class mod 30; pi_range invariance at randomized endpoints; segment-crossing x values (x = k·30S ± 1 — segment-boundary off-by-ones live exactly there).

---

## PART 9 — THE OPTIMIZATION LADDER

R0 lands first, correctness-green, model-calibrated. Then one rung per commit, oracle-quick between rungs, ±3% keep-or-revert, every step recorded with the canary sandwich:

| Rung | Change | Model-predicted effect |
|---|---|---|
| **R0** | Scalar v0: presieve memcpy, table-driven small/medium walks, u64 popcount tally | 1.2–1.8 B/s — *calibrate against Part 2 before proceeding* |
| **R1** | EratSmall byte-batched unrolled 8-cycle: register-resident Δ-tables, mask-accumulate per byte, flush on byte change | attacks the 75% — largest single rung (+15–30% if model holds) |
| **R2** | Medium state layout: inline packed (Δ:26 bits \| bit:3 bits) u32 tables vs SoA experiment | attacks the 5–8% bookkeeping |
| **R3** | NEON tally kernel + explicit vector copy (differential-certified) | small (~1–2%) — do it for the SIMD-swap-point discipline, not the speed |
| **R4** | Presieve {17, 19} extension | +8% crossing removal vs +0.8% memcpy — measure, it's borderline |
| **R5** | Segment-size sweep 32/64/128 KiB at 10¹⁰ and 10¹¹ | optimum may differ from primesieve's 64 KiB (our medium tier is unbucketed) — this number becomes the per-cluster default in Phase 3 |

Order is deliberate: cost-proportional, per the Part 2 model. If R1's measured gain is far off model, *stop and localize* — the model just told you your crossing cost assumptions are wrong somewhere.

---

## PART 10 — BENCHMARK PROTOCOL AND THE GATE

**Protocol:** hygiene gate → canary-selected big core pinned → canary sandwich around each measurement → in-process runs (no spawn overhead) → records. **Two columns, always:** burst = 10¹⁰, min-of-5 (fits inside the 14.5 s thermal cliff); sustained = 10¹¹, single long run, canary-normalized (crosses the cliff; expect the ledger's ~0.45 derate on the tail). Also: run primesieve single-threaded under benchref in the same session for the session-fair ratio — cross-session comparisons are exactly what Phase 0 was built to forbid.

**The Phase 2 Gate (all green before Phase 3):**

| # | Criterion |
|---|---|
| 1 | Oracle full mode, batch protocol, sieve-as-candidate: **exit 0**, all tiers including deep T3 at 10⁹/10¹⁰/10¹¹ |
| 2 | Randomized-x differential vs primesieve subprocess: bit-exact (5+ points in [10⁹, 10¹¹]) |
| 3 | Enumeration audit ≤ 10⁷: element-wise exact |
| 4 | pi_range invariance + edge matrix + N mod 30 matrix green |
| 5 | Local mutants M-mask, M-restore both killed |
| 6 | Zero-alloc tripwire: delta = 0 across steady-state gauntlet |
| 7 | **Burst ≥ 1.5 B/s** and **sustained (normalized) ≥ 1.5 B/s** at 10¹¹, single big core |
| 8 | Measured vs Part 2 model within 2×, per-tier decomposition recorded — else anomaly localized and explained |
| 9 | Segment sweep recorded; per-config medium-overhead measurement recorded (Phase 4 trigger data) |
| 10 | Ladder ledger complete (every rung: commit, before/after rates, keep/revert verdict); gate record in `bench/records/titan_sieve_gate.json` |

---

## PART 11 — DECISION MAP AND SPEC AMENDMENTS

**Where Phase 2 outputs flow:** pi_range contract + per-cluster segment geometry + the measured per-tier rates → Phase 3's partitioner and heterogeneous scheduler; medium-overhead number + the N-domain boundary math → Phase 4's bucket implementation threshold; the certified-at-10¹¹ engine → Phase 5's Lehmer, which needs a π-table source and a base-sieve anyway (the same
