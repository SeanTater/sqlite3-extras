/*
 * Written by Alexey Tourbin <at@altlinux.org>.
 * Later rewritten by Sean Gallagher <stgallag@gmail.com>
 *
 * The author has dedicated the code to the public domain.  Anyone is free
 * to copy, modify, publish, use, compile, sell, or distribute the original
 * code, either in source code form or as a compiled binary, for any purpose,
 * commercial or non-commercial, and by any means.
 */
#include <assert.h>
#include <stdlib.h>
#include <string.h>
#include <string>
#include <boost/regex.hpp>
#include <sqlite3ext.h>
SQLITE_EXTENSION_INIT1

#ifndef CACHE_SIZE
#define CACHE_SIZE 16
#endif

/* Captures, for sub()
 * See PCRE doc: slices are only the first 2/3 of the real space.
 */
#ifndef MAX_CAPTURES
#define MAX_CAPTURES 64
#endif

using namespace std;
using namespace boost;

/**
 * @brief SQLite3Regex, intended for storing a shared regex cache if necessary
 */
class SQLite3Regex {
private:
    // TODO: actually implement a cache
public:
    bool match(string re, string str)
    {
        // Before adding back a cache, let's see a use case for it
        regex r(re);
        return regex_match(str, r);
    }

    bool search(string re, string str)
    {
        // Before adding back a cache, let's see a use case for it
        regex r(re);
        return regex_search(str, r);
    }

};

extern "C" {

    /**
     * Match a regular expression, as in
     * SELECT * FROM table WHERE field REGEXP 'some regex';
     */
    void match(sqlite3_context *ctx, int argc, sqlite3_value **argv)
    {
        const char *re, *str;
        assert(argc == 2);

        SQLite3Regex * regex_mod = (SQLite3Regex *) sqlite3_user_data(ctx);
        assert(regex_mod);

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

        sqlite3_result_int(ctx, regex_mod->match(re, str));
        return;
    }

    void search(sqlite3_context *ctx, int argc, sqlite3_value **argv)
    {
        const char *re, *str;
        assert(argc == 2);

        SQLite3Regex * regex_mod = (SQLite3Regex *) sqlite3_user_data(ctx);
        assert(regex_mod);

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

        sqlite3_result_int(ctx, regex_mod->search(re, str));
        return;
    }

    int sqlite3_extension_init(sqlite3 *db, char **err, const sqlite3_api_routines *api)
    {
            SQLITE_EXTENSION_INIT2(api)
            SQLite3Regex * regex_inst = new SQLite3Regex();
            sqlite3_create_function(db, "MATCH", 2, SQLITE_UTF8, regex_inst, match, NULL, NULL);
            sqlite3_create_function(db, "SEARCH", 2, SQLITE_UTF8, regex_inst, search, NULL, NULL);
            return 0;
    }

}
