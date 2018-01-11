pub mod range;
pub mod internals;

use sqlite3_raw::*;
use std::os::raw::c_void;
use const_cstr::ConstCStr;
use std::ffi::CStr;

/// This represents whether a virtual table can be used as a virtual table,
/// as a function, or both ways. Keep in mind this affects whether the
/// create() and connect
pub enum VirtualEponymity {
    NonEponymous,
    Eponymous,
    EponymousOnly
}
pub trait VirtualTable {
    type Cursor : VirtualCursor;
    /// Whether this virtual table can be used via `CREATE TABLE`, as a function, or both
    fn vtable_eponymity() -> VirtualEponymity;
    fn vtable_definition() -> ConstCStr;
    /// Create a virtual table with CREATE TABLE.
    fn create() -> Self;
    // fn destroy(Self);
    
    /// Create a virtual table using function
    fn connect() -> Self;
    //fn disconnect(Self);
    
    fn open_cursor(&mut self) -> Self::Cursor;
    // fn close_cursor(&mut self, cursor: Self::Cursor);
    
    fn best_index(&self,
        idx_info: &mut sqlite3_index_info,
        constraints: &[sqlite3_index_info_sqlite3_index_constraint],
        order_bys: &[sqlite3_index_info_sqlite3_index_orderby],
        constraint_usages: &mut [sqlite3_index_info_sqlite3_index_constraint_usage]);
}


pub trait VirtualCursor {
    fn next(&mut self);
    fn column(&self, &SQLiteResponder, i32);
    fn rowid(&self) -> i64;
    fn eof(&self) -> bool;
    fn filter(&mut self,
        idx_num: i32,
        idx_str: Option<&CStr>,
        args: &[*mut sqlite3_value]);
}

/// This wraps an SQLite context, which is where you return values
///
/// The alternative to this approach is to return an Enum type.
/// The trouble is that then there is a good chance you would have to copy
/// strings and other large stuff twice (once to the enum, then once inside
/// this library from the enum to sqlite), which would hurt performance.
pub struct SQLiteResponder{ctx: *mut sqlite3_context}
impl SQLiteResponder {
    fn respond_copy(&self, thing: &SQLiteCopyRespondable) {
        thing.push_to(self.ctx);
    }
}
/// Things you can pass to SQLite. See [SQLiteResponder](SQLiteResponder).
///
/// You can't pass unsigned 64-bit integers
/// because the underlying API does not support it.
pub trait SQLiteCopyRespondable {
    fn push_to(&self, *mut sqlite3_context);
}
macro_rules! respondable_for_int {
    ($typename: ty) => {
        impl SQLiteCopyRespondable for $typename {
            fn push_to(&self, ctx: *mut sqlite3_context) {
                unsafe{ sql_call!(result_int64)(ctx, *self as i64); }
            }
        }
    }
}
respondable_for_int!(i64);
respondable_for_int!(i32);
respondable_for_int!(isize);
respondable_for_int!(u32);

impl SQLiteCopyRespondable for f32 {
    fn push_to(&self, ctx: *mut sqlite3_context) {
        unsafe{ sql_call!(result_double)(ctx, *self as f64); }
    }
}
impl SQLiteCopyRespondable for f64 {
    fn push_to(&self, ctx: *mut sqlite3_context) {
        unsafe{ sql_call!(result_double)(ctx, *self); }
    }
}

///// Simplest base case for text
/////
///// Better performance can be achieved by preventing unnecessary copies.
///// Right now this implementation will always copy, even if it is unnecessary.
///// Also, strings passed this way cannot be larger than 2GB because
///// `sqlite3_result_text` takes an `i32` length parameter.
//impl<'t> SQLiteRespondable for &'t str {
//    fn push_to(self, ctx: *mut sqlite3_context) {
//        // SQLITE_TRANSIENT is `#define SQLITE_TRANSIENT   ((sqlite3_destructor_type)-1)`
//        // in the original sqlite.h
//        // But unfortunately bindgen has trouble with that, which is
//        // understandable because it's playing games with the types.
//        // So instead of using SQLite's builtin functionality we can replicate
//        // this functionality using String and a destructor.
//        
//        unsafe{ sql_call!(result_text)(
//            ctx,
//            self.as_ptr() as *const i8, // The pointer to the string
//            self.len() as i32, // Length in bytes
//            Some(!0 as *const *const c_void) // Special value meaning copy this string
//            ); }
//    }
//}

//impl SQLiteRespondable for String {
//    fn push_to(&self, ctx: *mut sqlite3_context) {
//        unsafe{ sql_call!(result_text)(ctx, self.as_ptr(), self.len(), SQLITE_TRANSIENT); }
//    }
//}

//impl SQLiteRespondable for i64 {
//    fn push_to(&self, ctx: *mut sqlite3_context) {
//        unsafe{ sql_call!(result_int64)(ctx, *self); }
//    }
//}

pub struct SQLiteValue(*mut sqlite3_value);

macro_rules! from_value_for_int {
    ($typename: ty) => {
        impl From<SQLiteValue> for $typename {
            fn from(val: SQLiteValue) -> $typename {
                unsafe { sql_call!(value_int64)(val.0) as $typename }
            }
        }
    }
}
from_value_for_int!(i64);
from_value_for_int!(i32);
from_value_for_int!(isize);
from_value_for_int!(u32);
impl From<SQLiteValue> for f32 {
    fn from(val: SQLiteValue) -> f32 {
        unsafe { sql_call!(value_double)(val.0) as f32 }
    }
}
impl From<SQLiteValue> for f64 {
    fn from(val: SQLiteValue) -> f64 {
        unsafe { sql_call!(value_double)(val.0) as f64 }
    }
}

