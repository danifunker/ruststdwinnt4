//! A global allocator on the process heap (`HeapAlloc`).
//!
//! NT 4.0's heap returns blocks aligned to 8 bytes. A larger alignment is made by
//! allocating `size + align` bytes and keeping the heap's pointer in the word just below
//! the aligned block, as std's Windows allocator does.

use core::alloc::{GlobalAlloc, Layout};
use core::ffi::c_void;

use crate::sys;

/// The alignment `HeapAlloc` guarantees on 32-bit Windows.
pub const MIN_ALIGN: usize = 8;

/// The process heap as a global allocator: `#[global_allocator] static A: NtHeap = NtHeap;`
pub struct NtHeap;

/// The aligned block in an allocation that starts at `raw`, for `align` > [`MIN_ALIGN`]: the
/// first multiple of `align` above `raw`, which leaves room below it for the header word.
pub fn aligned_block(raw: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two() && align > MIN_ALIGN);
    (raw + align) & !(align - 1)
}

/// The bytes to ask the heap for to place a block of `layout` with [`aligned_block`].
pub fn padded_size(layout: Layout) -> Option<usize> {
    layout.size().checked_add(layout.align())
}

fn needs_header(layout: Layout) -> bool {
    layout.align() > MIN_ALIGN
}

unsafe fn alloc_flags(layout: Layout, flags: sys::DWORD) -> *mut u8 {
    // SAFETY (all calls): the process heap is valid for the life of the process.
    unsafe {
        let heap = sys::GetProcessHeap();
        if !needs_header(layout) {
            return sys::HeapAlloc(heap, flags, layout.size()) as *mut u8;
        }
        let Some(size) = padded_size(layout) else { return core::ptr::null_mut() };
        let raw = sys::HeapAlloc(heap, flags, size) as usize;
        if raw == 0 {
            return core::ptr::null_mut();
        }
        let block = aligned_block(raw, layout.align());
        *((block as *mut usize).sub(1)) = raw;
        block as *mut u8
    }
}

unsafe fn heap_pointer(ptr: *mut u8, layout: Layout) -> *mut c_void {
    if needs_header(layout) {
        // SAFETY: alloc_flags stored the heap's pointer in the word below the block.
        unsafe { *((ptr as *mut usize).sub(1)) as *mut c_void }
    } else {
        ptr as *mut c_void
    }
}

unsafe impl GlobalAlloc for NtHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe { alloc_flags(layout, 0) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        unsafe { alloc_flags(layout, sys::HEAP_ZERO_MEMORY) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe {
            sys::HeapFree(sys::GetProcessHeap(), 0, heap_pointer(ptr, layout));
        }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if needs_header(layout) {
            // The aligned block would move within a reallocated one: copy instead.
            unsafe {
                let new = self.alloc(Layout::from_size_align_unchecked(new_size, layout.align()));
                if !new.is_null() {
                    core::ptr::copy_nonoverlapping(ptr, new, layout.size().min(new_size));
                    self.dealloc(ptr, layout);
                }
                new
            }
        } else {
            unsafe {
                sys::HeapReAlloc(sys::GetProcessHeap(), 0, ptr as *mut c_void, new_size) as *mut u8
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aligned_block_is_aligned_with_room_for_the_header() {
        for align in [16usize, 32, 64, 4096] {
            for raw in (8..8192).step_by(8) {
                let block = aligned_block(raw, align);
                assert_eq!(block % align, 0);
                assert!(block - raw >= core::mem::size_of::<usize>(), "no room for the header");
                let layout = Layout::from_size_align(100, align).unwrap();
                assert!(block + layout.size() <= raw + padded_size(layout).unwrap(), "overruns");
            }
        }
    }

    #[test]
    fn padded_size_overflow_is_none() {
        let layout = Layout::from_size_align(usize::MAX - 8, 16);
        // Layout itself refuses sizes that overflow with their alignment.
        assert!(layout.is_err() || padded_size(layout.unwrap()).is_none());
    }
}
