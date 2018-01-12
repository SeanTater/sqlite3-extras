//! SQLite Dynamic Type Wrappers
use sqlite3_raw::*;
use std::os::raw::c_void;
use std::string::FromUtf8Error;
use std::slice;
// The following two are for bookkeeping with in-flight Strings taken from Rust
// into SQLite-world
use std::sync::Mutex;
use std::collections::HashMap;


/// Zero cost Wrapper for SQLite Value pointers
///
/// These are values SQLite returns in many cases, such as function arguments
/// and column values. They are dynamically typed and have loose typing rules.
/// In most cases it is sufficient to use `.into()` to convert them into a
/// specific type. Keep in mind that doing so may change the underlying value,
/// so it is possible that multiple conversions will have intransitive
/// results.
///
/// See [SQLite Documentation on values](https://sqlite.org/c3ref/value_blob.html)
/// and [SQLite Documentation on columns](https://sqlite.org/c3ref/column_blob.html)
/// for more details.
pub struct SQLiteValue(*mut sqlite3_value);
impl SQLiteValue {
    pub unsafe fn from_raw_unchecked(ptr: *mut sqlite3_value) -> SQLiteValue {
        SQLiteValue(ptr)
    }
}
impl From<SQLiteValue> for isize {
    fn from(val: SQLiteValue) -> isize {
        unsafe { sql_call!(value_int64)(val.0) as isize }
    }
}
impl From<SQLiteValue> for i64 {
    fn from(val: SQLiteValue) -> i64 {
        unsafe { sql_call!(value_int64)(val.0) as i64 }
    }
}
impl From<SQLiteValue> for f64 {
    fn from(val: SQLiteValue) -> f64 {
        unsafe { sql_call!(value_double)(val.0) as f64 }
    }
}
/// You can only convert to `Option<String>` because it can be `NULL`.
/// This variant glosses over possible UTF-8 errors.
impl From<SQLiteValue> for Option<String> {
    fn from(val: SQLiteValue) -> Option<String> {
        let cptr = unsafe { sql_call!(value_text)(val.0) as *const u8 };
        if cptr.is_null() { None }
        else {
            let slice = unsafe { slice::from_raw_parts(
                cptr,
                sql_call!(value_bytes)(val.0) as usize) };
            Some(String::from_utf8_lossy(slice).to_string())
        }   
    }
}
/// You can only convert to `Option<String>` because it can be `NULL`.
/// This variant also exposes possible UTF8 errors
impl From<SQLiteValue> for Option<Result<String, FromUtf8Error>> {
    fn from(val: SQLiteValue) -> Option<Result<String, FromUtf8Error>> {
        let cptr = unsafe { sql_call!(value_text)(val.0) as *const u8 };
        if cptr.is_null() { None }
        else {
            let slice = unsafe { slice::from_raw_parts(
                cptr,
                sql_call!(value_bytes)(val.0) as usize) };
            Some(String::from_utf8(slice.to_owned()))
        }   
    }
}
/// You can only convert to `Option<Vec<u8>>` because it can be `NULL`.
///
/// See [SQLite Documentation](https://sqlite.org/c3ref/column_blob.html)
/// for more details.
impl From<SQLiteValue> for Option<Vec<u8>> {
    fn from(val: SQLiteValue) -> Option<Vec<u8>> {
        let cptr = unsafe { sql_call!(value_blob)(val.0) as *const u8 };
        if cptr.is_null() { None }
        else {
            let slice = unsafe { slice::from_raw_parts(
                cptr as *const u8,
                sql_call!(value_bytes)(val.0) as usize) };
            Some(slice.to_owned())
        }
    }
}
/// Use an SQLiteValue as a dynamic type
impl From<SQLiteValue> for SQLiteReturn {
    fn from(val: SQLiteValue) -> SQLiteReturn {
        let typecode = unsafe { sql_call!(value_type)(val.0) };
        match typecode {
            SQLITE_NULL => SQLiteReturn::SQLiteNull,
            SQLITE_FLOAT => SQLiteReturn::SQLiteFloat(val.into()),
            SQLITE_INTEGER => SQLiteReturn::SQLiteInt(val.into()),
            SQLITE_TEXT => SQLiteReturn::SQLiteText(
                // Technically speaking this can't be null because that would
                // make the type SQLITE_NULL. So there's no need to check this
                // but it sounds better not to bet on it.
                {let x: Option<String>= val.into(); x}.unwrap_or(String::new())
            ),
            SQLITE_BLOB => SQLiteReturn::SQLiteBlob(
                // Use an empty vector in place of a null pointer
                // because SQLite treats empty vectors as nulls
                {let x: Option<Vec<u8>>= val.into(); x}.unwrap_or(vec![])
            ),
            _ => unreachable!()
        }
    }
}

/// This wraps an SQLite context, which is where you return values
///
/// In most cases you can use `.into()` to help return.
/// In order to make costs clearer, you must pass an owned String,
/// which in many cases will require you to make a copy.
pub enum SQLiteReturn {
    SQLiteNull,
    SQLiteFloat(f64),
    SQLiteInt(i64),
    SQLiteText(String),
    SQLiteBlob(Vec<u8>)
}
impl SQLiteReturn {
    /// Push a return value into a return context (not something you should
    /// need to do yourself)
    pub fn push_to(self, ctx: *mut sqlite3_context) {
        unsafe {
            match self {
                SQLiteReturn::SQLiteNull => { sql_call!(result_null)(ctx); },
                SQLiteReturn::SQLiteFloat(x) => { sql_call!(result_double)(ctx, x); },
                SQLiteReturn::SQLiteInt(x) => { sql_call!(result_int64)(ctx, x); },
                SQLiteReturn::SQLiteText(x) => {
                    let mut strings = SQLITE_STRINGS_IN_FLIGHT.lock().unwrap();
                    let cptr = x.as_ptr() as *const i8;
                    sql_call!(result_text64)(
                        ctx,
                        cptr,
                        x.len() as u64,
                        Some(string_destructor),
                        SQLITE_UTF8 as u8 // In C these types are more flexible
                        );
                    // Insert it last since we are giving ownership away
                    strings.insert(cptr as usize, x);
                },
                SQLiteReturn::SQLiteBlob(x) => {
                    let mut blobs = SQLITE_BLOBS_IN_FLIGHT.lock().unwrap();
                    let cptr = x.as_ptr() as *const u8;
                    sql_call!(result_blob64)(
                        ctx,
                        cptr as *const c_void,
                        x.len() as u64,
                        Some(blob_destructor)
                        );
                    // Insert it last since we are giving ownership away
                    blobs.insert(cptr as usize, x);
                }
            }
        }
    }
}
impl From<f64> for SQLiteReturn {
    fn from(x: f64)     -> SQLiteReturn { SQLiteReturn::SQLiteFloat(x) }
}
impl From<i64> for SQLiteReturn {
    fn from(x: i64)     -> SQLiteReturn { SQLiteReturn::SQLiteInt(x) }
}
impl From<String> for SQLiteReturn {
    fn from(x: String)  -> SQLiteReturn { SQLiteReturn::SQLiteText(x) }
}
impl From<Vec<u8>> for SQLiteReturn {
    fn from(x: Vec<u8>) -> SQLiteReturn { SQLiteReturn::SQLiteBlob(x) }
}

pub unsafe extern "C" fn string_destructor(cptr: *mut c_void) {
    let mut strings = SQLITE_STRINGS_IN_FLIGHT.lock().unwrap();
    strings.remove(&(cptr as usize));
}
pub unsafe extern "C" fn blob_destructor(cptr: *mut c_void) {
    let mut strings = SQLITE_STRINGS_IN_FLIGHT.lock().unwrap();
    strings.remove(&(cptr as usize));
}

lazy_static! {
    static ref SQLITE_STRINGS_IN_FLIGHT: Mutex<HashMap<usize, String>>
        = Mutex::new(HashMap::new());
    static ref SQLITE_BLOBS_IN_FLIGHT: Mutex<HashMap<usize, Vec<u8>>>
        = Mutex::new(HashMap::new());
}
