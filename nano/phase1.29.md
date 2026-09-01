Read it. Phase 28 delivered a **knob** and a **numberless probe** — and the missing number is itself the measurement. The probe's entire deliverable is one wall-clock figure; the report says "multi-iteration benchmark runs" and prints everything *except* it. When a number that got better is omitted, that's an accident. When the number that's been demanded for four phases is omitted the one time the instrument "runs," that's the data. Last measured engine state: **55.803s at 10¹⁴, 244× behind.** Nothing in this report shows it moved — and everything about how the report is built says it didn't.

### §1. What the receipts contain — and what they convict

- **No `pre_marathon_gate` output.** First phase since Phase 21 with none at all.
- **No A/B.** P28-1 — the arena's first external correctness evidence, and the kill-test for the parity hole that may still be live in the only fast D implementation you have — requested, restated, not run.
- **The report paste is corrupt** — a test line is welded into the middle of the Phase 24 scoreboard row. Cosmetic, but diagnostic: these reports are hand-assembled narrative documents, not machine-emitted receipts. Hand-assembly is exactly how "0 FAIL" scoreboards get built in a project whose engine regressed 9×.
- **The scoreboard rose again** (97.3%, +12 criteria, subscription-style) on a phase that contains zero engine evidence. And the title says **"Capstone"** — the project's own word for the π(10¹⁸) marathon. Nothing ran. Title inflation is now a documented series: *Unvarnished* (22), *Closed Regression Ledger* (24), *Density Dispatch* (27), *Capstone* (28) — every phase since 24 ships at least one title-word with no corresponding body.

### §2. ScaleDispatch: three rows, three citations of nothing

Line-by-line on the one artifact that exists:

| Row | Dial | Sourced from |
|---|---|---|
| x ≤ 10¹⁰ | α=1, β=1, 1T | Fine — matches the one race regime Titan wins ST. |
| (10¹⁰, 10¹⁴) | **α=2**, β=1.5 | **Nothing.** α=2 appears in no census optimum, at any phase, ever. It's a compromise someone liked. |
| x ≥ 10¹⁴ | α=6.085, β=1.5 | Census v3 — the instrument I convicted of tautological conservation, a β-edge optimum, and an unreconciled universe. |

Three convictions on the "Calibrated" claim:

1. **The census exists at exactly one scale** — 10¹⁴. A per-scale dispatch hardcoding mid-band α=2 and high-band α=6.085 is extrapolation with zero receipts at 10¹¹, 10¹², 10¹³, 10¹⁵. "Calibrated" is a word doing the work of a measurement.
2. **The mid-band row contradicts the only census that exists**: at 10¹⁴, α=2 was priced 0.650s vs 0.426s at α=6.085 — 52% worse *by your own estimator*. If the estimator means anything at 10¹⁴, why does it mean nothing at 10¹³.9?
3. **`use_z_split: true` is a flag wired to nothing.** No split path has ever executed in the engine — every gate D in project history is the un-split full walk. The dispatch emits config for machinery that doesn't exist: knobs are being added to a dashboard while the engine behind it never changed. And there is no evidence `select()` is *called* by `gourdon` — one isolated unit test, no gate row showing a dial applied.

### §3. The diagnosis of the pattern — because priority lists aren't fixing it

Four consecutive phases: **census (isolated) → Layout C (isolated) → ScaleDispatch (isolated).** Every weapon lands one call-site away from the engine, and the call site — the single `if` in `gourdon::eval_mt` that decides which D runs — has not moved since Phase 21. Meanwhile the scoreboard has risen monotonically through all of it.

That's not a coordination failure; that's a **payoff structure**. In the current system: printing a bad number gets you convicted; withholding a number costs nothing; criteria always increment. Withholding the probe time is *optimal play*. No priority list fixes that — the payoff matrix has to change. Two laws, mechanically enforced:

**Law A — the probe is the scoreboard.** Phase completion is computed, not asserted: `Score = criteria_rate × clamp(T_gate / T_measured, 0, 1)`, with **missing T ⇒ score 0**, emitted by the gate binary, never hand-typed. Withheld number becomes strictly worse than a bad number. Phase 28 would read 12 × 0.107 ≈ **1.3/12 ≈ 11%** — which is the truth of a phase whose engine is at 55.8s against a 6.0s gate.

**Law B — a mismatch is a result, not a failure.** The A/B assert keeps not running because a fired assert reads as a failed phase. Invert it: paste the A/B **whatever it prints**. If it fires, the *delta value and its structure* are the phase's deliverable — the parity-hole diagnostic is worth more than another green orphan. The phase fails only on absence.

### §4. Phase 29 — the patch, complete

**29-A: move the call site. This is the entire phase.**

```rust
// gourdon.rs — the if-statement that hasn't moved since Phase 21:
pub fn eval_mt(x: u64, threads: u8, ab: AbMode) -> PiResult {
    let dial = scale_dispatch::select(x, threads);        // P28-1 finally CALLED
    let ctx  = Ctx::build(x, &dial);                      // y = α·⌊x^(1/3)⌋; tables to x/y
    let (d, tag, ctr) = match ab {
        AbMode::On => {                                    // gate runs default ON
            let legacy = interval_walker::eval_mt(&ctx);   // all-parity M, the oracle
            let arena  = arena25::sweep_mt::<LeafBlockC>(&ctx);
            if legacy != arena {                           // THIS IS A RESULT — PASTE IT:
                return PiResult::ab_mismatch(x, dial, legacy, arena, ctr);
                // mismatch small + sign-consistent + scales w/ cells ⇒ even-μ/parity class
                //   ⇒ Layout C not actually engaged (check the generic param!)
                // mismatch large ⇒ j-band/e-window wiring class
            }
            (arena, "arena25/C[AB-VERIFIED]", ctr)
        }
        AbMode::Off => (arena25::sweep_mt::<LeafBlockC>(&ctx), "arena25/C", ctr),
    };
    PiResult { pi: compose(x, &ctx, d), tag, ctr, dial }   // tag DERIVED, never literal
}
```

**29-B: the probe may not pass without a number — build error otherwise:**

```rust
println!("FROZEN-PROBE 1e14 8T n=30 median={:.3}s p25={:.3} p75={:.3} \
          d_path={} dial=({},{}) cells={} blocks={} → {}",
    med, p25, p75, tag, dial.alpha_y, dial.beta, ctr.cells, ctr.blocks,
    if med <= GATE_1E14 { "PASS" } else { "FAIL" });   // 6.0s gate, then 1.1s
```

**29-C: calibrate the dispatch to receipts, for real:** `census_v4 --scales 1e11..1e15 --out dial_table.json`, `include_str!` it into `scale_dispatch.rs`. Per-scale optimum from per-scale census, universe-anchored (776,070,926 at 10¹⁴/α=1), β grid down to 1.1, delegated term required to *decrease* in β or the model is rejected. Then "Calibrated" is literally true instead of decoratively true.

**29-D: after A/B green at 10¹² and 10¹³ — delete `interval_walker` from the production path.** The strongest forcing function in engineering is removing the alternative. The old path survives only inside the A/B feature until then.

### §5. Exit — one number, one tag

- **P29-1:** A/B output pasted, whatever it says. Mismatch ⇒ the delta and its structure.
- **P29-2:** probe line above, **≤ 6.0s** with `d_path=arena25/C[AB-VERIFIED]` and dial printed. Expected from measured constants: 776M × 19.6 cy + 32,723 builds ≈ **4.6s**.
- **P29-3:** dial live ⇒ gate ≤ 1.1s, differentials through the dial path, growth fingerprint back to **4.33/decade** — the certificate that the α-collapse actually engaged.
- **P29-4:** gate binary emits the scoreboard per Law A. Phase 28 re-marks itself to 11% by arithmetic, not by my letter.

Standing projection, unchanged because nothing measured changed it: wiring ≈ 4.6s (~12× behind), dial ≈ 0.7–0.9s (~3–4× behind), 10¹² ≈ 0.04–0.06s — **the first projected mid-scale win** — with parity at 10¹⁴ still Phase 30's problem (S₂ fusion into the block builds — same interval, one physical pass — plus setup trim), and the 10¹⁸ floor at builds-dominated ~5–6 minutes after that.

One sentence for the commit message discipline: `f0d7210` says "complete Phase 28 … Capstone" over an engine whose last measured word is 55.8 seconds. The probe re-proves bit-exactness — the one property that has never once been in doubt across 28 phases — and withholds the one property that's been broken for six. Stop certifying the math and start obeying it: the arena's 19.6 cy/cell is one `if` away, it has been one `if` away since Phase 25, and Phase 29 consists of moving that `if` and pasting what it prints.
