#!/usr/bin/env python3
"""symbolize.py EXE MAP C-DIR -- ADDRESS...: name the functions at addresses of an Alpha exe that
link 3.10 made from mrustc's C, static ones included (most are: the map lists only publics, and
Visual C++ 4.1 writes no symbols for statics without debug information). The exe's .pdata
lists every function; within an object they are in the order of the C file's definitions, so
the publics anchor .pdata entries to definitions in C-DIR/<object>.c and the rest follow."""
import bisect
import re
import struct
import sys

args = sys.argv[1:]
sep = args.index('--')
exe_path, map_path, cdir = args[0], args[1], args[2]
addrs = [int(a, 16) for a in args[sep + 1:]]

exe = open(exe_path, 'rb').read()
pe = struct.unpack_from('<I', exe, 0x3c)[0]
nsec = struct.unpack_from('<H', exe, pe + 6)[0]
opt = struct.unpack_from('<H', exe, pe + 20)[0]
base = struct.unpack_from('<I', exe, pe + 24 + 28)[0]
starts = []
for i in range(nsec):
    o = pe + 24 + opt + 40 * i
    if exe[o:o + 8].rstrip(b'\0') == b'.pdata':
        vs, va, rs, rp = struct.unpack_from('<IIII', exe, o + 8)
        for k in range(0, min(vs, rs) - 19, 20):
            b = struct.unpack_from('<I', exe, rp + k)[0]
            if b:
                starts.append(b)
starts = sorted(set(starts))

publics = []  # (address, name, object)
for line in open(map_path, errors='replace'):
    m = re.match(r'\s*[0-9a-f]{4}:[0-9a-f]{8}\s+(\S+)\s+([0-9a-f]{8})\s+f\s+(\S+)', line)
    if m:
        publics.append((int(m.group(2), 16), m.group(1), m.group(3)))
publics.sort()
pub_by_obj = {}
for a, n, o in publics:
    pub_by_obj.setdefault(o, []).append((a, n))

defn = re.compile(r'^(?:static )?(?:struct |union )?[A-Za-z_][\w$]*\s*\*?\s+\*?\s*(ZR[\w$]+|[A-Za-z_]\w*)\($')
cache = {}
def definitions(obj):
    """Function definitions of the object's C file, in order (a header line, then the
    parameters, then a line with only `{` after the `) // -> ...` line)."""
    if obj in cache:
        return cache[obj]
    name = obj.rsplit('-', 1)[0]  # libstd.rlib-alpha.obj -> libstd.rlib
    try:
        lines = open(f'{cdir}/{name}.c', errors='replace').read().split('\n')
    except OSError:
        cache[obj] = None
        return None
    out = []
    for i, l in enumerate(lines):
        m = re.match(r'^(?:static\s+)?[^\s/#(][^(;]*?\s\**([A-Za-z_][\w$]*)\($', l)
        if not m:
            continue
        # a definition, not a prototype: the header ends with a line holding only "{"
        j = i + 1
        while j < len(lines) and not lines[j].startswith(')') and not lines[j].lstrip().startswith(')'):
            j += 1
        while j < len(lines) and lines[j].strip() == '' or (j < len(lines) and lines[j].lstrip().startswith(')')):
            j += 1
        if j < len(lines) and lines[j].strip() == '{':
            out.append(m.group(1))
    cache[obj] = out
    return out

for a in addrs:
    i = bisect.bisect_right(starts, a) - 1
    k = bisect.bisect_right([p[0] for p in publics], a) - 1
    if i < 0 or k < 0:
        print(f'{a:08x}  ?'); continue
    pa, pname, pobj = publics[k]
    defs = definitions(pobj)
    note = ''
    if defs and pname in defs:
        # the public's .pdata index and definition index give the offset for this stretch
        pi = bisect.bisect_left(starts, pa)
        j = defs.index(pname) + (i - pi)
        name = defs[j] if 0 <= j < len(defs) else '?'
        note = '' if starts[pi] == pa else ' (public not at a .pdata start!)'
    else:
        name = pname + ' (public)'
    print(f'{a:08x}  {name}+{a - starts[i]:#x}  [{pobj}]{note}')
