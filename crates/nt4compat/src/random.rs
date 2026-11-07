//! `RtlGenRandom` (`SystemFunction036` of advapi32, Windows XP) on NT 4.0: the random
//! numbers of std (HashMap keys), of getrandom, and of C code such as GnuTLS, which takes
//! its keys from it on NT 4.0.
//!
//! NT 4.0 gives programs no random source of its own on PINT (its advapi32 has no
//! CryptGenRandom), so the bytes come from entropy gathered in software, as NT 4.0's own
//! CryptoAPI and the random devices of the time did:
//!
//! - **Gathering.** Timing jitter: the performance counter read around memory work and
//!   thread switches, a thousand samples, whose low bits depend on caches, interrupts and
//!   the scheduler. Then the clocks; the process's and thread's ids and times; memory
//!   status; the kernel's performance, processor time and interrupt counters
//!   (NtQuerySystemInformation); addresses of the stack and heap; CryptGenRandom where
//!   advapi32 has it (NT 4.0 on x86); and a seed kept in the registry from the last run.
//!   All of it goes through SHA-256 into a 32-byte key.
//! - **Generating.** Output blocks are SHA-256(key, counter, block, performance counter).
//!   After each request the key is replaced by a hash of itself (so what was produced
//!   cannot be recovered from a later key), and every 10 seconds or 65536 requests fresh
//!   entropy is gathered into it.
//! - **The seed.** HKEY_CURRENT_USER\Software\ruststdwinnt4\RandomSeed: read when the
//!   generator starts and replaced, from the new key, right away, so that two runs never
//!   start from the same state even on a machine that boots identically (an emulator).
//!
//! The strength rests on the jitter; there is no hardware source to rely on.

use core::ffi::c_void;
use core::ptr;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::sha256::Sha256;
use crate::sys;

provide! {
    SystemFunction036 = rtl_gen_random, 8;
}

#[link(name = "kernel32")]
extern "system" {
    fn GlobalMemoryStatus(status: *mut [u32; 8]);
    fn GetProcessTimes(p: sys::HANDLE, c: *mut u64, e: *mut u64, k: *mut u64, u: *mut u64) -> sys::BOOL;
    fn GetThreadTimes(t: sys::HANDLE, c: *mut u64, e: *mut u64, k: *mut u64, u: *mut u64) -> sys::BOOL;
    fn GetCurrentProcess() -> sys::HANDLE;
    fn GetCurrentThread() -> sys::HANDLE;
    fn LoadLibraryA(name: *const u8) -> sys::HANDLE;
    fn GetProcAddress(m: sys::HANDLE, name: *const u8) -> *const c_void;
}

#[link(name = "ntdll")]
extern "system" {
    fn NtQuerySystemInformation(class: u32, info: *mut c_void, len: u32, ret: *mut u32) -> i32;
}

#[link(name = "advapi32")]
extern "system" {
    fn RegCreateKeyExW(
        key: sys::HANDLE, sub: *const u16, reserved: u32, class: *const u16, options: u32,
        access: u32, sa: *mut c_void, result: *mut sys::HANDLE, disposition: *mut u32,
    ) -> i32;
    fn RegQueryValueExW(
        key: sys::HANDLE, name: *const u16, reserved: *mut u32, kind: *mut u32, data: *mut u8,
        len: *mut u32,
    ) -> i32;
    fn RegSetValueExW(key: sys::HANDLE, name: *const u16, reserved: u32, kind: u32, data: *const u8, len: u32) -> i32;
    fn RegCloseKey(key: sys::HANDLE) -> i32;
}

struct State {
    key: [u8; 32],
    counter: u64,
    requests: u32,
    reseeded_at: u32,
    started: bool,
}

static LOCK: AtomicBool = AtomicBool::new(false);
static mut STATE: State = State { key: [0; 32], counter: 0, requests: 0, reseeded_at: 0, started: false };

fn counter() -> u64 {
    let mut c = 0i64;
    // SAFETY: c is valid.
    unsafe { sys::QueryPerformanceCounter(&mut c) };
    c as u64
}

/// Timing jitter: the performance counter around a little memory work, with a thread
/// switch now and then.
fn jitter(h: &mut Sha256, samples: usize) {
    let mut scratch = [0u8; 2048];
    let mut x: u32 = 0x9E37_79B9;
    let mut last = counter();
    for i in 0..samples {
        // Work whose length varies with the data and the caches.
        let n = 16 + (x as usize & 63);
        for j in 0..n {
            let k = (x as usize).wrapping_add(j * 67) & (scratch.len() - 1);
            scratch[k] = scratch[k].wrapping_add(x as u8);
            x = x.rotate_left(5) ^ scratch[k] as u32 ^ j as u32;
        }
        if i % 64 == 63 {
            // SAFETY: plain call.
            unsafe { sys::SwitchToThread() };
        }
        let now = counter();
        h.update(&(now.wrapping_sub(last)).to_le_bytes());
        x ^= now as u32;
        last = now;
    }
    h.update(&scratch[..64]);
}

/// Everything else the system state offers.
fn system_state(h: &mut Sha256) {
    let mut ft = sys::FILETIME::default();
    // SAFETY (whole body): plain calls on buffers of the sizes given.
    unsafe {
        sys::GetSystemTimeAsFileTime(&mut ft);
        h.update(&ft.low.to_le_bytes());
        h.update(&ft.high.to_le_bytes());
        h.update(&sys::GetTickCount().to_le_bytes());
        h.update(&sys::GetCurrentProcessId().to_le_bytes());
        h.update(&sys::GetCurrentThreadId().to_le_bytes());
        let mut mem = [0u32; 8];
        mem[0] = 32;
        GlobalMemoryStatus(&mut mem);
        for v in mem {
            h.update(&v.to_le_bytes());
        }
        let mut t = [0u64; 4];
        GetProcessTimes(GetCurrentProcess(), &mut t[0], &mut t[1], &mut t[2], &mut t[3]);
        GetThreadTimes(GetCurrentThread(), &mut t[0], &mut t[1], &mut t[2], &mut t[3]);
        for v in t {
            h.update(&v.to_le_bytes());
        }
        // The kernel's counters: performance (2), time of day (3), processor times (8),
        // interrupts (23). Their sizes vary by version: each is tried with a large buffer
        // and taken as far as the kernel filled it.
        let mut buf = [0u8; 1024];
        for class in [2u32, 3, 8, 23] {
            let mut ret = 0u32;
            let s = NtQuerySystemInformation(class, buf.as_mut_ptr().cast(), buf.len() as u32, &mut ret);
            h.update(&s.to_le_bytes());
            if s >= 0 {
                h.update(&buf[..(ret as usize).min(buf.len())]);
            }
        }
        let local = 0u8;
        h.update(&(&raw const local as usize).to_le_bytes());
        let heap = sys::HeapAlloc(sys::GetProcessHeap(), 0, 16);
        h.update(&(heap as usize).to_le_bytes());
        sys::HeapFree(sys::GetProcessHeap(), 0, heap);
        h.update(&counter().to_le_bytes());
    }
}

/// CryptGenRandom, where this NT 4.0 has it (x86, not PINT): 32 more bytes.
fn cryptoapi(h: &mut Sha256) {
    type Acquire = unsafe extern "system" fn(*mut usize, *const u8, *const u8, u32, u32) -> sys::BOOL;
    type Gen = unsafe extern "system" fn(usize, u32, *mut u8) -> sys::BOOL;
    type Release = unsafe extern "system" fn(usize, u32) -> sys::BOOL;
    const PROV_RSA_FULL: u32 = 1;
    const CRYPT_VERIFYCONTEXT: u32 = 0xF000_0000;
    // SAFETY: the functions, when there, have these signatures.
    unsafe {
        let m = LoadLibraryA(c"advapi32.dll".as_ptr().cast());
        if m.is_null() {
            return;
        }
        let (a, g, r) = (
            GetProcAddress(m, c"CryptAcquireContextA".as_ptr().cast()),
            GetProcAddress(m, c"CryptGenRandom".as_ptr().cast()),
            GetProcAddress(m, c"CryptReleaseContext".as_ptr().cast()),
        );
        if a.is_null() || g.is_null() || r.is_null() {
            return;
        }
        let (a, g, r): (Acquire, Gen, Release) =
            (core::mem::transmute(a), core::mem::transmute(g), core::mem::transmute(r));
        let mut prov = 0usize;
        if a(&mut prov, ptr::null(), ptr::null(), PROV_RSA_FULL, CRYPT_VERIFYCONTEXT) != 0 {
            let mut b = [0u8; 32];
            if g(prov, 32, b.as_mut_ptr()) != 0 {
                h.update(&b);
            }
            r(prov, 0);
        }
    }
}

const SEED_KEY: &[u16] = &[
    b'S' as u16, b'o' as u16, b'f' as u16, b't' as u16, b'w' as u16, b'a' as u16, b'r' as u16,
    b'e' as u16, b'\\' as u16, b'r' as u16, b'u' as u16, b's' as u16, b't' as u16, b's' as u16,
    b't' as u16, b'd' as u16, b'w' as u16, b'i' as u16, b'n' as u16, b'n' as u16, b't' as u16,
    b'4' as u16, 0,
];
const SEED_VALUE: &[u16] = &[
    b'R' as u16, b'a' as u16, b'n' as u16, b'd' as u16, b'o' as u16, b'm' as u16, b'S' as u16,
    b'e' as u16, b'e' as u16, b'd' as u16, 0,
];

/// The registry seed: mixed into `h` (when there), then replaced by `next`.
fn seed_file(h: &mut Sha256, next: Option<&[u8; 32]>) {
    const HKEY_CURRENT_USER: usize = 0x8000_0001;
    const KEY_ALL_ACCESS: u32 = 0xF003F;
    const REG_BINARY: u32 = 3;
    // SAFETY: registry calls on buffers of the sizes given.
    unsafe {
        let mut k: sys::HANDLE = ptr::null_mut();
        let mut disp = 0;
        if RegCreateKeyExW(
            HKEY_CURRENT_USER as sys::HANDLE, SEED_KEY.as_ptr(), 0, ptr::null(), 0, KEY_ALL_ACCESS,
            ptr::null_mut(), &mut k, &mut disp,
        ) != 0
        {
            return;
        }
        let mut b = [0u8; 32];
        let (mut kind, mut len) = (0u32, 32u32);
        if RegQueryValueExW(k, SEED_VALUE.as_ptr(), ptr::null_mut(), &mut kind, b.as_mut_ptr(), &mut len) == 0 {
            h.update(&b[..len.min(32) as usize]);
        }
        if let Some(n) = next {
            RegSetValueExW(k, SEED_VALUE.as_ptr(), 0, REG_BINARY, n.as_ptr(), 32);
        }
        RegCloseKey(k);
    }
}

fn hash_key(parts: &[&[u8]]) -> [u8; 32] {
    let mut h = Sha256::new();
    for p in parts {
        h.update(p);
    }
    h.finish()
}

/// Gather entropy into the key (the first time: all sources and the seed).
fn reseed(s: &mut State) {
    let mut h = Sha256::new();
    h.update(b"ruststdwinnt4 reseed");
    h.update(&s.key);
    jitter(&mut h, if s.started { 256 } else { 1024 });
    system_state(&mut h);
    if !s.started {
        cryptoapi(&mut h);
        seed_file(&mut h, None);
    }
    s.key = h.finish();
    if !s.started {
        s.started = true;
        let next = hash_key(&[b"ruststdwinnt4 next seed", &s.key]);
        seed_file(&mut Sha256::new(), Some(&next));
        s.key = hash_key(&[b"ruststdwinnt4 after seed", &s.key]);
    }
    // SAFETY: plain call.
    s.reseeded_at = unsafe { sys::GetTickCount() };
    s.requests = 0;
}

unsafe extern "system" fn rtl_gen_random(buffer: *mut u8, length: u32) -> u8 {
    while LOCK.compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() {
        // SAFETY: plain call.
        unsafe { sys::SwitchToThread() };
    }
    // SAFETY: the lock is held; one generator state.
    let s = unsafe { &mut *ptr::addr_of_mut!(STATE) };
    // SAFETY: plain call.
    let now = unsafe { sys::GetTickCount() };
    if !s.started || s.requests >= 65536 || now.wrapping_sub(s.reseeded_at) > 10_000 {
        reseed(s);
    }
    s.requests += 1;
    s.counter += 1;
    let stamp = counter().to_le_bytes();
    let mut done = 0usize;
    let mut block = 0u32;
    while done < length as usize {
        let out = hash_key(&[b"ruststdwinnt4 out", &s.key, &s.counter.to_le_bytes(), &block.to_le_bytes(), &stamp]);
        let n = (length as usize - done).min(32);
        // SAFETY: the caller's buffer holds length bytes.
        unsafe { ptr::copy_nonoverlapping(out.as_ptr(), buffer.add(done), n) };
        done += n;
        block += 1;
    }
    s.key = hash_key(&[b"ruststdwinnt4 rekey", &s.key, &s.counter.to_le_bytes()]);
    LOCK.store(false, Ordering::Release);
    1
}
