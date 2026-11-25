# ntsdk spike: does Visual C++ 4.1 accept mrustc's C?

Throwaway tools for the feasibility spike in `PLAN-mips-alpha-ppc.md`. Nothing here is built
by the normal build.

- `probes/*.c`: one C feature per file, for the questions mrustc's output raises.
- `probe.sh`: compiles each probe with `ntsdk nt40 CPU cl -c` for MIPS, Alpha and PowerPC
  and prints a table (needs `ntsdk` on `PATH`, as in the `tenox7/ntsdk` image).
- `c89scan.py`: counts the non-C89 constructs in a generated C file.
- `mrustc-c.sh TARGET`: mrustc's C for core, alloc, nt4rt and `hello/main.rs` (hello with a
  `start` lang item, since mrustc has no `#![no_main]`); `targets/` has the target files.
- `compile-c.sh CPU FILE.c...`: that C through `cl -c`, diagnostics counted by code.
- `c89filter.pl`: a crude text rewrite of the C toward Visual C++ 4.1, to see past the
  known errors. Not a design for the real C89 mode.
- `hello.c`: step 1, built and run with `ntrun` on each CPU.

The workflow `.github/workflows/ntsdk-spike.yml` runs all of it on GitHub's runners.
