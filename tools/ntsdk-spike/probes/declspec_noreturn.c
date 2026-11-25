void abort(void);
__declspec(noreturn) static void die(void) { abort(); }
int f(int x) { if (x) die(); return x; }
