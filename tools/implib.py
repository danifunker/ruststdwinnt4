#!/usr/bin/env python3
"""Write an x86 import library from a .def file, the way Microsoft's LIB does.

  tools/implib.py --def imports/x86/kernel32.def --out kernel32.lib

On x86 the names programs link against are decorated: `_CreateFileW@28` for a
__stdcall function, `_strlen` for a __cdecl one. The DLL exports them plain
(`CreateFileW`). rust-lld's /def keeps the decoration in the name it imports, so x86
import libraries are written here: each entry is a short import object whose name type
tells the linker how to get from the decorated name to the exported one.

In the .def file a __stdcall export is written with its argument bytes
(`CreateFileW@28`), a __cdecl one plain (`strlen`), a C++ one mangled (`?f@@YAXXZ`).
"""

import argparse
import re
import struct

MACHINE_I386 = 0x14C
# IMPORT_OBJECT_NAME_TYPE
NAME = 1  # the symbol is the export
NAME_NOPREFIX = 2  # the export is the symbol without its leading '_'
NAME_UNDECORATE = 3  # ... and without its '@bytes'
IMPORT_CODE = 0


def read_def(path):
    dll, names, in_exports = None, [], False
    with open(path) as f:
        for line in f:
            line = line.split(";", 1)[0].strip()
            if not line:
                continue
            word = line.split()[0]
            if word.upper() == "LIBRARY":
                dll = line.split()[1].strip('"')
                in_exports = False
            elif word.upper() == "EXPORTS":
                in_exports = True
            elif in_exports:
                names.append(word)
    if not dll:
        raise SystemExit(f"{path}: no LIBRARY line")
    return dll, names


def symbol(export):
    """The linker's name for an export as written in the .def file, and its name type."""
    if export.startswith("?"):
        return export, NAME
    if re.fullmatch(r".+@\d+", export):
        return "_" + export, NAME_UNDECORATE
    return "_" + export, NAME_NOPREFIX


def import_object(sym, dll, name_type):
    data = sym.encode() + b"\0" + dll.encode() + b"\0"
    head = struct.pack("<HHHHIIHH", 0, 0xFFFF, 0, MACHINE_I386, 0, len(data), 0, IMPORT_CODE | (name_type << 2))
    return head + data


def member_header(name, size):
    h = f"{name:<16}{0:<12}{0:<6}{0:<6}{0:<8}{size:<10}`\n".encode()
    assert len(h) == 60, name
    return h


def pad(b):
    return b + (b"\n" if len(b) % 2 else b"")


def write_lib(dll, exports, out):
    members = []  # (data, [symbols it defines])
    for e in exports:
        sym, name_type = symbol(e)
        members.append((import_object(sym, dll, name_type), ["__imp_" + sym, sym]))

    member_name = dll + "/"
    if len(member_name) > 16:
        raise SystemExit(f"{dll}: the name needs a long-names member, which this writer lacks")
    nsyms = sum(len(s) for _, s in members)
    names = [n for _, syms in members for n in syms]
    strtab1 = b"".join(n.encode() + b"\0" for n in names)
    first_size = 4 + 4 * nsyms + len(strtab1)
    second_size = 4 + 4 * len(members) + 4 + 2 * nsyms + len(strtab1)

    offset = 8 + 60 + first_size + (first_size & 1) + 60 + second_size + (second_size & 1)
    offsets = []
    for data, _ in members:
        offsets.append(offset)
        offset += 60 + len(data) + (len(data) & 1)

    # First linker member: big-endian, symbols in member order.
    sym_offsets = [offsets[i] for i, (_, syms) in enumerate(members) for _ in syms]
    first = struct.pack(">I", nsyms) + b"".join(struct.pack(">I", o) for o in sym_offsets) + strtab1
    # Second linker member: little-endian, symbols sorted, with 1-based member indexes.
    by_name = sorted(((n, i + 1) for i, (_, syms) in enumerate(members) for n in syms), key=lambda t: t[0].encode())
    second = (
        struct.pack("<I", len(members))
        + b"".join(struct.pack("<I", o) for o in offsets)
        + struct.pack("<I", nsyms)
        + b"".join(struct.pack("<H", i) for _, i in by_name)
        + b"".join(n.encode() + b"\0" for n, _ in by_name)
    )
    assert len(first) == first_size and len(second) == second_size

    with open(out, "wb") as f:
        f.write(b"!<arch>\n")
        f.write(member_header("/", first_size) + pad(first))
        f.write(member_header("/", second_size) + pad(second))
        for data, _ in members:
            f.write(member_header(member_name, len(data)) + pad(data))


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--def", dest="deffile", required=True)
    ap.add_argument("--out", required=True)
    a = ap.parse_args()
    dll, exports = read_def(a.deffile)
    write_lib(dll, exports, a.out)


if __name__ == "__main__":
    main()
