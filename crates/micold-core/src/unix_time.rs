//! Wall-clock Unix seconds. `clock` is the monotonic `Uptime` and stays wall-clock free; this is
//! the one place that asks the system clock for the epoch time.

/// Seconds since the Unix epoch now; `0` when the clock reads before it.
#[must_use]
pub fn unix_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}
