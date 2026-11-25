#include <setjmp.h>
static jmp_buf jb;
int f(void) { if (setjmp(jb)) return 1; longjmp(jb, 1); return 0; }
