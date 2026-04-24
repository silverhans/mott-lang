/* Mott runtime implementation. See mott_rt.h for the contract. */

#include "mott_rt.h"

#include <inttypes.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* --- yazde (print) ---------------------------------------------------- */

void mott_yazde_terah(int64_t v) {
    printf("%" PRId64 "\n", v);
}

void mott_yazde_daqosh(double v) {
    /* %g drops trailing zeros; "17 significant digits" is enough to
     * round-trip an IEEE-754 double. */
    printf("%.17g\n", v);
}

void mott_yazde_bool(bool v) {
    /* Print the Mott source literal so round-tripping is obvious. */
    fputs(v ? "baqderg\n" : "xarco\n", stdout);
}

void mott_yazde_deshnash(mott_str s) {
    if (s.len > 0) {
        fwrite(s.data, 1, s.len, stdout);
    }
    fputc('\n', stdout);
}

/* --- conversions used by interpolation -------------------------------- */

static mott_str mott__dup_cstr(const char *buf, size_t len) {
    char *data = (char *)malloc(len + 1);
    if (!data) {
        fputs("mott runtime: out of memory\n", stderr);
        abort();
    }
    memcpy(data, buf, len);
    data[len] = '\0';
    return (mott_str){ .data = data, .len = len };
}

mott_str mott_str_from_terah(int64_t v) {
    char buf[32];
    int n = snprintf(buf, sizeof(buf), "%" PRId64, v);
    if (n < 0) {
        return (mott_str){ .data = "", .len = 0 };
    }
    return mott__dup_cstr(buf, (size_t)n);
}

mott_str mott_str_from_daqosh(double v) {
    char buf[64];
    int n = snprintf(buf, sizeof(buf), "%.17g", v);
    if (n < 0) {
        return (mott_str){ .data = "", .len = 0 };
    }
    return mott__dup_cstr(buf, (size_t)n);
}

mott_str mott_str_from_bool(bool v) {
    /* Literals live in .rodata — no allocation needed. */
    return v ? MOTT_STR_LIT("baqderg") : MOTT_STR_LIT("xarco");
}

/* --- interpolation build --------------------------------------------- */

bool mott_str_eq(mott_str a, mott_str b) {
    if (a.len != b.len) {
        return false;
    }
    if (a.len == 0) {
        return true;
    }
    /* memcmp is UB with NULL even for length 0 — the guard above covers it. */
    return memcmp(a.data, b.data, a.len) == 0;
}

mott_str mott_str_build(const mott_str *parts, size_t n) {
    size_t total = 0;
    for (size_t i = 0; i < n; i++) {
        total += parts[i].len;
    }
    char *data = (char *)malloc(total + 1);
    if (!data) {
        fputs("mott runtime: out of memory\n", stderr);
        abort();
    }
    size_t off = 0;
    for (size_t i = 0; i < n; i++) {
        if (parts[i].len > 0) {
            memcpy(data + off, parts[i].data, parts[i].len);
            off += parts[i].len;
        }
    }
    data[total] = '\0';
    return (mott_str){ .data = data, .len = total };
}
