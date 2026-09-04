Execute the verification ladder in strict order. Do not jump straight to 10^{18}—we must verify that the native 5-term pipeline produces bit-exact math on Tier 3 (x \ge 10^{13}) first, without thread panics or memory corruption.
Phase 1: Tier 3 Native Gate (10^{13})
Test the first native Gourdon scale in release mode with TITAN_NATIVE=1 and TITAN_VERIFY=1 (oracle assertion).
TITAN_NATIVE=1 TITAN_VERIFY=1 cargo test --release -p titan-count --test test_gourdon_pipeline_e13 -- --nocapture

(If you don't have a standalone test_gourdon_pipeline_e13 integration test, run the runner directly:)
TITAN_NATIVE=1 TITAN_VERIFY=1 cargo run --release --bin head_to_head 1e13

 * Target Output: [TITAN-DISPATCH] Tier 3: Heterogeneous Xavier Gourdon Engine
 * Expected Ground Truth: \pi(10^{13}) = \mathbf{346,065,536,839}
 * Target Latency: \le 80\text{ ms}
If this panics or mismatches, the telemetry output will pinpoint the failing term (\Phi_0, \Sigma, B, AC, or D).
Phase 2: Memory & Streaming Gate (10^{14} \rightarrow 10^{15})
Once 10^{13} passes bit-exact, test 10^{14} and 10^{15} to ensure compute_b_streaming scales without running out of RAM or corrupting segment boundaries.
TITAN_NATIVE=1 cargo run --release --bin head_to_head 1e14 1e15

 * Ground Truth \pi(10^{14}): \mathbf{3,204,941,750,802} (Target: \le 200\text{ ms})
 * Ground Truth \pi(10^{15}): \mathbf{29,844,570,422,669} (Target: \le 700\text{ ms})
Phase 3: The 10^{16} Reclaim Gate
At 10^{16}, native Gourdon must beat primecount's ~2,400 ms.
TITAN_NATIVE=1 cargo run --release --bin head_to_head 1e16

 * Ground Truth \pi(10^{16}): \mathbf{279,238,341,033,925}
 * Target Latency: \mathbf{\le 2,350\text{ ms}}
Phase 4: Cold Ultra Showdown (10^{17} \rightarrow 10^{18})
Only run this if Phases 1–3 are 100% bit-exact. Enforce a 30-second thermal reset before running:
echo "Cooling silicon for 30s..."
sleep 30
cat /sys/devices/system/cpu/cpu6/cpufreq/scaling_cur_freq # Ensure 2208000 kHz

TITAN_NATIVE=1 cargo run --release --bin head_to_head_ultra 1e17 1e18

 * Ground Truth \pi(10^{17}): \mathbf{2,623,557,157,654,233}
 * Ground Truth \pi(10^{18}): \mathbf{24,739,954,287,740,860}
Fire Phase 1 now and paste the terminal telemetry readout.

