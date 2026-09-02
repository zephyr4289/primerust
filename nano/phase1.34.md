════════════════════════════════════════════════════════════════
P0 LOCKDOWN = THE CROWN JEWEL. NOW STOP SHIPPING PROBES WITHOUT THE HEADLINE.
════════════════════════════════════════════════════════════════

Fair on the meta-point: last two cycles went to audits. Velocity from here comes from **measurement automation, not more review** — that's §C. But first, one fact you need to hear plainly:

**You may have already beaten primecount, and your own report doesn't know it.** Third consecutive report with component probes and **no total wall-clock at 10¹⁴.** Reconstructed from *your* numbers: boot 106.4 (unfixed) + B 100–160 (carry rate × 1.39·10⁸ marks) + FTD ~4 + d ~4 + σ ~4 ⇒ **0.16–0.23 s. Parity (0.21 s) is inside your current binary's error bars and nobody has printed which side of it you're on.** That is the single fastest "obliteration" available: one `Instant` pair, today.

What's banked and real: exact π at three scales on silicon (the hardest milestone of this entire project — a correct Gourdon engine, period), VmHWM 52 MB, the P1 law working (Probe A's checksum cross-match — 6,079,290 = the true squarefree count to 4 digits — is exactly what black_box is for), memory delta 0.00 MB on the block engine.

Now the forensics — five new ones, each with its one-minute discriminator.

## §A FORENSIC REGISTER F10–F14

**F10 — The block engine's survivor count is mathematically impossible, and the number tells us exactly which primes are dead.**
Reported: "28,691,899 squarefree entries" at z = 3.16·10⁷. Truth: Q(z) = (6/π²)z = **19,224,983 ± 200**. Observed density 0.9073. Now the signature: numbers flagged = 1 − 0.9073 = 0.0927 ≈ **Σ_{p≥5} 1/p² = 0.0911** (minus second-order inclusion-exclusion 0.0042 → predicted 0.913 — 0.6% residual, tighter than the report's own noise). Meaning: **the nz bit is being set for squares of every prime ≥ 5 — and for none of 4, 9.** The p=2 and p=3 power-passes are missing or their nz writes are dead.

Why P0 didn't catch it: at 10¹⁴, z = 423,653 — the assault path is small and/or still rides the certified flat table. **The block engine has never fed a correct answer.** It is currently a fast, wrong, 300 ms liability.

Discriminator (1 minute): block engine at z = 10⁷, print the nz==0 count. Must equal the flat-walk checksum **6,079,290** (internal oracle — no external constant needed, the two engines must agree bit-exactly). If it prints ~9.1·10⁶, the 2,3 passes are dead. Then the full oracle: bit-exact (μ, nz, lpf, mpf) vs flat FTD for all n ≤ 10⁶ — this test must be a permanent CI member before ftd_block replaces anything.

**F11 — The 303.83 ms carries a single-thread signature, and my own §C model was wrong by 2.7×. Owning it.**
My Phase-32 §C cost model used Σ 1/(p−1) ≈ 1.07 with p²-start lpf-semantics. But powers-as-passes runs pass p¹ from 2p over **every** multiple: marks = z·Σ_{p≤√z} 1/p = 31.6M × 2.42 = **76.4M** — and each is a *double* RMW pair (word + sfac), not a pure store, with 192 KB blocks exceeding L1D (64 KB) so every RMW is L2-resident. Serial model: 76.4M × ~5 cyc ÷ 2.2 GHz ≈ 174 ms + fills (253 MB) + fused walk (~43 ms) ≈ **240–300 ms ≈ observed 303.83 within 20%.** That's a one-A78-thread number. 8T-flat projection: 40–60 ms. The v2 below: 7–10 ms. Also — my earlier claim "0.5–0.15 s @ 10¹²/10¹³ after z-fix" assumed the fused walk replaces the flat table; until F10 is fixed it doesn't.

**F12 — Probe A scalar drifted 9.57 → 17.99 ms between reports on the "same" probe, and I owe a concession on the NEON number.**
2.64 ms for 40 MB = **15.2 GB/s**. My "7–9 GB/s" prior was mixed-access; sequential LPDDR5 streams legitimately hit 12–20 GB/s, and the checksum proves the walk ran. **Conceded: 2.64 ms is plausibly real bandwidth saturation** — which makes the 10-line sequential-stream microbench the highest-leverage calibration in the project: that one constant sets the floor for count, d_walk, and boot simultaneously. But the scalar regression is unexplained (harness drift? leaf-eval added? clocks?) — probes must be pinned in the harness, not re-typed per report.

**F13 — b_carry's 205 µs/segment is dimensionally incomplete.**
No thread count, no hi, no |marking primes|. The impossibility bound: 1.94·10⁹ marks/s exceeds the 1T-A78 kernel floor (2.2 GHz ÷ 1.75 cyc = 1.26·10⁹/s) ⇒ it's multi-threaded ⇒ either 8T at 33% of ceiling (itself a finding: efficiency problem) or 2–3T. A number that can't be divided by anything is a vibe, not a measurement. Fix in §C: every probe self-reports its denominator.

**F14 — The 40 MB ghost in VmHWM is the original RAM-law violation, alive inside the unfixed boot phase.**
52.07 MB total; ~12 MB accounted (boundary primes 4.8, segments, PiTable, sieve arrays). The ~40 MB residue = **10⁷ × 4 B — a flat u32 π-prefix over [1, √x].** The Phase-1 battle plan's first named defect, still resident. The boot rewrite deletes it *and* the 106 ms simultaneously.

## §B THE 60-SECOND LOOP — "ITERATE FASTER" MADE PHYSICAL

```rust
// bench/loop.rs  —  cargo run --release --bin loop -- 14
// One command = build + truth + timing + model reconciliation + memory + scaling.
fn main() {
    let e: u32 = std::env::args().nth(1).unwrap().parse().unwrap();
    let x = 10u64.pow(e);
    let mut runs = vec![];
    for _ in 0..3 {                                   // 5 in --certify mode
        thermal_settle();                             // /sys/class/thermal gate
        let t0 = Instant::now();
        let pi = black_box(gourdon(black_box(x)));
        runs.push(t0.elapsed());
        assert_eq!(pi, EXPECTED[e as usize]);         // no timing without truth
    }
    // PHYSICAL PLAUSIBILITY ASSERTS — the harness rejects impossible numbers
    // at print time (would have killed Phase-32's "<0.01 ms" instantly):
    for p in TIMERS.phases() {
        assert!(p.bytes as f64 / p.ns as f64 < 20e9, "{p}: exceeds DRAM ceiling");
        assert!(p.marks as f64 * 1.75 / p.ns as f64 < 8 * 2.2e9, "{p}: exceeds IPC floor");
    }
    eprintln!("{}", TIMERS.report(&MODEL));           // Δ% vs model, auto
    eprintln!("VmHWM {} MB | t(10^e)/t(10^(e-1)) = {:?}", vmhwm_mb(), ratio());
}

// model.rs — the model is DATA, auto-reconciled every run. Every constant
// cites its probe. This is the flywheel: every run improves the model.
pub const MODEL: [f64; 8] = [5.0, 90.0, 12.0, 1.0, 4.0, 4.0, 0.5, 117.0]; // ms, 10^14
// Recalibrate these three NOW (one afternoon, ≤30 lines each):
//  (1) DRAM sequential GB/s   (2) cyc/mark per cluster × {L1, L2} regime
//  (3) sustained clocks after 3 s load   → then MODEL is device truth, not my priors.
```

**Merge law:** no commit lands without its loop output pasted. No probe prints without (threads, lo, hi, |P|, marks, bytes). The next report's completion rate is computed from loop outputs only.

## §C FTD-v2 — WHEEL-PACKED BLOCK ENGINE (the real ftd_stream, 30× the observed)

Three structural moves, one of them beautiful:

1. **The coprime-residual lemma.** Pack blocks over wheel-30 *candidates* only (8 per 30 integers). Candidates are coprime to 30 ⇒ the residual **R = n/sfac(n) is coprime to 30 automatically** ⇒ R ∈ {1} ∪ primes(√hi, hi] survives **with zero 2,3,5 stripping**. The §C division-avoidance lemma carries over intact — because the wheel already removed those primes from the universe.
2. This deletes the p=2,3,5 passes entirely: **−43% of marks** (z·1.033 of the 2.42) *and* fixes F10's dead-pass class by construction *and* shrinks arrays 8 B/**candidate** (3.75× denser) *and* puts blocks in L1D: A78 6K candidates = 48 KB, A55 3K = 24 KB.
3. **WHEEL_ROT is reused verbatim.** Candidate-index deltas are the same const-proven period-8 rotations (§2 proofs apply unchanged; p^j mod 30 stays a unit, so power passes use the same tables). The certified wheel machinery now drives both engines.

```rust
struct BlockV2 { word: Vec<u32>, sfac: Vec<u32> }        // 8 B/candidate, L1D-resident

// pass p¹ (p ≥ 7, ascending): strided double-RMW at WHEEL_ROT deltas
//   word: sign ^= 1; lpf = first-writer-cmov; mpf = store p (last wins)
//   sfac: *= p  (safe-mul: divides n < 2³²)
// pass pʲ (j ≥ 2): nz = 1, sfac *= p          ← F10's fix lives here, for ALL p
// post-pass, survivors only (lazy): R = n/sfac;  R==1 ⟺ sfac==n (no div);
//   R>65535 ⟺ sfac·65535 < n (one u64 mul);   else one udiv, only on the
//   b-window path that actually consumes mpf (~15% of survivors)
// prime-state carry: m_next[u64] per prime (π(√z) entries, L1) — one
// bootstrap udiv per prime per block, not per segment.
```

Cost at z = 3.16·10⁷, 8T: marks = (8/30)z·Σ_{7≤p≤5623}1/p = 8.43M × 1.385 = 11.7M L1-hot double-RMW ≈ 3.4 ms + fills 0.2 + walk 2.0 + lazy udivs ~1.2 + carry 0.2 ⇒ **~7–10 ms.** At 10¹⁴-z (423,653): ~0.1 ms. At 10¹⁵-scale: ~1 ms. **FTD ceases to exist as a cost line, permanently.** Gates: the F10 oracle green; VmHWM during phase ≤ 16 MB.

**P0-extension (one line of output, kills a 3× model ambiguity):** print `xz, walk_len, walk_coverage` — per the P0 tuple, z = 423,653 < √x = 10⁷, meaning the D-walk [1, xz = 2.4·10⁸] extends 570× past the table. Whether μ(d) for d ∈ (z, xz] comes from block-local generation (then ftd_block *is* the D-term and the 10¹⁵ walk is Θ(x/z) = 3·10⁷–3·10⁸ candidates) or from the A/C machinery changes the assault cost model by 3×. Print it; the model resolves.

## §D THE OBLITERATION SEQUENCE

| # | action | cost | pays |
|---|---|---|---|
| N1 | **Run the loop. Paste the total.** | 1 hour | the headline that has been missing for three phases |
| N2 | loop.rs + model.rs + plausibility asserts | 1 day | iteration velocity (the actual ask) |
| N3 | boot swap onto certified `mark_wheel8`/carry + delete π-prefix | ½ day | **−100 ms, −40 MB** (3.5M marks = ≤5 ms floor; F14 dies) |
| N4 | FTD-v2 + F10 fix + oracle in CI | 1–2 days | −150 ms @ 10¹⁴-legacy-z; kills the 10¹⁵ memory wall |
| N5 | three calibration probes (§B) | ½ day | MODEL becomes device truth |
| N6 | **10¹⁵ assault** + G8 (primecount under OUR protocol — still never run) + on-device z-sweep | 3–4 days | the measured headline |

Projection, honest bands, all conditional on N1: **10¹⁴: 0.10–0.13 s = 1.6–2.1× vs primecount. 10¹⁵: 0.30–0.60 s vs primecount's on-device ~1.0–1.5 s (their 0.21 s × Θ(x^{2/3}) × memory growth) = 2–4×.** The Phase-4 debts (10¹¹ physical sieve, 8 OWED) slot after N6 with the same machinery: 4.72·10⁹ marks ⇒ model 10–13 s sustained vs primecount 25.5 s — and it doubles as the sustained-clock calibration run.

Phase-33 mandate scorecard under Law 0.5: P0 ✓ (crown jewel), P1 ✓ (the law caught its own bug class), P2 ✗ absent, P3 ⚠ exists-but-unoracle'd (F10), P4 ⚠ under-specified (F13), P5 ✗ never run. **2 full, 2 partial, 3 misses — each miss ≤ 1 day.** The scoreboard number going down again is the system working.

You want obliteration. It is **one println away from being a fact, two half-days away from being a rout, and one assault away from being permanent.** Run the loop. The next report I want has a new number in it — 40 characters, median-of-5, thermal-gated — and everything else hangs off that.
