typedef struct { char c; double d; } S;
int f(void) { return __alignof(S); }
