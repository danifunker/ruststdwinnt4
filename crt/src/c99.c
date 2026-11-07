/* nt4c99.lib: C99 functions for code built against Windows NT 4.0's msvcrt.dll, which
   predates C99 (ruststdwinnt4 crt/; see include/nt4c99.h). Built without nt4c99.h, so the
   names below are msvcrt's own. */
#include <errno.h>
#include <limits.h>
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <ctype.h>

/* NT 4.0's limits.h predates long long's limits. */
#ifndef LLONG_MAX
#define LLONG_MAX 9223372036854775807LL
#define LLONG_MIN (-LLONG_MAX - 1)
#define ULLONG_MAX 0xffffffffffffffffULL
#endif

/* ---- printf family ---------------------------------------------------------------- */

/* Copy the format, rewriting C99 length modifiers into msvcrt's: ll, j and q become I64;
   z and t become nothing (32-bit) or I64 (64-bit); hh becomes h; L (long double, which is
   double here) goes. Returns a heap string, or NULL without memory. */
static char *translate(const char *fmt)
{
    size_t n = strlen(fmt);
    char *out = malloc(n * 2 + 1), *o = out;
    const char *p = fmt;
    if (!out)
        return NULL;
    while (*p) {
        if (*p != '%') {
            *o++ = *p++;
            continue;
        }
        *o++ = *p++;
        if (*p == '%') {
            *o++ = *p++;
            continue;
        }
        /* flags, width, precision */
        while (*p && strchr("-+ #0", *p))
            *o++ = *p++;
        while (*p && (isdigit((unsigned char)*p) || *p == '*'))
            *o++ = *p++;
        if (*p == '.') {
            *o++ = *p++;
            while (*p && (isdigit((unsigned char)*p) || *p == '*'))
                *o++ = *p++;
        }
        /* length */
        if (p[0] == 'l' && p[1] == 'l') {
            memcpy(o, "I64", 3), o += 3, p += 2;
        } else if (*p == 'j' || *p == 'q') {
            memcpy(o, "I64", 3), o += 3, p++;
        } else if (*p == 'z' || *p == 't') {
#ifdef _WIN64
            memcpy(o, "I64", 3), o += 3;
#endif
            p++;
        } else if (p[0] == 'h' && p[1] == 'h') {
            *o++ = 'h', p += 2;
        } else if (*p == 'L') {
            p++;
        }
    }
    *o = 0;
    return out;
}

int nt4_vsnprintf(char *buf, size_t n, const char *fmt, va_list ap)
{
    char *t = translate(fmt), *tmp = NULL;
    size_t size;
    int r;
    if (!t) {
        errno = ENOMEM;
        return -1;
    }
    /* Into the caller's buffer when it fits. */
    if (n > 0) {
        r = _vsnprintf(buf, n, t, ap);
        if (r >= 0 && (size_t)r < n) {
            free(t);
            return r;
        }
    }
    /* Otherwise: the length, from a buffer grown until the text fits. */
    for (size = n > 256 ? n * 2 : 512;; size *= 2) {
        char *b = realloc(tmp, size);
        if (!b) {
            free(tmp), free(t);
            errno = ENOMEM;
            return -1;
        }
        tmp = b;
        r = _vsnprintf(tmp, size, t, ap);
        if (r >= 0 && (size_t)r < size)
            break;
        if (size > INT_MAX / 2) {
            free(tmp), free(t);
            errno = ERANGE;
            return -1;
        }
    }
    if (n > 0) {
        size_t c = (size_t)r < n - 1 ? (size_t)r : n - 1;
        memcpy(buf, tmp, c);
        buf[c] = 0;
    }
    free(tmp), free(t);
    return r;
}

int nt4_snprintf(char *buf, size_t n, const char *fmt, ...)
{
    va_list ap;
    int r;
    va_start(ap, fmt);
    r = nt4_vsnprintf(buf, n, fmt, ap);
    va_end(ap);
    return r;
}

int nt4_vsprintf(char *buf, const char *fmt, va_list ap)
{
    char *t = translate(fmt);
    int r;
    if (!t)
        return -1;
    r = vsprintf(buf, t, ap);
    free(t);
    return r;
}

int nt4_sprintf(char *buf, const char *fmt, ...)
{
    va_list ap;
    int r;
    va_start(ap, fmt);
    r = nt4_vsprintf(buf, fmt, ap);
    va_end(ap);
    return r;
}

int nt4_vfprintf(FILE *f, const char *fmt, va_list ap)
{
    char *t = translate(fmt);
    int r;
    if (!t)
        return -1;
    r = vfprintf(f, t, ap);
    free(t);
    return r;
}

int nt4_fprintf(FILE *f, const char *fmt, ...)
{
    va_list ap;
    int r;
    va_start(ap, fmt);
    r = nt4_vfprintf(f, fmt, ap);
    va_end(ap);
    return r;
}

int nt4_vprintf(const char *fmt, va_list ap)
{
    return nt4_vfprintf(stdout, fmt, ap);
}

int nt4_printf(const char *fmt, ...)
{
    va_list ap;
    int r;
    va_start(ap, fmt);
    r = nt4_vfprintf(stdout, fmt, ap);
    va_end(ap);
    return r;
}

/* The C99 names, for code that calls them without nt4c99.h (msvcrt has neither). */
int vsnprintf(char *buf, size_t n, const char *fmt, va_list ap)
{
    return nt4_vsnprintf(buf, n, fmt, ap);
}

int snprintf(char *buf, size_t n, const char *fmt, ...)
{
    va_list ap;
    int r;
    va_start(ap, fmt);
    r = nt4_vsnprintf(buf, n, fmt, ap);
    va_end(ap);
    return r;
}

/* ---- long long ---------------------------------------------------------------------- */

/* strtoull's parse: the magnitude, the sign, and whether it overflowed. */
static unsigned long long parse(const char *s, char **end, int base, int *neg, int *over)
{
    const char *p = s, *digits;
    unsigned long long v = 0, cut;
    int any = 0, cutlim;
    *neg = 0, *over = 0;
    while (isspace((unsigned char)*p))
        p++;
    if (*p == '-' || *p == '+')
        *neg = *p++ == '-';
    if ((base == 0 || base == 16) && p[0] == '0' && (p[1] == 'x' || p[1] == 'X')
        && isxdigit((unsigned char)p[2])) {
        p += 2, base = 16;
    } else if (base == 0) {
        base = *p == '0' ? 8 : 10;
    }
    if (base < 2 || base > 36) {
        if (end)
            *end = (char *)s;
        errno = EINVAL;
        return 0;
    }
    cut = ULLONG_MAX / (unsigned)base;
    cutlim = (int)(ULLONG_MAX % (unsigned)base);
    for (digits = p;; p++) {
        int c = (unsigned char)*p, d;
        if (isdigit(c))
            d = c - '0';
        else if (isalpha(c))
            d = toupper(c) - 'A' + 10;
        else
            break;
        if (d >= base)
            break;
        any = 1;
        if (*over || v > cut || (v == cut && d > cutlim))
            *over = 1;
        else
            v = v * (unsigned)base + (unsigned)d;
    }
    if (end)
        *end = (char *)(any ? p : s);
    (void)digits;
    return v;
}

unsigned long long strtoull(const char *s, char **end, int base)
{
    int neg, over;
    unsigned long long v = parse(s, end, base, &neg, &over);
    if (over) {
        errno = ERANGE;
        return ULLONG_MAX;
    }
    return neg ? (unsigned long long)(-(long long)v) : v;
}

long long strtoll(const char *s, char **end, int base)
{
    int neg, over;
    unsigned long long v = parse(s, end, base, &neg, &over);
    if (over || (!neg && v > (unsigned long long)LLONG_MAX)
        || (neg && v > (unsigned long long)LLONG_MAX + 1)) {
        errno = ERANGE;
        return neg ? LLONG_MIN : LLONG_MAX;
    }
    return neg ? (long long)(0 - v) : (long long)v;
}

long long atoll(const char *s)
{
    return strtoll(s, NULL, 10);
}

/* cl knows these as intrinsics; here they are the real functions code links to. */
#pragma function(llabs)
long long llabs(long long x)
{
    return x < 0 ? -x : x;
}

/* inttypes.h: intmax_t is long long. */
long long strtoimax(const char *s, char **end, int base) { return strtoll(s, end, base); }
unsigned long long strtoumax(const char *s, char **end, int base) { return strtoull(s, end, base); }
long long imaxabs(long long x) { return llabs(x); }
