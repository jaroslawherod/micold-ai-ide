//! Wall-clock dates and times in a UTC offset, and the local zone's offset (issue #877).
//!
//! The calendar arithmetic is pure: [`LocalTime::at`] turns Unix seconds and a [`UtcOffset`] into
//! a civil date and time, so a test pins both. The one read of the host's zone is
//! [`UtcOffset::local_at`]: `localtime_r(3)` on Unix, `SystemTimeToTzSpecificLocalTime` on
//! Windows. Both apply the daylight-saving rule in force *at that instant*, so an offset is always
//! asked for the time it will format, never assumed from "now".

use std::fmt;

/// The latest Unix second this module formats: 9999-12-31 23:59:59 UTC. Later times, and times
/// before year 1, are out of range.
pub const MAX_UNIX_SECS: i64 = 253_402_300_799;
/// The earliest: 0001-01-01 00:00:00 UTC.
pub const MIN_UNIX_SECS: i64 = -62_135_596_800;

/// Whether `secs` is a Unix time this module can show as a date.
pub fn in_range(secs: i64) -> bool {
    (MIN_UNIX_SECS..=MAX_UNIX_SECS).contains(&secs)
}

/// Seconds east of UTC, within a day either side.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UtcOffset(i32);

impl UtcOffset {
    /// UTC itself.
    pub const UTC: UtcOffset = UtcOffset(0);

    /// `secs` east of UTC; `None` past a day either way.
    pub fn east(secs: i32) -> Option<Self> {
        (secs.abs() < 86_400).then_some(Self(secs))
    }

    /// `secs` west of UTC; `None` past a day either way.
    pub fn west(secs: i32) -> Option<Self> {
        Self::east(secs.checked_neg()?)
    }

    /// Seconds east of UTC.
    pub fn secs(self) -> i32 {
        self.0
    }

    /// The host zone's offset at the instant `secs`, daylight saving included; UTC when the
    /// zone cannot be read.
    pub fn local_at(secs: i64) -> Self {
        host_offset(secs).and_then(Self::east).unwrap_or(Self::UTC)
    }

    /// The host zone's offset now.
    pub fn local_now() -> Self {
        Self::local_at(now_secs())
    }
}

/// `+HH:MM` or `-HH:MM`, as RFC 3339 and `%:z` write an offset.
impl fmt::Display for UtcOffset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.0 < 0 { '-' } else { '+' };
        let minutes = self.0.unsigned_abs() / 60;
        write!(f, "{sign}{:02}:{:02}", minutes / 60, minutes % 60)
    }
}

/// A civil date and time, to the second, in one [`UtcOffset`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocalTime {
    pub year: i32,
    /// 1–12.
    pub month: u8,
    /// 1–31.
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    /// 0 for Monday through 6 for Sunday.
    pub weekday: u8,
    pub offset: UtcOffset,
}

impl LocalTime {
    /// The civil time of Unix second `secs` in `offset`. A `secs` out of [`in_range`] is clamped
    /// into it.
    pub fn at(secs: i64, offset: UtcOffset) -> Self {
        let local = secs.clamp(MIN_UNIX_SECS, MAX_UNIX_SECS) + i64::from(offset.0);
        let days = local.div_euclid(86_400);
        let of_day = local.rem_euclid(86_400);
        let (year, month, day) = civil_from_days(days);
        Self {
            year,
            month,
            day,
            hour: (of_day / 3600) as u8,
            minute: (of_day / 60 % 60) as u8,
            second: (of_day % 60) as u8,
            // 1970-01-01 was a Thursday, index 3.
            weekday: (days + 3).rem_euclid(7) as u8,
            offset,
        }
    }

    /// The civil time of `secs` in the host's zone, with the offset in force then.
    pub fn local(secs: i64) -> Self {
        Self::at(secs, UtcOffset::local_at(secs))
    }

    /// The host's civil time now.
    pub fn now() -> Self {
        Self::local(now_secs())
    }

    /// `HH:MM` (`%H:%M`).
    pub fn hh_mm(&self) -> String {
        format!("{:02}:{:02}", self.hour, self.minute)
    }

    /// The English weekday abbreviation (`%a`): `Mon` … `Sun`.
    pub fn weekday_abbr(&self) -> &'static str {
        ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"][usize::from(self.weekday)]
    }

    /// `YYYY-MM-DD HH:MM` (`%Y-%m-%d %H:%M`).
    pub fn date_hh_mm(&self) -> String {
        format!(
            "{:04}-{:02}-{:02} {}",
            self.year,
            self.month,
            self.day,
            self.hh_mm()
        )
    }
}

/// `secs` as a UTC calendar date, `2026-11-20`.
pub fn utc_date(secs: u64) -> String {
    rfc3339_utc(i64::try_from(secs).unwrap_or(i64::MAX))
        .chars()
        .take(10)
        .collect()
}

/// `secs` in UTC as RFC 3339 to the second with a `Z`: `2026-10-02T12:31:00Z`.
pub fn rfc3339_utc(secs: i64) -> String {
    let t = LocalTime::at(secs, UtcOffset::UTC);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        t.year, t.month, t.day, t.hour, t.minute, t.second
    )
}

/// A `SystemTime` as signed Unix seconds, before the epoch included.
pub fn unix_secs_of(time: std::time::SystemTime) -> i64 {
    match time.duration_since(std::time::UNIX_EPOCH) {
        Ok(after) => i64::try_from(after.as_secs()).unwrap_or(i64::MAX),
        Err(before) => i64::try_from(before.duration().as_secs()).map_or(i64::MIN, |s| -s),
    }
}

fn now_secs() -> i64 {
    unix_secs_of(std::time::SystemTime::now())
}

/// Howard Hinnant's `civil_from_days`: the proleptic Gregorian date `days` after 1970-01-01.
fn civil_from_days(days: i64) -> (i32, u8, u8) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u8;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u8;
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year as i32, month, day)
}

#[cfg(unix)]
fn host_offset(secs: i64) -> Option<i32> {
    // musl 1.2 made `time_t` 64-bit and the libc crate deprecated the alias for it
    // (rust-lang/libc#1848), so musl takes the `i64` directly; `libc::localtime_r` is the same call.
    #[cfg(target_env = "musl")]
    let t: i64 = secs;
    #[cfg(not(target_env = "musl"))]
    let t = libc::time_t::try_from(secs).ok()?;
    let mut tm = std::mem::MaybeUninit::<libc::tm>::uninit();
    // SAFETY: both pointers are valid for the call; `localtime_r` writes only through `tm`.
    if unsafe { libc::localtime_r(&t, tm.as_mut_ptr()) }.is_null() {
        return None;
    }
    // SAFETY: `localtime_r` returned non-null, so it filled `tm`.
    let tm = unsafe { tm.assume_init() };
    i32::try_from(tm.tm_gmtoff).ok()
}

#[cfg(windows)]
fn host_offset(secs: i64) -> Option<i32> {
    use windows_sys::Win32::Foundation::{FILETIME, SYSTEMTIME};
    use windows_sys::Win32::System::Time::{
        FileTimeToSystemTime, SystemTimeToFileTime, SystemTimeToTzSpecificLocalTime,
    };
    // FILETIME counts 100 ns ticks from 1601-01-01.
    const TICKS_PER_SEC: i64 = 10_000_000;
    const EPOCH_DIFF_SECS: i64 = 11_644_473_600;
    let ticks = u64::try_from((secs + EPOCH_DIFF_SECS).checked_mul(TICKS_PER_SEC)?).ok()?;
    let as_filetime = |t: u64| FILETIME {
        dwLowDateTime: t as u32,
        dwHighDateTime: (t >> 32) as u32,
    };
    let utc_ft = as_filetime(ticks);
    // SAFETY: zeroed SYSTEMTIME/FILETIME are valid plain structs; each call reads and writes only
    // through the pointers given, and a null zone means the zone currently in effect.
    unsafe {
        let mut utc: SYSTEMTIME = std::mem::zeroed();
        let mut local: SYSTEMTIME = std::mem::zeroed();
        let mut local_ft: FILETIME = std::mem::zeroed();
        if FileTimeToSystemTime(&utc_ft, &mut utc) == 0
            || SystemTimeToTzSpecificLocalTime(std::ptr::null(), &utc, &mut local) == 0
            || SystemTimeToFileTime(&local, &mut local_ft) == 0
        {
            return None;
        }
        let local_ticks =
            (u64::from(local_ft.dwHighDateTime) << 32) | u64::from(local_ft.dwLowDateTime);
        let diff = (local_ticks as i64 - ticks as i64) / TICKS_PER_SEC;
        i32::try_from(diff).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CEST: UtcOffset = UtcOffset(2 * 3600);

    #[test]
    fn a_time_in_an_offset_is_its_civil_date_and_time() {
        // 2026-10-02 12:31:00 UTC.
        let t = LocalTime::at(1_790_944_260, CEST);
        assert_eq!(
            (t.year, t.month, t.day, t.hour, t.minute, t.second),
            (2026, 10, 2, 14, 31, 0)
        );
        assert_eq!(t.weekday_abbr(), "Fri");
        assert_eq!(t.date_hh_mm(), "2026-10-02 14:31");
        assert_eq!(t.hh_mm(), "14:31");
    }

    #[test]
    fn an_offset_can_move_the_date_back_or_forward() {
        let ny = UtcOffset::west(4 * 3600).unwrap();
        // 2026-10-02 01:00 UTC is the evening before in New York.
        let t = LocalTime::at(1_790_904_000, ny);
        assert_eq!((t.month, t.day, t.hour), (10, 1, 21));
        assert_eq!(t.weekday_abbr(), "Thu");
        let tokyo = UtcOffset::east(9 * 3600).unwrap();
        let t = LocalTime::at(1_790_944_260 + 12 * 3600, tokyo);
        assert_eq!((t.day, t.hour, t.minute), (3, 9, 31));
    }

    #[test]
    fn leap_days_epoch_edges_and_the_range_ends_are_dated_right() {
        let t = LocalTime::at(951_782_400, UtcOffset::UTC);
        assert_eq!(
            (t.year, t.month, t.day, t.weekday_abbr()),
            (2000, 2, 29, "Tue")
        );
        let t = LocalTime::at(-1, UtcOffset::UTC);
        assert_eq!(
            (t.year, t.month, t.day, t.hour, t.minute, t.second),
            (1969, 12, 31, 23, 59, 59)
        );
        assert_eq!(t.weekday_abbr(), "Wed");
        assert_eq!(rfc3339_utc(MAX_UNIX_SECS), "9999-12-31T23:59:59Z");
        assert_eq!(rfc3339_utc(MIN_UNIX_SECS), "0001-01-01T00:00:00Z");
        assert!(in_range(MAX_UNIX_SECS) && !in_range(MAX_UNIX_SECS + 1));
        assert!(in_range(MIN_UNIX_SECS) && !in_range(MIN_UNIX_SECS - 1));
    }

    #[test]
    fn rfc3339_is_utc_to_the_second_with_a_z() {
        assert_eq!(rfc3339_utc(1_790_944_260), "2026-10-02T12:31:00Z");
        assert_eq!(rfc3339_utc(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn an_offset_prints_as_rfc3339_writes_it() {
        assert_eq!(CEST.to_string(), "+02:00");
        assert_eq!(UtcOffset::UTC.to_string(), "+00:00");
        assert_eq!(
            UtcOffset::west(5 * 3600 + 30 * 60).unwrap().to_string(),
            "-05:30"
        );
        assert_eq!(UtcOffset::east(86_400), None);
    }

    #[test]
    fn system_times_before_the_epoch_are_negative_seconds() {
        use std::time::{Duration, UNIX_EPOCH};
        assert_eq!(unix_secs_of(UNIX_EPOCH + Duration::from_secs(5)), 5);
        assert_eq!(unix_secs_of(UNIX_EPOCH - Duration::from_secs(5)), -5);
    }

    /// The host offset agrees with the system's own `date +%z` for the same instant.
    #[cfg(unix)]
    #[test]
    fn the_local_offset_matches_the_systems_date_tool() {
        let secs = 1_790_944_260;
        // Ask `date` for the offset at the same instant: GNU takes `-d @secs`, BSD `-r secs`.
        let gnu = std::process::Command::new("date")
            .args(["-d", &format!("@{secs}"), "+%z"])
            .output();
        let out = match gnu {
            Ok(o) if o.status.success() => o,
            _ => std::process::Command::new("date")
                .args(["-r", &secs.to_string(), "+%z"])
                .output()
                .expect("date runs"),
        };
        let z = String::from_utf8_lossy(&out.stdout).trim().to_string();
        let expected = format!("{}:{}", &z[..3], &z[3..]);
        assert_eq!(UtcOffset::local_at(secs).to_string(), expected);
    }
}
