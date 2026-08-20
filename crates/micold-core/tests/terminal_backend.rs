//! T039 — TerminalBackend LaunchSpec: cwd + fresh/resume args (FR-013, FR-021), and the argv a
//! launch actually builds (feature 026, T007a — FR-007).
//!
//! # Why this file is a gate on the seam and not only on the backend
//!
//! The launch path was the half of the seam nothing reached. `LaunchSpec` had no provider field
//! and the argument builder was called `claude_args` and ignored the spec entirely — it named one
//! provider and every session got that one's argv. So "take the provider from the record" had
//! nothing to take it *from*: a struct had to gain a field and a function had to lose its name
//! before the sentence meant anything here.
//!
//! That is a different defect from the one `no_concrete_implementations` finds. That guard looks
//! for a *name*, and it does see the line in `terminal.rs` — but deleting the name would not have
//! fixed anything, because there was nowhere for the daemon's two spawn sites to put an answer.
//! This file asserts the shape instead: the spec carries the provider, and the argv follows it.

use micold_core::session::AiCli;
use micold_core::terminal::{
    launch_args, FakeTerminalBackend, LaunchMode, LaunchSpec, TerminalBackend,
};
use std::path::PathBuf;
use uuid::Uuid;

fn spec(mode: LaunchMode, id: Uuid) -> LaunchSpec {
    spec_for(AiCli::ClaudeCode, mode, id)
}

fn spec_for(provider: AiCli, mode: LaunchMode, id: Uuid) -> LaunchSpec {
    LaunchSpec {
        cwd: PathBuf::from("/repo/.claude/worktrees/feat-x"),
        session_id: id,
        provider,
        mode,
        env: vec![("TERM".to_string(), "xterm-256color".to_string())],
    }
}

#[test]
fn fresh_launch_uses_session_id_flag() {
    let id = Uuid::new_v4();
    assert_eq!(
        launch_args(&spec(LaunchMode::Fresh, id)),
        vec!["--session-id".to_string(), id.to_string()]
    );
}

#[test]
fn resume_launch_uses_resume_flag() {
    let id = Uuid::new_v4();
    assert_eq!(
        launch_args(&spec(LaunchMode::Resume, id)),
        vec!["--resume".to_string(), id.to_string()]
    );
}

#[test]
fn the_argv_is_built_from_the_specs_own_provider() {
    // T007a. Same id, same mode, same cwd — only `provider` differs, and the argv is different in
    // shape rather than in a flag: Copilot glues its id to `--resume` and adds `--no-remote`.
    // Nothing here names a provider *type*; the spec carries a name and the seam resolves it.
    let id = Uuid::new_v4();

    assert_eq!(
        launch_args(&spec_for(AiCli::Copilot, LaunchMode::Fresh, id)),
        vec![
            "--session-id".to_string(),
            id.to_string(),
            "--no-remote".to_string(),
        ]
    );
    assert_eq!(
        launch_args(&spec_for(AiCli::Copilot, LaunchMode::Resume, id)),
        vec![format!("--resume={id}"), "--no-remote".to_string()]
    );

    // And Claude Code's is byte-for-byte what it was before the field existed. This is the half
    // that would go unnoticed: a generalisation that quietly changed provider one's argv would
    // break every existing session's resume with no test saying so.
    assert_eq!(
        launch_args(&spec_for(AiCli::ClaudeCode, LaunchMode::Fresh, id)),
        vec!["--session-id".to_string(), id.to_string()]
    );
    assert_eq!(
        launch_args(&spec_for(AiCli::ClaudeCode, LaunchMode::Resume, id)),
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
    assert_eq!(
        recorded.provider,
        AiCli::ClaudeCode,
        "the backend is handed which CLI to spawn, not left to assume one"
    );
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
