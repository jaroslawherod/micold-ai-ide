//! A monotonic, suspend-inclusive clock (feature 028, data-model G3).
//!
//! The idle rule in `micold-daemon` measures 30 continuous minutes without a client. "Continuous"
//! has to include the time the machine spends asleep — a laptop closed for an hour with nothing
//! connected has been idle for an hour — which rules out `std::time::Instant` on every platform
//! this ships to: it stops on suspend on Linux and macOS.
//!
//! So the reading is the one thing in this feature that is platform-split, behind a single
//! `cfg`-free signature (Constitution Principle VI): `CLOCK_BOOTTIME` on Linux,
//! `mach_continuous_time()` on macOS, `GetTickCount64()` on Windows.
//!
//! # Why a newtype and not a `Duration`
//!
//! [`Uptime`] is a *reading*, not a length. Subtracting two readings gives a `Duration`; adding one
//! to a wall-clock time is meaningless, and the type makes it impossible rather than merely
//! discouraged. That is the whole guard against the failure research R3 names: a clock correction
//! must not move the deadline, and the surest way to keep wall-clock time out of the rule is to
//! give the rule a type that cannot hold it.

use std::time::Duration;

/// Nanoseconds since the machine booted, on a clock that keeps counting while it sleeps.
///
/// Not persisted, not sent on the wire, not rendered (G3) — it means nothing outside the process
/// that read it, because it is relative to a boot this process may not have seen the start of.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Uptime(u64);

impl Uptime {
    /// A reading built from a raw nanosecond count.
    ///
    /// Public because the rule that reads this clock has a thirty-minute window, and a test that
    /// could only obtain readings from [`now`] would have to wait for it. Naming nanoseconds
    /// *since boot* is what keeps it from being mistaken for a way in for wall-clock time: there is
    /// no wall-clock value a caller could pass here and get a meaningful answer back.
    pub const fn from_nanos(nanos: u64) -> Self {
        Self(nanos)
    }

    /// How much later this reading is than `earlier`, or zero if it is not later.
    ///
    /// Saturating rather than panicking on purpose. The clock is monotonic, so a negative result is
    /// not supposed to be reachable — but "not supposed to be" is not a reason for the daemon to
    /// abort. Zero is also the safe direction: an underflow reads as *no time has passed*, which
    /// delays a stop rather than causing an early one.
    pub fn saturating_sub(self, earlier: Self) -> Duration {
        Duration::from_nanos(self.0.saturating_sub(earlier.0))
    }
}

/// Read the clock.
///
/// One signature, three implementations, no `cfg` visible to any caller (Principle VI). Infallible
/// on all three: each underlying call either cannot fail or fails only on arguments this code does
/// not pass, so a caller has no error to handle and the daemon has no "clock unavailable" branch.
pub fn now() -> Uptime {
    Uptime(read_nanos())
}

/// `CLOCK_BOOTTIME` — monotonic, and unlike `CLOCK_MONOTONIC` it advances across suspend.
#[cfg(target_os = "linux")]
fn read_nanos() -> u64 {
    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // Cannot fail for a valid clock id and a non-null destination, both of which are literals here.
    unsafe { libc::clock_gettime(libc::CLOCK_BOOTTIME, &mut ts) };
    (ts.tv_sec as u64) * 1_000_000_000 + (ts.tv_nsec as u64)
}

/// `mach_continuous_time()` — the suspend-inclusive counterpart of `mach_absolute_time()`.
///
/// The raw value is in Mach ticks, which are nanoseconds only when `timebase.numer == denom`; it is
/// converted rather than assumed, because on some Apple silicon it is not.
#[cfg(target_os = "macos")]
fn read_nanos() -> u64 {
    let mut timebase = libc::mach_timebase_info_data_t { numer: 0, denom: 0 };
    unsafe { libc::mach_timebase_info(&mut timebase) };
    let ticks = unsafe { libc::mach_continuous_time() };
    (ticks as u128 * timebase.numer as u128 / timebase.denom.max(1) as u128) as u64
}

/// `GetTickCount64()` — milliseconds since boot, and on Windows that count includes sleep.
#[cfg(windows)]
fn read_nanos() -> u64 {
    let millis = unsafe { windows_sys::Win32::System::SystemInformation::GetTickCount64() };
    millis.saturating_mul(1_000_000)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// G3's first property. Asserted over many consecutive reads rather than two, because a clock
    /// that steps backwards does it under contention or across a core migration, not on demand.
    #[test]
    fn the_clock_never_moves_backwards() {
        let mut previous = now();
        for _ in 0..10_000 {
            let current = now();
            assert!(
                current >= previous,
                "the clock went backwards: {previous:?} then {current:?}"
            );
            previous = current;
        }
    }

    /// …and it does move *forwards*, or the window would never expire. A clock stuck at a constant
    /// would satisfy the monotonicity test above perfectly.
    #[test]
    fn the_clock_advances() {
        let before = now();
        std::thread::sleep(Duration::from_millis(20));
        let after = now();
        assert!(
            after.saturating_sub(before) >= Duration::from_millis(10),
            "20ms of sleep moved the clock by {:?}",
            after.saturating_sub(before)
        );
    }

    /// It reads since **boot**, not since this process started — which is what makes it usable for
    /// a window that a daemon inherits rather than begins.
    ///
    /// A minute is far below any machine that has finished booting and started a test runner, and
    /// far above anything a process-relative clock would report here.
    #[test]
    fn the_reading_is_since_boot() {
        assert!(
            now().saturating_sub(Uptime::from_nanos(0)) > Duration::from_secs(60),
            "this looks like a process-relative clock, not a since-boot one"
        );
    }

    /// G3's second property, and the reason `saturating_sub` is spelled out rather than `-`.
    ///
    /// The clock is monotonic, so this is not supposed to happen — but a daemon that panics because
    /// something that cannot happen happened is a worse outcome than one that reads zero and waits.
    #[test]
    fn subtracting_a_later_reading_yields_zero_rather_than_panicking() {
        let earlier = Uptime::from_nanos(1_000);
        let later = Uptime::from_nanos(9_000);
        assert_eq!(earlier.saturating_sub(later), Duration::ZERO);
        assert_eq!(later.saturating_sub(earlier), Duration::from_nanos(8_000));
    }

    /// Suspend-inclusiveness is the one property of this module a unit test cannot reach: it needs
    /// a machine that actually sleeps. Quickstart B3 is where it is measured. What *is* checkable
    /// here is that the reading is not `Instant`-shaped — see `the_reading_is_since_boot`.
    #[test]
    fn equal_readings_are_zero_apart() {
        let reading = now();
        assert_eq!(reading.saturating_sub(reading), Duration::ZERO);
    }
}
