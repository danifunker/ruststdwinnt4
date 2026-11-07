/* POSIX directory reading for Windows NT 4.0 (ruststdwinnt4 crt/, include/dirent.h). */
#include <windows.h>
#include <errno.h>
#include <stdlib.h>
#include <string.h>
#include "../include/dirent.h"

struct nt4_dir {
    HANDLE find;
    int first;          /* data holds an entry not returned yet */
    WIN32_FIND_DATAA data;
    struct dirent entry;
    char pattern[MAX_PATH + 3];
};

static int to_errno(DWORD e)
{
    switch (e) {
    case ERROR_FILE_NOT_FOUND:
    case ERROR_PATH_NOT_FOUND: return ENOENT;
    case ERROR_ACCESS_DENIED: return EACCES;
    case ERROR_DIRECTORY: return ENOTDIR;
    case ERROR_NOT_ENOUGH_MEMORY: return ENOMEM;
    default: return EIO;
    }
}

static int start(DIR *d)
{
    d->find = FindFirstFileA(d->pattern, &d->data);
    if (d->find == INVALID_HANDLE_VALUE) {
        DWORD e = GetLastError();
        if (e == ERROR_NO_MORE_FILES || e == ERROR_FILE_NOT_FOUND) {
            d->first = 0;    /* an empty directory */
            return 0;
        }
        errno = to_errno(e);
        return -1;
    }
    d->first = 1;
    return 0;
}

DIR *opendir(const char *path)
{
    size_t n = strlen(path);
    DWORD attr;
    DIR *d;
    if (n == 0 || n > MAX_PATH) {
        errno = n ? ENAMETOOLONG : ENOENT;
        return NULL;
    }
    attr = GetFileAttributesA(path);
    if (attr == 0xFFFFFFFF) {
        errno = to_errno(GetLastError());
        return NULL;
    }
    if (!(attr & FILE_ATTRIBUTE_DIRECTORY)) {
        errno = ENOTDIR;
        return NULL;
    }
    d = calloc(1, sizeof *d);
    if (!d) {
        errno = ENOMEM;
        return NULL;
    }
    memcpy(d->pattern, path, n);
    if (path[n - 1] != '\\' && path[n - 1] != '/')
        d->pattern[n++] = '\\';
    d->pattern[n++] = '*';
    d->pattern[n] = 0;
    if (start(d) < 0) {
        free(d);
        return NULL;
    }
    return d;
}

struct dirent *readdir(DIR *d)
{
    if (!d->first) {
        if (d->find == INVALID_HANDLE_VALUE || !FindNextFileA(d->find, &d->data))
            return NULL;    /* the end (errno unchanged) */
    }
    d->first = 0;
    strncpy(d->entry.d_name, d->data.cFileName, sizeof d->entry.d_name - 1);
    d->entry.d_name[sizeof d->entry.d_name - 1] = 0;
    d->entry.d_namlen = (unsigned short)strlen(d->entry.d_name);
    d->entry.d_type = (d->data.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) ? DT_DIR : DT_REG;
    d->entry.d_ino = 0;
    return &d->entry;
}

void rewinddir(DIR *d)
{
    if (d->find != INVALID_HANDLE_VALUE)
        FindClose(d->find);
    start(d);
}

int closedir(DIR *d)
{
    if (d->find != INVALID_HANDLE_VALUE)
        FindClose(d->find);
    free(d);
    return 0;
}

/* ---- The wide forms ------------------------------------------------------------------- */

struct nt4_wdir {
    HANDLE find;
    int first;
    WIN32_FIND_DATAW data;
    struct _wdirent entry;
    WCHAR pattern[MAX_PATH + 3];
};

static int wstart(_WDIR *d)
{
    d->find = FindFirstFileW(d->pattern, &d->data);
    if (d->find == INVALID_HANDLE_VALUE) {
        DWORD e = GetLastError();
        if (e == ERROR_NO_MORE_FILES || e == ERROR_FILE_NOT_FOUND) {
            d->first = 0;
            return 0;
        }
        errno = to_errno(e);
        return -1;
    }
    d->first = 1;
    return 0;
}

_WDIR *_wopendir(const unsigned short *path)
{
    size_t n = wcslen((const wchar_t *)path);
    DWORD attr;
    _WDIR *d;
    if (n == 0 || n > MAX_PATH) {
        errno = n ? ENAMETOOLONG : ENOENT;
        return NULL;
    }
    attr = GetFileAttributesW((LPCWSTR)path);
    if (attr == 0xFFFFFFFF) {
        errno = to_errno(GetLastError());
        return NULL;
    }
    if (!(attr & FILE_ATTRIBUTE_DIRECTORY)) {
        errno = ENOTDIR;
        return NULL;
    }
    d = calloc(1, sizeof *d);
    if (!d) {
        errno = ENOMEM;
        return NULL;
    }
    memcpy(d->pattern, path, n * sizeof(WCHAR));
    if (path[n - 1] != '\\' && path[n - 1] != '/')
        d->pattern[n++] = '\\';
    d->pattern[n++] = '*';
    d->pattern[n] = 0;
    if (wstart(d) < 0) {
        free(d);
        return NULL;
    }
    return d;
}

struct _wdirent *_wreaddir(_WDIR *d)
{
    size_t n;
    if (!d->first) {
        if (d->find == INVALID_HANDLE_VALUE || !FindNextFileW(d->find, &d->data))
            return NULL;
    }
    d->first = 0;
    n = wcslen(d->data.cFileName);
    if (n >= 260)
        n = 259;
    memcpy(d->entry.d_name, d->data.cFileName, n * sizeof(WCHAR));
    d->entry.d_name[n] = 0;
    d->entry.d_namlen = (unsigned short)n;
    d->entry.d_type = (d->data.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) ? DT_DIR : DT_REG;
    d->entry.d_ino = 0;
    return &d->entry;
}

void _wrewinddir(_WDIR *d)
{
    if (d->find != INVALID_HANDLE_VALUE)
        FindClose(d->find);
    wstart(d);
}

int _wclosedir(_WDIR *d)
{
    if (d->find != INVALID_HANDLE_VALUE)
        FindClose(d->find);
    free(d);
    return 0;
}
