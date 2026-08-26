//! T003 — `ShellInstanceId` allocation and the `Session` instance mutators (feature 011,
//! FR-001–FR-004, FR-007–FR-013, FR-017; contracts/shell-instance-lifecycle.md).

use micold_core::session::{AiCli, Session, SessionLocation, ShellLifecycle, TerminalMode};

fn worktree(name: &str) -> SessionLocation {
    SessionLocation::Worktree(name.to_string())
}

#[test]
fn session_start_new_has_no_shell_instances() {
    let s = Session::start_new(worktree("feature-x"), AiCli::ClaudeCode);
    assert!(s.shells.is_empty());
    assert_eq!(s.active_shell, None);
    assert_eq!(s.mode, TerminalMode::AiCli);
}

#[test]
fn session_restored_has_no_shell_instances_even_in_regular_mode() {
    use micold_core::session::{SessionId, SessionLabel};
    let s = Session::restored(
        SessionId::new(),
        worktree("feature-x"),
        SessionLabel::Pending,
        TerminalMode::Regular,
        AiCli::ClaudeCode,
    );
    assert!(s.shells.is_empty(), "no process survives a restart");
    assert_eq!(s.active_shell, None);
    assert_eq!(
        s.mode,
        TerminalMode::Regular,
        "the mode itself IS persisted"
    );
}

#[test]
fn open_shell_instance_appends_and_becomes_active() {
    let mut s = Session::start_new(worktree("feature-x"), AiCli::ClaudeCode);
    let first = s.open_shell_instance();
    assert_eq!(s.shells.len(), 1);
    assert_eq!(s.shells[0].id, first);
    assert_eq!(s.shells[0].lifecycle, ShellLifecycle::Starting);
    assert_eq!(s.active_shell, Some(first));

    let second = s.open_shell_instance();
    assert_ne!(first, second, "each opened instance gets a distinct id");
    assert_eq!(s.shells.len(), 2);
    assert_eq!(s.shells[1].id, second, "appended at the end, in open order");
    assert_eq!(
        s.active_shell,
        Some(second),
        "the most recently opened instance becomes active"
    );
}

#[test]
fn shell_instance_ids_are_never_reused_across_open_and_close() {
    let mut s = Session::start_new(worktree("feature-x"), AiCli::ClaudeCode);
    let first = s.open_shell_instance();
    s.close_shell(first);
    assert!(s.shells.is_empty());

    let second = s.open_shell_instance();
    assert_ne!(
        first, second,
        "closing every instance must not reset the id counter"
    );
}

#[test]
fn select_shell_switches_the_active_instance() {
    let mut s = Session::start_new(worktree("feature-x"), AiCli::ClaudeCode);
    let first = s.open_shell_instance();
    let second = s.open_shell_instance();
    assert_eq!(s.active_shell, Some(second));

    s.select_shell(first);
    assert_eq!(s.active_shell, Some(first));
}

#[test]
fn select_shell_is_a_no_op_for_an_unknown_id() {
    let mut s = Session::start_new(worktree("feature-x"), AiCli::ClaudeCode);
    let first = s.open_shell_instance();
    let unknown = s.open_shell_instance();
    s.close_shell(unknown);

    s.select_shell(unknown);
    assert_eq!(
        s.active_shell,
        Some(first),
        "selecting a since-closed id must not change active_shell"
    );
}

#[test]
fn close_shell_of_a_background_instance_leaves_active_shell_untouched() {
    let mut s = Session::start_new(worktree("feature-x"), AiCli::ClaudeCode);
    let first = s.open_shell_instance();
    let second = s.open_shell_instance();
    s.select_shell(first);

    s.close_shell(second);
    assert_eq!(s.shells.len(), 1);
    assert_eq!(
        s.active_shell,
        Some(first),
        "closing a non-active sibling must not reassign active_shell"
    );
}

#[test]
fn close_shell_of_the_active_instance_activates_the_next_in_list() {
    let mut s = Session::start_new(worktree("feature-x"), AiCli::ClaudeCode);
    let first = s.open_shell_instance();
    let second = s.open_shell_instance();
    let third = s.open_shell_instance();
    s.select_shell(first);

    s.close_shell(first);
    assert_eq!(
        s.active_shell,
        Some(second),
        "closing the active instance activates the one now at its former position"
    );
    assert_eq!(
        s.shells.iter().map(|i| i.id).collect::<Vec<_>>(),
        vec![second, third]
    );
}

#[test]
fn close_shell_of_the_active_last_instance_falls_back_to_the_new_last() {
    let mut s = Session::start_new(worktree("feature-x"), AiCli::ClaudeCode);
    let first = s.open_shell_instance();
    let second = s.open_shell_instance();
    let third = s.open_shell_instance();
    assert_eq!(s.active_shell, Some(third));

    s.close_shell(third);
    assert_eq!(
        s.active_shell,
        Some(second),
        "closing the last instance in the list falls back to the new last instance"
    );
    let _ = first;
}

#[test]
fn close_shell_of_the_last_remaining_instance_reverts_mode_to_ai_cli() {
    let mut s = Session::start_new(worktree("feature-x"), AiCli::ClaudeCode);
    s.set_mode(TerminalMode::Regular);
    let only = s.open_shell_instance();

    s.close_shell(only);
    assert!(s.shells.is_empty());
    assert_eq!(s.active_shell, None);
    assert_eq!(
        s.mode,
        TerminalMode::AiCli,
        "closing the last instance falls back to AI CLI mode (FR-013)"
    );
}

#[test]
fn close_shell_is_a_no_op_for_an_unknown_id() {
    let mut s = Session::start_new(worktree("feature-x"), AiCli::ClaudeCode);
    let only = s.open_shell_instance();
    let unknown = s.open_shell_instance();
    s.close_shell(unknown);

    s.close_shell(unknown);
    assert_eq!(s.shells.len(), 1);
    assert_eq!(s.active_shell, Some(only));
}

#[test]
fn restart_shell_instance_delegates_to_that_instances_lifecycle() {
    let mut s = Session::start_new(worktree("feature-x"), AiCli::ClaudeCode);
    let id = s.open_shell_instance();
    s.mark_shell_running(id);
    assert_eq!(s.active_shell_lifecycle(), Some(ShellLifecycle::Running));

    s.mark_shell_exited(id);
    assert_eq!(s.active_shell_lifecycle(), Some(ShellLifecycle::Exited));

    s.restart_shell_instance(id);
    assert_eq!(s.active_shell_lifecycle(), Some(ShellLifecycle::Starting));
}

#[test]
fn restart_mark_running_mark_exited_are_no_ops_for_an_unknown_id() {
    let mut s = Session::start_new(worktree("feature-x"), AiCli::ClaudeCode);
    let id = s.open_shell_instance();
    let unknown = s.open_shell_instance();
    s.close_shell(unknown);

    // None of these must panic, and none must affect the surviving instance.
    s.restart_shell_instance(unknown);
    s.mark_shell_running(unknown);
    s.mark_shell_exited(unknown);
    assert_eq!(s.shells.len(), 1);
    assert_eq!(s.shells[0].id, id);
}

#[test]
fn active_shell_lifecycle_is_none_when_there_are_no_instances() {
    let s = Session::start_new(worktree("feature-x"), AiCli::ClaudeCode);
    assert_eq!(s.active_shell_lifecycle(), None);
}

#[test]
fn shell_instance_mutators_never_touch_ai_cli_lifecycle() {
    use micold_core::session::SessionLifecycle;

    let mut s = Session::start_new(worktree("feature-x"), AiCli::ClaudeCode);
    s.mark_running();
    let ai_cli_before = s.lifecycle.clone();
    assert_eq!(ai_cli_before, SessionLifecycle::Running);

    let id = s.open_shell_instance();
    assert_eq!(
        s.lifecycle, ai_cli_before,
        "open_shell_instance must not touch lifecycle"
    );
    s.mark_shell_running(id);
    assert_eq!(
        s.lifecycle, ai_cli_before,
        "mark_shell_running must not touch lifecycle"
    );
    s.mark_shell_exited(id);
    assert_eq!(
        s.lifecycle, ai_cli_before,
        "mark_shell_exited must not touch lifecycle"
    );
    s.close_shell(id);
    assert_eq!(
        s.lifecycle, ai_cli_before,
        "close_shell must not touch lifecycle"
    );
}
