//! `RtlGenRandom` (`SystemFunction036` of advapi32, Windows XP) on NT 4.0.
//!
//! std takes its random numbers from it (the keys of `HashMap`'s `RandomState`). NT 4.0
//! has neither it nor, on PINT, `CryptGenRandom`, so the numbers come from a SplitMix64
//! generator: its state starts from the clocks, the performance counter and the process's
//! and thread's ids and addresses, and each call mixes in the performance counter again.
//! That is unpredictable enough for hash keys, but it is **not** a cryptographic random
//! source.

use core::sync::atomic::{AtomicU64, Ordering};

use crate::sys;

provide! {
    SystemFunction036 = rtl_gen_random, 8;
}

static STATE: AtomicU64 = AtomicU64::new(0);

const GOLDEN: u64 = 0x9E37_79B9_7F4A_7C15;

fn mix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn counter() -> u64 {
    let mut c = 0i64;
    // SAFETY: c is valid.
    unsafe { sys::QueryPerformanceCounter(&mut c) };
    c as u64
}

fn seed() -> u64 {
    let mut t = sys::FILETIME::default();
    // SAFETY: plain calls; t is valid.
    unsafe {
        sys::GetSystemTimeAsFileTime(&mut t);
        let mut s = mix(counter() ^ ((t.high as u64) << 32 | t.low as u64));
        s = mix(s ^ ((sys::GetCurrentProcessId() as u64) << 32 | sys::GetCurrentThreadId() as u64));
        s = mix(s ^ sys::GetTickCount() as u64 ^ ((&raw const s as usize as u64) << 16));
        s | 1
    }
}

unsafe extern "system" fn rtl_gen_random(buffer: *mut u8, length: u32) -> u8 {
    if STATE.load(Ordering::Relaxed) == 0 {
        let _ = STATE.compare_exchange(0, seed(), Ordering::Relaxed, Ordering::Relaxed);
    }
    let extra = counter();
    let mut i = 0usize;
    while i < length as usize {
        let x = mix(STATE.fetch_add(GOLDEN, Ordering::Relaxed).wrapping_add(GOLDEN) ^ extra);
        for b in x.to_le_bytes() {
            if i == length as usize {
                break;
            }
            // SAFETY: the caller's buffer holds length bytes.
            unsafe { *buffer.add(i) = b };
            i += 1;
        }
    }
    1
}
