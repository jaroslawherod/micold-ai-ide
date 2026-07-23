//! T011 — extended app base state: defaults + new message wiring (feature 005).

use micold_ai_ide::app::{on_escape, Message, Overlay, State, WorktreeFormStatus};
use micold_ai_ide::naming::ConventionalType;
use micold_ai_ide::project::{Availability, Project};
use micold_ai_ide::session::{Session, SessionLifecycle, SessionLocation};
use micold_ai_ide::worktree::{CreateProgressEvent, CreateStage, Worktree, WorktreeStatus};
use std::path::PathBuf;

/// A stage-tagged progress event, for constructing `WorktreeCreateLogAppended` batches.
fn event(stage: CreateStage, line: &str) -> CreateProgressEvent {
    CreateProgressEvent {
        stage,
        line: line.to_string(),
    }
}

#[test]
fn defaults_are_empty() {
    let state = State::default();
    assert!(state.worktrees.is_empty());
    assert!(state.expanded.is_empty());
    assert!(state.active_session.is_none());
    assert!(state.worktree_form.is_none());
    assert!(state.worktree_error.is_none());
    assert_eq!(state.overlay, Overlay::None);
    assert!(state.active_sessions().is_empty());
}

#[test]
fn opening_the_form_sets_overlay_and_draft() {
    let mut state = State::default();
    state.update(Message::AddWorktreeOpened);
    assert_eq!(state.overlay, Overlay::AddWorktree);
    assert!(state.worktree_form.is_some());
}

#[test]
fn form_edits_build_a_derived_preview() {
    let mut state = State::default();
    state.update(Message::AddWorktreeOpened);
    state.update(Message::AddWorktreeTypeSelected(ConventionalType::Feat));
    state.update(Message::AddWorktreeTicketChanged("ABC-1".to_string()));
    state.update(Message::AddWorktreeNameChanged("Login".to_string()));

    let form = state.worktree_form.as_ref().unwrap();
    let derived = form.preview().unwrap();
    assert_eq!(derived.dir_name, "feat-abc-1-login");
    assert_eq!(derived.branch, "feat/abc-1-login");
}

#[test]
fn submitting_an_invalid_form_records_the_error() {
    let mut state = State::default();
    state.update(Message::AddWorktreeOpened);
    // No type selected, no name.
    state.update(Message::AddWorktreeSubmitted);
    assert!(state.worktree_form.as_ref().unwrap().error.is_some());
}

#[test]
fn cancelling_the_form_clears_it() {
    let mut state = State::default();
    state.update(Message::AddWorktreeOpened);
    state.update(Message::AddWorktreeCancelled);
    assert_eq!(state.overlay, Overlay::None);
    assert!(state.worktree_form.is_none());
}

#[test]
fn created_worktree_is_added_and_form_closed() {
    let mut state = State::default();
    state.update(Message::AddWorktreeOpened);
    let wt = Worktree {
        dir_name: "feat-x".to_string(),
        path: PathBuf::from("/repo/.claude/worktrees/feat-x"),
        branch: Some("feat/x".to_string()),
        status: WorktreeStatus::Valid,
    };
    state.update(Message::WorktreeCreated(wt));
    assert_eq!(state.overlay, Overlay::None);
    assert!(state.worktree_form.is_none());
    assert_eq!(state.worktrees.len(), 1);
}

#[test]
fn create_started_marks_form_creating() {
    let mut state = State::default();
    state.update(Message::AddWorktreeOpened);
    state.update(Message::WorktreeCreateStarted);
    assert_eq!(
        state.worktree_form.as_ref().unwrap().status,
        WorktreeFormStatus::Creating
    );
}

#[test]
fn field_edits_are_ignored_while_creating() {
    let mut state = State::default();
    state.update(Message::AddWorktreeOpened);
    state.update(Message::AddWorktreeTypeSelected(ConventionalType::Feat));
    state.update(Message::AddWorktreeNameChanged("Login".to_string()));
    state.update(Message::WorktreeCreateStarted);

    // The whole form is inactive while a create is in flight (feature 010 follow-up), not
    // just the submit button — edits during this window must be no-ops.
    state.update(Message::AddWorktreeTypeSelected(ConventionalType::Fix));
    state.update(Message::AddWorktreeTicketChanged("ABC-1".to_string()));
    state.update(Message::AddWorktreeNameChanged(
        "Something else".to_string(),
    ));

    let form = state.worktree_form.as_ref().unwrap();
    assert_eq!(form.type_, Some(ConventionalType::Feat));
    assert_eq!(form.ticket, "");
    assert_eq!(form.name, "Login");
}

#[test]
fn create_log_lines_accumulate_and_reset_on_new_attempt() {
    let mut state = State::default();
    state.update(Message::AddWorktreeOpened);
    state.update(Message::WorktreeCreateStarted);
    state.update(Message::WorktreeCreateLogAppended(vec![event(
        CreateStage::CreatingWorktree,
        "$ git worktree add -b feat/x .claude/worktrees/feat-x HEAD",
    )]));
    state.update(Message::WorktreeCreateLogAppended(vec![
        event(
            CreateStage::SettingUpSubmodules,
            "$ git submodule update --init --recursive",
        ),
        event(CreateStage::SettingUpSubmodules, "Cloning into 'vendor/sub'..."),
    ]));
    assert_eq!(state.worktree_form.as_ref().unwrap().log.len(), 3);

    // A fresh attempt clears the previous attempt's log (feature 010 follow-up).
    state.update(Message::WorktreeCreateStarted);
    assert!(state.worktree_form.as_ref().unwrap().log.is_empty());
}

#[test]
fn create_failed_keeps_the_log_visible_for_diagnosis() {
    let mut state = State::default();
    state.update(Message::AddWorktreeOpened);
    state.update(Message::WorktreeCreateStarted);
    state.update(Message::WorktreeCreateLogAppended(vec![event(
        CreateStage::SettingUpSubmodules,
        "submodule update failed: network error",
    )]));
    state.update(Message::WorktreeCreateFailed("boom".to_string()));

    // Unlike on success (form closes entirely), a failure keeps the form open with its log
    // intact so the user can see what happened before retrying.
    assert_eq!(
        state.worktree_form.as_ref().unwrap().log,
        vec!["submodule update failed: network error".to_string()]
    );
}

// --- Feature 013 US3: stage-tagged creation progress ---

#[test]
fn log_appended_sets_stage_to_the_batch_last_events_stage() {
    let mut state = State::default();
    state.update(Message::AddWorktreeOpened);
    state.update(Message::WorktreeCreateStarted);
    assert_eq!(state.worktree_form.as_ref().unwrap().stage, None);

    state.update(Message::WorktreeCreateLogAppended(vec![event(
        CreateStage::PreflightCheck,
        "Checking for naming conflicts…",
    )]));
    assert_eq!(
        state.worktree_form.as_ref().unwrap().stage,
        Some(CreateStage::PreflightCheck)
    );

    state.update(Message::WorktreeCreateLogAppended(vec![
        event(CreateStage::CreatingWorktree, "$ git worktree add …"),
        event(CreateStage::SettingUpSubmodules, "$ git submodule update …"),
    ]));
    assert_eq!(
        state.worktree_form.as_ref().unwrap().stage,
        Some(CreateStage::SettingUpSubmodules),
        "the last event in the batch wins"
    );
}

#[test]
fn stage_resets_on_started_opened_and_cancelled() {
    let mut state = State::default();
    state.update(Message::AddWorktreeOpened);
    state.update(Message::WorktreeCreateStarted);
    state.update(Message::WorktreeCreateLogAppended(vec![event(
        CreateStage::CreatingWorktree,
        "$ git worktree add …",
    )]));
    assert!(state.worktree_form.as_ref().unwrap().stage.is_some());

    // A fresh attempt clears the previous attempt's stage, same reset point as `log`.
    state.update(Message::WorktreeCreateStarted);
    assert_eq!(state.worktree_form.as_ref().unwrap().stage, None);

    state.update(Message::WorktreeCreateLogAppended(vec![event(
        CreateStage::CreatingWorktree,
        "$ git worktree add …",
    )]));
    state.update(Message::AddWorktreeCancelled);
    state.update(Message::AddWorktreeOpened);
    assert_eq!(state.worktree_form.as_ref().unwrap().stage, None);
}

#[test]
fn create_failed_keeps_form_open_and_resets_status_to_editing() {
    let mut state = State::default();
    state.update(Message::AddWorktreeOpened);
    state.update(Message::WorktreeCreateStarted);
    state.update(Message::WorktreeCreateFailed("boom".to_string()));
    assert_eq!(state.worktree_error.as_deref(), Some("boom"));
    assert!(state.worktree_form.is_some(), "form stays open for retry");
    assert_eq!(
        state.worktree_form.as_ref().unwrap().status,
        WorktreeFormStatus::Editing
    );
}

#[test]
fn resubmitting_while_creating_is_a_no_op() {
    let mut state = State::default();
    state.update(Message::AddWorktreeOpened);
    state.update(Message::AddWorktreeTypeSelected(ConventionalType::Feat));
    state.update(Message::AddWorktreeNameChanged("Login".to_string()));
    state.update(Message::WorktreeCreateStarted);
    // Corrupt the form to prove the guard skips validation entirely while Creating —
    // an unguarded AddWorktreeSubmitted would call preview() and record an error here.
    state.update(Message::AddWorktreeNameChanged(String::new()));
    state.update(Message::AddWorktreeSubmitted);
    assert!(state.worktree_form.as_ref().unwrap().error.is_none());
}

// --- Feature 013 US1: type field is a Material select (wraps iced's `pick_list`) ---

#[test]
fn selecting_a_type_sets_the_form_value() {
    let mut state = State::default();
    state.update(Message::AddWorktreeOpened);
    assert_eq!(state.worktree_form.as_ref().unwrap().type_, None);

    state.update(Message::AddWorktreeTypeSelected(ConventionalType::Feat));
    assert_eq!(
        state.worktree_form.as_ref().unwrap().type_,
        Some(ConventionalType::Feat)
    );
}

#[test]
fn type_selection_is_ignored_while_creating() {
    let mut state = State::default();
    state.update(Message::AddWorktreeOpened);
    state.update(Message::AddWorktreeTypeSelected(ConventionalType::Feat));
    state.update(Message::AddWorktreeNameChanged("Login".to_string()));
    state.update(Message::WorktreeCreateStarted);

    state.update(Message::AddWorktreeTypeSelected(ConventionalType::Fix));
    assert_eq!(
        state.worktree_form.as_ref().unwrap().type_,
        Some(ConventionalType::Feat)
    );
}

#[test]
fn session_started_selected_and_closed() {
    let mut state = State::default();
    let path = PathBuf::from("/repo");
    state.workspace.projects.push(Project {
        path: path.clone(),
        display_name: "repo".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    state.workspace.active = Some(path);

    let session = Session::start_new(SessionLocation::Worktree("feat-x".to_string()));
    let id = session.id;
    state.update(Message::SessionStarted(session));
    assert_eq!(state.active_session, Some(id));
    assert_eq!(state.active_sessions().len(), 1);
    assert!(state.expanded.contains("feat-x"));

    state.update(Message::SessionRunning(id));
    assert!(state.active_sessions()[0].is_active());

    state.update(Message::SessionTitleUpdated {
        id,
        title: "Titled".to_string(),
    });
    assert_eq!(state.active_sessions()[0].label.display(), "Titled");

    state.update(Message::SessionCloseRequested(id));
    assert!(state.active_session.is_none());
    // Bugfix BUG-003 (FR-015a): close archives the record (kept, not deleted) so a still-existing
    // `claude` transcript isn't reconstructed by reconciliation later — it just stops appearing
    // in the sidebar (`sidebar_entries`/`worktree_tree`, not `active_sessions()` itself).
    let closed = state
        .active_sessions()
        .iter()
        .find(|s| s.id == id)
        .expect("closing archives the record rather than deleting it");
    assert!(closed.archived);
    assert_eq!(closed.lifecycle, SessionLifecycle::Idle);
}

// T015 (010-root-dir-session): a Default-located session enters `Workspace.sessions` exactly
// like a worktree session. Note: `Message::SessionStartRequested` itself has no pure-reducer
// effect for ANY location (it's an I/O trigger the binary consumes to spawn a PTY before
// dispatching `SessionStarted` — see `src/app.rs`'s `on_escape`-adjacent no-op arm list); the
// pure-core assertion point is `SessionStarted`, exercised identically to the existing
// `session_started_selected_and_closed` test above.
#[test]
fn default_session_started_enters_workspace_sessions() {
    let mut state = State::default();
    let path = PathBuf::from("/repo");
    state.workspace.projects.push(Project {
        path: path.clone(),
        display_name: "repo".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    state.workspace.active = Some(path);

    let session = Session::start_new(SessionLocation::Default);
    let id = session.id;
    state.update(Message::SessionStarted(session));

    assert_eq!(state.active_session, Some(id));
    assert_eq!(state.active_sessions().len(), 1);
    assert_eq!(
        state.active_sessions()[0].location,
        SessionLocation::Default
    );
    // The Default row's own expansion flag opens, not the worktree `expanded` set.
    assert!(state.default_expanded);
}

// --- Feature 008 US2: worktree delete reducer ---

fn state_with_worktree_and_session(dir: &str) -> State {
    let mut state = State::default();
    let path = PathBuf::from("/repo");
    state.workspace.projects.push(Project {
        path: path.clone(),
        display_name: "repo".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    state.workspace.active = Some(path);
    state.worktrees.push(Worktree {
        dir_name: dir.to_string(),
        path: PathBuf::from(format!("/repo/.claude/worktrees/{dir}")),
        branch: Some(format!("feat/{dir}")),
        status: WorktreeStatus::Valid,
    });
    let session = Session::start_new(SessionLocation::Worktree(dir.to_string()));
    state.update(Message::SessionStarted(session));
    state
}

#[test]
fn delete_requested_opens_confirm_then_confirmed_drops_records() {
    let mut state = state_with_worktree_and_session("feat-x");
    assert_eq!(state.active_sessions().len(), 1);

    state.update(Message::WorktreeDeleteRequested("feat-x".to_string()));
    assert_eq!(state.overlay, Overlay::ConfirmWorktreeDelete);
    assert_eq!(state.worktree_delete_target.as_deref(), Some("feat-x"));
    assert!(state.worktree_menu_open.is_none());

    state.update(Message::WorktreeDeleteConfirmed);
    assert!(state.active_sessions().is_empty(), "sessions dropped");
    assert!(state.active_session.is_none(), "active cleared");
    assert!(!state.worktrees.iter().any(|w| w.dir_name == "feat-x"));
    assert_eq!(state.overlay, Overlay::None);
    assert!(state.worktree_delete_target.is_none());
}

#[test]
fn delete_cancelled_changes_nothing() {
    let mut state = state_with_worktree_and_session("feat-x");
    state.update(Message::WorktreeDeleteRequested("feat-x".to_string()));
    state.update(Message::WorktreeDeleteCancelled);
    assert_eq!(state.overlay, Overlay::None);
    assert!(state.worktree_delete_target.is_none());
    assert_eq!(state.active_sessions().len(), 1, "session untouched");
    assert!(state.worktrees.iter().any(|w| w.dir_name == "feat-x"));
}

#[test]
fn escape_cancels_confirm_delete() {
    let mut state = state_with_worktree_and_session("feat-x");
    state.update(Message::WorktreeDeleteRequested("feat-x".to_string()));
    assert_eq!(on_escape(&state), Some(Message::WorktreeDeleteCancelled));
}

// --- Feature 013 US2: delete confirmation's branch-deletion choice ---

#[test]
fn delete_requested_resets_keep_branch_even_if_previously_set() {
    let mut state = state_with_worktree_and_session("feat-x");
    state.update(Message::WorktreeDeleteRequested("feat-x".to_string()));
    state.update(Message::WorktreeDeleteKeepBranchToggled(true));
    assert!(state.worktree_delete_keep_branch);

    // Cancel and request again on a different worktree — the choice must not carry over.
    state.update(Message::WorktreeDeleteCancelled);
    state.update(Message::WorktreeDeleteRequested("feat-x".to_string()));
    assert!(!state.worktree_delete_keep_branch);
}

#[test]
fn delete_keep_branch_toggled_sets_the_field() {
    let mut state = state_with_worktree_and_session("feat-x");
    state.update(Message::WorktreeDeleteRequested("feat-x".to_string()));
    assert!(!state.worktree_delete_keep_branch, "defaults to delete");

    state.update(Message::WorktreeDeleteKeepBranchToggled(true));
    assert!(state.worktree_delete_keep_branch);

    state.update(Message::WorktreeDeleteKeepBranchToggled(false));
    assert!(!state.worktree_delete_keep_branch);
}

// --- Feature 008 US3: worktree rename changes display only ---

#[test]
fn worktree_rename_changes_display_only_not_branch_or_path() {
    let mut state = state_with_worktree_and_session("feat-x");
    let before = state
        .worktrees
        .iter()
        .find(|w| w.dir_name == "feat-x")
        .unwrap()
        .clone();
    let tags_before = state.worktree_tree()[0].tags.clone();

    state.update(Message::WorktreeRenameStarted("feat-x".to_string()));
    state.update(Message::WorktreeRenameTextChanged("Renamed".to_string()));
    state.update(Message::WorktreeRenameConfirmed);

    assert_eq!(state.worktree_display_name("feat-x"), "Renamed");
    let after = state
        .worktrees
        .iter()
        .find(|w| w.dir_name == "feat-x")
        .unwrap();
    // FR-007/FR-014: the on-disk identity is untouched.
    assert_eq!(after.dir_name, "feat-x");
    assert_eq!(after.path, before.path);
    assert_eq!(after.branch, before.branch);
    // FR-016: tags still derive from the branch/dir, unaffected by the rename.
    assert_eq!(state.worktree_tree()[0].tags, tags_before);
}

#[test]
fn escape_cancels_worktree_rename() {
    let mut state = state_with_worktree_and_session("feat-x");
    state.update(Message::WorktreeRenameStarted("feat-x".to_string()));
    assert_eq!(on_escape(&state), Some(Message::WorktreeRenameCancelled));
}

// --- Feature 010: switchable regular terminal mode ---

#[test]
fn terminal_mode_toggled_flips_the_active_sessions_mode() {
    use micold_ai_ide::session::TerminalMode;

    let mut state = state_with_worktree_and_session("feat-x");
    assert_eq!(state.active_sessions()[0].mode, TerminalMode::AiCli);

    state.update(Message::TerminalModeToggled);
    assert_eq!(state.active_sessions()[0].mode, TerminalMode::Regular);

    state.update(Message::TerminalModeToggled);
    assert_eq!(state.active_sessions()[0].mode, TerminalMode::AiCli);
}

#[test]
fn terminal_mode_toggled_is_a_no_op_with_no_active_session() {
    let mut state = State::default();
    // No panic, no-op: there is no active session to address.
    state.update(Message::TerminalModeToggled);
    assert!(state.active_session.is_none());
}

#[test]
fn shell_instance_running_and_exited_update_that_instances_lifecycle() {
    use micold_ai_ide::session::ShellLifecycle;

    let mut state = state_with_worktree_and_session("feat-x");
    let id = state.active_session.unwrap();
    let shell_id = state
        .workspace
        .find_session_mut(id)
        .unwrap()
        .1
        .open_shell_instance();

    state.update(Message::ShellInstanceRunning(id, shell_id));
    assert_eq!(
        state.active_sessions()[0].active_shell_lifecycle(),
        Some(ShellLifecycle::Running)
    );

    state.update(Message::ShellInstanceExited(id, shell_id));
    assert_eq!(
        state.active_sessions()[0].active_shell_lifecycle(),
        Some(ShellLifecycle::Exited)
    );
}

// Feature 011 (environment-include) — contracts/settings-ui.md.

#[test]
fn settings_env_include_enabled_toggled_updates_only_that_field() {
    let mut state = State::default();
    state.update(Message::SettingsOpened);
    state.update(Message::SettingsEnvIncludeEnabledToggled(false));

    let draft = state.settings_draft.as_ref().unwrap();
    assert!(!draft.env_include_enabled);
}

#[test]
fn settings_env_include_path_changed_updates_only_that_field() {
    let mut state = State::default();
    state.update(Message::SettingsOpened);
    state.update(Message::SettingsEnvIncludePathChanged(
        "/custom/script.sh".to_string(),
    ));

    let draft = state.settings_draft.as_ref().unwrap();
    assert_eq!(draft.env_include_script_path, "/custom/script.sh");
}

#[test]
fn settings_env_include_timeout_changed_updates_only_that_field() {
    let mut state = State::default();
    state.update(Message::SettingsOpened);
    state.update(Message::SettingsEnvIncludeTimeoutChanged("30".to_string()));

    let draft = state.settings_draft.as_ref().unwrap();
    assert_eq!(draft.env_include_timeout, "30");
}

#[test]
fn env_include_field_changes_leave_other_draft_fields_untouched() {
    let mut state = State::default();
    state.update(Message::SettingsOpened);
    state.update(Message::SettingsScrollbackChanged("25000".to_string()));
    state.update(Message::SettingsEnvIncludeEnabledToggled(false));
    state.update(Message::SettingsEnvIncludePathChanged(
        "/custom/script.sh".to_string(),
    ));
    state.update(Message::SettingsEnvIncludeTimeoutChanged("30".to_string()));

    let draft = state.settings_draft.as_ref().unwrap();
    assert_eq!(draft.scrollback_lines, "25000");
    assert!(!draft.env_include_enabled);
    assert_eq!(draft.env_include_script_path, "/custom/script.sh");
    assert_eq!(draft.env_include_timeout, "30");
}
