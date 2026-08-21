//! T011 — extended app base state: defaults + new message wiring (feature 005).

use micold_client::app::{on_escape, Message, State};
use micold_client::features::window::FieldId;
use micold_client::ui::terminal::StripTab;

/// Which dialog is open, by name — the question `state.overlay` answered before T037 deleted it.
/// Asked of the registry, which reads each dialog's own state, so this is the same question about
/// the same fact rather than a weaker one.
/// Switch project the way the root does: apply the outcomes, don't just produce them (T067a-6).
///
/// `switch_active` reports arriving in a project rather than performing its view resets, so a test
/// that dropped the outcomes would assert against a half-applied switch.
fn switch(state: &mut State, path: &std::path::Path) -> bool {
    match state.switch_active(path) {
        Some(outcomes) => {
            micold_client::app::drain(outcomes, |o| micold_client::app::interpret(state, o));
            true
        }
        None => false,
    }
}

/// Change the current session the way the root does (T067a-6): the commit of the outgoing row and
/// the armed scroll are outcomes now, so a test that dropped them would assert against half a move.
fn set_current(state: &mut State, next: Option<SessionId>) {
    let outcomes = state.set_current_session(next);
    micold_client::app::drain(outcomes, |o| micold_client::app::interpret(state, o));
}

fn open_dialog(state: &State) -> Option<&'static str> {
    micold_client::overlay::registry::open_dialog(state).map(|open| open.id().as_str())
}
use micold_client::features::sidebar::TagFilter;
use micold_client::features::worktree_form::WorktreeFormStatus;
use micold_core::naming::ConventionalType;
use micold_core::project::{Availability, Project};
use micold_core::session::{Session, SessionId, SessionLifecycle, SessionLocation, TerminalMode};
use micold_core::worktree::{CreateStage, Worktree, WorktreeStatus};
use std::path::PathBuf;

#[test]
fn defaults_are_empty() {
    let state = State::default();
    assert!(state.worktrees.is_empty());
    assert!(state.expanded.is_empty());
    assert!(state.active_session.is_none());
    assert!(state.worktree_form.is_none());
    assert!(state.worktree_error.is_none());
    assert_eq!(open_dialog(&state), None);
    assert!(state.active_sessions().is_empty());
}

#[test]
fn opening_the_form_sets_overlay_and_draft() {
    let mut state = State::default();
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::Opened,
    ));
    assert_eq!(open_dialog(&state), Some("add_worktree"));
    assert!(state.worktree_form.is_some());
}

#[test]
fn form_edits_build_a_derived_preview() {
    let mut state = State::default();
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::Opened,
    ));
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::TypeSelected(ConventionalType::Feat),
    ));
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::TicketChanged("ABC-1".to_string()),
    ));
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::NameChanged("Login".to_string()),
    ));

    let form = state.worktree_form.as_ref().unwrap();
    let derived = form.preview().unwrap();
    // Both carry the ticket boundary, so the branch maps back to this directory exactly (BUG-003).
    assert_eq!(derived.dir_name, "feat-abc-1_login");
    assert_eq!(derived.branch, "feat/abc-1_login");
}

#[test]
fn submitting_an_invalid_form_records_the_error() {
    let mut state = State::default();
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::Opened,
    ));
    // No type selected, no name.
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::Submitted,
    ));
    assert!(state.worktree_form.as_ref().unwrap().error.is_some());
}

#[test]
fn cancelling_the_form_clears_it() {
    let mut state = State::default();
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::Opened,
    ));
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::Cancelled,
    ));
    assert_eq!(open_dialog(&state), None);
    assert!(state.worktree_form.is_none());
}

#[test]
fn created_worktree_is_added_and_form_closed() {
    let mut state = State::default();
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::Opened,
    ));
    let wt = Worktree {
        dir_name: "feat-x".to_string(),
        path: PathBuf::from("/repo/.claude/worktrees/feat-x"),
        branch: Some("feat/x".to_string()),
        status: WorktreeStatus::Valid,
        included: false,
    };
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::Created(wt),
    ));
    assert_eq!(open_dialog(&state), None);
    assert!(state.worktree_form.is_none());
    assert_eq!(state.worktrees.len(), 1);
}

#[test]
fn create_started_marks_form_creating() {
    let mut state = State::default();
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::Opened,
    ));
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::CreateStarted(CreateMode::NewBranch),
    ));
    assert_eq!(
        state.worktree_form.as_ref().unwrap().status,
        WorktreeFormStatus::Creating
    );
}

#[test]
fn field_edits_are_ignored_while_creating() {
    let mut state = State::default();
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::Opened,
    ));
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::TypeSelected(ConventionalType::Feat),
    ));
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::NameChanged("Login".to_string()),
    ));
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::CreateStarted(CreateMode::NewBranch),
    ));

    // The whole form is inactive while a create is in flight (feature 010 follow-up), not
    // just the submit button — edits during this window must be no-ops.
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::TypeSelected(ConventionalType::Fix),
    ));
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::TicketChanged("ABC-1".to_string()),
    ));
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::NameChanged("Something else".to_string()),
    ));

    let form = state.worktree_form.as_ref().unwrap();
    assert_eq!(form.type_, Some(ConventionalType::Feat));
    assert_eq!(form.ticket, "");
    assert_eq!(form.name, "Login");
}

#[test]
fn create_failed_keeps_form_open_and_resets_status_to_editing() {
    let mut state = State::default();
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::Opened,
    ));
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::CreateStarted(CreateMode::NewBranch),
    ));
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::CreateFailed("boom".to_string()),
    ));
    assert_eq!(state.worktree_error.as_deref(), Some("boom"));
    assert!(state.worktree_form.is_some(), "form stays open for retry");
    assert_eq!(
        state.worktree_form.as_ref().unwrap().status,
        WorktreeFormStatus::Editing
    );
}

/// A worktree list change clears a create failure shown against the old list (T067a-4).
///
/// Written because probe P3 fired nothing: removing the root's routing of `WorktreesReplaced` to
/// the form left all 104 client suites green, so this behaviour was unguarded — and had been since
/// before T067a-4, when the clear lived in `worktree::loaded` with no test either. The routing is
/// what makes `worktree_error` the form's own field rather than a thing the worktree feature
/// reaches into, so it is worth pinning rather than trusting.
#[test]
fn a_worktree_list_change_clears_a_stale_create_failure() {
    let mut state = State::default();
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::Opened,
    ));
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::CreateFailed("boom".to_string()),
    ));
    assert_eq!(state.worktree_error.as_deref(), Some("boom"));

    state.update(Message::WorktreesLoaded(vec![]));
    assert!(
        state.worktree_error.is_none(),
        "discovery answering makes a failure against the previous list stale"
    );
}

/// The same, for the other path that alters the list (016 BUG-002).
#[test]
fn an_include_clears_a_stale_create_failure_too() {
    let mut state = State::default();
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::Opened,
    ));
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::CreateFailed("boom".to_string()),
    ));

    state.update(Message::WorktreeIncluded(Worktree {
        dir_name: "feat-x".to_string(),
        path: PathBuf::from("/p/.claude/worktrees/feat-x"),
        branch: Some("feat/x".to_string()),
        status: WorktreeStatus::Valid,
        included: true,
    }));
    assert!(state.worktree_error.is_none());
    assert_eq!(state.worktrees.len(), 1, "the include still lands");
}

#[test]
fn resubmitting_while_creating_is_a_no_op() {
    let mut state = State::default();
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::Opened,
    ));
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::TypeSelected(ConventionalType::Feat),
    ));
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::NameChanged("Login".to_string()),
    ));
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::CreateStarted(CreateMode::NewBranch),
    ));
    // Corrupt the form to prove the guard skips validation entirely while Creating —
    // an unguarded AddWorktreeSubmitted would call preview() and record an error here.
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::NameChanged(String::new()),
    ));
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::Submitted,
    ));
    assert!(state.worktree_form.as_ref().unwrap().error.is_none());
}

// --- Feature 013 US1: type field is a Material select (wraps iced's `pick_list`) ---

#[test]
fn selecting_a_type_sets_the_form_value() {
    let mut state = State::default();
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::Opened,
    ));
    assert_eq!(state.worktree_form.as_ref().unwrap().type_, None);

    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::TypeSelected(ConventionalType::Feat),
    ));
    assert_eq!(
        state.worktree_form.as_ref().unwrap().type_,
        Some(ConventionalType::Feat)
    );
}

#[test]
fn type_selection_is_ignored_while_creating() {
    let mut state = State::default();
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::Opened,
    ));
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::TypeSelected(ConventionalType::Feat),
    ));
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::NameChanged("Login".to_string()),
    ));
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::CreateStarted(CreateMode::NewBranch),
    ));

    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::TypeSelected(ConventionalType::Fix),
    ));
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
    // The worktree the session is started in — discovered, as it is in the application, since a
    // session is started *from* its row. Feature 024 made this matter: a row is opened for the
    // current session only when the panel knows the location (FR-013), where the old expansion
    // write happened whether the location existed or not.
    micold_client::app::drain(
        state.set_worktrees(vec![Worktree {
            dir_name: "feat-x".to_string(),
            path: PathBuf::from("/repo/.claude/worktrees/feat-x"),
            branch: Some("feat/x".to_string()),
            status: WorktreeStatus::Valid,
            included: false,
        }]),
        |o| micold_client::app::interpret(&mut state, o),
    );

    let session = Session::start_new(SessionLocation::Worktree("feat-x".to_string()));
    let id = session.id;
    state.update(Message::SessionStarted(session));
    assert_eq!(state.active_session, Some(id));
    assert_eq!(state.active_sessions().len(), 1);
    assert!(state.expanded.contains("feat-x"));
    // Feature 024: and the row reads as open, which is now a second question — open-ness is
    // derived from which session is current, and the line above is the user's own set.
    assert!(state.location_open(&SessionLocation::Worktree("feat-x".to_string())));

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
    assert!(state.expanded.is_empty());
    // Feature 024: and the row reads as open, by derivation as well as by the flag.
    assert!(state.location_open(&SessionLocation::Default));
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
        included: false,
    });
    let session = Session::start_new(SessionLocation::Worktree(dir.to_string()));
    state.update(Message::SessionStarted(session));
    state
}

/// Confirming a delete only dismisses the dialog: the daemon performs the removal and reports it,
/// and the `CatalogChanged` that follows carries git's refreshed truth.
///
/// Dropping the records here instead would make a delete git *refuses* still look like it worked —
/// the worktree would vanish from the sidebar and then silently return on the next catalog push,
/// which reads as the app resurrecting a deleted worktree rather than as the refusal it is.
#[test]
fn delete_requested_opens_confirm_then_confirmed_only_dismisses_the_dialog() {
    let mut state = state_with_worktree_and_session("feat-x");
    assert_eq!(state.active_sessions().len(), 1);

    state.update(Message::WorktreeDeleteRequested("feat-x".to_string()));
    assert_eq!(open_dialog(&state), Some("confirm_worktree_delete"));
    assert_eq!(state.worktree_delete_target.as_deref(), Some("feat-x"));
    assert!(state.worktree_menu_open.is_none());

    state.update(Message::WorktreeDeleteConfirmed);
    assert_eq!(open_dialog(&state), None);
    assert!(state.worktree_delete_target.is_none());
    assert_eq!(
        state.active_sessions().len(),
        1,
        "records stand until the daemon confirms the removal"
    );
    assert!(
        state.worktrees.iter().any(|w| w.dir_name == "feat-x"),
        "the row stands until the daemon confirms the removal"
    );
}

#[test]
fn delete_cancelled_changes_nothing() {
    let mut state = state_with_worktree_and_session("feat-x");
    state.update(Message::WorktreeDeleteRequested("feat-x".to_string()));
    state.update(Message::WorktreeDeleteCancelled);
    assert_eq!(open_dialog(&state), None);
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
        included: false,
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
    let dialog_before = open_dialog(&state);

    state.update(Message::ShowAgentWorktreesToggled);

    assert!(state.show_agent_worktrees);
    assert_eq!(state.sidebar_filters, filters_before);
    assert_eq!(state.expanded, expanded_before);
    assert_eq!(open_dialog(&state), dialog_before);
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

    assert!(switch(&mut state, &other));
    assert!(
        !state.show_agent_worktrees,
        "the incoming project must be entered with agent worktrees hidden"
    );

    // Switching back does not restore it either — nothing is remembered per project.
    let first = PathBuf::from("/repo");
    assert!(switch(&mut state, &first));
    assert!(!state.show_agent_worktrees);
}

// --- Feature 027: the tab strip is the only route between a session's panes ---

/// A terminal tab pressed while the AI pane is showing has to **show that terminal** (FR-002).
///
/// # The defect this exists for, which feature 026 shipped
///
/// `Session::select_shell` sets `active_shell` and nothing else. From the AI pane that moved a
/// field nobody was looking at: `marked_tab` derives the indicator from the *mode*, so the mark
/// stayed on the AI tab, and `session_process` derives the attachment from the mode too, so the
/// keyboard stayed with the AI CLI. Three layers agreeing on a no-op is indistinguishable from a
/// press that never registered.
///
/// It stayed hidden because a mode toggle sat beside the strip: the way to a terminal was to press
/// *that*, and it landed on whichever instance `active_shell` already named — including one this
/// no-op had quietly set. Feature 027 deletes the toggle, so the tab is the only route and has to
/// carry the whole switch.
#[test]
fn selecting_a_terminal_tab_shows_that_terminal() {
    use micold_core::session::TerminalMode;

    let mut state = state_with_worktree_and_session("feat-x");
    let id = state.active_session.unwrap();
    let shell = state
        .workspace
        .find_session_mut(id)
        .unwrap()
        .1
        .open_shell_instance();
    // The precondition that makes this a test rather than a tautology: the AI pane is showing.
    state.update(Message::TerminalAiCliSelected(id));
    assert_eq!(state.active_sessions()[0].mode, TerminalMode::AiCli);

    state.update(Message::ShellInstanceSelected(id, shell));

    assert_eq!(
        state.active_sessions()[0].mode,
        TerminalMode::Regular,
        "pressing a terminal tab has to display that terminal, not merely record it as the one a \
         later switch would land on"
    );
    assert_eq!(state.active_sessions()[0].active_shell, Some(shell));
}

/// And back again by the AI tab — the round trip the deleted mode toggle used to make on its own.
#[test]
fn the_two_kinds_of_tab_move_the_session_between_its_panes() {
    use micold_core::session::TerminalMode;

    let mut state = state_with_worktree_and_session("feat-x");
    let id = state.active_session.unwrap();
    let shell = state
        .workspace
        .find_session_mut(id)
        .unwrap()
        .1
        .open_shell_instance();
    assert_eq!(state.active_sessions()[0].mode, TerminalMode::AiCli);

    state.update(Message::ShellInstanceSelected(id, shell));
    assert_eq!(state.active_sessions()[0].mode, TerminalMode::Regular);

    state.update(Message::TerminalAiCliSelected(id));
    assert_eq!(state.active_sessions()[0].mode, TerminalMode::AiCli);
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

/// The tab context menu remembers **which tab** it was opened on (feature 012, BUG-005, FR-010b).
///
/// The whole reason the menu carries an instance id rather than reading the active one: FR-010a is
/// about restarting an instance that is *not* selected, and reading `active_shell` at the moment the
/// item is pressed would restart whichever tab the user happens to be looking at instead. That is a
/// mistake no screenshot would catch — the menu opens on the right tab and does the wrong thing.
#[test]
fn the_tab_menu_belongs_to_the_tab_it_was_opened_on() {
    let mut state = state_with_worktree_and_session("feat-x");
    let id = state.active_session.unwrap();
    let (background, active) = {
        let session = state.workspace.find_session_mut(id).unwrap().1;
        let first = session.open_shell_instance();
        let second = session.open_shell_instance();
        (first, second)
    };
    // `open_shell_instance` leaves the last-opened one active, so `background` is not selected —
    // which is the case under test.
    assert_eq!(state.active_sessions()[0].active_shell, Some(active));

    state.update(Message::StripTabMenuRequested(
        StripTab::Instance(background),
        742,
        761,
    ));
    assert_eq!(
        state.shell_instance_menu,
        Some((StripTab::Instance(background), 742, 761)),
        "the menu must record the instance whose tab was clicked, not the active one"
    );

    // Opening another tab's menu moves the one menu rather than stacking a second: two open menus
    // would each claim the next click, and only one of them would be the one the user is looking at.
    state.update(Message::StripTabMenuRequested(
        StripTab::Instance(active),
        880,
        761,
    ));
    assert_eq!(
        state.shell_instance_menu,
        Some((StripTab::Instance(active), 880, 761))
    );

    state.update(Message::ShellInstanceMenuClosed);
    assert_eq!(state.shell_instance_menu, None);
}

/// Feature 026 FR-006/FR-007/FR-011: a primary press on the AI tab shows the AI CLI and does
/// nothing else at all.
///
/// The three "nothing else" clauses are the test. FR-006 says selecting is *all* a primary press
/// does — it must not start, stop, restart or otherwise disturb any process, neither the AI CLI nor
/// any terminal instance — and FR-007 says pressing it while already displayed is a no-op with no
/// visible change. Acting on the process is FR-006a's menu, reached by a different press.
#[test]
fn pressing_the_ai_tab_shows_the_ai_cli_and_disturbs_nothing() {
    let mut state = state_with_worktree_and_session("feat-x");
    let id = state.active_session.unwrap();
    let shell = {
        let session = state.workspace.find_session_mut(id).unwrap().1;
        let opened = session.open_shell_instance();
        session.set_mode(TerminalMode::Regular);
        opened
    };
    let before = state.active_sessions()[0].clone();
    assert_eq!(before.mode, TerminalMode::Regular);

    state.update(Message::TerminalAiCliSelected(id));
    let after = &state.active_sessions()[0];
    assert_eq!(
        after.mode,
        TerminalMode::AiCli,
        "FR-006: the pane shows the AI CLI"
    );
    assert_eq!(
        after.lifecycle, before.lifecycle,
        "FR-006: selecting must not disturb the AI CLI process"
    );
    assert_eq!(
        after.active_shell,
        Some(shell),
        "FR-006: the instance the user was on stays selected, so switching back returns to it \
         rather than to an arbitrary one"
    );
    assert_eq!(
        after.shells.iter().map(|s| s.lifecycle).collect::<Vec<_>>(),
        before
            .shells
            .iter()
            .map(|s| s.lifecycle)
            .collect::<Vec<_>>(),
        "FR-006: no terminal instance is started, stopped or restarted by selecting a tab"
    );

    // FR-007: pressing it again, while it is already what the pane shows, changes nothing at all.
    let displayed = state.active_sessions()[0].clone();
    state.update(Message::TerminalAiCliSelected(id));
    let again = &state.active_sessions()[0];
    assert_eq!(again.mode, displayed.mode);
    assert_eq!(again.lifecycle, displayed.lifecycle);
    assert_eq!(again.active_shell, displayed.active_shell);
}

/// FR-006a: the menu records **which tab** it was opened on, and the AI tab is one of them.
///
/// Extends feature 012's BUG-005 test to the AI tab, through the widened `StripTab` (research R8).
/// One surface, not a second one: FR-006a defines the AI tab's menu as the terminal tab's menu with
/// an item filtered, and two surfaces is the shape that lets them drift — which is the thing FR-006a
/// is worded to prevent.
#[test]
fn the_tab_menu_records_which_tab_including_the_ai_one() {
    let mut state = state_with_worktree_and_session("feat-x");
    let id = state.active_session.unwrap();
    let shell = state
        .workspace
        .find_session_mut(id)
        .unwrap()
        .1
        .open_shell_instance();

    state.update(Message::StripTabMenuRequested(
        StripTab::Instance(shell),
        742,
        761,
    ));
    assert_eq!(
        state.shell_instance_menu,
        Some((StripTab::Instance(shell), 742, 761))
    );

    // Opening the AI tab's menu **replaces** the instance's rather than stacking a second: two open
    // menus would each claim the next click, and only one is the one the user is looking at.
    state.update(Message::StripTabMenuRequested(StripTab::Ai, 880, 761));
    assert_eq!(
        state.shell_instance_menu,
        Some((StripTab::Ai, 880, 761)),
        "the AI tab's menu is the same surface, moved — not a second one beside it"
    );

    state.update(Message::ShellInstanceMenuClosed);
    assert_eq!(state.shell_instance_menu, None);
}

/// Opening a dialog closes the tab menu with every other popover (feature 021, T031).
///
/// Registered rather than remembered: `clear_for_dialog` asks the registry which surfaces are open
/// instead of assigning to a list of fields, so this passes because `ShellInstanceMenu` is in the
/// registry. A menu left out of it would survive a dialog opening over it and take the next click.
#[test]
fn the_tab_menu_closes_when_a_dialog_opens() {
    let mut state = state_with_worktree_and_session("feat-x");
    let id = state.active_session.unwrap();
    let shell = state
        .workspace
        .find_session_mut(id)
        .unwrap()
        .1
        .open_shell_instance();

    state.update(Message::StripTabMenuRequested(
        StripTab::Instance(shell),
        742,
        761,
    ));
    assert!(state.shell_instance_menu.is_some());

    state.clear_for_dialog();
    assert_eq!(
        state.shell_instance_menu, None,
        "a menu that outlives the dialog opening over it claims the next click"
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

use micold_client::features::worktree_form::{BranchSource, ResolutionState, WorktreeForm};
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
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::Opened,
    ));
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::TypeSelected(ConventionalType::Feat),
    ));
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::NameChanged("login".to_string()),
    ));
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

    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::ConflictDetected(local_conflict()),
    ));

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
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::TicketChanged("ABC-123".to_string()),
    ));
    let before = form(&state).clone();

    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::ConflictDetected(local_conflict()),
    ));
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::ResolutionCancelled,
    ));

    let after = form(&state);
    assert_eq!(after.resolution, ResolutionState::Idle);
    // FR-007: the user lands back on the form exactly as they left it.
    assert_eq!(after.type_, before.type_);
    assert_eq!(after.ticket, before.ticket);
    assert_eq!(after.name, before.name);
    assert_eq!(after.source, before.source);
    assert_eq!(after.selected_branch, before.selected_branch);
    // And the form is still open — cancelling the prompt is not cancelling the form.
    assert_eq!(open_dialog(&state), Some("add_worktree"));
}

#[test]
fn overwrite_requires_passing_through_the_confirmation() {
    let mut state = form_state();
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::ConflictDetected(local_conflict()),
    ));

    // Invariant 1: the destructive mode cannot be chosen straight from the prompt.
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::ResolutionChosen(CreateMode::Overwrite),
    ));
    assert_eq!(
        form(&state).resolution,
        ResolutionState::Choosing {
            situation: local_conflict()
        },
        "Overwrite must not resolve directly from the choice"
    );

    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::OverwriteRequested,
    ));
    assert_eq!(
        form(&state).resolution,
        ResolutionState::ConfirmingOverwrite {
            situation: local_conflict()
        }
    );

    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::OverwriteConfirmed,
    ));
    assert_eq!(form(&state).resolution, ResolutionState::Idle);
}

#[test]
fn backing_out_of_the_confirmation_returns_to_the_choice_not_the_form() {
    // Invariant 3 (US2 AS3): reuse and cancel must still be available afterwards.
    let mut state = form_state();
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::ConflictDetected(local_conflict()),
    ));
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::OverwriteRequested,
    ));

    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::ResolutionCancelled,
    ));

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
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::ConflictDetected(remote_conflict()),
    ));
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::OverwriteRequested,
    ));

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
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::ConflictDetected(local_conflict()),
    ));
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::ResolutionChosen(CreateMode::ReuseLocal),
    ));
    assert_eq!(form(&state).resolution, ResolutionState::Idle);

    let mut state = form_state();
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::ConflictDetected(remote_conflict()),
    ));
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::ResolutionChosen(CreateMode::TrackRemote {
            remote: "origin".to_string(),
        }),
    ));
    assert_eq!(form(&state).resolution, ResolutionState::Idle);
}

/// FR-018 — "start fresh" over a remote-only name is an ordinary new branch.
#[test]
fn starting_fresh_over_a_remote_branch_resolves_to_a_new_branch() {
    let mut state = form_state();
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::ConflictDetected(remote_conflict()),
    ));
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::ResolutionChosen(CreateMode::NewBranch),
    ));
    assert_eq!(form(&state).resolution, ResolutionState::Idle);
}

/// Invariant 4: a prompt and an in-flight create cannot coexist.
#[test]
fn a_conflict_is_never_raised_while_a_create_is_in_flight() {
    let mut state = form_state();
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::CreateStarted(CreateMode::NewBranch),
    ));
    assert_eq!(form(&state).status, WorktreeFormStatus::Creating);

    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::ConflictDetected(local_conflict()),
    ));

    assert_eq!(form(&state).resolution, ResolutionState::Idle);
}

#[test]
fn edits_are_ignored_while_a_prompt_is_open() {
    let mut state = form_state();
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::ConflictDetected(local_conflict()),
    ));

    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::SourceChanged(BranchSource::Existing),
    ));
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::BranchSelected(candidate("feat/other")),
    ));

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
        state.update(Message::WorktreeForm(
            micold_client::features::worktree_form::Msg::ConflictDetected(situation),
        ));
        state.update(Message::WorktreeForm(
            micold_client::features::worktree_form::Msg::ResolutionCancelled,
        ));

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
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::SourceChanged(BranchSource::Existing),
    ));
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::BranchSelected(candidate("feat/other")),
    ));
    assert_eq!(form(&state).source, BranchSource::Existing);
    assert!(form(&state).selected_branch.is_some());

    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::SourceChanged(BranchSource::New),
    ));

    // FR-015: no residual state, and the new-branch inputs are untouched.
    assert_eq!(form(&state).selected_branch, None);
    assert_eq!(form(&state).type_, Some(ConventionalType::Feat));
    assert_eq!(form(&state).name, "login");
}

#[test]
fn the_preview_follows_the_active_source() {
    let mut state = form_state();
    assert_eq!(form(&state).preview().unwrap().branch, "feat/login");

    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::SourceChanged(BranchSource::Existing),
    ));
    // Nothing picked yet — no preview to show.
    assert!(form(&state).preview().is_err());

    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::BranchSelected(candidate("release/v1.2")),
    ));
    let derived = form(&state).preview().unwrap();
    // FR-014: the directory is derived from the branch, using the same naming rules.
    assert_eq!(derived.branch, "release/v1.2");
    assert_eq!(derived.dir_name, "release-v1-2");
}

/// A branch checked out elsewhere cannot be used — and since feature 021 that is settled one layer
/// earlier than it used to be.
///
/// Feature 016 let such a branch be *selected* and refused it at the point of creating, because
/// `pick_list` could not disable an individual row. The type-ahead can, so FR-012a moved the
/// refusal to the point of choosing: the press is ignored outright. The old assertion
/// (`!can_submit()` after selecting a blocked branch) still passes, but now only because nothing
/// was selected at all — a vacuous pass. This asserts what actually holds.
#[test]
fn a_blocked_candidate_cannot_even_become_the_selection() {
    let mut state = form_state();
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::SourceChanged(BranchSource::Existing),
    ));

    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::BranchSelected(blocked_candidate("main")),
    ));
    assert!(
        form(&state).selected_branch.is_none(),
        "a branch that is checked out elsewhere must not be choosable at all (FR-012a)"
    );
    assert!(!form(&state).can_submit());

    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::BranchSelected(candidate("feat/free")),
    ));
    assert_eq!(
        form(&state).selected_branch.as_ref().unwrap().name,
        "feat/free"
    );
    assert!(form(&state).can_submit());
}

/// …and the guard at the point of action stays, unreachable through the picker but still the
/// invariant's last line of defence. Driven directly, because nothing routes a blocked branch into
/// the form any more.
#[test]
fn the_submit_guard_still_refuses_a_blocked_selection_if_one_ever_reached_it() {
    let form = WorktreeForm {
        source: BranchSource::Existing,
        selected_branch: Some(blocked_candidate("main")),
        ..WorktreeForm::default()
    };
    assert!(!form.can_submit());
}

#[test]
fn the_listed_candidates_are_stored_for_the_picker() {
    let mut state = form_state();
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::SourceChanged(BranchSource::Existing),
    ));
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::BranchesListed(vec![
            candidate("feat/a"),
            blocked_candidate("main"),
        ]),
    ));
    assert_eq!(form(&state).candidates.len(), 2);
}

#[test]
fn submission_is_blocked_while_a_prompt_is_open_or_a_create_is_running() {
    let mut state = form_state();
    assert!(form(&state).can_submit());

    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::ConflictDetected(local_conflict()),
    ));
    assert!(!form(&state).can_submit());

    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::ResolutionCancelled,
    ));
    assert!(form(&state).can_submit());

    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::CreateStarted(CreateMode::NewBranch),
    ));
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

// --- FR-024: the progress display names the step actually being performed ----------------

#[test]
fn the_stage_label_is_worded_for_the_mode_in_flight() {
    // The whole point of FR-024: a reuse must not claim to be creating a branch.
    let mut state = form_state();
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::CreateStarted(CreateMode::ReuseLocal),
    ));
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::CreateStageChanged(
            CreateStage::CreatingWorktree,
            None,
        ),
    ));
    assert_eq!(
        form(&state).stage_label(),
        Some("Checking out existing branch")
    );

    let mut state = form_state();
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::CreateStarted(CreateMode::NewBranch),
    ));
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::CreateStageChanged(
            CreateStage::CreatingWorktree,
            None,
        ),
    ));
    assert_eq!(
        form(&state).stage_label(),
        Some("Creating branch and worktree")
    );

    let mut state = form_state();
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::CreateStarted(CreateMode::Overwrite),
    ));
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::CreateStageChanged(
            CreateStage::CreatingWorktree,
            None,
        ),
    ));
    assert_eq!(
        form(&state).stage_label(),
        Some("Replacing branch and creating worktree")
    );

    let mut state = form_state();
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::CreateStarted(CreateMode::TrackRemote {
            remote: "origin".to_string(),
        }),
    ));
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::CreateStageChanged(
            CreateStage::CreatingWorktree,
            None,
        ),
    ));
    assert_eq!(
        form(&state).stage_label(),
        Some("Creating tracking branch and worktree")
    );
}

#[test]
fn stages_that_do_not_vary_by_mode_read_the_same_everywhere() {
    for mode in [CreateMode::NewBranch, CreateMode::ReuseLocal] {
        let mut state = form_state();
        state.update(Message::WorktreeForm(
            micold_client::features::worktree_form::Msg::CreateStarted(mode),
        ));
        state.update(Message::WorktreeForm(
            micold_client::features::worktree_form::Msg::CreateStageChanged(
                CreateStage::SettingUpSubmodules,
                None,
            ),
        ));
        assert_eq!(form(&state).stage_label(), Some("Setting up submodules"));
    }
}

#[test]
fn there_is_no_stage_until_the_daemon_reports_one() {
    // The window between sending the RPC and git starting is real; the view falls back to the
    // generic wording rather than inventing a step.
    let mut state = form_state();
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::CreateStarted(CreateMode::ReuseLocal),
    ));
    assert_eq!(form(&state).stage_label(), None);
}

#[test]
fn a_new_attempt_never_inherits_the_previous_attempts_stage() {
    let mut state = form_state();
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::CreateStarted(CreateMode::Overwrite),
    ));
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::CreateStageChanged(
            CreateStage::CreatingWorktree,
            None,
        ),
    ));
    // The create fails and the user retries with a plain new branch.
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::CreateFailed("boom".to_string()),
    ));
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::CreateStarted(CreateMode::NewBranch),
    ));

    assert_eq!(form(&state).stage_label(), None);
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::CreateStageChanged(
            CreateStage::CreatingWorktree,
            None,
        ),
    ));
    assert_eq!(
        form(&state).stage_label(),
        Some("Creating branch and worktree"),
        "a retry must not keep the previous attempt's wording"
    );
}

// --- BUG-009 T123: a long stage shows where it has got to, not just its name -------------

#[test]
fn a_live_output_line_is_kept_beside_the_stage_it_belongs_to() {
    // The reported case: "Setting up submodules" sat unchanged for the length of a fetch. The
    // daemon rate-limits these; the form's job is simply to hold the latest one.
    let mut state = form_state();
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::CreateStarted(CreateMode::NewBranch),
    ));
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::CreateStageChanged(
            CreateStage::SettingUpSubmodules,
            None,
        ),
    ));
    assert_eq!(
        form(&state).stage_detail,
        None,
        "a stage arrives on its own"
    );

    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::CreateStageChanged(
            CreateStage::SettingUpSubmodules,
            Some("Cloning into 'vendor/a'…".to_string()),
        ),
    ));
    assert_eq!(
        form(&state).stage_detail.as_deref(),
        Some("Cloning into 'vendor/a'…")
    );

    // Superseded, not accumulated — this is a "where it is now" signal, not a log.
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::CreateStageChanged(
            CreateStage::SettingUpSubmodules,
            Some("Receiving objects:  47%".to_string()),
        ),
    ));
    assert_eq!(
        form(&state).stage_detail.as_deref(),
        Some("Receiving objects:  47%")
    );
    assert_eq!(
        form(&state).stage_label(),
        Some("Setting up submodules"),
        "a line never displaces the stage it describes"
    );
}

#[test]
fn a_stage_change_drops_the_previous_stages_trailing_line() {
    // Otherwise a rollback would be captioned with the fetch line that preceded it — the most
    // misleading moment to show a stale line.
    let mut state = form_state();
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::CreateStarted(CreateMode::NewBranch),
    ));
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::CreateStageChanged(
            CreateStage::SettingUpSubmodules,
            None,
        ),
    ));
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::CreateStageChanged(
            CreateStage::SettingUpSubmodules,
            Some("Receiving objects:  47%".to_string()),
        ),
    ));
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::CreateStageChanged(
            CreateStage::RollingBack,
            None,
        ),
    ));

    assert_eq!(form(&state).stage_label(), Some("Rolling back"));
    assert_eq!(form(&state).stage_detail, None);
}

#[test]
fn a_new_attempt_never_inherits_the_previous_attempts_line() {
    let mut state = form_state();
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::CreateStarted(CreateMode::NewBranch),
    ));
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::CreateStageChanged(
            CreateStage::SettingUpSubmodules,
            None,
        ),
    ));
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::CreateStageChanged(
            CreateStage::SettingUpSubmodules,
            Some("Cloning into 'vendor/a'…".to_string()),
        ),
    ));
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::CreateFailed("boom".to_string()),
    ));
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::CreateStarted(CreateMode::NewBranch),
    ));

    assert_eq!(form(&state).stage_detail, None);
}

// ---- BUG-003: which field holds the keyboard ----

#[test]
fn a_field_that_takes_the_keyboard_is_the_one_the_view_draws_focused() {
    let mut state = State::default();
    assert_eq!(
        state.focused_field, None,
        "nothing is focused to begin with"
    );

    state.update(Message::FieldFocusChanged(FieldId::RenameProjectName, true));

    assert_eq!(state.focused_field, Some(FieldId::RenameProjectName));
}

#[test]
fn moving_between_two_fields_leaves_the_second_focused_whichever_order_the_reports_arrive() {
    let mut state = State::default();
    state.update(Message::FieldFocusChanged(FieldId::AddWorktreeTicket, true));

    // Gaining and losing are reported by two different widgets, in whichever order the frame
    // produced them. The late blur is from the field that no longer holds focus and must be
    // ignored — believing it would leave both fields at rest, which is the bug reappearing on
    // every click from one field to the next.
    state.update(Message::FieldFocusChanged(FieldId::AddWorktreeName, true));
    state.update(Message::FieldFocusChanged(
        FieldId::AddWorktreeTicket,
        false,
    ));

    assert_eq!(state.focused_field, Some(FieldId::AddWorktreeName));
}

#[test]
fn a_field_losing_the_keyboard_leaves_nothing_focused() {
    let mut state = State::default();
    state.update(Message::FieldFocusChanged(
        FieldId::SettingsScrollback,
        true,
    ));

    state.update(Message::FieldFocusChanged(
        FieldId::SettingsScrollback,
        false,
    ));

    assert_eq!(state.focused_field, None);
}

#[test]
fn opening_a_dialog_forgets_the_field_that_had_focus() {
    let mut state = State::default();
    state.update(Message::FieldFocusChanged(FieldId::RenameProjectName, true));

    // The fields that reported focus belong to a widget tree being torn down, and will never report
    // losing it. A remembered focus would outlive them and draw the next dialog's field focused
    // over an input nobody has clicked.
    state.update(Message::SettingsOpened);

    assert_eq!(state.focused_field, None);
}

// --- Feature 024: collapsing the row the app opened -------------------------------------------
//
// Contract §2.1. This lands in the foundational phase, ahead of the toggle it covers, because
// `app.rs` is a render-free reducer with decision logic of its own — Principle I's GUI-wiring
// exception does not reach it, so Red comes first here even though the toggle already exists.

fn state_with_current_session_in(dir: &str) -> State {
    let mut state = State::default();
    let path = PathBuf::from("/repo");
    state.workspace.projects.push(Project {
        path: path.clone(),
        display_name: "repo".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    state.workspace.active = Some(path.clone());
    state.worktrees = vec![Worktree {
        dir_name: dir.to_string(),
        path: PathBuf::from(format!("/repo/.claude/worktrees/{dir}")),
        branch: Some(format!("feat/{dir}")),
        status: WorktreeStatus::Valid,
        included: false,
    }];
    let session = Session::start_new(SessionLocation::Worktree(dir.to_string()));
    let id = session.id;
    state.workspace.sessions.insert(path, vec![session]);
    state.active_session = Some(id);
    state
}

#[test]
fn collapsing_the_revealed_row_closes_it_and_it_stays_closed() {
    let mut state = state_with_current_session_in("feat-a");
    let location = SessionLocation::Worktree("feat-a".to_string());
    assert!(
        state.location_open(&location),
        "precondition: the row is open because it holds the current session"
    );

    state.update(Message::WorktreeExpansionToggled("feat-a".to_string()));

    assert!(
        !state.location_open(&location),
        "the twisty closes a row the app opened, exactly as it closes one the user opened — \
         otherwise the control does nothing on the one row the feature added (FR-005)"
    );
    assert_eq!(
        state.reveal_suppressed_for, state.active_session,
        "and the close is remembered against the session it was made for, so a later reveal for \
         a different session is not swallowed by it (invariant I2)"
    );

    micold_client::app::drain(
        state.set_worktrees(vec![Worktree {
            dir_name: "feat-a".to_string(),
            path: PathBuf::from("/repo/.claude/worktrees/feat-a"),
            branch: Some("feat/feat-a".to_string()),
            status: WorktreeStatus::Valid,
            included: false,
        }]),
        |o| micold_client::app::interpret(&mut state, o),
    );
    assert!(
        !state.location_open(&location),
        "and a worktree re-discovery does not undo it — the case a one-shot implementation gets \
         wrong (SC-008)"
    );
}

#[test]
fn re_expanding_a_suppressed_row_lifts_the_suppression() {
    let mut state = state_with_current_session_in("feat-a");
    let location = SessionLocation::Worktree("feat-a".to_string());

    state.update(Message::WorktreeExpansionToggled("feat-a".to_string()));
    state.update(Message::WorktreeExpansionToggled("feat-a".to_string()));

    assert!(
        state.location_open(&location),
        "the twisty is a toggle in both directions, on the revealed row as much as any other"
    );
    assert!(
        state.reveal_suppressed_for.is_none(),
        "re-opening it by hand withdraws the close, rather than leaving a suppression that only \
         a change of session can clear"
    );
}

#[test]
fn the_default_rows_twisty_suppresses_the_same_way() {
    let mut state = State::default();
    let path = PathBuf::from("/repo");
    state.workspace.projects.push(Project {
        path: path.clone(),
        display_name: "repo".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    state.workspace.active = Some(path.clone());
    let session = Session::start_new(SessionLocation::Default);
    let id = session.id;
    state.workspace.sessions.insert(path, vec![session]);
    state.active_session = Some(id);
    assert!(state.location_open(&SessionLocation::Default));

    state.update(Message::DefaultExpansionToggled);

    assert!(
        !state.location_open(&SessionLocation::Default),
        "the project-root row is a location like any other — FR-005 is not worktree-only"
    );
    assert_eq!(state.reveal_suppressed_for, state.active_session);
}

// --- Feature 024: what a change of current session does to the rows ---------------------------
//
// Contract §2.3 and §3. The commit is the clause that stops a derived model from snapping a row
// shut the instant its session stops being current (FR-001c).

#[test]
fn a_location_that_stops_holding_the_current_session_stays_open() {
    let mut state = state_with_current_session_in("feat-a");
    let location = SessionLocation::Worktree("feat-a".to_string());

    set_current(&mut state, None);

    assert!(
        state.location_open(&location),
        "ceasing to be current takes away the mark, never the open row — otherwise closing the \
         session you were on would collapse the row under you, taking its siblings out of view \
         with it (FR-001c)"
    );
    assert!(
        state.expanded.contains("feat-a"),
        "and it stays open by becoming ordinary user-open state, which is the honest description \
         of what the user was looking at (invariant I3)"
    );
}

#[test]
fn a_row_the_user_closed_is_not_re_opened_by_the_commit() {
    let mut state = state_with_current_session_in("feat-a");
    let location = SessionLocation::Worktree("feat-a".to_string());
    state.update(Message::WorktreeExpansionToggled("feat-a".to_string()));

    set_current(&mut state, None);

    assert!(
        !state.location_open(&location),
        "committing a row the user had closed would re-open exactly what they closed — the commit \
         is for rows that were actually on screen (contract §2.3)"
    );
}

#[test]
fn clearing_the_current_session_arms_no_scroll() {
    let mut state = state_with_current_session_in("feat-a");
    state.pending_reveal_scroll = false;

    set_current(&mut state, None);

    assert!(
        !state.pending_reveal_scroll,
        "there is no row to scroll to. An armed scroll with no target stays armed — nothing drains \
         it — and then fires against whatever row appears next; FR-001a forbids scrolling at all \
         when the user closes the session they were on (invariant I5)"
    );
}

#[test]
fn a_change_of_current_session_lifts_a_suppression_made_against_the_old_one() {
    let mut state = state_with_current_session_in("feat-a");
    state.update(Message::WorktreeExpansionToggled("feat-a".to_string()));
    assert_eq!(state.reveal_suppressed_for, state.active_session);

    let next = Session::start_new(SessionLocation::Worktree("feat-a".to_string()));
    let next_id = next.id;
    let path = state.workspace.active.clone().unwrap();
    state.workspace.sessions.get_mut(&path).unwrap().push(next);
    set_current(&mut state, Some(next_id));

    assert!(
        state.reveal_suppressed_for.is_none(),
        "the close was made against a session that is no longer current; keeping it would swallow \
         the next reveal for a reason the user could not see (invariant I2)"
    );
    assert!(
        state.pending_reveal_scroll,
        "and the new current session arms its own scroll"
    );
}

// --- Feature 024: draining the armed reveal into a scroll -------------------------------------
//
// Contract §6.4 and §6.5. The arm is a flag rather than an offset because the offset cannot be
// known when it is set: the incoming project's worktrees are discovered asynchronously and the
// viewport reports its height only once laid out.

/// A project with `count` worktrees, the current session in the last of them — SC-003's shape.
fn state_with_many_worktrees(count: usize) -> State {
    let mut state = State::default();
    let path = PathBuf::from("/repo");
    state.workspace.projects.push(Project {
        path: path.clone(),
        display_name: "repo".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    state.workspace.active = Some(path.clone());
    state.worktrees = (0..count)
        .map(|i| Worktree {
            dir_name: format!("feat-{i:02}"),
            path: PathBuf::from(format!("/repo/.claude/worktrees/feat-{i:02}")),
            branch: Some(format!("feat/{i:02}")),
            status: WorktreeStatus::Valid,
            included: false,
        })
        .collect();
    let session = Session::start_new(SessionLocation::Worktree(format!("feat-{:02}", count - 1)));
    let id = session.id;
    state.workspace.sessions.insert(path, vec![session]);
    state.active_session = Some(id);
    state
}

#[test]
fn a_row_near_the_bottom_of_a_long_list_is_scrolled_to() {
    let mut state = state_with_many_worktrees(30);
    state.sidebar_viewport_height = 400;

    assert!(
        state.current_session_is_listed(),
        "precondition: the row exists in the projection"
    );
    let offset = state
        .reveal_scroll_offset()
        .expect("a row 30 locations down is not visible in a 400px viewport");
    assert!(
        offset > 0,
        "SC-003: in a project with 30 locations the current session's row is brought on screen, \
         which is the whole of US2 — opening the right row is no use if it is below the fold"
    );
}

#[test]
fn a_row_already_on_screen_is_not_scrolled_to() {
    let mut state = state_with_many_worktrees(3);
    state.sidebar_viewport_height = 400;

    assert_eq!(
        state.reveal_scroll_offset(),
        None,
        "three locations fit in a 400px viewport, so there is nothing to do — and doing something \
         anyway would jerk the list for no reason the user could see (FR-009, SC-007)"
    );
}

#[test]
fn nothing_is_scrolled_to_before_the_viewport_has_been_laid_out() {
    let state = state_with_many_worktrees(30);

    assert_eq!(
        state.sidebar_viewport_height, 0,
        "precondition: no layout has happened yet"
    );
    assert_eq!(
        state.reveal_scroll_offset(),
        None,
        "a viewport height of zero means 'not laid out yet'. Reading it as 'nothing fits' would \
         scroll on the frame before the panel knew its own size (contract §6.3)"
    );
}

#[test]
fn a_reveal_waits_for_the_worktree_list_rather_than_scrolling_to_a_stale_row() {
    let mut state = state_with_many_worktrees(30);
    state.sidebar_viewport_height = 400;
    // The switch has happened but discovery has not reported yet: the panel knows of no locations.
    micold_client::app::drain(state.set_worktrees(Vec::new()), |o| {
        micold_client::app::interpret(&mut state, o)
    });

    assert!(
        !state.current_session_is_listed(),
        "with no locations there is no row for the current session, so the reveal has nothing to \
         scroll to and must stay armed rather than scroll to an offset computed from other rows \
         (contract §6.4, research R7)"
    );
}

// --- Feature 024: which paths reveal, and which pointedly do not ------------------------------
//
// Contract §3. The rule is "any app-initiated transition of `active_session` to Some", so these
// check the behaviour at each path rather than the plumbing — `current_session_writers.rs` is what
// checks that the plumbing cannot be bypassed.

#[test]
fn starting_a_session_reveals_it() {
    let mut state = state_with_current_session_in("feat-a");
    micold_client::app::drain(
        state.set_worktrees(vec![
            Worktree {
                dir_name: "feat-a".to_string(),
                path: PathBuf::from("/repo/.claude/worktrees/feat-a"),
                branch: Some("feat/feat-a".to_string()),
                status: WorktreeStatus::Valid,
                included: false,
            },
            Worktree {
                dir_name: "feat-b".to_string(),
                path: PathBuf::from("/repo/.claude/worktrees/feat-b"),
                branch: Some("feat/feat-b".to_string()),
                status: WorktreeStatus::Valid,
                included: false,
            },
        ]),
        |o| micold_client::app::interpret(&mut state, o),
    );
    state.pending_reveal_scroll = false;

    let started = Session::start_new(SessionLocation::Worktree("feat-b".to_string()));
    let id = started.id;
    state.update(Message::SessionStarted(started));

    assert_eq!(state.active_session, Some(id));
    assert!(
        state.location_open(&SessionLocation::Worktree("feat-b".to_string())),
        "a session you just started is one the app put in front of you, so it is revealed like any \
         other (US3 scenario 2)"
    );
    assert!(state.pending_reveal_scroll, "and brought into view");
    assert!(
        state.location_open(&SessionLocation::Worktree("feat-a".to_string())),
        "while the row that held the outgoing current session stays open — ceasing to be current \
         never closes a row (FR-001c)"
    );
}

#[test]
fn clicking_a_session_marks_it_and_moves_nothing() {
    let mut state = state_with_current_session_in("feat-a");
    let other = Session::start_new(SessionLocation::Worktree("feat-a".to_string()));
    let other_id = other.id;
    let path = state.workspace.active.clone().unwrap();
    state.workspace.sessions.get_mut(&path).unwrap().push(other);
    state.pending_reveal_scroll = false;

    state.update(Message::SessionSelected(other_id));

    assert_eq!(state.active_session, Some(other_id), "it is now current");
    assert!(
        !state.pending_reveal_scroll,
        "but nothing is opened or scrolled on the user's behalf: they clicked a row they could \
         already see, and scrolling it would move the list they were reading (FR-006)"
    );
}

#[test]
fn closing_the_current_session_promotes_nothing_in_its_place() {
    let mut state = state_with_current_session_in("feat-a");
    let sibling = Session::start_new(SessionLocation::Worktree("feat-a".to_string()));
    let path = state.workspace.active.clone().unwrap();
    state
        .workspace
        .sessions
        .get_mut(&path)
        .unwrap()
        .push(sibling);
    let closing = state.active_session.unwrap();

    state.update(Message::SessionCloseRequested(closing));

    assert!(
        state.active_session.is_none(),
        "this feature reveals where you are; it does not decide where you go next. A sibling \
         session in the same location is not promoted (FR-001a)"
    );
    assert!(
        state.location_open(&SessionLocation::Worktree("feat-a".to_string())),
        "and the row stays open, so the sibling you might want next is still on screen (FR-001c)"
    );
    assert!(
        !state.pending_reveal_scroll,
        "with nothing armed to scroll to"
    );
}

#[test]
fn removing_the_current_session_behaves_the_same_way() {
    let mut state = state_with_current_session_in("feat-a");
    let removing = state.active_session.unwrap();
    state.update(Message::SessionRemoveRequested(removing));

    state.update(Message::SessionRemoveConfirmed);

    assert!(state.active_session.is_none());
    assert!(
        state.location_open(&SessionLocation::Worktree("feat-a".to_string())),
        "remove drops the record where close archives it, but neither is the app moving you to a \
         session — so neither opens, closes or scrolls anything (FR-001a)"
    );
    assert!(!state.pending_reveal_scroll);
}

// --- Feature 025: applying a project's remembered session at launch ---------------------------
//
// The launch resolves the memory with the same function a switch uses, and applies it with the
// same one — so everything feature 008 and 024 already decided about a session becoming current
// holds here too, without a second path deciding it again.

fn state_with_remembered_session() -> (State, SessionId) {
    let mut state = state_with_current_session_in("feat-a");
    let id = state.active_session.unwrap();
    state.record_foreground();
    // The shape after a restart: the memory survived, the pointer did not.
    state.active_session = None;
    state.pending_reveal_scroll = false;
    (state, id)
}

#[test]
fn applying_the_memory_makes_that_session_current_and_reveals_it() {
    let (mut state, id) = state_with_remembered_session();
    let path = state.workspace.active.clone().unwrap();

    let choice = state.explain_foreground(&path);
    set_current(&mut state, choice.session());

    assert_eq!(
        state.active_session,
        Some(id),
        "reopening lands on the session you were last using, which is the whole feature"
    );
    assert!(
        state.location_open(&SessionLocation::Worktree("feat-a".to_string())),
        "and its location is listed open — not because this feature opens it, but because feature \
         024 reveals whatever becomes current, on any app-initiated transition"
    );
}

#[test]
fn applying_the_memory_starts_only_the_session_it_displays() {
    let (mut state, id) = state_with_remembered_session();
    let path = state.workspace.active.clone().unwrap();
    let before: Vec<_> = state
        .active_sessions()
        .iter()
        .map(|s| (s.id, s.lifecycle))
        .collect();

    let choice = state.explain_foreground(&path).session();
    set_current(&mut state, choice);

    let after: Vec<_> = state
        .active_sessions()
        .iter()
        .map(|s| (s.id, s.lifecycle))
        .collect();
    assert_eq!(
        before, after,
        "the reducer moves no lifecycle. Under FR-004a the restore resumes, but by sending the \
         daemon a `SessionStart` — the daemon is the only thing that may say a session is running. \
         Setting one here would render a session as running with no process behind it, which is the \
         lie BUG-001 fixed, arrived at from the other direction"
    );
    assert_eq!(
        state.active_session,
        Some(id),
        "and exactly one session is made current — the one the start will name"
    );
}

/// The other half of the bound (SC-005a, FR-008): restoring one project's memory resolves nothing
/// for any other project, so a launch cannot fan out into a resume per remembered project.
#[test]
fn applying_one_projects_memory_leaves_every_other_project_alone() {
    let (mut state, _id) = state_with_remembered_session();
    let opened = state.workspace.active.clone().unwrap();
    // A second project the user has not opened, carrying a memory of its own.
    let elsewhere = PathBuf::from("/repo/elsewhere");
    let other_session = SessionId::new();
    state.workspace.sessions.insert(
        elsewhere.clone(),
        vec![Session::start_new(SessionLocation::Default)],
    );
    state
        .workspace
        .foreground_by_project
        .insert(elsewhere.clone(), other_session);
    let untouched = state.workspace.sessions.get(&elsewhere).cloned().unwrap();

    let choice = state.explain_foreground(&opened).session();
    set_current(&mut state, choice);

    assert_eq!(
        state.workspace.sessions.get(&elsewhere),
        Some(&untouched),
        "the unopened project's sessions are not read, resolved, or altered by another project's \
         restore — nothing will be started for it until the user switches there"
    );
    assert_eq!(
        state.workspace.foreground_by_project.get(&elsewhere),
        Some(&other_session),
        "and its memory is still waiting, unspent"
    );
}

#[test]
fn a_memory_whose_worktree_is_gone_is_still_restored() {
    let (mut state, id) = state_with_remembered_session();
    let path = state.workspace.active.clone().unwrap();
    // Deleted outside the application: the worktree is gone from discovery, but the session's
    // record survives in the project's state file.
    micold_client::app::drain(state.set_worktrees(Vec::new()), |o| {
        micold_client::app::interpret(&mut state, o)
    });
    state.expanded.insert("kept-open".to_string());

    let choice = state.explain_foreground(&path).session();
    set_current(&mut state, choice);

    assert_eq!(
        state.active_session,
        Some(id),
        "the application already lists a session whose worktree is missing and lets you select it,          so refusing to *return* you to it would be the same inconsistency BUG-001 was about.          Declining would also need the worktree list at resolve time, which a project switch does          not have yet — one rule that breaks switching to handle a case the user can see"
    );
    assert!(
        state.expanded.contains("kept-open"),
        "and the rest of the project is untouched — a memory that cannot be honoured must not cost \
         the user anything else (FR-006)"
    );
}

#[test]
fn a_memory_naming_a_closed_session_restores_nothing_and_disturbs_nothing() {
    let (mut state, id) = state_with_remembered_session();
    let path = state.workspace.active.clone().unwrap();
    // Closed between one run and the next.
    if let Some((_, session)) = state.workspace.find_session_mut(id) {
        session.archive();
    }
    state.expanded.insert("kept-open".to_string());

    let choice = state.explain_foreground(&path).session();
    set_current(&mut state, choice);

    assert!(
        state.active_session.is_none(),
        "a closed session is not listed at all, so restoring one would display something the panel \
         cannot show (FR-005). Nothing is chosen in its place either (FR-007)"
    );
    assert!(
        state.expanded.contains("kept-open"),
        "and the rest of the project is exactly as it was (FR-006)"
    );
}
