//! T029 [US1] — two sessions' grids never cross-contaminate (Constitution Principle II).
//!
//! Each [`PtySession`] owns a separate `Term`, so isolation is structural (no shared buffer). This
//! replaces the old in-memory `SessionRouter` byte-routing approximation (removed in T030) with an
//! end-to-end check against two real VT sessions.

#![cfg(unix)]

use std::time::{Duration, Instant};

use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line};
use micold_core::session::SessionId;
use micold_daemon::supervisor::PtySession;
use portable_pty::CommandBuilder;

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

fn wait_for(session: &PtySession, needle: &str) -> bool {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if visible_text(session).contains(needle) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    visible_text(session).contains(needle)
}

fn echo_then_idle(marker: &str) -> CommandBuilder {
    let mut cmd = CommandBuilder::new("sh");
    cmd.arg("-c");
    // Print a distinctive marker, then idle so the session stays alive for inspection.
    cmd.arg(format!("echo {marker}; sleep 5"));
    cmd
}

#[test]
fn two_sessions_do_not_leak_into_each_other() {
    let a = PtySession::spawn(
        SessionId::new(),
        echo_then_idle("AAAAA"),
        10_000,
        Some((80, 24)),
    )
    .expect("spawn A");
    let b = PtySession::spawn(
        SessionId::new(),
        echo_then_idle("BBBBB"),
        10_000,
        Some((80, 24)),
    )
    .expect("spawn B");

    assert!(
        wait_for(&a, "AAAAA"),
        "session A should show its own output"
    );
    assert!(
        wait_for(&b, "BBBBB"),
        "session B should show its own output"
    );

    let a_text = visible_text(&a);
    let b_text = visible_text(&b);

    assert!(a_text.contains("AAAAA"), "A has its marker");
    assert!(!a_text.contains("BBBBB"), "A must NOT contain B's output");
    assert!(b_text.contains("BBBBB"), "B has its marker");
    assert!(!b_text.contains("AAAAA"), "B must NOT contain A's output");

    a.kill().expect("kill A");
    b.kill().expect("kill B");
}
