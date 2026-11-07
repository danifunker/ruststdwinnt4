/* C compiled by PINT's SDK (Microsoft C for ARM, NT 4.0 headers) inside a Rust program:
   the C run-time (msvcrt.dll), a callback into Rust, and data both sides share. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include <time.h>

typedef int (*rust_cb)(int);

static int cmp(const void *a, const void *b) { return *(const int *)a - *(const int *)b; }

int c_sort_sum(int *v, int n) {
    int i, s = 0;
    qsort(v, n, sizeof *v, cmp);
    for (i = 0; i < n; i++) s += v[i];
    return s;
}

int c_format(char *out, int size, double x, const char *word) {
    char *dup = _strdup(word);
    int n = _snprintf(out, size, "%s:%.4f:%ld", strupr(dup), sqrt(x), strtol("12345", NULL, 10));
    free(dup);
    return n;
}

int c_call_back(rust_cb f, int x) { return f(x) * 2; }

long c_time(void) { return (long)time(NULL); }

/* A C initializer, as GnuTLS registers its library constructor with Microsoft C: the
   program's entry point (nt4compat) must run it before main. */
static int initialized;
static void init_mix(void) { initialized = 42; }
#pragma section(".CRT$XCU", read)
__declspec(allocate(".CRT$XCU")) void (*const init_mix_entry)(void) = init_mix;

int c_initialized(void) { return initialized; }
