/* limits.h for Windows NT 4.0's C run-time (ruststdwinnt4 crt/): NT 4.0's, which predates
   long long, with C99's limits for it. The SDK's header is included by its path in PINT's
   SDK sandbox, where every compile of the target runs. */
#ifndef NT4_LIMITS_H
#define NT4_LIMITS_H
#include "Z:/opt/pint40sdk/include/crt/limits.h"
#ifndef LLONG_MAX
#define LLONG_MAX  9223372036854775807LL
#define LLONG_MIN  (-LLONG_MAX - 1)
#define ULLONG_MAX 0xffffffffffffffffULL
#endif
#endif
