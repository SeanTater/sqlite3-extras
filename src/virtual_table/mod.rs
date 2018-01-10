pub mod range;

use sqlite3_raw::*;
use const_cstr::ConstCStr;
use std::ffi::CStr;
use std::slice;
use std::os::raw::c_void;
use std::mem;

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


#[repr(C)]
#[derive(Default)]
pub struct VTabWrapper<T> {
    base: sqlite3_vtab,
    inner: T
}


#[repr(C)]
#[derive(Default)]
pub struct CursorWrapper<T> {
    base: sqlite3_vtab_cursor,
    inner: T
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

pub unsafe extern "C" fn range_connect<Tab: VirtualTable>(
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

pub unsafe extern "C" fn range_disconnect<Tab: VirtualTable>(vtab: *mut sqlite3_vtab) -> i32 {
    Box::from_raw(vtab as *mut VTabWrapper<Tab>);
    println!("disconnecting");
    // It will be dropped when it goes out of scope here.
    SQLITE_OK
}

/*
** Constructor for a new RangeCursor object.
*/
pub unsafe extern "C" fn range_open<Tab: VirtualTable>(
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

/*
** Destructor for a RangeCursor.
*/
pub unsafe extern "C" fn range_close<Tab: VirtualTable>(
    cur: *mut sqlite3_vtab_cursor
) -> i32 {
    println!("closing");
    Box::from_raw(cur as *mut CursorWrapper<Tab::Cursor>);
    SQLITE_OK
}


/*
** Advance a RangeCursor to its next row of output.
*/
pub unsafe extern "C" fn range_next<Tab: VirtualTable>(
    cur: *mut sqlite3_vtab_cursor
) -> i32 {
    let pcur = (cur as *mut CursorWrapper<Tab::Cursor>).as_mut().unwrap();
    pcur.inner.next();
    SQLITE_OK
}

/*
** Return values of columns for the row at which the RangeCursor
** is currently pointing.
*/
pub unsafe extern "C" fn range_column<Tab: VirtualTable>(
  cur: *mut sqlite3_vtab_cursor,   /* The cursor */
  ctx: *mut sqlite3_context,       /* First argument to sqlite3_result_...() */
  i: i32                           /* Which column to return */
) -> i32 {
    let pcur = (cur as *mut CursorWrapper<Tab::Cursor>).as_ref().unwrap();
    pcur.inner.column(&SQLiteResponder{ctx: ctx}, i);
    SQLITE_OK
}

/*
** Return the rowid for the current row. In this implementation, the
** first row returned is assigned rowid value 1, and each subsequent
** row a value 1 more than that of the previous.
*/
pub unsafe extern "C" fn range_rowid<Tab: VirtualTable>(
    cur: *mut sqlite3_vtab_cursor,
    p_rowid: *mut sqlite_int64
) -> i32 {
    let pcur = (cur as *mut CursorWrapper<Tab::Cursor>).as_ref().unwrap();
    *p_rowid = pcur.inner.rowid();
    SQLITE_OK
}

/*
** Return TRUE if the cursor has been moved off of the last
** row of output.
*/
pub unsafe extern "C" fn range_eof<Tab: VirtualTable>(
    cur: *mut sqlite3_vtab_cursor
) -> i32 {
    let pcur = (cur as *mut CursorWrapper<Tab::Cursor>).as_ref().unwrap();
    pcur.inner.eof() as i32
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
pub unsafe extern "C" fn range_filter<Tab: VirtualTable>(
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
    pcur.inner.filter(
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
pub unsafe extern "C" fn range_best_index<Tab: VirtualTable>(
  pvtab: *mut sqlite3_vtab,
  p_idx_info: *mut sqlite3_index_info
) -> i32 {
    let vtab = (pvtab as *mut VTabWrapper<Tab>).as_ref().unwrap();
    vtab.inner.best_index(
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


unsafe fn infer_sqlite3_malloc<T>() -> Option<*mut T> {
    let size = mem::size_of::<T>();
    let mut p = sql_call!(malloc)(size as i32) as *mut T;
    if p.is_null() { None } else { Some(p) }
}