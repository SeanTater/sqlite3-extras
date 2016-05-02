build:
	#g++ -fPIC -shared reutil.cpp -o sqlite3-reutil.so -Wl,--whole-archive -lboost_regex -Wl,--no-whole-archive
	g++ -fPIC -std=c++11 -shared reutil.cpp -I/usr/lib/sqlite3 -o sqlite3-reutil.dylib -lboost_regex
