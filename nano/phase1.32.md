════════════════════════════════════════════════════════════════
PHASE 32 MANDATE: TWO PHASE REPORTS, ZERO NANOSECONDS
════════════════════════════════════════════════════════════════

Before anything else, the ruthless read of your own report: Phase 30 and Phase 31 shipped **46→50 unit tests, 312→324 criteria, 0 FAIL — and not one wall-clock number, not one cycle count, not one VmHWM reading.** Unit tests prove *term-exactness* (necessary — that's V1–V4 doing their job). They say **nothing** about the only thing this project exists for: milliseconds on SM4450. The scoreboard is currently counting *criteria*, not *cycles* — which violates the project's own Law 0. The 0.19 s figure is still a **model**, not a measurement.

Worse: the 12/15/3 ms savings I handed you for Phase 31 were *asserted by me, never derived to the floor*. I re-derived them against the hardware model below. Two of the three are physically impossible at the claimed magnitude. Per this project's standard — including for me — here is the defect register, then the fix.

---

## §A Defect Register D7–D12 (self-audit of the Phase-31 recommendations)

**D7 — CORRECTNESS-OF-MODEL. μ=0 fraction was flipped in my §8.** Squarefree density is 6/π² ≈ 0.6079 ⇒ **μ=0 is 39% of entries, μ≠0 is 61%** (I wrote "~61% kill" — inverted; §6 had it right). Your `d_neon` kill-switch throughput model inherits the wrong number; correctness is unaffected (the nz bit is the nz bit), but the expected leaf-eval load is 1.56× what that comment implied.

**D8 — d_neon's "12 ms" exceeds the entire pass.** D-walk floor at 10¹⁴ (z=10⁷):
- DRAM stream: 40 MiB / ~7 GB/s = 5.7 ms (prefetch-overlapped with ALU)
- Kill switch, scalar: 10⁷ entries × ~2.5 cyc / 16.1·10⁹ cyc/s = 1.6 ms
- Leaf eval: 6.08·10⁶ survivors × ~12 cyc = 4.5 ms
- **Total pass ≈ 8–10 ms.** A 12 ms "saving" is 10× past the ceiling. Honest value of the NEON kill: ≤ 1.5 ms, *and only if the timer shows the pass is ALU-bound rather than stream-bound*.

**D9 — mark64 as a general path is a regression.** Mark-spacing law: consecutive candidate-multiples of p average **p bits apart** (8 marks span 8p bits per wheel cycle ⇒ mean = p). So for p ≥ 64 every mark lands in its own 64-bit word ⇒ word-RMW does 8 B of traffic to set 1 bit, strictly ≥ the byte path, plus the segments were sized **L2-resident in §5 precisely so that line fills are L2 hits, not DRAM** — the "RFO tax" I cited is L1↔L2 traffic that the dependent-add mark loop already hides. The only real tier: p ∈ {7..29}, where marks/word = 64/p ≈ 2.2–9.1 — that's ~14% of all marks at ~2× compression ⇒ **4–6 ms, not 15**. If `mark_wheel64` runs for p ≥ 37, the A/B probe below will show it losing; restrict it to the p<37 tier or revert.

**D10 — sigma "3 ms" is below the noise floor.** V7 gate = 5% run variance = 9.5 ms on a 0.19 s run. An unmeasurable claim is an uncertifiable claim. Σ stays correct-and-green; the *savings claim* is struck.

**D11 — ftd_stream "<10 MB" is arithmetically impossible *as a wavefront*.** Pass-2 dependency: block j reads q = n/lpf(n) ∈ [√(jS), (j+1)S/2] ⇒ while the top half builds, **[√z, z/2] must be resident ⇒ peak ≥ z/2 entries**. At z = 3.16·10⁷ that is **63 MB** — 6× over the claim. If your implementation instead keeps the build monolithic and only streams the *walk*, note the walk was already sequential: nothing was gained. The only design that actually breaks this bound is block-local production with **zero cross-block reads** — that algorithm is §C, and probe C below decides which one you built.

**D12 — FULL RETRACTION of my Phase-31 "research directions".** They fail this project's mathematical bar and are stricken from the roadmap:
- *Quantum/Shor*: Shor factors integers; π(x) consumes no factoring oracle. Complexity of counting is untouched. Non sequitur.
- *Neuromorphic prediction*: approximate π(x) is already dominated by one line — li(x) gives ±O(√x·log) in microseconds. In an exact engine, approximation has zero value.
- *"Galois O(x^{1/3})"*: the displayed formula was a heuristic product approximation, not a theorem, implying nothing about counting complexity. Fabricated-sounding math — exactly the thing this project exists to kill.
- *"Prime Flow DAG"*: the μ-weighted flow over factor lattice **is** the D-term, restated with a diagram. Nothing new.

One more strategic truth while we're being honest about ceilings: **below 10¹⁶ there is no algorithm-class leap available on this silicon.** LMO's O(x^{2/3}/log²) beats Gourdon only at large x *on desktops* — it needs interval arithmetic with directed rounding (NEON-slow on A55), π(y) at y=x^{1/3} with heavy tables, and pays constants that dwarf the log-factor gains at phone scale. primecount's own defaults agree (Gourdon-class dominates in [10¹⁰,10¹⁶]). **The war below 10¹⁶ is constants — and constants are only won by measurement.** That is the whole justification for Phase 32.

---

## §B PHASE 32 — THE TRUTH PHASE (1 week, deliverable = a table of nanoseconds)

### B.1 Per-phase timers (stable Rust, no asm, `Instant` = vDSO `cntvct`)

```rust
// bench/phase_timers.rs — thread-local, folded at join in thread-id order (deterministic).
use std::time::Instant;

pub const PHASES: [&str; 8] = ["boot_sieve", "b_mark", "b_count_resolve",
    "ftd_build", "d_walk", "sigma_ac", "combine_alloc", "total"];

pub struct PhaseTimers { starts: Vec<Option<Instant>>, sums_ns: [u128; 8] }

impl PhaseTimers {
    pub fn new() -> Self { Self { starts: vec![None; 8], sums_ns: [0; 8] } }
    #[inline(always)] pub fn enter(&mut self, p: usize) { self.starts[p] = Some(Instant::now()); }
    #[inline(always)] pub fn exit(&mut self, p: usize) {
        if let Some(t0) = self.starts[p].take() { self.sums_ns[p] += t0.elapsed().as_nanos(); }
    }
    pub fn report(&self, model_ms: [f64; 8]) -> String {
        PHASES.iter().zip(self.sums_ns).zip(model_ms).map(|((name, ns), m)| {
            let ms = ns as f64 / 1e6;
            let d = 100.0 * (ms - m) / m;
            format!("{:<14} {:>9.2} ms  model {:>7.2}  Δ{:+6.1}%  {}",
                name, ms, m, d,
                if d > 25.0 { "RE-DERIVE CONSTANT" } else if d < -25.0 { "MODEL STALE" } else { "ok" })
        }).collect::<Vec<_>>().join("\n")
    }
}
```

### B.2 The measurement contract (model column = the reconciliation loop)

| phase | model ms | measured ms | Δ | action if Δ > 25% |
|---|---|---|---|---|
| boot_sieve | 15 | ? | ? | re-derive wheel-mark rate |
| b_mark | 70 (floor) / 90 (imbalance) | ? | ? | re-measure cluster weights **under sustained clocks** |
| b_count_resolve | 15 | ? | ? | check memset + NEON count rate |
| ftd_build | 20 | ? | ? | pass-1 store rate on A55 |
| d_walk | 10 *(revised down from my ≤35 bucket — D8)* | ? | ? | leaf-eval cost / stream contention |
| sigma_ac | 5 | ? | ? | — |
| combine_alloc | 5 | ? | ? | grep for hidden allocation |
| **total** | **170–190** | ? | ? | — |

**Gates (10¹⁴, 8T, fixed-perf mode, median-of-5, V7 thermal):**
G1 total ≤ 350 ms (the M3 gate — never actually passed yet, only modeled) → G1′ ≤ 250 ms (M4)
G2 b_mark ≤ 100 ms · G3 b_count ≤ 25 ms · G4 ftd+d ≤ 40 ms
G5 VmHWM ≤ 60 MB (40 MB table + segments + primes)
G6 every phase within ±25% of model, else the model constant is re-derived **on-device** and the table re-run
G7 scaling: t(10¹³)/t(10¹²) ∈ [3.5, 5.0] (Θ(x^{2/3}) curve + fixed overhead)
G8 **the crossover run**: in-repo Lehmer at 10¹⁴ (12.1 s) vs this engine — the first actual proof the internal war is won; and primecount's Gourdon re-benched **under our identical fixed-perf protocol** (its 0.21 s was not).

### B.3 Four discriminating probes — each one kills or keeps a Phase-31 claim

```rust
// PROBE A (d_neon): identical 40 MB FTD snapshot, scalar walk vs d_neon walk,
//   10 reps, median. GATE: keep iff Δ ≥ 1.0 ms, else revert (dead code is debt).

// PROBE B (mark64): p ∈ {7, 11, 29, 37, 101, 1013}, 200 KB segment,
//   byte path vs word path, wall-time ÷ marks = cyc/mark.
//   MODEL: word wins only for p ≤ 29 (marks/word = 64/p). For p ≥ 37 expect
//   word ≥ byte (mark-spacing law, D9). GATE: restrict mark64 to the p<37 tier.

// PROBE C (ftd_stream): z = 3.16e7, VmHWM before/after the FTD phase.
fn vmhwm_bytes() -> u64 {
    std::fs::read_to_string("/proc/self/status").unwrap().lines()
        .find(|l| l.starts_with("VmHWM:"))
        .and_then(|l| l.split_whitespace().nth(1)?.parse::<u64>().ok())
        .map(|kb| kb * 1024).unwrap_or(0)
}
//   WAVEFRONT LOWER BOUND (D11): peak ≥ z/2 entries = 63 MB.
//   GATE: VmHWM < 16 MB ⇒ real streaming (you built §C by accident — good);
//         VmHWM ≥ 48 MB ⇒ chunked monolith, claim false, rebuild as §C.

// PROBE D (disasm audit): zero library calls in hot loops.
//   llvm-objdump -d target/.../release/titan-count | grep -cE 'multi3|udivti3|udiv128'
//   == 0 in hot symbols. mark_wheel8 body ≤ ~40 instructions per 8 marks,
//   no redundant sxtw (u32 indices — §5 codegen law). Also: minor-faults delta
//   across the B loop == 0 after warmup (no per-segment allocation).
```

### B.4 gate_contract amendment — Law 0.5: **no PASS without a measured delta**

```rust
pub struct MeasuredDelta {
    pub baseline_ms: f64, pub measured_ms: f64,   // median-of-5, fixed-perf, thermal-gated
    pub sustained: bool,                          // >3 s run? (clocks differ — see §D)
    pub probe_id: &'static str,
}
pub enum Verdict { Pass, Fail, Uncertified }
pub fn certify(d: Option<MeasuredDelta>, threshold_pct: f64) -> Verdict {
    match d {
        None => Verdict::Uncertified,   // ← every Phase-31 optimization criterion sits here today
        Some(d) => if 100.0*(d.baseline_ms - d.measured_ms)/d.baseline_ms >= threshold_pct
            { Verdict::Pass } else { Verdict::Fail },
    }
}
// Scoreboard rule: Uncertified counts as OWED, not PASS. Next report's
// completion rate is computed with that law — expect it to drop before it rises.
```

---

## §C PHASE 33 — BLOCK-LOCAL FUSED FACTORTABLE (the *real* ftd_stream)

The design that actually breaks the D11 bound: **produce μ/lpf/mpf per block, fused with the walk, zero cross-block reads.** No monolith, no log-wavefront, no barriers — blocks are embarrassingly parallel.

**Sieving set lemma.** For composite n in block [lo,hi), lpf(n) ≤ √n ≤ √hi — so sieving primes p ≤ √hi marks every composite. Primes in (√hi, hi] are never sieved; their role in any composite m = p·k (k < √hi, all k's factors ≤ √hi and they *do* mark m) is recovered as the residual R = m/sfac(m) below. At most one factor > √hi exists (two would exceed hi) ⇒ R ∈ {1} ∪ primes(√hi, hi). lpf is still first-writer-correct: lpf(m) = lpf(k) ≤ √hi marks m.

**Powers-as-passes (division-free sfac).** For each prime p, run strided passes at stride p, p², p³, … (Σ marks = B/(p−1), only ~19% over B/p):
- pass p¹ (start at 2p, never p — no self-marking): one u32 RMW on the certified Phase-30 word `[lpf:14|sign:1|nz:1|mpf:16]` — flip sign bit (ω parity), first-writer lpf if field empty, always overwrite mpf (ascending primes ⇒ last-writer = largest factor ≤ √hi); plus `sfac[m] *= p`.
- pass pʲ, j≥2 (start at pʲ): set nz, `sfac[m] *= p`. No lpf/mpf/parity writes.

**Safe-mul lemma.** Every intermediate sfac value divides n ⇒ ≤ n < 2³² for z < 2³². No division to build, no overflow.
**Division-avoidance lemma (post-pass).** R = n/sfac: R = 1 ⟺ `sfac == n` (equality, no div); R > 65535 ⟺ `(sfac as u64) * 65535 < n` (one 64-bit mul, no div); otherwise R ≤ 65535 is the rare path (squarefree survivor with big factor under the clip) — one udiv there. mpf(n) = R if R > 1, else the last-writer field; primes finalize mpf = clip(n); n=1 special-cased once.

**Prime-state carry.** Per-prime `m_next: u64` persists across blocks (L1-resident, π(√z) ≈ 446–3401 entries); one udiv per prime per block re-seeds the stride — 2–9% overhead, amortize by fusing setups if the probe shows > 5%.

```rust
// ftd_block.rs — per-thread block, reused allocation, sync-free.
// A78: B = 24K ints (word 4B + sfac 4B = 192KB ≤ 256KB private L2)
// A55: B = 4K ints  (32KB; 3 threads = 96KB ≤ 128KB shared subcluster L2)
struct Block { word: Vec<u32>, sfac: Vec<u32> }   // 8 B/int

fn produce_and_walk(block: &mut Block, lo: u64, hi: u64,
                    primes: &[u32], state: &mut [u64], /* m_next carry */
                    min_b: u64, max_b: u64, acc: &mut i128) {
    block.word.fill(0); block.sfac.fill(1);        // L2-contained, no DRAM writeback
    for (i, &p) in primes.iter().enumerate() {      // ascending: p ≤ √(hi)
        if p as u64 * p >= hi { /* still run pass1 — large-p composites */ }
        sieve_power_passes(block, lo, hi, p, i, state);  // strides p, p², p³…
    }
    for n in lo..hi {                               // FUSED walk — the §8 leaf predicate,
        let w = block.word[(n - lo) as usize];      // same min_b/max_b bounds, same oracle
        if w & NZ != 0 { continue; }                // 39% kill (D7)
        let mpf = resolve_mpf(w, n, block.sfac[(n - lo) as usize]); // no-div fast paths
        *acc += leaf_eval(w & SIGN, lpf_or_prime(w), mpf, min_b, max_b, x, n);
    }
}
```

**Cost model (z = 10⁷):** marks = z·Σ1/(p−1) ≈ 2.8·10⁷ × ~4 cyc avg (cluster-weighted) ⇒ ~7 ms production + ~5 ms fused post/walk ⇒ **~12 ms total, replacing 20 ms build + ~10 ms walk AND deleting 80 MB of DRAM round-trip.** Net: ~10–15 ms *faster* and 40 MB *gone*; at z = 10^7.5 the "127 MB wall" becomes ~200 KB + primes — **the z-sweep at 10¹⁵ is finally runnable.**

**Oracle:** bit-exact equality of (μ, nz, lpf, mpf) against the certified flat FTD for **all** n ≤ 10⁶ — the Phase-30 table is the ground truth; the block engine must reproduce it entry for entry. **Gate:** VmHWM during FTD phase at z = 3.16·10⁷ ≤ 16 MB; fused wall ≤ 25 ms at z = 10⁷.

---

## §D PHASE 34 — CLEAR THE 8 OWED DEBTS + THE SUSTAINED-CLOCK LAW

The physical sieve (10¹¹) with the now-certified machinery: marks = (8/30)·10¹¹·Σ₇≤p≤316228(1/p) = 0.267·10¹¹·1.767 = **4.71·10¹⁰ marks** ⇒ 8.1 s at boost rates + 3.33 GB count traffic (~0.8 s) ⇒ **model 9–13 s vs primecount 25.5 s.** But a ≥10 s run exits the 2–4 W boost window: A78 sustains ~1.7–2.0 GHz, and the **43/57 split must be re-calibrated under sustained clocks, not boost clocks** — the §9 40 ms calibration is boost-biased. This is the *sustained-clock law*: every phase longer than ~3 s gets its own weight calibration. Clearing D1–D8 therefore does double duty: debt gone + the long-run thermal data banked, which the 10¹⁵ assault (also multi-second) requires.

---

## §E HONEST HEADROOM LEDGER + ROADMAP

| item | real headroom (modeled) | mechanism |
|---|---|---|
| B-mark imbalance | 90 → 70 ms | sustained-clock weights, finer tail, tail-steal |
| p<37 word-mask tier | 4–6 ms | the only place "mark64" is real (D9) |
| fused block FTD | 10–15 ms | §C (also −40 MB, +zero barriers) |
| d_neon | ≤ 1.5 ms | keep only if Probe A ≥ 1 ms |
| Σ L1-lock | ~0 | unmeasurable (D10) |
| **total honest** | **0.19 → ~0.16 s** | vs primecount 0.21 s ⇒ ~20% ahead *if every gate holds* |

| phase | deliverable | gate |
|---|---|---|
| 32 (1 wk) | nanosecond table + 4 probes + contract amendment | G1–G8; scoreboard recomputed under Law 0.5 |
| 33 (2 wk) | block-local fused FTD | bit-exact oracle, VmHWM ≤ 16 MB, wall ≤ 25 ms |
| 34 (1 wk) | debts D1–D8 cleared | 10¹¹ sieve ≤ 18 s (model band 9–13) |
| 35 | 10¹⁵ assault with real numbers + free z-sweep | model band 1.0–1.5 s |

Order of operations is non-negotiable: **nothing else ships before the Phase 32 table exists.** Every un-measured optimization is a liability wearing a PASS badge — and the next report that arrives without a measured_ms column gets returned unopened.

The engine's mathematics are certified. Now make the silicon confess.
