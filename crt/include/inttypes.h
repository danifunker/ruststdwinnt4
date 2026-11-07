/* inttypes.h for Windows NT 4.0's C run-time (ruststdwinnt4 crt/). The 64-bit formats
   are written the C99 way (ll); nt4c99.h's printf family translates them for NT 4.0's
   msvcrt, which knows only I64. */
#ifndef NT4_INTTYPES_H
#define NT4_INTTYPES_H
#include <stdint.h>

#define PRId8 "d"
#define PRId16 "d"
#define PRId32 "d"
#define PRId64 "lld"
#define PRIi8 "i"
#define PRIi16 "i"
#define PRIi32 "i"
#define PRIi64 "lli"
#define PRIu8 "u"
#define PRIu16 "u"
#define PRIu32 "u"
#define PRIu64 "llu"
#define PRIx8 "x"
#define PRIx16 "x"
#define PRIx32 "x"
#define PRIx64 "llx"
#define PRIX8 "X"
#define PRIX16 "X"
#define PRIX32 "X"
#define PRIX64 "llX"
#define PRIo8 "o"
#define PRIo16 "o"
#define PRIo32 "o"
#define PRIo64 "llo"
#define PRIdMAX "lld"
#define PRIiMAX "lli"
#define PRIuMAX "llu"
#define PRIxMAX "llx"
#define PRIXMAX "llX"
#define PRIoMAX "llo"
#ifdef _WIN64
#define PRIdPTR "lld"
#define PRIiPTR "lli"
#define PRIuPTR "llu"
#define PRIxPTR "llx"
#define PRIXPTR "llX"
#define PRIoPTR "llo"
#else
#define PRIdPTR "d"
#define PRIiPTR "i"
#define PRIuPTR "u"
#define PRIxPTR "x"
#define PRIXPTR "X"
#define PRIoPTR "o"
#endif
#define PRIdLEAST64 PRId64
#define PRIuLEAST64 PRIu64
#define PRIxLEAST64 PRIx64
#define PRIdFAST64 PRId64
#define PRIuFAST64 PRIu64
#define PRIxFAST64 PRIx64
#define PRIdLEAST32 PRId32
#define PRIuLEAST32 PRIu32
#define PRIxLEAST32 PRIx32
#define PRIdFAST32 PRId32
#define PRIuFAST32 PRIu32
#define PRIxFAST32 PRIx32

intmax_t strtoimax(const char *s, char **end, int base);
uintmax_t strtoumax(const char *s, char **end, int base);
intmax_t imaxabs(intmax_t x);
#endif
