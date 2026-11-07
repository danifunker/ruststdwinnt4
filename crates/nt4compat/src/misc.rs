//! Other functions of later Windows that std imports, on NT 4.0.

use core::ffi::c_void;
use core::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};

use crate::sys::{self, BOOL, HANDLE};

provide! {
    InitOnceBeginInitialize = init_once_begin_initialize, 16;
    InitOnceComplete = init_once_complete, 12;
    GetProcessId = get_process_id, 4;
    CompareStringOrdinal = compare_string_ordinal, 20;
    AddVectoredExceptionHandler = add_vectored_exception_handler, 8;
    SetThreadStackGuarantee = set_thread_stack_guarantee, 4;
    CreateWaitableTimerExW = create_waitable_timer_ex_w, 16;
    InitializeProcThreadAttributeList = initialize_proc_thread_attribute_list, 16;
    UpdateProcThreadAttribute = update_proc_thread_attribute, 28;
    DeleteProcThreadAttributeList = delete_proc_thread_attribute_list, 4;
}

// ---- One-time initialization (Windows Vista) ----
//
// INIT_ONCE is one pointer-sized word: 0 not started, 1 being initialized, otherwise the
// context of a completed initialization with bit 1 set (its low two bits are reserved,
// as in Windows). A thread that finds it being initialized waits for the word to change
// with WaitOnAddress, which this crate also provides.

const INIT_ONCE_CHECK_ONLY: u32 = 1;
const INIT_ONCE_ASYNC: u32 = 2;
const INIT_ONCE_INIT_FAILED: u32 = 4;
const BUSY: usize = 1;
const DONE: usize = 2;

use crate::sync::{wait_on_address as WaitOnAddress, wake_by_address_all as WakeByAddressAll};

unsafe extern "system" fn init_once_begin_initialize(
    once: *mut c_void,
    flags: u32,
    pending: *mut BOOL,
    context: *mut *mut c_void,
) -> BOOL {
    if flags & INIT_ONCE_ASYNC != 0 {
        return sys::fail(sys::ERROR_NOT_SUPPORTED);
    }
    // SAFETY: once is the caller's INIT_ONCE, a word used only through these functions.
    let word = unsafe { &*(once as *const AtomicUsize) };
    loop {
        let v = word.load(Ordering::Acquire);
        if v & DONE != 0 {
            // SAFETY: the caller's out parameters.
            unsafe {
                *pending = 0;
                if !context.is_null() {
                    *context = (v & !3) as *mut c_void;
                }
            }
            return 1;
        }
        if flags & INIT_ONCE_CHECK_ONLY != 0 {
            return sys::fail(sys::ERROR_INVALID_PARAMETER);
        }
        if v == 0 {
            if word.compare_exchange(0, BUSY, Ordering::Acquire, Ordering::Acquire).is_ok() {
                // SAFETY: the caller's out parameter.
                unsafe { *pending = 1 };
                return 1;
            }
            continue;
        }
        // Being initialized by another thread: wait until the word changes.
        // SAFETY: the word is valid; BUSY is the value we compare with.
        unsafe { WaitOnAddress(once, (&BUSY as *const usize).cast(), core::mem::size_of::<usize>(), sys::INFINITE) };
    }
}

unsafe extern "system" fn init_once_complete(once: *mut c_void, flags: u32, context: *mut c_void) -> BOOL {
    // SAFETY: as above.
    let word = unsafe { &*(once as *const AtomicUsize) };
    let v = if flags & INIT_ONCE_INIT_FAILED != 0 { 0 } else { (context as usize & !3) | DONE };
    if word.compare_exchange(BUSY, v, Ordering::Release, Ordering::Relaxed).is_err() {
        return sys::fail(sys::ERROR_INVALID_PARAMETER);
    }
    // SAFETY: the caller's word.
    unsafe { WakeByAddressAll(once) };
    1
}

// ---- Processes ----

/// Windows XP SP1: from the process's basic information.
unsafe extern "system" fn get_process_id(process: HANDLE) -> u32 {
    // PROCESS_BASIC_INFORMATION: ExitStatus, PebBaseAddress, AffinityMask, BasePriority,
    // UniqueProcessId, InheritedFromUniqueProcessId.
    let mut info = [0usize; 6];
    // SAFETY: info is ours.
    let s = unsafe {
        sys::NtQueryInformationProcess(process, 0, info.as_mut_ptr().cast(), core::mem::size_of_val(&info) as u32, core::ptr::null_mut())
    };
    if sys::nt_success(s) {
        info[4] as u32
    } else {
        sys::fail_status(s);
        0
    }
}

const ERROR_NOT_SUPPORTED: u32 = 50;

unsafe extern "system" fn initialize_proc_thread_attribute_list(
    _list: *mut c_void,
    _count: u32,
    _flags: u32,
    _size: *mut usize,
) -> BOOL {
    // Windows Vista; std uses an attribute list only when a program asks for one.
    sys::fail(ERROR_NOT_SUPPORTED)
}

unsafe extern "system" fn update_proc_thread_attribute(
    _list: *mut c_void,
    _flags: u32,
    _attribute: usize,
    _value: *const c_void,
    _size: usize,
    _previous: *mut c_void,
    _return_size: *mut usize,
) -> BOOL {
    sys::fail(ERROR_NOT_SUPPORTED)
}

unsafe extern "system" fn delete_proc_thread_attribute_list(_list: *mut c_void) {}

// ---- Strings ----

const CSTR_LESS_THAN: i32 = 1;
const CSTR_EQUAL: i32 = 2;
const CSTR_GREATER_THAN: i32 = 3;

/// Windows Vista: compare UTF-16 strings by code unit, optionally ignoring case (by the
/// system's upper-case table). A length of -1 means NUL-terminated.
unsafe extern "system" fn compare_string_ordinal(
    s1: *const u16,
    n1: i32,
    s2: *const u16,
    n2: i32,
    ignore_case: BOOL,
) -> i32 {
    // SAFETY: the caller's strings, of the lengths given or NUL-terminated.
    unsafe {
        let len = |s: *const u16, n: i32| -> usize {
            if n >= 0 {
                n as usize
            } else {
                let mut i = 0;
                while *s.add(i) != 0 {
                    i += 1;
                }
                i
            }
        };
        let (a, b) = (
            core::slice::from_raw_parts(s1, len(s1, n1)),
            core::slice::from_raw_parts(s2, len(s2, n2)),
        );
        for (&x, &y) in a.iter().zip(b) {
            let (x, y) = if ignore_case != 0 {
                (sys::RtlUpcaseUnicodeChar(x), sys::RtlUpcaseUnicodeChar(y))
            } else {
                (x, y)
            };
            if x != y {
                return if x < y { CSTR_LESS_THAN } else { CSTR_GREATER_THAN };
            }
        }
        match a.len().cmp(&b.len()) {
            core::cmp::Ordering::Less => CSTR_LESS_THAN,
            core::cmp::Ordering::Equal => CSTR_EQUAL,
            core::cmp::Ordering::Greater => CSTR_GREATER_THAN,
        }
    }
}

// ---- Exceptions and stacks ----

type Handler = unsafe extern "system" fn(*mut c_void) -> i32;

static HANDLER: AtomicPtr<c_void> = AtomicPtr::new(core::ptr::null_mut());
static PREVIOUS: AtomicPtr<c_void> = AtomicPtr::new(core::ptr::null_mut());

unsafe extern "system" fn unhandled(pointers: *mut c_void) -> i32 {
    const EXCEPTION_CONTINUE_SEARCH: i32 = 0;
    // SAFETY: the pointers were stored from handlers of this type.
    unsafe {
        let h = HANDLER.load(Ordering::Acquire);
        if !h.is_null() {
            let r = core::mem::transmute::<*mut c_void, Handler>(h)(pointers);
            if r != EXCEPTION_CONTINUE_SEARCH {
                return r;
            }
        }
        let p = PREVIOUS.load(Ordering::Acquire);
        if p.is_null() { EXCEPTION_CONTINUE_SEARCH } else { core::mem::transmute::<*mut c_void, Handler>(p)(pointers) }
    }
}

/// Windows XP. NT 4.0 has no vectored handlers; the handler std adds (which reports a
/// stack overflow) becomes the unhandled-exception filter, so it sees the exceptions
/// nothing else handled. One handler is kept.
unsafe extern "system" fn add_vectored_exception_handler(_first: u32, handler: Option<Handler>) -> *mut c_void {
    let Some(h) = handler else { return core::ptr::null_mut() };
    HANDLER.store(h as *mut c_void, Ordering::Release);
    // SAFETY: plain call.
    if let Some(prev) = unsafe { sys::SetUnhandledExceptionFilter(Some(unhandled)) } {
        if prev as usize != unhandled as usize {
            PREVIOUS.store(prev as *mut c_void, Ordering::Release);
        }
    }
    h as *mut c_void
}

/// Windows Server 2003: nothing to reserve on NT 4.0; std only needs it to succeed.
unsafe extern "system" fn set_thread_stack_guarantee(_size: *mut u32) -> BOOL {
    1
}

/// Windows Vista: std asks for a high-resolution timer and sleeps with `Sleep` without
/// one.
unsafe extern "system" fn create_waitable_timer_ex_w(
    _sa: *mut c_void,
    _name: *const u16,
    _flags: u32,
    _access: u32,
) -> HANDLE {
    sys::fail(ERROR_NOT_SUPPORTED);
    core::ptr::null_mut()
}
