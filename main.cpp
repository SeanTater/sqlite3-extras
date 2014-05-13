/*
 * Written by Alexey Tourbin <at@altlinux.org>.
 *
 * The author has dedicated the code to the public domain.  Anyone is free
 * to copy, modify, publish, use, compile, sell, or distribute the original
 * code, either in source code form or as a compiled binary, for any purpose,
 * commercial or non-commercial, and by any means.
 */
#include <assert.h>
#include <stdlib.h>
#include <string.h>
#include <pcre.h>
#include <boost/regex.hpp>
#include <sqlite3ext.h>
SQLITE_EXTENSION_INIT1


/* Caching, for getPattern() */
typedef struct {
    char *s;
    pcre *p;
    pcre_extra *e;
} cache_entry;

#ifndef CACHE_SIZE
#define CACHE_SIZE 16
#endif

/* Captures, for sub()
 * See PCRE doc: slices are only the first 2/3 of the real space.
 */
#ifndef MAX_CAPTURES
#define MAX_CAPTURES 64
#endif

/*
 * Get a PCRE Pattern, first searching through a LRU cache
 */
static
void getPattern(sqlite3_context *ctx, const char *re, pcre **p, pcre_extra **e) {
    /* simple LRU cache */
    int i;
    int found = 0;
    cache_entry *cache = (cache_entry*) sqlite3_user_data(ctx);

    assert(cache);

    for (i = 0; i < CACHE_SIZE && cache[i].s; i++)
        if (strcmp(re, cache[i].s) == 0) {
            found = 1;
            break;
        }
    if (found) {
        if (i > 0) {
            cache_entry c = cache[i];
            memmove(cache + 1, cache, i * sizeof(cache_entry));
            cache[0] = c;
        }
    }
    else {
        cache_entry c;
        const char *err;
        int pos;
        c.p = pcre_compile(re, 0, &err, &pos, NULL);
        if (!c.p) {
            char *e2 = sqlite3_mprintf("%s: %s (offset %d)", re, err, pos);
            sqlite3_result_error(ctx, e2, -1);
            sqlite3_free(e2);
            return;
        }
        c.e = pcre_study(c.p, 0, &err);
        c.s = strdup(re);
        if (!c.s) {
            sqlite3_result_error(ctx, "strdup: ENOMEM", -1);
            pcre_free(c.p);
            pcre_free(c.e);
            return;
        }
        i = CACHE_SIZE - 1;
        if (cache[i].s) {
            free(cache[i].s);
            assert(cache[i].p);
            pcre_free(cache[i].p);
            pcre_free(cache[i].e);
        }
        memmove(cache + 1, cache, i * sizeof(cache_entry));
        cache[0] = c;
    }
    *p = cache[0].p;
    *e = cache[0].e;
}



/**
 * Create a replacement string, after pcre_exec is called.
 * It doesn't append it to where it would be in the output.
 * This is mostly to clarify code.
 *
 * @param format  Format string with \1 style backreferences
 * @param subject  String to make replaceme
 */
/*static
void formatReplacement(sqlite3_context *ctx, char *format, char *subject, int substring_count, int captures[]) {
    int fmtsize = strlen(format);
    int outspace = fmtsize * 2;
    int outused = 0;
    const char *out = malloc(outspace);
    if (!out) {
        sqlite3_result_error(ctx, "error formatting replacement: ENOMEM");
    }

    char *remains = format;
    char *end = strchr(format, 0);

    while (remains < end) {
        char *backref = strchr(format, '\\');
        int segment_length = backref-remains;
        strncpy(remains, segment_length, out);
    }
}*/

extern "C" {
    /**
     * Match a regular expression, as in
     * SELECT * FROM table WHERE field REGEXP 'some regex';
     */
    static
    void regexp(sqlite3_context *ctx, int argc, sqlite3_value **argv)
    {
        const char *re, *str;
        pcre *p;
        pcre_extra *e;

        assert(argc == 2);

        re = (const char *) sqlite3_value_text(argv[0]);
        if (!re) {
            sqlite3_result_error(ctx, "no regexp", -1);
            return;
        }

        str = (const char *) sqlite3_value_text(argv[1]);
        if (!str) {
            sqlite3_result_error(ctx, "no string", -1);
            return;
        }

        getPattern(ctx, re, &p, &e);

        {
            int rc;
            assert(p);
            rc = pcre_exec(p, e, str, strlen(str), 0, 0, NULL, 0);
            sqlite3_result_int(ctx, rc >= 0);
            return;
        }
    }


    /**
     * Substitute using regular expressions
     *
     * Used (in SQL) as sub('regex', 'replacement with \1 style groups', text)
     */
    /*
    static
    void sub(sqlite3_context *ctx, int argc, sqlite3_value **argv)
    {
        const char *re, *subject, *format;

        assert(argc == 3);

        re = (const char *) sqlite3_value_text(argv[0]);
        if (!re) {
            sqlite3_result_error(ctx, "no regexp", -1);
            return;
        }

        format = (const char *) sqlite3_value_text(argv[1]);
        if (!format) {
            sqlite3_result_error(ctx, "no replacement format", -1);
            return;
        }

        subject = (const char *) sqlite3_value_text(argv[2]);
        if (!subject) {
            sqlite3_result_error(ctx, "no string", -1);
            return;
        }

        {
            int substring_count;
            pcre *p;
            pcre_extra *e;
            int outsize = 0;
            // Defined at the top
            int captures[MAX_CAPTURES*3];
            getPattern(ctx, re, &p, &e);
            assert(p);

            substring_count = pcre_exec(p, e, subject, strlen(subject), 0, 0, captures, MAX_CAPTURES * 3);
            if (substring_count) {
                // TODO
            } else {
                // No match
                sqlite3_result_null(ctx);
            }
            return;
        }
    }
    */

    int sqlite3_extension_init(sqlite3 *db, char **err, const sqlite3_api_routines *api)
    {
            SQLITE_EXTENSION_INIT2(api)
            cache_entry *cache = (cache_entry *) calloc(CACHE_SIZE, sizeof(cache_entry));
            if (!cache) {
                *err = "calloc: ENOMEM";
                return 1;
            }
            sqlite3_create_function(db, "REGEXP", 2, SQLITE_UTF8, cache, regexp, NULL, NULL);
            //sqlite3_create_function(db, "SUB", 3, SQLITE_UTF8, cache, sub, NULL, NULL);
            return 0;
    }

}
