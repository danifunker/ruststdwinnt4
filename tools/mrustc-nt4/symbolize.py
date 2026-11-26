#!/usr/bin/env python3
"""symbolize.py MAP OBJ... -- ADDRESS...: name the functions at addresses of an exe that link
3.10 made, including static ones (mrustc emits most functions static, and the map lists
only public symbols). Each object's .text is contiguous in the exe (no COMDATs), so an
address is placed in an object by the publics the map gives for it, and named from that
object's COFF symbol table, which has the statics."""
import bisect
import re
import struct
import sys

args = sys.argv[1:]
sep = args.index('--')
mapfile, objs, addrs = args[0], args[1:sep], [int(a, 16) for a in args[sep + 1:]]

publics = []  # (exe address, name, object)
for line in open(mapfile, errors='replace'):
    m = re.match(r'\s*[0-9a-f]{4}:[0-9a-f]{8}\s+(\S+)\s+([0-9a-f]{8})\s+f\s+(\S+)', line)
    if m:
        publics.append((int(m.group(2), 16), m.group(1), m.group(3)))

def coff_text_symbols(path):
    """{name: offset in .text} and sorted [(offset, name)] of a COFF object's functions."""
    d = open(path, 'rb').read()
    nsec, _, symptr, nsym = struct.unpack_from('<HHIII', d, 2)[0], 0, *struct.unpack_from('<II', d, 8)
    secs = [d[20 + 40 * i:20 + 40 * i + 8].rstrip(b'\0') for i in range(nsec)]
    strtab = symptr + 18 * nsym
    out, i = [], 0
    while i < nsym:
        o = symptr + 18 * i
        raw = d[o:o + 8]
        if raw[:4] == b'\0\0\0\0':
            off = struct.unpack_from('<I', raw, 4)[0]
            name = d[strtab + off:d.index(b'\0', strtab + off)].decode('latin1')
        else:
            name = raw.rstrip(b'\0').decode('latin1')
        value, secno, typ, cls, naux = struct.unpack_from('<IhHBB', d, o + 8)
        if secno > 0 and secs[secno - 1] == b'.text' and cls in (2, 3) and not name.startswith('.') \
                and not name.startswith('$'):
            out.append((value, name))
        i += 1 + naux
    out.sort()
    return {n: v for v, n in out}, out

tables = {}
for path in objs:
    key = path.replace('\\', '/').split('/')[-1]
    tables[key] = coff_text_symbols(path)

publics.sort()
keys = [p[0] for p in publics]
for a in addrs:
    i = bisect.bisect_right(keys, a) - 1
    if i < 0:
        print(f'{a:08x}  ?'); continue
    pa, pname, pobj = publics[i]
    t = tables.get(pobj)
    if not t or pname not in t[0]:
        print(f'{a:08x}  {pname}+{a - pa:#x}  ({pobj}, public only)'); continue
    objoff = t[0][pname] + (a - pa)
    syms = t[1]
    j = bisect.bisect_right([s[0] for s in syms], objoff) - 1
    print(f'{a:08x}  {syms[j][1]}+{objoff - syms[j][0]:#x}  ({pobj})')
