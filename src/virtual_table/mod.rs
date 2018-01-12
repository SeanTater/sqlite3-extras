//! Extensions using Virtual Tables
pub mod range;
pub mod internals;

use sqlite3_raw::*;
use const_cstr::ConstCStr;
use std::ffi::CStr;
use dynamics::*;

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
    fn column(&self, i32) -> SQLiteReturn;
    fn rowid(&self) -> i64;
    fn eof(&self) -> bool;
    fn filter(&mut self,
        idx_num: i32,
        idx_str: Option<&CStr>,
        args: &[*mut sqlite3_value]);
}