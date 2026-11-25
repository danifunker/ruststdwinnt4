#!/usr/bin/env python3
"""std-edits.py LIBRARY: what std (with std/1.90.0/nt4.patch) further needs for mrustc and
Alpha, applied to a copy of Rust's library/ (mkstd.sh's). Each edit is a plain text
replacement that must match exactly once; the script stops at the first that doesn't.

Kept apart from nt4.patch because rustc's builds (x86, ARM) must not get them."""
import pathlib
import sys

lib = pathlib.Path(sys.argv[1])

EDITS = [
    # mrustc can't compile a #![no_core] crate (it wants the lang items); windows_targets
    # only defines macros, so as a plain no_std crate it is the same.
    ("windows_targets/src/lib.rs", "#![no_std]\n#![no_core]\n", "#![no_std]\n"),
    ("windows_targets/src/lib.rs", "#![feature(no_core)]\n", ""),
    # Alpha: 32-bit Windows like ARM as far as std's bindings go (WSADATA's layout; CONTEXT,
    # which std doesn't look into there).
    ("std/src/sys/pal/windows/c/windows_sys.rs",
     '#[cfg(target_arch = "arm")]\n#[repr(C)]\npub struct WSADATA {',
     '#[cfg(any(target_arch = "arm", target_arch = "alpha"))]\n#[repr(C)]\npub struct WSADATA {'),
    ("std/src/sys/pal/windows/c/windows_sys.rs",
     '#[cfg(target_arch = "arm")]\npub enum CONTEXT {}',
     '#[cfg(any(target_arch = "arm", target_arch = "alpha"))]\npub enum CONTEXT {}'),
    # Alpha: no stack walker (backtraces print nothing), as on targets without one.
    ("backtrace/src/backtrace/mod.rs",
     """                mod win32;
                use self::win32::trace as trace_imp;
                pub(crate) use self::win32::Frame as FrameImp;
            }
""",
     """                mod win32;
                use self::win32::trace as trace_imp;
                pub(crate) use self::win32::Frame as FrameImp;
            } else {
                mod noop;
                use self::noop::trace as trace_imp;
                pub(crate) use self::noop::Frame as FrameImp;
            }
"""),
]

for rel, old, new in EDITS:
    p = lib / rel
    s = p.read_text()
    if s.count(old) != 1:
        if (new and s.count(new) == 1) or (not new and s.count(old) == 0):
            continue  # already applied
        sys.exit(f"std-edits: {rel}: {s.count(old)} matches for {old[:60]!r}")
    p.write_text(s.replace(old, new))
