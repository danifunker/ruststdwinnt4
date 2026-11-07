/* unistd.h for Windows NT 4.0's C run-time (ruststdwinnt4 crt/): the POSIX names msvcrt
   has under other headers (io.h, process.h, direct.h) and with a leading underscore. */
#ifndef NT4_UNISTD_H
#define NT4_UNISTD_H
#include <io.h>
#include <process.h>
#include <direct.h>
#include <stdlib.h>

#ifndef R_OK
#define F_OK 0
#define W_OK 2
#define R_OK 4
#define X_OK 0
#endif
#ifndef STDIN_FILENO
#define STDIN_FILENO  0
#define STDOUT_FILENO 1
#define STDERR_FILENO 2
#endif
#ifndef SEEK_SET
#define SEEK_SET 0
#define SEEK_CUR 1
#define SEEK_END 2
#endif
#ifndef _SSIZE_T_DEFINED
#define _SSIZE_T_DEFINED
typedef int ssize_t;
#endif

#define access   _access
#define close    _close
#define dup      _dup
#define dup2     _dup2
#define isatty   _isatty
#define lseek    _lseek
#define read     _read
#define write    _write
#define unlink   _unlink
#define rmdir    _rmdir
#define chdir    _chdir
#define getcwd   _getcwd
#define getpid   _getpid
#define swab     _swab
#endif
