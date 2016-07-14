sqlite3-extras
==============
Regular Expressions and Scalar/Vector Math for SQLite3

Usage
-----
### Regular Expressions
reutil adds to SQLite's builtin LIKE and GLOB expressions by adding full featured regular expression support.
It's based on another [module that used PCRE.](https://github.com/ralight/sqlite3-pcre).
Whereas its predecessor supported only matches, this supports searches, matches, and formatted replacements,
as implemented by the
[Boost project regex module](http://www.boost.org/doc/libs/1_55_0/libs/regex/doc/html/index.html).

 - [Regex syntax reference](http://www.boost.org/doc/libs/1_55_0/libs/regex/doc/html/boost_regex/syntax/perl_syntax.html) (Perl compatible)
 - [Replacement format reference](http://www.boost.org/doc/libs/1_55_0/libs/regex/doc/html/boost_regex/format/perl_format.html) (also Perl compatible)

Examples:
```sql
SELECT * FROM table WHERE column MATCH "<tag [^>]+>";
SELECT * FROM table WHERE column SEARCH "is the (thir|four)teenth of May";
SELECT sub("(\w+) lives by lake (\w+)", "$1 thinks $2 is cool.", column) FROM table;
```

### Vector Math
SQLite omits a lot of functions, to reduce API footprint. As such, the functions provided here probably not as bulletproof as those in SQLite. But in practice they're still pretty useful:

```sql
sqlite> .load sqlite3-reutil
sqlite> SELECT sin(4);
-0.756802495307928
sqlite> SELECT vshow(sin(vread('3 7 6 1.2')));
0.14112 0.656987 -0.279415 0.932039
```

The following functions are included:
- Regular Expressions
 - `match(regular expression, subject)`: Match the regular expression against the subject, only at the beginning.
 - `search(regular expression, subject)`: Search through the subject to see if there is a match for the regular expression anywhere within the text
 - `sub(regular expression, format string, subject)`: Replace any matches of the regular expression with the format string, with may contain backticks of the form:
 - `\0`: Replaced with the whole match
 - `\1`: Replaced with the first captured group
 - `\2`: Replaced with the second captured group
 - See the Boost Regex reference above for more information.
- Math
  - Create Vectors
    - `vzero(length)`: Create a 0-vector of a specific length
    - `vone(length)`: Create a 1-vector of a specific length
    - `vread(text)`: Read space-separated floating point values from text into a vector
    - `vshow(text)`: Format a vector as a human readable string compatible with vread()
 - Unary Operators on Vectors or Scalars
    - `sin(V)`
    - `asin(V)`
    - `cos(V)`
    - `acos(V)`
    - `tan(V)`
    - `atan(V)`
    - `log(V)`
    - `exp(V)`
    - `sqrt(V)`
 - Binary Operators on any combination of vector and scalars
    - `add(V, V)`
    - `subtract(V, V)`
    - `mult(V, V)`
    - `div(V, V)`
    - `pow(V, V)`
 - Vector operations (operate only on vectors)
    - `vsum(V)`: Compute the sum of the elements of a vector
    - `vprod(V)`: Compute the product of the elements of a vector
    - `dot(V, V)`: Compute the dot product of two vectors
    - `cossim(V, V)`: Compute the cosine similarity of two vectors

Install
-------

It includes a QMake project file, but you can also compile it by hand. It's very small.
```sh
# Download
git clone https://github.com/SeanTater/sqlite3-extras.git && cd sqlite3-extras
make build-mac # or build-linux
```
To begin using it, do any of the following:
  - Open an `sqlite3` window, and  `.load sqlite3-extras`, then do as you please.
  - OR, use `SELECT load_extension('/path/to/sqlite3-extras.so')`
    (replacing .so with .dylib on Mac, or .dll on Windows)
  - OR, put `.load sqlite3-extras` in `~/.sqliterc` so that it will load
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

### I need something faster
In disk-bound computations, it can be helpful to lower the precision and use
32-bit precision instead of 64-bit. To do that, pass -DREDUCED_PRECISION to gcc
or use CPPFlags to do the same. The vectors are not compatible so you may need
to use a combination of the 64-bit vshow and 32-bit vread to convert existing
tables.
