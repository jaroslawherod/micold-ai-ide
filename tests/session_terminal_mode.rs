//! T003/T016/T022 — `TerminalMode`/`ShellLifecycle` transitions, `Session` defaults, the
//! mode→icon/tooltip mapping, and lifecycle independence (feature 010, FR-001–FR-010, FR-013).

use micold_ai_ide::icons::Icon;
use micold_ai_ide::session::{Session, SessionLocation, ShellLifecycle, TerminalMode};

fn worktree(name: &str) -> SessionLocation {
    SessionLocation::Worktree(name.to_string())
}

#[test]
fn terminal_mode_defaults_to_ai_cli() {
    assert_eq!(TerminalMode::default(), TerminalMode::AiCli);
}

#[test]
fn terminal_mode_other_toggles_both_directions() {
    assert_eq!(TerminalMode::AiCli.other(), TerminalMode::Regular);
    assert_eq!(TerminalMode::Regular.other(), TerminalMode::AiCli);
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
fn session_start_new_defaults_mode_and_shell_lifecycle() {
    let s = Session::start_new(worktree("feature-x"));
    assert_eq!(s.mode, TerminalMode::AiCli);
    assert_eq!(s.shell_lifecycle, ShellLifecycle::NotStarted);
}

#[test]
fn session_restored_takes_the_persisted_mode() {
    use micold_ai_ide::session::{SessionId, SessionLabel};
    let s = Session::restored(
        SessionId::new(),
        worktree("feature-x"),
        SessionLabel::Pending,
        TerminalMode::Regular,
    );
    assert_eq!(s.mode, TerminalMode::Regular);
    assert_eq!(s.shell_lifecycle, ShellLifecycle::NotStarted);
}

#[test]
fn session_set_mode_always_succeeds_regardless_of_process_state() {
    let mut s = Session::start_new(worktree("feature-x"));
    s.set_mode(TerminalMode::Regular);
    assert_eq!(s.mode, TerminalMode::Regular);
    s.set_mode(TerminalMode::AiCli);
    assert_eq!(s.mode, TerminalMode::AiCli);
}

#[test]
fn session_shell_transitions_delegate_to_shell_lifecycle() {
    let mut s = Session::start_new(worktree("feature-x"));
    s.start_shell();
    assert_eq!(s.shell_lifecycle, ShellLifecycle::Starting);
    s.mark_shell_running();
    assert_eq!(s.shell_lifecycle, ShellLifecycle::Running);
    s.mark_shell_exited();
    assert_eq!(s.shell_lifecycle, ShellLifecycle::Exited);
}

// --- T016 (US1): the mode -> icon/tooltip mapping is total and distinct per variant ---

#[test]
fn mode_glyph_is_distinct_per_variant() {
    assert_ne!(
        micold_ai_ide::session::mode_glyph(TerminalMode::AiCli),
        micold_ai_ide::session::mode_glyph(TerminalMode::Regular)
    );
    assert_eq!(
        micold_ai_ide::session::mode_glyph(TerminalMode::AiCli),
        Icon::AiCli
    );
    assert_eq!(
        micold_ai_ide::session::mode_glyph(TerminalMode::Regular),
        Icon::RegularTerminal
    );
}

#[test]
fn mode_tooltip_is_distinct_per_variant() {
    let ai = micold_ai_ide::session::mode_tooltip(TerminalMode::AiCli);
    let regular = micold_ai_ide::session::mode_tooltip(TerminalMode::Regular);
    assert_ne!(ai, regular);
    assert!(!ai.is_empty());
    assert!(!regular.is_empty());
}

// --- T022 (US2): the two lifecycles are independent by construction (FR-006) ---

#[test]
fn shell_transitions_never_mutate_ai_cli_lifecycle() {
    use micold_ai_ide::session::SessionLifecycle;

    let mut s = Session::start_new(worktree("feature-x"));
    s.mark_running();
    let ai_cli_before = s.lifecycle;
    assert_eq!(ai_cli_before, SessionLifecycle::Running);

    s.set_mode(TerminalMode::Regular);
    assert_eq!(
        s.lifecycle, ai_cli_before,
        "set_mode must not touch lifecycle"
    );

    s.start_shell();
    assert_eq!(
        s.lifecycle, ai_cli_before,
        "start_shell must not touch lifecycle"
    );

    s.mark_shell_running();
    assert_eq!(
        s.lifecycle, ai_cli_before,
        "mark_shell_running must not touch lifecycle"
    );

    s.mark_shell_exited();
    assert_eq!(
        s.lifecycle, ai_cli_before,
        "mark_shell_exited must not touch lifecycle"
    );
}
