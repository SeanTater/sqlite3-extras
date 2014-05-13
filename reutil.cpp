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

using namespace std;
using namespace boost;

/**
 * @brief SQLite3Regex, intended for storing a shared regex cache if necessary
 */
class SQLite3Regex {
private:
    // TODO: actually implement a cache
public:
    bool match(string re_str, string subject)
    {
        regex r(re_str);
        return regex_match(subject, r);
    }

    bool search(string re_str, string subject)
    {
        regex r(re_str);
        return regex_search(subject, r);
    }

    string sub(string re_str, string format, string subject)
    {
        regex r(re_str);
        return regex_replace(subject, r, format);
    }

};

extern "C" {
    /**
     * @brief Match a regular expression, giving a boolean.
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

        // Catch all for regex errors and API cleanliness
        try {
            sqlite3_result_int(ctx, regex_mod->match(re, str));
        } catch (const regex_error& e) {
            sqlite3_result_error(ctx, e.what(), -1);
        }

        return;
    }

    /**
     * @brief Search a string with a regex.
     *
     * This differs from match. See Boost::regex for more information.
     */
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


        // Catch all for regex errors and API cleanliness
        try {
            sqlite3_result_int(ctx, regex_mod->search(re, str));
        } catch (const regex_error& e) {
            sqlite3_result_error(ctx, e.what(), -1);
        }
        return;
    }

    /**
     * @brief Substitute regex matches with a formatted string.
     * For more information about the string format, see Boost::regex or the README.
     */
    void sub(sqlite3_context *ctx, int argc, sqlite3_value **argv)
    {
        const char *re, *str, *format;
        assert(argc == 3);

        SQLite3Regex * regex_mod = (SQLite3Regex *) sqlite3_user_data(ctx);
        assert(regex_mod);

        re = (const char *) sqlite3_value_text(argv[0]);
        if (!re) {
            sqlite3_result_error(ctx, "no regexp", -1);
            return;
        }

        format = (const char *) sqlite3_value_text(argv[1]);
        if (!format) {
            sqlite3_result_error(ctx, "no format", -1);
            return;
        }

        str = (const char *) sqlite3_value_text(argv[2]);
        if (!str) {
            sqlite3_result_error(ctx, "no string", -1);
            return;
        }

        // Catch all for regex errors and API cleanliness
        try {
            sqlite3_result_text(ctx, regex_mod->sub(re, format, str).data(), -1, SQLITE_TRANSIENT);
        } catch (const regex_error& e) {
            sqlite3_result_error(ctx, e.what(), -1);
        }
        return;
    }

    int sqlite3_extension_init(sqlite3 *db, char **err, const sqlite3_api_routines *api)
    {
            SQLITE_EXTENSION_INIT2(api)
            SQLite3Regex * regex_inst = new SQLite3Regex();
            sqlite3_create_function(db, "MATCH", 2, SQLITE_UTF8, regex_inst, match, NULL, NULL);
            sqlite3_create_function(db, "SEARCH", 2, SQLITE_UTF8, regex_inst, search, NULL, NULL);
            sqlite3_create_function(db, "SUB", 3, SQLITE_UTF8, regex_inst, sub, NULL, NULL);
            return 0;
    }

}
