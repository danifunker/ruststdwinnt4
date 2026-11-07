//! C in a Rust program on NT 4.0: functions compiled by PINT's SDK through the cc crate,
//! using the C run-time (PINT's msvcrt.dll), called from Rust and calling back.
use std::ffi::{c_char, c_int, c_long, CStr};

extern "C" {
    fn c_sort_sum(v: *mut c_int, n: c_int) -> c_int;
    fn c_format(out: *mut c_char, size: c_int, x: f64, word: *const c_char) -> c_int;
    fn c_call_back(f: extern "C" fn(c_int) -> c_int, x: c_int) -> c_int;
    fn c_time() -> c_long;
}

extern "C" fn plus_one(x: c_int) -> c_int {
    x + 1
}

fn main() {
    let mut failures = 0;
    let mut check = |name: &str, ok: bool, got: String| {
        println!("  {name:<20} {}", if ok { "ok".to_string() } else { format!("FAILED: {got}") });
        if !ok {
            failures += 1;
        }
    };
    let mut v = [5, 3, 9, 1, 7];
    let s = unsafe { c_sort_sum(v.as_mut_ptr(), v.len() as c_int) };
    check("qsort, sum", s == 25 && v == [1, 3, 5, 7, 9], format!("{s} {v:?}"));
    let mut buf = [0 as c_char; 64];
    let n = unsafe { c_format(buf.as_mut_ptr(), 64, 2.0, c"pint".as_ptr()) };
    let text = unsafe { CStr::from_ptr(buf.as_ptr()) }.to_string_lossy().into_owned();
    check("_snprintf, strupr", n > 0 && text == "PINT:1.4142:12345", text);
    let r = unsafe { c_call_back(plus_one, 20) };
    check("callback into Rust", r == 42, r.to_string());
    let t = unsafe { c_time() };
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
    // NT 4.0's C run-time takes time() from the local time and its own idea of the time
    // zone, which can differ from Windows' by whole hours (PINT's image: 8, Pacific time);
    // only the clock itself is checked here.
    let off = t as i64 - now;
    check("time()", off.rem_euclid(3600).min(3600 - off.rem_euclid(3600)) < 5, format!("{t} vs {now}"));
    if off.abs() >= 5 {
        println!("    (C's time() is {:+} hours from Windows' UTC)", (off as f64 / 3600.0).round());
    }
    println!("cmix: {}", if failures == 0 { "ok".to_string() } else { format!("{failures} FAILED") });
    std::process::exit(failures);
}
