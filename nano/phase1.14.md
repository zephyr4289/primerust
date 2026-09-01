# Phase 13 — The Race: Same Silicon, Same Session, No Excuses

Good — you've finally demanded the thing I've been demanding for four phases. "Race both on same hardware" **is** the S5 debt, and it has been runnable since the day you switched phones. So before any god-level engineering: you don't design the winner before timing the opponent on the track. The race runs today, costs one cool session, and replaces every number below with truth.

And to close the last exchange: the 1–3 h was Lehmer's bill — the price of the FAIL branch your own Phase 11 ledger recorded. This phase exists to make that number die. That's the whole point of going Gourdon.

---

## PART 1 — WHERE WE STAND TODAY (the calculation you asked for)

Titan numbers are **measured** (your Phase 10 matrix, 8T, Lehmer — the best engine we own per the selector). Primecount numbers are **estimates** — G100 measurements × ~1.25 IPC uplift, ±30% — because primecount has *never been timed on this device*. That's the confession at the center of every "we beat primecount" claim ever made in this project: they are all cross-device. Every single one.

| x | titan Lehmer 8T | primecount est (4 Gen 2) | Ratio | Verdict |
|---|---|---|---|---|
| 10¹⁰ | ~0.033 s (extrap) | ~0.045 s | 0.7–1.1× | **probable WIN** (setup regime) — race decides |
| 10¹¹ | 0.125 s | ~0.06 s | ~2× | LOSS |
| 10¹² | 0.593 s | ~0.08 s | ~7× | LOSS |
| 10¹³ | 2.267 s | ~0.10 s | ~22× | LOSS |
| 10¹⁴ | 11.683 s | ~0.23 s | ~50× | LOSS |
| 10¹⁵ | ~50 s (extrap) | ~0.55 s | ~90× | LOSS |
| 10¹⁶ | ~3.6 min (extrap) | ~2.2 s | ~100× | LOSS |

**The win map as of today, honestly enumerated:**

1. **10¹⁰ setup regime** — Lehmer's table build is ~0.01 s; Gourdon's setup (y-sieve, μ-structures, α machinery) is heavier at small x. This is the one compute win we plausibly hold, and it's narrow.
2. **Phone-native by forfeit** — checkpoints, kill-resume, zero-alloc. primecount has none of these; a phone OS that can murder a process at any second is *our* home field and their absent feature.
3. **Physical ST sieve** — titan beat primesieve single-thread by 5% on the G100. Needs re-measurement here.
4. **Everything else is red**, and the gap *grows* with scale, because Lehmer is O(x^(3/4)) fighting O(x^(2/3)). No scheduling genius, no NEON, no wheel recovers a 100× algorithm gap. That's not pessimism; that's the exponent.

That's "where we are winning on same hardware from Walisch" — one narrow regime, one forfeit column, and a war to fight.

---

## PART 2 — WHAT A LEGAL WIN IS (The Race Protocol, R0)

New permanent law, because this phase will generate claims:

> **The definition-of-winning law:** a win is same device, same session, pure-compute timing, opponent at best-config, N pinned, both columns where duration exceeds the thermal cliff, recorded in Section B. CLI-vs-CLI spawn overhead cancels on both sides (the ~200 ms Termux fork is shared tax, re-measured on this device). Coverage rates are banned from the race table — time at pinned N only.

**The session (~40 minutes):**
- primecount: 8T and **1T** at 10¹⁰–10¹⁶; algorithm ladder (lehmer/lmo/DR/gourdon) at 10¹³/10¹⁴; alpha sweep if the flags exist (check `--help` first — flag-drift law); 10¹⁷/10¹⁸ attempted — if it OOMs or refuses on 8 GB, that's a recorded device-class fact, not a failure.
- titan: Lehmer 8T and 1T; Meissel for the record.
- primesieve vs titan-sieve, ST and MT, best-config swept — the physical column, on this silicon at last.
- **The 1T/8T scaling measurement of the opponent is the sharpest instrument in the session.** Our pool extracts ~5.5–6× from this 2+6 topology; if primecount's OpenMP extracts only ~2.5–3× (uniform splits + in-order little cores gating the tail), there is a real 2× scheduling gap on this exact silicon — the one front Walisch cannot answer without rewriting his scheduler for phones. Measure it before betting on it.

**Adopted from your blueprint — the cachegrind referee:** `valgrind --tool=cachegrind` on both binaries at 10¹²/10¹³ gives deterministic instruction counts (Ir) — a throttle-proof, session-independent metric. Caveats recorded: aarch64 valgrind in Termux, thread aggregation, generic cache model (compare Ir, not cache-miss counts, unless configured). Honest expectation: today we lose the Ir race ~30× at 10¹⁴ — 5.4×10⁹ tree nodes against their ~10⁹ total operations. Ir becomes the *constants referee* only after the algorithm class matches. Adopt it now, read it then.

---

## PART 3 — YOUR BLUEPRINT, AUDITED (receipts, not opinions)

The instinct — "asymmetric design, not brute force" — is exactly right. But four of its load-bearing claims are refuted by our own frozen measurements, and one is written for a phone you no longer own:

| # | Blueprint claim | Verdict | Receipt |
|---|---|---|---|
| 1 | "Cortex-A76, 64 KiB L1D, G100" | **STALE** | You switched devices. Phase 8's geometry sweep *measured* the A78's 32 KiB L1D — the 64 KiB default was thrashing at a 10.8% penalty. The blueprint plans around a cache this phone doesn't have. All constants come from the device profile, not from prose. |
| 2 | π(x) = D + W₁…W₅ | **REJECTED as-is** | Third convention set in this project's history (v2.4 froze AC − B + D + Φ₀ + Σ; primecount's source has its own). The Phase 1 convention war and the Phase 5 sign-mutant both taught this lesson: unverified decompositions are where silent wrong answers are born. **D-lock-3** derives the true term set from Gourdon's paper + primecount's `pi_gourdon.cpp`, term by term, our conventions, term oracles before any perf code. |
| 3 | O(x^(2/3)/ln³x) | Corrected | Your own reference.md froze O(x^(2/3)/log²x). D-lock-3 fixes the record; we don't fight complexity claims in prose. |
| 4 | Rigid pipeline: big0=sieve, big1=tree, cores 2–7 = W₁/W₂ + atomic accumulators | **REJECTED** | Phase 3 measured 58% of sieve capacity on the little cores and a pool that self-balances thermal droop to ≤3% skew. Static role assignment forfeits both. Atomics per partial violate the sync-inventory law — per-worker private + join is the owned pattern. The blueprint's asymmetry is *imagined*; ours is *measured* (Part 4). |
| 5 | Wheel-210 mandatory, 77.14% reduction | **Rung, not foundation** | Phase 1 amendment stands: +14.3% fewer candidates, +16.7% span per L1 bit, 6× table complexity. Its priority rises slightly on 32 KiB L1D. It gets measured, in R7, after the machinery that matters exists. |
| 6 | Two-level Φ cache (x ≤ 10⁵, a ≤ 100) | Already owned, better | PhiTiny (k ≤ 6 flat, O(1)) + π-table T2 exits *are* this, certified exhaustively over full periods. The tree's measured mass is mid-levels, and the bottom 8 levels are already table-served. |
| 7 | StaticVec const-generic stack | Already owned | The explicit arena stack: 4,096 entries, 441 consumed at 10¹⁴, stack-overflow class structurally dead since Phase 5. |
| 8 | CSEL / CNT branchlessness, jump-table dispatch | Adopted as micro-rungs | Tally is already vectorized CNT; CSEL selects and match-dispatch are R7 rungs under the unsafe-in-kernels law. |
| 9 | α ∈ [1.5, 3.0] dynamic per x | Adopted as hypothesis | This is C11's rebirth — and now legitimate, because it prices *the real tradeoff* (sweep span vs leaf machinery vs memory). Census-fitted, per scale, per device. |
| 10 | C-FFI surface | Phase 14 | Product phase item. |

The blueprint's soul is right; its body was written for a dead device and an unverified formula. We keep the soul.

---

## PART 4 — THE REAL ASYMMETRIC ARCHITECTURE (four measured asymmetries, three fronts)

The asymmetries this device *actually* has, all from our own instruments:

1. **Thermal asymmetry** — big cluster derates ~2× under sustained load; little cores hold ~91–100%. The pool converts this from enemy to balancer (certified ≤3% skew).
2. **Memory asymmetry** — two OoO cores saturate the 11 GB/s-class bus; little cores add ~0 bandwidth. Consequence: big-core streams stay L1/L2-resident, little-core state stays compact (the SoA constant/mutable split — owned pattern, third consumer).
3. **IPC asymmetry** — in-order A55s are murdered by division and branch; magic division and branchless kernels are already owned and certified.
4. **Setup asymmetry** at small x — Lehmer's near-zero setup vs Gourdon's heavier apparatus. The 10¹⁰ win.

**The war on three fronts, with honest targets:**

| Front | Weapon | Closes | Honest target |
|---|---|---|---|
| **Algorithm** | Gourdon-class via D-lock-3 + the interval substrate | the 7–100× | **parity to 2× at 10¹²–10¹⁶** — matching a two-decade-tuned engine's class on this silicon from scratch |
| **Constants** | A78-tuned kernels, wheel-210 rung, our +5% sieve edge, NEON leaf counters, Ir referee | the residual 2–4× | eat into it; every rung measured |
| **Scheduling** | Thermal pool vs their OpenMP on 2+6 | the 8T-scaling gap *if R0 measures one* | **the winnable championship: fastest π(x) on thermally-constrained heterogeneous mobile silicon, ≥10¹⁵ sustained** |

Plus the **forfeit front**: crash tolerance, checkpoints, zero-alloc — wins primecount structurally cannot match without a redesign.

And the honest boundary, said once, plainly: universal x86-server dethroning is not on this menu — Walisch at 10¹⁹ on a 64-core server is untouchable from a phone. "Completely dethrones a decade of C++" on all scales is the 30 B/sec disease wearing a new t-shirt. What *is* on the menu, and is genuinely industry-beating: **the fastest prime-counting engine ever built for phone-class silicon, exact, crash-proof, beating primecount-on-this-device in the regimes this device punishes it in.** That claim, measured under Part 2's law, is one the HPC community actually has to respect.

---

## PART 5 — BUILD ORDER (R-series; each gets its own ultra-deep spec in turn)

- **R0 — The Race Session** (today): Part 2's protocol; Section B live; win/loss map replaces Part 1's estimates; opponent 1T/8T scaling measured.
- **R1 — The Census** (rides along, ~10 min): the Phase 12 counters — distinct-⌊x/d⌋ volume, M-lookup counts, μ-coverage span, node-count confirmation (~5.4×10⁹ ± 20%). This sizes the interval substrate before it's designed.
- **R2 — D-lock-3**: the derivation gate. Gourdon's paper + primecount's actual source, term by term, our conventions, worked anchors, the α parameterization derived, term oracles green. No performance line of code exists before this is green.
- **R3 — The μ/Mertens extension**: segmented μ-sieve riding both passes to **x^(2/3)** — the exact coverage gap Phase 11's timing fingerprint exposed (0.0123 s = x^½-scoped, full stop). Checkpointed Mertens partials; anchors cross-checked at extended scale.
- **R4 — The one-pass shared sweep with special-leaf epilogue**: per-j constant-v interval splitting, Mertens partials, batched distinct-v π-lookups, riding the existing pool sweep — with the **structure test as a hard gate**: S₁-events-must-execute-during-sweep-segments, because Phase 11 proved a leaf engine that doesn't ride the sweep is a 0.33× corpse.
- **R5 — α-tuner**: census-fitted per-scale table; M-alpha-domain finally killed.
- **R6 — MT + thermal policy**: pool integration, partition invariance, telemetry columns.
- **R7 — micro-rungs**: CSEL selects, match-dispatch nodes, wheel-210 sweep rung, presieve extension — one change per commit, ±3% keep-or-revert, Ir-refereed.
- **R8 — The re-race**: Section B v2 with the honest gap column; marathons re-run on the new engine when it beats Lehmer's extrapolation; Ir table published.

Your menu asked: wheel-210 or the W₃/W₄ loop. **Neither is first.** The W-loop is dead until μ covers x^(2/3) and the leaves ride the sweep — Phase 11's autopsy said exactly that — and the wheel tunes a sweep that doesn't yet exist in the right shape. First engine sub-part is **R3+R4, the interval substrate**, and it is gated by R2.

---

## PART 6 — PASTE BACK (in this order)

1. **R0 raw race table** — both engines, both thread counts, pure-compute, best-config — and primecount's 8T/1T scaling factor, which sizes the scheduling front.
2. **R1 census counters** — which write R3/R4's design constants and Phase 14's first sentence.
3. Marathon I cert-record (15–25 min, run tonight in the background — the checkpoint machinery's first true-scale exercise, feeding the sustained column of the race table).

Then we go ultra-deep, one sub-part at a time, exactly as you want: **D-lock-3's derivation is the next document** — the Gourdon decomposition, term by term, with the interval inventory that R3/R4 implement. The race tells us the score; the census sizes the weapon; the derivation builds it. Same order as every phase that ever went green for real: measure, derive, build, prove.
