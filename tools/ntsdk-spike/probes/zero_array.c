struct S { int len; char data[0]; };
union U { struct S s; unsigned char raw[0]; };
union U u;
