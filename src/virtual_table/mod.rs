pub mod range;

use sqlite3_raw::*;
use const_cstr::ConstCStr;

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
    fn respond(&self, thing: &SQLiteRespondable) {
        thing.push_to(self.ctx);
    }
}
pub trait SQLiteRespondable {
    fn push_to(&self, *mut sqlite3_context);
}
impl SQLiteRespondable for i64 {
    fn push_to(&self, ctx: *mut sqlite3_context) {
        unsafe{ sql_call!(result_int64)(ctx, *self); }
    }
}