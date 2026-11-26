/* CreateThread on Alpha NT 4.0 the ways std calls it: does the new thread start? */
#include <windows.h>
#include <stdio.h>
static DWORD WINAPI run(void *p) { printf("  thread %d running\n", (int)p); fflush(stdout); return (DWORD)p; }
static void try(int n, DWORD stack, DWORD flags) {
    DWORD code = 0, id;
    HANDLE h;
    printf("%d: stack %lu flags %lx\n", n, stack, flags); fflush(stdout);
    h = CreateThread(0, stack, run, (void *)n, flags, &id);
    if (!h) { printf("  CreateThread failed %lu\n", GetLastError()); return; }
    WaitForSingleObject(h, INFINITE);
    GetExitCodeThread(h, &code);
    printf("  exit %lx\n", code); fflush(stdout);
    CloseHandle(h);
}
int main(void) {
    try(1, 0, 0);
    try(2, 2097152, 0);
    try(3, 0, 0x10000);
    try(4, 2097152, 0x10000);
    printf("thread: done\n");
    return 0;
}
