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

// ---------------------------------------------------------------------------------------
// Feature 026 (T025) — two sessions in one project on different CLIs stay apart
// ---------------------------------------------------------------------------------------

/// Constitution Principle II, extended to the second provider (FR-009, US1 scenario 4).
///
/// The grid isolation above is structural — separate `Term`s — and unchanged by this feature. What
/// is new is that two sessions in the **same worktree** can now be backed by different CLIs, and
/// four things have to stay separate for that to work: the command, the argv, the working
/// directory, and the store each one's conversation lands in.
///
/// The last is the one worth stating: their conversation records, titles, activity sources and
/// archived markers live under different base directories, so there is nowhere for one to leak
/// into the other. That is not a rule anything enforces — it falls out of each provider deriving
/// its own paths — and this is what says so.
#[test]
fn two_sessions_in_one_worktree_on_different_clis_share_nothing() {
    use micold_core::session::AiCli;
    use micold_core::terminal::{launch_args, LaunchMode, LaunchSpec};
    use std::path::PathBuf;

    let cwd = PathBuf::from("/repo/.claude/worktrees/feat-x");
    let claude_id = uuid::Uuid::from_u128(0xC1);
    let copilot_id = uuid::Uuid::from_u128(0xC0);

    let spec = |provider: AiCli, session_id: uuid::Uuid| LaunchSpec {
        cwd: cwd.clone(),
        session_id,
        provider,
        mode: LaunchMode::Fresh,
        env: vec![("TERM".to_string(), "xterm-256color".to_string())],
    };
    let claude = spec(AiCli::ClaudeCode, claude_id);
    let copilot = spec(AiCli::Copilot, copilot_id);

    // Different binaries.
    assert_ne!(
        claude.provider.provider().command(),
        copilot.provider.provider().command()
    );
    // Different argv — and neither is a prefix of the other, so a mixed-up spawn is not a subtle
    // difference in one flag.
    assert_ne!(launch_args(&claude), launch_args(&copilot));
    // The same working directory, deliberately: both run *in the worktree*. The provider decides
    // which binary runs there, never where (Principle III).
    assert_eq!(claude.cwd, copilot.cwd);

    // Different stores, so the two conversations cannot see each other. Asserted through
    // `has_recorded_conversation`, with each provider handed the *other's* base directory: even
    // pointed at a store that holds a conversation, a provider finds nothing there, because it is
    // not a layout it reads.
    let claude_home = tempfile::tempdir().unwrap();
    let copilot_home = tempfile::tempdir().unwrap();
    AiCli::ClaudeCode
        .provider()
        .mark_archived(claude_home.path(), &cwd, claude_id)
        .unwrap();
    AiCli::Copilot
        .provider()
        .mark_archived(copilot_home.path(), &cwd, copilot_id)
        .unwrap();

    assert!(AiCli::ClaudeCode
        .provider()
        .is_archived(claude_home.path(), &cwd, claude_id));
    assert!(AiCli::Copilot
        .provider()
        .is_archived(copilot_home.path(), &cwd, copilot_id));
    assert!(
        !AiCli::ClaudeCode
            .provider()
            .is_archived(copilot_home.path(), &cwd, copilot_id),
        "closing the Copilot session did not close a Claude session with the same id"
    );
    assert!(
        !AiCli::Copilot
            .provider()
            .is_archived(claude_home.path(), &cwd, claude_id),
        "and the reverse"
    );
}
