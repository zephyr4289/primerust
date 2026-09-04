//! Phase 7.4: Hardware Thread Affinity Subsystem (SM4450 Heterogeneous DynamIQ).
//!
//! Cores 0..=5: Cortex-A55 (Little / Sieve Cluster)
//! Cores 6..=7: Cortex-A78 (Big / Analytical Cluster)

#[cfg(target_os = "linux")]
extern "C" {
    fn sched_setaffinity(pid: i32, cpusetsize: usize, mask: *const u64) -> i32;
    fn gettid() -> i32;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CoreClass {
    Little, // Cortex-A55 (Cores 0..=5)
    Big,    // Cortex-A78 (Cores 6, 7)
}

impl CoreClass {
    #[inline(always)]
    pub fn from_core_id(core_id: usize) -> Self {
        if core_id >= 6 {
            CoreClass::Big
        } else {
            CoreClass::Little
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClusterRole {
    BigAnalytical,   // Cores 6 & 7: AC(x, y), B(x, y), High-Stride Sieve
    LittleStreaming, // Cores 0..=5: D(x, y, z) Wheel-30 Linear Segment Sieve
    AllCores,        // Cores 0..=7: Unified DynamIQ Execution
}

/// Pins the calling thread to a specific single core.
///
/// CI-1: on Android keeps DynamIQ 0..7 map; on all other Linux (CI SMP)
/// clamps into `0..ncpu` so we never pin 4 threads onto cores 6/7.
#[cfg(target_os = "linux")]
pub fn pin_thread_to_core(core_id: usize) -> bool {
    #[cfg(target_os = "android")]
    {
        if core_id >= 64 {
            return false;
        }
        let mask: u64 = 1u64 << core_id;
        unsafe {
            let tid = gettid();
            sched_setaffinity(tid, std::mem::size_of::<u64>(), &mask) == 0
        }
    }
    #[cfg(not(target_os = "android"))]
    {
        let ncpu = crate::cpu::CpuTopology::detect().ncpu.max(1).min(64);
        let target = core_id % ncpu;
        let mask: u64 = 1u64 << target;
        unsafe {
            let tid = gettid();
            sched_setaffinity(tid, std::mem::size_of::<u64>(), &mask) == 0
        }
    }
}

#[cfg(not(target_os = "linux"))]
pub fn pin_thread_to_core(_core_id: usize) -> bool {
    false
}

/// Binds calling thread to designated core_id.
#[inline(always)]
pub fn pin_to_core(core_id: usize) -> bool {
    pin_thread_to_core(core_id)
}

/// Binds the current calling OS thread to the designated physical hardware cluster.
///
/// CI-1: DynamIQ masks only on Android. On SMP CI all roles map to the
/// full detected mask (pinning half the VM would strand threads).
#[cfg(target_os = "linux")]
pub fn pin_thread_to_cluster(role: ClusterRole) -> bool {
    #[cfg(target_os = "android")]
    {
        let mask: u64 = match role {
            ClusterRole::BigAnalytical => (1u64 << 6) | (1u64 << 7),
            ClusterRole::LittleStreaming => (1u64 << 0) | (1u64 << 1) | (1u64 << 2) | (1u64 << 3) | (1u64 << 4) | (1u64 << 5),
            ClusterRole::AllCores => 0xFF,
        };
        unsafe {
            let tid = gettid();
            sched_setaffinity(tid, std::mem::size_of::<u64>(), &mask) == 0
        }
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = role;
        let ncpu = crate::cpu::CpuTopology::detect().ncpu.max(1).min(64);
        let mut mask: u64 = 0;
        for c in 0..ncpu {
            mask |= 1u64 << c;
        }
        unsafe {
            let tid = gettid();
            sched_setaffinity(tid, std::mem::size_of::<u64>(), &mask) == 0
        }
    }
}

#[cfg(not(target_os = "linux"))]
pub fn pin_thread_to_cluster(_role: ClusterRole) -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_core_class_detection() {
        assert_eq!(CoreClass::from_core_id(0), CoreClass::Little);
        assert_eq!(CoreClass::from_core_id(5), CoreClass::Little);
        assert_eq!(CoreClass::from_core_id(6), CoreClass::Big);
        assert_eq!(CoreClass::from_core_id(7), CoreClass::Big);
    }

    #[test]
    fn test_pin_thread_to_cluster() {
        let _ = pin_thread_to_cluster(ClusterRole::BigAnalytical);
        let _ = pin_thread_to_cluster(ClusterRole::LittleStreaming);
        let _ = pin_thread_to_cluster(ClusterRole::AllCores);
    }
}
