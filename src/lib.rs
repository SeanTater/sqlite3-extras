//! SQLite3 extras: powerups for the world's favorite database
//! String Operations
//! =================
//! 
//!
//! Trigonometrics and Exponentials
//! ========
//! See Rust's builtin f64 for more detail
//! 
//! - `sin()`
//! - `asin(x)`
//! - `sinh(x)`
//! - `asinh(x)`
//! - `cos(x)`
//! - `acos(x)`
//! - `cosh(x)`
//! - `acosh(x)`
//! - `tan(x)`
//! - `atan(x)`
//! - `tanh(x)`
//! - `atanh(x)`
//! - `ln(x)`
//! - `ln_1p(x)`
//! - `log2(x)`
//! - `log10(x)`
//! - `exp(x)`
//! - `exp2(x)`
//! - `exp_m1(x)`
//! - `to_degrees(x)`
//! - `to_radians(x)`
//! - `sqrt(x)`
//! - `cbrt(x)`
//!
mod sqlite3_raw;
#[macro_use] mod macros;
pub mod virtual_table;

#[macro_use] extern crate const_cstr;
extern crate libc;
extern crate nodrop;
extern crate smallvec;

use std::ptr;
use std::os::raw::*;
use std::slice;
use const_cstr::ConstCStr;
use sqlite3_raw::*;

/// How to push and pull types from SQLite (for extensions)
pub trait SQLType {
    fn from_sqlite(arg: *mut sqlite3_value) -> Self;
    fn to_sqlite(&self, ctx: *mut sqlite3_context);
}

impl SQLType for f64 {
    fn from_sqlite(arg: *mut sqlite3_value) -> Self {
        unsafe{ sqlite3_value_double(arg) }
    }
    fn to_sqlite(&self, ctx: *mut sqlite3_context) {
        unsafe{ sqlite3_result_double(ctx, *self) };
    }
}

static mut SQL_API_PTR : *mut sqlite3_api_routines = ptr::null_mut();


#[no_mangle]
pub unsafe extern "C" fn sqlite3_extension_init(db: *mut sqlite3, err: *mut *mut c_char, api: *mut sqlite3_api_routines) -> i32 {
    SQL_API_PTR = api;
    
    create_unop!(db, sin, f64::sin);
    create_unop!(db, asin, f64::asin);
    create_unop!(db, sinh, f64::sinh);
    create_unop!(db, asinh, f64::asinh);
    create_unop!(db, cos, f64::cos);
    create_unop!(db, acos, f64::acos);
    create_unop!(db, cosh, f64::cosh);
    create_unop!(db, acosh, f64::acosh);
    create_unop!(db, tan, f64::tan);
    create_unop!(db, atan, f64::atan);
    create_unop!(db, tanh, f64::tanh);
    create_unop!(db, atanh, f64::atanh);
    create_unop!(db, ln, f64::ln);
    create_unop!(db, ln_1p, f64::ln_1p);
    create_unop!(db, log2, f64::log2);
    create_unop!(db, log10, f64::log10);
    create_unop!(db, exp, f64::exp);
    create_unop!(db, exp2, f64::exp2);
    create_unop!(db, exp_m1, f64::exp_m1);
    create_unop!(db, to_degrees, f64::to_degrees);
    create_unop!(db, to_radians, f64::to_radians);
    create_unop!(db, sqrt, f64::sqrt);
    create_unop!(db, cbrt, f64::cbrt);
    
//    
//    def_plain(const_cstr!("is_finite"), sql_is_finite);
//    def_plain(const_cstr!("is_infinite"), sql_is_infinite);
//    def_plain(const_cstr!("is_normal"), sql_is_infinite);
    
    
    if sqlite3_libversion_number() < 3008012 {
        *err = sqlite3_mprintf(const_cstr!("generate_series() requires SQLite 3.8.12 or later").as_ptr());
        return SQLITE_ERROR;
    } else {
        return assert_ok!(sql_call!(create_module)(db, const_cstr!("generate_series").as_ptr(), &virtual_table::range::range_module, ptr::null_mut()));
    }
}

//
//create_unop!(sql_is_finite, "is_finite", |x:f64| x.is_finite() as i64 as f64);
//create_unop!(sql_is_infinite, "is_infinite", |x:f64| x.is_infinite() as i64 as f64);
//create_unop!(sql_is_normal, "is_normal", |x:f64| x.is_normal() as i64 as f64);

//#[cfg(test)]
//mod tests {
//    use do_nothing;
//    #[test]
//    fn it_works() {
//        assert_eq!(2 + 2, 4);
//        assert_eq!(do_nothing(), 3);
//    }
//}
