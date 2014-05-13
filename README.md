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
    SELECT * FROM table WHERE column MATCH "is the (thir|four)teenth of May";
    SELECT sub("(thir|four)teenth", "eighteenth", column) FROM table;
```

Install
-------

It includes a QMake project file, but you can also compile it by hand. It's very small.
```sh
# Download
git pull https://github.com/SeanTater/sqlite3-reutil.git
cd sqlite3-reutil
# Compile
g++ -fPIC -shared reutil.cpp -o sqlite3-reutil.so -Wl,--whole-archive -lboost_regex -Wl,--no-whole-archive
# Install: Replace with su if necessary
sudo chown root.root reutil.so
sudo mv reutil.so /usr/lib/sqlite/
# Setup to load automatically when you run sqlite3 from the terminal
echo '.load /usr/lib/sqlite/reutil.so' >>~/.sqlite3
```

Packaged versions are planned for Linux distributions. Windows packages would be welcome.
