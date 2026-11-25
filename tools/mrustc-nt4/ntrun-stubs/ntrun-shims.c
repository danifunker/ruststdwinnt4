/* ntrun-shims.c: functions of NT 4.0's KERNEL32 and NTDLL that std calls and ntrun (tenox7/ntsdk's
 * emulator) doesn't have, made of ones it has. Linked ahead of the import libraries into the test
 * programs build-std.sh runs under ntrun, and nowhere else: on NT 4.0 the real ones are used.
 * Compiled by Visual C++ 4.1 (C89). */

typedef unsigned long DWORD;
typedef int BOOL;
typedef void *HANDLE;
typedef long NTSTATUS;
typedef struct { NTSTATUS Status; DWORD Information; } IO_STATUS_BLOCK;

__declspec(dllimport) BOOL __stdcall WriteFile(HANDLE, const void *, DWORD, DWORD *, void *);
__declspec(dllimport) BOOL __stdcall ReadFile(HANDLE, void *, DWORD, DWORD *, void *);
__declspec(dllimport) DWORD __stdcall SetFilePointer(HANDLE, long, long *, DWORD);
__declspec(dllimport) DWORD __stdcall GetLastError(void);
__declspec(dllimport) void __stdcall SetLastError(DWORD);
__declspec(dllimport) void __stdcall Sleep(DWORD);
__declspec(dllimport) unsigned int __stdcall GetSystemDirectoryA(char *, unsigned int);
__declspec(dllimport) unsigned int __stdcall GetWindowsDirectoryA(char *, unsigned int);
__declspec(dllimport) int __stdcall MultiByteToWideChar(unsigned int, DWORD, const char *, int, unsigned short *, int);

#define STATUS_SUCCESS 0L
#define STATUS_END_OF_FILE ((NTSTATUS)0xC0000011L)
#define STATUS_PIPE_BROKEN ((NTSTATUS)0xC000014BL)
#define STATUS_UNSUCCESSFUL ((NTSTATUS)0xC0000001L)
#define ERROR_HANDLE_EOF 38
#define ERROR_BROKEN_PIPE 109

static NTSTATUS from_error(DWORD e)
{
    if (e == ERROR_HANDLE_EOF) return STATUS_END_OF_FILE;
    if (e == ERROR_BROKEN_PIPE) return STATUS_PIPE_BROKEN;
    return STATUS_UNSUCCESSFUL;
}

/* An offset of -2 (FILE_USE_FILE_POINTER_POSITION) or none: where the file is */
static void seek(HANDLE h, const __int64 *offset)
{
    long high;
    if (offset == 0 || *offset == -2) return;
    high = (long)(*offset >> 32);
    SetFilePointer(h, (long)*offset, &high, 0);
}

NTSTATUS __stdcall NtWriteFile(HANDLE h, HANDLE event, void *apc, void *ctx, IO_STATUS_BLOCK *iosb,
                               const void *buf, DWORD len, const __int64 *offset, DWORD *key)
{
    DWORD n = 0;
    seek(h, offset);
    if (!WriteFile(h, buf, len, &n, 0)) {
        iosb->Status = from_error(GetLastError());
        iosb->Information = 0;
        return iosb->Status;
    }
    iosb->Status = STATUS_SUCCESS;
    iosb->Information = n;
    return STATUS_SUCCESS;
}

NTSTATUS __stdcall NtReadFile(HANDLE h, HANDLE event, void *apc, void *ctx, IO_STATUS_BLOCK *iosb,
                              void *buf, DWORD len, const __int64 *offset, DWORD *key)
{
    DWORD n = 0;
    seek(h, offset);
    if (!ReadFile(h, buf, len, &n, 0)) {
        iosb->Status = from_error(GetLastError());
        iosb->Information = 0;
        return iosb->Status;
    }
    iosb->Status = (n == 0 && len != 0) ? STATUS_END_OF_FILE : STATUS_SUCCESS;
    iosb->Information = n;
    return iosb->Status;
}

BOOL __stdcall SwitchToThread(void)
{
    Sleep(0);
    return 1;
}

static unsigned int widen(unsigned int n, const char *a, unsigned short *w, unsigned int size)
{
    if (n == 0 || n >= size) return n;
    return (unsigned int)MultiByteToWideChar(0, 0, a, -1, w, (int)size) - 1;
}

unsigned int __stdcall GetSystemDirectoryW(unsigned short *w, unsigned int size)
{
    char a[260];
    return widen(GetSystemDirectoryA(a, sizeof a), a, w, size);
}

unsigned int __stdcall GetWindowsDirectoryW(unsigned short *w, unsigned int size)
{
    char a[260];
    return widen(GetWindowsDirectoryA(a, sizeof a), a, w, size);
}

/* No times: callers (nt4compat's random seed) take what they get */
BOOL __stdcall GetProcessTimes(HANDLE p, void *c, void *e, void *k, void *u)
{
    SetLastError(120);
    return 0;
}

BOOL __stdcall GetThreadTimes(HANDLE t, void *c, void *e, void *k, void *u)
{
    SetLastError(120);
    return 0;
}
