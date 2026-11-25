typedef struct { void *p; void *m; } T;
T mk(void *p, void *m) { T rv = { p, m }; return rv; }
