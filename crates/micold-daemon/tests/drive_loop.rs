//! T039/US2 (minimal drive loop) — input stamped under the client contract reaches a live session's
//! PTY, in order, through the daemon's per-session [`InputReceiver`] (protocol.md §7, data-model G2,
//! FR-019).
//!
//! This is the daemon half of the drive loop: `ClientMsg::SessionInput { serial, bytes }` →
//! `DaemonState::session_input` → the receiver classifies the serial → the bytes are written to the
//! PTY. The *classification* rules (monotonic, gap = loss, stale = dropped) are proven in isolation
//! by `micold-core`'s `input_ordering` tests against the very same `InputReceiver`; here we prove the
//! wiring — that an Applied batch actually lands on the PTY, and in order. The client transport half
//! (opening a connection and stamping via `InputSeq`) lands with the client retarget, T041/T044.

#![cfg(unix)]

use std::time::{Duration, Instant};

use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line};
use micold_core::session::SessionId;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use micold_daemon::supervisor::PtySession;
use portable_pty::CommandBuilder;

/// The visible screen as one string, for content assertions.
fn visible_text(session: &PtySession) -> String {
    let term = session.term().lock();
    let grid = term.grid();
    let (cols, rows) = (grid.columns(), grid.screen_lines());
    let mut out = String::new();
    for line in 0..rows {
        for col in 0..cols {
            out.push(grid[Line(line as i32)][Column(col)].c);
        }
        out.push('\n');
    }
    out
}

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

/// A `cat` session echoes back exactly what is written to its PTY — a deterministic sink for
/// asserting that driven input reached the process.
fn spawn_cat(state: &DaemonState) -> (SessionId, std::sync::Arc<PtySession>) {
    let id = SessionId::new();
    let mut cmd = CommandBuilder::new("cat");
    cmd.cwd(std::env::temp_dir());
    let session = PtySession::spawn(id, cmd, 1_000, Some((80, 24))).expect("spawn cat session");
    let handle = state.register_session(session);
    (id, handle)
}

#[test]
fn driven_input_reaches_the_live_session_pty() {
    let state = DaemonState::new(Catalog::ephemeral());
    let (id, session) = spawn_cat(&state);

    // Stamp exactly as the client would: the first per-session serial is 0.
    state.session_input(id, 0, b"driveloop_marker\n");

    assert!(
        wait_until(Duration::from_secs(5), || visible_text(&session)
            .contains("driveloop_marker")),
        "input driven through session_input must reach the PTY and echo back:\n{}",
        visible_text(&session)
    );

    session.kill().expect("kill session");
}

#[test]
fn in_order_input_batches_arrive_in_order() {
    let state = DaemonState::new(Catalog::ephemeral());
    let (id, session) = spawn_cat(&state);

    // Three contiguous, in-order batches. The append-only log must preserve their order on the PTY.
    for (serial, ch) in [(0u64, "AAA"), (1, "BBB"), (2, "CCC")] {
        state.session_input(id, serial, format!("{ch}\n").as_bytes());
    }

    assert!(
        wait_until(Duration::from_secs(5), || {
            let t = visible_text(&session);
            t.contains("AAA") && t.contains("BBB") && t.contains("CCC")
        }),
        "all in-order batches must arrive"
    );

    // Order preserved: AAA before BBB before CCC in the echoed stream.
    let text = visible_text(&session);
    let a = text.find("AAA").unwrap();
    let b = text.find("BBB").unwrap();
    let c = text.find("CCC").unwrap();
    assert!(a < b && b < c, "batches must not be reordered:\n{text}");

    session.kill().expect("kill session");
}

#[test]
fn input_for_an_unknown_session_is_a_no_op() {
    // A session the daemon is not hosting: dropping its input must not panic (it is logged loudly).
    let state = DaemonState::new(Catalog::ephemeral());
    state.session_input(SessionId::new(), 0, b"nowhere\n");
    assert!(
        state.live_session(SessionId::new()).is_none(),
        "no session was registered"
    );
}
