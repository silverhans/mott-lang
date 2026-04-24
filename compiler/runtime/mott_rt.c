/* Mott runtime implementation. See mott_rt.h for the contract. */

#include "mott_rt.h"

#include <errno.h>
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

/* --- Arrays --- */

#define MOTT_DEFINE_ARR_NEW(name, elem_t)                                      \
    mott_arr_##name mott_arr_##name##_new(size_t n, const elem_t *src) {       \
        elem_t *data = NULL;                                                   \
        if (n > 0) {                                                           \
            data = (elem_t *)malloc(n * sizeof(elem_t));                       \
            if (!data) {                                                       \
                fputs("mott runtime: out of memory\n", stderr);                \
                abort();                                                       \
            }                                                                  \
            memcpy(data, src, n * sizeof(elem_t));                             \
        }                                                                      \
        return (mott_arr_##name){ .data = data, .len = n };                    \
    }

MOTT_DEFINE_ARR_NEW(terah,    int64_t)
MOTT_DEFINE_ARR_NEW(daqosh,   double)
MOTT_DEFINE_ARR_NEW(bool,     bool)
MOTT_DEFINE_ARR_NEW(deshnash, mott_str)

#undef MOTT_DEFINE_ARR_NEW

mott_str mott_input(void) {
    /* getline allocates and grows as needed — standard POSIX 2008 since
     * macOS 10.7 and Linux glibc forever. Perfect fit for unknown-length
     * lines. Ownership of `line` transfers to the returned mott_str. */
    char *line = NULL;
    size_t cap = 0;
    ssize_t n = getline(&line, &cap, stdin);
    if (n < 0) {
        /* EOF or read error — return a safe empty string, free any buffer
         * getline may have allocated. */
        free(line);
        return (mott_str){ .data = "", .len = 0 };
    }
    /* Strip a single trailing '\n' (and a preceding '\r' if present) so
     * `yazde(esha())` doesn't double-space. */
    size_t len = (size_t)n;
    if (len > 0 && line[len - 1] == '\n') {
        len--;
        if (len > 0 && line[len - 1] == '\r') {
            len--;
        }
        line[len] = '\0';
    }
    return (mott_str){ .data = line, .len = len };
}

/* --- Number parsing --- */

/* Fatal error used by the parse helpers. Prints the offending input so the
 * user can see what they actually got — handy when the string came from
 * esha() / a file / stdin and isn't obvious from the source. */
static void mott__parse_fatal(const char *what, mott_str s) {
    fprintf(stderr, "mott runtime: %s: '%.*s'\n",
            what, (int)s.len, s.data);
    abort();
}

int64_t mott_parse_terah(mott_str s) {
    if (s.len == 0) {
        mott__parse_fatal("parse_terah: empty string", s);
    }
    /* All Mott strings are NUL-terminated exactly at s.data[s.len]; strtoll
     * stops at the first non-digit anyway, so we check `end` below to
     * ensure the entire input was consumed (modulo leading whitespace). */
    char *end = NULL;
    errno = 0;
    long long v = strtoll(s.data, &end, 10);
    if (errno == ERANGE) {
        mott__parse_fatal("parse_terah: out of range", s);
    }
    if (end != s.data + s.len) {
        mott__parse_fatal("parse_terah: not a valid integer", s);
    }
    return (int64_t)v;
}

double mott_parse_daqosh(mott_str s) {
    if (s.len == 0) {
        mott__parse_fatal("parse_daqosh: empty string", s);
    }
    char *end = NULL;
    errno = 0;
    double v = strtod(s.data, &end);
    if (errno == ERANGE) {
        mott__parse_fatal("parse_daqosh: out of range", s);
    }
    if (end != s.data + s.len) {
        mott__parse_fatal("parse_daqosh: not a valid float", s);
    }
    return v;
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
