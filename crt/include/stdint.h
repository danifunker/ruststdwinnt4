/* stdint.h for Windows NT 4.0's C run-time, which predates C99 (ruststdwinnt4 crt/). */
#ifndef NT4_STDINT_H
#define NT4_STDINT_H

typedef signed char        int8_t;
typedef short              int16_t;
typedef int                int32_t;
typedef long long          int64_t;
typedef unsigned char      uint8_t;
typedef unsigned short     uint16_t;
typedef unsigned int       uint32_t;
typedef unsigned long long uint64_t;

typedef int8_t   int_least8_t;   typedef uint8_t  uint_least8_t;
typedef int16_t  int_least16_t;  typedef uint16_t uint_least16_t;
typedef int32_t  int_least32_t;  typedef uint32_t uint_least32_t;
typedef int64_t  int_least64_t;  typedef uint64_t uint_least64_t;
typedef int8_t   int_fast8_t;    typedef uint8_t  uint_fast8_t;
typedef int32_t  int_fast16_t;   typedef uint32_t uint_fast16_t;
typedef int32_t  int_fast32_t;   typedef uint32_t uint_fast32_t;
typedef int64_t  int_fast64_t;   typedef uint64_t uint_fast64_t;
typedef int64_t  intmax_t;       typedef uint64_t uintmax_t;

#ifndef _INTPTR_T_DEFINED
#define _INTPTR_T_DEFINED
#ifdef _WIN64
typedef long long intptr_t;
#else
typedef int intptr_t;
#endif
#endif
#ifndef _UINTPTR_T_DEFINED
#define _UINTPTR_T_DEFINED
#ifdef _WIN64
typedef unsigned long long uintptr_t;
#else
typedef unsigned int uintptr_t;
#endif
#endif

#define INT8_MIN   (-127 - 1)
#define INT16_MIN  (-32767 - 1)
#define INT32_MIN  (-2147483647 - 1)
#define INT64_MIN  (-9223372036854775807LL - 1)
#define INT8_MAX   127
#define INT16_MAX  32767
#define INT32_MAX  2147483647
#define INT64_MAX  9223372036854775807LL
#define UINT8_MAX  0xff
#define UINT16_MAX 0xffff
#define UINT32_MAX 0xffffffffU
#define UINT64_MAX 0xffffffffffffffffULL

#define INT_LEAST8_MIN  INT8_MIN
#define INT_LEAST16_MIN INT16_MIN
#define INT_LEAST32_MIN INT32_MIN
#define INT_LEAST64_MIN INT64_MIN
#define INT_LEAST8_MAX  INT8_MAX
#define INT_LEAST16_MAX INT16_MAX
#define INT_LEAST32_MAX INT32_MAX
#define INT_LEAST64_MAX INT64_MAX
#define UINT_LEAST8_MAX  UINT8_MAX
#define UINT_LEAST16_MAX UINT16_MAX
#define UINT_LEAST32_MAX UINT32_MAX
#define UINT_LEAST64_MAX UINT64_MAX
#define INT_FAST8_MIN   INT8_MIN
#define INT_FAST16_MIN  INT32_MIN
#define INT_FAST32_MIN  INT32_MIN
#define INT_FAST64_MIN  INT64_MIN
#define INT_FAST8_MAX   INT8_MAX
#define INT_FAST16_MAX  INT32_MAX
#define INT_FAST32_MAX  INT32_MAX
#define INT_FAST64_MAX  INT64_MAX
#define UINT_FAST8_MAX  UINT8_MAX
#define UINT_FAST16_MAX UINT32_MAX
#define UINT_FAST32_MAX UINT32_MAX
#define UINT_FAST64_MAX UINT64_MAX
#define INTMAX_MIN  INT64_MIN
#define INTMAX_MAX  INT64_MAX
#define UINTMAX_MAX UINT64_MAX

#ifdef _WIN64
#define INTPTR_MIN  INT64_MIN
#define INTPTR_MAX  INT64_MAX
#define UINTPTR_MAX UINT64_MAX
#define PTRDIFF_MIN INT64_MIN
#define PTRDIFF_MAX INT64_MAX
#ifndef SIZE_MAX
#define SIZE_MAX    UINT64_MAX
#endif
#else
#define INTPTR_MIN  INT32_MIN
#define INTPTR_MAX  INT32_MAX
#define UINTPTR_MAX UINT32_MAX
#define PTRDIFF_MIN INT32_MIN
#define PTRDIFF_MAX INT32_MAX
#ifndef SIZE_MAX
#define SIZE_MAX    UINT32_MAX
#endif
#endif
#define SIG_ATOMIC_MIN INT32_MIN
#define SIG_ATOMIC_MAX INT32_MAX
#define WCHAR_MIN 0
#define WCHAR_MAX 0xffff
#define WINT_MIN  0
#define WINT_MAX  0xffff

#define INT8_C(x)    (x)
#define INT16_C(x)   (x)
#define INT32_C(x)   (x)
#define INT64_C(x)   (x##LL)
#define UINT8_C(x)   (x)
#define UINT16_C(x)  (x)
#define UINT32_C(x)  (x##U)
#define UINT64_C(x)  (x##ULL)
#define INTMAX_C(x)  INT64_C(x)
#define UINTMAX_C(x) UINT64_C(x)

#endif
