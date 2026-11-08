/* limits.h for Windows NT 4.0's C run-time (ruststdwinnt4 crt/): NT 4.0's, which predates
   long long, with C99's limits for it. The SDK's header is included by its path in PINT's
   SDK sandbox, where every compile for ARM runs; clang (x86, tools/nt4-clang) finds it
   next on its include path. */
#ifndef NT4_LIMITS_H
#define NT4_LIMITS_H
#ifdef __clang__
#include_next <limits.h>
#else
#include "Z:/opt/pint40sdk/include/crt/limits.h"
#endif
#ifndef LLONG_MAX
#define LLONG_MAX  9223372036854775807LL
#define LLONG_MIN  (-LLONG_MAX - 1)
#define ULLONG_MAX 0xffffffffffffffffULL
#endif
#endif
