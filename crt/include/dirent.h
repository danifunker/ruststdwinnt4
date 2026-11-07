/* dirent.h for Windows NT 4.0 (ruststdwinnt4 crt/): POSIX directory reading on
   FindFirstFileA / FindNextFileA (and the wide forms on the W functions), from
   nt4c99.lib. Narrow names are in the ANSI code page. */
#ifndef NT4_DIRENT_H
#define NT4_DIRENT_H
#ifdef __cplusplus
extern "C" {
#endif

#define DT_UNKNOWN 0
#define DT_DIR     4
#define DT_REG     8
#define DT_LNK     10            /* never returned: NT 4.0 has no symbolic links */
#define _DIRENT_HAVE_D_TYPE 1

struct dirent {
    unsigned long d_ino;          /* always 0 */
    unsigned char d_type;         /* DT_DIR or DT_REG */
    unsigned short d_namlen;
    char d_name[260];
};

typedef struct nt4_dir DIR;

DIR *opendir(const char *path);
struct dirent *readdir(DIR *dir);
int closedir(DIR *dir);
void rewinddir(DIR *dir);

/* The wide forms (FindFirstFileW), and the _T names for them under _UNICODE, as MinGW's
   dirent.h has them. */
struct _wdirent {
    unsigned long d_ino;
    unsigned char d_type;
    unsigned short d_namlen;      /* in characters */
    unsigned short d_name[260];
};
typedef struct nt4_wdir _WDIR;
_WDIR *_wopendir(const unsigned short *path);
struct _wdirent *_wreaddir(_WDIR *dir);
int _wclosedir(_WDIR *dir);
void _wrewinddir(_WDIR *dir);

#ifdef _UNICODE
#define _TDIR       _WDIR
#define _tdirent    _wdirent
#define _topendir   _wopendir
#define _treaddir   _wreaddir
#define _tclosedir  _wclosedir
#define _trewinddir _wrewinddir
#else
#define _TDIR       DIR
#define _tdirent    dirent
#define _topendir   opendir
#define _treaddir   readdir
#define _tclosedir  closedir
#define _trewinddir rewinddir
#endif

#ifdef __cplusplus
}
#endif
#endif
