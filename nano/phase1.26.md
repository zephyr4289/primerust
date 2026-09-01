Read it. We *will* engineer ahead — but not on top of this, because Phase 25 as reported is a **paper phase**: not one wall-clock number in the entire document, the frozen probe skipped for the second consecutive release (the omission is itself data — you don't hide a number that got better), and the single instrument that *was* run — the P25-0 census — **refutes itself in its own table.** The discipline of measurement-first was right; the execution returned garbage and stamped a conclusion on it. Autopsy first, then the real prize this phase walked past.

---

### §1. The census is provably broken — by conservation

A census must conserve cells: for every β, `hard_cells(β) + delegated_cells(β) = 776,070,926`. Your table reports **hard cells = 0 for all five β** — including β = 1.2, where z barely extends past y. That means even a 6.085×-retuned band "contains" nothing, all 776M cells are delegated for every dial setting, D is identically zero — **which contradicts the engine's own bit-exact π(10¹⁴)**, since a real z-split moves real cells. An instrument that returns zero everywhere isn't measuring nothing; it's *broken*, and zero got believed. Note also: every row's "Est Time" is the identical 0.100s — that's the estimator's base constant with zero cells fed in, not five independent predictions.

**Where the cells died — three mechanisms, in confidence order:**

- **H1 (lead, ~80%): the Phase 23 disease, third occurrence.** The prime-count columns are *real* (856 / 2,108 / 4,171 — I checked: width/ln checks out), so π(z) − π(y) is computed correctly. But iterating the band as `primes.iter().skip(π(y)).take(π(z) − π(y))` against a base-primes array that only holds primes ≤ y (4,800 entries) yields a **silently empty iterator** — skip past the end, take returns nothing, no panic, zero iterations, zero cells. Value-vs-index collision: Phase 23 passed 46,416 where 4,792 was wanted; Phase 25 indexed 8,971 into an array of 4,800.
- **H2: predicate on the wrong variable** — the hard-band test comparing `v` or `e` against (y, z] instead of `p`.
- **H3: span arithmetic underflow** — `e_hi.saturating_sub(e_lo)` returning 0.

**The design principle that kills this class permanently:** the census must *share the walker's cell predicate* — same function, instrumented — never a re-derivation beside it. A census that counts a different universe than the engine walks will always be able to lie. And instruments must fail loudly: `debug_assert!(hard_cells > 0)` for any non-degenerate band is not optional.

### §2. "Matches opponent's calibrated dial" — numerology, twice

Even if the table had worked, the claim fails twice:

1. **It was argmin over a constant function.** All rows identical ⇒ the β = 2 selection is the tiebreaker's arbitrary choice (first-minimum or middle) dressed up as calibration. The machine said nothing; the conclusion was written first.
2. **The dials are in different units.** The opponent's α_y = 6.085 means their y = 6.085·x^(1/3) = 282,435 and z = 2y = **12.17·x^(1/3)**. Your β multiplies *your* y = x^(1/3). Your "β = 2" is z = 2·x^(1/3) — **one-sixth of the opponent's z** in absolute terms, one-sixth the hard-band primes. Comparing β across different y-bases is comparing temperatures in two different scales and calling them equal.

### §3. The real miss: α_y — the dial you never touched

Here's the actual engineering insight this phase walked past, and it's bigger than β:

**Titan runs with y = x^(1/3) (α_y = 1). The opponent runs α_y = 6.085.** That single dial sets the physical v-horizon x/y:

| α_y | y | v-span [√x, x/y] | Blocks @64k | π-table / Mertens horizon |
|---|---|---|---|---|
| 1 (Titan) | 46,415 | 2.15×10⁹ | ~32,800 | 6.1× opponent |
| 6.085 (opponent) | 282,435 | 3.44×10⁸ | ~5,250 | baseline |

**Titan has been physically sweeping a v-axis 6.085× longer than the opponent's, for 25 phases, while the opponent's tuned dial sat in your own Phase 21 telemetry dump unused.** α_y isn't a minor tuning constant — it's the center of gravity of the whole algorithm: it trades physical sweep length (shrinks 1/α, with π-table/Mertens/block-build horizons shrinking in lockstep) against delegated analytical surface (grows, but at libdivide-rank cost ~0.1s-class, which your 24-B flat Φ machinery exists to serve). The hard-cell total stays roughly invariant (~10⁶ — prime count in the band grows ~5.2× while per-prime e-span shrinks ~6×), which is exactly why the opponent's D is 0.090s: tiny physical walk, big cheap analytics. Titan inverted that trade without ever choosing to.

**The census must sweep the 2-D dial, not β alone:**

```rust
pub struct SplitDial { pub alpha_y: f64, pub beta: f64 }   // y = α·x^(1/3), z = β·y

pub fn census(x: u64, d: SplitDial, primes: &BasePrimes /* FULL array to √x */,
              walker_pred: &dyn Fn(u64, u64) -> CellSpan) -> CensusRow {
    let (y, z) = (d.alpha_y.mul_cbrt(x), d.beta.mul(y));
    // TYPE-SAFE band — the H1 kill site:
    let j_lo = primes.pi_index_after(PrimeVal(y));   // PrimeIdx, panics on misuse
    let j_hi = primes.pi_index_after(PrimeVal(z));
    let band = primes.range(PrimeIdx(j_lo), PrimeIdx(j_hi));  // hard panic if j_hi > len
    debug_assert_eq!(band.len() as u64, pi(z) - pi(y));       // cross-check vs π-table
    let mut hard = 0u64;
    for &p in band { hard += walker_pred.span(x, p, y); }      // WALKER'S OWN predicate
    // CONSERVATION — a census that doesn't conserve is not a census:
    debug_assert_eq!(hard + delegated(x, y, z, walker_pred), walker_pred.total_cells(x, y));
    debug_assert!(hard > 0);                                   // loud, not silent
    CensusRow { /* …, est derived: T(α,β) = builds(α)·t_b + hard·t_cell + flat(α,β)·t_flat */ }
}
// Sweep: α ∈ {1, 2, 3, 4, 6.085, 8} × β ∈ {1.5, 2, 2.5, 3} — argmin T, store in
// scale-indexed dispatch. Include (6.085, 2) as a grid point: it's the only
// externally calibrated point in existence — but tune for Titan's cost profile.
```

Sweeping β on a fixed α = 1 base is tuning the trim of a ship whose hull is 6× too long.

### §4. Arena25 and Tier-2: status discipline

- **Arena25**: a struct, two counter *fields*, one `test_arena25_basic` green at trivial scale, suite at 3.80s — it is not wired into the dispatch. Nothing in this report shows a single 10¹²+ evaluation through the arena path, no 1T ≡ 8T bit-exact partition proof, no counter *receipts* (blocks_built, cells_served, builds-per-block — the H1-vs-thrash discriminator from Phase 24, still unmeasured). Invariants as bullet points are prose; invariants as `debug_assert!`s are law.
- **Tier-2**: §3 is two sentences — my Phase 24 §5 projection read back to me as a deliverable. The i128 A/B equality at three scales (P25-2 exit) does not exist. The coefficient-collapse derivation remains blocked on the cell-semantics pin I've requested three times (cells vs runs vs distinct-v), which the counters would resolve in one run.

### §5. Scoreboard

Phase 25 re-mark: **~3 PASS / 9 OWED** (census instrument exists and its prime-count column is real; arena struct + test; measurement-first discipline attempted). The phase adds 12 criteria and claims 12/12 with zero performance receipts — the criteria budget inflates by exactly 12 per phase the way a subscription adds minutes. Phase 24 was ordered re-marked **FAIL** two turns ago and still reads 12/12 — that instruction has now been ignored twice. Standing law, restated: **no phase closes without the frozen-probe receipt attached; a probe regression >10% auto-fails regardless of criteria count; a census without conservation asserts is inadmissible.** 96.9% completion while the engine sits at 62.6s — the scoreboard is now fully decoupled from the silicon.

### §6. The path — four receipts, in order

1. **Frozen probe on current HEAD.** Expected ~62.6s (the tests imply the hot path is untouched — which means Phase 25 changed nothing that runs). This single number settles whether anything is wired. Run it first; it's one command.
2. **Fixed census, conserving, 2-D grid** per §3, with the walker-predicate sharing and loud asserts. Ship the table — the corrected numbers will *not* be zeros, and β-optimal will move.
3. **`term_breakdown` at 10¹⁴ after the α_y retune** — watch D's build/walk columns collapse with v-span, and watch the delegated/Φ_flat column grow. This is also the first honest receipt for the still-unevidenced "3–5 ms" Φ_flat claim from 24-B.
4. **Arena25 counters through the wired path at 10¹²**, with 1T ≡ 8T bit-exact on the chunk partition — pinning cell semantics and closing the Phase-21 E4 debt in the same run.

The prize for the retune, honestly held: α ≈ 6 shrinks the physical sweep to opponent scale (~5,250 blocks ≈ 75 ms pooled builds), hard walk ≈ 10⁶ cells, leaving S₂ (0.41s) and setup (0.29s) as the visible bottlenecks — **~1.0–1.5s at 10¹⁴, 4–6× behind**, with parity still a Phase 26 problem (S₂ fusion + setup trim), not a promise. But the marathon window reopens *structurally*: at 10¹⁸ the physical v-sweep at α = 6 is 1.7×10¹¹ slots — the sub-15-minute π(10¹⁸) floor I derived before is now reachable by design rather than by luck.

Deliver receipt #1 and #2 and I'll write the Tier-2 coefficient-collapse derivation against your exact 5-term identity plus the full Arena25 builder patch in the same sitting — the semantics pin from #4 is the only input I'm missing. The engine has been bit-exact through a 10× slowdown and a paper phase; the math has never been the problem. The instruments are. Fix the census, run the probe, and Phase 26 starts from truth instead of from 12/12.
