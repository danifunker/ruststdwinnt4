/* NaN through compare, convert and arithmetic: does it trap (Alpha needs IEEE mode)? */
#include <stdio.h>
#include <string.h>
int main(void) {
    unsigned __int64 b = 0x7FF8000000000000ui64; double n, z = 0.0; volatile double v;
    memcpy(&n, &b, 8); v = n;
    printf("compare %d\n", v != v);
    printf("add %d\n", (v + 1.0) != (v + 1.0));
    printf("div0 %d\n", (1.0 / (v * z + z)) > 0.0);
    printf("nan: ok\n");
    return 0;
}
