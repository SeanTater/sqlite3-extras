/// This quick hack allows easily creating c_string literals
macro_rules! c_str {
    ($s:expr) => { {
        concat!($s, "\0").as_ptr() as *const i8
    } }
}

/// Check that a return code is OK before passing back to SQLite
macro_rules! assert_ok {
    ($return_code: expr) => { {
        let rc = $return_code;
        if rc != SQLITE_OK {
            println!("SQLite return code {} (failure) on {}, line {}.", rc, file!(), line!());
        }
        rc
    } }
}

macro_rules! or_die {
    ($return_code: expr) => { {
        let rc = $return_code;
        if rc != SQLITE_OK {
            println!("SQLite return code {} (failure) on {}, line {}.", rc, file!(), line!());
            return rc;
        }
        rc
    } }
}

macro_rules! sql_call {
    ($function_name: tt) => { {
        use SQL_API_PTR;
        (*SQL_API_PTR).$function_name.unwrap()
    } }
}

macro_rules! create_unop {
    ($db: expr, $name:ident, $f:expr) => {
        extern "C" fn $name(ctx: *mut sqlite3_context, argc: c_int, argv: *mut *mut sqlite3_value) {
            let args = unsafe{ slice::from_raw_parts(argv, argc as usize) };
            let arg = unsafe{SQLiteValue::from_raw_unchecked(args[0])};
            let res: SQLiteReturn = $f(arg.into()).into();
            res.push_to(ctx);
        }
        sql_call!(create_function)(
            $db,
            const_cstr!(stringify!($name)).as_ptr(),
            1,
            (SQLITE_UTF8 | SQLITE_DETERMINISTIC) as i32,
            ptr::null_mut(),
            Some($name),
            None,
            None);
    }
}