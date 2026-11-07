/* nt4c99.h: what C99 code expects of the C library and NT 4.0's msvcrt.dll lacks, given
   by nt4c99.lib (ruststdwinnt4 crt/). Code without gnulib can include it first (-FI); the
   library also has snprintf, vsnprintf and the long long functions under their own
   names, for code that does not.

   - snprintf and vsnprintf (msvcrt has only _snprintf, which neither terminates a
     truncated string nor returns the length needed), and the printf family understanding
     C99's length modifiers (ll, j, z, t, hh): msvcrt knows I64 and h only. printf,
     fprintf, sprintf and their v forms become nt4_ functions translating the format.
     hh prints as h (a short, not a char): the value would have to be changed, not the
     format;
   - strtoll, strtoull, atoll, llabs and the intmax_t functions (not lldiv and imaxdiv,
     which cl builds in with types of its own);
   - ssize_t, va_copy; isnan, isinf, isfinite, NAN, INFINITY. */
#ifndef NT4_C99_H
#define NT4_C99_H

#ifndef RC_INVOKED
#include <stdarg.h>
#include <stddef.h>
/* Included first, so their declarations keep msvcrt's names (and its dllimport); the
   macros below then rename the calls only. */
#include <stdio.h>
#include <float.h>

#ifndef _SSIZE_T_DEFINED
#define _SSIZE_T_DEFINED
#ifdef _WIN64
typedef long long ssize_t;
#else
typedef int ssize_t;
#endif
#endif

#ifndef va_copy
#define va_copy(d, s) ((d) = (s))
#endif

#ifdef __cplusplus
extern "C" {
#endif

int nt4_printf(const char *fmt, ...);
int nt4_fprintf(FILE *f, const char *fmt, ...);
int nt4_sprintf(char *buf, const char *fmt, ...);
int nt4_snprintf(char *buf, size_t n, const char *fmt, ...);
int nt4_vprintf(const char *fmt, va_list ap);
int nt4_vfprintf(FILE *f, const char *fmt, va_list ap);
int nt4_vsprintf(char *buf, const char *fmt, va_list ap);
int nt4_vsnprintf(char *buf, size_t n, const char *fmt, va_list ap);

long long strtoll(const char *s, char **end, int base);
unsigned long long strtoull(const char *s, char **end, int base);
long long atoll(const char *s);
long long llabs(long long x);

#ifdef __cplusplus
}
#endif

#define printf    nt4_printf
#define fprintf   nt4_fprintf
#define sprintf   nt4_sprintf
#define snprintf  nt4_snprintf
#define vprintf   nt4_vprintf
#define vfprintf  nt4_vfprintf
#define vsprintf  nt4_vsprintf
#define vsnprintf nt4_vsnprintf

/* C99 classification of doubles, on msvcrt's _isnan and _finite (float.h). */
#ifndef isnan
#define isnan(x)    _isnan((double)(x))
#define isfinite(x) _finite((double)(x))
#define isinf(x)    (!_finite((double)(x)) && !_isnan((double)(x)))
#endif
#ifndef INFINITY
#define INFINITY ((float)(1e300 * 1e300))
#define NAN      ((float)(INFINITY * 0.0f))
#endif

#endif /* RC_INVOKED */
#endif
