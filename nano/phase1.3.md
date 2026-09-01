# Phase 1 Engineering Specification — `titan-core`: The Correctness Substrate

Phase 0 closed exactly right: H2 proven (≈200 ms Termux fork+linker overhead — which means every ledger row for sub-second children must forever carry the overhead-subtracted "internal compute" annotation you've now established), the 14.5 s thermal cliff and the 9.15 B/s best-config summit are frozen as Phase 2/3 inputs. Phase 1's character is the opposite of Phase 0's: no thermometers, no oracles, no silicon — **pure mathematics, frozen at compile time**. Nothing in titan-core may allocate, may depend on anything, may vary between runs. Every engine above it inherits its exactness or its errors, invisibly. That asymmetry dictates everything below.

---

## PART 1 — MANDATE, BOUNDARIES, AND THE FIVE LAWS

**Scope (exactly four files):** `roots.rs`, `wheel.rs`, `phi_tiny.rs`, `bit_array.rs`, plus `lib.rs` wiring. **Explicitly excluded:** any sieving (Phase 2), any SIMD kernel (Phase 2, as the differential target), any threading (Phase 3), any counting algorithm beyond Φ-tiny (Phase 5), and **wheel-210 (deferred — amendment §3 below)**.

titan-core is the layer where the project's epistemology compresses into crate law:

1. **Purity** — zero runtime dependencies, `no_std`-compatible core. `libc` and friends belong to titan-bench/titan-pool, never here.
2. **Constancy** — every lookup table is generated at *compile time* and lives in rodata as a `static`. Nothing is built, sized, or tuned at runtime. Consequence: zero init cost, zero allocation, bit-identical behavior across every architecture and every build.
3. **Exhaustive-or-bust** — anything periodic or exhaustible is tested over its **full period**. Random sampling is for unexhaustible domains only, and always paired with boundary sweeps and invariant checks. A structure with a 30,030-entry period gets all 30,030 entries tested, not 30,000 random ones.
4. **Guarded powers** — every `r^k` comparison in root code evaluates in **u128**, no exceptions, because `(r+1)²` overflows u64 exactly at the domain's upper edge where the danger lives.
5. **Single truth per semantic** — the wheel layout, the prime constants, each exist in exactly one definition in this crate. The oracle's copies stay separate by the isolation law; duplication across crates is deliberate and documented, never unified "for convenience." One shared constant module between oracle and engine is a single point of corruption for the entire verification system.

And one discipline carried down from the oracle: **every module's test suite must demonstrably kill a deliberately-broken variant of itself.** A test suite that has never failed anything has proven nothing. Each PART below names its mutant.

---

## PART 2 — `roots.rs`: Integer Roots, Exact Over All of u64

### 2.1 Why this file exists at all

seive.md died of `pow(x, 1.0/4)`. The mechanism, precisely: f64 has a 53-bit mantissa; every u64 above 2⁵³ rounds on conversion, hardware `sqrt`/`cbrt` round again, and the combined error lands the seed on *either side* of the true floor. The root functions in Lehmer/Gourdon partition the entire search space (`a = π(x¼)`, `b = π(x½)`, `c = π(x⅓)`) — an off-by-one in a root produces a wrong count **only at scales beyond any exhaustively-testable range**. This is not a hypothetical: it is the M4/scale-fraud defect class wearing a math library's clothes. roots.rs is where that class is made structurally impossible.

### 2.2 The contract

| Function | Postulate (must hold for ALL u64 x) |
|---|---|
| `isqrt(x)` | r² ≤ x < (r+1)² — **both inequalities** |
| `icbrt(x)` | r³ ≤ x < (r+1)³ |
| `iroot4(x)` | r⁴ ≤ x < (r+1)⁴ |
| `iroot(x, k)` k∈2..=63 | rᵏ ≤ x < (r+1)ᵏ |
| Edge identities | x=0 → 0; x=1 → 1 for all k; isqrt(u64::MAX) = 4294967295 (since (2³²−1)² = 2⁶⁴−2³³+1 ≤ 2⁶⁴−1 while (2³²)² overflows — the case that *forces* the u128 guard) |

**Spec amendment #1:** file_structure.md demands "branchless" roots. Wrong requirement — roots are called O(1) times per π(x) query, never in a per-element loop, so A55's division latency and branch costs are irrelevant here; the correction branches run at most 1–2 iterations. The requirement is **totality and exactness over the full domain**, and the design below buys it cheaply. Amend the spec: "exact, guarded, total" replaces "branchless."

### 2.3 The algorithm and why

**Float seed + two-sided correction, all power math in u128.** Seed r from hardware `sqrt`/`cbrt`/`powf` (the seed only needs to be *close*, never right), then walk down while rᵏ > x and up while (r+1)ᵏ ≤ x, computing every comparison in u128. Seed-quality analysis says the loops run ≤ ~2 iterations: for x near u64::MAX, f64 conversion error ≤ 2¹¹, and propagated through 1/2, 1/3, 1/4 exponents the root-seed error is under 2 — but *analysis is not proof*; the test matrix in §2.4 is the proof, the loops are the insurance, and the u128 guard is the load-bearing wall that makes "≤ 2 iterations" survivable even when the analysis is wrong somewhere.

Fallback for `iroot(x, k)` general-k: integer Newton on u128 with the same final two-sided correction — no float seed for exotic k, because nobody has audited float seed quality at every k, and unaudited paths in a correctness substrate are forbidden.

### 2.4 The test matrix (the actual deliverable)

The danger zone for root functions is **x within ±1 of an exact power** — that's where a floor flips. The matrix attacks exactly there, in three zones:

| Zone | isqrt | icbrt | iroot4 |
|---|---|---|---|
| Exhaustive boundary | r ≤ 2¹⁸: test r²−1, r², r²+1 (786K cases) | r ≤ 2,642,245 (covers u64): r³−1, r³, r³+1 — **full domain boundary sweep**, ~8M cases, full-gate mode | r ≤ 65,535 (covers u64): r⁴−1, r⁴, r⁴+1 — **fully exhaustive, always** |
| High-range LCG | r sampled near 2³² (beyond f64 exactness) with fixed-seed LCG, ±1 around each square | same near 2,642,245 | n/a (exhausted) |
| Invariant check | Every tested x: assert rᵏ ≤ x < (r+1)ᵏ computed independently in u128 — the postulate itself is the oracle | same | same |

**The mutant:** "M-root" — the float seed *without* correction loops. The boundary sweep must catch it (somewhere above 2⁵² it flips). If it doesn't, the matrix is insufficient and you add zones until it does. This is the module proving its own teeth.

---

## PART 3 — `wheel.rs`: The Mod-30 Law, One Convention, Zero Ambiguity

### 3.1 The convention war, resolved

file_structure.md currently contains **two different wheels**: the residue list {1,7,11,13,17,19,23,29} and a bit table mapping bit0→+7 … bit7→+31 (which is a *different* layout — one where bit 7 wraps into the next byte's residue-1 slot). Both are individually valid; mixed, they are the M6 defect class — a silent residue-class corruption that passes small tests and eats primes forever. Phase 2 builds directly on this file, so the ambiguity dies **now**:

**Ruling: Convention A.** Byte k covers integers [30k, 30k+29]. **Bit i ↔ residue `RESIDUES[i]`**, with `RESIDUES = [1,7,11,13,17,19,23,29]` in ascending order. Defense: (a) number↔slot round-trip is trivially auditable (`n = 30k + r ↔ byte k, bit RESIDUE_TO_BIT[r]`); (b) this ordering matches every literature presentation of the mod-30 wheel, so cross-referencing Gourdon/Lehmer formulas and primesieve's algorithmic papers never requires a mental layout translation; (c) Convention B's wrap-around bit is precisely the confusion vector. **Corollary rule:** when consulting primesieve's *source* for algorithmic insight, re-derive every layout detail under our convention — never transcribe. Transcription across conventions is how M6-class bugs enter codebases.

### 3.2 The complete const-table artifact list (the file's real content)

| Table | Shape | Meaning | Consumer |
|---|---|---|---|
| `RESIDUES` | [u8; 8] | bit i ↔ residue, Convention A | everything |
| `RESIDUE_TO_BIT` | [u8; 30] | residue → bit index, sentinel for non-coprime residues | number→slot; kills all runtime mod-30 probing |
| `WHEEL_INC` | [u8; 8] | additive gap from residue i to next: **[6,4,2,4,2,4,6,2]** (sums to 30 — a build-time asserted invariant) | marking loops, presieve |
| `WHEEL_NEXT` | [u8; 8]×[u8; 8] | `[bit(p·RESIDUES[j] mod 30)]` — see §3.3 | erat_medium stepping |
| `NEXT_COPRIME` | [u8; 30] | smallest coprime residue ≥ r (for arbitrary r) | segment-boundary "advance m to wheel" at every sieving-prime insert |
| `HIGH_MASK` | [u8; 8] | bits ≤ i set | end-of-range masking, §3.5 |

All generated by const fn at compile time, with const-asserted invariants (gaps sum to 30; `WHEEL_NEXT` rows are permutations of 0..8 — see §3.3 for why they must be).

### 3.3 The deep math: how multiples of p walk the wheel (the part Phase 2 lives on)

A prime p > 5 has its *relevant* multiples at p·m where **m itself is coprime to 30** (any other m makes p·m divisible by 2, 3, or 5 — already pre-sieved). So the marking loop for p walks m through the 8 residues, and two things happen per step, both table-driven:

- **Integer delta:** consecutive m-residues differ by the additive gap → the multiple jumps by **p × WHEEL_INC[j]** (worked check, p=7: multiples 7,49,77,91,… deltas 42,28,14 = 7×[6,4,2] ✓).
- **Residue of the product:** (p·m) mod 30 = (p mod 30)·(m mod 30) mod 30. Multiplication by a unit is a **bijection** on the 8 residues — so each prime's multiples visit a *permutation* of the 8 residues (p=7: 7,19,17,1,29,13,11,23 ✓). That's why `WHEEL_NEXT` rows must be permutations: it's a mathematical identity, and const-asserting it turns a theorem into a build gate.

**The consequence — the marking inner loop contains zero division and zero modulo.** At init, each sieving prime precomputes its own 8-entry delta table (p × gap) and 8-entry next-bit table (one mod per entry, once). Phase 2's cross-off loop is then pure: index arithmetic + table lookups + bit clears. This is the entire point of wheel.rs: pay the modular arithmetic once, at setup, never in the loop.

### 3.4 Segment alignment invariants (binding on Phase 2, stated here because wheel.rs owns them)

- Segment low is always a **multiple of 30**; segment span is bytes × 30. Slots therefore align identically at every segment boundary — no cross-boundary residue arithmetic may ever exist.
- Sanity constant for Phase 2's tally checks: a 32 KiB segment spans 983,040 numbers; near 10¹⁰ expect ≈ 42.7K primes per segment (983,040 / ln 10¹⁰). A tally far from this flags a wheel or counting defect immediately.

### 3.5 The end-of-range mask law

10ᵏ ≡ 10 (mod 30) for every k ≥ 1 — never a wheel residue — so **every power-of-ten range ends mid-byte** and the final byte's invalid high bits must be masked via `HIGH_MASK` before counting. Unmasked, every unaligned N reports phantom primes: an M5-class domain defect at *every* boundary. Tests must hit N = 10ᵏ±1 and every residue class of N mod 30.

### 3.6 Test matrix and mutant

Exhaustive round-trip for all n < 30×10⁶ (coprime n: slot↔number round-trip; non-coprime: conversion must reject); **prime-containment invariant** — every prime > 5 has residue ∈ RESIDUES, checked against a local trial-division list to 10⁶ (this is the invariant M6 violates; M6 already lives in the oracle and this test is its unit-level twin); gaps-sum-30 and permutation const-asserts; a scalar wheel-sieve built *on this file alone* counting π(7919) = 1000 exactly — the first primes the engine ever finds are found through titan-core's own semantics.

---

## PART 4 — `phi_tiny.rs`: Φ(x, k) in O(1), Sized to the Silicon

### 4.1 The identity and the design it forces

Φ(x, k) — integers ≤ x divisible by none of the first k primes — is exactly periodic: **Φ(x, k) = ⌊x/P_k⌋·φ(P_k) + Φ(x mod P_k, k)**, where P_k is the primorial. One division, one multiplication, one table lookup. The entire engineering problem is *which k get flat tables*, and the answer is dictated by two hard numbers:

| k | P_k | φ(P_k) — max table value | Flat table size (u16) |
|---|---|---|---|
| 5 | 2,310 | 480 | 4.6 KiB |
| **6** | **30,030** | **5,760** | **58.7 KiB** |
| 7 | 510,510 | **92,160 > 65,535** | u16 impossible; u32 → 2.04 MiB |

**φ(P₇) = 92,160 exceeds u16 — that is the proof** that k = 6 is the flat-table ceiling (spec amendment #2: file_structure.md's "k ≤ 8 fits in tiny totient tables" is wrong by 16× on memory). Total flat footprint k ≤ 6 ≈ **63.7 KiB — almost exactly the A76's 64 KiB L1D**, and ~2× the A55's 32 KiB. This is a *feature to be aware of, not a bug*: PhiTiny belongs to titan-count (Phase 5), not the sieve, so it doesn't compete with segment arrays; but the Phase 5 cache plan must know phi queries on little cores pay an L2 trip. Design response: the flat depth is a compile-time constant — **default k = 6 flat** (φ queries sit directly in Lehmer's hot recursion; halving memory to k = 5 flat doubles query cost for no current need), with the k = 5-flat + recursion variant documented as the fallback if Phase 5 profiling shows little-core L2 pressure dominating.

For k = 7, 8: **one recursion level** via Φ(x, 7) = Φ(x, 6) − Φ(x/17, 6) and Φ(x, 8) = Φ(x, 7) − Φ(x/19, 7) — 2 and 4 flat queries respectively, saving 2+ MiB of tables. Hard cap K_MAX = 8; beyond that is titan-count's strategy problem, not titan-core's.

### 4.2 Overflow proof (do it once, in the docs, with numbers)

Worst case k ≤ 6: ⌊x/P₆⌋·φ(P₆) at x = u64::MAX ≈ 6.15×10¹⁴ × 5,760 ≈ 3.5×10¹⁸ < 2⁶⁴ ✓. k = 1 identity Φ(x,0) = x and Φ(x,1) = x − x/2 ≈ 9.2×10¹⁸ ✓. All u64-safe; the test suite re-proves it mechanically via u128 cross-checks anyway, because proofs in docs rot and u128 asserts don't.

### 4.3 Verification: the exhaustive-over-period standard, fully enforced

Law 3 applies with teeth here because the periods are *cheap to exhaust*: build a sieve-based reference (mark non-coprimes to the first k primes over [0, P_k), prefix-sum — auditable by reading), then compare the flat table at **every one of the P_k entries** for every k ≤ 6. That is 30,030 + 2,310 + … exhaustive points — not sampling, *certification*. For k = 7, 8: exhaustive over P₇ = 510,510 via the recursion identity checked against the same sieve reference (the reference build is O(P₇ log log P₇), trivial). Plus periodicity identity checks at LCG-sampled large x (Φ(x + P_k, k) = Φ(x, k) + φ(P_k)) and boundary points x = m·P_k ± 1.

**The mutant:** drop the `x mod P_k` reduction (use x directly as index) — must be caught by the periodicity test at the first sampled x ≥ P_k. If the suite can't catch a missing modulo, it can't certify the identity the whole module is built on.

---

## PART 5 — `bit_array.rs`: Mechanics Without Semantics

### 5.1 The layering decision

The sieve segment will be a byte array whose bits mean "wheel candidate at 30k+r is still prime." Split that: **wheel.rs owns what bits mean; bit_array.rs owns how bits behave.** This file is a *borrowed view* (a `BitWindow` over a `&mut [u8]`), never an owner — allocation policy belongs to Phase 2's segment arena, which is what "zero-allocation" actually means operationally: all allocation at arena construction, zero thereafter, enforced by tripwire (§5.3).

### 5.2 The contract

- Bit-level: get/set/clear by index within the window; indices u32 (a 32 KiB segment holds 262,144 bits — u32 is safe with enormous margin).
- **`count_range(lo, hi)`** — the tally primitive and the performance-critical surface: u64-word-wise `count_ones` over the aligned interior, byte/halfword tails masked. This exact function signature is the **SIMD swap point**: Phase 2's NEON kernel implements the same contract, and a permanent differential test (fixed-seed LCG byte patterns, both backends, assert equal counts, including all unaligned tail geometries) makes the scalar path the forever-oracle for the vector path. One interface, two backends, bit-exact equivalence — the Phase 2 optimization ladder inherits the M-corpus epistemology.
- `mask_above(last_valid)` — end-of-range masking, married to wheel's `HIGH_MASK` semantics.
- Invariant: ranges clamp to window bounds in debug; release assumes callers obey (documented).

### 5.3 The zero-alloc tripwire

A counting global allocator (an AtomicU64-wrapping wrapper, offered as an opt-in type for binaries and test harnesses to install as `#[global_allocator]`). The Phase 1 gate installs it and asserts **allocation-delta = 0** across a steady-state gauntlet (10⁶ bit ops, 10⁵ phi queries, root calls, mask/count cycles). titan-core passes by construction — const tables, borrowed views — and the tripwire exists to *keep it true* as the crate grows. This is the regression boundary for the "zero-alloc engine" claim: the claim is only real if a machine failure mode exists for violating it.

---

## PART 6 — THE PHASE 1 GATE

| # | Criterion | Evidence |
|---|---|---|
| 1 | Roots: full boundary matrix (isqrt r≤2¹⁸ exhaustive, icbrt **full-domain** r≤2,642,245, iroot4 **fully exhaustive**) + u128 invariant asserts, all green | test log |
| 2 | M-root mutant (uncorrected float seed) **killed** by the matrix | test log |
| 3 | Wheel: Convention A tables const-generated; gaps-sum-30 and `WHEEL_NEXT`-permutation asserts pass at build time | build + test log |
| 4 | Wheel: 30×10⁶ exhaustive round-trip + prime-containment invariant + π(7919) = 1000 via wheel-only scalar sieve | test log |
| 5 | PhiTiny: **exhaustive over full periods**, all k ≤ 8, vs sieve-built reference; periodicity + overflow u128 checks green | test log |
| 6 | Phi-mutant (missing mod reduction) killed | test log |
| 7 | BitWindow: round-trip, count-vs-scalar differential, all tail geometries green | test log |
| 8 | Zero-alloc gauntlet: allocation delta = 0 | test log |
| 9 | All tables compile-time `static`; crate has zero runtime deps; report binary's rodata table sizes (~63.7 KiB expected) | build artifact |
| 10 | Gate record written to `bench/records/titan_core_gate.json` | record |

Quick/full test modes mirror the oracle: quick runs the exhaustive-under-a-second tiers, full runs the complete matrix (icbrt full-domain sweep dominates, ~seconds on an A76). `cargo test` green on-device is the merge criterion — oracle-law discipline, one level down the stack.

---

## PART 7 — SPEC AMENDMENTS & THE DECISION MAP

**Amendments to record in file_structure.md v2:** (1) "branchless roots" → "exact, guarded, total roots" (§2.2); (2) PhiTiny "k ≤ 8" → "k ≤ 6 flat (u16 ceiling proven by φ(P₇)=92,160), k ≤ 8 via one recursion level" (§4.1); (3) **wheel-210 deferred**: 48/210 = 22.9% candidate density vs 8/30 = 26.7% — a 14% marking-density gain for 6× the table complexity and worse locality, while our own Phase 0 data says the binding constraints are thermal scheduling and memory behavior, not marking density. Revisit only if Phase 2 profiling proves density dominates; (4) §2.1's bit table superseded by Convention A — the old table must be *struck from the doc*, because a stale second convention in the spec is how the M6 class re-enters through documentation.

**Decision map — where Phase 1 outputs flow:** roots → titan-count's partitioning (the exact a/b/c that killed seive.md) and titan-sieve's segment/bucket bounds; wheel tables → presieve patterns, all three erat tiers' inner loops (§3.3's zero-division marking), NEXT_COPRIME at every bucket insert; PhiTiny → the φ-recursion base case that makes Lehmer terminate in tables instead of hash-cache thrash; BitWindow + counting allocator → every segment and every future zero-alloc claim; the differential-test pattern (scalar-oracle vs SIMD-candidate) → the template for every optimization commit in Phase 2's ladder.

Four files, one law each: roots are exact everywhere or nowhere, the wheel speaks one convention, Φ's tables certify their own periods, bits carry no meaning they weren't given. Translate it — and when the gate record lands, paste the test output and the rodata sizes; then Phase 2 opens with a scalar segmented sieve that has no excuse left to be wrong, and every reason to be made fast.
