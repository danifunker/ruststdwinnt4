//! The compatibility layer of std on Windows NT 4.0 SP0 (layer 2).
//!
//! std links this crate on the `nt4` targets (its `std/1.90.0/nt4.patch` adds the
//! dependency). It gives a program what NT 4.0 and the absent C run-time do not:
//!
//! - [`crt`]: the entry point `mainCRTStartup`, which calls the `main` rustc generates, and
//!   the TLS directory (`_tls_used`), through which NT 4.0's loader calls std's TLS
//!   callback when a thread ends;
//! - the helpers the compiler calls that the MSVC C run-time defines (`helpers`, `half`);
//! - the Win32 functions std (and other crates) import that NT 4.0 does not export,
//!   implemented with what it does ([`sync`], [`random`], [`file`], [`net`], [`misc`]).
//!
//! The functions are given the way an import library would give them: as the `__imp_`
//! pointer a call through an import goes through. NT 4.0's import libraries
//! (`imports/<arch>/*.def`) do not have these names, so the linker takes ours, for std and
//! for every other crate that imports them.
#![no_std]
#![allow(non_snake_case, clippy::missing_safety_doc)]

#[macro_use]
mod imp;

mod sys;

pub mod crt;
pub mod file;
pub mod misc;
pub mod net;
pub mod random;
mod sha256;
pub mod sync;

pub mod half;
#[cfg(target_arch = "arm")]
mod helpers;
