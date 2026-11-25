typedef struct { void *p; unsigned long n; } SLICE;
typedef struct { unsigned __int64 lo, hi; } U128;
SLICE mk(void *p, unsigned long n) { SLICE s; s.p = p; s.n = n; return s; }
U128 add(U128 a, U128 b) { U128 r; r.lo = a.lo + b.lo; r.hi = a.hi + b.hi + (r.lo < a.lo); return r; }
