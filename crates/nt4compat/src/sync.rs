//! `WaitOnAddress`, `WakeByAddressSingle` and `WakeByAddressAll` (Windows 8) on NT 4.0.
//!
//! std builds its locks, condition variables, `Once` and thread parking on these three
//! functions, so they are the whole of its synchronization on NT 4.0.
//!
//! A waiting thread puts a record of itself (on its own stack) in the list of one of 64
//! buckets, chosen by the address it waits on, and waits on its own auto-reset event. A
//! wake takes the records with that address off the list and sets their events. The
//! value at the address is compared with the expected one while the bucket's lock is held,
//! and the thread that changes the value wakes after it, taking the same lock, so a wake
//! is never lost. As with the real functions, a wait may also end without a wake
//! (spuriously), which std allows for.
//!
//! A bucket's lock is a spin lock that yields the processor (`SwitchToThread`, then
//! `Sleep`): it is held only for a few list operations. Each thread's event is made the
//! first time it waits and closed when the thread ends (a TLS callback).

use core::cell::UnsafeCell;
use core::ffi::c_void;
use core::ptr;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use crate::sys::{self, HANDLE};

provide! {
    WaitOnAddress = wait_on_address, 16;
    WakeByAddressSingle = wake_by_address_single, 4;
    WakeByAddressAll = wake_by_address_all, 4;
}

struct Waiter {
    address: usize,
    event: HANDLE,
    woken: bool,
    next: *mut Waiter,
}

struct Bucket {
    locked: AtomicBool,
    head: UnsafeCell<*mut Waiter>,
}

// SAFETY: head is only touched with the lock held.
unsafe impl Sync for Bucket {}

const BUCKETS: usize = 64;

static TABLE: [Bucket; BUCKETS] =
    [const { Bucket { locked: AtomicBool::new(false), head: UnsafeCell::new(ptr::null_mut()) } }; BUCKETS];

fn bucket(address: usize) -> &'static Bucket {
    // Fibonacci hashing of the address, without its low bits (aligned values).
    let h = ((address >> 2) as u32).wrapping_mul(0x9E37_79B9) >> 26;
    &TABLE[h as usize % BUCKETS]
}

impl Bucket {
    fn lock(&self) -> Guard<'_> {
        let mut spins = 0u32;
        while self
            .locked
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            spins += 1;
            // SAFETY: plain calls.
            unsafe {
                if spins < 16 {
                    sys::SwitchToThread();
                } else {
                    sys::Sleep(if spins < 64 { 0 } else { 1 });
                }
            }
        }
        Guard(self)
    }
}

struct Guard<'a>(&'a Bucket);

impl Guard<'_> {
    fn head(&mut self) -> &mut *mut Waiter {
        // SAFETY: the lock is held for as long as the guard lives.
        unsafe { &mut *self.0.head.get() }
    }

    /// Take `w` off the list, if it is on it.
    fn unlink(&mut self, w: *mut Waiter) {
        let mut link: *mut *mut Waiter = self.head();
        // SAFETY: the list's records are live: a waiter takes its record off before it
        // returns, or a waker does before it sets the event.
        unsafe {
            while !(*link).is_null() {
                if *link == w {
                    *link = (*w).next;
                    return;
                }
                link = &mut (**link).next;
            }
        }
    }
}

impl Drop for Guard<'_> {
    fn drop(&mut self) {
        self.0.locked.store(false, Ordering::Release);
    }
}

/// The TLS slot of each thread's event; 0 until the first wait, then the index + 1.
static EVENT_SLOT: AtomicU32 = AtomicU32::new(0);

fn event_slot() -> Option<u32> {
    let s = EVENT_SLOT.load(Ordering::Acquire);
    if s != 0 {
        return Some(s - 1);
    }
    // SAFETY: plain call.
    let index = unsafe { sys::TlsAlloc() };
    if index == sys::TLS_OUT_OF_INDEXES {
        return None;
    }
    match EVENT_SLOT.compare_exchange(0, index + 1, Ordering::AcqRel, Ordering::Acquire) {
        Ok(_) => Some(index),
        // Another thread got there first; this index stays allocated (once).
        Err(s) => Some(s - 1),
    }
}

/// This thread's auto-reset event, made the first time.
fn thread_event() -> Option<HANDLE> {
    let slot = event_slot()?;
    // SAFETY: a valid TLS index; the value is this thread's event or null.
    unsafe {
        let e = sys::TlsGetValue(slot);
        if !e.is_null() {
            return Some(e);
        }
        let e = sys::CreateEventW(ptr::null_mut(), 0, 0, ptr::null());
        if e.is_null() {
            return None;
        }
        sys::TlsSetValue(slot, e);
        Some(e)
    }
}

/// Close the event of a thread that ends (`DLL_THREAD_DETACH`).
unsafe extern "system" fn close_thread_event(_module: *mut c_void, reason: u32, _reserved: *mut c_void) {
    const DLL_THREAD_DETACH: u32 = 3;
    let s = EVENT_SLOT.load(Ordering::Acquire);
    if reason == DLL_THREAD_DETACH && s != 0 {
        // SAFETY: the slot holds this thread's event or null; nothing waits on it now.
        unsafe {
            let e = sys::TlsGetValue(s - 1);
            if !e.is_null() {
                sys::TlsSetValue(s - 1, ptr::null_mut());
                sys::CloseHandle(e);
            }
        }
    }
}

#[link_section = ".CRT$XLW"]
#[used]
static CLOSE_THREAD_EVENT: unsafe extern "system" fn(*mut c_void, u32, *mut c_void) = close_thread_event;

/// Whether the `size` bytes at `address` equal those at `compare`.
unsafe fn same(address: *const c_void, compare: *const c_void, size: usize) -> bool {
    // SAFETY: the caller's pointers are valid for size bytes (1, 2, 4 or 8).
    unsafe {
        match size {
            1 => ptr::read_volatile(address as *const u8) == *(compare as *const u8),
            2 => ptr::read_volatile(address as *const u16) == *(compare as *const u16),
            4 => ptr::read_volatile(address as *const u32) == *(compare as *const u32),
            _ => ptr::read_volatile(address as *const u64) == *(compare as *const u64),
        }
    }
}

pub(crate) unsafe extern "system" fn wait_on_address(
    address: *const c_void,
    compare: *const c_void,
    size: usize,
    ms: u32,
) -> sys::BOOL {
    if !matches!(size, 1 | 2 | 4 | 8) {
        // SAFETY: plain call.
        unsafe { sys::SetLastError(sys::ERROR_INVALID_PARAMETER) };
        return 0;
    }
    let Some(event) = thread_event() else {
        // No event: wait a little instead, which the callers take as a spurious wake-up.
        // SAFETY: plain call.
        unsafe { sys::Sleep(if ms == 0 { 0 } else { 1 }) };
        return 1;
    };
    let b = bucket(address as usize);
    let mut w = Waiter { address: address as usize, event, woken: false, next: ptr::null_mut() };
    let wp: *mut Waiter = &mut w;
    {
        let mut g = b.lock();
        // SAFETY: the caller's pointers; compared with the lock held, see above.
        if !unsafe { same(address, compare, size) } {
            return 1;
        }
        let head = g.head();
        // SAFETY: wp is our record; it stays on our stack until it is off the list.
        unsafe { (*wp).next = *head };
        *head = wp;
    }

    // SAFETY: our own event.
    let r = unsafe { sys::WaitForSingleObject(event, ms) };

    let mut g = b.lock();
    // SAFETY: wp is ours; wakers only touch it with the lock held.
    if unsafe { (*wp).woken } {
        if r != sys::WAIT_OBJECT_0 {
            // Woken after the wait timed out: the event is set; clear it for the next wait.
            // SAFETY: our own event.
            unsafe { sys::WaitForSingleObject(event, 0) };
        }
        1
    } else {
        g.unlink(wp);
        drop(g);
        if r == sys::WAIT_OBJECT_0 {
            // The event was set but we were not woken: cannot happen (only wakes set it).
            1
        } else {
            // SAFETY: plain call.
            unsafe { sys::SetLastError(sys::ERROR_TIMEOUT) };
            0
        }
    }
}

/// Wake the threads waiting on `address`: one, or all.
fn wake(address: *const c_void, all: bool) {
    let mut g = bucket(address as usize).lock();
    let mut link: *mut *mut Waiter = g.head();
    // SAFETY: the records on the list are live (see Guard::unlink); each is taken off
    // before its event is set, and the waiter reads `woken` only with the lock held.
    unsafe {
        while !(*link).is_null() {
            let w = *link;
            if (*w).address == address as usize {
                *link = (*w).next;
                (*w).woken = true;
                sys::SetEvent((*w).event);
                if !all {
                    break;
                }
            } else {
                link = &mut (*w).next;
            }
        }
    }
}

unsafe extern "system" fn wake_by_address_single(address: *const c_void) {
    wake(address, false)
}

pub(crate) unsafe extern "system" fn wake_by_address_all(address: *const c_void) {
    wake(address, true)
}
