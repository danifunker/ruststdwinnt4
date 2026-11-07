#!/usr/bin/env python3
"""Make NT 4.0 versions of the crates.io crates that need one, for a project's build.

    tools/nt4-crates.py PROJECT_DIR      # prints cargo --config arguments, one per line

For each crate of PROJECT_DIR/Cargo.lock listed in EDITS, the source is copied from
cargo's registry (fetched first if missing) to target/nt4-crates/<name>-<version> of this
repository and edited there; tools/cargo-nt4 passes the printed arguments to cargo, which
then takes these copies instead ([patch.crates-io], one entry per version). The project's
own files are not changed, except that cargo records the patched sources in its
Cargo.lock.

The edits:
  - windows-sys, windows: Microsoft's bindings define the structures that differ between
    32-bit and 64-bit Windows for x86 and the 64-bit architectures, not for 32-bit ARM,
    which Windows dropped. 32-bit ARM Windows has the 32-bit layouts (its headers test
    _WIN64, as for x86), so `target_arch = "x86"` becomes "x86 or arm".
  - parking_lot_core: it finds WaitOnAddress (Windows 8) or NT keyed events (Windows XP)
    at run time and panics without either. On the nt4 targets it imports WaitOnAddress,
    which nt4compat gives.
"""

import glob
import os
import re
import shutil
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "target", "nt4-crates")


def arch_32bit_arm(src):
    pat = re.compile(r'target_arch = "x86"')
    new = 'any(target_arch = "x86", target_arch = "arm")'
    for path in glob.glob(os.path.join(src, "src", "**", "*.rs"), recursive=True):
        with open(path) as f:
            s = f.read()
        t = pat.sub(new, s)
        if t != s:
            with open(path, "w") as f:
                f.write(t)


PARKING_LOT_NT4 = '''    #[allow(non_snake_case)]
    pub fn create() -> Option<WaitAddress> {
        // Windows NT 4.0 (ruststdwinnt4): WaitOnAddress comes from nt4compat, through
        // the usual import.
        #[cfg(target_vendor = "nt4")]
        {
            #[link(name = "kernel32")]
            extern "system" {
                fn WaitOnAddress(
                    address: *const ffi::c_void,
                    compare: *const ffi::c_void,
                    size: usize,
                    ms: u32,
                ) -> BOOL;
                fn WakeByAddressSingle(address: *const ffi::c_void);
            }
            return Some(WaitAddress { WaitOnAddress, WakeByAddressSingle });
        }
        #[allow(unreachable_code)]
'''


def parking_lot_core(src):
    path = os.path.join(src, "src", "thread_parker", "windows", "waitaddress.rs")
    with open(path) as f:
        s = f.read()
    old = "    #[allow(non_snake_case)]\n    pub fn create() -> Option<WaitAddress> {\n"
    if old not in s:
        sys.exit(f"nt4-crates: {path} is not as expected")
    with open(path, "w") as f:
        f.write(s.replace(old, PARKING_LOT_NT4, 1))


EDITS = {
    "windows-sys": arch_32bit_arm,
    "windows": arch_32bit_arm,
    "parking_lot_core": parking_lot_core,
}
# Bump to remake every copy when an edit changes.
EDITS_VERSION = "1"


def locked(project):
    lock = os.path.join(project, "Cargo.lock")
    if not os.path.exists(lock):
        subprocess.run(["cargo", "generate-lockfile"], cwd=project, check=True)
    with open(lock) as f:
        text = f.read()
    for block in text.split("[[package]]")[1:]:
        name = re.search(r'^name = "([^"]+)"', block, re.M)
        version = re.search(r'^version = "([^"]+)"', block, re.M)
        source = re.search(r'^source = "registry\+', block, re.M)
        if name and version and source and name.group(1) in EDITS:
            yield name.group(1), version.group(1)


def registry_source(project, name, version):
    home = os.environ.get("CARGO_HOME", os.path.expanduser("~/.cargo"))
    pattern = os.path.join(home, "registry", "src", "*", f"{name}-{version}")
    found = glob.glob(pattern)
    if not found:
        subprocess.run(["cargo", "fetch"], cwd=project, check=True, stdout=sys.stderr)
        found = glob.glob(pattern)
    if not found:
        sys.exit(f"nt4-crates: {name} {version} is not in cargo's registry")
    return found[0]


def main():
    project = os.path.abspath(sys.argv[1] if len(sys.argv) > 1 else ".")
    os.makedirs(OUT, exist_ok=True)
    for name, version in sorted(set(locked(project))):
        dst = os.path.join(OUT, f"{name}-{version}")
        stamp = os.path.join(dst, ".nt4-edits")
        if not (os.path.exists(stamp) and open(stamp).read() == EDITS_VERSION):
            shutil.rmtree(dst, ignore_errors=True)
            shutil.copytree(registry_source(project, name, version), dst)
            EDITS[name](dst)
            with open(stamp, "w") as f:
                f.write(EDITS_VERSION)
        key = f"{name}-{version}".replace(".", "-")
        print(f'patch.crates-io.{key}.path="{dst}"')
        print(f'patch.crates-io.{key}.package="{name}"')


if __name__ == "__main__":
    main()
