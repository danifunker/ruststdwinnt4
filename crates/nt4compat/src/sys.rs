//! The NT 4.0 functions this crate is built on: all exported by NT 4.0 SP0.
#![allow(dead_code)]

use core::ffi::c_void;

pub type HANDLE = *mut c_void;
pub type BOOL = i32;

pub const INFINITE: u32 = 0xFFFF_FFFF;
pub const WAIT_OBJECT_0: u32 = 0;
pub const WAIT_TIMEOUT: u32 = 0x102;
pub const TLS_OUT_OF_INDEXES: u32 = 0xFFFF_FFFF;
pub const ERROR_TIMEOUT: u32 = 1460;
pub const ERROR_INVALID_PARAMETER: u32 = 87;
pub const ERROR_CALL_NOT_IMPLEMENTED: u32 = 120;
pub const ERROR_NOT_SUPPORTED: u32 = 50;
pub const ERROR_INSUFFICIENT_BUFFER: u32 = 122;
pub const INVALID_HANDLE_VALUE: HANDLE = -1isize as HANDLE;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct FILETIME {
    pub low: u32,
    pub high: u32,
}

#[link(name = "kernel32")]
extern "system" {
    pub fn ExitProcess(code: u32) -> !;
    pub fn GetLastError() -> u32;
    pub fn SetLastError(code: u32);
    pub fn CloseHandle(h: HANDLE) -> BOOL;
    pub fn CreateEventW(sa: *mut c_void, manual: BOOL, initial: BOOL, name: *const u16) -> HANDLE;
    pub fn SetEvent(h: HANDLE) -> BOOL;
    pub fn WaitForSingleObject(h: HANDLE, ms: u32) -> u32;
    pub fn Sleep(ms: u32);
    pub fn SwitchToThread() -> BOOL;
    pub fn TlsAlloc() -> u32;
    pub fn TlsGetValue(index: u32) -> *mut c_void;
    pub fn TlsSetValue(index: u32, value: *mut c_void) -> BOOL;
    pub fn GetCurrentProcessId() -> u32;
    pub fn GetCurrentThreadId() -> u32;
    pub fn GetTickCount() -> u32;
    pub fn QueryPerformanceCounter(count: *mut i64) -> BOOL;
    pub fn GetSystemTimeAsFileTime(t: *mut FILETIME);
}

pub type NTSTATUS = i32;
pub const STATUS_NO_MORE_FILES: NTSTATUS = 0x8000_0006u32 as i32;
pub const STATUS_BUFFER_OVERFLOW: NTSTATUS = 0x8000_0005u32 as i32;
pub const ERROR_NO_MORE_FILES: u32 = 18;
pub const ERROR_PATH_NOT_FOUND: u32 = 3;

pub fn nt_success(s: NTSTATUS) -> bool {
    s >= 0
}

#[repr(C)]
pub struct IO_STATUS_BLOCK {
    pub status: usize,
    pub information: usize,
}

impl IO_STATUS_BLOCK {
    pub const fn new() -> Self {
        IO_STATUS_BLOCK { status: 0, information: 0 }
    }
}

#[repr(C)]
pub struct UNICODE_STRING {
    pub length: u16,
    pub maximum_length: u16,
    pub buffer: *mut u16,
}

#[link(name = "kernel32")]
extern "system" {
    pub fn GetProcessHeap() -> HANDLE;
    pub fn HeapAlloc(heap: HANDLE, flags: u32, size: usize) -> *mut c_void;
    pub fn HeapFree(heap: HANDLE, flags: u32, p: *mut c_void) -> BOOL;
    pub fn SetFilePointer(h: HANDLE, low: i32, high: *mut i32, method: u32) -> u32;
    pub fn GetFileSize(h: HANDLE, high: *mut u32) -> u32;
    pub fn CreateFileW(
        name: *const u16, access: u32, share: u32, sa: *mut c_void, disposition: u32,
        flags: u32, template: HANDLE,
    ) -> HANDLE;
    pub fn QueryDosDeviceW(device: *const u16, target: *mut u16, max: u32) -> u32;
    pub fn SetUnhandledExceptionFilter(
        filter: Option<unsafe extern "system" fn(*mut c_void) -> i32>,
    ) -> Option<unsafe extern "system" fn(*mut c_void) -> i32>;
}

#[link(name = "ntdll")]
extern "system" {
    pub fn NtQueryInformationFile(
        h: HANDLE, io: *mut IO_STATUS_BLOCK, info: *mut c_void, length: u32, class: u32,
    ) -> NTSTATUS;
    pub fn NtSetInformationFile(
        h: HANDLE, io: *mut IO_STATUS_BLOCK, info: *const c_void, length: u32, class: u32,
    ) -> NTSTATUS;
    pub fn NtQueryDirectoryFile(
        h: HANDLE, event: HANDLE, apc: *mut c_void, apc_context: *mut c_void,
        io: *mut IO_STATUS_BLOCK, info: *mut c_void, length: u32, class: u32,
        single: u8, name: *const UNICODE_STRING, restart: u8,
    ) -> NTSTATUS;
    pub fn NtQueryObject(h: HANDLE, class: u32, info: *mut c_void, length: u32, ret: *mut u32) -> NTSTATUS;
    pub fn NtQueryInformationProcess(
        h: HANDLE, class: u32, info: *mut c_void, length: u32, ret: *mut u32,
    ) -> NTSTATUS;
    pub fn RtlNtStatusToDosError(status: NTSTATUS) -> u32;
    pub fn RtlDosPathNameToNtPathName_U(
        dos: *const u16, nt: *mut UNICODE_STRING, file_part: *mut *mut u16, relative: *mut c_void,
    ) -> u8;
    pub fn RtlUpcaseUnicodeChar(c: u16) -> u16;
}

/// Set the thread's last error from an NTSTATUS; FALSE.
pub fn fail_status(s: NTSTATUS) -> BOOL {
    // SAFETY: plain calls.
    unsafe { SetLastError(RtlNtStatusToDosError(s)) };
    0
}

/// Set the thread's last error; FALSE.
pub fn fail(code: u32) -> BOOL {
    // SAFETY: plain call.
    unsafe { SetLastError(code) };
    0
}
