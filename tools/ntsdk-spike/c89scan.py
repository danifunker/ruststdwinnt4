"""c89scan.py FILE...: count constructs in mrustc's C that Visual C++ 4.1 (C89 + MS extensions of 1996) lacks."""
import re, sys
from collections import Counter
PATS = [
 ("#include <stdint.h>/<stdbool.h>", r'#include <std(int|bool)\.h>'),
 ("#include <intrin.h>/<immintrin.h>", r'#include <(imm)?intrin\.h>'),
 ("static inline / inline", r'\binline\b'),
 ("__forceinline", r'\b__forceinline\b'),
 ("ALIGNOF/_Alignof/__alignof uses", r'\b(ALIGNOF|_Alignof|__alignof__|__alignof)\s*\('),
 ("__declspec(align(N))", r'__declspec\(align'),
 ("__attribute__", r'__attribute__'),
 ("designated init  .f = / [i] =", r'[{,]\s*(\.[A-Za-z_]\w*|\[\d+\])\s*='),
 ("compound literal (T){", r'\)\s*\{\s*[^}]'),
 ("__int128 / int128_t", r'__int128|\bu?int128_t\b'),
 ("__builtin_*", r'\b__builtin_\w+'),
 ("thread-local (__thread/_Thread_local/__declspec(thread))", r'\b__thread\b|_Thread_local|__declspec\(thread\)'),
 ("long long", r'\blong\s+long\b'),
 ("LL/ULL integer suffix", r'\b(0x[0-9A-Fa-f]+|\d+)[uU]?(ll|LL)\b'),
 ("hex float literal", r'\b0x[0-9A-Fa-f.]*[pP][+-]?\d'),
 ("_Static_assert/static_assert", r'\b_?[sS]tatic_assert\b'),
 ("__declspec(noreturn/noinline/selectany)", r'__declspec\((noreturn|noinline|selectany)\)'),
 ("_Interlocked*/atomic intrinsics", r'\b_Interlocked\w+'),
 ("x86 intrinsics (_mm_*, __popcnt, _BitScan*, __debugbreak, __cpuid)", r'\b(_mm_\w+|__popcnt\w*|_BitScan\w+|__debugbreak|__cpuid\w*|_rdtsc|__rdtsc)\b'),
 ("_ReturnAddress/_AddressOfReturnAddress/__assume", r'\b(_ReturnAddress|_AddressOfReturnAddress|__assume)\b'),
 ("C99 float math (…f functions, fma, round, trunc, copysign, isnan)", r'\b(fabsf|sqrtf|floorf|ceilf|truncf?|roundf?|fmaf?|copysignf?|fminf?|fmaxf?|exp2f?|log2f?|isnan|isinf|nanf?|rintf?|nearbyintf?|powif?|INFINITY|NAN)\b'),
 ("variable-length/zero-length array [0]", r'\[0\]\s*;'),
 ("empty struct {}", r'struct\s*\w*\s*\{\s*\}'),
 ("setjmp/longjmp", r'\b(setjmp|longjmp)\b'),
 ("__pragma/#pragma", r'__pragma|#pragma'),
 ("for (T x = …) decl in for-init", r'\bfor\s*\(\s*(int|size_t|uint\w+|int\w+|unsigned)\s+\w+\s*='),
]
def scan(path):
    src = open(path, encoding='latin-1').read()
    c = Counter()
    for name, p in PATS:
        n = len(re.findall(p, src))
        if n: c[name] = n
    # identifier, string and line lengths (MSVC 4: 247 significant chars in names; 2048-byte string literals)
    ids = re.findall(r'\b[A-Za-z_]\w*\b', src)
    long_ids = {i for i in ids if len(i) > 247}
    c["identifiers > 247 chars (distinct)"] = len(long_ids)
    trunc = Counter(i[:247] for i in long_ids)
    c["  ... that collide when cut to 247"] = sum(v for v in trunc.values() if v > 1)
    c["longest identifier"] = max((len(i) for i in set(ids)), default=0)
    strs = re.findall(r'"(?:[^"\\\n]|\\.)*"', src)
    c["longest string literal (bytes in source)"] = max((len(s) for s in strs), default=0)
    lines = src.split('\n')
    c["lines"] = len(lines); c["longest line"] = max(len(l) for l in lines)
    c["bytes"] = len(src)
    return c
for f in sys.argv[1:]:
    print("==", f)
    for k, v in scan(f).items(): print(f"  {v:>9}  {k}")
