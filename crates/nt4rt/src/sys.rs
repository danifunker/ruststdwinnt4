//! The functions of KERNEL32 and NTDLL the runtime calls. Each is exported by Windows NT
//! 4.0 SP0 (imports/arm/*.def).
#![allow(non_snake_case, clippy::upper_case_acronyms)]

use core::ffi::c_void;

pub type BOOL = i32;
pub type DWORD = u32;
pub type HANDLE = *mut c_void;

pub const STD_OUTPUT_HANDLE: DWORD = -11i32 as DWORD;
pub const STD_ERROR_HANDLE: DWORD = -12i32 as DWORD;
pub const INVALID_HANDLE_VALUE: HANDLE = -1isize as HANDLE;
pub const HEAP_ZERO_MEMORY: DWORD = 0x0000_0008;

#[cfg_attr(windows, link(name = "kernel32"))]
extern "system" {
    pub fn GetStdHandle(nStdHandle: DWORD) -> HANDLE;
    pub fn WriteFile(
        hFile: HANDLE,
        lpBuffer: *const u8,
        nNumberOfBytesToWrite: DWORD,
        lpNumberOfBytesWritten: *mut DWORD,
        lpOverlapped: *mut c_void,
    ) -> BOOL;
    pub fn ExitProcess(uExitCode: u32) -> !;
    pub fn GetProcessHeap() -> HANDLE;
    pub fn HeapAlloc(hHeap: HANDLE, dwFlags: DWORD, dwBytes: usize) -> *mut c_void;
    pub fn HeapReAlloc(hHeap: HANDLE, dwFlags: DWORD, lpMem: *mut c_void, dwBytes: usize)
        -> *mut c_void;
    pub fn HeapFree(hHeap: HANDLE, dwFlags: DWORD, lpMem: *mut c_void) -> BOOL;
    pub fn GetLastError() -> DWORD;
    pub fn GetVersion() -> DWORD;
}

// The compiler's own calls into NTDLL on ARM: __chkstk (stack probes of frames over a
// page) and the integer division helpers (__rt_udiv, __rt_sdiv64...). Naming the library
// is enough for the linker to resolve them.
#[cfg(all(windows, target_arch = "arm"))]
#[link(name = "ntdll")]
extern "C" {}
