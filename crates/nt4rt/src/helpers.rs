//! The helpers LLVM calls on Windows on ARM that the MSVC C run-time defines (PINT's are
//! in its crt/helper/arm/llcvt.c, which NTDLL does not export): the conversions between
//! 64-bit integers and floating point. NTDLL exports the division helpers (__rt_udiv,
//! __rt_sdiv64...) and __chkstk, so those come from there.
//!
//! Each forwards to compiler_builtins' function of the same conversion. A Rust `as` cast
//! cannot be used: LLVM lowers it to a call of the very helper being defined.
#![allow(non_upper_case_globals)]

extern "C" {
    fn __floatdidf(a: i64) -> f64;
    fn __floatundidf(a: u64) -> f64;
    fn __floatdisf(a: i64) -> f32;
    fn __floatundisf(a: u64) -> f32;
    fn __fixdfdi(a: f64) -> i64;
    fn __fixunsdfdi(a: f64) -> u64;
    fn __fixsfdi(a: f32) -> i64;
    fn __fixunssfdi(a: f32) -> u64;
}

#[no_mangle]
pub extern "C" fn __i64tod(a: i64) -> f64 {
    unsafe { __floatdidf(a) }
}

#[no_mangle]
pub extern "C" fn __u64tod(a: u64) -> f64 {
    unsafe { __floatundidf(a) }
}

#[no_mangle]
pub extern "C" fn __i64tos(a: i64) -> f32 {
    unsafe { __floatdisf(a) }
}

#[no_mangle]
pub extern "C" fn __u64tos(a: u64) -> f32 {
    unsafe { __floatundisf(a) }
}

#[no_mangle]
pub extern "C" fn __dtoi64(a: f64) -> i64 {
    unsafe { __fixdfdi(a) }
}

#[no_mangle]
pub extern "C" fn __dtou64(a: f64) -> u64 {
    unsafe { __fixunsdfdi(a) }
}

#[no_mangle]
pub extern "C" fn __stoi64(a: f32) -> i64 {
    unsafe { __fixsfdi(a) }
}

#[no_mangle]
pub extern "C" fn __stou64(a: f32) -> u64 {
    unsafe { __fixunssfdi(a) }
}

/// Half to single precision; the base AAPCS (core registers), as compiler_builtins
/// declares it on ARM.
#[no_mangle]
pub extern "aapcs" fn __gnu_h2f_ieee(a: u16) -> f32 {
    f32::from_bits(crate::half::half_to_single(a))
}

/// Single to half precision; the base AAPCS (core registers).
#[no_mangle]
pub extern "aapcs" fn __gnu_f2h_ieee(a: f32) -> u16 {
    crate::half::single_to_half(a.to_bits())
}

/// The fused multiply-add of the C library (`x * y + z`, rounded once), with the VFPv4
/// instruction every PINT processor has (Cortex-A7, A53, A72).
#[no_mangle]
pub extern "C" fn fma(x: f64, y: f64, z: f64) -> f64 {
    let mut acc = z;
    // SAFETY: vfma only reads and writes the registers named.
    unsafe {
        core::arch::asm!("vfma.f64 {acc}, {x}, {y}", acc = inout(dreg) acc, x = in(dreg) x,
            y = in(dreg) y, options(pure, nomem, nostack));
    }
    acc
}

/// The single-precision fused multiply-add of the C library.
#[no_mangle]
pub extern "C" fn fmaf(x: f32, y: f32, z: f32) -> f32 {
    let mut acc = z;
    // SAFETY: vfma only reads and writes the registers named.
    unsafe {
        core::arch::asm!("vfma.f32 {acc}, {x}, {y}", acc = inout(sreg) acc, x = in(sreg) x,
            y = in(sreg) y, options(pure, nomem, nostack));
    }
    acc
}

/// The symbol a program that uses floating point refers to, which tells the MSVC C run-time
/// to set up floating point. NT sets up the VFP itself; the symbol only has to exist.
#[no_mangle]
#[used]
pub static _fltused: i32 = 0;
