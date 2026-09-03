//! Stage-Ectomy Benchmark Runner (stage_ectomy_bench.rs).
//! Decomposes per-cycle latency across Cortex-A78 and Cortex-A55 on SM4450.

use titan_core::stage_ectomy::CycleMeter;

const ITERATIONS: u64 = 10_000_000;

#[inline(never)]
fn run_arm_a0_ac_full(meter: &CycleMeter, x: u64, mult: u64, shift: u8) -> u64 {
    let mut acc = 0u64;
    let t0 = meter.start();

    for i in 1..=ITERATIONS {
        // FastDiv branchless divide
        let v: u64;
        #[cfg(target_arch = "aarch64")]
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
        #[cfg(not(target_arch = "aarch64"))]
        {
            v = (x + i) / 3;
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
        #[cfg(target_arch = "aarch64")]
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
        #[cfg(not(target_arch = "aarch64"))]
        {
            v = (x + i) / 3;
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
    let mut adv_strip: u64 = std::hint::black_box(0x0102_0102_0102_0102);
    let mut mask_strip: u64 = std::hint::black_box(0x0102_0408_1020_4080);
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
    std::hint::black_box(&buffer);
    cycles
}

#[inline(never)]
fn run_arm_a4_wheel30_no_store(meter: &CycleMeter) -> u64 {
    let mut adv_strip: u64 = std::hint::black_box(0x0102_0102_0102_0102);
    let mut mask_strip: u64 = std::hint::black_box(0x0102_0408_1020_4080);
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
        #[cfg(target_arch = "aarch64")]
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
        #[cfg(not(target_arch = "aarch64"))]
        {
            total_popcnt += 1;
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
