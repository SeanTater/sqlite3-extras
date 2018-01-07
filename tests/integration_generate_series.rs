extern crate rusqlite;
extern crate glob;
use rusqlite as sql;

#[test]
fn can_generate_series() {
    let conn = sql::Connection::open_in_memory().unwrap();
    conn.load_extension_enable().unwrap();
    let path = [".", "target/debug", "target/release", "./**", "../**"]
        .into_iter()
        .flat_map(|folder| ["dylib", "so", "dll"].into_iter().map(move |ext| (folder, ext)))
        .flat_map(|(folder, ext)| glob::glob(&format!("{}/{}.{}", folder, "libsqlite3_extras", ext)).unwrap())
        .map(|x| x.unwrap())
        .next()
        .expect("Couldn't find the dynamic library for SQLite to load. \
            Looked in target/debug/libsqlite3_extras.{dll,so,dylib}");
    conn.load_extension(path, None).unwrap();

    conn.execute("SELECT * FROM generate_series(0, 10) LIMIT 5;", &[]).unwrap();
}