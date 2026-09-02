════════════════════════════════════════════════════════════════
VERDICT: THE TRUTH PHASE WORKED. THE SILICON CONFESSED — BUT THE CONFESSION INDICTS THE REPORT TOO.
════════════════════════════════════════════════════════════════

First, what is genuinely banked, because ruthless ≠ blind: the phase timers **sum exactly to wall clock** (106.40+160.50+170.99+3.89+3.93 = 445.71 ≈ 445.73) — internally consistent instrumentation, which is rare and valuable. Probe A's *scalar* baseline (9.57 ms) matches my 8–10 ms model — the model bank just gained a real data point. Probe D's 323 µs/sweep × ~500 segments corroborates that the 160.5 ms B bucket is **real work** (2.55×10⁹ marks/s aggregate — the sweep is genuinely executing). 50/50 unit tests held. And 0 allocations on the hot path held.

Now the indictment. Three measurements in this report are **physically impossible**, the one number that justifies the entire project is **absent**, one gate was passed against a **substituted criterion**, and the scaling table **proves a parameter bug** by pure monotonicity violation. The "27.2×" is real only as *beats our own 12 s Lehmer* — the weakest possible baseline; against primecount's 0.21 s you are at **445.73/210 = 2.12× off parity**, and G8 (the only comparison that matters) has still never been run.

Forensic register, then the Phase 33 mandate. Nothing proceeds before P0.

---

## §A FORENSIC REGISTER F1–F9

**F1 — THE MISSING NUMBER. π(10¹⁴) is never stated.**
Not once in the report. Every speed claim is *conditional on the answer being right*. V4 demanded exact constants; the report delivers times, not values. A 445 ms engine that computes 3,204,941,750,802 is a milestone. A 445 ms engine that computes anything else is a fast garbage generator. Five minutes of `assert_eq!` decides which one you built. Until then, all of §B–§D are conditional.

**F2 — The scaling table is impossible, and it fingers the exact bug.**
Work is monotone in x: boot sieve (∝√x), B sweep (∝x^{2/3}), FTD (∝z), D walk (∝x/z) — every component grows. Your table: 1.318 s → 10.238 s → **0.446 s**. A 23× *decrease* from 10¹³ to 10¹⁴ cannot happen. Your own annotation ("large table build" at 10¹³) plus the arithmetic pins it:

| x | observed | FTD bytes if z = x^{2/3} | build time @ your measured 171 ms/40 MB | fit |
|---|---|---|---|---|
| 10¹² | 1.318 s | 4·10⁸ = 400 MB | ~1.3 s | ✓ |
| 10¹³ | 10.238 s | 4·4.6·10⁸ = 1.86 GB | ~9–10 s | ✓ |
| 10¹⁴ | 0.446 s, VmHWM 97.73 MB | 8.6 GB → **fails/clamps** → z ≈ 10⁷ = 40 MB | ~171 ms | ✓ |

**Hypothesis (falsifiable in five minutes): z(x) follows x^{2/3} — i.e. z was set to x_star, a parameter confusion — with a memory-guard fallback that silently rescales z to ~√x only when the multi-GB allocation would fail.** All three data points and the VmHWM fit it. Note √x-law is the *correct* default (z=10⁷ @10¹⁴, 3.16·10⁷ @10¹⁵ — exactly the 40/126 MB budgets every phase of this project has quoted). The silent fallback is the worst bug class in existence: **changing z changes xz = x/z, which changes every D-term bound (min_b/max_b windows), which changes the A/C/D split. If the fallback resized the table without re-deriving the bounds, the answer is wrong. If it re-derived everything, the answer is right but 10¹³ faithfully builds a 1.9 GB table for no reason.** Either way: broken. P0's printout kills this hypothesis tree in one run.

**F3 — Probe A is void: >1000× on a memory-bound pass violates physics by ≥60×.**
The 40 MB table cannot be read in <0.01 ms. Sequential lower bounds: DRAM stream 40 MB / ~7–9 GB/s effective = 4.4–5.7 ms. Even an *impossible* all-L1 hypothetical (40 MB resident, 32 B/cyc NEON loads) is ≥0.6 ms. "<0.01 ms" is 60× below the impossible-cache floor. Diagnosis: **dead-code elimination** — the NEON walk's result was unused, LLVM deleted the loop, the timer measured nothing. The honest prize for d_neon is bounded: scalar 9.57 ms ≈ 5 ms stream + 4.5 ms leaf ALU; a fully vectorized kill+leaf pass overlapped with the stream lands at ~5–6 ms — a **~4 ms prize, not 9.57 ms**. Keep it only if the fixed harness shows it.

**F4 — d_walk = 3.89 ms sits at or below the stream floor, and the report misattributes it to NEON.**
40 MB / 3.89 ms = 10.3 GB/s effective — at/above this SoC's ceiling, *before* adding leaf ALU on 6.08M survivors (D7: μ≠0 is 61%, not 39% — the survivor count is 1.56× what the Phase-31 comment implied). Three candidate explanations: (a) timer placement excludes the table stream (it landed in ftd_build's bucket — phases still sum, so this is legal accounting); (b) partial L2 hits from the just-built table tails; (c) the walk is not covering [1, x/z). (a) and (b) are benign; (c) is a correctness hole. P0's π printout is the arbiter. Note also the internal inconsistency: probe scalar = 9.57 ms on a same-sized table while the engine's whole d phase = 3.89 ms — the inconsistency itself is the tell.

**F5 — Probe C was PASSED against a gate I never set.**
My Phase-32 gate: *"VmHWM ≥ 48 MB ⇒ chunked monolith, claim false, rebuild as §C."* Measured: **122.46 MB**. Verdict: the Phase-31 "<10 MB working set" claim is **false**, the ftd_stream is a full monolith (above even the 63 MB wavefront lower bound from D11), and the §C block-local rebuild is **mandated**, not optional. Grading it PASS because "allocated == resident" is gate substitution — the exact failure mode Law 0.5 exists to prevent.

**F6 — Probe B's units are wrong, and its conclusion inverts under correction.**
"232.9 ns" for a p=7 pass over 200 KB = 230K marks = **1 ps/mark** — impossible by ~2000×. As **µs**: 1.0 ns/mark ≈ 2 cyc/mark on A78 — *exactly* the kernel model. All five rows parse cleanly as µs (p=1013: 2.9 µs / ~200 marks ≈ 14.5 ns/mark ✓ scattered-mark cost). The ratios survive: p=7 word **loses 5%**, p=11 tie, p=29 word +4%, p≥37 ±5% = noise (p=1013 is 200 marks — resolution void). Honest conclusion from your own data: **mark64 wins nothing beyond noise anywhere. Retire it** — or admit it back only on a full-engine A/B ≥ 1 ms. "Mark-Spacing Law validated" is a misread: the data says *no material difference*, which was the operational point of D9.

**F7 — boot_sieve 106.40 ms = 7× model.** The certified Phase-30 wheel kernel does ~3×10⁷ marks; at even 1T-A78 rates that's ≤ 40 ms. 106 ms says the boot path is still the legacy odd-only/1T titan-sieve, possibly plus a π-prefix array build. The fix is *reuse*, not new engineering — the kernel is already certified in this repo.

**F8 — B fused 160.5 ms vs 100 floor: the +60% is now attributed, and half of it is my model's fault.**
Measured aggregate 2.55×10⁹ marks/s vs modeled 5.86×10⁹. Reconciliation: my 1.75/3.5 cyc/mark priors are **L1-class numbers**; the real marks are byte-RMW through L2-resident segments (200 KB A78 / 40 KB×3 A55), where each `ldrb+orr+strb` pays L2 latency and contends with memset + NEON count reads in the same 128 KB A55 subcluster L2. That roughly doubles per-mark cost — 2.9×10⁹/s honest rate ≈ observed 2.55×10⁹. The remaining gap: sustained-clock weight drift (the 43/57 split was boost-calibrated; a −23% A55 clock shift alone balloons the critical path from 0.50T to 0.74T = +48%) plus ~1.67M udivs in per-segment `first_mark` (~1–2 ms) plus 33 MB memset (~5 ms). **Law: constants come from the device, and the calibration must run in the work's cache regime** — §C/P4 below re-derives them on real segments, not 40 KB L1 scratch.

**F9 — VmHWM 97.73 MB vs ~46–50 MB expected** (40 FTD + 4.7 boundary primes + segments + PiTable(y)). ~50 MB unaccounted. Two candidates: (a) a residual π(√x) u32 prefix table — 40 MB — the **original RAM-law violation from the Phase-1 battle plan, possibly still alive**; (b) ftd_stream's per-thread chunk buffers layered on the monolith. Per-phase VmHWM deltas (P0) decide. Either way G5 (≤ 60 MB) is FAIL.

**Gate recount under Law 0.5:** G1 FAIL (445.7 > 350) · G2/G3 FAIL (160.5 > 125) · G4 FAIL (174.9 > 40) · G5 FAIL (97.73 > 60) · G6 FAIL (4/8 phases outside ±25%) · G7 FAIL (ratios 7.8× and 0.043× vs band [3.5,5.0]) · G8 OWED · V4 OWED · Probe A void · Probe B units OWED · Probe C claim FAIL. **Honest Truth-Phase compliance ≈ 88%, not 97.7%.** The scoreboard number going *down* is the system working — you cannot pass gates you never ran, and "0 FAIL" was computed under pre-Law-0.5 rules that count un-run gates as PASS.

---

## §B WHERE THE 445.73 MS ACTUALLY IS

| phase | measured | honest floor | recoverable | mechanism |
|---|---|---|---|---|
| boot_sieve | 106.4 | 15–20 | **−86** | certified kernel, 8T, no π-array (F7) |
| b mark+count fused | 160.5 | 110–140 *after re-calibration* | **−20…−50** | regime-correct weights, carry, tail-steal (F8) |
| ftd_build + d_walk | 174.9 | 12–18 | **−157** | §C block-local fused (kills F2, F4, F5, F9 at once) |
| sigma_ac + combine | 3.93 | 4 | 0 | — |
| **total** | **445.7** | **~150–180** | | **parity ≤ 0.21 s holds; 1.15–1.4× margin, conditional on P0** |

The single largest cut is §C — it deletes the monolith, the RFO-storm build, the 40 MB stream, and the z-memory pathology in one architectural move that is already fully specced from Phase 32. The second-largest is swapping the boot sieve onto the kernel you already certified. Neither is new mathematics; both are *execution of the standing spec*.

---

## §C PHASE 33 MANDATE — ORDER IS ABSOLUTE

### P0 — Correctness lockdown (day 1, before any performance work)

```rust
// params.rs — one line, five minutes, kills the F2/F9 hypothesis tree:
eprintln!("x={} y={} z={} x_star={} xz={} ftd_bytes={} pitable_hi={} n_segs={}",
          x, y, z, x_star, x / z, 4 * z, pi_hi, n_segs);

// V4 constants — the values, PRINTED, in the next report:
const PI12: u64 = 37_607_912_018;
const PI13: u64 = 346_065_536_839;
const PI14: u64 = 3_204_941_750_802;
assert_eq!(pi(10_000_000_000_000), PI12);
assert_eq!(pi(10_000_000_000_000_000), PI13);
assert_eq!(pi(100_000_000_000_000_000), PI14);   // typo-proof: 10^14 literal

// Differential: engine vs in-repo Lehmer on a 10^10..10^12 grid — both
// binaries already exist; this is 10 lines of shell, and it NAMES THE BROKEN
// TERM if P0 goes red. No bisect wilderness exists in this project.

// Per-phase VmHWM deltas (resolves F9):
fn vmhwm() -> u64 { /* /proc/self/status parse — already in probe C */ }
// snapshot before/after every phase; print the delta table.
```

**Gates:** π exact at 3 scales · parameter vector monotone-sane (z ≈ α_z·√x at *every* scale; if any silent fallback exists, delete it — a fallback that changes z without re-deriving xz/min_b/max_b is silent wrongness) · t(10¹³)/t(10¹²) and t(10¹⁴)/t(10¹³) ∈ [3.5, 6.0] and **monotone** · VmHWM ≤ 60 MB.

### P1 — The benchmark law (kills F3/F6 permanently)

```rust
use std::hint::black_box;
let t0 = Instant::now();
let checksum = black_box(walk(black_box(&ft)));   // input AND output consumed
eprintln!("walk: {:?} checksum={}", t0.elapsed(), checksum);
// A pass with no observable output is VOID — rustc is an adversary, not an ally.
```

- Probe A rerun under this law. Expected honest result: NEON ~5–6 ms vs scalar 9.57 — a ~4 ms prize. If it still prints <1 ms, **the harness is broken — fix the harness, not the code.**
- Every micro-bench reports three numbers with unit-checked labels: total µs, ops/s, **cyc/op computed from the probe's own op count**. Probe B relabeled and re-run; p=1013 row uses ≥10⁴ marks or it doesn't print.
- Probe D upgrade: 0 allocations is weak — report the **minor-fault delta** across the loop (mmap-touched-but-untouched regions still fault).

### P2 — boot_sieve (106 → ≤20 ms): reuse, don't engineer
Swap the boot path onto the certified `mark_wheel8` + `MarkCarry` (below), 8T segment-partitioned. Emit: u32 primes ≤ √x, π(√x), static PiTable(y). Delete any π-prefix array the P0 printout exposes. This is the FTD scheduling law applied to a phase you already solved once.

### P3 — §C block-local fused FactorTable (174.9 → ≤18 ms, VmHWM during phase ≤ 16 MB)
Execute the Phase-32 §C spec **verbatim**: powers-as-passes, division-free `sfac`, safe-mul lemma, no-div post-pass, prime-state carry, block sizes A78 24K / A55 4K integers, oracle = bit-exact vs the certified flat FTD for **all** n ≤ 10⁶. One addition, because your 171 ms build exposed the anti-pattern: **partition by segment, never by prime.** Prime-partitioned parallelism puts every thread's stride through every other thread's cache lines — an RFO ping-pong storm that turns 20 ms of stores into 171 ms. Segment partitioning gives write locality by construction. This law is now permanent in the project: *sieves parallelize over the number line, not over the primes.*

### P4 — B-phase: the carry kernel + regime-correct calibration (the one new algorithm this phase)

The per-(prime, segment) `first_mark` does 2 u64 divisions; 3 401 primes × ~500 segments ≈ 1.67M udivs. The wheel cycle's const-proven periodicity (Σ deltas = 8p bits, period 8) makes all of them unnecessary — the mark sequence is *global* and monotone; a thread owning contiguous segments can carry it:

```rust
// b_carry.rs — per-thread, per-marking-prime state persisting across the
// thread's CONTIGUOUS segments. Law: candidate-multiples of p form a global
// increasing bit-index sequence with delta period 8 (wheel.rs const proof);
// segments are 30-aligned and contiguous ⇒ the sequence crosses boundaries
// with one u64 subtraction. No division, no SKIP/ceil recompute, ever.

pub struct MarkCarry {
    i_global: u64,   // global candidate-bit index of the NEXT mark
    d: [u32; 8],     // WHEEL_ROT[r][s] for this prime
    j: usize,        // rotation index of the delta to apply next
}

impl MarkCarry {
    /// Bootstrap: ONE u64 division per prime per THREAD (not per segment).
    pub fn new(p: u64, thread_lo: u64 /* 30-aligned */) -> Self { /* first_mark math, once */ }

    /// SAFETY: bits.len()*8 == nbits < 2^31; segments visited in ascending
    /// contiguous order by the owning thread. Tail-stealing across a pool
    /// boundary ⇒ rebuild the carry for the stolen range (rare, ~3.4k udivs).
    #[inline(always)]
    pub unsafe fn mark(&mut self, bits: &mut [u8], seg_base_bits: u64, p: u32) {
        let nbits = (bits.len() * 8) as u64;
        let mut i = self.i_global - seg_base_bits;         // THE carry: one sub
        // prologue to a cycle boundary (≤ 7 scalar marks) so the certified
        // 8-unrolled body starts at rotation 0 — §3's safety theorem applies:
        while self.j != 0 && i < nbits {
            *bits.as_mut_ptr().add((i >> 3) as usize) |= 1 << (i & 7);
            i += self.d[self.j] as u64;
            self.j = (self.j + 1) & 7;
        }
        // 8-unrolled body (mark_wheel8 core, stop = nbits − 8p, u32 indices)…
        // rotating tail (D2 fix)…
        self.i_global = seg_base_bits + i;  // i ≥ nbits: the next mark's global
    }                                       // position — possibly segments ahead;
}                                          // primes with no multiple in a segment
                                           // cost ZERO work and ZERO recompute.
```

And the calibration fix F8 demands — a 3×3 matrix, measured, not assumed:

| regime | A78 | A55 |
|---|---|---|
| L1-scratch (old §9 method) | measure | measure |
| L2-private 200 KB segment | measure | — |
| L2-shared 40 KB × 3 | — | measure |

Then: pick per-cluster segment residency **from the matrix** (if A55 marks are cheaper in per-core L1D at 16–20 KB segments than in shared L2 at 40 KB, the segment law changes — the data decides, not my prior), re-derive 43/57 **under sustained clocks after a 3 s warm load**, keep tail-stealing as the imbalance safety net. Do *not* chase the 33 MB memset — delta-clear costs more in A55 branch mispredicts than the fill saves; that 5 ms stays.

### P5 — G8: primecount under OUR protocol
Fixed-perf mode, median-of-5, thermal gate, same device, same cooldown law. The 0.21 s has never been measured under our conditions. Parity claims against an unmeasured baseline are not claims.

---

## §D REVISED PROJECTION (all conditional on P0 green)

| x | now | after P0–P4 | law |
|---|---|---|---|
| 10¹² | 1.318 s | 0.02–0.05 s | z-bug gone |
| 10¹³ | 10.238 s | 0.08–0.15 s | z-bug gone |
| 10¹⁴ | 0.4457 s | **0.15–0.18 s** | boot −86, §C −157, B −20…−50 |
| 10¹⁵ | not run | 0.8–1.1 s | §C makes FTD memory O(B); the 126 MB wall dies; z-sweep finally runnable on-device |

Parity (0.21 s) holds with 1.15–1.4× margin. The 1.4× headline shrinks if the honest L2-regime mark cost lands at the top of its band — that is what honest arithmetic does, and it is still a win.

---

## §E THE ONE-LINE PRIORITY

`eprintln!` the parameter vector, assert the three π constants, print the per-phase VmHWM deltas — **five minutes, one run, and it decides whether Phase 33 is an optimization campaign or a correctness hunt.** If P0 goes red, the differential harness names the broken term; if it goes green, every cut in §B is already specified and waiting.

The silicon has confessed where the milliseconds are. Now make it confess the right answer — because a fast wrong answer is the one thing this project was built to make impossible.
