//! What a program gets from rustc but not from mrustc (the Rust-to-C translator used for
//! Alpha, MIPS and PowerPC, see PLAN-mips-alpha-ppc.md), compiled only by mrustc:
//!
//! - mrustc has no `#![no_main]`: it needs the `start` lang item and writes a C `main` that
//!   calls it. The process still starts in [`mainCRTStartup`](crate::start), which calls the
//!   `__nt4rt_main` that [`entry!`](crate::entry) defines; this `start` is only there to be
//!   referenced.
//! - mrustc ignores `#[global_allocator]`: its allocator shims (`__rust_alloc`...) always call
//!   std's default allocator, `__rdl_alloc` and the rest. A `no_std` program gets those here,
//!   on [`NtHeap`].

use core::alloc::{GlobalAlloc, Layout};

use crate::heap::NtHeap;
use crate::sys;

#[lang = "start"]
fn lang_start<T: crate::Termination + 'static>(
    main: fn() -> T,
    _argc: isize,
    _argv: *const *const u8,
    _sigpipe: u8,
) -> isize {
    main().report() as isize
}

#[no_mangle]
unsafe extern "C" fn __rdl_alloc(size: usize, align: usize) -> *mut u8 {
    // SAFETY (all four): the shims pass a size and alignment that came from a valid Layout.
    unsafe { NtHeap.alloc(Layout::from_size_align_unchecked(size, align)) }
}

#[no_mangle]
unsafe extern "C" fn __rdl_alloc_zeroed(size: usize, align: usize) -> *mut u8 {
    unsafe { NtHeap.alloc_zeroed(Layout::from_size_align_unchecked(size, align)) }
}

#[no_mangle]
unsafe extern "C" fn __rdl_dealloc(ptr: *mut u8, size: usize, align: usize) {
    unsafe { NtHeap.dealloc(ptr, Layout::from_size_align_unchecked(size, align)) }
}

#[no_mangle]
unsafe extern "C" fn __rdl_realloc(ptr: *mut u8, size: usize, align: usize, new_size: usize) -> *mut u8 {
    unsafe { NtHeap.realloc(ptr, Layout::from_size_align_unchecked(size, align), new_size) }
}

#[no_mangle]
extern "C" fn __rdl_oom(_size: usize, _align: usize) -> ! {
    let _ = core::fmt::Write::write_str(&mut crate::console::stderr(), "memory allocation failed\r\n");
    // SAFETY: ExitProcess takes any code and does not return.
    unsafe { sys::ExitProcess(101) }
}
