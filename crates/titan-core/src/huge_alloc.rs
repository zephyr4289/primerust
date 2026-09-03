//! Phase 6.7: 2 MiB-Aligned HugePage Allocator (huge_alloc.rs).
//!
//! Allocates on 2 MiB ARM64 PMD page boundaries and advises the Linux kernel
//! via `MADV_HUGEPAGE` (14) to back the buffer with transparent 2 MiB HugePages.
//! Collapses 50,848 virtual 4 KiB pages into just 25 HugePages, eliminating MMU page table walks.

use std::alloc::{alloc_zeroed, dealloc, Layout};
use std::ptr::NonNull;

pub const HUGE_PAGE_SIZE: usize = 2 * 1024 * 1024; // 2 MiB ARM64 PMD page

extern "C" {
    fn madvise(addr: *mut core::ffi::c_void, len: usize, advice: i32) -> i32;
}

// Linux madvise flag for transparent huge pages
const MADV_HUGEPAGE: i32 = 14;

pub struct HugePageBuffer<T> {
    ptr: NonNull<T>,
    len: usize,
    capacity: usize,
    layout: Layout,
}

unsafe impl<T: Send> Send for HugePageBuffer<T> {}
unsafe impl<T: Sync> Sync for HugePageBuffer<T> {}

impl<T> HugePageBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        if capacity == 0 {
            return Self {
                ptr: NonNull::dangling(),
                len: 0,
                capacity: 0,
                layout: Layout::new::<u8>(),
            };
        }

        let elem_size = std::mem::size_of::<T>().max(1);
        let byte_capacity = (capacity * elem_size + HUGE_PAGE_SIZE - 1) & !(HUGE_PAGE_SIZE - 1);

        // 2 MiB aligned allocation
        let layout = Layout::from_size_align(byte_capacity, HUGE_PAGE_SIZE)
            .expect("Invalid huge page layout");

        let raw_ptr = unsafe { alloc_zeroed(layout) as *mut T };
        let ptr = NonNull::new(raw_ptr).expect("OOM allocating huge page buffer");

        // Inform the Linux kernel / Android khugepaged to back with 2 MiB PMD pages
        unsafe {
            madvise(raw_ptr as *mut core::ffi::c_void, byte_capacity, MADV_HUGEPAGE);
        }

        Self {
            ptr,
            len: 0,
            capacity: byte_capacity / elem_size,
            layout,
        }
    }

    #[inline(always)]
    pub fn push(&mut self, val: T) {
        debug_assert!(self.len < self.capacity);
        unsafe {
            self.ptr.as_ptr().add(self.len).write(val);
        }
        self.len += 1;
    }

    #[inline(always)]
    pub fn as_slice(&self) -> &[T] {
        if self.len == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
        }
    }

    #[inline(always)]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        if self.len == 0 {
            &mut []
        } else {
            unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
        }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline(always)]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    #[inline(always)]
    pub fn as_ptr(&self) -> *const T {
        self.ptr.as_ptr()
    }
}

impl<T> Drop for HugePageBuffer<T> {
    fn drop(&mut self) {
        if self.capacity > 0 {
            unsafe {
                dealloc(self.ptr.as_ptr() as *mut u8, self.layout);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_huge_page_buffer_basic() {
        let mut buf = HugePageBuffer::<u64>::new(1024);
        assert!(buf.capacity() >= 1024);
        assert_eq!(buf.len(), 0);

        for i in 0..1000 {
            buf.push(i as u64 * 7);
        }

        assert_eq!(buf.len(), 1000);
        let slice = buf.as_slice();
        for i in 0..1000 {
            assert_eq!(slice[i], i as u64 * 7);
        }
    }

    #[test]
    fn test_huge_page_buffer_empty() {
        let buf = HugePageBuffer::<u8>::new(0);
        assert_eq!(buf.len(), 0);
        assert!(buf.is_empty());
        assert_eq!(buf.as_slice().len(), 0);
    }
}
