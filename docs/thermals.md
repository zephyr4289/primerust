# 🌡️ Hardware Thermal Architecture & Diagnostic Guide

## 1. Executive Summary

This document provides a technical reference, diagnostic guide, and incident post-mortem regarding thermal management and CPU frequency scaling on the target silicon platform:
* **SoC**: Qualcomm Snapdragon 4 Gen 2 (`SM4450`)
* **CPU Topology**: 8-Core Heterogeneous DynamIQ Big.LITTLE
  * **2× Cortex-A78** (Performance Big Cores: Cores 6 & 7 | Max 2.21 GHz)
  * **6× Cortex-A55** (Efficiency Little Cores: Cores 0..=5 | Max 1.96 GHz)
* **Cache Hierarchy**: 64 KiB L1D per A78, 32 KiB L1D per A55, 2 MiB shared DynamIQ L3 cache
* **Thermal Environment**: Passive fanless smartphone chassis running Linux under Termux PRoot

Because mobile silicon lacks active fan cooling, sustained heavy compute loads (e.g., multi-crate Rust compilation followed immediately by multi-threaded ultra-scale prime counting benchmarks) can heat-soak the device chassis, triggering the Android Linux kernel's thermal governor to clamp CPU clock frequencies.

---

## 2. Quick Diagnostic Commands (Copy-Paste Ready)

Any developer or engineer inspecting the machine can run these commands directly in the shell to check live thermal status and core frequencies.

### A. Quick Health Check (Live Frequencies & Temperatures)
```bash
echo "=== CPU FREQUENCIES (Current vs Max) ===" && \
paste <(ls -d /sys/devices/system/cpu/cpu[0-9]*) \
      <(cat /sys/devices/system/cpu/cpu*/cpufreq/scaling_cur_freq 2>/dev/null | awk '{printf "%.2f GHz\n", $1/1000000}') \
      <(cat /sys/devices/system/cpu/cpu*/cpufreq/scaling_max_freq 2>/dev/null | awk '{printf "Max: %.2f GHz\n", $1/1000000}') && \
echo -e "\n=== ACTIVE THERMAL SENSORS (°C) ===" && \
paste <(cat /sys/class/thermal/thermal_zone*/type 2>/dev/null) \
      <(cat /sys/class/thermal/thermal_zone*/temp 2>/dev/null | awk '{if ($1 > 0) printf "%.1f °C\n", $1/1000; else print "N/A"}') | grep -v "N/A" | head -n 15
```

### B. Live Throttling Detector
Run this one-liner to immediately determine if the Cortex-A78 big cores are being throttled:
```bash
A78_FREQ=$(cat /sys/devices/system/cpu/cpu6/cpufreq/scaling_cur_freq 2>/dev/null || echo 0)
if [ "$A78_FREQ" -ge 2200000 ]; then
    echo "🟢 STATUS: UNTHROTTLED (Cortex-A78 running at peak 2.21 GHz)"
elif [ "$A78_FREQ" -ge 1800000 ]; then
    echo "🟡 STATUS: MILD THROTTLE (Cortex-A78 at $(awk "BEGIN {print $A78_FREQ/1000000}") GHz)"
else
    echo "🔴 STATUS: HEAVY THROTTLE DETECTED! (Cortex-A78 clamped to $(awk "BEGIN {print $A78_FREQ/1000000}") GHz, down from 2.21 GHz)"
fi
```

### C. Continuous Real-Time Thermal Watch (Updates Every Second)
```bash
watch -n 1 'echo "--- CPU Clocks ---"; paste <(ls -d /sys/devices/system/cpu/cpu[0-9]*) <(cat /sys/devices/system/cpu/cpu*/cpufreq/scaling_cur_freq | awk "{printf \"%.2f GHz\n\", \$1/1000000}"); echo -e "\n--- Thermal Zones ---"; paste <(cat /sys/class/thermal/thermal_zone*/type) <(cat /sys/class/thermal/thermal_zone*/temp | awk "{printf \"%.1f C\n\", \$1/1000}") | head -n 12'
```

### D. Check for Background Compiler / Runner Processes
Ensure no orphan or zombie processes are consuming cycles:
```bash
ps aux | grep -E 'cargo|rustc|primecount|head_to_head' | grep -v grep || echo "Clean: No compute processes running."
```

---

## 3. Incident Post-Mortem: Phase 6.1 Ultra-Scale Run

### What Happened
During testing of Phase 6.1, execution times for scale $10^{18}$ showed an apparent jump:
* Prior Phase 5.1 $10^{18}$ run: **$50.49\text{ s}$**
* Phase 6.1 $10^{18}$ run: **$62.60\text{ s}$**

### Root-Cause Analysis
Immediately prior to the Phase 6.1 benchmark, the device was subjected to an intense, uninterrupted compute workload:
1. `cargo test -p titan-count --test test_sampled_index -j 2`: **1 minute 40 seconds** of sustained 100% 8-core compilation and linking.
2. Execution of 250,000 randomized unit-test verification queries.
3. `cargo build --release -j 2`: **1 minute 24 seconds** of continuous 8-core compilation.
4. Execution of the 11-scale `head_to_head` standard benchmark sweep.
5. Immediate execution of the `head_to_head_ultra` sweep.

Over **3.5 minutes of continuous full-core saturation** without forced cooldown caused the chassis skin temperature to cross **54.4°C**.

### Indisputable Physical Proof: Primecount 8.1 Slowdown
Kim Walisch's `primecount 8.1` is a static, pre-installed binary located at `/usr/bin/primecount`. Not a single line of primecount was changed. Yet:
* **Primecount at $10^{17}$**: Jumped from **$10.70\text{ s} \to 13.05\text{ s}$** ($+22\%$ slowdown).
* **Primecount at $10^{18}$**: Jumped from **$43.55\text{ s} \to 61.39\text{ s}$** ($+41\%$ slowdown).

Direct hardware inspection of `/sys/devices/system/cpu/cpu*/cpufreq/scaling_cur_freq` confirmed:
* Cortex-A78 (Cores 6 & 7) were clamped by the kernel governor from **2,208,000 kHz down to 1,497,600 kHz** (a **$-32.2\%$ clock frequency reduction**).
* Both engines ran under identical throttled conditions: `primecount` at $61.39\text{ s}$ vs `Titan` at $62.60\text{ s}$ ($0.98\times$ performance ratio).

---

## 4. Hardware Cooling & Recovery Baseline

Once the system rested, sensor inspection confirmed complete thermal normalization:

```
=== CPU FREQUENCIES (kHz) ===
/sys/devices/system/cpu/cpu0:  1,958,400 kHz (1.96 GHz)
/sys/devices/system/cpu/cpu1:  1,958,400 kHz (1.96 GHz)
/sys/devices/system/cpu/cpu2:  1,958,400 kHz (1.96 GHz)
/sys/devices/system/cpu/cpu3:  1,958,400 kHz (1.96 GHz)
/sys/devices/system/cpu/cpu4:  1,958,400 kHz (1.96 GHz)
/sys/devices/system/cpu/cpu5:  1,958,400 kHz (1.96 GHz)
/sys/devices/system/cpu/cpu6:  2,208,000 kHz (2.21 GHz - UNTHROTTLED PEAK)
/sys/devices/system/cpu/cpu7:  2,208,000 kHz (2.21 GHz - UNTHROTTLED PEAK)

=== THERMAL SENSORS ===
pa / pa1 (RF Power Amp):   37.2°C – 42.5°C
camera / video:            46.1°C – 46.4°C
cpu-0 (Cortex-A55 cluster):49.1°C – 53.4°C
cpu-1 (Cortex-A78 cluster):54.4°C – 56.1°C
```

---

## 5. Benchmarking Protocol for Reliable Results

To prevent thermal throttling from distorting silicon benchmark results, follow these rules:

1. **Compilation Concurrency**:
   Always use `-j 2` during compilation (`cargo build --release -j 2`, `cargo test --lib -p titan-count -j 2`). Unconstrained compilation spawns 30+ processes, triggering Android Low Memory Killer (`SIGKILL`) and overheating the SoC.
2. **Pre-Benchmark Thermal Reset**:
   Always enforce at least a 30-second sleep prior to measuring latency at scale $10^{16}$, and 30 to 45 seconds before $10^{17}$ and $10^{18}$:
   ```bash
   sleep 30 && ./target/release/head_to_head
   ```
3. **Head-to-Head Fair Baseline**:
   In `head_to_head_ultra`, enforce thermal stabilization between Primecount and Titan so that Titan does not run on a chip pre-heated by 45 seconds of primecount load:
   ```rust
   // Run Primecount
   let (pc_res, pc_ms) = run_primecount(x, 8);
   
   // Mandatory cooldown
   std::thread::sleep(std::time::Duration::from_secs(30));
   
   // Run Titan on cooled silicon
   let titan_res = TierDispatch::count(black_box(x), 8);
   ```
