//! Standard output and standard error through `WriteFile`.

use core::fmt;

use crate::sys;

/// A standard handle as a `core::fmt::Write` sink.
pub struct StdHandle(sys::DWORD);

/// The process's standard output.
pub fn stdout() -> StdHandle {
    StdHandle(sys::STD_OUTPUT_HANDLE)
}

/// The process's standard error.
pub fn stderr() -> StdHandle {
    StdHandle(sys::STD_ERROR_HANDLE)
}

impl StdHandle {
    /// Write all of `bytes`; false when the handle is missing or a write fails.
    pub fn write_bytes(&mut self, mut bytes: &[u8]) -> bool {
        // SAFETY: GetStdHandle takes any value and returns a handle, null or
        // INVALID_HANDLE_VALUE; WriteFile only reads `bytes` and writes `written`.
        unsafe {
            let h = sys::GetStdHandle(self.0);
            if h.is_null() || h == sys::INVALID_HANDLE_VALUE {
                return false;
            }
            while !bytes.is_empty() {
                let chunk = bytes.len().min(u32::MAX as usize) as sys::DWORD;
                let mut written: sys::DWORD = 0;
                if sys::WriteFile(h, bytes.as_ptr(), chunk, &mut written, core::ptr::null_mut()) == 0
                    || written == 0
                {
                    return false;
                }
                bytes = &bytes[written as usize..];
            }
        }
        true
    }
}

impl fmt::Write for StdHandle {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if self.write_bytes(s.as_bytes()) { Ok(()) } else { Err(fmt::Error) }
    }
}

#[doc(hidden)]
pub fn _print(handle: StdHandle, args: fmt::Arguments) {
    let mut h = handle;
    let _ = fmt::Write::write_fmt(&mut h, args);
}

/// Print to standard output.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => { $crate::console::_print($crate::console::stdout(), format_args!($($arg)*)) };
}

/// Print a line to standard output (CR LF, as NT's console programs end lines).
#[macro_export]
macro_rules! println {
    () => { $crate::print!("\r\n") };
    ($($arg:tt)*) => {{
        $crate::console::_print($crate::console::stdout(), format_args!($($arg)*));
        $crate::print!("\r\n");
    }};
}

/// Print to standard error.
#[macro_export]
macro_rules! eprint {
    ($($arg:tt)*) => { $crate::console::_print($crate::console::stderr(), format_args!($($arg)*)) };
}

/// Print a line to standard error.
#[macro_export]
macro_rules! eprintln {
    () => { $crate::eprint!("\r\n") };
    ($($arg:tt)*) => {{
        $crate::console::_print($crate::console::stderr(), format_args!($($arg)*));
        $crate::eprint!("\r\n");
    }};
}
