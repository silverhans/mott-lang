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

#endif /* MOTT_RT_H */
