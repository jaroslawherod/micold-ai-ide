//! The host's IANA time zone, passed to a sandbox as `TZ` (feature 041, R11).
//!
//! On Unix: `$TZ` when it names a zone, then the zone the `/etc/localtime` symlink points into,
//! then `/etc/timezone`. Windows keeps `iana-time-zone`, which carries the Windows-to-IANA table.

/// The host's IANA zone name, or `None` when none can be read.
#[cfg(unix)]
pub fn host() -> Option<String> {
    std::env::var("TZ")
        .ok()
        .and_then(|tz| from_tz_var(&tz))
        .or_else(|| {
            std::fs::read_link("/etc/localtime")
                .ok()
                .and_then(|target| from_zoneinfo_path(&target.to_string_lossy()))
        })
        .or_else(|| {
            std::fs::read_to_string("/etc/timezone")
                .ok()
                .and_then(|text| from_timezone_file(&text))
        })
}

#[cfg(windows)]
pub fn host() -> Option<String> {
    iana_time_zone::get_timezone()
        .ok()
        .filter(|zone| !zone.is_empty())
}

/// A `TZ` value that names a zone: `Europe/Berlin`, `:Europe/Berlin`, or a path into a zoneinfo
/// directory. A POSIX rule (`CET-1CEST,M3.5.0,M10.5.0/3`) names no zone, so it gives `None`.
#[cfg_attr(not(unix), allow(dead_code))]
fn from_tz_var(tz: &str) -> Option<String> {
    let tz = tz.strip_prefix(':').unwrap_or(tz).trim();
    if tz.starts_with('/') {
        return from_zoneinfo_path(tz);
    }
    // A rule carries an offset (`EST5EDT`) or a `<…>` name; a zone with digits has a `/` (`Etc/GMT+5`).
    let named = !tz.is_empty()
        && !tz.contains(['<', ','])
        && (tz.contains('/') || !tz.contains(|c: char| c.is_ascii_digit()));
    named.then(|| tz.to_string())
}

/// The zone a zoneinfo path names: the part after the last `zoneinfo/`. On macOS that is
/// `/var/db/timezone/zoneinfo/Europe/Berlin`; on Linux `/usr/share/zoneinfo/Europe/Berlin`.
#[cfg_attr(not(unix), allow(dead_code))]
fn from_zoneinfo_path(path: &str) -> Option<String> {
    let (_, zone) = path.rsplit_once("zoneinfo/")?;
    let zone = zone.strip_prefix("posix/").unwrap_or(zone);
    (!zone.is_empty()).then(|| zone.to_string())
}

/// The first non-comment line of a Debian-style `/etc/timezone`.
#[cfg_attr(not(unix), allow(dead_code))]
fn from_timezone_file(text: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tz_variable_naming_a_zone_is_that_zone() {
        assert_eq!(
            from_tz_var("Europe/Berlin").as_deref(),
            Some("Europe/Berlin")
        );
        assert_eq!(
            from_tz_var(":Europe/Berlin").as_deref(),
            Some("Europe/Berlin")
        );
        assert_eq!(from_tz_var("UTC").as_deref(), Some("UTC"));
        assert_eq!(
            from_tz_var("/usr/share/zoneinfo/America/New_York").as_deref(),
            Some("America/New_York")
        );
        assert_eq!(from_tz_var("Etc/GMT+5").as_deref(), Some("Etc/GMT+5"));
    }

    #[test]
    fn a_tz_variable_that_names_no_zone_gives_none() {
        assert_eq!(from_tz_var(""), None);
        assert_eq!(from_tz_var("CET-1CEST,M3.5.0,M10.5.0/3"), None);
        assert_eq!(from_tz_var("EST5EDT"), None);
        assert_eq!(from_tz_var("<+03>-3"), None);
    }

    #[test]
    fn a_localtime_link_names_the_zone_after_zoneinfo() {
        assert_eq!(
            from_zoneinfo_path("/usr/share/zoneinfo/Europe/Warsaw").as_deref(),
            Some("Europe/Warsaw")
        );
        assert_eq!(
            from_zoneinfo_path("/var/db/timezone/zoneinfo/Asia/Tokyo").as_deref(),
            Some("Asia/Tokyo")
        );
        assert_eq!(
            from_zoneinfo_path("../usr/share/zoneinfo/posix/Etc/UTC").as_deref(),
            Some("Etc/UTC")
        );
        assert_eq!(from_zoneinfo_path("/etc/localtime.real"), None);
        assert_eq!(from_zoneinfo_path("/usr/share/zoneinfo/"), None);
    }

    #[test]
    fn a_timezone_file_gives_its_first_zone_line() {
        assert_eq!(
            from_timezone_file("# set by installer\n\nEurope/Paris\n").as_deref(),
            Some("Europe/Paris")
        );
        assert_eq!(from_timezone_file("\n# only a comment\n"), None);
    }
}
