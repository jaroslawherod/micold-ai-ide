//! T011 — extended app base state: defaults + new message wiring (feature 005).

use micold_client::app::{on_escape, Message, Overlay, State, TagFilter, WorktreeFormStatus};
use micold_core::naming::ConventionalType;
use micold_core::project::{Availability, Project};
use micold_core::session::{Session, SessionLifecycle, SessionLocation};
use micold_core::worktree::{Worktree, WorktreeStatus};
use std::path::PathBuf;

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

// --- Feature 014 US3: everything derived from the list stays consistent ---

fn agent_worktree(hex: &str) -> Worktree {
    let dir = format!("agent-{hex}");
    Worktree {
        dir_name: dir.clone(),
        path: PathBuf::from(format!("/repo/.claude/worktrees/{dir}")),
        branch: Some(format!("worktree-agent-{hex}")),
        status: WorktreeStatus::Valid,
    }
}

#[test]
fn hidden_worktrees_offer_no_tag_filters() {
    // FR-003 / research R7: an agent worktree's machine name carries no conventional type, so
    // leaving it in `available_tag_filters()` would conjure an `Untyped` chip that matches nothing
    // the user can see — the most confusing possible way for this to fail.
    let mut state = state_with_worktree_and_session("feat-x");
    state.worktrees.push(agent_worktree("a885b42dc521fbda1"));

    let filters = state.available_tag_filters();
    assert!(
        !filters.contains(&TagFilter::Untyped),
        "a hidden worktree must not contribute a filter chip, got {filters:?}"
    );
    assert!(filters.contains(&TagFilter::Type(ConventionalType::Feat)));
}

#[test]
fn empty_state_distinguishes_no_worktrees_from_none_visible() {
    // FR-003 / US1 acceptance #2: a project whose only worktrees are agent-owned must read as
    // "no worktrees yet", not "nothing matched the filter" — there is no filter to clear.
    let agent_only = State {
        worktrees: vec![agent_worktree("a885b42dc521fbda1")],
        ..Default::default()
    };
    assert!(!agent_only.has_visible_worktrees());

    let with_user = state_with_worktree_and_session("feat-x");
    assert!(with_user.has_visible_worktrees());

    assert!(!State::default().has_visible_worktrees());
}

#[test]
fn rename_override_for_a_hidden_worktree_survives_reload() {
    // The pruning in `set_worktrees` reasons about EXISTENCE, not visibility: a hidden worktree
    // still exists, so dropping its rename override would silently lose user data the moment the
    // reveal control is switched on (contracts/agent-worktree-classification.md § non-consumers).
    let mut state = state_with_worktree_and_session("feat-x");
    let agent = agent_worktree("a885b42dc521fbda1");
    let agent_dir = agent.dir_name.clone();
    state.worktrees.push(agent.clone());

    state.update(Message::WorktreeRenameStarted(agent_dir.clone()));
    state.update(Message::WorktreeRenameTextChanged("Scratch".to_string()));
    state.update(Message::WorktreeRenameConfirmed);
    assert_eq!(state.worktree_display_name(&agent_dir), "Scratch");

    // Re-discovery still reports both worktrees.
    let all = state.worktrees.clone();
    state.update(Message::WorktreesLoaded(all));
    assert_eq!(
        state.worktree_display_name(&agent_dir),
        "Scratch",
        "a hidden worktree still exists, so its rename override must not be pruned"
    );
}

// --- Feature 014 US4: the reveal control ---

#[test]
fn reveal_control_is_off_by_default() {
    // FR-010a: the safe default, with no persisted field to migrate.
    assert!(!State::default().show_agent_worktrees);
}

#[test]
fn toggling_reveal_changes_only_that_field() {
    // FR-010d: the reveal control and the tag filters are independent. Clobbering the filters
    // would silently discard the user's filtering work every time they peeked at agent worktrees.
    let mut state = state_with_worktree_and_session("feat-x");
    state.update(Message::SidebarFilterToggled(TagFilter::Type(
        ConventionalType::Feat,
    )));
    state.update(Message::WorktreeExpansionToggled("feat-x".to_string()));
    let filters_before = state.sidebar_filters.clone();
    let expanded_before = state.expanded.clone();
    let overlay_before = state.overlay;

    state.update(Message::ShowAgentWorktreesToggled);

    assert!(state.show_agent_worktrees);
    assert_eq!(state.sidebar_filters, filters_before);
    assert_eq!(state.expanded, expanded_before);
    assert_eq!(state.overlay, overlay_before);
}

#[test]
fn two_toggles_restore_the_prior_list() {
    let mut state = state_with_worktree_and_session("feat-x");
    state.worktrees.push(agent_worktree("a885b42dc521fbda1"));
    let before: Vec<String> = state
        .worktree_tree()
        .iter()
        .map(|n| n.worktree.dir_name.clone())
        .collect();

    state.update(Message::ShowAgentWorktreesToggled);
    assert_eq!(state.worktree_tree().len(), 2, "revealed");
    state.update(Message::ShowAgentWorktreesToggled);

    let after: Vec<String> = state
        .worktree_tree()
        .iter()
        .map(|n| n.worktree.dir_name.clone())
        .collect();
    assert_eq!(after, before);
}

#[test]
fn switching_projects_resets_the_reveal_control() {
    // FR-010e: view state switched on for one project must not silently render in another. The
    // filter accordion is collapsed by default, so a sticky toggle would show unexplained extra
    // rows with its cause hidden behind a closed panel.
    let mut state = state_with_worktree_and_session("feat-x");
    let other = PathBuf::from("/other-repo");
    state.workspace.projects.push(Project {
        path: other.clone(),
        display_name: "other-repo".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });

    state.update(Message::ShowAgentWorktreesToggled);
    assert!(state.show_agent_worktrees);

    assert!(state.switch_active(&other));
    assert!(
        !state.show_agent_worktrees,
        "the incoming project must be entered with agent worktrees hidden"
    );

    // Switching back does not restore it either — nothing is remembered per project.
    let first = PathBuf::from("/repo");
    assert!(state.switch_active(&first));
    assert!(!state.show_agent_worktrees);
}

// --- Feature 010: switchable regular terminal mode ---

#[test]
fn terminal_mode_toggled_flips_the_active_sessions_mode() {
    use micold_core::session::TerminalMode;

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
    use micold_core::session::ShellLifecycle;

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

// =======================================================================================
// Feature 016 — the existing-branch source and the conflict-resolution state machine
// (contract `branch-conflict.md` §3, `branch-picker.md` §5).
// =======================================================================================

use micold_client::app::{BranchSource, ResolutionState, WorktreeForm};
use micold_core::worktree::{
    BlockReason, BranchCandidate, BranchOrigin, BranchSituation, CreateMode,
};

fn local_conflict() -> BranchSituation {
    BranchSituation::LocalAvailable {
        branch: "feat/login".to_string(),
    }
}

fn remote_conflict() -> BranchSituation {
    BranchSituation::RemoteOnly {
        branch: "feat/login".to_string(),
        remotes: vec!["origin".to_string()],
    }
}

/// The same branch name on two remotes — the ambiguous case the app must not resolve itself.
fn multi_remote_conflict() -> BranchSituation {
    BranchSituation::RemoteOnly {
        branch: "feat/login".to_string(),
        remotes: vec!["origin".to_string(), "upstream".to_string()],
    }
}

/// A form with valid new-branch inputs, ready to submit.
fn form_state() -> State {
    let mut state = State::default();
    state.update(Message::AddWorktreeOpened);
    state.update(Message::AddWorktreeTypeSelected(ConventionalType::Feat));
    state.update(Message::AddWorktreeNameChanged("login".to_string()));
    state
}

fn form(state: &State) -> &WorktreeForm {
    state.worktree_form.as_ref().unwrap()
}

// --- the state machine ----------------------------------------------------------------

#[test]
fn a_detected_conflict_opens_the_choice_prompt() {
    let mut state = form_state();
    assert_eq!(form(&state).resolution, ResolutionState::Idle);

    state.update(Message::AddWorktreeConflictDetected(local_conflict()));

    assert_eq!(
        form(&state).resolution,
        ResolutionState::Choosing {
            situation: local_conflict()
        }
    );
    assert!(form(&state).resolution.is_prompting());
}

#[test]
fn cancelling_the_choice_restores_idle_and_preserves_every_input() {
    let mut state = form_state();
    state.update(Message::AddWorktreeTicketChanged("ABC-123".to_string()));
    let before = form(&state).clone();

    state.update(Message::AddWorktreeConflictDetected(local_conflict()));
    state.update(Message::AddWorktreeResolutionCancelled);

    let after = form(&state);
    assert_eq!(after.resolution, ResolutionState::Idle);
    // FR-007: the user lands back on the form exactly as they left it.
    assert_eq!(after.type_, before.type_);
    assert_eq!(after.ticket, before.ticket);
    assert_eq!(after.name, before.name);
    assert_eq!(after.source, before.source);
    assert_eq!(after.selected_branch, before.selected_branch);
    // And the form is still open — cancelling the prompt is not cancelling the form.
    assert_eq!(state.overlay, Overlay::AddWorktree);
}

#[test]
fn overwrite_requires_passing_through_the_confirmation() {
    let mut state = form_state();
    state.update(Message::AddWorktreeConflictDetected(local_conflict()));

    // Invariant 1: the destructive mode cannot be chosen straight from the prompt.
    state.update(Message::AddWorktreeResolutionChosen(CreateMode::Overwrite));
    assert_eq!(
        form(&state).resolution,
        ResolutionState::Choosing {
            situation: local_conflict()
        },
        "Overwrite must not resolve directly from the choice"
    );

    state.update(Message::AddWorktreeOverwriteRequested);
    assert_eq!(
        form(&state).resolution,
        ResolutionState::ConfirmingOverwrite {
            situation: local_conflict()
        }
    );

    state.update(Message::AddWorktreeOverwriteConfirmed);
    assert_eq!(form(&state).resolution, ResolutionState::Idle);
}

#[test]
fn backing_out_of_the_confirmation_returns_to_the_choice_not_the_form() {
    // Invariant 3 (US2 AS3): reuse and cancel must still be available afterwards.
    let mut state = form_state();
    state.update(Message::AddWorktreeConflictDetected(local_conflict()));
    state.update(Message::AddWorktreeOverwriteRequested);

    state.update(Message::AddWorktreeResolutionCancelled);

    assert_eq!(
        form(&state).resolution,
        ResolutionState::Choosing {
            situation: local_conflict()
        }
    );
}

#[test]
fn overwrite_cannot_be_requested_for_a_remote_only_branch() {
    // There is no local branch to destroy, so the confirmation must never open.
    let mut state = form_state();
    state.update(Message::AddWorktreeConflictDetected(remote_conflict()));
    state.update(Message::AddWorktreeOverwriteRequested);

    assert_eq!(
        form(&state).resolution,
        ResolutionState::Choosing {
            situation: remote_conflict()
        }
    );
}

#[test]
fn choosing_reuse_or_track_resolves_the_prompt() {
    let mut state = form_state();
    state.update(Message::AddWorktreeConflictDetected(local_conflict()));
    state.update(Message::AddWorktreeResolutionChosen(CreateMode::ReuseLocal));
    assert_eq!(form(&state).resolution, ResolutionState::Idle);

    let mut state = form_state();
    state.update(Message::AddWorktreeConflictDetected(remote_conflict()));
    state.update(Message::AddWorktreeResolutionChosen(
        CreateMode::TrackRemote {
            remote: "origin".to_string(),
        },
    ));
    assert_eq!(form(&state).resolution, ResolutionState::Idle);
}

/// FR-018 — "start fresh" over a remote-only name is an ordinary new branch.
#[test]
fn starting_fresh_over_a_remote_branch_resolves_to_a_new_branch() {
    let mut state = form_state();
    state.update(Message::AddWorktreeConflictDetected(remote_conflict()));
    state.update(Message::AddWorktreeResolutionChosen(CreateMode::NewBranch));
    assert_eq!(form(&state).resolution, ResolutionState::Idle);
}

/// Invariant 4: a prompt and an in-flight create cannot coexist.
#[test]
fn a_conflict_is_never_raised_while_a_create_is_in_flight() {
    let mut state = form_state();
    state.update(Message::WorktreeCreateStarted);
    assert_eq!(form(&state).status, WorktreeFormStatus::Creating);

    state.update(Message::AddWorktreeConflictDetected(local_conflict()));

    assert_eq!(form(&state).resolution, ResolutionState::Idle);
}

#[test]
fn edits_are_ignored_while_a_prompt_is_open() {
    let mut state = form_state();
    state.update(Message::AddWorktreeConflictDetected(local_conflict()));

    state.update(Message::AddWorktreeSourceChanged(BranchSource::Existing));
    state.update(Message::AddWorktreeBranchSelected(candidate("feat/other")));

    assert_eq!(form(&state).source, BranchSource::New);
    assert_eq!(form(&state).selected_branch, None);
}

// --- US5: blocked situations offer no resolution --------------------------------------

#[test]
fn a_blocked_situation_offers_no_actionable_mode_and_dismisses_to_idle() {
    for situation in [
        BranchSituation::Blocked {
            branch: "main".to_string(),
            reason: BlockReason::CheckedOutInProjectRoot,
        },
        BranchSituation::DirectoryTaken {
            dir: PathBuf::from("/repo/.claude/worktrees/feat-login"),
        },
    ] {
        // No mode exists for these — reuse/overwrite are unrepresentable, not merely hidden.
        assert_eq!(WorktreeForm::mode_for(&situation, None), None);

        let mut state = form_state();
        let before = form(&state).clone();
        state.update(Message::AddWorktreeConflictDetected(situation));
        state.update(Message::AddWorktreeResolutionCancelled);

        assert_eq!(form(&state).resolution, ResolutionState::Idle);
        assert_eq!(form(&state).name, before.name);
    }
}

#[test]
fn mode_for_maps_actionable_situations_and_never_yields_overwrite() {
    assert_eq!(
        WorktreeForm::mode_for(&BranchSituation::Free, None),
        Some(CreateMode::NewBranch)
    );
    assert_eq!(
        WorktreeForm::mode_for(&local_conflict(), None),
        Some(CreateMode::ReuseLocal)
    );
    assert_eq!(
        WorktreeForm::mode_for(&remote_conflict(), None),
        Some(CreateMode::TrackRemote {
            remote: "origin".to_string()
        })
    );
    // Picking a branch is never consent to destroy it (contract branch-picker.md §5).
    for situation in [
        BranchSituation::Free,
        local_conflict(),
        remote_conflict(),
        BranchSituation::Blocked {
            branch: "x".to_string(),
            reason: BlockReason::CheckedOutInProjectRoot,
        },
    ] {
        assert_ne!(
            WorktreeForm::mode_for(&situation, None),
            Some(CreateMode::Overwrite)
        );
    }
}

// --- US2: the existing-branch source --------------------------------------------------

fn candidate(name: &str) -> BranchCandidate {
    BranchCandidate {
        name: name.to_string(),
        origin: BranchOrigin::Local,
        blocked_by: None,
    }
}

fn blocked_candidate(name: &str) -> BranchCandidate {
    BranchCandidate {
        name: name.to_string(),
        origin: BranchOrigin::Local,
        blocked_by: Some(BlockReason::CheckedOutInProjectRoot),
    }
}

#[test]
fn switching_to_the_existing_source_and_back_clears_the_selection() {
    let mut state = form_state();
    state.update(Message::AddWorktreeSourceChanged(BranchSource::Existing));
    state.update(Message::AddWorktreeBranchSelected(candidate("feat/other")));
    assert_eq!(form(&state).source, BranchSource::Existing);
    assert!(form(&state).selected_branch.is_some());

    state.update(Message::AddWorktreeSourceChanged(BranchSource::New));

    // FR-015: no residual state, and the new-branch inputs are untouched.
    assert_eq!(form(&state).selected_branch, None);
    assert_eq!(form(&state).type_, Some(ConventionalType::Feat));
    assert_eq!(form(&state).name, "login");
}

#[test]
fn the_preview_follows_the_active_source() {
    let mut state = form_state();
    assert_eq!(form(&state).preview().unwrap().branch, "feat/login");

    state.update(Message::AddWorktreeSourceChanged(BranchSource::Existing));
    // Nothing picked yet — no preview to show.
    assert!(form(&state).preview().is_err());

    state.update(Message::AddWorktreeBranchSelected(candidate(
        "release/v1.2",
    )));
    let derived = form(&state).preview().unwrap();
    // FR-014: the directory is derived from the branch, using the same naming rules.
    assert_eq!(derived.branch, "release/v1.2");
    assert_eq!(derived.dir_name, "release-v1-2");
}

#[test]
fn a_blocked_candidate_cannot_be_submitted_but_an_available_one_can() {
    let mut state = form_state();
    state.update(Message::AddWorktreeSourceChanged(BranchSource::Existing));

    state.update(Message::AddWorktreeBranchSelected(blocked_candidate(
        "main",
    )));
    assert!(
        !form(&state).can_submit(),
        "a branch that is checked out elsewhere must not be creatable (FR-012)"
    );

    state.update(Message::AddWorktreeBranchSelected(candidate("feat/free")));
    assert!(form(&state).can_submit());
}

#[test]
fn the_listed_candidates_are_stored_for_the_picker() {
    let mut state = form_state();
    state.update(Message::AddWorktreeSourceChanged(BranchSource::Existing));
    state.update(Message::AddWorktreeBranchesListed(vec![
        candidate("feat/a"),
        blocked_candidate("main"),
    ]));
    assert_eq!(form(&state).candidates.len(), 2);
}

#[test]
fn submission_is_blocked_while_a_prompt_is_open_or_a_create_is_running() {
    let mut state = form_state();
    assert!(form(&state).can_submit());

    state.update(Message::AddWorktreeConflictDetected(local_conflict()));
    assert!(!form(&state).can_submit());

    state.update(Message::AddWorktreeResolutionCancelled);
    assert!(form(&state).can_submit());

    state.update(Message::WorktreeCreateStarted);
    assert!(!form(&state).can_submit());
}

/// Spec Edge Cases — with the name on two remotes, the app must not resolve the choice itself.
#[test]
fn an_ambiguous_remote_requires_the_user_to_pick() {
    // No preference: ambiguous, so no mode — the prompt opens instead.
    assert_eq!(WorktreeForm::mode_for(&multi_remote_conflict(), None), None);

    // The row the user picked names the remote, and that is what gets tracked.
    assert_eq!(
        WorktreeForm::mode_for(&multi_remote_conflict(), Some("upstream")),
        Some(CreateMode::TrackRemote {
            remote: "upstream".to_string()
        })
    );
    assert_eq!(
        WorktreeForm::mode_for(&multi_remote_conflict(), Some("origin")),
        Some(CreateMode::TrackRemote {
            remote: "origin".to_string()
        })
    );

    // A preference for a remote that doesn't carry the branch is not silently substituted.
    assert_eq!(
        WorktreeForm::mode_for(&multi_remote_conflict(), Some("fork")),
        None
    );
}

#[test]
fn a_single_remote_needs_no_preference() {
    assert_eq!(
        WorktreeForm::mode_for(&remote_conflict(), None),
        Some(CreateMode::TrackRemote {
            remote: "origin".to_string()
        })
    );
}
