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

  tools/implib.py --aliases imports/x86/oldnames.txt --out oldnames.lib

writes a library of weak aliases instead, one object per line `NAME TARGET` of the list,
for the C names and their __imp_ pointers (`_open` for `__open`, `__imp__open` for
`__imp___open`): what oldnames.lib is. A definition elsewhere takes precedence over them.

The libraries have no __IMPORT_DESCRIPTOR_<dll>, __NULL_IMPORT_DESCRIPTOR or
<dll>_NULL_THUNK_DATA objects: lld makes the import tables itself. Microsoft's linker
would want them.
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
IMAGE_SYM_CLASS_EXTERNAL = 2
IMAGE_SYM_CLASS_STATIC = 3
IMAGE_SYM_CLASS_WEAK_EXTERNAL = 105
IMAGE_WEAK_EXTERN_SEARCH_ALIAS = 3


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


def alias_object(name, target):
    """A COFF object whose weak externals make NAME and __imp_NAME aliases of TARGET and
    __imp_TARGET (IMAGE_WEAK_EXTERN_SEARCH_ALIAS)."""
    strtab, syms = b"", b""

    def sym_name(n):
        nonlocal strtab
        b = n.encode()
        if len(b) <= 8:
            return b.ljust(8, b"\0")
        off = 4 + len(strtab)
        strtab += b + b"\0"
        return struct.pack("<II", 0, off)

    # @feat.00 = 1: the object is safe for /SAFESEH (it has no code), which rustc asks for.
    syms = sym_name("@feat.00") + struct.pack("<IhHBB", 1, -1, 0, IMAGE_SYM_CLASS_STATIC, 0)
    index = 1
    for alias, real in ((name, target), ("__imp_" + name, "__imp_" + target)):
        # The target, undefined; then the alias, with its aux record naming the target.
        syms += sym_name(real) + struct.pack("<IhHBB", 0, 0, 0, IMAGE_SYM_CLASS_EXTERNAL, 0)
        syms += sym_name(alias) + struct.pack("<IhHBB", 0, 0, 0, IMAGE_SYM_CLASS_WEAK_EXTERNAL, 1)
        syms += struct.pack("<II10x", index, IMAGE_WEAK_EXTERN_SEARCH_ALIAS)
        index += 3
    head = struct.pack("<HHIIIHH", MACHINE_I386, 0, 0, 20, index, 0, 0)
    return head + syms + struct.pack("<I", 4 + len(strtab)) + strtab


def member_header(name, size):
    h = f"{name:<16}{0:<12}{0:<6}{0:<6}{0:<8}{size:<10}`\n".encode()
    assert len(h) == 60, name
    return h


def pad(b):
    return b + (b"\n" if len(b) % 2 else b"")


def write_archive(members, out):
    """members: (member name, data, [symbols it defines]) in order."""
    nsyms = sum(len(s) for _, _, s in members)
    names = [n for _, _, syms in members for n in syms]
    strtab1 = b"".join(n.encode() + b"\0" for n in names)
    first_size = 4 + 4 * nsyms + len(strtab1)
    second_size = 4 + 4 * len(members) + 4 + 2 * nsyms + len(strtab1)

    offset = 8 + 60 + first_size + (first_size & 1) + 60 + second_size + (second_size & 1)
    offsets = []
    for _, data, _ in members:
        offsets.append(offset)
        offset += 60 + len(data) + (len(data) & 1)

    # First linker member: big-endian, symbols in member order.
    sym_offsets = [offsets[i] for i, (_, _, syms) in enumerate(members) for _ in syms]
    first = struct.pack(">I", nsyms) + b"".join(struct.pack(">I", o) for o in sym_offsets) + strtab1
    # Second linker member: little-endian, symbols sorted, with 1-based member indexes.
    by_name = sorted(((n, i + 1) for i, (_, _, syms) in enumerate(members) for n in syms), key=lambda t: t[0].encode())
    second = (
        struct.pack("<I", len(members))
        + b"".join(struct.pack("<I", o) for o in offsets)
        + struct.pack("<I", nsyms)
        + b"".join(struct.pack("<H", i) for _, i in by_name)
        + b"".join(n.encode() + b"\0" for n, _ in by_name)
    )
    assert len(first) == first_size and len(second) == second_size
    assert len(members) < 65536

    with open(out, "wb") as f:
        f.write(b"!<arch>\n")
        f.write(member_header("/", first_size) + pad(first))
        f.write(member_header("/", second_size) + pad(second))
        for name, data, _ in members:
            f.write(member_header(name, len(data)) + pad(data))


def write_lib(dll, exports, out):
    member_name = dll + "/"
    if len(member_name) > 16:
        raise SystemExit(f"{dll}: the name needs a long-names member, which this writer lacks")
    members = []
    for e in exports:
        sym, name_type = symbol(e)
        members.append((member_name, import_object(sym, dll, name_type), ["__imp_" + sym, sym]))
    write_archive(members, out)


def write_aliases(path, out):
    members = []
    with open(path) as f:
        for line in f:
            line = line.split("#", 1)[0].split()
            if not line:
                continue
            name, target = line
            # The C names: x86's '_' before each.
            members.append((f"{len(members)}.obj/", alias_object("_" + name, "_" + target),
                            ["_" + name, "__imp__" + name]))
    write_archive(members, out)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    src = ap.add_mutually_exclusive_group(required=True)
    src.add_argument("--def", dest="deffile")
    src.add_argument("--aliases", help="a list of NAME TARGET lines: write weak aliases")
    ap.add_argument("--out", required=True)
    a = ap.parse_args()
    if a.aliases:
        write_aliases(a.aliases, a.out)
    else:
        dll, exports = read_def(a.deffile)
        write_lib(dll, exports, a.out)


if __name__ == "__main__":
    main()
