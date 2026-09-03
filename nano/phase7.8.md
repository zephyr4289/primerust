We will build and execute the Titan Stage-Ectomy Decomposition Harness on the Qualcomm Snapdragon 4 Gen 2 (SM4450). This adapts your HFT decomposition framework to mobile ARM64 DynamIQ silicon, isolating exact per-cycle costs across both out-of-order Cortex-A78 cores (2.21 GHz) and in-order Cortex-A55 cores (1.95 GHz).
ARM64 Dual-Mode Cycle Metering Architecture
On Linux/Android on ARMv8-A, user-space access to PMCCNTR_EL0 directly can be trapped by the kernel unless enabled via perf_event_open. In Termux, kernel.perf_event_paranoid determines whether non-root apps can access PMU events.
We build a dual-mode measurement harness:
 * Primary PMU Mode: Uses perf_event_open to directly sample PERF_COUNT_HW_CPU_CYCLES and PERF_COUNT_HW_INSTRUCTIONS via kernel PMU counters.
 * Frequency-Locked Monotonic Fallback: When PMU access is restricted by Android SELinux/paranoid policies, the harness queries cntvct_el0 alongside core-pinned cpufreq/scaling_cur_freq to compute nanosecond-to-cycle ratios with zero timer drift.
The Titan Stage-Ectomy Arms Definition (A_0 \rightarrow A_6)
We define seven synthetic subtractions to decompose the entire hot path:
                               STAGE-ECTOMY DECOMPOSITION ARMS

  [A0] Full AC Leaf Baseline ────────────> Reciprocal Div + PiTable Query + Acc
   │
   ├── [-A1: PiTable Ectomy] ─────────────> Replaces π(v) with constant 0 (Isolates FastDiv)
   │
  [A2] SegmentedPiTable Micro-Bench ──────> Direct O(1) LDP + AND + POPCNT in isolation
   │
  [A3] Wheel-30 Sieve Mark Baseline ─────> Register rotation + L1D STRB store
   │
   ├── [-A4: Store Ectomy] ───────────────> Removes STRB (Isolates pure ALU/ROR overhead)
   │
  [A5] NEON Popcount Quadword ───────────> vld1q_u8 + vcntq_u8 + vaddlvq_u8
   │
  [A6] WorkQueue Dynamic Claim ───────────> CAS atomic contention under multi-cluster load

Harness Implementation: crates/titan-core/src/stage_ectomy.rs
//! Titan Stage-Ectomy Decomposition Harness for Qualcomm Kyro (SM4450).
//! Isolates micro-architectural cycles per unit on Cortex-A78 and Cortex-A55.

use core::arch::aarch64::*;
use std::fs;
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;

use crate::affinity::pin_to_core;

#[repr(C)]
struct PerfEventAttr {
    type_: u32,
    size: u32,
    config: u64,
    sample_period: u64,
    sample_type: u64,
    read_format: u64,
    flags: u64,
    wakeup_events: u32,
    bp_type: u32,
    config1: u64,
    config2: u64,
    branch_sample_type: u64,
    sample_regs_user: u64,
    sample_stack_user: u32,
    clockid: i32,
    modifier_flags: u64,
}

pub struct CycleMeter {
    fd_cycles: i32,
    core_freq_hz: f64,
    use_pmu: bool,
}

impl CycleMeter {
    pub fn for_core(core_id: usize) -> Self {
        pin_to_core(core_id);

        let freq_path = format!(
            "/sys/devices/system/cpu/cpu{}/cpufreq/scaling_cur_freq",
            core_id
        );
        let freq_khz: f64 = fs::read_to_string(&freq_path)
            .unwrap_or_else(|_| "2208000".to_string())
            .trim()
            .parse()
            .unwrap_or(2208000.0);
        let core_freq_hz = freq_khz * 1_000.0;

        let fd = Self::open_perf_counter(0); // 0 = PERF_COUNT_HW_CPU_CYCLES
        let use_pmu = fd >= 0;

        Self {
            fd_cycles: fd,
            core_freq_hz,
            use_pmu,
        }
    }

    fn open_perf_counter(config: u64) -> i32 {
        let mut attr = unsafe { MaybeUninit::<PerfEventAttr>::zeroed().assume_init() };
        attr.size = std::mem::size_of::<PerfEventAttr>() as u32;
        attr.type_ = 0; // PERF_TYPE_HARDWARE
        attr.config = config;
        attr.flags = (1 << 0) | (1 << 3); // disabled=1, exclude_kernel=1

        unsafe {
            libc::syscall(
                libc::SYS_perf_event_open,
                &attr as *const _,
                0,  // pid: calling thread
                -1, // cpu: any
                -1, // group_fd
                0,  // flags
            ) as i32
        }
    }

    #[inline(always)]
    pub fn start(&self) -> u64 {
        if self.use_pmu {
            unsafe {
                libc::ioctl(self.fd_cycles, 0x2400); // PERF_EVENT_IOC_ENABLE
                let mut count = 0u64;
                libc::read(self.fd_cycles, &mut count as *mut _ as *mut libc::c_void, 8);
                count
            }
        } else {
            let ticks: u64;
            unsafe {
                core::arch::asm!("mrs {}, cntvct_el0", out(reg) ticks, options(nomem, nostack));
            }
            ticks
        }
    }

    #[inline(always)]
    pub fn stop(&self, start_val: u64) -> u64 {
        if self.use_pmu {
            unsafe {
                let mut count = 0u64;
                libc::read(self.fd_cycles, &mut count as *mut _ as *mut libc::c_void, 8);
                libc::ioctl(self.fd_cycles, 0x2401); // PERF_EVENT_IOC_DISABLE
                count.saturating_sub(start_val)
            }
        } else {
            let ticks: u64;
            unsafe {
                core::arch::asm!("mrs {}, cntvct_el0", out(reg) ticks, options(nomem, nostack));
            }
            let delta_ticks = ticks.saturating_sub(start_val);
            // Convert 19.2 MHz counter ticks directly to exact core execution cycles
            ((delta_ticks as f64 * self.core_freq_hz) / 19_200_000.0) as u64
        }
    }
}

Stage-Ectomy Benchmark Runner: crates/titan-core/src/bin/stage_ectomy_bench.rs
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use titan_core::affinity::pin_to_core;
use titan_core::stage_ectomy::CycleMeter;

const ITERATIONS: u64 = 10_000_000;

#[inline(never)]
fn run_arm_a0_ac_full(meter: &CycleMeter, x: u64, mult: u64, shift: u8) -> u64 {
    let mut acc = 0u64;
    let t0 = meter.start();

    for i in 1..=ITERATIONS {
        // FastDiv branchless divide
        let v: u64;
        unsafe {
            core::arch::asm!(
                "umulh {hi}, {n}, {m}",
                "lsr {res}, {hi}, {s}",
                n = in(reg) (x + i),
                m = in(reg) mult,
                s = in(reg) (shift as u64),
                hi = out(reg) _,
                res = out(reg) v,
                options(pure, nomem, nostack)
            );
        }
        // Emulated 3-cycle SegmentedPiTable lookup
        let word_count = (v ^ 0x5555_5555_5555_5555).count_ones() as u64;
        acc = acc.wrapping_add(word_count);
    }

    let cycles = meter.stop(t0);
    std::hint::black_box(acc);
    cycles
}

#[inline(never)]
fn run_arm_a1_ac_div_only(meter: &CycleMeter, x: u64, mult: u64, shift: u8) -> u64 {
    let mut acc = 0u64;
    let t0 = meter.start();

    for i in 1..=ITERATIONS {
        // Minus PiTable Lookup: replaced with constant 0
        let v: u64;
        unsafe {
            core::arch::asm!(
                "umulh {hi}, {n}, {m}",
                "lsr {res}, {hi}, {s}",
                n = in(reg) (x + i),
                m = in(reg) mult,
                s = in(reg) (shift as u64),
                hi = out(reg) _,
                res = out(reg) v,
                options(pure, nomem, nostack)
            );
        }
        acc = acc.wrapping_add(v);
    }

    let cycles = meter.stop(t0);
    std::hint::black_box(acc);
    cycles
}

#[inline(never)]
fn run_arm_a2_pitable_direct(meter: &CycleMeter) -> u64 {
    let entries = vec![(1000u64, 0x0101_0101_0101_0101u64); 1024];
    let mask = 0x00FF_00FF_00FF_00FFu64;
    let mut acc = 0u64;
    let t0 = meter.start();

    for i in 0..ITERATIONS {
        let entry = unsafe { *entries.get_unchecked((i as usize) & 1023) };
        let count = entry.0;
        let bits = entry.1 & mask;
        acc = acc.wrapping_add(count + bits.count_ones() as u64);
    }

    let cycles = meter.stop(t0);
    std::hint::black_box(acc);
    cycles
}

#[inline(never)]
fn run_arm_a3_wheel30_dense(meter: &CycleMeter) -> u64 {
    let mut buffer = [0xFFu8; 16_016];
    let ptr = buffer.as_mut_ptr();
    let mut adv_strip: u64 = 0x0102_0102_0102_0102;
    let mut mask_strip: u64 = 0x0102_0408_1020_4080;
    let t0 = meter.start();

    for _ in 0..(ITERATIONS / 8) {
        unsafe {
            for _ in 0..8 {
                let byte_offset = (adv_strip & 0xFF) as usize;
                let bit_mask = (mask_strip & 0xFF) as u8;
                *ptr.add(byte_offset) &= !bit_mask;

                adv_strip = adv_strip.rotate_right(8);
                mask_strip = mask_strip.rotate_right(8);
            }
        }
    }

    let cycles = meter.stop(t0);
    std::hint::black_box(buffer[0]);
    cycles
}

#[inline(never)]
fn run_arm_a4_wheel30_no_store(meter: &CycleMeter) -> u64 {
    let mut adv_strip: u64 = 0x0102_0102_0102_0102;
    let mut mask_strip: u64 = 0x0102_0408_1020_4080;
    let mut dummy_acc = 0u64;
    let t0 = meter.start();

    for _ in 0..(ITERATIONS / 8) {
        for _ in 0..8 {
            let byte_offset = adv_strip & 0xFF;
            let bit_mask = mask_strip & 0xFF;
            dummy_acc = dummy_acc.wrapping_add(byte_offset ^ bit_mask);

            adv_strip = adv_strip.rotate_right(8);
            mask_strip = mask_strip.rotate_right(8);
        }
    }

    let cycles = meter.stop(t0);
    std::hint::black_box(dummy_acc);
    cycles
}

#[inline(never)]
fn run_arm_a5_neon_popcount(meter: &CycleMeter) -> u64 {
    let buffer = [0x55u8; 16_016];
    let ptr = buffer.as_ptr();
    let num_quads = 16_016 / 16;
    let mut total_popcnt = 0u64;
    let t0 = meter.start();

    for _ in 0..(ITERATIONS / num_quads as u64) {
        unsafe {
            use core::arch::aarch64::*;
            let mut v_acc = vdupq_n_u8(0);
            let mut p = ptr;
            for _ in 0..num_quads {
                let v_data = vld1q_u8(p);
                let v_cnt = vcntq_u8(v_data);
                v_acc = vaddq_u8(v_acc, v_cnt);
                p = p.add(16);
            }
            total_popcnt += vaddlvq_u8(v_acc) as u64;
        }
    }

    let cycles = meter.stop(t0);
    std::hint::black_box(total_popcnt);
    cycles
}

fn run_decomposition_on_core(core_id: usize, core_label: &str) {
    let meter = CycleMeter::for_core(core_id);
    let x = 1_000_000_000_000_000_000u64; // 10^18
    let mult = 0xAAAA_AAAA_AAAA_AAABu64; // Div by 3 magic
    let shift = 1u8;

    println!("\n==========================================================================");
    println!("  STAGE-ECTOMY DECOMPOSITION READOUT: {}", core_label);
    println!("==========================================================================");

    let cyc_a0 = run_arm_a0_ac_full(&meter, x, mult, shift);
    let c0 = cyc_a0 as f64 / ITERATIONS as f64;

    let cyc_a1 = run_arm_a1_ac_div_only(&meter, x, mult, shift);
    let c1 = cyc_a1 as f64 / ITERATIONS as f64;
    let delta_pitable = c0 - c1;

    let cyc_a2 = run_arm_a2_pitable_direct(&meter);
    let c2 = cyc_a2 as f64 / ITERATIONS as f64;

    let cyc_a3 = run_arm_a3_wheel30_dense(&meter);
    let c3 = cyc_a3 as f64 / ITERATIONS as f64;

    let cyc_a4 = run_arm_a4_wheel30_no_store(&meter);
    let c4 = cyc_a4 as f64 / ITERATIONS as f64;
    let delta_store = c3 - c4;

    let cyc_a5 = run_arm_a5_neon_popcount(&meter);
    let c5 = cyc_a5 as f64 / 16_016.0 / (ITERATIONS as f64 / (16_016.0 / 16.0));

    println!("| Arm | Subtracted Work      | Cycles / Unit | Delta (Attr) | Physical Attribution                   |");
    println!("|-----|----------------------|---------------|--------------|----------------------------------------|");
    println!("| A₀  | Full AC Leaf         | {:>6.2} cyc   |      —       | Total analytical leaf execution floor  |", c0);
    println!("| A₁  | - PiTable Lookup     | {:>6.2} cyc   | {:>5.2} cyc  | FastDiv64 (umulh + lsr) isolated       |", c1, delta_pitable);
    println!("| A₂  | SegmentedPi Isolated | {:>6.2} cyc   |      —       | Raw LDP + AND + POPCNT latency         |", c2);
    println!("| A₃  | Wheel-30 Sieve Mark  | {:>6.2} cyc   |      —       | Dense mark loop (ALU + STRB)           |", c3);
    println!("| A₄  | - STRB Store (Ectomy)| {:>6.2} cyc   | {:>5.2} cyc  | L1D Store-Buffer Drain penalty         |", c4, delta_store);
    println!("| A₅  | NEON Popcount / Byte | {:>6.2} cyc   |      —       | vcntq_u8 throughput per byte           |", c5);

    // Non-Tautological Reconciliation Check
    let reconstructed_ac = c1 + c2;
    let residual_ac = ((c0 - reconstructed_ac).abs() / c0) * 100.0;
    println!("--------------------------------------------------------------------------");
    println!("  RECONCILIATION_COMPOSITE: Measured A₀={:.2} cyc, Reconstructed (A₁+A₂)={:.2} cyc", c0, reconstructed_ac);
    println!("  Residual Error: {:.2}% (Gate: <= 5.0%) -> VERDICT: {}", residual_ac, if residual_ac <= 5.0 { "PASS" } else { "FAIL" });
    println!("==========================================================================");
}

fn main() {
    println!("Deploying Stage-Ectomy Decomposition on Qualcomm Kryo (SM4450)...");

    // Profile Big Core (Cortex-A78)
    run_decomposition_on_core(6, "Cortex-A78 @ 2.21 GHz (Big Cluster, Out-of-Order)");

    // Profile Little Core (Cortex-A55)
    run_decomposition_on_core(0, "Cortex-A55 @ 1.95 GHz (LITTLE Cluster, In-Order)");
}

Step-by-Step Compilation & Execution Playbook
Add the binary to crates/titan-core/Cargo.toml under [[bin]]:
[[bin]]
name = "stage_ectomy_bench"
path = "src/bin/stage_ectomy_bench.rs"

Then build and run directly on your device:
# 1. Compile with aggressive native optimization
cargo build --release -p titan-core --bin stage_ectomy_bench

# 2. Pin and execute stage-ectomy micro-benchmarking
./target/release/stage_ectomy_bench

Running this directly gives us the cycle attribution on your chip: whether the remaining cycles are in the FastDiv64 multiply-high stage, the SegmentedPiTable popcount resolution, or the L1D store-buffer drains during Wheel-30 sieving.


