/* The start of a C program on x86 Windows NT 4.0 (ruststdwinnt4 crt/, linked by
   tools/x86/nt4-gcc): what Microsoft's msvcrt.lib adds to a program on ARM, where PINT's
   SDK has one, and what nt4compat's crt.rs gives a Rust program. Not in nt4c99.lib, so a
   Rust program, which has its own, never takes it.

   mainCRTStartup runs the initializers of the C and C++ code linked in (the .CRT$XI? and
   .CRT$XC? tables, between our A and Z entries; the linker sorts the sections by name),
   then main with msvcrt.dll's arguments and environment, and exits with its result. The
   TLS directory makes thread-locals (_Thread_local, __declspec(thread)) work in the
   program's image, as the NT 4.0 loader supports there. */

#include <stdlib.h>

typedef int (__cdecl *c_init)(void);
typedef void (__cdecl *cpp_init)(void);
typedef void (__stdcall *tls_callback)(void *, unsigned long, void *);

#pragma section(".CRT$XIA", long, read)
#pragma section(".CRT$XIZ", long, read)
#pragma section(".CRT$XCA", long, read)
#pragma section(".CRT$XCZ", long, read)
#pragma section(".CRT$XLA", long, read)
#pragma section(".CRT$XLZ", long, read)
__declspec(allocate(".CRT$XIA")) static c_init xi_a = 0;
__declspec(allocate(".CRT$XIZ")) static c_init xi_z = 0;
__declspec(allocate(".CRT$XCA")) static cpp_init xc_a = 0;
__declspec(allocate(".CRT$XCZ")) static cpp_init xc_z = 0;
__declspec(allocate(".CRT$XLA")) static tls_callback xl_a = 0;
__declspec(allocate(".CRT$XLZ")) static tls_callback xl_z = 0;

/* Referenced by every object that uses floating point; nothing to pull in here. */
int _fltused = 0x9875;

/* The TLS directory (IMAGE_TLS_DIRECTORY32), which the linker points the image at by
   this name, and the bounds of the TLS data template (.tls$ sections sort between). */
unsigned long _tls_index = 0;
#pragma data_seg(".tls")
char _tls_start = 0;
#pragma data_seg(".tls$ZZZ")
char _tls_end = 0;
#pragma data_seg()
const struct {
    void *start, *end, *index, *callbacks;
    unsigned long zero_fill, characteristics;
} _tls_used = { &_tls_start, &_tls_end, &_tls_index, &xl_a + 1, 0, 0 };

/* Microsoft C's x86 code reaches thread-locals through fs:[_tls_array]: the offset of the
   TEB's ThreadLocalStoragePointer. */
__asm__(".globl __tls_array\n.set __tls_array, 0x2C");

int main(int, char **, char **);
void __cdecl __getmainargs(int *, char ***, char ***, int, int *);
void __cdecl __set_app_type(int);

void mainCRTStartup(void)
{
    int argc, startinfo = 0;
    char **argv, **envp;
    const volatile c_init *i;
    const volatile cpp_init *c;

    /* The XLZ entry ends the TLS callback list: referenced, so the linker keeps it. */
    (void)*(const volatile tls_callback *)&xl_z;
    __set_app_type(1); /* a console program */
    __getmainargs(&argc, &argv, &envp, 0, &startinfo);
    for (i = &xi_a + 1; i < &xi_z; i++)
        if (*i && (*i)() != 0)
            exit(255);
    for (c = &xc_a + 1; c < &xc_z; c++)
        if (*c)
            (*c)();
    exit(main(argc, argv, envp));
}
