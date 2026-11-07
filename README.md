# Rust for Windows NT 4.0

A Rust runtime, and later a `std` port, for Windows NT 4.0 SP0. The first target is
[PINT](https://github.com/tenox7/PINT), NT 4.0 on 32-bit ARM (the Raspberry Pi 2, 3 and 4);
x86 NT 4.0 comes next. Built with Rust 1.90, the version mrustc can bootstrap, so the same
patches can later reach Alpha and PowerPC through mrustc.

Status: layers 0 and 1, for PINT only. `std` does not build for the target yet.

## Layers

| Layer | What | Where | Tied to the Rust version |
|---|---|---|---|
| 0 | Toolchain pin, target spec, import libraries of NT 4.0 SP0 | `rust-toolchain.toml`, `targets/`, `imports/`, `tools/` | only the spec's format |
| 1 | Runtime for `no_std` programs: entry point, console, heap, compiler helpers | `crates/nt4rt` | no |
| 2 | Compatibility library: the Win32 functions `std` needs that NT 4.0 lacks | (to do) | no |
| 3 | `std` patches | (to do) | yes |
| 4 | Unwinding | (to do) | partly |
| 5 | Crate patches and tests | `tests/` | partly |

## Build

Needs only the pinned Rust (rustup installs it from `rust-toolchain.toml`) and Python 3:
no MSVC or PINT SDK. `rust-lld` links, and makes the import libraries from `imports/arm/*.def`.

    tools/cargo-nt4 build --release -p hello
    tools/peinfo.py target/thumbv7a-nt4-windows-msvc/release/hello.exe --defs imports/arm

`tools/cargo-nt4` runs cargo with `-Zbuild-std=core,alloc` and `RUSTC_BOOTSTRAP=1` (a JSON
target and `-Z` flags on a stable release). `tools/peinfo.py` shows the headers NT 4.0 checks
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

A GUI program (`windows_subsystem = "windows"`) is not handled yet: rustc's own
`/SUBSYSTEM:WINDOWS` would drop the 4.0 version.

## Import libraries

`imports/arm/*.def` list the exports of NT 4.0 SP0 for ARM, made by `tools/gen-defs.py` from
the DLLs of an NT 4.0 CD of another architecture (there is no ARM one), adjusted as the NT 4.0
source does for ARM: ntdll without the other architecture's names and with PINT's ARM exports
(`__rt_udiv`, `__chkstk`...), kernel32 without its architecture block. Only names are taken.

## Testing on PINT

    tools/cargo-nt4 build --release -p hello -p reboot
    QEMU=.../pint40-qemu/bin/linux-x86_64/qemu-system-arm ARCFW=.../pint40-qemu/arcfw.elf \
        tests/pint/run-exe.sh base.img target/thumbv7a-nt4-windows-msvc/release/hello.exe

`tests/pint/run-exe.sh` boots a copy of a PINT SD image in QEMU raspi2b, runs the programs
from the All Users Startup folder, restarts NT with `examples/reboot` (`NtShutdownSystem`),
and prints the programs' output and exit codes. About 2 minutes per run.
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
