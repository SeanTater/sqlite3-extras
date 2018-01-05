use sqlite3_raw::*;
use macros;
use std::slice;
use std::os::raw::c_void;
use std::ptr;
use std::mem;

enum VirtualEponymity {
    NonEponymous,
    Eponymous,
    EponymousOnly
}
trait VirtualTable {
    type Cursor;
    fn vtable_eponymity() -> &'static VirtualEponymity;
    fn vtable_definition() -> &'static str;
    fn connect(&mut self);
    fn disconnect(&mut self);
    fn open_cursor(&mut self) -> Self::Cursor;
    fn close_cursor(&mut self, cursor: Self::Cursor);
    fn next(&mut self,      cursor: &Self::Cursor);
    fn column(&mut self,    cursor: &Self::Cursor);
    fn rowid(&self,         cursor: &Self::Cursor);
    fn eof(&self,           cursor: &Self::Cursor);
    fn filter(&self,
        idx_num: i32,
        idx_str: &str,
        args: &[*const i8],
        cursor: &Self::Cursor);
    fn best_index(&self,
        tab: *mut sqlite3_vtab,
        p_idx_info: *mut sqlite3_index_info);
}


unsafe fn infer_sqlite3_malloc<T>() -> Option<*mut T> {
    let size = mem::size_of::<T>();
    let mut p = sql_call!(malloc)(size as i32) as *mut T;
    if p.is_null() { None } else { Some(p) }
}

struct VTableWrapper<T> {
    base: sqlite3_vtab,
    inner: T
}
struct CursorWrapper<T> {
    base: sqlite3_vtab_cursor,
    inner: T
}

#[repr(C)]
struct RangeCursor {
    base: sqlite3_vtab_cursor,
    rowid: i64,
    value: i64,
    start: i64,
    stop: i64,
    step: i64
}
#[repr(C)]
struct RangeVTab {
    base: sqlite3_vtab
    // You can put more here later.
}

unsafe extern "C" fn range_connect(
    db: *mut sqlite3,
    state: *mut c_void,
    argc: i32,
    argv: *const *const i8,
    pp_vtab: *mut *mut sqlite3_vtab,
    pz_err_: *mut *mut i8
) -> i32 {
    or_die!(sql_call!(declare_vtab)(db, const_cstr!(
        "CREATE TABLE range(value, start HIDDEN, stop HIDDEN, step HIDDEN);").as_ptr()));
    
    // declare vtab succeeded
    assert_ok!(match infer_sqlite3_malloc::<RangeVTab>() {
        None => SQLITE_NOMEM as i32,
        Some(ptab) => {
            // Pass this pointer back to sqlite
            *pp_vtab = ptab as *mut sqlite3_vtab;
            SQLITE_OK
        }
    })
}

unsafe extern "C" fn range_disconnect(vtab: *mut sqlite3_vtab) -> i32 {
  sql_call!(free)(vtab as *mut c_void);
  SQLITE_OK
}

/*
** Constructor for a new RangeCursor object.
*/
unsafe extern "C" fn range_open(p: *mut sqlite3_vtab, pp_cursor: *mut *mut sqlite3_vtab_cursor) -> i32 {
    assert_ok!(match infer_sqlite3_malloc::<RangeCursor>() {
        Some(pcur) => {
            *pp_cursor = pcur as *mut sqlite3_vtab_cursor;
            SQLITE_OK
        },
        None => SQLITE_NOMEM
    })
}

/*
** Destructor for a RangeCursor.
*/
unsafe extern "C" fn range_close(cur: *mut sqlite3_vtab_cursor) -> i32 {
    sql_call!(free)(cur as *mut c_void);
    SQLITE_OK
}


/*
** Advance a RangeCursor to its next row of output.
*/
unsafe extern "C" fn range_next(cur: *mut sqlite3_vtab_cursor) -> i32 {
    let pcur = (cur as *mut RangeCursor).as_mut().unwrap();
    pcur.value += pcur.step;
    pcur.rowid += 1;
    SQLITE_OK
}

/*
** Return values of columns for the row at which the RangeCursor
** is currently pointing.
*/
unsafe extern "C" fn range_column(
  cur: *mut sqlite3_vtab_cursor,   /* The cursor */
  ctx: *mut sqlite3_context,       /* First argument to sqlite3_result_...() */
  i: i32                           /* Which column to return */
) -> i32 {
    let pcur = (cur as *mut RangeCursor).as_ref().unwrap();
    let x = match i {
        SERIES_COLUMN_START =>  pcur.start,
        SERIES_COLUMN_STOP =>   pcur.stop,
        SERIES_COLUMN_STEP =>   pcur.step,
        _ =>                    pcur.value
    };
    sql_call!(result_int64)(ctx, x);
    SQLITE_OK
}

/*
** Return the rowid for the current row. In this implementation, the
** first row returned is assigned rowid value 1, and each subsequent
** row a value 1 more than that of the previous.
*/
unsafe extern "C" fn range_rowid(
    cur: *mut sqlite3_vtab_cursor,
    p_rowid: *mut sqlite_int64) -> i32 {
    let pcur = (cur as *mut RangeCursor).as_ref().unwrap();
    *p_rowid = pcur.rowid;
    SQLITE_OK
}

/*
** Return TRUE if the cursor has been moved off of the last
** row of output.
*/
unsafe extern "C" fn range_eof(cur: *mut sqlite3_vtab_cursor) -> i32 {
    let pcur = (cur as *mut RangeCursor).as_ref().unwrap();
    (if pcur.step < 0 {
        pcur.value < pcur.start
    } else {
        pcur.value > pcur.stop
    }) as i32
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
unsafe extern "C" fn range_filter(
    cur: *mut sqlite3_vtab_cursor, 
    idx_num: i32,
    idx_str: *const i8,
    argc: i32,
    pp_argv: *mut *mut sqlite3_value
) -> i32 {
    let pcur = (cur as *mut RangeCursor).as_mut().unwrap();
    let argv = slice::from_raw_parts(pp_argv, argc as usize);
    let mut i = 0usize;
    if idx_num & 1 != 0 {
        pcur.start = sql_call!(value_int64)(argv[i]);
        i += 1;
    } else {
        pcur.start = 0;
    }
    if idx_num & 2 != 0{
        pcur.stop = sql_call!(value_int64)(argv[i]);
        i += 1;
    } else {
        pcur.stop = 0xffffffff;
    }
    if idx_num & 4 != 0 {
        pcur.step = sql_call!(value_int64)(argv[i]);
        i += 1;
        if pcur.step < 1 {
            pcur.step = 1;
        }
    } else {
        pcur.step = 1;
    }
    if idx_num & 8 != 0 {
        //pcur->isDesc = 1;
        pcur.value = pcur.stop;
        if pcur.step > 0 {
            pcur.value -= (pcur.stop - pcur.start) % pcur.step;
        }
    } else {
        //pcur->isDesc = 0;
        pcur.value = pcur.start;
    }
    pcur.rowid = 1;
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
unsafe extern "C" fn range_best_index(
  tab: *mut sqlite3_vtab,
  p_idx_info: *mut sqlite3_index_info
) -> i32 {
    let mut idx_num = 0i32;        /* The query plan bitmask */
    let mut start_idx = -1i32;     /* Index of the start= constraint, or -1 if none */
    let mut stop_idx = -1i32;      /* Index of the stop= constraint, or -1 if none */
    let mut step_idx = -1i32;      /* Index of the step= constraint, or -1 if none */
    let mut n_arg = 0i32;          /* Number of arguments that range_Filter() expects */
    let idx_info = p_idx_info.as_mut().unwrap();
    let constraints = slice::from_raw_parts((*p_idx_info).aConstraint, (*p_idx_info).nConstraint as usize);
    for (ui, constraint) in constraints.iter().enumerate() {
        let i = ui as i32;
        if constraint.usable != 0 && constraint.op == SQLITE_INDEX_CONSTRAINT_EQ {
            match constraint.iColumn {
                SERIES_COLUMN_START => {
                    start_idx = i;
                    idx_num |= 1;
                },
                SERIES_COLUMN_STOP => {
                    stop_idx = i;
                    idx_num |= 2;
                },
                SERIES_COLUMN_STEP => {
                    step_idx = i;
                    idx_num |= 4;
                },
                _ => ()
            }
        }
    }
    
    if start_idx >= 0 {
        n_arg += 1;
        idx_info.aConstraintUsage.offset(start_idx as isize).as_mut().unwrap().argvIndex = n_arg;
        idx_info.aConstraintUsage.offset(start_idx as isize).as_mut().unwrap().omit = 1; // No longer checked by sqlite
    }
    if stop_idx >= 0 {
        n_arg += 1;
        idx_info.aConstraintUsage.offset(stop_idx as isize).as_mut().unwrap().argvIndex = n_arg;
        idx_info.aConstraintUsage.offset(stop_idx as isize).as_mut().unwrap().omit = 1;
    }
    if step_idx >= 0 {
        n_arg += 1;
        idx_info.aConstraintUsage.offset(step_idx as isize).as_mut().unwrap().argvIndex = n_arg;
        idx_info.aConstraintUsage.offset(step_idx as isize).as_mut().unwrap().omit = 1;
    }
    if (idx_num & 3) == 3 {
        /* Both start= and stop= boundaries are available.  This is the 
        ** the preferred case */
        idx_info.estimatedCost = (2 - ((idx_num & 4)!=0) as i64) as f64;
        idx_info.estimatedRows = 1000;
        if idx_info.nOrderBy == 1 {
            if idx_info.aOrderBy.as_ref().unwrap().desc != 0 {
                idx_num |= 8;
            }
            idx_info.orderByConsumed = 1;
        }
    } else {
        /* If either boundary is missing, we have to generate a huge span
        ** of numbers.  Make this case very expensive so that the query
        ** planner will work hard to avoid it. */
        idx_info.estimatedCost = 2147483647.0f64;
        idx_info.estimatedRows = 2147483647;
    }
    idx_info.idxNum = idx_num;
    SQLITE_OK
}

/*
** This following structure defines all the methods for the 
** generate_series virtual table.
*/
pub static range_module : sqlite3_module = sqlite3_module {
    iVersion:       0,
    xCreate:        None,
    xConnect:       Some(range_connect),
    xBestIndex:     Some(range_best_index),
    xDisconnect:    Some(range_disconnect),
    xDestroy:       None,
    xOpen:          Some(range_open),   // open a cursor
    xClose:         Some(range_close),  // close a cursor
    xFilter:        Some(range_filter), // configure scan constraints
    xNext:          Some(range_next),   // advance a cursor
    xEof:           Some(range_eof),    // check for end of scan
    xColumn:        Some(range_column), // read data
    xRowid:         Some(range_rowid),  // read data
    xUpdate:        None,
    xBegin:         None,
    xSync:          None,
    xCommit:        None,
    xRollback:      None,
    xFindFunction:  None,
    xRename:        None,
    // The following are for version 2 and above
    xSavepoint:     None,
    xRelease:       None,
    xRollbackTo:    None
};

const SERIES_COLUMN_VALUE : i32 = 0;
const SERIES_COLUMN_START : i32 = 1;
const SERIES_COLUMN_STOP  : i32 = 2;
const SERIES_COLUMN_STEP  : i32 = 3;
