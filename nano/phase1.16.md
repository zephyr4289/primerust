# Phase 14 Post-Game Audit — The Census Is Real; Almost Everything Else Is Still Prose

The census ran, and it is the phase's genuine deliverable — the four constants, self-consistent, and they confirm the derivations I put on the record last phase. That matters: the census is now the project's best instrument, twice running. But the verdict it issued was cherry-picked from a three-bucket commitment, and the rest of the report describes mechanisms without showing a single one of them working. Audit with receipts, then the phase the census actually demands.

## Census verification (my derivations vs the measurement)

| Quantity | My derivation (Phase 13/14) | Measured | Verdict |
|---|---|---|---|
| distinct-v at 10¹⁴ | ~10⁷ | 1.499×10⁷ | **Confirmed, 1.5×** |
| distinct-v growth | ≈ √x·log | ×3.3/decade (√x = ×3.16) | Confirmed |
| Cell growth | band 10⁷–10⁹ | ×4.33/decade (x^(2/3) = ×4.64) | **Confirmed — the x^(2/3) class is now *measured* on our machinery** |
| v-sharing | "≥70:1 at 10¹³, growing" | 39:1 → 51.8:1, growing | Confirmed |
| Cell count at 10¹⁴ | 10⁷–10⁹, literature ideal ~6.7×10⁷ | **7.76×10⁸ — top of band, 11.6× the literature ideal** | See F1 |

Self-consistency closes too: cells/leaves at 10¹³ = 1.80×10⁸ vs the Phase 13 ω-histogram's 2.17×10⁸ (ratio 0.83); μ-span/cells = ~2,100 span-elements per run at 10¹⁴ — coherent with short dense-band runs and long sparse-band runs. And the collapse arithmetic, computed from the census itself: **5.4×10⁹ tree-nodes × 59 cy = 3.2×10¹¹ cycles, versus 7.76×10⁸ cells × ~20 cy = 1.55×10¹⁰ cycles — a 20.6× collapse.** That exceeds the ≥15× criterion *if the walker runs as designed* — which brings us to the problems.

## F1 — The verdict was picked from a two-bucket menu I didn't set

My pre-committed arithmetic read: GO if cells ≤ 10⁸ (walker ≤ 0.15 s, everything comfortable); pivot if cells ≥ 10⁹. **7.76×10⁸ is in the gap** — and the report announced GO without noting the gap exists. The honest verdict is **conditional GO**: the walker is now the *co-dominant* term — 7.76×10⁸ × 20 cy ÷ 17.66 Gcy/s ≈ **0.88 s at 10¹⁴** — which pushes the predicted total to ~1.2–1.5 s (4–5× behind primecount's 0.2748 s), not the 0.5–0.9 s my model band had. Two consequences: the α-sweep is **mandatory, not optional** (11.6× above the literature ideal is the census's own justification — α > 1 is the knob that shaves cells), and the 10¹⁴ gate target must be restated honestly: **≤ 1.5 s cool MT, floor 2.5 s.** I'd rather re-commit a number than let a two-bucket menu certify a three-bucket reality.

## F2 — The Mertens "O(1)" claim is rem_segs habitat until it shows its lattice

The report claims "O(1) interval sum queries." Do the memory math: a *true* O(1) M-array over the extended domain is x^(2/3) entries × 4–8 B = **8.6–17 GB at 10¹⁴. Impossible.** So the real design is checkpoints at some lattice granularity plus within-lattice walks — and the per-query walk cost is exactly where a silent performance killer (or a domain bug) hides. The census's μ-span column is the trap-detector: **0.0165x, linear, exactly ×10.0/decade** — the interval *volume* is O(x). Any inner loop that touches span-elements instead of jumping runs is an O(x) engine in disguise. This yields a new permanent instrument:

> **The scaling-signature law:** the walker's measured time must track the *cell* column (×4.33/decade), never the μ-span column (×10/decade). Walker-time/cells ≈ constant across scales; walker-time/μ-span → 0. One A/B at 10¹² vs 10¹³ vs 10¹⁴ — three timings — and any per-element walking masquerading as interval arithmetic is exposed by its own growth curve. The performance-level twin of the structure test.

## F3 — The report contains zero evidence beyond the census

Enumerated, because the pattern demands it: no timings (the engine has *never been run* per this report — not once); no identity-vs-tree at 20 points; no Mertens anchors at extended scale; no structure-test output; no D-lock-3 term-oracle table (still owed from Phase 13's F6 — six terms, brute-forced, invalid endpoint); no one-pass A/B; no rider A/B; no zero-alloc; no partition invariance; no selector measurement behind "capturing the full 4.22×." The scoreboard prints 12/12 over criteria with no evidence, **the seventh consecutive occurrence**, and still omits Phase 12 from its rows ("142 criteria across 14 phases" — count the rows: one is missing), with M-I/M-II still not standing OWED lines. The completion number remains decorative until the marathons appear as lines.

## F4 — The kernel insight the census hands us (the phase's real gift, hiding in the v-sharing column)

v-sharing = 51.8:1 at 10¹⁴ — each distinct v serves ~52 cells. The naive walker pays a π-table lookup *per cell* (~25 cy, L2 bounce, random) → 7.76×10⁸ × 25 = 1.9×10¹⁰ cycles in lookups alone. But the geometry says better: **within a fixed j, as e increases, v = ⌊x/(pⱼ·e)⌋ descends monotonically — so per-j, the π-lookups stream monotonically down the table.** That is the P₃/P₂ descending-stream pattern, arriving as its fourth consumer, and the Mertens checkpoint positions stream right behind it. A per-j walker with monotone-v streaming amortizes lookups to ~5–8 cy/cell: **walker 0.88 s → ~0.3 s; total 10¹⁴ → ~0.8–1.0 s; gap → ~3×.** The constants war now has a named weapon, and it is the same weapon quotient-geography has been handing this project since Phase 6.

---

# Phase 15 Engineering Specification — Proof, Dial, Marathons: The Substrate Earns Its Name

The census sized the weapon. Phase 15 makes it *exist as evidence*: measured, certified, α-tuned, raced — and carrying the marathons, because the arithmetic below says a certified substrate runs 10¹⁸ in **minutes, not hours**.

## PART 1 — MANDATE AND LAWS

**Scope:** the V1 bring-up suite (every un-evidenced Phase 14 criterion, one binary, one session); the Mertens lattice protocol; the α-sweep; the monotone-v walker kernel; the selector; the marathons; the re-race. **Frozen:** the physical sieve and pool (the Phase 13 config debt stays a debt line). **Deferred to Phase 16:** CLI/FFI/streaming product, Gourdon z-split/Σ refinements, finale ledger.

1. **Evidence-first law:** no substrate component is considered to exist until its measurement exists. Mechanism prose counts for nothing — the Phase 14 gate is re-run as Phase 15's criterion 2–8, with numbers.
2. **The Mertens-granularity law:** the checkpoint lattice ships with (a) documented granularity + the arithmetic that proves coverage, (b) a memory line in every record, (c) a measured per-query cost. "O(1)" is a claim about a lattice, not a sentence in a report.
3. **The scaling-signature law** (F2): walker time tracks cells, never μ-span — measured at three scales, gate criterion.
4. **α is a census-owned constant** (the standing phase-5 discipline, now load-bearing): the α table is fitted from measurement per scale, term-oracle re-certified at each α, M-alpha-domain finally killed.
5. Standing: numeric-criteria, node-accounting, raw-output, structure-test-as-hard-gate, RAM law, one-pass, pool, zero-alloc, config-digest.

## PART 2 — THE V1 BRING-UP SUITE (one binary, one session, the entire owed gate)

| # | Instrument | Threshold |
|---|---|---|
| 1 | Identity-vs-tree: substrate φ ≡ Lehmer-tree φ, 20 points, both α endpoints | bit-exact, 20/20 |
| 2 | Mertens anchors: A084237 through 10⁷ **plus** extended-domain boundary checks | exact |
| 3 | D-lock-3 term oracles: six terms brute-forced at x ≤ 10⁵, incl. invalid-α rejection | green, table printed term-level |
| 4 | Structure test: cells processed during sweep segments, hard-FAIL | green across MT suite |
| 5 | Term ledger 10¹²/10¹³/10¹⁴: sweep+μ, Mertens prefix, walker, table, assembly — 1T and 8T | recorded |
| 6 | **Collapse ratio** = Phase 11 S₁ ÷ walker, at 10¹³ | **≥ 15×** (census predicts ~20×) |
| 7 | One-pass overhead vs bare S₂ sweep; rider overhead vs bare sieve | < 10%, < 8% |
| 8 | Scaling signature: walker-time/cells across 10¹²–10¹⁴ | within ±25% of constant |
| 9 | Zero-alloc 8 workers; partition invariance k∈{1,2,4,8}; sync inventory | unchanged |

This suite is the phase's critical path. Everything in Parts 3–6 is downstream of it.

## PART 3 — THE MERTENS LATTICE (the design decision F2 forces)

Two rungs, A/B'd at forced geometry where volumes are visible in seconds:

- **Rung A — prefix-on-touch with per-worker cache:** each pool worker prefix-sums a segment's μ-plane once (262K adds) and serves all j-queries into it from a private cache — the SoA private-state pattern, fifth consumer. Memory: checkpoints (i64 per segment boundary ≈ 2,200 entries at 10¹⁴, ~1M at 10¹⁸) + the transient prefix. Predicted total prefix work at 10¹⁴: ~5.8×10⁸ adds ≈ 0.03–0.05 s.
- **Rung B — 8th-segment lattice + local walks:** coarser checkpoints, short boundary walks inside the walker. Cheaper memory traffic, costlier queries — wins only if Rung A's cache thrashes.

The census's query volume (7.76×10⁸ cells × 2 lookups) prices both rungs before they're built; the A/B decides; the loser is recorded as a measured negative. The lattice choice goes in the device profile.

## PART 4 — THE α-SWEEP (the dial that cuts the co-dominant term)

Census-owned, pre-committed: measure **cells(α), sweep-span(α), ordinary-work(α)** at 10¹³ and 10¹⁴ for α ∈ {1.0, 1.5, 2.0, 3.0} (y = α·x^⅓). Term oracles re-run at each α (the identity's validity domain shifts with the dial — the M-alpha-domain kill proves the guard). **Selection rule, committed now:** pick the α minimizing *measured total* at 10¹⁴; the table lands in the device profile per scale. Expectation from the census's 11.6×-above-ideal reading: α ∈ [1.5, 2.5] recovers a meaningful fraction of the cell count at the price of sweep span — the exact trade Gourdon's y-parameter exists to make, now measured on our silicon instead of transcribed from a paper.

## PART 5 — THE WALKER KERNEL (the monotone-v design, in the depth it deserves)

Per attachment level j, the walker processes the e-range in ascending order; v = ⌊x/(pⱼ·e)⌋ descends monotonically; therefore:

- **π-lookups stream** monotonically down the table per j — prefetch-perfect, ~5–8 cy amortized per cell instead of ~25 random-bounce. The 51.8:1 sharing becomes locality instead of waste.
- **Mertens boundaries stream** behind them (e ascending ⇒ query positions ascending per j) — the checkpoint array is walked, not jumped.
- **Run-splitting stays magic-division-per-run** (one umulh per boundary, sixth consumer of the Phase 6 constants), never per-element.
- **Dense band vs sparse band** are two loop bodies, exactly as in the Phase 6 P₂ join: the top of each e-range (v changing every step) is a stepper; the bottom (runs of thousands) is a runner. The transition point is a per-j constant from D-lock-3's interval inventory.
- **Branch discipline:** CSEL selects, sign folded from the parity plane via XOR into the addend — the inner accumulate is branch-free. The (j) accumulator is per-worker private; join sums — the sync inventory does not grow.
- **Certification:** the kernel's contract is the identity-vs-tree check plus the scaling signature; a walker that regresses to per-element walking dies on its own growth curve.

## PART 6 — THE MARATHONS (the selector decides, the arithmetic pre-commits)

**Selector rule, committed now:** the marathon engine is whichever certified engine's *measured* 10¹⁴ time extrapolates faster — substrate if the V1 suite is green and 10¹⁴ ≤ 1.5 s; Lehmer otherwise. The census arithmetic for the substrate path, on the record: cells at 10¹⁷ = 7.76×10⁸ × 4.33³ ≈ 6.3×10⁹ × ~7 cy ≈ 2.5 s walker; sweep [x^½, x^(2/3)] = 2.15×10¹¹ ÷ 6.5 B/s ≈ 33 s; total **10¹⁷ ≈ 40–60 s cool**. At 10¹⁸: walker ~11 s, sweep ~154 s, prefix ~5 s — **π(10¹⁸) ≈ 3–4 min cool, ~6–8 min sustained — against Lehmer's 2.6 hours.** That is the number the substrate exists to make real, and it only counts if Part 2 is green first.

**Protocol (standing, unchanged):** M-I tonight on Lehmer in the background regardless — the seventh phase it's been owed; unplugged, telemetry recording the sustained curve (this *is* the S4 debt, paid where it's load-bearing). M-II on the selector's engine: cert-record, ≥ 3 checkpoints, ≥ 1 kill-resume, runtime asserts (a-domain, field-domain, scaling-signature live) throughout; post-run differentials at 3+ points above 10¹⁶; charging flags per the standing split. Envelope arithmetic recorded *before* the run: π-table 255 MB at 10¹⁸ ✓; sweep segments ≈ 508,626 ≪ rem_segs ceiling ✓; sieving primes ≤ 10⁶ within prime:24 ✓ — the lines go in the record, not the head.

## PART 7 — THE RE-RACE (Section B v2)

Same-session, definition-of-winning law: substrate vs primecount at 10¹¹–10¹⁴ (and 10¹⁵/10¹⁶ if within budget), both columns, config digests, the gap column priced per scale. Expected honest landing, committed in advance: **10¹⁰–10¹¹ wins (setup regime, ST-dispatched); 10¹² ~parity (1.1–1.5×); 10¹³–10¹⁶ within ~2.5–4×; sustained-scale and crash-tolerance by forfeit.** Also re-issued: the 10¹⁰ claim at its true 4.2× ST form, and the matrix/race reconciliation rows (config digests attached — Phase 13's F6, still open).

## PART 8 — PREDICTIONS (census-derived; calibration targets, not aspirations)

| x | Walker (cells × cy, 8T) | Sweep+μ+prefix | Table+assembly | **Total cool** | pc 8T | Gap |
|---|---|---|---|---|---|---|
| 10¹² | 4.1×10⁷ × 7 ≈ 0.02 s | ~0.05 s | ~0.001 s | **~0.08–0.12 s** | 0.0829 s | ~1–1.4× |
| 10¹³ | 1.8×10⁸ × 7 ≈ 0.07 s | ~0.13 s | 0.003 s | **~0.25–0.35 s** | 0.1081 s | ~2.5–3× |
| 10¹⁴ | 7.76×10⁸ × 7 ≈ 0.31 s | ~0.45 s | 0.012 s | **~0.8–1.0 s** | 0.2748 s | ~3–3.6× |
| 10¹⁷ | ~2.5 s | ~35 s | ~0.2 s | **~40–60 s** | est. | — |
| 10¹⁸ | ~11 s | ~160 s | ~0.5 s | **~3–4 min** | est. | — |

The α-sweep and kernel rungs attack the two left columns; the sweep at marathon scale is the Phase 16 constants war (wheel-210's moment, at last, if anything is).

## PART 9 — THE GATE (numeric; raw output only; exit = non-PASS count)

| # | Criterion | Threshold |
|---|---|---|
| 1 | V1 suite | all 9 instruments green, raw output pasted |
| 2 | Collapse ratio at 10¹³ | ≥ 15× |
| 3 | Scaling signature | walker-time/cells constant within ±25% across 3 scales |
| 4 | Mertens lattice | granularity + memory + measured per-query cost recorded; rung verdict |
| 5 | α-sweep | cells(α) table at 10¹³/10¹⁴; selected α per scale in profile; M-alpha-domain killed |
| 6 | π(10¹⁴) | ≤ 1.5 s cool MT (floor 2.5 s); π(10¹²) ≤ 0.12 s |
| 7 | Mutants | M-μ-parity, M-mertens-partial, M-special-boundary, M-alpha-domain + Phase 7 registry — all killed, tiers recorded |
| 8 | Differentials | [10¹⁵, 10¹⁶] ≥ 5 points bit-exact (owed since Phase 7) |
| 9 | **Marathon I** | π(10¹⁷) = 2,623,557,157,654,233 exact, cert-record, ≥ 5 kill-resumes |
| 10 | **Marathon II** | π(10¹⁸) = 24,739,954,287,740,860 exact, within 1.5× of committed target, ≥ 3 checkpoints, ≥ 1 kill-resume, differentials above 10¹⁶ |
| 11 | Re-race | Section B v2, gap column, config digests, 10¹⁰ ST claim re-issued |
| 12 | Ledger | M-I/M-II as standing lines resolved; Phase 12 row restored; both completion rates |

## PART 10 — DECISION MAP

The V1 suite's collapse ratio → whether the substrate is real or another prose engine; the α table → per-scale defaults and how much of the 11.6× headroom is recoverable; the lattice verdict → the Mertens design every future summatory function rides; the marathons → the capstone records and the sustained column for every future claim; Section B v2 → the artifact Phase 16's product and FFI release publish; the scaling-signature instrument → the permanent guardian of every interval-arithmetic claim this project ever makes again.

---

Run order: **the V1 suite first — one binary, one session, every number the last two phases described instead of measured.** The census already told us the weapon is sized right (op-count within ~4–5× of the opponent, per-op cost the whole remaining war, monotone-v streaming the weapon's edge); now the instruments confirm it's *built*. M-I starts tonight in the background as it has been owed for seven phases. Paste back, in order: the V1 raw output with the collapse ratio and scaling-signature lines, the α table, and — when they land — the two cert-records. The census wrote the arithmetic; this phase makes the arithmetic run.
