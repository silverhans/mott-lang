/* Mott runtime — linked into every compiled .mott program.
 *
 * Keep this header self-contained and minimal: the C backend emits code
 * that refers to these symbols directly, so changing anything here is an
 * ABI break for already-generated .c files.
 *
 * Memory model for MVP: string buffers produced by the formatting helpers
 * and mott_str_build are malloc()'d and never freed. That's deliberate —
 * real ownership comes in v0.2 along with sema.
 */

#ifndef MOTT_RT_H
#define MOTT_RT_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

/* UTF-8 string view. `data` need not be NUL-terminated (but the helpers
 * below always allocate NUL-terminated buffers for convenience). */
typedef struct {
    const char *data;
    size_t len;
} mott_str;

/* Construct a mott_str from a string literal. `sizeof(s) - 1` works because
 * (s) is always a char array, never a pointer — the backend only uses this
 * macro on literal expressions. */
#define MOTT_STR_LIT(s) ((mott_str){ .data = (s), .len = sizeof(s) - 1 })

/* Per-type yazde (print) helpers — one per Mott primitive. All append '\n'. */
void mott_yazde_terah(int64_t v);
void mott_yazde_daqosh(double v);
void mott_yazde_bool(bool v);
void mott_yazde_deshnash(mott_str s);

/* String conversions used by interpolation. Each returns a heap-allocated
 * buffer (leaked in MVP). */
mott_str mott_str_from_terah(int64_t v);
mott_str mott_str_from_daqosh(double v);
mott_str mott_str_from_bool(bool v);

/* Concatenate `n` parts into a single freshly-allocated mott_str. The backend
 * lowers interpolation like "x = {x}" into a call to this function with a
 * compound-literal array. */
mott_str mott_str_build(const mott_str *parts, size_t n);

/* Byte-level equality. `==` and `!=` on `deshnash` lower to this (NOT to
 * C's `==`, which would compare struct fields and usually miscompare). */
bool mott_str_eq(mott_str a, mott_str b);

/* Read one line from stdin. Returns a heap-allocated mott_str with the
 * trailing newline stripped. On EOF or read error, returns an empty
 * mott_str (zero length, static data pointer). Leaks in MVP. */
mott_str mott_input(void);

/* Parse a deshnash into a terah/daqosh.
 *
 * Semantics: leading whitespace is tolerated (strtoll/strtod habit),
 * everything else must be a well-formed number — trailing garbage, empty
 * string, or overflow all trigger a fatal runtime error with `abort()`.
 * When we gain a Result/Option type these will flip to non-fatal.
 *
 * Input must be NUL-terminated at s.data[s.len] (which every Mott string
 * is — see notes on mott_str above). Callers should never construct
 * mott_str values that violate this invariant. */
int64_t mott_parse_terah(mott_str s);
double  mott_parse_daqosh(mott_str s);

/* --- Arrays ------------------------------------------------------------
 *
 * One struct per element type (until we get generics). All share the same
 * layout — `data` pointer followed by `len` — so `baram(arr)` in the
 * language always lowers to `arr.len` regardless of element type.
 *
 * Arrays in MVP are "fixed after creation": length is set when the literal
 * is evaluated and never changes. No push/pop, no grow. Element mutation
 * via `arr[i] = x` is allowed.
 *
 * Memory: the `*_new` helpers malloc a buffer, memcpy the provided
 * elements in, and return a struct pointing at the buffer. Leaked in MVP,
 * same policy as strings and mott_str_build. */

typedef struct { int64_t  *data; size_t len; } mott_arr_terah;
typedef struct { double   *data; size_t len; } mott_arr_daqosh;
typedef struct { bool     *data; size_t len; } mott_arr_bool;
typedef struct { mott_str *data; size_t len; } mott_arr_deshnash;

mott_arr_terah    mott_arr_terah_new   (size_t n, const int64_t  *src);
mott_arr_daqosh   mott_arr_daqosh_new  (size_t n, const double   *src);
mott_arr_bool     mott_arr_bool_new    (size_t n, const bool     *src);
mott_arr_deshnash mott_arr_deshnash_new(size_t n, const mott_str *src);

#endif /* MOTT_RT_H */
