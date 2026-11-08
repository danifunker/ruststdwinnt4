#!/usr/bin/env python3
"""Show what Windows NT 4.0 will make of a PE image, and check its imports.

  tools/peinfo.py IMAGE [--defs imports/arm]

Prints the machine, the OS and subsystem versions, the sections, the relocation types
and the imports. With --defs, every imported name must be in the .def file of its DLL
(the exports of NT 4.0 SP0); the exit status is 1 when one is not, or when the image is
marked for a Windows later than 4.0.
"""

import argparse
import os
import re
import struct
import sys

MACHINES = {0x14C: "i386", 0x166: "R4000", 0x184: "Alpha", 0x1F0: "PowerPC", 0x1C4: "ARMNT"}
SUBSYSTEMS = {1: "native", 2: "windows", 3: "console", 7: "posix"}
RELOC_TYPES = {0: "ABSOLUTE", 3: "HIGHLOW", 7: "THUMB_MOV32", 5: "ARM_MOV32"}


class Image:
    def __init__(self, path):
        with open(path, "rb") as f:
            self.d = f.read()
        d = self.d
        if d[:2] != b"MZ":
            sys.exit(f"{path}: not a PE image")
        pe = struct.unpack_from("<I", d, 0x3C)[0]
        if d[pe : pe + 4] != b"PE\0\0":
            sys.exit(f"{path}: no PE signature")
        self.machine, self.nsec = struct.unpack_from("<HH", d, pe + 4)
        self.characteristics = struct.unpack_from("<H", d, pe + 22)[0]
        optsize = struct.unpack_from("<H", d, pe + 20)[0]
        o = pe + 24
        self.magic = struct.unpack_from("<H", d, o)[0]
        if self.magic != 0x10B:
            sys.exit(f"{path}: not a 32-bit image")
        self.entry, = struct.unpack_from("<I", d, o + 16)
        self.base, = struct.unpack_from("<I", d, o + 28)
        self.os_version = struct.unpack_from("<HH", d, o + 40)
        self.image_version = struct.unpack_from("<HH", d, o + 44)
        self.subsystem_version = struct.unpack_from("<HH", d, o + 48)
        self.subsystem, self.dll_characteristics = struct.unpack_from("<HH", d, o + 68)
        self.stack = struct.unpack_from("<II", d, o + 72)
        ndirs = struct.unpack_from("<I", d, o + 92)[0]
        self.dirs = [struct.unpack_from("<II", d, o + 96 + 8 * i) for i in range(ndirs)]
        self.sections = []
        for i in range(self.nsec):
            s = o + optsize + 40 * i
            name = d[s : s + 8].rstrip(b"\0").decode("ascii", "replace")
            vsize, va, rsize, rptr = struct.unpack_from("<IIII", d, s + 8)
            flags = struct.unpack_from("<I", d, s + 36)[0]
            self.sections.append((name, va, vsize, rptr, rsize, flags))

    def off(self, rva):
        for _, va, vsize, rptr, rsize, _ in self.sections:
            if va <= rva < va + max(vsize, rsize):
                return rva - va + rptr
        raise ValueError(f"RVA {rva:#x} is in no section")

    def cstr(self, rva):
        o = self.off(rva)
        return self.d[o : self.d.index(b"\0", o)].decode("ascii", "replace")

    def imports(self):
        rva, size = self.dirs[1] if len(self.dirs) > 1 else (0, 0)
        out = []
        if not rva:
            return out
        o = self.off(rva)
        while True:
            ilt, _, _, name, iat = struct.unpack_from("<IIIII", self.d, o)
            if not name:
                break
            dll = self.cstr(name)
            t = self.off(ilt or iat)
            while True:
                e = struct.unpack_from("<I", self.d, t)[0]
                if not e:
                    break
                out.append((dll, f"#{e & 0xFFFF}" if e >> 31 else self.cstr(e + 2)))
                t += 4
            o += 20
        return out

    def reloc_types(self):
        rva, size = self.dirs[5] if len(self.dirs) > 5 else (0, 0)
        counts = {}
        if not rva:
            return counts
        o, end = self.off(rva), self.off(rva) + size
        while o < end:
            page, block = struct.unpack_from("<II", self.d, o)
            if block < 8:
                break
            for i in range((block - 8) // 2):
                t = struct.unpack_from("<H", self.d, o + 8 + 2 * i)[0] >> 12
                counts[t] = counts.get(t, 0) + 1
            o += block
        return counts


def def_names(path):
    names = set()
    with open(path) as f:
        for line in f:
            m = re.match(r"^\s+(\S+)", line)
            if m and not line.lstrip().startswith(";"):
                # x86 lists write __stdcall names with their argument bytes
                # (CreateFileW@28); the image imports them plain.
                name = m.group(1)
                if not name.startswith("?"):
                    name = re.sub(r"@\d+$", "", name)
                names.add(name)
    return names


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("image")
    ap.add_argument("--defs", help="directory of the .def files to check the imports against")
    a = ap.parse_args()
    im = Image(a.image)
    ok = True

    print(f"machine     {MACHINES.get(im.machine, hex(im.machine))}")
    print(f"base/entry  {im.base:#010x} / {im.base + im.entry:#010x}")
    print(f"os version  {im.os_version[0]}.{im.os_version[1]:02}")
    print(f"subsystem   {SUBSYSTEMS.get(im.subsystem, im.subsystem)} "
          f"{im.subsystem_version[0]}.{im.subsystem_version[1]:02}")
    print(f"stack       reserve {im.stack[0]:#x}, commit {im.stack[1]:#x}")
    print(f"dll chars   {im.dll_characteristics:#06x}")
    for v, what in ((im.os_version, "OS"), (im.subsystem_version, "subsystem")):
        if v > (4, 0):
            print(f"ERROR: {what} version {v[0]}.{v[1]:02}: NT 4.0 does not start the image")
            ok = False
    tls = im.dirs[9] if len(im.dirs) > 9 else (0, 0)
    print(f"TLS dir     {'yes' if tls[0] else 'none'}")
    print("sections    " + ", ".join(f"{n} {vs:#x}" for n, _, vs, _, _, _ in im.sections))
    relocs = im.reloc_types()
    print("relocations " + (", ".join(f"{RELOC_TYPES.get(t, t)} x{c}" for t, c in sorted(relocs.items())) or "none"))

    imports = im.imports()
    by_dll = {}
    for dll, name in imports:
        by_dll.setdefault(dll, []).append(name)
    for dll, names in by_dll.items():
        print(f"imports     {dll}: {' '.join(sorted(names))}")

    if a.defs:
        for dll, names in by_dll.items():
            path = os.path.join(a.defs, dll.lower().removesuffix(".dll") + ".def")
            if not os.path.exists(path):
                print(f"ERROR: {dll} is not a DLL of NT 4.0 SP0 ({path} missing)")
                ok = False
                continue
            exports = def_names(path)
            for n in names:
                if n not in exports:
                    print(f"ERROR: {dll}!{n} is not exported by NT 4.0 SP0")
                    ok = False
        print("check       " + ("all imports are in NT 4.0 SP0" if ok else "FAILED"))
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
