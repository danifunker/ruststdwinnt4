/* stdint.h for Visual C++ 4.1, which has none (and no long long): what mrustc's C uses. */
#ifndef NT4_STDINT_H
#define NT4_STDINT_H
typedef signed char int8_t;       typedef unsigned char uint8_t;
typedef short int16_t;            typedef unsigned short uint16_t;
typedef int int32_t;              typedef unsigned int uint32_t;
typedef __int64 int64_t;          typedef unsigned __int64 uint64_t;
typedef int intptr_t;             typedef unsigned int uintptr_t;
#define INT8_MIN (-128)
#define INT8_MAX 127
#define UINT8_MAX 255
#define INT16_MIN (-32767-1)
#define INT16_MAX 32767
#define UINT16_MAX 65535
#define INT32_MIN (-2147483647-1)
#define INT32_MAX 2147483647
#define UINT32_MAX 0xffffffffu
#define INT64_MIN (-9223372036854775807i64-1)
#define INT64_MAX 9223372036854775807i64
#define UINT64_MAX 0xffffffffffffffffui64
#define SIZE_MAX UINT32_MAX
#endif
