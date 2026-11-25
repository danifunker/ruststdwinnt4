//! What a program gets from rustc but not from mrustc (the Rust-to-C translator used for
//! Alpha, MIPS and PowerPC, see PLAN-mips-alpha-ppc.md), compiled only by mrustc:
//!
//! - mrustc has no `#![no_main]`: it needs the `start` lang item and writes a C `main` that
//!   calls it, with the program's `main`. The C run-time's start-up calls that `main` (nt4rt's
//!   own `mainCRTStartup` is left out here), so a program's `main` is reached the same way
//!   [`entry!`](crate::entry) reaches it under rustc.
//! - mrustc ignores `#[global_allocator]`: its allocator shims (`__rust_alloc`...) always call
//!   std's default allocator, `__rdl_alloc` and the rest. A `no_std` program gets those here,
//!   on [`NtHeap`].
//! - `cbrt` and `fdim` (and the `f32` versions), which core calls and Visual C++ 4.1's C
//!   library lacks.

use core::alloc::{GlobalAlloc, Layout};

use crate::heap::NtHeap;

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

// (alloc defines __rdl_oom itself.)

// What core calls in the C library that Visual C++ 4.1's lacks (C99's).
#[no_mangle]
extern "C" fn cbrt(x: f64) -> f64 {
    if x == 0.0 || x != x || x == f64::INFINITY || x == f64::NEG_INFINITY {
        return x;
    }
    // A first guess from the exponent (the bits divided by 3), then Newton's method.
    let bits = x.abs().to_bits();
    let mut y = f64::from_bits(bits / 3 + 0x2A9F_7893_782D_A1CE);
    for _ in 0..6 {
        y -= (y * y * y - x.abs()) / (3.0 * y * y);
    }
    if x < 0.0 { -y } else { y }
}

#[no_mangle]
extern "C" fn cbrtf(x: f32) -> f32 {
    cbrt(x as f64) as f32
}

#[no_mangle]
extern "C" fn fdim(x: f64, y: f64) -> f64 {
    if x != x || y != y {
        x + y
    } else if x > y {
        x - y
    } else {
        0.0
    }
}

#[no_mangle]
extern "C" fn fdimf(x: f32, y: f32) -> f32 {
    fdim(x as f64, y as f64) as f32
}
