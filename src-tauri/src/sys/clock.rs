//! The local wall clock, for "Paused until 10:15".

use std::time::{SystemTime, UNIX_EPOCH};

/// `struct tm` from `<time.h>` as macOS lays it out.
#[repr(C)]
struct Tm {
    sec: i32,
    min: i32,
    hour: i32,
    mday: i32,
    mon: i32,
    year: i32,
    wday: i32,
    yday: i32,
    isdst: i32,
    gmtoff: std::ffi::c_long,
    zone: *const std::ffi::c_char,
}

extern "C" {
    fn localtime_r(time: *const i64, result: *mut Tm) -> *mut Tm;
}

/// Now, in milliseconds since the Unix epoch.
pub fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

/// The local time of day at `ms` (since the Unix epoch), as "10:15" — 24-hour and Western digits
/// in both languages, as the menu and the notice show it (A2 · Paused until).
pub fn time_of_day(ms: u64) -> String {
    let secs = (ms / 1000) as i64;
    let mut tm = Tm { sec: 0, min: 0, hour: 0, mday: 0, mon: 0, year: 0, wday: 0, yday: 0, isdst: 0, gmtoff: 0, zone: std::ptr::null() };
    // SAFETY: both pointers are valid for the call; localtime_r is the reentrant variant and
    // writes only into `tm`.
    if unsafe { localtime_r(&secs, &mut tm) }.is_null() {
        return String::new();
    }
    format!("{}:{:02}", tm.hour, tm.min)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_time_of_day_has_hours_and_two_digit_minutes() {
        let shown = time_of_day(now_ms());
        let (h, m) = shown.split_once(':').unwrap();
        assert!(h.parse::<u32>().unwrap() < 24);
        assert_eq!(m.len(), 2);
        assert!(m.parse::<u32>().unwrap() < 60);
    }
}
