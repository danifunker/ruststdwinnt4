//! The entry point and the panic handler.

use core::fmt::Write;

use crate::{console, sys};

/// The exit code of the process for the value `main` returns.
pub trait Termination {
    fn report(self) -> u32;
}

impl Termination for () {
    fn report(self) -> u32 {
        0
    }
}

impl Termination for u32 {
    fn report(self) -> u32 {
        self
    }
}

impl Termination for i32 {
    fn report(self) -> u32 {
        self as u32
    }
}

impl<E: core::fmt::Debug> Termination for Result<(), E> {
    fn report(self) -> u32 {
        match self {
            Ok(()) => 0,
            Err(e) => {
                let _ = writeln!(console::stderr(), "Error: {e:?}\r");
                1
            }
        }
    }
}

extern "Rust" {
    /// Defined by [`entry!`](crate::entry) in the program.
    fn __nt4rt_main() -> u32;
}

/// The entry point the linker gives a console program.
#[no_mangle]
pub unsafe extern "C" fn mainCRTStartup() -> ! {
    // SAFETY: the program defines __nt4rt_main with entry!; ExitProcess does not return.
    unsafe {
        let code = __nt4rt_main();
        sys::ExitProcess(code)
    }
}

/// Make `$main` the program's main function: `nt4rt::entry!(main);`. It returns `()`, `u32`,
/// `i32` or `Result<(), E: Debug>`, which gives the exit code.
#[macro_export]
macro_rules! entry {
    ($main:path) => {
        #[no_mangle]
        pub fn __nt4rt_main() -> u32 {
            $crate::Termination::report($main())
        }
    };
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    let mut err = console::stderr();
    match info.location() {
        Some(l) => {
            let _ = write!(err, "panicked at {}:{}:{}:\r\n", l.file(), l.line(), l.column());
        }
        None => {
            let _ = write!(err, "panicked:\r\n");
        }
    }
    let _ = write!(err, "{}\r\n", info.message());
    // SAFETY: ExitProcess takes any code and does not return.
    unsafe { sys::ExitProcess(101) }
}
