# Golden partials (Phase 0 S1) — reading rules

Frozen by `rig run --config crates/titan-rig/sessions/golden_1e16.json`
(oracle binary, 1 thread, `PC_DUMP=1`). Regenerate identically any time.

## Files

- `ac_1e16.tsv`: A_64 (one line per segment×b), C1 (once per b), C2_64
  (one line per segment×b) at x=1e16, y=1921752, z=3843504.
- `b_1e16.tsv`: Bseg per segment (depth=0) plus nested-run forensic lines.
- `d_1e16.tsv`: Dseg per segment.

## Aggregation rules (all three are load-bearing)

1. **Depth filter**: keep `depth=0` lines only. B's first-iteration
   `pi_noprint()` runs a nested full Gourdon(1e8) whose A/C2/Bseg/Dseg
   lines are tagged `depth=1` — forensic data, excluded from sums.
2. **u64 ring arithmetic**: C1's alternating sums go negative while the
   oracle accumulates unsigned — aggregate with `& 0xFFFF_FFFF_FFFF_FFFF`
   and reinterpret the final total as i64. (Plain `int()` parsing gives
   5e20 garbage.)
3. **Multi-line keys sum**: A/C2 keys repeat per segment; sum all lines.

## Verified identities (x=1e16, y=1921752, z=3843504)

- `sum(A_64) − wrap(C1) + sum(C2_64) = 90734744872579` (FFI AC, exact)
- `sum(Bseg depth=0) = 124768874104374` (single segment at 1T)
- `sum(Dseg) = 221235115317419` (133 segments, exact)
