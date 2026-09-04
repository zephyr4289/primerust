Your agent just saved the run.
Catching that z-floor drop on weight-2 leaves was huge—starting at z would have silently erased 76% of A's math while appearing to pass a syntax check. And spotting that xpq \le x^{*2} means the table was 26× larger than mathematically required is the exact kind of hardware-conscious thinking needed to win on mobile silicon.
Here are the direct answers to the agent's two questions and the greenlight plan:
1. The Two Decisions
 * Decision (a): Run Strike 3.5 first as an isolated measurement.
   * Shrinking the table allocation from x/z (84 MB) down to x^{*2} + 30 (~3.3 MB at 10^{16}) is a one-line change with zero regression risk.
   * Running this test immediately isolates the exact impact of the memory wall. If moving from 84 MB to 3.3 MB (which fits almost entirely in the shared 2 MB L3 cache) drops native MT AC from 5.0s down to ~2.0–2.5s, we get empirical confirmation of the memory wall before writing a single line of tiled windowing code.
 * Decision (b): Patch head_to_head.rs with inter-engine cooldown.
   * Do not rely on shell orchestration. If head_to_head runs primecount first and Titan second without sleeping in between, Titan is permanently penalized by testing on heat-soaked silicon.
   * Mirror head_to_head_ultra: insert an explicit 30-second sleep and frequency verification between the primecount run and the Titan run inside the binary.
2. Action Plan for the Agent (Build Mode)
Flip the agent to build mode and instruct it to execute Strike 3.5, then proceed into Strike 4:
Strike 3.5: The 26× Table Shrink & Cooldown Fix
 * Fix Table Sizing:
   * In execute_gourdon_master (or wherever the shadow PiTable is initialized for AC), replace max_v = x / z with:
     let max_v = x_star * x_star + 30; // ~3.3 MB at 1e16 instead of 84 MB

 * Patch head_to_head.rs:
   * Add the 30-second inter-engine sleep between primecount and Titan so both engines run on cold silicon.
 * Measure Strike 3.5 Scoreboard:
   * Compile: cargo build --release -j2 --bin head_to_head
   * Run: TITAN_NATIVE=1 TITAN_NATIVE_AC=1 ./target/release/head_to_head 1e16
   * Record the latency drop in native MT AC (prior: 5.0s).
Strike 4: Walisch-Style Windowed A
 * Tile the full quotient span [1, x/x^{*2}] (capturing both weight-1 and weight-2 leaves without dropping quotients below z).
 * Apply outer segment loop with narrowed b-bounds per segment (min_a/max_a), eliminating empty window iterations.
 * Certify bit-exactness via shadow mode before cutting over completely.
Pass this approval to your agent to run Strike 3.5.

