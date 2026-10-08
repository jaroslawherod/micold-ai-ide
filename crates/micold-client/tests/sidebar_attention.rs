//! The attention indicator on the sidebar's location rows (feature 575, US1, US2; contract
//! `attention-indicator.md` A1, A2, A7).
//!
//! Each worktree row and the Default row carries the number of its sessions that count as unread:
//! unread, not closed, and not the session this window has in view. The number does not depend on
//! whether the row is expanded, and a worktree the sidebar hides has no row to carry it.

use micold_client::app::{Message, State};
use micold_client::catalog_sync::reconcile_catalog;
use micold_client::features::attention::in_view;
use micold_client::features::sidebar::Msg as SidebarMsg;
use micold_client::features::sidebar::{
    unread_tooltip_line, with_unread_line, worktree_tooltip, SidebarEntry, TagFilter,
    DEFAULT_LOCATION_LABEL,
};
use micold_client::features::worktree::Msg as WorktreeMsg;
use micold_core::naming::ConventionalType;
use micold_core::project::{Availability, Project};
use micold_core::protocol::messages::{
    ActivitySignal, CatalogSnapshot, ProjectSnapshot, SessionSummary, WireLifecycle,
};
use micold_core::session::{AiCli, Session, SessionId, SessionLabel, SessionLocation};
use micold_core::worktree::{Worktree, WorktreeStatus};
use std::path::{Path, PathBuf};

const REPO: &str = "/repo";
const FEATURE_X: &str = "feat-x";
const FEATURE_Y: &str = "feat-y";
/// A worktree this app has no record of creating: hidden while "Show agent worktrees" is off.
const AGENT: &str = "agent-00112233445566aa";

fn worktree(dir: &str, status: WorktreeStatus) -> Worktree {
    Worktree {
        dir_name: dir.to_string(),
        path: Path::new(REPO).join(".claude/worktrees").join(dir),
        branch: Some(format!("feat/{dir}")),
        status,
        included: false,
    }
}

fn session_at(location: SessionLocation, unread: bool) -> Session {
    let mut session = Session::start_new(location, AiCli::ClaudeCode);
    session.unread = unread;
    session
}

fn in_worktree(dir: &str, unread: bool) -> Session {
    session_at(SessionLocation::Worktree(dir.to_string()), unread)
}

/// The active project `/repo` with the recorded worktrees `feat-x` and `feat-y`, the unrecorded
/// `AGENT` one, and `sessions`.
fn state_with(sessions: Vec<Session>) -> State {
    state_with_worktrees(
        vec![
            worktree(FEATURE_X, WorktreeStatus::Valid),
            worktree(FEATURE_Y, WorktreeStatus::Valid),
        ],
        sessions,
    )
}

fn state_with_worktrees(recorded: Vec<Worktree>, sessions: Vec<Session>) -> State {
    let mut state = State::default();
    let path = PathBuf::from(REPO);
    state.workspace.projects.push(Project {
        path: path.clone(),
        display_name: "repo".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    state.workspace.active = Some(path.clone());
    for w in &recorded {
        state.workspace.record_user_created(&path, &w.dir_name);
    }
    state.worktree.worktrees = recorded
        .into_iter()
        .chain([worktree(AGENT, WorktreeStatus::Valid)])
        .collect();
    state.workspace.sessions.insert(path, sessions);
    state
}

fn entry_name(entry: &SidebarEntry) -> String {
    match entry {
        SidebarEntry::Default(_) => "Default".to_string(),
        SidebarEntry::Worktree(node) => node.worktree.dir_name.clone(),
    }
}

/// Every listed location with its indicator's number, as this window would draw it.
fn counts(state: &State) -> Vec<(String, usize)> {
    let in_view = in_view(&state.attention);
    state
        .sidebar_entries()
        .iter()
        .map(|entry| (entry_name(entry), entry.unread_count(in_view)))
        .collect()
}

fn count_of(state: &State, location: &str) -> Option<usize> {
    counts(state)
        .into_iter()
        .find(|(name, _)| name == location)
        .map(|(_, n)| n)
}

/// Make `session` current and report it in view, as a focused window does.
fn view(state: &mut State, session: SessionId) {
    state.session.active = Some(session);
    let _ = state.view_report(true);
}

/// US1 scenario 1 (FR-001, FR-002).
#[test]
fn a_worktree_row_counts_its_unread_sessions_and_a_read_only_one_shows_none() {
    let state = state_with(vec![
        in_worktree(FEATURE_X, true),
        in_worktree(FEATURE_X, true),
        in_worktree(FEATURE_Y, false),
    ]);

    assert_eq!(
        count_of(&state, FEATURE_X),
        Some(2),
        "feat-x holds two unread sessions"
    );
    assert_eq!(
        count_of(&state, FEATURE_Y),
        Some(0),
        "feat-y holds only read sessions, so its row shows no indicator"
    );
}

/// US1 scenario 2 (FR-001).
#[test]
fn the_default_row_counts_its_unread_sessions() {
    let state = state_with(vec![session_at(SessionLocation::Default, true)]);

    assert_eq!(
        count_of(&state, "Default"),
        Some(1),
        "the project-root row carries the indicator like a worktree row does"
    );
}

/// US1 scenario 3 (FR-003): the count is the same with the row expanded and collapsed.
#[test]
fn expanding_a_row_does_not_change_its_count() {
    let mut state = state_with(vec![
        in_worktree(FEATURE_X, true),
        in_worktree(FEATURE_X, true),
        session_at(SessionLocation::Default, true),
    ]);
    let collapsed = counts(&state);

    state.sidebar.expanded.insert(FEATURE_X.to_string());
    state.sidebar.default_expanded = true;
    for location in [
        SessionLocation::Worktree(FEATURE_X.to_string()),
        SessionLocation::Default,
    ] {
        assert!(
            state.location_open(&location),
            "the fixture must open {location:?} for this test to compare anything"
        );
    }

    assert_eq!(
        counts(&state),
        collapsed,
        "an expanded row carries the same number as when it was collapsed"
    );
    assert_eq!(
        count_of(&state, FEATURE_X),
        Some(2),
        "the expanded feat-x row still counts its two unread sessions"
    );
}

/// US1 scenario 4: a session awaiting input the user has already viewed is not unread.
#[test]
fn a_viewed_session_awaiting_input_does_not_count() {
    let mut viewed = in_worktree(FEATURE_X, false);
    viewed.activity = ActivitySignal::AwaitingInput;
    let state = state_with(vec![in_worktree(FEATURE_X, true), viewed]);

    assert_eq!(
        count_of(&state, FEATURE_X),
        Some(1),
        "a viewed session awaiting input is read, so only the unread one is counted"
    );
}

/// Edge case "The session in view" (FR-002, 039 FR-019): the count does not wait for the service.
#[test]
fn the_session_in_view_is_not_counted() {
    let viewed = in_worktree(FEATURE_X, true);
    let id = viewed.id;
    let mut state = state_with(vec![viewed, in_worktree(FEATURE_X, true)]);
    assert_eq!(
        count_of(&state, FEATURE_X),
        Some(2),
        "before anything is in view, both unread sessions count"
    );

    view(&mut state, id);

    assert_eq!(
        count_of(&state, FEATURE_X),
        Some(1),
        "the session in view no longer counts, though the catalog still says unread"
    );
}

/// US1 scenario 5, sidebar half; edge case "only unread sessions were closed".
#[test]
fn a_closed_unread_session_adds_nothing_to_its_row() {
    let mut closed = in_worktree(FEATURE_X, true);
    closed.archived = true;
    let state = state_with(vec![closed]);

    assert_eq!(
        count_of(&state, FEATURE_X),
        Some(0),
        "a closed session has no row and adds nothing to its location's indicator"
    );
}

/// A project with no worktrees has only the Default row, and it carries the count.
#[test]
fn a_project_without_worktrees_counts_on_its_default_row() {
    let state = state_with_worktrees(
        Vec::new(),
        vec![
            session_at(SessionLocation::Default, true),
            session_at(SessionLocation::Default, true),
        ],
    );

    assert_eq!(
        counts(&state),
        [("Default".to_string(), 2)],
        "with no worktrees, the Default row is the only row and carries both unread sessions"
    );
}

/// Contract A2: a worktree the tag filters or the agent setting hide has no row, so no count.
#[test]
fn a_hidden_worktree_has_no_row_to_carry_a_count() {
    let mut state = state_with(vec![in_worktree(FEATURE_X, true), in_worktree(AGENT, true)]);
    state
        .sidebar
        .filters
        .insert(TagFilter::Type(ConventionalType::Fix));

    assert_eq!(
        count_of(&state, FEATURE_X),
        None,
        "a worktree the tag filter hides has no row (008 FR-025)"
    );
    assert_eq!(
        count_of(&state, AGENT),
        None,
        "an agent worktree hidden by the setting has no row (014)"
    );
}

/// Contract A2: the row feature 024 re-admits for the current session carries its count.
#[test]
fn the_row_shown_for_the_current_session_carries_its_count() {
    let current = in_worktree(AGENT, false);
    let id = current.id;
    let mut state = state_with(vec![current, in_worktree(AGENT, true)]);
    state.session.active = Some(id);

    let readmitted = state
        .sidebar_entries()
        .into_iter()
        .find_map(|entry| match entry {
            SidebarEntry::Worktree(node) if node.worktree.dir_name == AGENT => Some(node),
            _ => None,
        })
        .expect("the hidden worktree holding the current session is listed (024)");
    assert!(
        readmitted.shown_for_current_session,
        "the agent worktree's row is listed only because it holds the current session"
    );
    assert_eq!(
        count_of(&state, AGENT),
        Some(1),
        "the re-admitted row counts its unread session like any other row"
    );
}

/// Contract A2: a missing or invalid worktree (010 FR-011) still has a row and carries its count.
#[test]
fn a_missing_or_invalid_worktree_carries_its_count() {
    let state = state_with_worktrees(
        vec![
            worktree("feat-gone", WorktreeStatus::Missing),
            worktree("feat-bad", WorktreeStatus::Invalid),
        ],
        vec![
            in_worktree("feat-gone", true),
            in_worktree("feat-bad", true),
        ],
    );

    assert_eq!(
        count_of(&state, "feat-gone"),
        Some(1),
        "a missing worktree keeps its row, so its unread session is counted there"
    );
    assert_eq!(
        count_of(&state, "feat-bad"),
        Some(1),
        "an invalid worktree keeps its row, so its unread session is counted there"
    );
}

// --- The tooltip line (FR-005, contract A7) ---

/// FR-005: no unread session, no line.
#[test]
fn unread_tooltip_line_is_absent_for_zero() {
    assert_eq!(
        unread_tooltip_line(0),
        None,
        "a row with no unread session adds no tooltip line"
    );
}

/// FR-005: one unread session reads in the singular.
#[test]
fn unread_tooltip_line_is_singular_for_one() {
    assert_eq!(
        unread_tooltip_line(1).as_deref(),
        Some("1 unread session"),
        "one unread session is named in the singular"
    );
}

/// FR-005: more than one unread session reads in the plural.
#[test]
fn unread_tooltip_line_is_plural_for_several() {
    assert_eq!(
        unread_tooltip_line(12).as_deref(),
        Some("12 unread sessions"),
        "several unread sessions are named in the plural with their number"
    );
}

/// Contract A7: a row with nothing unread keeps its tooltip as it was.
#[test]
fn with_unread_line_leaves_the_tooltip_unchanged_for_zero() {
    assert_eq!(
        with_unread_line(DEFAULT_LOCATION_LABEL.to_string(), 0),
        DEFAULT_LOCATION_LABEL,
        "no unread session leaves the row's tooltip untouched"
    );
}

/// Contract A7: the unread line goes on its own line after the Default row's label.
#[test]
fn with_unread_line_appends_after_the_default_rows_label() {
    assert_eq!(
        with_unread_line(DEFAULT_LOCATION_LABEL.to_string(), 2),
        format!("{DEFAULT_LOCATION_LABEL}\n2 unread sessions"),
        "the unread line follows the label on a new line"
    );
}

/// Contract A7: the unread line goes last, after every line of a worktree's tooltip.
#[test]
fn with_unread_line_appends_after_a_multi_line_worktree_tooltip() {
    let tooltip = worktree_tooltip(
        Some(Path::new(REPO)),
        &worktree(FEATURE_X, WorktreeStatus::Missing),
        "x",
        None,
    );
    assert!(
        tooltip.lines().count() > 1,
        "the fixture must be multi-line"
    );

    let with_line = with_unread_line(tooltip.clone(), 1);

    assert_eq!(
        with_line,
        format!("{tooltip}\n1 unread session"),
        "the worktree tooltip is kept whole and the unread line is appended"
    );
    assert_eq!(
        with_line.lines().last(),
        Some("1 unread session"),
        "the unread line is the tooltip's last line"
    );
}

// --- Live updates (US2, FR-007, FR-009) ---

/// A catalog snapshot of `REPO` holding `sessions` as `(id, worktree, unread)`: what the service
/// publishes after every change (039 FR-024).
fn catalog(sessions: &[(SessionId, Option<&str>, bool)]) -> CatalogSnapshot {
    CatalogSnapshot {
        schema_version: 1,
        last_active: Some(PathBuf::from(REPO)),
        projects: vec![ProjectSnapshot {
            path: PathBuf::from(REPO),
            display_name: "repo".to_string(),
            is_git_repo: true,
            available: true,
            worktrees: Vec::new(),
            sessions: sessions
                .iter()
                .map(|&(id, worktree, unread)| SessionSummary {
                    id,
                    worktree_dir: worktree.map(str::to_string),
                    title: SessionLabel::Pending,
                    lifecycle: WireLifecycle::Running,
                    activity: ActivitySignal::AwaitingInput,
                    provider: AiCli::ClaudeCode,
                    input_serial: 0,
                    live_shells: Vec::new(),
                    attention_seq: 1,
                    unread,
                })
                .collect(),
        }],
        env_include_failures: Vec::new(),
    }
}

/// `feat-x` with two read sessions, and their ids.
fn two_read_sessions_in_feature_x() -> (State, SessionId, SessionId) {
    let (a, b) = (in_worktree(FEATURE_X, false), in_worktree(FEATURE_X, false));
    let (a_id, b_id) = (a.id, b.id);
    (state_with(vec![a, b]), a_id, b_id)
}

/// US2 scenarios 1 and 3 (FR-007): the count follows the service's unread state.
#[test]
fn a_catalog_update_raises_and_lowers_the_count() {
    let (mut state, a, b) = two_read_sessions_in_feature_x();
    assert_eq!(
        count_of(&state, FEATURE_X),
        Some(0),
        "two read sessions start the row at zero"
    );

    reconcile_catalog(
        &mut state,
        &catalog(&[(a, Some(FEATURE_X), true), (b, Some(FEATURE_X), false)]),
        false,
    );
    assert_eq!(
        count_of(&state, FEATURE_X),
        Some(1),
        "a session that became unread adds to its row"
    );

    reconcile_catalog(
        &mut state,
        &catalog(&[(a, Some(FEATURE_X), false), (b, Some(FEATURE_X), false)]),
        false,
    );
    assert_eq!(
        count_of(&state, FEATURE_X),
        Some(0),
        "a session that stopped being unread no longer adds to its row"
    );
}

/// US2 scenario 2 (039 FR-019): a session coming into view stops counting at once, before the
/// service's next snapshot says it is read.
#[test]
fn selecting_a_session_drops_the_count_before_any_catalog_update() {
    let (mut state, a, b) = two_read_sessions_in_feature_x();
    reconcile_catalog(
        &mut state,
        &catalog(&[(a, Some(FEATURE_X), true), (b, Some(FEATURE_X), true)]),
        false,
    );
    assert_eq!(
        count_of(&state, FEATURE_X),
        Some(2),
        "the snapshot marked both sessions unread"
    );

    view(&mut state, a);

    assert_eq!(
        count_of(&state, FEATURE_X),
        Some(1),
        "the session in view no longer counts, though no snapshot has said so yet"
    );
}

/// FR-007: a session the service no longer reports (closed or removed) stops counting.
#[test]
fn a_closed_or_removed_session_lowers_the_count() {
    let (mut state, a, b) = two_read_sessions_in_feature_x();
    reconcile_catalog(
        &mut state,
        &catalog(&[(a, Some(FEATURE_X), true), (b, Some(FEATURE_X), true)]),
        false,
    );
    assert_eq!(
        count_of(&state, FEATURE_X),
        Some(2),
        "the snapshot marked both sessions unread"
    );

    reconcile_catalog(&mut state, &catalog(&[(a, Some(FEATURE_X), true)]), false);

    assert_eq!(
        count_of(&state, FEATURE_X),
        Some(1),
        "a session closed or removed on the service's side adds nothing"
    );
}

/// Edge case "A burst of changes": several snapshots in a row settle on the last one's count.
#[test]
fn a_burst_of_updates_settles_on_the_right_count() {
    let (mut state, a, b) = two_read_sessions_in_feature_x();
    for snapshot in [
        catalog(&[(a, Some(FEATURE_X), true), (b, Some(FEATURE_X), false)]),
        catalog(&[(a, Some(FEATURE_X), true), (b, Some(FEATURE_X), true)]),
        catalog(&[(a, Some(FEATURE_X), false), (b, Some(FEATURE_X), true)]),
        catalog(&[(a, Some(FEATURE_X), true), (b, Some(FEATURE_X), true)]),
    ] {
        reconcile_catalog(&mut state, &snapshot, false);
    }

    assert_eq!(
        count_of(&state, FEATURE_X),
        Some(2),
        "the last snapshot, with both sessions unread, decides the count"
    );
}

/// `feat-x` with two unread sessions, its view already reported, before and after the user
/// expands, hovers, unhovers and collapses its row and toggles the Default row (FR-009).
fn rows_toggled_and_hovered() -> (State, State) {
    let (mut state, a, b) = two_read_sessions_in_feature_x();
    reconcile_catalog(
        &mut state,
        &catalog(&[(a, Some(FEATURE_X), true), (b, Some(FEATURE_X), true)]),
        false,
    );
    let _ = state.view_report(true);
    let before = state.clone();

    for message in [
        Message::Sidebar(SidebarMsg::WorktreeExpansionToggled(FEATURE_X.to_string())),
        Message::Worktree(WorktreeMsg::Hovered(FEATURE_X.to_string())),
        Message::Worktree(WorktreeMsg::Unhovered(FEATURE_X.to_string())),
        Message::Sidebar(SidebarMsg::WorktreeExpansionToggled(FEATURE_X.to_string())),
        Message::Sidebar(SidebarMsg::DefaultExpansionToggled),
    ] {
        state.update(message);
    }
    (before, state)
}

fn unread_flags(state: &State) -> Vec<bool> {
    state.workspace.sessions[Path::new(REPO)]
        .iter()
        .map(|s| s.unread)
        .collect()
}

/// FR-009: expanding, collapsing and hovering a location row mark no session read.
#[test]
fn expanding_collapsing_and_hovering_a_row_mark_no_session_read() {
    let (before, after) = rows_toggled_and_hovered();

    assert_eq!(
        unread_flags(&after),
        unread_flags(&before),
        "no session's unread state changed"
    );
}

/// FR-009: expanding, collapsing and hovering a location row bring no session into view.
#[test]
fn expanding_collapsing_and_hovering_a_row_bring_nothing_into_view() {
    let (_, mut after) = rows_toggled_and_hovered();

    assert_eq!(
        after.view_report(true),
        None,
        "the window's view report is unchanged, so nothing came into view"
    );
}

/// FR-009: expanding, collapsing and hovering a location row leave its count as it was.
#[test]
fn expanding_collapsing_and_hovering_a_row_keep_its_count() {
    let (_, after) = rows_toggled_and_hovered();

    assert_eq!(
        count_of(&after, FEATURE_X),
        Some(2),
        "both sessions are still unread, so the row still counts two"
    );
}

fn switcher_count(state: &State) -> usize {
    let active = state.workspace.active.clone();
    state
        .switcher_entries()
        .iter()
        .find(|e| Some(&e.path) == active.as_ref())
        .map(|e| e.unread_count)
        .expect("the active project is on the switcher")
}

fn rows_total(state: &State) -> usize {
    counts(state).iter().map(|(_, n)| n).sum()
}

/// FR-010: the location rows add up to the switcher's count for the project, closed sessions
/// counted on neither.
#[test]
fn the_location_rows_add_up_to_the_switchers_count() {
    let mut closed = in_worktree(FEATURE_X, true);
    closed.archived = true;
    let state = state_with(vec![
        in_worktree(FEATURE_X, true),
        in_worktree(FEATURE_Y, true),
        in_worktree(FEATURE_Y, true),
        session_at(SessionLocation::Default, true),
        closed,
    ]);

    assert_eq!(
        switcher_count(&state),
        4,
        "the switcher counts the four unread open sessions and skips the closed one"
    );
    assert_eq!(
        rows_total(&state),
        switcher_count(&state),
        "with nothing hidden, the location rows add up to the switcher's count"
    );
}

/// FR-010, R8: an agent worktree the sidebar hides is still counted on the switcher, so the sum
/// holds only for a project with nothing hidden.
#[test]
fn an_agent_worktree_hidden_by_the_setting_still_counts_on_the_switcher() {
    let state = state_with(vec![in_worktree(FEATURE_X, true), in_worktree(AGENT, true)]);

    assert_eq!(
        switcher_count(&state),
        2,
        "the switcher counts the hidden agent worktree's unread session too"
    );
    assert_eq!(
        rows_total(&state),
        1,
        "the agent worktree is hidden, so only feat-x's row counts"
    );
}

/// FR-010, R8: a worktree the tag filter hides is still counted on the switcher.
#[test]
fn a_worktree_hidden_by_the_tag_filter_still_counts_on_the_switcher() {
    let mut state = state_with(vec![
        in_worktree(FEATURE_X, true),
        in_worktree(FEATURE_Y, true),
    ]);
    state
        .sidebar
        .filters
        .insert(TagFilter::Type(ConventionalType::Fix));

    assert_eq!(
        switcher_count(&state),
        2,
        "the switcher counts the unread sessions of worktrees the filter hides"
    );
    assert_eq!(
        rows_total(&state),
        0,
        "the tag filter hides both worktrees, so no row carries a count"
    );
}
