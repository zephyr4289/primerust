//! Cortex-A55 Dual-Issue Paced Wheel-30 Sieve Kernel.
//! Interleaves two prime streams to hide in-order store-buffer drain latencies.

#[inline(always)]
pub unsafe fn sieve_wheel30_paced_dual(
    buf_ptr: *mut u8,
    mut off_a: usize,
    mut adv_a: u64,
    mut mask_a: u64,
    mut off_b: usize,
    mut adv_b: u64,
    mut mask_b: u64,
    unrolled_steps: usize,
) -> (usize, u64, u64, usize, u64, u64) {
    let mut step = unrolled_steps;
    if step == 0 {
        return (off_a, adv_a, mask_a, off_b, adv_b, mask_b);
    }

    #[cfg(target_arch = "aarch64")]
    core::arch::asm!(
        "2:",
        // --- STEP 1: Process Prime A Mark & Prep Prime B ---
        "ldrb   {tmp_a:w}, [{buf}, {off_a}]",
        "and    {m_a:w}, {mask_a:w}, #0xff",
        "bic    {tmp_a:w}, {tmp_a:w}, {m_a:w}",
        "ror    {mask_a}, {mask_a}, #8",
        
        "ldrb   {tmp_b:w}, [{buf}, {off_b}]",
        "and    {m_b:w}, {mask_b:w}, #0xff",
        "strb   {tmp_a:w}, [{buf}, {off_a}]",  // Store A (LSU)
        
        "and    {step_a:w}, {adv_a:w}, #0xff", // ALU op executes during Store A drain
        "add    {off_a}, {off_a}, {step_a}",
        "ror    {adv_a}, {adv_a}, #8",

        // --- STEP 2: Process Prime B Mark & Prep Prime A ---
        "bic    {tmp_b:w}, {tmp_b:w}, {m_b:w}",
        "ror    {mask_b}, {mask_b}, #8",
        "strb   {tmp_b:w}, [{buf}, {off_b}]",  // Store B (LSU)
        
        "and    {step_b:w}, {adv_b:w}, #0xff", // ALU op executes during Store B drain
        "add    {off_b}, {off_b}, {step_b}",
        "ror    {adv_b}, {adv_b}, #8",

        "subs   {cnt}, {cnt}, #1",
        "b.ne   2b",

        buf = in(reg) buf_ptr,
        off_a = inout(reg) off_a,
        adv_a = inout(reg) adv_a,
        mask_a = inout(reg) mask_a,
        off_b = inout(reg) off_b,
        adv_b = inout(reg) adv_b,
        mask_b = inout(reg) mask_b,
        cnt = inout(reg) step,
        tmp_a = out(reg) _,
        tmp_b = out(reg) _,
        m_a = out(reg) _,
        m_b = out(reg) _,
        step_a = out(reg) _,
        step_b = out(reg) _,
        options(nostack)
    );

    #[cfg(not(target_arch = "aarch64"))]
    {
        let _ = (buf_ptr, step);
    }

    (off_a, adv_a, mask_a, off_b, adv_b, mask_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paced_dual_basic() {
        let mut buffer = [0xFFu8; 1024];
        let off_a = 10;
        let adv_a = 0x0101_0101_0101_0101u64;
        let mask_a = 0x0101_0101_0101_0101u64;
        let off_b = 20;
        let adv_b = 0x0202_0202_0202_0202u64;
        let mask_b = 0x0202_0202_0202_0202u64;

        unsafe {
            let res = sieve_wheel30_paced_dual(
                buffer.as_mut_ptr(),
                off_a,
                adv_a,
                mask_a,
                off_b,
                adv_b,
                mask_b,
                4,
            );
            assert_eq!(res.0, off_a + 4);
            assert_eq!(res.3, off_b + 8);
        }
    }
}
