//! SQLite connection internals
//!
//! These are public so that they will be in the documentation, but don't use
//! them directly. Instead, use them as a reference on how the library
//! operates.
//! 
//! The majority of free functions are glue code either
//! between VirtualTable and sqlite_vtab, or
//! between VirtualCursor and sqlite_vtab_cursor.

use sqlite3_raw::*;
use std::ffi::CStr;
use std::slice;
use std::os::raw::c_void;
use std::ops::{Deref, DerefMut};
use virtual_table::*;

/// Wrapper for SQLite Virtual Table `sqlite3_vtab` objects
///
/// Implements Deref, only `.base` is overloaded.
#[repr(C)]
#[derive(Default)]
pub struct VTabWrapper<T> {
    base: sqlite3_vtab,
    inner: T
}
impl<T> Deref for VTabWrapper<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.inner
    }
}
impl<T> DerefMut for VTabWrapper<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

/// Wrapper for SQLite Virtual Table Cursor `sqlite3_vtab_cursor` objects
///
/// Implements Deref, only `.base` is overloaded.
#[repr(C)]
#[derive(Default)]
pub struct CursorWrapper<T> {
    base: sqlite3_vtab_cursor,
    inner: T
}
impl<T> Deref for CursorWrapper<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.inner
    }
}
impl<T> DerefMut for CursorWrapper<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}


/// Construct a VirtualTable.
/// See [`sqlite3_module.xConnect`](https://sqlite.org/vtab.html)
pub unsafe extern "C" fn vtab_connect<Tab: VirtualTable>(
    db: *mut sqlite3,
    _state: *mut c_void,
    _argc: i32,
    _argv: *const *const i8,
    pp_vtab: *mut *mut sqlite3_vtab,
    _pz_err: *mut *mut i8
) -> i32 {
    println!("connecting");
    or_die!(sql_call!(declare_vtab)(db, Tab::vtable_definition().as_ptr()));
    
    let vtab : VTabWrapper<Tab> = VTabWrapper{
        base: Default::default(),
        inner: Tab::create()
    };
    *pp_vtab = Box::into_raw(Box::new(vtab)) as *mut sqlite3_vtab;
    SQLITE_OK
}

/// Destroy a VirtualTable.
/// See [`sqlite3_module.xDisconnect`](https://sqlite.org/vtab.html)
pub unsafe extern "C" fn vtab_disconnect<Tab: VirtualTable>(vtab: *mut sqlite3_vtab) -> i32 {
    Box::from_raw(vtab as *mut VTabWrapper<Tab>);
    println!("disconnecting");
    // It will be dropped when it goes out of scope here.
    SQLITE_OK
}

/// Construct a VirtualCursor.
/// See [`sqlite3_module.xOpen`](https://sqlite.org/vtab.html)
pub unsafe extern "C" fn vtab_open<Tab: VirtualTable>(
    _p: *mut sqlite3_vtab,
    pp_cursor: *mut *mut sqlite3_vtab_cursor
) -> i32 where
    Tab: VirtualTable,
    Tab::Cursor: Default
{
    println!("opening");
    let cursor : CursorWrapper<Tab::Cursor> = Default::default();
    *pp_cursor = Box::into_raw(Box::new(cursor)) as *mut sqlite3_vtab_cursor;
    SQLITE_OK
}

/// Destroy a VirtualCursor.
/// See [`sqlite3_module.xClose`](https://sqlite.org/vtab.html)
pub unsafe extern "C" fn cursor_close<Tab: VirtualTable>(
    cur: *mut sqlite3_vtab_cursor
) -> i32 {
    println!("closing");
    Box::from_raw(cur as *mut CursorWrapper<Tab::Cursor>);
    SQLITE_OK
}


/// Advance a VirtualCursor.
/// See [`sqlite3_module.xNext`](https://sqlite.org/vtab.html)
pub unsafe extern "C" fn cursor_next<Tab: VirtualTable>(
    cur: *mut sqlite3_vtab_cursor
) -> i32 {
    let pcur = (cur as *mut CursorWrapper<Tab::Cursor>).as_mut().unwrap();
    pcur.next();
    SQLITE_OK
}

/// Extract values from a VirtualCursor.
/// See [`sqlite3_module.xOpen`](https://sqlite.org/vtab.html)
///
/// Return values of columns for the row at which the VirtualCursor
/// is currently pointing.
pub unsafe extern "C" fn cursor_column<Tab: VirtualTable>(
  cur: *mut sqlite3_vtab_cursor,   /* The cursor */
  ctx: *mut sqlite3_context,       /* First argument to sqlite3_result_...() */
  i: i32                           /* Which column to return */
) -> i32 {
    let pcur = (cur as *mut CursorWrapper<Tab::Cursor>).as_ref().unwrap();
    pcur.column(&SQLiteResponder{ctx: ctx}, i);
    SQLITE_OK
}

/// Get a rowid for a VirtualCursor.
/// See [`sqlite3_module.xRowid`](https://sqlite.org/vtab.html)
///
/// Return the rowid for the current row. In this implementation, the
/// first row returned is assigned rowid value 1, and each subsequent
/// row a value 1 more than that of the previous.
pub unsafe extern "C" fn cursor_rowid<Tab: VirtualTable>(
    cur: *mut sqlite3_vtab_cursor,
    p_rowid: *mut sqlite_int64
) -> i32 {
    let pcur = (cur as *mut CursorWrapper<Tab::Cursor>).as_ref().unwrap();
    *p_rowid = pcur.rowid();
    SQLITE_OK
}

/// Return whether the Cursor has moved past the end.
/// See [`sqlite3_module.xEof`](https://sqlite.org/vtab.html)
/// 
/// Note that in C, bool is an integer (0 or 1)
pub unsafe extern "C" fn cursor_eof<Tab: VirtualTable>(
    cur: *mut sqlite3_vtab_cursor
) -> i32 {
    let pcur = (cur as *mut CursorWrapper<Tab::Cursor>).as_ref().unwrap();
    pcur.eof() as i32
}

/*
** This method is called to "rewind" the RangeCursor object back
** to the first row of output.  This method is always called at least
** once prior to any call to range_Column() or range_Rowid() or 
** range_Eof().
**
** The query plan selected by range_BestIndex is passed in the idx_num
** parameter.  (idxStr is not used in this implementation.)  idx_num
** is a bitmask showing which constraints are available:
**
**    1:    start=VALUE
**    2:    stop=VALUE
**    4:    step=VALUE
**
** Also, if bit 8 is set, that means that the series should be output
** in descending order rather than in ascending order.
**
** This routine should initialize the cursor and position it so that it
** is pointing at the first row, or pointing off the end of the table
** (so that range_Eof() will return true) if the table is empty.
*/
pub unsafe extern "C" fn cursor_filter<Tab: VirtualTable>(
    cur: *mut sqlite3_vtab_cursor, 
    idx_num: i32,
    idx_c_str: *const i8,
    argc: i32,
    pp_argv: *mut *mut sqlite3_value
) -> i32 {
    let pcur = (cur as *mut CursorWrapper<Tab::Cursor>).as_mut().unwrap();
    let argv = slice::from_raw_parts_mut(pp_argv, argc as usize);
    let idx_str = if idx_c_str.is_null() {
        None
    } else {
        Some(CStr::from_ptr(idx_c_str))
    };
    pcur.filter(
        idx_num,
        idx_str,
        argv
    );
    SQLITE_OK
}



/*
** SQLite will invoke this method one or more times while planning a query
** that uses the generate_series virtual table.  This routine needs to create
** a query plan for each invocation and compute an estimated cost for that
** plan.
**
** In this implementation idx_num is used to represent the
** query plan.  idxStr is unused.
**
** The query plan is represented by bits in idx_num:
**
**  (1)  start = $value  -- constraint exists
**  (2)  stop = $value   -- constraint exists
**  (4)  step = $value   -- constraint exists
**  (8)  output in descending order
*/
pub unsafe extern "C" fn vtab_best_index<Tab: VirtualTable>(
  pvtab: *mut sqlite3_vtab,
  p_idx_info: *mut sqlite3_index_info
) -> i32 {
    let vtab = (pvtab as *mut VTabWrapper<Tab>).as_ref().unwrap();
    vtab.best_index(
        // Raw index info
        p_idx_info.as_mut().unwrap(),
        // Constraints
        slice::from_raw_parts(
            (*p_idx_info).aConstraint,
            (*p_idx_info).nConstraint as usize
        ),
        // Order Bys
        slice::from_raw_parts(
            (*p_idx_info).aOrderBy,
            (*p_idx_info).nOrderBy as usize
        ),
        // Constraint Usages
        slice::from_raw_parts_mut(
            (*p_idx_info).aConstraintUsage,
            (*p_idx_info).nConstraint as usize
        )
    );
    SQLITE_OK
}