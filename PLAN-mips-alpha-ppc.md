# Plan: MIPS, Alpha and PowerPC

Status: **Alpha first, in progress** (branch `alpha`; mrustc's fork, branch `alpha-nt4`).
PINT (ARM) and x86 are done. Rust runs natively on Windows NT 4.0 for Alpha: `hello`
through mrustc's new C89 mode and Visual C++ 4.1 runs under `ntrun` (see "Alpha,
2026-11-25" below). MIPS and PowerPC follow on the same path.

- 2026-11-08: plan written for Alpha and PowerPC.
- 2026-11-25: `tenox7/ntsdk` appeared and changes the toolchain side: Microsoft's own C
  compilers for all three, run on Linux or macOS with no VM. MIPS added.
- 2026-11-25: FX!32 dropped: every CPU gets a native build. That makes mrustc the main
  path, since it is the only one that reaches Alpha and PowerPC.
- 2026-11-25: spike: ntsdk's `hello.c` passes on MIPS, Alpha and PowerPC; mrustc's C for
  core, alloc, nt4rt and hello measured against Visual C++ 4.1; the C89 mode is about
  3 to 5 weeks.
- 2026-11-25: Alpha first (the user's priority), the minimum it needs rather than the full
  C89 mode: hello runs on Alpha the same day; std and stdtest translate (see below).

## Shape

1. **mrustc to C89, compiled by ntsdk** is the path for all three. Its work (a C89 mode
   in mrustc, a build driver) is shared; each CPU then adds a target definition, import
   lists, start-up code and helpers.
2. **The MIPS shortcut** (rustc's `mipsel` objects converted to COFF) is optional. It is
   worth taking only if it is cheap and proves out before the mrustc work is done, or if
   mrustc's output is too slow or too big for Visual C++ 4.1 on MIPS.
3. **Alpha first**, then MIPS and PowerPC. Alpha's 32-bit pointers make it a 32-bit target
   for the C, which is the same as MIPS's byte for byte; the CPU-specific part is small
   (import libraries, the C run-time, a target file). PowerPC needs its arch name kept away
   from core's AltiVec module.

## Why not the ARM and x86 way

rustc cannot produce code for any of the three:

- **Alpha**: LLVM dropped its Alpha backend in 2011.
- **PowerPC**: LLVM's 32-bit PowerPC backend writes ELF (and AIX's XCOFF), not NT's
  little-endian PE/COFF, and does not use NT's calling convention.
- **MIPS**: LLVM has little-endian 32-bit MIPS (`mipsel`), and NT's MIPS calling
  convention is close to o32, but LLVM writes only ELF for MIPS, not COFF. Unlike the
  other two, there is a possible native-rustc route (the MIPS shortcut), still unproven.

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
    (`DEC-Alpha/NT4_2GB.img`) on the NAS; nothing set up yet. Needed only for GUI
    programs: `ntrun` runs Alpha console programs.
  - MIPS: none yet (MAME's MIPS Magnum/Jazz is a candidate).
- The native compilers on the NAS (`DEC-Alpha/NT4 Apps/msvc60pro_en-us_alpha.iso`,
  `IBMPowerPC/Windows PPC/msvc-4.1-ppc.iso`, copied to
  `/run/media/dani/BigDisk/nt4work/compilers`) are no longer needed for building; ntsdk
  replaces them. Visual C++ 6 for Alpha is newer than ntsdk's 4.1, if 4.1 proves too weak.
- mrustc (`~/repos/mrustc`, the user's fork) translates Rust 1.90 to C and has been used
  for PowerPC Mac and SPARC Solaris.

## The main path: mrustc with C89 output (all three)

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

## Spike, 2026-11-25

Tools and inputs are in `tools/ntsdk-spike/` (target files, a hello mrustc can take, the
scripts below). mrustc: the user's fork at its `master` (eb5df108), branch `ntsdk-spike`
with no changes. Rust 1.90's `library/` from rustup's `rust-src`.

### 1. ntsdk itself: passes on all three

The workflow `.github/workflows/ntsdk-spike.yml` (this branch) pulls `tenox7/ntsdk` at run
time on GitHub's Ubuntu runners: no Docker on m900 (its data is on the full root disk)
and nothing of Microsoft's in the repository. `hello.c` (with an `__int64` and `%I64x`)
builds with `ntsdk nt40 CPU cl` and runs with `ntrun` on **MIPS, Alpha and PowerPC**:
`sizeof(void*)` and `sizeof(long)` are 4 on all three. Each compile takes well under a
second.

The probes (`probe.sh`, one feature per file, `cl -c -W3`), the same on all three CPUs:

| Rejected | Accepted |
|---|---|
| `inline` (`__inline` works) | `unsigned __int64` to and from `double` and `float` |
| `long long`, `ULL`/`LL` suffixes (`ui64`/`i64` work) | `__int64` `*`, `/`, `%`, shifts; `switch` on an `__int64` |
| `__alignof` | `$` in names |
| `__declspec(align(N))`, `__declspec(noreturn)`, `__declspec(selectany)` (C2485) | `__declspec(thread)` |
| designated initializers | zero-length arrays, in structs and in unions |
| compound literals | `#pragma pack(push, N)` / `pop` |
| hex float literals | `#pragma comment(linker, "/alternatename:...")` (compiles; the linker is untested) |
| declarations after statements | non-constant aggregate initializers, struct return by value, `setjmp` |
| | kernel32's `Interlocked*` from the NT 4.0 SDK; a 70 KB static array |

`fabsf` is missing from Alpha's and PowerPC's headers (MIPS has it).

### 2. mrustc's C, measured

`mrustc-c.sh TARGET` translates `core`, `alloc`, `nt4rt` and hello. Target files for
32-bit little-endian NT on each CPU, in `msvc` and `gnu` flavours
(`targets/*-nt4-*.toml`: pointers 32 bits, `u64`/`f64` aligned to 8, no 64-bit atomics).

- It works as it is: no mrustc change was needed to get C for all three CPUs.
- **mrustc has no `#![no_main]`**: a binary needs the `start` lang item.
  `hello/main.rs` is `examples/hello` with one, and with `__nt4rt_main` written out
  instead of `nt4rt::entry!`. A real port needs `no_main` in mrustc, or nt4rt providing
  `start` for it.
- **Alpha and MIPS get the same C, byte for byte**: Alpha NT's 32-bit pointers make it a
  32-bit target as far as the C is concerned. PowerPC's `core` is 1.9 MB bigger: with
  `target_arch = "powerpc"`, core's AltiVec module comes in, which NT's PowerPCs (601, 603,
  604) don't have. The target needs another arch name, or that module cut off.
- Sizes (Msvc flavour): `core` 30.2 MB of C in 520,000 lines (the longest line 58 KB, a
  float table), `alloc` 5.0 MB, `nt4rt` 0.15 MB, hello 0.38 MB. Names stay short (at most
  143 characters; longer ones are cut with a `$` and a hash), and string literals at most
  287 bytes.

**What the C uses that Visual C++ 4.1 lacks.** Counted in all four files, Msvc flavour
(17 MB without comments), checked by sample and against clang's `-std=c89 -pedantic`;
the last column is updated with the probes' results:

| Construct | Count | Where it comes from | For Visual C++ 4.1 |
|---|---|---|---|
| `ull`/`ll` literals, `uint64_t` as `long long` | 21,400 | constants, `case` labels | `ui64`/`i64` suffix, `__int64` |
| `bool`/`_Bool`, `stdint.h`, `stdbool.h`, `intrin.h`, `immintrin.h` | header | prelude | typedefs in the prelude |
| `inline` | 122 | prelude helpers | `__inline` |
| `__declspec(selectany)` | 362 | type ids, generic statics | not in 4.1: link with `-force:multiple`, or define them once |
| `#pragma comment(linker, "/alternatename:...")` | 36 | weak symbols (`__rust_alloc` and others) | link 3.10 likely lacks it: the driver writes the shims |
| `__declspec(align(N))` | 686 | `repr(align)`, zero-size arrays | not in 4.1. 607 are `align(1)` (drop). Over 8: 65 types (16 to 256), from core's cache-padded and SIMD types and hello's own `align(64)`: they need padding plus run-time alignment |
| `#pragma pack(push, N)` | 126 | `repr(packed)` and layout | works |
| `__alignof` | 2,200 | layout checks (`typedef char x[cond ? 1 : -1]`) | drop them: mrustc computed the layout |
| designated initializers `{ .raw = ... }` | 995 | every static (a union of the value and its bytes) | put `raw` first in the union, initialize it by position |
| `InterlockedXxx8/16/64`, `_Interlocked*` | ~900 | atomics | x86 only. NT 4.0's kernel32 has 32-bit `InterlockedExchange`, `...CompareExchange` and `...ExchangeAdd`; 8- and 16-bit ones become loops on the 32-bit word |
| `_byteswap_*`, `_BitScan*`, `__popcnt` | 150 | prelude | plain C |
| `fabsf`, `copysignf`, `copysign`, `INFINITY`, `NAN` | 27 | floats | our own helpers |
| zero-length arrays `raw[0]` | 290 | zero-size statics and types | work |
| `__declspec(noreturn)` | 4 | prelude | drop |
| mixed declarations, `for (int i...)` | 30 | prelude helpers | rewrite those helpers |
| `for (unsigned int i = 0; ...)` | 165 | `[x; N]` in generated code | declare `i` in a block around the loop |
| a `static` after statements | 5 | `mrustc_empty_caller_location` (`#[track_caller]`) | a block around it |
| repeated `typedef` (C11) | 36 | function-pointer types | emit once |
| `$` in names | 6,100 | long names cut with a hash | works |

The generated function bodies are nearly C89: locals are declared at the top, no
compound literals, no VLAs; the two exceptions are the `[x; N]` loops and the
caller-location static in the table. `__int128` is a struct of two 64-bit halves (the Msvc
flavour does this for 32-bit targets). Of the plan's table, `_Alignof`, designated
initializers, `static inline`, `__int128` and `__builtin_*` are confirmed (the
`__builtin_*` names are the prelude's own functions in the Msvc flavour); thread-locals
don't appear in `no_std` code.

**mrustc's C through `cl`** (the workflow's `compile` job, per CPU, `cl -c -W1` with
`inc/` for `stdint.h` and friends):

- As generated: every file stops at `cl`'s limit of 100 errors within its first 170 lines,
  in mrustc's prelude (`inline`, `__declspec`), before any generated code.
- After `c89filter.pl`, a crude text rewrite for the spike only (`__inline`, `ui64`/`i64`
  literals, no `__declspec` or `__alignof` checks, statics' unions with `raw` first and
  initialized by position, blocks around the `[x; N]` loops and the caller-location
  static): on all three CPUs, hello and `nt4rt` fail only in the prelude (44 errors: its
  `for (int i ...)` and declarations after statements, and what follows from them).
  `alloc` adds one: `SliceArcInnerForStatic` is `repr(align(16))` and comes out 12 bytes
  instead of 16, the alignment problem above. `core` gets to line 131,000 of 520,000
  before the 100-error limit; past the prelude its errors are calls to the 128-bit
  helpers (`shr128`...) that the broken prelude failed to define. PowerPC's `core` also
  fails in the AltiVec code (`core_arch::simd`), which shouldn't be there anyway.
- No size limit hit: `cl` reads the 30 MB `core` (lines up to 58 KB) and stops after 100
  errors in 6 to 11 s. Whether it can compile all of it, and in how much memory, is open
  until the prelude is fixed.
- No difference between the CPUs in what `cl` accepts, apart from `fabsf` and AltiVec.

Still open: the linker (`alternatename`, `-force:multiple`, link 3.10 on a large object),
whether `cl` gets through all of `core` and how fast its code runs, and Alpha's and
PowerPC's code generation (only MIPS's `__int64` helpers were linked, by `hello.c`).

### 3. Estimate for the C89 mode

All of it belongs in the generator (`src/trans/codegen_c.cpp`, 8,600 lines), not a
post-pass: the literals, unions, alignment and atomics need the types, and each comes
from one or a few places (the 64-bit literals from 8 lines; static initializers from 2;
alignment from 2; the prelude is one block of fixed text). A post-pass would only do the
easy part (`inline`, headers) and still need the hard part in the generator.

| Piece | Days |
|---|---|
| A third `Compiler` (`Msvc4`, from a target file's `backend.c.variant`), the prelude rewritten in C89 (types, 128-bit and overflow helpers, bit operations, byte swaps) | 2 to 3 |
| 64-bit literals and `case` labels; statics without designators; no `__alignof`, `noreturn`, repeated typedefs | 1 to 2 |
| Atomics on kernel32's 32-bit Interlocked functions, 8- and 16-bit as loops | 1 to 2 |
| Alignment over 8: padding and run-time alignment for statics and locals (the `high_align` handling already passes such values by pointer) | 2 to 4 |
| `selectany` and `alternatename` replacements, with the driver | 1 to 2 |
| `[x; N]` loops and the caller-location static in the generated code | 0.5 |
| Whatever `cl` still finds: the rest of `core`, compiler limits (splitting `core`'s 30 MB), code generation bugs, the linker | 3 to 10 |
| `#![no_main]` (or `start` in nt4rt), and a target file per CPU | 1 |

About **3 to 5 weeks** for hello on all three CPUs; the probes removed the worst risk
(64-bit integer to floating-point conversions work), so the low end is likelier. Then the driver: mrustc's own
`minicargo` already builds a cargo package and can stop before the C compiler, so the
driver is minicargo plus an `ntsdk nt40 CPU cl`/`link` step, import libraries and start-up
code: **1 to 2 weeks**, then `std` with `std/1.90.0/nt4.patch`. Upstream mrustc doesn't
take AI-written code (its CONTRIBUTING.md), so all of this stays in the user's fork.

### 4. The MIPS shortcut: not tried

### Also found

- ntsdk's `axp64` (Platform SDK for Windows 2000 RC2) has `cl` 13.00 for Alpha. Without
  `-Ap64` it makes 32-bit code, but that SDK has no 32-bit libraries. A possible newer
  Alpha compiler if 4.1 proves too weak; Visual C++ 6 for Alpha on the NAS is the other.

## Alpha, 2026-11-25

Branch `alpha` here, `alpha-nt4` in mrustc's fork (based on its `master`, eb5df108). Until
the fork's branch is pushed, CI applies `tools/mrustc-nt4/mrustc.patch` (that branch's
diff) to the fork's `master`. The workflow (`.github/workflows/ntsdk-spike.yml`) has two
jobs: `alpha-hello` and `alpha-stdtest`.

### What it took in mrustc (the C89 mode)

`backend.c.c89 = true` in a target file (with `variant = "msvc"`): C for Visual C++ 4.x.
Not the full mode the spike estimated (3 to 5 weeks), the parts Alpha needed:

- The prelude: `__inline` (by a define), the C89 doubles maths and the C99 ones on top
  (`__mrustc_trunc`, `__mrustc_sqrtf`...), plain-C popcount/ctz/clz/byte swaps, `INFINITY`
  and `NAN`, `va_copy`; no `intrin.h`, `windows.h` or C library headers: extern functions are
  declared under their own names (no `/alternatename`), which the headers' prototypes would
  clash with.
- `i64`/`ui64` literals; statics initialized by position (`raw` first in their union).
- No `__alignof`, `__declspec(align|noreturn|selectany)`. Alignment over 8 is capped at 8:
  explicit padding keeps the sizes mrustc computed (`SliceArcInnerForStatic` is 16), and a
  zero-length `uint64_t` array gives such a struct the C alignment the padding assumes.
  Weak statics (vtables) are `static` in each object file; typeids are communal.
- Atomics on NT 4.0's only one, kernel32's 32-bit `InterlockedCompareExchange`:
  compare-exchange loops, 8 and 16 bits on the aligned word, 64 bits under one lock.
- Float-to-integer `as` saturates through helpers: Alpha's conversion traps out of range
  (`hello` stopped with c0000090 on `1e19 as u64` before).
- No passing of 8-aligned arguments by pointer (an MSVC x86 workaround that broke the C ABI
  of `extern "C"` functions taking `f64`).
- `#[link_section]` as `#pragma data_seg`; `[x; N]` and drop loops, the caller-location
  static and `panic_impl` made C89.
- `[target] vendor-name` (for `target_vendor = "nt4"`), `#[export_name]`.
- Fixes for every target: `&raw const STATIC` lost its pointer type in const propagation;
  `c"..."` wasn't a literal in `macro_rules`; the saturating minimum of `i64`/`i128` was
  2^59; float `%` used `remainder()` (Rust's truncates: `fmod`).

Checked locally with clang (`-std=c89 -fms-extensions --target=i686-pc-windows-msvc` and
stub NT 4.0 headers; `-DMRUSTC_C89_ALIGNCHECK` turns on alignment checks the output
carries for that), then with the real compiler in CI.

### Here

- `crates/nt4rt/src/mrustc.rs` (mrustc only): the `start` lang item (mrustc has no
  `#![no_main]`), `__rdl_alloc` and the rest on `NtHeap` (mrustc's allocator shims ignore
  `#[global_allocator]`), `cbrt` and `fdim` (core calls them; the C library lacks them).
  Under mrustc nt4rt has no `mainCRTStartup`: Microsoft's C run-time (LIBC.LIB, linked by
  default) starts the program and calls the C `main` mrustc writes.
- nt4compat under mrustc: each replacement function also under its Win32 name (mrustc's C
  calls imports by name, rustc through `__imp_`), no `crt` module (the C run-time's start-up
  and TLS directory instead).
- `tools/mrustc-nt4/`: `targets/alpha-nt4.toml` (32-bit pointers, `target_arch = "alpha"`,
  which has no module in core or std, so no AltiVec-like surprises), `include/` (`stdint.h`,
  `stdbool.h`), `translate.sh` and `build.sh` (hello), `mkstd.sh`, `translate-std.sh` and
  `build-std.sh` (std and stdtest), `std-edits.py` (what std needs for mrustc and Alpha beyond
  `nt4.patch`, kept out of it because rustc's builds mustn't get them), `wrap/mrustc`.

### Results

- **hello**: RESULT_HELLO
- **Visual C++ 4.1 on core**: `cl` compiles all of core (30 MB of C, 520,000 lines) in
  134 to 161 s with a peak of about 360 MB (the container's); the object is 3.7 MB. alloc
  (4.9 MB) takes 13 to 21 s. No compiler limit hit. link 3.10 links the 3.7 MB object and
  the rest in about a second; hello.exe is 2.3 MB (no optimization yet, `-Od`).
- **std and stdtest**: RESULT_STD

## The MIPS shortcut: rustc with COFF objects (to investigate)

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

- **FX!32 on Alpha** (running the x86 build under DEC's translator): the user wants
  native builds only (2026-11-25).
- **GCC**: on Alpha it only has 64-bit pointers (NT uses 32-bit ones); on PowerPC its
  little-endian ABI is System V, not NT's (NT support ended with GCC 2.x); both would need
  ELF objects converted to COFF.
- **New LLVM backends**: PowerPC (little-endian, PE/COFF, NT's import glue, on top of
  LLVM's AIX work) is weeks to months; Alpha has nothing to start from (LLVM 2.9's backend
  dates from 2011). Either would also need rustc 1.90 built against that LLVM.
- **Another Rust-to-C translator**: none is as far along as mrustc.
