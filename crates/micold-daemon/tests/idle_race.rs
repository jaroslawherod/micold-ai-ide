//! T037 [US3] — connecting *exactly* as the window expires (FR-016, lifecycle contract §4.14,
//! research R5).
//!
//! This is the race the feature creates, and it is the one a user meets by accident: they come back
//! to the machine at the thirty-minute mark and click. The daemon is deciding to stop at the same
//! instant the client is deciding to connect, and every outcome except "attached to a working
//! daemon" is a bug the user reads as the application being broken.
//!
//! There are two ways it can go wrong, and both are covered by driving the *real* client-side entry
//! point (`connect_or_spawn`) rather than a bare connect:
//!
//! - the client connects to a daemon that is already unwinding, and the connection dies under it;
//! - the client finds nothing listening, and reports a failure instead of starting one.
//!
//! `connect_or_spawn` is what the application actually calls, and it is allowed to spawn a fresh
//! daemon — that is the resolution, not a workaround. What must never happen is a `ConnectFailed`
//! reaching the user for a gap the client can close itself in under a second.

use std::path::Path;
use std::time::Duration;

use micold_core::connect::{connect_or_spawn, Connected};
use micold_core::spawn::DAEMON_BIN_ENV;
use micold_daemon::idle::IDLE_STOP_ENV;

const DAEMON_BIN: &str = env!("CARGO_BIN_EXE_micold-daemon");

fn endpoint_in(_dir: &Path) -> micold_core::endpoint::Endpoint {
    micold_core::endpoint::resolve().expect("resolve isolated endpoint")
}

/// Fifty connects, each issued as the previous daemon's window runs out.
///
/// The shape matters more than the count. Every attempt drops its connection immediately, which arms
/// the window; the test then waits a hair either side of that window before connecting again, so the
/// connect lands while the daemon is deciding to stop — some attempts a moment early, some a moment
/// late, and the ones in between exactly on it. Connecting in a tight loop instead would prove
/// nothing: fifty connects finish inside a single daemon's first window and never meet the race at
/// all.
///
/// Fifty because a race lost one time in ten passes a test that tries once, and the report this
/// guards against ("it sometimes says it can't connect") is exactly that shape.
#[tokio::test]
async fn a_connect_as_the_window_expires_ends_attached() {
    let dir = tempfile::tempdir().unwrap();
    // SAFETY: this test binary sets these before any spawn and is the only test in it.
    std::env::set_var(DAEMON_BIN_ENV, DAEMON_BIN);
    std::env::set_var("XDG_RUNTIME_DIR", dir.path());
    std::env::set_var("HOME", dir.path());
    std::env::set_var("MICOLD_LOG", "warn");
    // Every daemon this test starts — including the ones `connect_or_spawn` starts itself — stops
    // almost immediately, so no attempt gets a settled daemon by luck.
    std::env::set_var(IDLE_STOP_ENV, "150ms");

    let endpoint = endpoint_in(dir.path());

    for attempt in 1..=50 {
        // Walk the connect across the 150 ms window: 140, 145, 150, 155, 160 ms since the last
        // disconnect armed it. Two of the five land before it fires, one lands on it, two after.
        if attempt > 1 {
            let offset = 140 + (attempt % 5) * 5;
            tokio::time::sleep(Duration::from_millis(offset)).await;
        }
        match connect_or_spawn(&endpoint, "race-test", Duration::from_secs(20)).await {
            Ok(Connected::Ready(conn, _welcome)) => drop(conn),
            Ok(Connected::Refused(reason)) => {
                panic!("attempt {attempt}: refused by a daemon we just started: {reason:?}")
            }
            Err(e) => panic!(
                "attempt {attempt}: a connect issued as the window expired reached the user as a \
                 failure ({e}) — the client must close a gap this short itself (FR-016)"
            ),
        }
    }

    // Nothing to clean up, and deliberately no `pkill`: the daemons this test starts are detached,
    // so their pids are not ours to collect — but every one of them was started with a 150ms window
    // and stops itself. A pattern kill here would match the same build path a developer's own
    // running client spawned from, which is a far worse outcome than a process that exits on its own
    // a moment after the test ends.
}
