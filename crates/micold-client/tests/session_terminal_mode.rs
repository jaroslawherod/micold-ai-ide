//! T003/T016/T022 — `TerminalMode`/`ShellLifecycle` transitions, `Session` defaults, the
//! mode→icon/tooltip mapping, and lifecycle independence (feature 010, FR-001–FR-010, FR-013).

use micold_core::session::{Session, SessionLocation, ShellLifecycle, TerminalMode};

fn worktree(name: &str) -> SessionLocation {
    SessionLocation::Worktree(name.to_string())
}

#[test]
fn terminal_mode_defaults_to_ai_cli() {
    assert_eq!(TerminalMode::default(), TerminalMode::AiCli);
}


#[test]
fn shell_lifecycle_defaults_to_not_started() {
    assert_eq!(ShellLifecycle::default(), ShellLifecycle::NotStarted);
}

#[test]
fn shell_lifecycle_start_shell_is_a_no_op_once_active() {
    let mut s = ShellLifecycle::Starting;
    s.start_shell();
    assert_eq!(s, ShellLifecycle::Starting, "no-op from Starting");

    let mut s = ShellLifecycle::Running;
    s.start_shell();
    assert_eq!(s, ShellLifecycle::Running, "no-op from Running");
}

#[test]
fn shell_lifecycle_start_shell_transitions_from_not_started_or_exited() {
    let mut s = ShellLifecycle::NotStarted;
    s.start_shell();
    assert_eq!(s, ShellLifecycle::Starting);

    let mut s = ShellLifecycle::Exited;
    s.start_shell();
    assert_eq!(s, ShellLifecycle::Starting);
}

#[test]
fn shell_lifecycle_mark_running_and_exited() {
    let mut s = ShellLifecycle::Starting;
    s.mark_running();
    assert_eq!(s, ShellLifecycle::Running);

    s.mark_exited();
    assert_eq!(s, ShellLifecycle::Exited);
}

#[test]
fn shell_lifecycle_is_active() {
    assert!(!ShellLifecycle::NotStarted.is_active());
    assert!(ShellLifecycle::Starting.is_active());
    assert!(ShellLifecycle::Running.is_active());
    assert!(!ShellLifecycle::Exited.is_active());
}

#[test]
fn session_start_new_defaults_mode_and_has_no_shell_instances() {
    let s = Session::start_new(worktree("feature-x"));
    assert_eq!(s.mode, TerminalMode::AiCli);
    assert!(s.shells.is_empty());
}

#[test]
fn session_restored_takes_the_persisted_mode() {
    use micold_core::session::{SessionId, SessionLabel};
    let s = Session::restored(
        SessionId::new(),
        worktree("feature-x"),
        SessionLabel::Pending,
        TerminalMode::Regular,
    );
    assert_eq!(s.mode, TerminalMode::Regular);
    assert!(s.shells.is_empty());
}

#[test]
fn session_set_mode_always_succeeds_regardless_of_process_state() {
    let mut s = Session::start_new(worktree("feature-x"));
    s.set_mode(TerminalMode::Regular);
    assert_eq!(s.mode, TerminalMode::Regular);
    s.set_mode(TerminalMode::AiCli);
    assert_eq!(s.mode, TerminalMode::AiCli);
}

// Shell *instance* mechanics (open/select/close/restart, id allocation) are covered by
// tests/session_shell_instances.rs (feature 011) — this file keeps only what's still exclusively
// its own concern: the `TerminalMode`/`ShellLifecycle` enums themselves, `set_mode`, and the
// mode -> icon/tooltip mapping below.

// --- T016 (US1): the mode -> icon/tooltip mapping ---
//
// Deleted with the control it existed for (feature 027). `icons::mode_glyph`/`mode_tooltip`
// answered "which glyph and which words does the toggle wear right now", a question only a
// control that flips between two modes can ask. The tab strip names its destination instead: the
// AI tab wears `Icon::AiCli` unconditionally, and it is the tab's own label rather than a mapping
// from state, so `tests/terminal_tabs.rs` covers it where the tab is built.

// --- T022 (feature 010, US2): set_mode never mutates the AI CLI lifecycle (FR-006) ---
// (Shell-instance-mutator independence from `lifecycle` is covered by
// tests/session_shell_instances.rs's `shell_instance_mutators_never_touch_ai_cli_lifecycle`.)

#[test]
fn set_mode_never_mutates_ai_cli_lifecycle() {
    use micold_core::session::SessionLifecycle;

    let mut s = Session::start_new(worktree("feature-x"));
    s.mark_running();
    let ai_cli_before = s.lifecycle;
    assert_eq!(ai_cli_before, SessionLifecycle::Running);

    s.set_mode(TerminalMode::Regular);
    assert_eq!(
        s.lifecycle, ai_cli_before,
        "set_mode must not touch lifecycle"
    );
}
