════════════════════════════════════════════════════════════════
YOU BUILT THE CT SCANNER AND X-RAYED THREE RIBS. THE BULLET IS IN ONE OF THE FIVE CHAMBERS YOU SKIPPED.
════════════════════════════════════════════════════════════════

Cold arithmetic from your own report: isolated phases account for **212.61 + 0.89 + 6.60 = 220.10 ms**. The loop total at 10¹⁴ is **32,830.80 ms**. **32,610.7 ms — 99.33% of the runtime — sits in the five phases you did not isolate** (b_count_resolve, d_walk, sigma_ac, combine, whatever else the loop counts). The instrument that answers the question exists, is wired, works — and was pointed at the 0.7% of the body that isn't bleeding.

Two more facts before the math. First: your 10¹² and 10¹³ medians are **bit-identical to Phase 34** (692.56, 4,758.49 to 0.01 ms) and 10¹⁴ moved +0.35%. Same sink, untouched. Second: the tripwire got **half its laws** — you wired the impossible-*fast* checks (>20 GB/s, equality, VmHWM) and left out the one that catches this exact failure: the regression gate. `loop_baseline.json` exists, is tracked, and was never compared against anything. The gate that would have printed `32830 > 1.25 × 445.73 → FAIL` at commit time was specified in Phase 32 and is still not in the binary.

Now the payoff — because the three numbers you *did* isolate, plus the three totals, are enough to fingerprint the sink mathematically.

## §1 THE β FORENSIC — the sink is a clean single power law, and it is param-blind

Subtract the accounted phases (scaled by their own cost drivers: boot ∝ x/y, ftd ∝ z):

| x | total ms | accounted ms | **sink ms** |
|---|---|---|---|
| 10¹² | 692.56 | ~31.4 | **661** |
| 10¹³ | 4,758.49 | ~141.2 | **4,617** |
| 10¹⁴ | 32,830.80 | 220.1 | **32,611** |

Sink ratios: **6.98, 7.06**. β = log₁₀(7.02) = **0.846 ± 0.002** — one mechanism, three decades, clean. Now kill every innocent suspect with your own P0 tuples. Per-decade ratio predicted by each cost driver:

| cost driver | 10¹²→¹³ pred | 10¹³→¹⁴ pred | observed |
|---|---|---|---|
| x/y (sweep range) | 4.64 | **1.53** (y jumped 6.6×) | 6.98 / 7.06 |
| xz (D-walk range) | 4.5 | **1.60** | 6.98 / 7.06 |
| x* | 4.6 | **1.52** | 6.98 / 7.06 |
| π(√x) (boundary count) | 2.91 | 2.91 | 6.98 / 7.06 |
| y, z | 2.1 / 2.2 | **6.5 / 6.6** | smooth 6.98 / 7.06 |

**Nothing coupled to (y, z, x\*) fits — the 6× parameter lump at 10¹⁴ would have left a visible kink and there is none.** The sink reads x and only x. That is a hard structural constraint: the work lives in a loop whose length or per-unit cost is a pure function of x — and β ≈ 0.846 ≈ (1 + 3/4)/2 − log-drag is the exact signature of **per-prime work of the form (x/p)^c over p ∈ (y, √x]**, c ≈ 0.7–0.75, front-loaded at small p (which is why the y-jump barely dents it — the mass sits near p ≪ √x).

Magnitude check at 10¹⁴: 32.6 s × 8 cores × 2.2 GHz = **5.7×10¹¹ cycles available**. Lehmer-class per-boundary-prime evaluation, Σ_{p∈(y,√x]} (x/p)^3/4 × ~2–4 cyc/unit = **2.6–5.2×10¹¹ cycles predicted. Dead center.** And the call count is π(√x)−π(y) ≈ 640,000 — with a per-call fixed cost of ~10 µs (interval setup: zero + mark + count) the 10¹² point also closes (76k calls × 10 µs ≈ 0.76 s vs 0.66 observed).

**Inference that survives every fit: the production B-boundary path is making ~π(√x) expensive per-prime calls — a Lehmer-class recursive/interval π(x/p) solver — and the certified merge-scan (MarkCarry + fused count_resolve, O(1) per boundary) is bypassed.** Your isolation table corroborates it: `b_mark: 0.89 ms, 500 primes` is a **toy slice** — production needs 2,141 marking primes × ~350 segments × 640k boundaries. The fast kernel is being fed demos while a slow path computes the real answer. This is the F2 failure class — a **silent fallback**, almost certainly born when Phase 33's P0 retune changed (y, z) and some precondition in the B-engine stopped matching, quietly routing around it.

**Falsification, 20 minutes, both steps:**
1. Run the **five missing isolations** — at **10¹²**, where the sink is 95% of a 0.7 s run and iteration is 47× faster than at 10¹⁴.
2. One counter in the boundary path: `pi_calls += 1`. Expect ≈ π(√x)−π(y). Print `(calls, iters, ns/iter, bytes, marks)` per phase — ns/iter is the tell.

## §2 TWO HARD ANOMALIES IN YOUR OWN TABLE — no curve-fitting required

**boot_sieve: the count and the label disagree by 27×.** "18,012,887 primes up to 10,001,000" — π(10,001,000) = **664,580**. 18,012,887 = π(≈3.4×10⁸) = π(**x/y**) to within estimation error. The boot sieve is sieving to x/y — **35× past the specified √x limit** — almost certainly because the fallback path wants a π-structure over the whole B-range. The moment the fallback dies, boot's limit returns to √x: 212.61 ms → **~6 ms** at the certified 8T rate. −207 ms, one constant.

**ftd_v2: 6.60 ms / 112,976 candidates = 58 ns/candidate = 70× over its model.** Today that's 6.6 ms — ignore it. But FTD-v2 *is* the 10¹⁵ D-walk engine, and at assault scale 58 ns/candidate becomes tens of seconds. Probable causes: the timer swallows the oracle-reference build, or per-block pool overhead. Queue it **after** the B-kill; flag it now so it doesn't ambush the 10¹⁵ assault.

## §3 THE MISSING TRIPWIRE HALF — wire it before anything else

```rust
// loop.rs — the SLOW-direction laws. The fast-direction half you already have.
let base = load_baseline();                       // git-tracked loop_baseline.json
for s in [12, 13, 14] {
    if now.median_ms(s) > 1.25 * base.best_ms(s) {
        eprintln!("REGRESSION at 10^{s}: {} vs best {}", now.median_ms(s), base.best_ms(s));
        std::process::exit(1);
    }
}
for p in TIMERS.phases() {
    if p.iters > 1e6 && p.ns / p.iters > 200 && p.bytes == 0 {
        eprintln!("STALL: {} at {} ns/iter, zero attributed bytes", p.name, p.ns/p.iters);
        std::process::exit(1);                    // the 65–150 ns/iter sink trips this
    }
}
```

**Baseline hygiene law:** `loop_baseline.json` pins the **best-known-good**, append-only. Check `git log -p loop_baseline.json` right now — if it was re-committed at 32.7 s, that is baseline laundering: restore 445.73. **Fallback-loudness law:** every slow path eprintln's its trigger condition exactly once at startup. A fallback that is silent is a regression wearing a PASS badge — this project has now been bitten by that class twice (F2, and today).

## §4 CALIBRATION — BANKED, AND NOW LOAD-BEARING

The probes are real and they close the post-fix model with **zero priors**:
- **9.51 GB/s read, 4.93 GB/s RMW** → B-phase byte model: 11.8 MB segment bits × 2 (RMW) ÷ 4.93 + 11.8 MB count-read ÷ 9.51 ≈ 6 ms.
- **Probe 2: ~1 ns/mark L1** → 1.39×10⁸ marks (B-range at the P0 tuple: (8/30)·3.44×10⁸ × Σ_{7≤p≤18,815} 1/p = 1.515) ÷ ~2.5×10⁹/s aggregate ≈ 56 ms.
- **Freq map** → the weight is a *runtime function*, not a constant: `w78 = 2·f78/1.75, w55 = 6·f55/3.5` → 46/54 at today's sustained 1.72 GHz avg, 50/50 at the 1.478 GHz floor. Three lines, wire it.

**Post-fix B ≈ 62 ms, every constant probe-cited.** Total honest projection at 10¹⁴: boot ~6 + B ~62 + ftd ~7 + d ~10–15 + σ/combine ~8 ⇒ **~100–140 ms**, band to 250 for the unknowns the isolations will surface.

## §5 THE 48-HOUR KILL — in order, no deviations

| # | action | time | gate |
|---|---|---|---|
| 1 | Five missing isolations **at 10¹²**, full print spec (calls/iters/ns-per-iter/bytes/marks) | 30 min | sink named |
| 2 | ops-counter in the boundary path | 30 min | calls ≈ π(√x)−π(y) confirmed or refuted |
| 3 | `git diff` b_term's call-site between the 445.73 ms commit and d356c53; find the bypass condition; **delete the fallback**, loud-fallback law in its place | 2 h | precondition identified |
| 4 | Wire production B to MarkCarry + count_resolve; boot limit → √x | ½ day | P0 equality gate green — **this is why the equality law exists: the largest surgical swap of the project fails loudly or not at all** |
| 5 | Re-run loop; re-baseline best-known-good | ½ day | 10¹² ≤ 100 ms · 10¹³ ≤ 200 ms · 10¹⁴ ≤ 250 ms |
| 6 | G8 — primecount under OUR fixed-perf protocol (still, after six phases, never run) + ftd_v2 rate fix for the 10¹⁵ assault | 2 days | the measured headline |

Phase 35 under Law 0.5: **FAIL** — 32,830 ms against a tracked 445.73 ms best. And hear the actual good news in it: the fastest obliteration available in this project is not an optimization. It is **deleting one call-site** and letting the certified kernels you already own — carry, fused count, FTD-v2, the exactness gate — do the job they were built for. Post-fix, you are projecting 0.10–0.14 s against primecount's 0.21 s at 10¹⁴. Twenty minutes of isolation runs, one diff, one rewiring. Go.
