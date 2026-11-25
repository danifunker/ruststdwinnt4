#include <math.h>
double f(double x) { return fabs(x) + floor(x) + ceil(x) + sqrt(x) + fmod(x, 2.0); }
float g(float x) { return fabsf(x); }
