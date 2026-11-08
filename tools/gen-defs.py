#!/usr/bin/env python3
"""Generate the module-definition (.def) files the import libraries are built from.

The exports are those of Windows NT 4.0 as released (SP0, build 1381): the names the
DLLs of the NT 4.0 CD export. No NT 4.0 CD for ARM exists, so the names come from
another architecture's CD and are adjusted the way the NT 4.0 source does it for that
architecture:

  - ntdll: the exports of that CD's ntdll, minus the names of its architecture's
    ntdll def file (alphadef.src for Alpha), plus PINT's ARM ntdll exports
    (PINT/kernel/dll/armdef.src).
  - kernel32: the exports of that CD's kernel32, minus the names of the
    architecture block of kernel32.src (forwarders to ntdll that ARM does not have).
  - the other DLLs: the CD's exports as they are; they have no architecture-specific
    exports.

With --pint-dlls, each list is then cut to the names PINT's own DLLs export: PINT does
not build everything of NT 4.0 (its advapi32 has no Crypt* functions, its oleaut32 about
half of the CD's). A name in these files is therefore in NT 4.0 SP0 and in PINT 4.0.
Only names are read from Microsoft's files; nothing else of them is copied.

For x86 (--arch x86) the DLLs are an x86 system's, from its CD or an installed
SYSTEM32, and their names are taken as they are (with winspool.drv, which PINT lacks). They are
written with their __stdcall argument bytes (CreateFileW@28), which x86 import libraries
need (tools/implib.py) and the DLLs do not record: --decorations names an x86 import
library that has them, such as the windows_i686_msvc crate's windows.0.53.0.lib. NTDLL's
system calls (Nt*, Zw*) are read from their stubs (ret BYTES) instead, documented or not;
other undocumented ones are in EXTRA_DECORATIONS. A name none of them has is written
plain, as a __cdecl function.

Usage:
  tools/gen-defs.py --cd DIR --nt4-src DIR --pint DIR [--arch arm] [--out imports/arm]
  tools/gen-defs.py --arch x86 --cd DIR --decorations LIB [--out imports/x86]

  --cd       a directory with the CD's DLLs expanded (ADVAPI32.DLL, KERNEL32.DLL...);
             expand the compressed ones (KERNEL32.DL_) with 7z or msexpand first.
             For x86, an installed system's SYSTEM32 will do too (its DLLs are expanded;
             imports/x86 come from an NT 4.0 SP1 install's).
  --cd-arch  the architecture of that CD (default alpha)
  --nt4-src  the NT 4.0 source tree (the directory holding private/ and public/)
  --pint     the PINT tree
  --pint-dlls  a directory with the DLLs of a PINT image's WINNT/SYSTEM32 (optional)
"""

import argparse
import os
import re
import struct
import sys

DLLS = [
    "kernel32", "ntdll", "advapi32", "userenv", "ws2_32", "wsock32",
    # GUI
    "user32", "gdi32", "comctl32", "comdlg32", "shell32", "ole32", "oleaut32",
    "opengl32", "imm32", "version", "winmm",
    # the C run-time of C code (PINT ships its own msvcrt.dll)
    "msvcrt",
]
# x86 only: PINT 4.0 has no print spooler. A name with its extension is not a .dll.
X86_DLLS = ["winspool.drv"]

# The architecture blocks of kernel32.src: the #if lines that open each one.
KERNEL32_ARCH_BLOCK = {
    "alpha": r"#elif defined\(ALPHA\)",
    "mips": r"#if defined\(MIPS\)",
    "ppc": r"#elif defined \(PPC\)",
}


def pe_exports(path):
    """Return the exported names of a PE image."""
    with open(path, "rb") as f:
        d = f.read()
    if d[:2] != b"MZ":
        sys.exit(f"{path}: not a PE image (expand compressed .DL_ files first)")
    pe = struct.unpack_from("<I", d, 0x3C)[0]
    nsec = struct.unpack_from("<H", d, pe + 6)[0]
    optsize = struct.unpack_from("<H", d, pe + 20)[0]
    export_rva = struct.unpack_from("<I", d, pe + 24 + 96)[0]
    sections = []
    for i in range(nsec):
        o = pe + 24 + optsize + i * 40
        vsize, va, rsize, rptr = struct.unpack_from("<IIII", d, o + 8)
        sections.append((va, max(vsize, rsize), rptr))

    def off(rva):
        for va, size, rptr in sections:
            if va <= rva < va + size:
                return rva - va + rptr
        raise ValueError(f"{path}: RVA {rva:#x} is in no section")

    e = off(export_rva)
    count = struct.unpack_from("<I", d, e + 24)[0]
    names = off(struct.unpack_from("<I", d, e + 32)[0])
    out = set()
    for i in range(count):
        n = off(struct.unpack_from("<I", d, names + 4 * i)[0])
        out.add(d[n : d.index(b"\0", n)].decode("ascii"))
    return out


def syscall_bytes(path):
    """x86: the argument bytes of each system-call stub an image exports (NTDLL's Nt* and
    Zw*), read from the stub: mov eax, N / lea edx, [esp+4] / int 2Eh / ret BYTES."""
    with open(path, "rb") as f:
        d = f.read()
    pe = struct.unpack_from("<I", d, 0x3C)[0]
    nsec = struct.unpack_from("<H", d, pe + 6)[0]
    optsize = struct.unpack_from("<H", d, pe + 20)[0]
    export_rva = struct.unpack_from("<I", d, pe + 24 + 96)[0]
    sections = [struct.unpack_from("<IIII", d, pe + 24 + optsize + i * 40 + 8) for i in range(nsec)]

    def off(rva):
        for vsize, va, rsize, rptr in sections:
            if va <= rva < va + max(vsize, rsize):
                return rva - va + rptr
        raise ValueError(f"{path}: RVA {rva:#x} is in no section")

    e = off(export_rva)
    count = struct.unpack_from("<I", d, e + 24)[0]
    funcs, names, ords = (off(struct.unpack_from("<I", d, e + o)[0]) for o in (28, 32, 36))
    out = {}
    for i in range(count):
        n = off(struct.unpack_from("<I", d, names + 4 * i)[0])
        name = d[n : d.index(b"\0", n)].decode("ascii")
        o = struct.unpack_from("<H", d, ords + 2 * i)[0]
        code = d[off(struct.unpack_from("<I", d, funcs + 4 * o)[0]) :][:15]
        if code[0] != 0xB8 or code[5:11] != b"\x8d\x54\x24\x04\xcd\x2e":
            continue
        if code[11] == 0xC2:
            out[name] = struct.unpack_from("<H", code, 12)[0]
        elif code[11] == 0xC3:
            out[name] = 0
    return out


def src_names(path):
    """The names of a .src/.def export list: first word of each indented line."""
    names = set()
    with open(path, errors="replace") as f:
        for line in f:
            m = re.match(r"^\s+([A-Za-z_][A-Za-z0-9_@]*)", line)
            if m and not line.lstrip().startswith(("#", ";")):
                names.add(m.group(1))
    return names - {"EXPORTS", "LIBRARY", "DESCRIPTION"}


def kernel32_arch_names(path, arch):
    """The names in the architecture block of kernel32.src for arch."""
    opener = re.compile(KERNEL32_ARCH_BLOCK[arch])
    names, inside = set(), False
    with open(path, errors="replace") as f:
        for line in f:
            if opener.match(line.strip()):
                inside = True
                continue
            if inside and re.match(r"\s*#(elif|else|endif)", line):
                break
            if inside:
                m = re.match(r"^\s+([A-Za-z_][A-Za-z0-9_]*)", line)
                if m:
                    names.add(m.group(1))
    if not names:
        sys.exit(f"{path}: no {arch} block found")
    return names


def file_name(dll):
    """A DLL's file name: winspool.drv as it is, kernel32 as kernel32.dll."""
    return dll if "." in dll else dll + ".dll"


def find_dll(cd, name):
    for f in os.listdir(cd):
        if f.lower() == file_name(name):
            return os.path.join(cd, f)
    sys.exit(f"{cd}: no {file_name(name)}")


# __stdcall argument bytes of exports the --decorations library lacks (undocumented NTDLL
# functions other than system calls) that code for NT 4.0 calls.
EXTRA_DECORATIONS = {
    "RtlAdjustPrivilege": 16,
    "RtlDosPathNameToNtPathName_U": 16,
}


def lib_decorations(path):
    """Undecorated name -> decorated .def name (Name@bytes), from an x86 import library's
    short import objects (_Name@bytes)."""
    with open(path, "rb") as f:
        d = f.read()
    if d[:8] != b"!<arch>\n":
        sys.exit(f"{path}: not an import library")
    out, o = {}, 8
    while o + 60 <= len(d):
        size = int(d[o + 48 : o + 58])
        body = d[o + 60 : o + 60 + size]
        o += 60 + size + (size & 1)
        if len(body) < 20 or body[:4] != b"\0\0\xff\xff" or struct.unpack_from("<H", body, 6)[0] != 0x14C:
            continue
        sym = body[20 : body.index(b"\0", 20)].decode("ascii")
        m = re.fullmatch(r"_(.+)@(\d+)", sym)
        if m:
            out[m.group(1)] = f"{m.group(1)}@{m.group(2)}"
    for name, n in EXTRA_DECORATIONS.items():
        out.setdefault(name, f"{name}@{n}")
    return out


def write_def(path, dll, names, note, arch="arm"):
    target = {"arm": "ARM (PINT 4.0)", "x86": "x86"}[arch]
    with open(path, "w") as f:
        f.write(f"; {file_name(dll)} exports of Windows NT 4.0 SP0 for {target}.\n")
        f.write(f"; Generated by tools/gen-defs.py: {note}\n")
        f.write(f"LIBRARY {file_name(dll)}\nEXPORTS\n")
        for n in sorted(names, key=str.lower):
            f.write(f"    {n}\n")


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--cd", required=True)
    ap.add_argument("--cd-arch", default="alpha", choices=sorted(KERNEL32_ARCH_BLOCK))
    ap.add_argument("--nt4-src")
    ap.add_argument("--pint")
    ap.add_argument("--pint-dlls")
    ap.add_argument("--arch", default="arm", choices=["arm", "x86"])
    ap.add_argument("--decorations", help="x86: an import library with decorated names")
    ap.add_argument("--out")
    a = ap.parse_args()
    a.out = a.out or f"imports/{a.arch}"

    if a.arch == "x86":
        if not a.decorations:
            sys.exit("--arch x86 needs --decorations")
        deco = lib_decorations(a.decorations)
        os.makedirs(a.out, exist_ok=True)
        for dll in DLLS + X86_DLLS:
            path = find_dll(a.cd, dll)
            names = pe_exports(path)
            stubs = {n: f"{n}@{b}" for n, b in syscall_bytes(path).items()}
            for n in sorted(stubs.keys() & deco.keys()):
                if stubs[n] != deco[n]:
                    print(f"{dll}: {n}: the stub says {stubs[n]}, the library {deco[n]}", file=sys.stderr)
            written = sorted((stubs.get(n) or deco.get(n, n) for n in names), key=str.lower)
            plain = sum(1 for n in names if n not in deco and n not in stubs)
            write_def(os.path.join(a.out, f"{dll.split('.')[0]}.def"), dll, written,
                      f"the NT 4.0 x86 CD's exports, __stdcall bytes from {os.path.basename(a.decorations)} "
                      f"({plain} names it lacks are __cdecl or unknown)", "x86")
            print(f"{dll}: {len(names)} exports, {len(names) - plain} decorated")
        return
    if not (a.nt4_src and a.pint):
        sys.exit("--arch arm needs --nt4-src and --pint")

    os.makedirs(a.out, exist_ok=True)
    dll_dir = os.path.join(a.nt4_src, "private", "ntos", "dll")
    armdef = os.path.join(a.pint, "kernel", "dll", "armdef.src")
    k32src = os.path.join(a.nt4_src, "private", "windows", "base", "client", "kernel32.src")

    for dll in DLLS:
        names = pe_exports(find_dll(a.cd, dll))
        note = f"the NT 4.0 {a.cd_arch} CD's exports"
        if dll == "ntdll":
            names -= src_names(os.path.join(dll_dir, f"{a.cd_arch}def.src"))
            names |= src_names(armdef)
            note += f" - {a.cd_arch}def.src + PINT armdef.src"
        elif dll == "kernel32":
            names -= kernel32_arch_names(k32src, a.cd_arch)
            note += f" - the {a.cd_arch} block of kernel32.src"
        if a.pint_dlls:
            missing = names - pe_exports(find_dll(a.pint_dlls, dll))
            names -= missing
            if missing:
                note += f" - {len(missing)} names PINT's {dll}.dll does not export"
                print(f"{dll}: not in PINT: {' '.join(sorted(missing))}")
        write_def(os.path.join(a.out, f"{dll}.def"), dll, names, note)
        print(f"{dll}: {len(names)} exports")


if __name__ == "__main__":
    main()
