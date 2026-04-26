/* Mott math runtime — implementations for the `math` stdlib module.
 *
 * Each function delegates to libm. Linker pulls in `-lm` (driver passes
 * the flag automatically when math is in the import graph).
 *
 * Naming: `mott_math_<name>` to match the codegen mangling for
 * module-qualified calls (`math.sqrt` -> `mott_math_sqrt`). Without the
 * prefix we'd collide with libc's `sqrt`, `pow`, `sin`, etc. */

#include <math.h>
#include <stdint.h>
#include <stdlib.h>

double mott_math_sqrt(double x) { return sqrt(x); }
double mott_math_pow(double base, double exp) { return pow(base, exp); }
double mott_math_exp(double x) { return exp(x); }

int64_t mott_math_abs_terah(int64_t x) { return llabs(x); }
double  mott_math_abs_daqosh(double x) { return fabs(x); }

double mott_math_floor(double x) { return floor(x); }
double mott_math_ceil(double x)  { return ceil(x); }
double mott_math_round(double x) { return round(x); }

double mott_math_sin(double x) { return sin(x); }
double mott_math_cos(double x) { return cos(x); }
double mott_math_tan(double x) { return tan(x); }

double mott_math_log(double x)   { return log(x); }
double mott_math_log2(double x)  { return log2(x); }
double mott_math_log10(double x) { return log10(x); }

/* Constants as 0-arg functions because Mott has no `const` keyword yet.
 * libm's M_PI / M_E live behind feature flags on some toolchains, so
 * we just spell them out — same precision as POSIX values. */
double mott_math_pi(void) { return 3.14159265358979323846; }
double mott_math_e(void)  { return 2.71828182845904523536; }
