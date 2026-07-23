//! T027 [US1] — a daemon PTY session keeps running and producing output with no client reading it
//! (FR-001).
//!
//! This is the core persistence property at the supervisor layer: [`PtySession`] owns the child and
//! the VT `Term`, and a per-session reader thread pumps output into the grid **whether or not**
//! anything consumes it. Wiring a session to a client connection lands in Phase 4; there is
//! deliberately no connection here — which is exactly the point: output continues with no viewer.

#![cfg(unix)]

use std::time::{Duration, Instant};

use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line};
use micold_core::session::SessionId;
use micold_daemon::supervisor::PtySession;
use portable_pty::CommandBuilder;

/// The visible screen as text (rows joined by newlines), for content assertions.
fn visible_text(session: &PtySession) -> String {
    let term = session.term().lock();
    let grid = term.grid();
    let cols = grid.columns();
    let rows = grid.screen_lines();
    let mut out = String::new();
    for line in 0..rows {
        for col in 0..cols {
            out.push(grid[Line(line as i32)][Column(col)].c);
        }
        out.push('\n');
    }
    out
}

/// Count how many `tick` lines are currently on screen — a coarse "how much has been produced".
fn tick_count(session: &PtySession) -> usize {
    visible_text(session).matches("tick").count()
}

/// Poll `cond` until true or `timeout` elapses; returns whether it became true.
fn wait_until(timeout: Duration, mut cond: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if cond() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    cond()
}

#[test]
fn a_session_keeps_producing_output_with_no_client_reading() {
    // A child that emits a line ~20×/s forever. Nothing attaches to consume it.
    let mut cmd = CommandBuilder::new("sh");
    cmd.arg("-c");
    cmd.arg("while true; do echo tick; sleep 0.05; done");

    let session =
        PtySession::spawn(SessionId::new(), cmd, 10_000, Some((80, 24))).expect("spawn session");

    // The child is running and the grid is receiving output — with no client attached.
    assert!(session.is_alive(), "child should be running");
    assert!(
        wait_until(Duration::from_secs(5), || tick_count(&session) >= 1),
        "grid should receive output from the unattended child"
    );

    // Let time pass with still no consumer, then confirm output kept flowing and the child lived.
    let before = tick_count(&session);
    assert!(
        wait_until(Duration::from_secs(5), || tick_count(&session) > before
            || before >= 20),
        "output must keep arriving while detached (before={before})"
    );
    assert!(
        session.is_alive(),
        "the child must outlive the absence of any client (FR-001)"
    );

    // Test-owned process: stop it so nothing leaks.
    session.kill().expect("kill session");
    assert!(
        wait_until(Duration::from_secs(5), || !session.is_alive()),
        "killed child should be reaped"
    );
}
