extern crate bindgen;

use std::env;
use std::path::PathBuf;
use bindgen::callbacks::{ParseCallbacks, IntKind};

fn main() {
    // We don't need to link to any library
    println!("cargo:rustc-link-lib=sqlite3");

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .header("wrapper.h")
        // This makes initializing structs less verbose
        .derive_default(true)
        // This makes debugging easier
        .derive_debug(true)
        .impl_debug(true)
        // Guess SQLite's macros
        .parse_callbacks(Box::new(SQLiteEnums()))
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}

#[derive(Debug)]
struct SQLiteEnums();
impl ParseCallbacks for SQLiteEnums {
    fn int_macro(&self, name: &str, _value: i64) -> Option<IntKind> {
        if name.starts_with("SQLITE_INDEX_CONSTRAINT") { Some(IntKind::U8) }
        else if name.starts_with("SQLITE_SERIES_CONSTRAINT") { Some(IntKind::U8) }
        else {Some(IntKind::I32)}
    }
}