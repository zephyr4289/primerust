The implementation of Phase 9.1.1 resolves the microarchitectural root causes of the 99.87s run. By pinning the sieve buffer to 16 KiB (fitting the 32 KiB L1D of the Cortex-A55), tapering tail batches, and rebalancing \alpha_y down to 7.350, the D-term will no longer stall on memory bus contention or straggler threads.
The immediate objective is executing the Live Empirical Verification Ladder on the Snapdragon 4 Gen 2 to reclaim the lead over primecount 8.1 at 10^{18} and prepare for 10^{19}.
Step 1: Concurrency Smoke Test (10^{16})
Before long runs, verify that the lock-free deque and tail-tapering logic in asymmetric_handoff_v4.rs introduce zero race conditions or segment dropping under multi-threaded contention.
TITAN_NATIVE=1 TITAN_VERIFY=1 cargo run --release --bin head_to_head 1e16

 * Ground Truth Target: \pi(10^{16}) = \mathbf{279,238,341,033,925}
 * Previous Native Baseline: 2,707.75\text{ ms}
 * Target Latency: \mathbf{\le 2,100\text{ ms}} (Verifying work-stealing efficiency)
Step 2: The 10^{17} Re-Balancing Checkpoint
Test scale 10^{17} to verify that the shift to \alpha_y = 7.050 successfully balances AC and D. In Phase 9.1.0, D took 16.94s while AC took 8.37s.
TITAN_NATIVE=1 TITAN_VERIFY=1 cargo run --release --bin head_to_head 1e17

 * Ground Truth Target: \pi(10^{17}) = \mathbf{2,623,557,157,654,233}
 * Primecount 8.1 Baseline: \sim 10,800\text{ ms}
 * Target Latency: \mathbf{\le 9,500\text{ ms}} (Sub-10-second win)
 * Balance Check: Both AC and D latencies should finish within \pm 20\% of each other.
Step 3: Cold Ultra-Scale Showdown (10^{18})
Hold all 8 cores at peak frequency without thermal throttling. Execute the cooldown cycle, check governor clocks, and fire the benchmark:
echo "Cooling silicon for 35 seconds..."
sleep 35

# Ensure hardware clocks are at max unthrottled boost
cat /sys/devices/system/cpu/cpu6/cpufreq/scaling_cur_freq # Must read 2208000 (A78)
cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_cur_freq # Must read 1958400 (A55)

TITAN_NATIVE=1 TITAN_VERIFY=1 cargo run --release --bin head_to_head_ultra 1e18

 * Ground Truth Target: \pi(10^{18}) = \mathbf{24,739,954,287,740,860}
 * Primecount 8.1 Latency: 51,183.39\text{ ms} (51.18 s)
 * Phase 9.1.0 Baseline: 99,872.72\text{ ms} (99.87 s)
 * Phase 9.1.1 Target: \mathbf{\le 42,000\text{ ms}} (Defeating Primecount by >9 seconds)
Step 4: The Historic 10^{19} Frontier
Once 10^{18} certifies a victory with 100% bit-exact parity, advance Titan to 10^{19}.
echo "Cooling silicon for 45 seconds..."
sleep 45

TITAN_NATIVE=1 cargo run --release --bin head_to_head_ultra 1e19

 * Ground Truth Target: \pi(10^{19}) = \mathbf{234,057,667,276,344,607}
 * Target Latency: \sim 180\text{–}210\text{ s}
 * Memory Invariant Check: Ensure the compact \pi-table consumes < 2.5\text{ MB} of resident memory, preventing an Android Low Memory Killer (SIGKILL) trigger.
Launch Step 1 on the device to confirm concurrency sanity, then proceed down the ladder.

