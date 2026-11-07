/* Checks of nt4c99 (ruststdwinnt4 crt/) on NT 4.0: built with tools/sdk/nt4-gcc. */
#include <stdio.h>
#include <string.h>
#include <stdint.h>
#include <inttypes.h>
#include <stdbool.h>
#include <stdlib.h>
#include <errno.h>
#include <math.h>

static int failures;
static void check(const char *name, bool ok, const char *got)
{
    printf("  %-28s %s%s\n", name, ok ? "ok" : "FAILED: ", ok ? "" : got);
    if (!ok)
        failures++;
}

int main(void)
{
    char b[64], small[8];
    int n;
    int64_t big = INT64_C(-1234567890123);
    uint64_t ubig = UINT64_MAX;
    size_t sz = 4000000000u;

    snprintf(b, sizeof b, "%lld|%" PRIu64 "|%zu", (long long)big, ubig, sz);
    check("ll, PRIu64, z", strcmp(b, "-1234567890123|18446744073709551615|4000000000") == 0, b);
    n = snprintf(small, sizeof small, "%s", "truncated text");
    check("snprintf truncates", n == 14 && strcmp(small, "truncat") == 0, small);
    n = snprintf(NULL, 0, "%d-%d", 12, 345);
    check("snprintf length only", n == 6, "");
    check("strtoll", strtoll("  -9223372036854775807", NULL, 10) == -INT64_MAX, "");
    check("strtoull hex", strtoull("0xFFFFFFFFFFFFFFFF", NULL, 0) == UINT64_MAX, "");
    errno = 0;
    strtoll("99999999999999999999", NULL, 10);
    check("strtoll overflow", errno == ERANGE, "");
    check("isnan, isinf", isnan(NAN) && isinf(INFINITY) && isfinite(1.5), "");
    sprintf(b, "%" PRIx64 "%s", (uint64_t)0xDEADBEEFCAFEull, "!");
    check("sprintf PRIx64", strcmp(b, "deadbeefcafe!") == 0, b);
    printf("c99test: %s\n", failures ? "FAILED" : "ok");
    return failures;
}
