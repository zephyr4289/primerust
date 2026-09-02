════════════════════════════════════════════════════════════════
THE LOOP JUST EARNED ITS KEEP — AND THE REPORT STAMPED "PASS" ON A 73× REGRESSION
════════════════════════════════════════════════════════════════

Straight read, no anesthesia: **the 60-second loop worked.** It found in one run what 360 PASS criteria, 50 unit tests, and two phase reports could not see. That is the iteration velocity you asked for — it arrived, and it's screaming. The problem is not speed of iteration. The problem is that **your harness computed the impossibility, printed it in a column, and stamped PASS next to it.** The tripwire fired and somebody unplugged the bell.

## §1 THE ARITHMETIC (your numbers, no interpretation needed)

| x | Phase 32 | Phase 34 | regression | vs primecount 0.21 s | honest model |
|---|---|---|---|---|---|
| 10¹² | 1.318 s | 0.6926 s | **1.9× faster** ✓ | 3.3× | 0.02–0.05 s → **14–35× over** |
| 10¹³ | 10.238 s | 4.758 s | **2.15× faster** ✓ | 22.7× | 0.08–0.15 s → **32–60× over** |
| 10¹⁴ | 0.4457 s | **32.716 s** | **73.4× SLOWER** | **155.8×** | 0.15–0.18 s → **180× over** |

Four fingerprints, each one narrows the kill zone:

**FP1 — One dominant sink, present at every scale.** 10¹² and 10¹³ improved 2× (z-fix + FTD-v2 + carry are real — see §5), yet all three rows sit 14–180× over model. This is not a 10¹⁴ cliff. One mechanism dominates everywhere.

**FP2 — Perfect power law: t ∝ x^0.837.** Ratios 6.871, 6.874 (β = log₁₀ 6.872 = 0.8372, three points on a clean line). **No Gourdon component has this law.** B ∝ x^(2/3) (y-coupled), D ∝ x/z, FTD ∝ z, boot ∝ √x, Σ ∝ π(y). Nothing is x^(5/6).

**FP3 — The sink does not read your parameters.** P0's α multipliers are lumpy across the decade: α_y = 2.0 → 2.0 → 6.09; α_z = 3.0 → 3.0 → 9.13. A B-dominated run would jump 1.5× per decade at the 10¹³→10¹⁴ boundary; a D-dominated run, 1.6×. You observe a *perfectly smooth* 6.87× straight through the lump. **The 32.7 s lives in code that never consults (y, z)** — a wrong range, a recomputed parameter, a scalar fallback, or a stall.

**FP4 — Your own physics column says 0.01 GB/s, constant, all rows.** That is **851× below the same report's Probe 1 (8.51 GB/s)**. An honest memory-bound run at 10¹⁴ shows ~0.3–0.5 GB/s effective. 0.01 GB/s = the run is not moving data — it is *stalled or serially computing*. The loop told you this in three rows. The harness replied PASS.

## §2 WHY EVERY GATE SAID PASS — WIRE THE TRIPWIRE (30 minutes)

The plausibility asserts from §B were specified to **fail loudly**. They were implemented as prints. A print cannot gate anything.

```rust
// loop.rs — the loop is a TRIPWIRE, not a reporter. Physics law:
for p in TIMERS.phases() {
    let gbps = p.bytes as f64 / p.ns as f64;
    let cyc_per_op = p.marks as f64 * 1.75 / p.ns as f64 * 1e9; // probe-2 calibrated
    if gbps > 20.0 || cyc_per_op > 17.6e9 {          // 8 × 2.2 GHz floor
        eprintln!("PHYSICS VIOLATION: {p} {gbps:.2} GB/s, {cyc_per_op:.1} eq-cyc/s");
        std::process::exit(1);                        // RED. Nonzero. CI dies. Report cannot ship.
    }
}
```

And the gate_contract amendment that was specified in Phase 32 and never wired — **the scoreboard is now formally anti-correlated with reality: completion went 97.7 → 97.8 while wall-clock went 0.446 → 32.7 s.** A metric that rises as the machine gets 73× slower is not a metric; it is decoration.

```rust
// gate_contract.rs — Law 0.5, mechanical: baseline is a git-tracked artifact.
// loop --certify > loop_baseline.json   (commit with every accepted phase)
fn gate(prev: &Baseline, now: &Baseline) -> Verdict {
    if now.median_ms(14) > 1.25 * prev.median_ms(14) { return Verdict::Fail; } // REGRESSION = FAIL
    Verdict::Pass
}
// Rule: no phase may raise the completion rate while any scale regresses.
// Phase 34 under this law: 32,716 > 1.25 × 445.7 ⇒ grade FAIL, scoreboard drops,
// and this conversation happens in CI instead of in my inbox.
```

## §3 THE ONE-RUN CONFESSION — your binary already knows the answer

The Phase-34 report contains totals, VmHWM, and probes. It contains **no per-phase table** — the single most diagnostic artifact the loop was built to produce, specified in §B as `TIMERS.report()`, absent from the report. Either it isn't printed (wire it: 10 minutes) or it wasn't pasted (paste it: 30 seconds). I will not pretend to armchair-pin FP2–FP4 from here — the honest statement is: **five candidate mechanisms, one table decides.**

| H | mechanism (fits x^0.837, param-blind) | phase-table signature | one-line discriminator |
|---|---|---|---|
| H1 35% | boundary resolution re-scan (D4-class): 640k boundaries × O(segment) scalar popcount | `b_count_resolve` = 15–30 s | count bytes-re-read = boundaries × avg-scan |
| H2 25% | loop.rs recomputes its own (y,z) ≠ P0 tuple (e.g. y = x^(1/3)) — sweep 269 MB ≈ your 327 MB "physical" bytes | `b_mark`/`b_count` ≫ model; param eprintln ≠ P0 | print the tuple *inside* gourdon() |
| H3 15% | D-walk over [1, xz) or [1, x*) scalar 1T with per-survivor udiv (lazy-R fast paths inverted) | `d_walk` = 20–30 s | d_walk ns + survivors count |
| H4 15% | pool/affinity collapse or barrier spin — 7 cores spinning, 1 working | all phases uniformly 6–8× over; util print | print threads-actually-running per phase |
| H5 10% | FTD-v2 covering the walk with degenerate block sizing | `ftd` phase ≫ (unlikely at z=423k) | ftd ns vs z |

Add phase isolation so no diagnosis ever needs a full run again:

```rust
// loop.rs --phase d_walk 14 : runs ONLY that phase on synthetic inputs,
// prints ns, bytes, marks, eq-cycles. Regression bisect without git-bisect.
// Wire it for all 8 phases today. This is the "iterate faster" machinery, literally.
```

## §4 THE CALIBRATION LAYER CONTRADICTS ITSELF (fix before trusting any model column)

- **Probe 1 says 8.51 GB/s. Phase 33's Probe A claimed 40 MB / 2.64 ms = 15.2 GB/s.** Both cannot be the sequential DRAM ceiling. One canonical bench arbitrates: **256 MB** (larger than every cache), read-only AND RMW patterns, median-of-5. Every bandwidth-bearing model constant cites that one number.
- Probe 3 gives sweeps/s but **no sustained GHz**. Without the sustained clock, every ns→cyc conversion in Probe 2 (assumed 2.2 GHz) is uncalibrated under thermals. Read `/sys/devices/system/cpu/cpu*/cpufreq/scaling_cur_freq` *during* the 3 s load. One number, one line of output.
- The "Physical" column has no labeled formula. An unlabeled bandwidth number is exactly how Phase 32's "<0.01 ms" happened. **Every column: units + formula, or it doesn't print.**
- Probe 2 is genuinely good — 1.0 ns/mark L1 vs 3.47 ns/mark L2-scattered at p=1013 is the number that explains Phase-32's 160 ms B-phase and belongs in MODEL. Keep it.

## §5 WHAT IS BANKED — DO NOT REVERT ANYTHING

- 10¹²: 1.9× faster, 10¹³: 2.15× faster, same binary as the 10¹⁴ blowup ⇒ **FTD-v2, carry, and the z-fix are net-positive.** The regression is a wiring/range/path bug, not the algorithms. Isolate, don't rollback.
- P0 exactness (three scales, bit-exact) — still the crown jewel; nothing in Phase 34 touched it.
- VmHWM 122.5 → 52.07 → 49.35 MB — F14's 40 MB ghost is dead and stayed dead.
- FTD-v2 mechanism with the 100k oracle — provisionally in; the full-z oracle at assault-z (3.16·10⁷) is still owed before it replaces the flat table on the 10¹⁵ path.

## §6 THE 48-HOUR OBLITERATION SEQUENCE

The largest recoverable prize in this project's history is sitting in your own loop output: **one sink, 14×–73× across all scales, 155.8× total vs primecount at 10¹⁴.** Killing it doesn't shave milliseconds — it collapses all three rows at once.

| step | action | time | pays |
|---|---|---|---|
| 1 | Wire tripwire `exit(1)` + paste the phase table | 30 min | names the sink |
| 2 | `--phase` isolation + param-tuple eprintln inside gourdon() | 1 h | confirms H1–H5 in one run each |
| 3 | Kill the sink | ½–1 day | **10¹⁴: 32.7 s → ≤0.25 s (130×). 10¹³ → ≤0.15 s. 10¹² → ≤0.05 s.** |
| 4 | Canonical 256 MB bandwidth bench + sustained clock read; MODEL cites probes only | ½ day | model becomes device truth |
| 5 | gate_contract ingests `loop_baseline.json`; regression = FAIL | ½ day | this class of report becomes unshippable |
| 6 | 10¹⁵ assault + G8 (primecount under OUR protocol — still never run) | 2–3 days | the headline |

Then the standing sequence still applies untouched: boot-swap (−100 ms), FTD-v2 full oracle, Phase-4 debts as the sustained-clock calibration run.

One closing law, and it is the whole message: **a green scoreboard next to a red loop is a broken scoreboard.** Phase 34's real grade under Law 0.5 is the first FAIL of this project — and it is the most valuable phase you've shipped, because the machine finally refused to lie to you. Wire the exit code, read one table, kill one sink. The obliteration is not ahead of you — it's behind a println you already own.
