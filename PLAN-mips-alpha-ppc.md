# Plan: MIPS, Alpha and PowerPC (parked)

Status: **parked**. PINT (ARM) and x86 are done. Nothing here has been built or tried yet.

- 2026-11-08: plan written for Alpha and PowerPC.
- 2026-11-25: `tenox7/ntsdk` appeared and changes the toolchain side: Microsoft's own C
  compilers for all three, run on Linux or macOS with no VM. MIPS added.

## Why not the ARM and x86 way

rustc cannot produce code for any of the three:

- **Alpha**: LLVM dropped its Alpha backend in 2011.
- **PowerPC**: LLVM's 32-bit PowerPC backend writes ELF (and AIX's XCOFF), not NT's
  little-endian PE/COFF, and does not use NT's calling convention.
- **MIPS**: LLVM has little-endian 32-bit MIPS (`mipsel`), and NT's MIPS calling
  convention is close to o32, but LLVM writes only ELF for MIPS, not COFF. Unlike the
  other two, there is a possible native-rustc route (Path 3), still unproven.

## What is at hand

- **`tenox7/ntsdk`** (Docker Hub, source at github.com/tenox7/ntsdk; `linux/amd64` and
  `linux/arm64`, about 670 MB):
  - The Win32 SDK for NT 4.0 plus Visual C++ 4.1 (`cl` 10.10, `link` 3.10, `nmake`) for
    MIPS, Alpha and PowerPC (`ntsdk nt40 mips|alpha|ppc ...`). Also the NT 3.1/3.5 SDKs
    and an x86 Visual C++ 4.2.
  - The tools run under tenox's `ntrun`: no Wine, no VM. Builds are byte-identical to
    real NT's apart from time stamps.
  - **`ntrun hello.exe` runs 32-bit console programs** for x86, MIPS, Alpha and PowerPC.
    GUI programs build but don't run. This is a test harness for `hello` and `stdtest`
    with no emulated machine.
  - Like PINT's SDK, it is used at build time only: none of it goes into this repository.
- Emulated NT machines, for GUI programs and a final check:
  - PowerPC: NT 4.0 on QEMU's IBM 40P, `/run/media/dani/BigDisk/winnt-ppc` on m900
    (Administrator's password is `password`). The rustdesk repository's `crustdesk-winnt`
    has `scripts/ntx.py` and `tools/nt/ntexec.c` for sending files and running programs.
  - Alpha: the ES40 emulator (`DEC-Alpha/NT4 Apps/es40.zip`) and an NT 4.0 disk image
    (`DEC-Alpha/NT4_2GB.img`) on the NAS; nothing set up yet.
  - MIPS: none yet (MAME's MIPS Magnum/Jazz is a candidate).
- The native compilers on the NAS (`DEC-Alpha/NT4 Apps/msvc60pro_en-us_alpha.iso`,
  `IBMPowerPC/Windows PPC/msvc-4.1-ppc.iso`, copied to
  `/run/media/dani/BigDisk/nt4work/compilers`) are no longer needed for building; ntsdk
  replaces them. Visual C++ 6 for Alpha is newer than ntsdk's 4.1, if 4.1 proves too weak.
- mrustc (`~/repos/mrustc`, the user's fork) translates Rust 1.90 to C and has been used
  for PowerPC Mac and SPARC Solaris.

## Path 1: FX!32 on Alpha (cheapest; try first for Alpha)

DEC's FX!32 runs x86 Win32 programs on NT 4.0 for Alpha, translating x86 code and
calling Alpha NT's own DLLs for Win32 functions. The x86 target's programs (and
vintrdp-nt4-x86.exe) might run unchanged. Not native, and there is nothing like it for
MIPS or PowerPC. Needs the ES40 machine: ntrun runs Alpha programs, not FX!32.

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

## Path 2: mrustc with C89 output (native; all three)

mrustc needs no exceptions: it implements panics with `setjmp`/`longjmp`, and these
targets abort on panic anyway. What Visual C++ 4.1 lacks is C99. Counted in a GCC-mode
output of mrustc (`output-1.74.0/dump.c`, 950 KB):

| In mrustc's C | Count | For Visual C++ 4.1 |
|---|---|---|
| `stdint.h`, `stdbool.h` | 1 each | our `crt/include` |
| `static inline` | 88 | `__inline` |
| `_Alignof` | 183 | computed constants (no `__alignof`) |
| designated initializers | 51 | positional initializers |
| `__int128` | 102 | a struct (mrustc's MSVC mode does this) |
| `__builtin_*` | 37 | functions of our own |
| thread-locals | 4 | `__declspec(thread)` |

Steps:

1. A C89 output mode in mrustc's C generator (`src/trans/codegen_c.cpp`), for old
   Microsoft C: it knows the types and sizes, so it can write C89 directly rather than a
   rewrite of its output afterwards.
2. mrustc target definitions for NT 4.0 on MIPS and PowerPC (little-endian, 32-bit) and
   Alpha (32-bit pointers in 64-bit registers).
3. `hello` (nt4rt, `no_std`) through mrustc, built with `ntsdk nt40 <cpu> cl` and run
   with `ntrun`. All three CPUs at once, since only the compiler word changes.
4. Per platform: NT 4.0 export lists (`tools/gen-defs.py` from each system's DLLs; the
   SDK's import libraries name them too), the compiler's helpers, start-up code; then std
   (the same `std/1.90.0/nt4.patch`) and `stdtest` under `ntrun`.
5. A `cargo-nt4`-like driver: cargo's build plan through mrustc, then ntsdk's `cl` and
   `link`. This is the biggest piece.

Risks: Visual C++ 4.1 on multi-megabyte generated files (limits, bugs, 64-bit
arithmetic); mrustc's own coverage of the crates vintrdp needs.

## Path 3: rustc for MIPS, with COFF objects (native; MIPS only; to investigate)

rustc's `mipsel` code generation is mature. If its ELF objects can become NT COFF
objects, MIPS could work like x86 and ARM: rustc, our import libraries, a linker.

To find out:

- Whether NT's MIPS calling convention matches o32 closely enough (argument registers,
  stack home area, `gp` use, which NT doesn't have).
- An ELF-to-COFF object converter: ntsdk's `mip2coff` converts objects from the MIPS
  compiler of NT 3.1's SDK; check whether it takes LLVM's output. Otherwise, write one.
- The linker: ntsdk's MIPS `link`, or lld if it learns MIPS COFF (it doesn't today).

## Then, in vintrdp

Each new CPU is a matrix entry in vintrdp's `build.yml` and an `--arch` in
`scripts/dist/build-nt4.sh`, once this repository builds `std` programs for it. TLS would
come later: GnuTLS needs C99, so it would have to go through ntsdk's Visual C++ 4.1 with
the C99 layer, or ship without TLS first.

## Ruled out

- **GCC**: on Alpha it only has 64-bit pointers (NT uses 32-bit ones); on PowerPC its
  little-endian ABI is System V, not NT's (NT support ended with GCC 2.x); both would need
  ELF objects converted to COFF.
- **New LLVM backends**: PowerPC (little-endian, PE/COFF, NT's import glue, on top of
  LLVM's AIX work) is weeks to months; Alpha has nothing to start from (LLVM 2.9's backend
  dates from 2011). Either would also need rustc 1.90 built against that LLVM.
- **Another Rust-to-C translator**: none is as far along as mrustc.
