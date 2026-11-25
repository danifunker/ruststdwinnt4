__declspec(thread) int tls_value;
int f(void) { return ++tls_value; }
