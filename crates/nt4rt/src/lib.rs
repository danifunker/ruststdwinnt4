//! The runtime of a `no_std` program on Windows NT 4.0 SP0: what the Microsoft C run-time
//! would otherwise give it.
//!
//! - [`entry!`]: the entry point, `mainCRTStartup`, which calls the program's `main` and
//!   exits the process with its code;
//! - [`print!`] and [`println!`] (standard output), [`eprint!`] and [`eprintln!`]
//!   (standard error);
//! - [`heap::NtHeap`]: a global allocator on the process heap;
//! - the panic handler: the message on standard error, then exit code 101;
//! - the helpers the compiler calls on ARM that the MSVC C run-time defines (`__u64tod`...)
//!   and `_fltused`.
//!
//! Only functions NT 4.0 SP0 exports are used.
#![cfg_attr(not(test), no_std)]

pub mod console;
// The compiler's helpers are shared with nt4compat, the compatibility layer std links on
// this target: the same source, compiled into whichever of the two a program uses.
#[path = "../../nt4compat/src/half.rs"]
pub mod half;
pub mod heap;
pub mod sys;

#[cfg(target_arch = "arm")]
#[path = "../../nt4compat/src/helpers.rs"]
mod helpers;
#[cfg(all(feature = "rt", not(test)))]
mod start;

#[cfg(all(feature = "rt", not(test)))]
pub use start::Termination;

/// The program's exit code for the value `main` returns.
#[cfg(any(not(feature = "rt"), test))]
pub trait Termination {
    fn report(self) -> u32;
}
