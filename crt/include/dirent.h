/* dirent.h for Windows NT 4.0 (ruststdwinnt4 crt/): POSIX directory reading on
   FindFirstFileA / FindNextFileA, from nt4c99.lib. Names are in the ANSI code page. */
#ifndef NT4_DIRENT_H
#define NT4_DIRENT_H
#ifdef __cplusplus
extern "C" {
#endif

#define DT_UNKNOWN 0
#define DT_DIR     4
#define DT_REG     8

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

#ifdef __cplusplus
}
#endif
#endif
