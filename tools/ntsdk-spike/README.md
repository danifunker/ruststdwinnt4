# ntsdk spike: does Visual C++ 4.1 accept mrustc's C?

Throwaway tools for the feasibility spike in `PLAN-mips-alpha-ppc.md`. Nothing here is built
by the normal build.

- `probes/*.c`: one C feature per file, for the questions mrustc's output raises.
- `probe.sh`: compiles each probe with `ntsdk nt40 CPU cl -c` for MIPS, Alpha and PowerPC
  and prints a table (needs `ntsdk` on `PATH`, as in the `tenox7/ntsdk` image).
- `c89scan.py`: counts the non-C89 constructs in a generated C file.
