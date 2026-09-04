//! Phase CI-1: Portable CPU topology (SM4450 DynamIQ vs CI SMP).
//!
//! SD4G2 assumptions replaced with runtime detection:
//! - Termux ARM: 6x Cortex-A55 (0..=5, 32KiB L1D shared model, in-order)
//!   + 2x Cortex-A78/A77 (6..=7, 64KiB L1D, OoO). Heterogeneous dispatch wins.
//! - GitHub free CI (`ubuntu-24.04` x64, 4 vCPU / 16GB, Azure Xeon
//!   Platinum 8370C/8272CL/8171M or Emerald Rapids 8573C, HT on):
//!   symmetric SMP, 32-48KiB L1D, large L2/L3, AVX2+POPCNT+BMI2 guaranteed,
//!   AVX512 on newer hosts. No big.LITTLE, no `6/7` pinning.
//! - `ubuntu-24.04-arm` (Cobalt 100, Neoverse-N2 class, 4 vCPU): symmetric
//!   ARM SMP — NEON yes, DynamIQ map no.
//!
//! This module is additive: existing DynamIQ paths stay bit-identical on
//! Android; CI paths clamp to `ncpu` and use symmetric dispatch.

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CpuKind {
    /// Snapdragon 4 Gen 2 / SM4450 class: heterogeneous DynamIQ.
    ArmDynamIQ,
    /// Symmetric SMP (x86_64 CI, ARM CI, unknown): all cores equal.
    Smp,
}

#[derive(Clone, Debug)]
pub struct CpuTopology {
    pub ncpu: usize,
    pub kind: CpuKind,
    /// Usable L1D bytes per core for segment sizing.
    pub l1d_bytes: usize,
    pub arch: &'static str,
}

impl CpuTopology {
    /// Detect topology at runtime. Never panics; falls back to 4-core SMP
    /// (the free-CI shape) when detection fails.
    pub fn detect() -> Self {
        let ncpu = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
            .clamp(1, 128);

        #[cfg(target_os = "android")]
        {
            // Termux on SM4450: trust the DynamIQ map (6L+2B).
            // Even if cpuset restricts us, pinning logic falls back to masks.
            return Self {
                ncpu: ncpu.clamp(1, 8),
                kind: CpuKind::ArmDynamIQ,
                l1d_bytes: 32 * 1024,
                arch: "aarch64-android-dynamiq",
            };
        }

        #[cfg(not(target_os = "android"))]
        {
            // CI / dev hosts: symmetric. Distinguish ISA for segment/SIMD picks.
            #[cfg(target_arch = "x86_64")]
            {
                return Self {
                    ncpu,
                    kind: CpuKind::Smp,
                    // Xeon L1D is 32KiB (older) / 48KiB (newer); 32KiB is safe.
                    l1d_bytes: 32 * 1024,
                    arch: "x86_64-smp",
                };
            }
            #[cfg(target_arch = "aarch64")]
            {
                // Includes ubuntu-24.04-arm (Cobalt 100): SMP, not DynamIQ.
                return Self {
                    ncpu,
                    kind: CpuKind::Smp,
                    l1d_bytes: 32 * 1024,
                    arch: "aarch64-smp",
                };
            }
            #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
            {
                return Self {
                    ncpu,
                    kind: CpuKind::Smp,
                    l1d_bytes: 32 * 1024,
                    arch: "generic-smp",
                };
            }
        }
    }

    /// Threads to use when caller passes 0 / oversized request.
    /// CI free tier: exactly ncpu (4). Never exceed 32 to bound atomics.
    #[inline(always)]
    pub fn optimal_threads(&self, requested: usize) -> usize {
        if requested == 0 {
            return self.ncpu.clamp(1, 32);
        }
        requested.clamp(1, 32).min(self.ncpu.max(1))
    }

    /// Pin target for worker `tid`: DynamIQ map on Android, round-robin on SMP.
    /// Always in `0..ncpu` so we never pin onto non-existent cores 6/7 on 4-vCPU CI.
    #[inline(always)]
    pub fn pin_target(&self, tid: usize) -> usize {
        match self.kind {
            CpuKind::ArmDynamIQ => match tid {
                0..=5 => tid % self.ncpu.max(1),
                6 => 6 % self.ncpu.max(1),
                7 => 7 % self.ncpu.max(1),
                _ => tid % 8 % self.ncpu.max(1),
            },
            CpuKind::Smp => tid % self.ncpu.max(1),
        }
    }

    /// Recommended sieve segment bytes: half of L1D to leave room for
    /// pi-tables / reciprocals / stacks. ARM legacy used 16KiB; x86 uses 32KiB.
    #[inline(always)]
    pub fn segment_bytes(&self) -> usize {
        (self.l1d_bytes / 2).clamp(16 * 1024, 48 * 1024)
    }

    #[inline(always)]
    pub fn is_heterogeneous(&self) -> bool {
        self.kind == CpuKind::ArmDynamIQ
    }
}

/// Convenience: detected optimal thread count for this host.
#[inline(always)]
pub fn optimal_threads(requested: usize) -> usize {
    CpuTopology::detect().optimal_threads(requested)
}

/// Convenience: pin target for worker `tid` on this host.
#[inline(always)]
pub fn pin_target(tid: usize) -> usize {
    CpuTopology::detect().pin_target(tid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_sane() {
        let t = CpuTopology::detect();
        assert!(t.ncpu >= 1 && t.ncpu <= 128);
        assert!(t.l1d_bytes >= 16 * 1024);
        #[cfg(target_os = "android")]
        assert_eq!(t.kind, CpuKind::ArmDynamIQ);
        #[cfg(not(target_os = "android"))]
        assert_eq!(t.kind, CpuKind::Smp);
    }

    #[test]
    fn test_pin_target_in_range() {
        let t = CpuTopology::detect();
        for tid in 0..32 {
            assert!(t.pin_target(tid) < t.ncpu.max(1));
        }
    }

    #[test]
    fn test_smp_round_robin() {
        let t = CpuTopology { ncpu: 4, kind: CpuKind::Smp, l1d_bytes: 32768, arch: "test" };
        assert_eq!(t.pin_target(0), 0);
        assert_eq!(t.pin_target(4), 0);
        assert_eq!(t.pin_target(6), 2);
        assert_eq!(t.pin_target(7), 3);
        assert_eq!(t.optimal_threads(8), 4);
        assert_eq!(t.optimal_threads(0), 4);
    }
}
