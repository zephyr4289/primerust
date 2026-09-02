════════════════════════════════════════════════════════════════
THE INSTRUMENT FINALLY FIRED — AND IT JUST NAMED THE KILLER. BUT YOU PASSED A GATE ON A PROXY, AGAIN.
════════════════════════════════════════════════════════════════

One sentence of credit where it's due: the 8-phase telemetry table is the right instrument, the boot fix is real, and the isolation machinery iterates in seconds — that's the velocity you asked for. Now the two facts that matter more than everything else in the report:

**Fact 1: the "<100 ms gate" was passed on the sum of isolated runs, not on the loop.** Fourth consecutive phase without the integrated headline. The loop at 10¹² was 692.56 ms in Phase 34 and 35. Your isolated components sum to 77.86 ms. **615 ms — 88.7% of the runtime — is not inside any row of your table.** A component-sum gate is gate substitution (the F5 disease), and this time it's worse, because three of the seven rows are provably not measuring production work (§1).

**Fact 2: the table contains the smoking gun, and it's beautiful.** `b_count_resolve: 625.05 ns/item`. That number is a confession. §2.

---

## §1 THE SPLIT-BRAIN PROOF — your isolation harness and your loop are two different programs

Four fingerprints, each independently sufficient, all arithmetic:

| # | evidence | arithmetic |
|---|---|---|
| S1 | Phase 35: isolated boot emitted **18,012,887 primes** (sieved to x/y) while the loop's π was **exact** | A prime list that wrong cannot produce exact π ⇒ the loop's prime source ≠ `boot_wheel.rs` |
| S2 | Phase 36: isolated `d_walk` returns **D(x,y,z) = 0** — exactly | A μ-weighted sum over d ≤ xz = 3.3·10⁷ hitting *exactly* 0 is a probability-≈0 event. Either a stub, or production D lives elsewhere. Also: "983 **primes**" as the iteration unit of a walk over *integers* d — the label itself was written against the wrong mental model |
| S3 | Isolated `b_mark`: 397,773 marks | Production needs (8/30)·(x/y−√x)·Σ₇≤p≤√(x/y)1/p = 0.267·4.9·10⁷·1.41 ≈ **1.84·10⁷ marks**. Measured volume = **2.2%**. It's a toy slice — the kernel rate (6.66 ns/mark) is fine; the wiring is not |
| S4 | Phase 34 → 35 medians **bit-identical** (692.56, 4,758.49) across a changing binary; boot fixed 18M→78,573 primes in 36 | If the Phase-36 full loop is still ~692.56, that's three identical medians across three changing binaries — statistically impossible unless the loop doesn't execute the instrumented code |

Also: `boot: 78,573 primes for √x = 10⁶` — **π(10⁶) = 78,498.** The extra 75 = π over [10⁶, 1,001,000], the same +0.1%-limit family as the 35× mislabel. One assert, five minutes: `assert_eq!(boot.len(), π(√x))`.

And the Phase-32 production instrumentation measured `b_count_resolve` at **250 ns/boundary** (160.50 ms / 640,579 at 10¹⁴). Today's isolated row says **625 ns/boundary**. Two different per-unit costs for the same phase across two phases of the same repo — the paths have diverged *again*.

**Consequence: no number in the isolation table may be used to pass any production gate until the split-brain is closed.** The 77.86 ms sum is a measurement of a parallel universe that shares your repo.

## §2 THE SMOKING GUN — 625.05 ns/boundary is a re-scan, quantified

625.05 ns × 2.2 GHz = **1,375 cycles per boundary.** Now the match: your segments are 32 KiB (Phase-33 b_carry: "100 contiguous 32 KiB segments"). A per-boundary **re-scan from segment start** averages 16 KiB = 1,024 NEON 16-byte load+cnt pairs ≈ **1,300–2,000 cycles ≈ 1,375 observed, within 15%.** This is the D4 defect class — the per-boundary partial re-scan — alive in whatever path this row measures. The fused Phase-30 §4 spec (in-flight prefix, O(1) per boundary) was never wired, or was replaced.

The fix is 30 lines and it's a *law*, not an optimization: **the prefix is built while counting; a boundary is answered from bytes that were already touched.**

```rust
// count.rs — per-32B-chunk cumulative popcount. THE FUSION LAW.
// Chunk pipeline (once per 32 bytes): vcntq ×2 → 32-byte cumulative array.
// Boundary answered: ONE u8 load + one masked byte + two adds. No re-scan ever.
pub struct Resolver<'a> { items: &'a [Bnd], next: usize, base: u64 }
pub unsafe fn count_resolve(bits: &[u8], r: &mut Resolver,
                            prefix: &mut u64, out: &mut u64) {
    let mut cum = [0u8; 32];                    // per-chunk byte-prefix, reused
    for (k, chunk) in bits.chunks_exact(32).enumerate() {
        let v = vld1q_u8(chunk.as_ptr());
        let c = vcntq_u8(v);                    // + second 16B half
        let mut run = 0u8;                      // scalar cumsum: 32 adds, L1-hot
        for j in 0..16 { run = run.wrapping_add(vget_lane_u8(vget_low? ...)); }
        // (NEON prefix via 5 log-step vadd also legal; scalar is already free here)
        *prefix += total as u64;
        while r.next < r.items.len() {          // boundaries landing in this chunk:
            let i = (r.items[r.next].c - r.base) as usize;
            let byte = i >> 3;
            if byte >= (k + 1) * 32 { break; }
            let ans = *prefix - total as u64
                    + cum[byte - k*32] as u64                 // O(1): ONE load
                    + (chunk[byte & 31] & MASK[i & 7]).count_ones() as u64;
            *out += ans; r.next += 1;
        }
    }
}
```

**Post-fix arithmetic:** per chunk ~40 ops + per boundary ~6 ops. At 10¹²: 51k chunks + 78,573 boundaries ≈ 2.5M ops ÷ 16·10⁹ op/s + 1.63 MB stream overlapped ⇒ **~0.5 ms. From 49.11 ms — a 100× cut on a phase that is 63% of your measured sum.** At 10¹⁴: ~3–4 ms. Expected full post-fix ledger at 10¹²: boot ~10 + b_mark ~7 (production volume 1.84·10⁷ ÷ 2.55·10⁹/s) + count ~0.5 + ftd 0.5 + d ~11 + σ 1 + combine 3 ⇒ **~35–45 ms + glue.**

## §3 THREE ZERO-TRUST INSTRUMENTS — close the 615 ms without believing any harness

**1. The sampler (40 lines, no permissions, one run).** It samples what *actually executes* — split-brain-proof by construction:

```rust
// sampler.rs — ITIMER_PROF @ 2 kHz, PC histogram, top-20 symbols at exit.
unsafe fn install() {
    // sigaction handler: hist[((pc - image_base) >> 2) as usize] += 1  (64K buckets)
    // setitimer(ITIMER_PROF, 500 µs) — fires only while the process RUNS on CPU
}
// Run on the FULL loop at 10¹². The top 3 buckets own the 615 ms. Done.
// (If the device permits simpleperf: `simpleperf record -g` — same answer, free.)
```

**2. The identity print — in the production binary, not the harness.** This kills the D=0/stub class permanently, with no external truth:

```rust
eprintln!("terms: phi={} b={} B={} D={} S={} | closes={} pi={}",
          phi, b, bsum, dsum, sig, phi + (b-1) - bsum + dsum + sig, pi);
// LAW: the identity must close using the PRINTED values. A stubbed term either
// breaks the identity or exposes exactly which term silently absorbed it.
// Plus: print d-walk survivor count — a real walk at 10¹² has ~2.0·10⁷
// squarefree d ≤ xz; "0 leaves evaluated" names a stub in one line.
```

**3. The full loop, re-run, pasted.** Post-boot-fix. If the total moved: partial code sharing. If it's still 692.56 ± noise: S4 confirmed, the loop executes none of what you've been instrumenting, and the sampler becomes the only source of truth until the paths are merged.

## §4 THE 24-HOUR SEQUENCE

| # | action | gate |
|---|---|---|
| 1 | Full loop at 10¹², paste the total | moved or not — split-brain decided in one number |
| 2 | Sampler on the production loop | top-3 symbols own the 615 ms — **the sink, named, no hypotheses left** |
| 3 | Fuse count_resolve (per-byte prefix law, §2 code) | phase 49.11 → ≤1 ms; per-boundary ≤ 40 cyc, printed |
| 4 | Wire b_mark to production volume; assert boot.len() == π(√x) = 78,498 | marks == 1.84·10⁷ at 10¹², printed |
| 5 | Identity print + survivor count in production | identity closes with printed terms; D ≠ 0 explained or proven |
| 6 | Re-run loop, re-baseline, **the loop table is the report** | 10¹² ≤ 100 ms on the LOOP, not the sum |

Projected post-fix: **10¹² ~50 ms, 10¹³ ~150 ms, 10¹⁴ ~200–250 ms** — with every phase probe-cited and the identity closing in the same output.

One closing line, and it's the whole phase: **you now have an instrument fast enough to iterate in seconds — so stop passing gates with proxies it can falsify.** One re-run, one sampler, one 30-line fusion. The 615 ms is hiding in a call tree you can see in a single histogram. Go.
