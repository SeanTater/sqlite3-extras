#include <assert.h>
#include <stdlib.h>
#include <string.h>
#include <string>
#include <boost/regex.hpp>
#include <sqlite3ext.h>
SQLITE_EXTENSION_INIT1

/**
  Get a string argument from SQLite. It assumes you already know it's there.
  You can check that with argc() or trust SQLite to do that. */
const char *getSQLiteString(
    sqlite3_context *ctx,
    sqlite3_value *arg,
    const std::string& func,
    const std::string& name) {
  const char *value = (const char *) sqlite3_value_text(arg);
  if (value) {
    return value;
  } else {
    sqlite3_result_error(ctx, (func + "(): missing " + name).data(), -1);
    return NULL;
  }
}

extern "C" {
  /**
   * @brief Match a regular expression, giving a boolean.
   */
  void match(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
      const char *re = getSQLiteString(ctx, argv[0], "match", "regular expression");
      const char *subject = getSQLiteString(ctx, argv[1], "match", "subject");
      if (!re || !subject) return;

      // Catch all for regex errors and API cleanliness
      try {
        boost::regex regex(re);
        sqlite3_result_int(ctx, boost::regex_match(subject, regex));
      } catch (const boost::regex_error& e) {
        sqlite3_result_error(ctx, e.what(), -1);
      }

      return;
  }

  /**
   * @brief Search a string with a regex.
   *
   * This differs from match. See Boost::regex for more information.
   */
  void search(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
    const char *re = getSQLiteString(ctx, argv[0], "search", "regular expression");
    const char *subject = getSQLiteString(ctx, argv[1], "subject", "regular expression");
    if (!re || !subject) return;

    // Catch all for regex errors and API cleanliness
    try {
      boost::regex regex(re);
      sqlite3_result_int(ctx, boost::regex_search(std::string(subject), regex));
    } catch (const boost::regex_error& e) {
      sqlite3_result_error(ctx, e.what(), -1);
    }
    return;
  }

  /**
   * @brief Substitute regex matches with a formatted string.
   * For more information about the string format, see Boost::regex or the README.
   */
  void sub(sqlite3_context *ctx, int argc, sqlite3_value **argv) {
    const char *re = getSQLiteString(ctx, argv[0], "sub", "regular expression");
    const char *format = getSQLiteString(ctx, argv[1], "sub", "format string");
    const char *subject = getSQLiteString(ctx, argv[2], "sub", "subject");
    if (!re || !format || !subject) return;
    // Catch all for regex errors and API cleanliness
    try {
      boost::regex regex(re);
      std::string replaced = boost::regex_replace(std::string(subject), regex, format);
      sqlite3_result_text(ctx, replaced.data(), -1, SQLITE_TRANSIENT);
    } catch (const boost::regex_error& e) {
      sqlite3_result_error(ctx, e.what(), -1);
    }
    return;
  }

  int sqlite3_extension_init(sqlite3 *db, char **err, const sqlite3_api_routines *api) {
    SQLITE_EXTENSION_INIT2(api)
    sqlite3_create_function(db, "MATCH", 2, SQLITE_UTF8, SQLITE_DETERMINISTIC, match, NULL, NULL);
    sqlite3_create_function(db, "SEARCH", 2, SQLITE_UTF8, SQLITE_DETERMINISTIC, search, NULL, NULL);
    sqlite3_create_function(db, "SUB", 3, SQLITE_UTF8, SQLITE_DETERMINISTIC, sub, NULL, NULL);
    return 0;
  }
}
