#include <windows.h>
LONG f(LONG volatile *p) { InterlockedIncrement((LPLONG)p); return InterlockedExchange((LPLONG)p, 5); }
PVOID g(PVOID *p, PVOID n, PVOID c) { return InterlockedCompareExchange(p, n, c); }
