# Rust for Windows NT 4.0

A Rust runtime, and later a `std` port, for Windows NT 4.0 SP0. The first target is
[PINT](https://github.com/tenox7/PINT), NT 4.0 on 32-bit ARM (the Raspberry Pi 2, 3 and 4);
x86 NT 4.0 comes next. Built with Rust 1.90, the version mrustc can bootstrap, so the same
patches can later reach Alpha and PowerPC through mrustc.

Status: layers 0 to 3, for PINT only. `std` programs run on PINT: threads and locks,
thread-locals, files and directories, processes with pipes, TCP and name resolution, time,
float maths (`examples/stdtest`). `no_std` console and GUI programs run as well. Panics
abort (no unwinding yet).

## Layers

| Layer | What | Where | Tied to the Rust version |
|---|---|---|---|
| 0 | Toolchain pin, target spec, import libraries of NT 4.0 SP0 | `rust-toolchain.toml`, `targets/`, `imports/`, `tools/` | only the spec's format |
| 1 | Runtime for `no_std` programs: entry point, console, heap, compiler helpers | `crates/nt4rt` | no |
| 2 | Compatibility library: the Win32 functions `std` needs that NT 4.0 lacks, the C run-time's startup | `crates/nt4compat` | no |
| 3 | `std` patch | `std/1.90.0/nt4.patch` | yes |
| 4 | Unwinding | (to do) | partly |
| 5 | Crate patches and tests | `tests/` | partly |

## Build

Needs only the pinned Rust (rustup installs it from `rust-toolchain.toml`) and Python 3:
no MSVC or PINT SDK. `rust-lld` links, and makes the import libraries from `imports/arm/*.def`.

    tools/cargo-nt4 build --release -p hello
    tools/peinfo.py target/thumbv7a-nt4-windows-msvc/release/hello.exe --defs imports/arm

`tools/cargo-nt4` builds std for the target from a patched copy of the toolchain's
`rust-src` (`tools/mkstd.sh`: `std/<version>/nt4.patch` applied, `crates/nt4compat` added),
with `-Zbuild-std=std,panic_abort` and `RUSTC_BOOTSTRAP=1` (a JSON target and `-Z` flags on
a stable release). `tools/peinfo.py` shows the headers NT 4.0 checks
and fails when an import is not exported by NT 4.0 SP0.

Host tests of the runtime's pure code:

    cargo test -p nt4rt --target x86_64-unknown-linux-gnu

## The target

`targets/thumbv7a-nt4-windows-msvc.json` is Rust's `thumbv7a-pc-windows-msvc` with:

- vendor `nt4`, the `cfg` the `std` patches will select on;
- the image marked for NT 4.0 (`/OSVERSION:4.0 /SUBSYSTEM:CONSOLE,4.0`): NT 4.0 does not
  start an image marked for a later Windows;
- no implicit TLS (`has-thread-local: false`);
- VFPv4, which every PINT processor has (Cortex-A7, A53, A72), for `fma`;
- `/NODEFAULTLIB`: nothing of the MSVC run-time is linked.

The linker is `tools/nt4-link` (set in `.cargo/config.toml`): the toolchain's rust-lld, with
`,4.0` added to any `/SUBSYSTEM` that has no version. For a program with
`#![windows_subsystem = "windows"]` rustc passes `/SUBSYSTEM:windows` after the target's
arguments, and lld would mark the image 6.0. With the wrapper, GUI programs need nothing of
their own: `examples/gui` is one.

## Import libraries

`imports/arm/*.def` list the exports of NT 4.0 SP0 for ARM, made by `tools/gen-defs.py` from
the DLLs of an NT 4.0 CD of another architecture (there is no ARM one), adjusted as the NT 4.0
source does for ARM: ntdll without the other architecture's names and with PINT's ARM exports
(`__rt_udiv`, `__chkstk`...), kernel32 without its architecture block. Each list is then
cut to what PINT's own DLLs export (`--pint-dlls`, the DLLs of a PINT image's
`WINNT\SYSTEM32`): PINT's advapi32 has no `Crypt*` functions and its oleaut32 about half of
NT 4.0's. Only names are taken.

The DLLs: kernel32, ntdll, advapi32, userenv, ws2_32, wsock32, and for GUI programs user32,
gdi32, comctl32 (4.70: no functions or messages of Internet Explorer's later versions),
comdlg32, shell32, ole32, oleaut32, opengl32, imm32, version, winmm.

## std on NT 4.0

**Layer 2, `crates/nt4compat`**, is a dependency of std on the `nt4` targets (the patch adds
it), so every std program links it. It gives:

- `mainCRTStartup`, calling the `main` rustc generates, and the TLS directory
  (`_tls_used`): NT 4.0's loader then calls std's TLS callback, which runs thread-local
  destructors when a thread ends. There is no C run-time.
- The Win32 functions std imports that NT 4.0 lacks, made from what it has. They are given
  as the `__imp_` pointers an import library would give: NT 4.0's import libraries do not
  have these names, so the linker takes ours, for std and for any crate that imports them.
  - `WaitOnAddress`, `WakeByAddressSingle`/`All`: events and a hashed wait table. std's
    Mutex, RwLock, Condvar, Once and thread parking all rest on these.
  - `InitOnceBeginInitialize`/`Complete`, on the above.
  - `SetFilePointerEx`, `GetFileSizeEx`; `Get`/`SetFileInformationByHandle(Ex)` through
    `NtQueryInformationFile`, `NtSetInformationFile` and `NtQueryDirectoryFile`;
    `GetFinalPathNameByHandleW` through `NtQueryObject` and the drive letters;
    `CreateHardLinkW` through `FileLinkInformation`.
  - `getaddrinfo`/`freeaddrinfo` on `gethostbyname` (IPv4).
  - `RtlGenRandom`: SplitMix64 seeded from clocks and ids, **not** a cryptographic source
    (NT 4.0 has none to offer: PINT's advapi32 has no `CryptGenRandom`).
  - `GetProcessId`, `CompareStringOrdinal`; `AddVectoredExceptionHandler` as the
    unhandled-exception filter.
  - Failing as std expects when a feature is missing: `CreateSymbolicLinkW`,
    `CreateWaitableTimerExW`, the process attribute list functions.
    `SetThreadStackGuarantee` succeeds without doing anything.
- The compiler's ARM helpers (shared with nt4rt).

**Layer 3, `std/1.90.0/nt4.patch`** (about 90 lines), makes vendor `nt4` take:

- std's `win7` paths for console output, the home directory, random numbers and the precise
  time;
- the modern `WaitOnAddress` path for synchronization, linked through an import library
  instead of `raw-dylib`;
- `FindExInfoStandard` for directory listing (`FindExInfoBasic` is Windows 7);
- named pipes made with `CreateNamedPipeW` for process I/O (NT 4.0 has no unnamed pipes
  through `NtCreateNamedPipeFile`);
- compiler_builtins' libm for the maths functions (NT 4.0's C run-time has no C99 maths);
- advapi32, linked as for `win7`.

Edit it as files: `tools/stdpatch.sh open`, change `target/stdwork/b/library`, then
`tools/stdpatch.sh save`.

## Testing on PINT

    tools/cargo-nt4 build --release -p hello -p reboot
    QEMU=.../pint40-qemu/bin/linux-x86_64/qemu-system-arm ARCFW=.../pint40-qemu/arcfw.elf \
        tests/pint/run-exe.sh base.img target/thumbv7a-nt4-windows-msvc/release/hello.exe

`tests/pint/run-exe.sh` boots a copy of a PINT SD image in QEMU raspi2b, runs the programs
from the All Users Startup folder, restarts NT with `examples/reboot` (`NtShutdownSystem`),
and prints the programs' output and exit codes, and `NAME.LOG` for a GUI program, which has
no console. `ARGS_<NAME>='...'` gives a program its command line. `OUTDIR=dir` copies the files the programs left (`examples/gui` saves its window
as `GUI.BMP`). About 2 minutes per run. PINT's restart sometimes hangs; the harness then
warns and reads the results anyway.
`tests/pint/qemu.sh` boots an image without a window; `tests/pint/mon.py` talks to its
monitor (`sendkey`, `screendump`).

The base image is a PINT SD image past NT's first run (three boots), made once:

1. Boot a fresh `nt-arcfw-sd.img` with `tests/pint/qemu.sh` **with networking** (the
   default): without PINT's USB Ethernet adapter the first run fails its network steps and
   NT does not log on by itself afterwards. A QEMU without user-mode networking will not
   do; PINT's portable package (`pint40-qemu-*.tar.gz`) has one (`QEMU=`), and its
   `arcfw.elf` (`ARCFW=`).
2. First boot: "Windows NT will now restart" (`mon.py WORKDIR sendkey ret`, or wait 30 s).
3. Second boot: Enter for each box (Detected Display, the Service Control Manager box) and
   for the console's pause; NT restarts.
4. Third boot: NT logs on as Administrator. Close Welcome, Start > Shut Down, then stop QEMU
   once it is safe to turn off. That disk is the base image.

## Rules

- Only functions NT 4.0 SP0 exports; `tools/peinfo.py --defs` checks every image.
- No Microsoft code in this repository. The NT source is a reference only.
