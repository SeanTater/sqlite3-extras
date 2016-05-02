build-mac:
	g++ -fPIC -std=c++11 -shared extras.cpp -I/usr/lib/sqlite3 -o sqlite3-extras.dylib -lboost_regex
build-linux:
	g++ -fPIC -std=c++11 -shared extras.cpp -I/usr/lib/sqlite3 -o sqlite3-extras.so -lboost_regex
