PROCEED WITH AUDITED STRIKE 2 EXECUTION PLAN:

Decisions Confirmed:
  1. Faithful A: Implement plain ascending loops matching AC.cpp:53-90 (weight x1 and x2). Cluster ONLY C2.
  2. Shadow Mode: Gate native AC behind TITAN_NATIVE_AC=1 (run native + FFI, assert exact match, log divergence if any).
  3. Signatures: Keep &[u64] across all AC functions and use exact `icbrt64` / `isqrt64`.

Execute the 5-step plan:
  Step 0: Fix the hardcoded /usr/bin/primecount path in crates/titan-count/src/bin/head_to_head*.rs with the dynamic PREFIX resolver.
  Step 1: Create crates/titan-count/tests/test_gourdon_pipeline_e13.rs recording FFI AC at 10^13 (105,017,131,716).
  Step 2: Implement `compute_c2_clustered` in ac_parallel_v2.rs (descending i, clustered region first via (i - imin), sparse tail, underflow guards).
  Step 3: Implement faithful `compute_a_formula` matching AC.cpp.
  Step 4: Wire both into gourdon_pipeline.rs under TITAN_NATIVE_AC=1 shadow mode and run test_gourdon_pipeline_e13.

