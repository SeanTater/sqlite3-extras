extern crate rusqlite;
extern crate glob;
use rusqlite as sql;

fn get_connection() -> sql::Connection {
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
    conn
}

macro_rules! fetch_one_cell {
    ($conn: expr, $sql_string: expr) => {
        $conn.query_row($sql_string, &[], |r| r.get(0)).unwrap()
    }
}

#[test]
fn tange_can_generate_series() {
    let conn = get_connection();
    let rows: i64 = fetch_one_cell!(conn, "SELECT count(*) FROM range(0, 10) LIMIT 20;");
    assert_eq!(rows, 10);
    let sum: i64 = fetch_one_cell!(conn, "SELECT sum(value) FROM range(0, 10) LIMIT 20;");
    assert_eq!(sum, (0..10).sum());
}