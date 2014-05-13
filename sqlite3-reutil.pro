TEMPLATE = lib
CONFIG -= app_bundle
CONFIG -= qt
CONFIG += dll
VERSION = 0.1.0
SOURCES += reutil.cpp
# SQLite is C based, but this library is in C++. That's fine, but then boost is
# also C++ based and the C dynamic linker won't take it. So static link boost.
QMAKE_LFLAGS += -Wl,--whole-archive -lboost_regex -Wl,--no-whole-archive
