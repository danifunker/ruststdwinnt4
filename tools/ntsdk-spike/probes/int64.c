typedef unsigned __int64 u64; typedef __int64 i64;
u64 mul(u64 a, u64 b) { return a * b; }
u64 divu(u64 a, u64 b) { return a / b + a % b; }
i64 divs(i64 a, i64 b) { return a / b + a % b; }
u64 shl(u64 a, int n) { return (a << n) | (a >> (64 - n)); }
u64 lit(void) { return 0xffffffffffffffffui64 + 0x123456789i64; }
