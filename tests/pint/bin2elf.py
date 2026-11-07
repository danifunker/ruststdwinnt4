#!/usr/bin/env python3
"""Wrap PINT's ARC firmware (arcfw.bin, a raw image the Pi firmware loads at 0x8000) in an
ELF executable with one segment at that address, for QEMU's -kernel. PINT's own build
makes the same file (obj/arcfw.elf); a PINT SD image only carries arcfw.bin.

  tests/pint/bin2elf.py arcfw.bin arcfw.elf [--base 0x8000]
"""

import argparse
import struct


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("bin")
    ap.add_argument("elf")
    ap.add_argument("--base", type=lambda s: int(s, 0), default=0x8000)
    a = ap.parse_args()
    with open(a.bin, "rb") as f:
        body = f.read()
    ehsize, phsize = 52, 32
    header = struct.pack(
        "<16sHHIIIIIHHHHHH",
        b"\x7fELF\x01\x01\x01" + bytes(9),  # 32-bit, little-endian, version 1
        2,  # executable
        40,  # ARM
        1,
        a.base,  # entry: the first instruction, ARM state
        ehsize,  # program headers right after this header
        0,
        0x05000000,  # EABI version 5
        ehsize,
        phsize,
        1,
        40,
        0,
        0,
    )
    phdr = struct.pack(
        "<IIIIIIII",
        1,  # PT_LOAD
        ehsize + phsize,
        a.base,
        a.base,
        len(body),
        len(body),
        7,  # RWX
        4,
    )
    with open(a.elf, "wb") as f:
        f.write(header + phdr + body)


if __name__ == "__main__":
    main()
