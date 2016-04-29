sqlite3-reutil
==============

Regular Expression Extension for SQLite3

Usage
-----

reutil adds to SQLite's builtin LIKE and GLOB expressions by adding full featured regular expression support.
It's based on another [module that used PCRE.](https://github.com/ralight/sqlite3-pcre).
Whereas its predecessor supported only matches, this supports searches, matches, and formatted replacements,
as implemented by the
[Boost project regex module](http://www.boost.org/doc/libs/1_55_0/libs/regex/doc/html/index.html).

 - [Regex syntax reference](http://www.boost.org/doc/libs/1_55_0/libs/regex/doc/html/boost_regex/syntax/perl_syntax.html) (Perl compatible)
 - [Replacement format reference](http://www.boost.org/doc/libs/1_55_0/libs/regex/doc/html/boost_regex/format/perl_format.html) (also Perl compatible)

Examples
--------
```sql
    SELECT * FROM table WHERE column MATCH "<tag [^>]+>";
    SELECT * FROM table WHERE column SEARCH "is the (thir|four)teenth of May";
    SELECT sub("(\w+) lives by lake (\w+)", "$1 thinks $2 is cool.", column) FROM table;
```

Install
-------

It includes a QMake project file, but you can also compile it by hand. It's very small.
```sh
# Download
git pull https://github.com/SeanTater/sqlite3-reutil.git && cd sqlite3-reutil
make
```
To begin using it, do any of the following:
  - Open an `sqlite3` window, and  `.load sqlite3-reutil`, then do as you please.
  - OR, use `SELECT load_extension('/path/to/sqlite3-reutil.so')`
    (replacing .so with .dylib on Mac, or .dll on Windows)
  - OR, put `.load sqlite3-reutil` in `~/.sqliterc` so that it will load
    automatically every time you open `sqlite3`.


FAQ
---

### Error: unknown command or invalid arguments:  "load". Enter ".help" for help
Your SQLite3 installation has runtime extensions disabled at compile time.
(As of this writing `brew` does this.) Recompiling sqlite3 is painless, though.
Download the [amalgamation](https://www.sqlite.org/download.html) and do
something like the following:

```sh
gcc -O3 -I. \
    -DSQLITE_ENABLE_FTS3 \
    -DSQLITE_ENABLE_FTS4 \
    -DSQLITE_ENABLE_FTS5 \
    -DSQLITE_ENABLE_JSON1 \
    -DSQLITE_ENABLE_RTREE \
    -DSQLITE_ENABLE_EXPLAIN_COMMENTS \
    -DSQLITE_THREADSAFE=2 \
    -DHAVE_USLEEP \
    -DHAVE_READLINE \
    shell.c sqlite3.c -ldl -lreadline -lncurses -o sqlite3
```
Then you'll have an `sqlite3` binary you can use with extensions. If you want to
play with it or know more about what this does, check the
[options list](https://www.sqlite.org/compile.html)
and also check out
[how to compile sqlite](https://www.sqlite.org/howtocompile.html).
