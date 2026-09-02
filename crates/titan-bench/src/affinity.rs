//! Phase 42: Heterogeneous CPU Affinity Pinning (Hardware Architecture on SM4450).
//!
//! Qualcomm Snapdragon 4 Gen 2 (SM4450) Architecture:
//!   - Cores 0..=5: Cortex-A55 @ 2.0 GHz (In-Order, 32 KiB L1D) -> Pinned to Sieve & Bucket Workers
//!   - Cores 6..=7: Cortex-A78 @ 2.2 GHz (OoO, 64 KiB L1D) -> Pinned to Coordinator, A(x, y), B(x, y)

#[cfg(target_os = "linux")]
pub fn pin_thread_to_core(core_id: usize) -> bool {
    unsafe {
        let mut set: libc::cpu_set_t = std::mem::zeroed();
        libc::CPU_SET(core_id, &mut set);
        libc::sched_setaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &set) == 0
    }
}

#[cfg(not(target_os = "linux"))]
pub fn pin_thread_to_core(_core_id: usize) -> bool {
    true
}

#[cfg(target_os = "linux")]
pub fn pin_thread_to_cluster(is_big_core: bool) -> bool {
    unsafe {
        let mut set: libc::cpu_set_t = std::mem::zeroed();
        if is_big_core {
            libc::CPU_SET(6, &mut set);
            libc::CPU_SET(7, &mut set);
        } else {
            for c in 0..=5 {
                libc::CPU_SET(c, &mut set);
            }
        }
        libc::sched_setaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &set) == 0
    }
}

#[cfg(not(target_os = "linux"))]
pub fn pin_thread_to_cluster(_is_big_core: bool) -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_affinity_pinning_call() {
        let _ = pin_thread_to_cluster(true);
        let _ = pin_thread_to_cluster(false);
    }
}
