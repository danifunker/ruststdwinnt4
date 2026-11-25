#pragma pack(push, 1)
struct P { char c; int i; };
#pragma pack(pop)
typedef char check[sizeof(struct P) == 5 ? 1 : -1];
