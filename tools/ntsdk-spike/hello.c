/* Step 1 of the spike: does ntsdk's cl build a console program, and does ntrun run it? */
#include <stdio.h>
int main(int argc, char **argv)
{
    __int64 x = 0x123456789i64;
    printf("hello from %s, sizeof(void*)=%d sizeof(long)=%d i64=%I64x argc=%d\n",
#if defined(_M_MRX000)
           "MIPS",
#elif defined(_M_ALPHA)
           "Alpha",
#elif defined(_M_PPC)
           "PowerPC",
#else
           "x86",
#endif
           (int)sizeof(void *), (int)sizeof(long), x, argc);
    return 0;
}
