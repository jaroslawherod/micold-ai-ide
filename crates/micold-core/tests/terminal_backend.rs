//! T039 — TerminalBackend LaunchSpec: cwd + fresh/resume args (FR-013, FR-021).

use micold_core::terminal::{
    claude_args, FakeTerminalBackend, LaunchMode, LaunchSpec, TerminalBackend,
};
use std::path::PathBuf;
use uuid::Uuid;

fn spec(mode: LaunchMode, id: Uuid) -> LaunchSpec {
    LaunchSpec {
        cwd: PathBuf::from("/repo/.claude/worktrees/feat-x"),
        session_id: id,
        mode,
        env: vec![("TERM".to_string(), "xterm-256color".to_string())],
    }
}

#[test]
fn fresh_launch_uses_session_id_flag() {
    let id = Uuid::new_v4();
    assert_eq!(
        claude_args(&spec(LaunchMode::Fresh, id)),
        vec!["--session-id".to_string(), id.to_string()]
    );
}

#[test]
fn resume_launch_uses_resume_flag() {
    let id = Uuid::new_v4();
    assert_eq!(
        claude_args(&spec(LaunchMode::Resume, id)),
        vec!["--resume".to_string(), id.to_string()]
    );
}

#[test]
fn backend_records_launch_spec_with_worktree_cwd() {
    let backend = FakeTerminalBackend::new();
    let id = Uuid::new_v4();
    let _handle = backend.spawn(spec(LaunchMode::Fresh, id)).unwrap();

    let recorded = backend.last_spec().unwrap();
    assert_eq!(
        recorded.cwd,
        PathBuf::from("/repo/.claude/worktrees/feat-x")
    );
    assert_eq!(recorded.session_id, id);
    assert_eq!(recorded.mode, LaunchMode::Fresh);
}

#[test]
fn handle_records_input_resize_kill() {
    let backend = FakeTerminalBackend::new();
    let mut handle = backend
        .spawn(spec(LaunchMode::Fresh, Uuid::new_v4()))
        .unwrap();
    handle.write_input(b"ls\n").unwrap();
    handle.resize(40, 120).unwrap();
    handle.kill().unwrap();
    // No panic; the fake handle accepted the calls (recorded internally).
}
