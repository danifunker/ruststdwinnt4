//! The helpers the compiler calls on x86 that the MSVC C run-time would define.
//!
//! The 64-bit integer helpers (`_alldiv`, `_aulldiv`, `_allrem`, `_aullrem`, `_allmul`,
//! the shifts) and `_chkstk` are NTDLL's on x86 NT 4.0, as on every NT since 3.1: naming
//! the library is enough for the linker to take them from there.

#[link(name = "ntdll")]
extern "C" {}

/// Referenced by every object that uses floating point, for the C run-time to pull in
/// its floating-point support; there is none to pull in here.
#[no_mangle]
#[used]
pub static _fltused: i32 = 0x9875;

/// Half to single precision, as LLVM calls it on x86 without SSE.
#[no_mangle]
pub extern "C" fn __gnu_h2f_ieee(a: u16) -> f32 {
    f32::from_bits(crate::half::half_to_single(a))
}

/// Single to half precision.
#[no_mangle]
pub extern "C" fn __gnu_f2h_ieee(a: f32) -> u16 {
    crate::half::single_to_half(a.to_bits())
}
