TEMPLATE = app
#CONFIG += console
CONFIG -= app_bundle
CONFIG -= qt
CONFIG += dll

SOURCES += main.cpp


unix|win32: LIBS += -lpcre
unix|win32: LIBS += -lpcrecpp
