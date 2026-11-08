# Plan: Alpha and PowerPC (parked)

Status: **parked** (2026-11-08). PINT (ARM) and x86 are done; these two are next when the
work resumes. Nothing here has been built or tried yet.

## Why not the ARM and x86 way

rustc cannot produce code for either platform:

- **Alpha**: LLVM dropped its Alpha backend in 2011.
- **PowerPC**: LLVM's 32-bit PowerPC backend writes ELF (and AIX's XCOFF), not NT's
  little-endian PE/COFF, and does not use NT's calling convention.

## What is at hand

- Compilers, both on the NAS share `software` and copied to
  `/run/media/dani/BigDisk/nt4work/compilers`. Both are native programs of their machine
  (Alpha or PowerPC PE images), so compiles run inside emulated NT:
  - Alpha: Visual C++ 6 (`DEC-Alpha/NT4 Apps/msvc60pro_en-us_alpha.iso`).
  - PowerPC: Visual C++ 4.1 (`IBMPowerPC/Windows PPC/msvc-4.1-ppc.iso`).
- PowerPC test machine: NT 4.0 for PowerPC on QEMU's IBM 40P,
  `/run/media/dani/BigDisk/winnt-ppc` (Visual C++ 4 installed; Administrator's password
  is `password`). The rustdesk repository's `crustdesk-winnt` builds C there already:
  `scripts/ntx.py` and `tools/nt/ntexec.c` send files and run `nmake` in the VM, and its
  staging rewrites sources to C89.
- Alpha: the ES40 emulator (`DEC-Alpha/NT4 Apps/es40.zip`) and an NT 4.0 disk image
  (`DEC-Alpha/NT4_2GB.img`) on the NAS; nothing set up here yet.
- mrustc (`~/repos/mrustc`, the user's fork) translates Rust 1.90 to C and has been used
  for PowerPC Mac and SPARC Solaris.

## Path 1: FX!32 on Alpha (cheapest; try first)

DEC's FX!32 runs x86 Win32 programs on NT 4.0 for Alpha, translating x86 code and
calling Alpha NT's own DLLs for Win32 functions. The x86 target's programs might run
unchanged. Not native, and there is nothing like it for PowerPC.

What may break, to check on a first run (`hello`, `stdtest`, `cmix`, `tlstest`):

- NTDLL's x86-only exports: x86 programs import `_alldiv`, `_aulldiv`, `_allrem`,
  `_aullrem`, `_allmul`, the shifts and `_chkstk` from NTDLL, which Alpha's NTDLL does not
  have. If FX!32 does not supply them, nt4compat can carry its own (an x86 build option).
- nt4compat's direct NTDLL calls (`NtQueryInformationFile`, `NtQueryDirectoryFile`,
  `NtSetInformationFile`, `RtlDosPathNameToNtPathName_U`...): FX!32 may only map the
  documented Win32 functions.
- `cmpxchg8b` (Rust's 64-bit atomics on i686) and x87 precision (FX!32 computes in 64-bit,
  not 80-bit).
- The static TLS directory and its callbacks.

Steps: set up ES40 with NT 4.0 for Alpha and FX!32 (find FX!32 in the collection), add an
Alpha variant of `tests/x86/run-exe.sh`, run the x86 programs, fix what breaks.

## Path 2: mrustc with C89 output (native; both platforms)

mrustc needs no exceptions: it implements panics with `setjmp`/`longjmp`, and these
targets abort on panic anyway. What Visual C++ 4 and 6 lack is C99. Counted in a GCC-mode
output of mrustc (`output-1.74.0/dump.c`, 950 KB):

| In mrustc's C | Count | For Visual C++ 4/6 |
|---|---|---|
| `stdint.h`, `stdbool.h` | 1 each | our `crt/include` |
| `static inline` | 88 | `__inline` |
| `_Alignof` | 183 | computed constants (VC++ 6 likely has no `__alignof`) |
| designated initializers | 51 | positional initializers |
| `__int128` | 102 | a struct (mrustc's MSVC mode does this) |
| `__builtin_*` | 37 | functions of our own |
| thread-locals | 4 | `__declspec(thread)` |

Steps:

1. A C89 output mode in mrustc's C generator (`src/trans/codegen_c.cpp`), for old
   Microsoft C: it knows the types and sizes, so it can write C89 directly rather than a
   rewrite of its output afterwards.
2. mrustc target definitions for NT 4.0 on PowerPC (little-endian, 32-bit) and Alpha
   (32-bit pointers in 64-bit registers).
3. `hello` (nt4rt, `no_std`) through mrustc, compiled and linked by Visual C++ 4 in the
   PowerPC VM over `ntx.py`; then by Visual C++ 6 on Alpha.
4. Per platform: NT 4.0 export lists (`tools/gen-defs.py` from each system's DLLs), the
   compiler's helpers, start-up code; then std (the same `std/1.90.0/nt4.patch`),
   `stdtest`, and a test harness like `tests/x86/run-exe.sh`.

Risks: Visual C++ 4 on multi-megabyte generated files (limits, bugs, 64-bit arithmetic),
and build times inside emulated machines.

## Ruled out

- **GCC**: on Alpha it only has 64-bit pointers (NT uses 32-bit ones); on PowerPC its
  little-endian ABI is System V, not NT's (NT support ended with GCC 2.x); both would need
  ELF objects converted to COFF.
- **New LLVM backends**: PowerPC (little-endian, PE/COFF, NT's import glue, on top of
  LLVM's AIX work) is weeks to months; Alpha has nothing to start from (LLVM 2.9's backend
  dates from 2011). Either would also need rustc 1.90 built against that LLVM.
- **Another Rust-to-C translator**: none is as far along as mrustc.
