use sqlite3_raw::*;
use macros;
use std::ffi::CStr;
use const_cstr::ConstCStr;
use virtual_table::*;

impl VirtualTable for RangeVTab {
    type Cursor = RangeCursor;
    fn vtable_eponymity() -> VirtualEponymity {
        VirtualEponymity::EponymousOnly
    }
    fn vtable_definition() -> ConstCStr {
        const_cstr!("CREATE TABLE range(value, start HIDDEN, stop HIDDEN, step HIDDEN);")
    }
    fn create()  -> Self { Default::default() }
    fn connect() -> Self { Default::default() }
    fn open_cursor(&mut self) -> Self::Cursor {
        Default::default()
    }
    fn best_index(&self,
        idx_info: &mut sqlite3_index_info,
        constraints: &[sqlite3_index_info_sqlite3_index_constraint],
        order_bys: &[sqlite3_index_info_sqlite3_index_orderby],
        constraint_usages: &mut [sqlite3_index_info_sqlite3_index_constraint_usage]
    ){        
        let mut idx_num = 0i32;        /* The query plan bitmask */
        let mut start_idx = -1i32;     /* Index of the start= constraint, or -1 if none */
        let mut stop_idx = -1i32;      /* Index of the stop= constraint, or -1 if none */
        let mut step_idx = -1i32;      /* Index of the step= constraint, or -1 if none */
        let mut n_arg = 0i32;          /* Number of arguments that range_Filter() expects */
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
            constraint_usages[start_idx as usize].argvIndex = n_arg;
            constraint_usages[start_idx as usize].omit = 1; // No longer checked by sqlite
        }
        if stop_idx >= 0 {
            n_arg += 1;
            constraint_usages[stop_idx as usize].argvIndex = n_arg;
            constraint_usages[stop_idx as usize].omit = 1;
        }
        if step_idx >= 0 {
            n_arg += 1;
            constraint_usages[step_idx as usize].argvIndex = n_arg;
            constraint_usages[step_idx as usize].omit = 1;
        }
        if (idx_num & 3) == 3 {
            /* Both start= and stop= boundaries are available.  This is the 
            ** the preferred case */
            idx_info.estimatedCost = (2 - ((idx_num & 4)!=0) as i64) as f64;
            idx_info.estimatedRows = 1000;
            if order_bys.len() == 1 {
                if order_bys[0].desc != 0 {
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
    }
}
impl VirtualCursor for RangeCursor {
    fn next(&mut self) {
        self.value += self.step;
        self.rowid += 1;
    }
    fn column(&self, responder: &SQLiteResponder, index: i32) {
        let x = match index {
            SERIES_COLUMN_START =>  self.start,
            SERIES_COLUMN_STOP =>   self.stop,
            SERIES_COLUMN_STEP =>   self.step,
            _ =>                    self.value
        };
        // TODO: Don't call unsafe.
        responder.respond(&x);
    }
    fn rowid(&self) -> i64 { self.rowid }
    fn eof(&self) -> bool {
        if self.step < 0 {
            self.value < self.start
        } else {
            self.value > self.stop
        }
    }
    fn filter(&mut self,
        idx_num: i32,
        _idx_str: Option<&CStr>,
        args: &[*mut sqlite3_value]
    ) {
        let mut i = 0usize;
        if idx_num & 1 != 0 {
            self.start = unsafe{sql_call!(value_int64)(args[i])};
            i += 1;
        } else {
            self.start = 0;
        }
        if idx_num & 2 != 0{
            self.stop = unsafe{sql_call!(value_int64)(args[i])};
            i += 1;
        } else {
            self.stop = 0xffffffff;
        }
        if idx_num & 4 != 0 {
            self.step = unsafe{sql_call!(value_int64)(args[i])};
            i += 1;
            if self.step < 1 {
                self.step = 1;
            }
        } else {
            self.step = 1;
        }
        if idx_num & 8 != 0 {
            //pcur->isDesc = 1;
            self.value = self.stop;
            if self.step > 0 {
                self.value -= (self.stop - self.start) % self.step;
            }
        } else {
            //pcur->isDesc = 0;
            self.value = self.start;
        }
        self.rowid = 1;
    }
}

#[repr(C)]
#[derive(Default)]
pub struct RangeVTab {
}


#[repr(C)]
#[derive(Default)]
pub struct RangeCursor {
    rowid: i64,
    value: i64,
    start: i64,
    stop: i64,
    step: i64
}



/*
** This following structure defines all the methods for the 
** generate_series virtual table.
*/
pub static range_module : sqlite3_module = sqlite3_module {
    iVersion:       0,
    xCreate:        None,
    xConnect:       Some(range_connect::<RangeVTab>),
    xBestIndex:     Some(range_best_index::<RangeVTab>),
    xDisconnect:    Some(range_disconnect::<RangeVTab>),
    xDestroy:       None,
    xOpen:          Some(range_open::<RangeVTab>),   // open a cursor
    xClose:         Some(range_close::<RangeVTab>),  // close a cursor
    xFilter:        Some(range_filter::<RangeVTab>), // configure scan constraints
    xNext:          Some(range_next::<RangeVTab>),   // advance a cursor
    xEof:           Some(range_eof::<RangeVTab>),    // check for end of scan
    xColumn:        Some(range_column::<RangeVTab>), // read data
    xRowid:         Some(range_rowid::<RangeVTab>),  // read data
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
