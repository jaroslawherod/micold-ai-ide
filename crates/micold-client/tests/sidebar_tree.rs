//! T017 — sidebar tree building + expand/collapse (FR-002/003).

use micold_client::app::{Message, State};
use micold_client::features::sidebar::Msg as SidebarMsg;
use micold_client::features::sidebar::{SidebarEntry, TagFilter};
use micold_client::features::worktree::Msg as WorktreeMsg;
use micold_core::naming::{ConventionalType, Tag};
use micold_core::project::{Availability, Project};
use micold_core::session::{Session, SessionLocation};
use micold_core::worktree::{Worktree, WorktreeStatus};
use std::path::PathBuf;

fn worktree(dir: &str, status: WorktreeStatus) -> Worktree {
    Worktree {
        dir_name: dir.to_string(),
        path: PathBuf::from(format!("/repo/.claude/worktrees/{dir}")),
        branch: Some(format!("feat/{dir}")),
        status,
        included: false,
    }
}

fn state_with_active_project() -> State {
    let mut state = State::default();
    let path = PathBuf::from("/repo");
    state.workspace.projects.push(Project {
        path: path.clone(),
        display_name: "repo".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    state.workspace.active = Some(path.clone());
    state.worktrees = vec![
        worktree("feat-a", WorktreeStatus::Valid),
        worktree("feat-b", WorktreeStatus::Valid),
    ];
    // A session on feat-a.
    state.workspace.sessions.insert(
        path,
        vec![Session::start_new(SessionLocation::Worktree(
            "feat-a".to_string(),
        ))],
    );
    state
}

/// Change the current session the way the root does (T067a-6): the commit of the outgoing row is
/// an outcome now, so dropping it would assert against half a move.
fn set_current(state: &mut State, next: Option<micold_core::session::SessionId>) {
    let outcomes = state.set_current_session(next);
    micold_client::app::drain(outcomes, |o| micold_client::app::interpret(state, o));
}

#[test]
fn tree_has_a_node_per_worktree_collapsed_by_default() {
    let state = state_with_active_project();
    let tree = state.worktree_tree();
    assert_eq!(tree.len(), 2);
    assert!(tree.iter().all(|n| !n.expanded));
    assert_eq!(tree[0].worktree.dir_name, "feat-a");
}

#[test]
fn sessions_are_joined_to_their_worktree_by_dir_name() {
    let state = state_with_active_project();
    let tree = state.worktree_tree();
    let feat_a = tree
        .iter()
        .find(|n| n.worktree.dir_name == "feat-a")
        .unwrap();
    let feat_b = tree
        .iter()
        .find(|n| n.worktree.dir_name == "feat-b")
        .unwrap();
    assert_eq!(feat_a.sessions.len(), 1);
    assert_eq!(feat_b.sessions.len(), 0);
}

#[test]
fn toggling_expands_then_collapses() {
    let mut state = state_with_active_project();
    state.update(Message::Sidebar(SidebarMsg::WorktreeExpansionToggled(
        "feat-a".to_string(),
    )));
    let expanded = state
        .worktree_tree()
        .into_iter()
        .find(|n| n.worktree.dir_name == "feat-a")
        .unwrap()
        .expanded;
    assert!(expanded);

    state.update(Message::Sidebar(SidebarMsg::WorktreeExpansionToggled(
        "feat-a".to_string(),
    )));
    let collapsed = !state
        .worktree_tree()
        .into_iter()
        .find(|n| n.worktree.dir_name == "feat-a")
        .unwrap()
        .expanded;
    assert!(collapsed);
}

#[test]
fn reloading_worktrees_drops_stale_expansion_state() {
    let mut state = state_with_active_project();
    state.update(Message::Sidebar(SidebarMsg::WorktreeExpansionToggled(
        "feat-a".to_string(),
    )));
    // Reload without feat-a.
    state.update(Message::Worktree(WorktreeMsg::Loaded(vec![worktree(
        "feat-b",
        WorktreeStatus::Valid,
    )])));
    assert!(!state.expanded.contains("feat-a"));
}

// --- Feature 008 US1: display name + tags per worktree ---

fn state_with_named_worktrees(dirs: &[(&str, WorktreeStatus)]) -> State {
    let mut state = State::default();
    let path = PathBuf::from("/repo");
    state.workspace.projects.push(Project {
        path: path.clone(),
        display_name: "repo".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    state.workspace.active = Some(path);
    state.worktrees = dirs.iter().map(|(d, s)| worktree(d, *s)).collect();
    state
}

fn node<'a>(
    state: &'a [micold_client::features::sidebar::WorktreeNode],
    dir: &str,
) -> &'a micold_client::features::sidebar::WorktreeNode {
    state.iter().find(|n| n.worktree.dir_name == dir).unwrap()
}

#[test]
fn worktree_node_exposes_type_and_issue_tags() {
    let state = state_with_named_worktrees(&[("feat-abc-123_login-page", WorktreeStatus::Valid)]);
    let tree = state.worktree_tree();
    let n = node(&tree, "feat-abc-123_login-page");
    assert_eq!(
        n.tags,
        vec![
            Tag::Type(ConventionalType::Feat),
            Tag::Issue("ABC-123".to_string())
        ]
    );
}

#[test]
fn non_valid_worktree_gets_status_tag() {
    // US5 / FR-011: missing/invalid worktrees carry a status tag (the cue that replaces the
    // removed git icon); valid ones do not.
    let state = state_with_named_worktrees(&[
        ("feat-ok", WorktreeStatus::Valid),
        ("feat-gone", WorktreeStatus::Missing),
        ("orphan-dir", WorktreeStatus::Invalid),
    ]);
    let tree = state.worktree_tree();
    assert!(node(&tree, "feat-ok")
        .tags
        .iter()
        .all(|t| !matches!(t, Tag::Status(_))));
    assert!(node(&tree, "feat-gone")
        .tags
        .contains(&Tag::Status(WorktreeStatus::Missing)));
    assert!(node(&tree, "orphan-dir")
        .tags
        .contains(&Tag::Status(WorktreeStatus::Invalid)));
}

#[test]
fn worktree_node_type_only_and_untyped() {
    let state = state_with_named_worktrees(&[
        ("fix-crash-on-open", WorktreeStatus::Valid),
        ("my-experiment", WorktreeStatus::Valid),
    ]);
    let tree = state.worktree_tree();
    assert_eq!(
        node(&tree, "fix-crash-on-open").tags,
        vec![Tag::Type(ConventionalType::Fix)]
    );
    assert!(node(&tree, "my-experiment").tags.is_empty());
}

#[test]
fn worktree_node_display_name_derived_when_no_override() {
    let state = state_with_named_worktrees(&[
        ("feat-abc-123_login-page", WorktreeStatus::Valid),
        ("my-experiment", WorktreeStatus::Valid),
    ]);
    let tree = state.worktree_tree();
    assert_eq!(
        node(&tree, "feat-abc-123_login-page").display_name,
        "Login page"
    );
    assert_eq!(node(&tree, "my-experiment").display_name, "My experiment");
}

// --- Feature 010 US2: location tooltip text (FR-010) ---

// T019: the worktree location label is the worktree's path relative to the project root
// (research.md R6 — Path::strip_prefix, since every worktree always lives directly under
// `<project_root>/.claude/worktrees/`).
#[test]
fn worktree_location_label_is_relative_to_project_root() {
    let root = PathBuf::from("/repo");
    let wt = worktree("feat-a", WorktreeStatus::Valid);
    assert_eq!(
        micold_client::features::sidebar::worktree_location_label(&root, &wt),
        ".claude/worktrees/feat-a"
    );
}

#[test]
fn default_location_label_is_a_fixed_project_root_string() {
    assert_eq!(
        micold_client::features::sidebar::DEFAULT_LOCATION_LABEL,
        "Project root"
    );
}

// --- Feature 008 US4: tag filtering ---

fn dirs(tree: &[micold_client::features::sidebar::WorktreeNode]) -> Vec<String> {
    tree.iter().map(|n| n.worktree.dir_name.clone()).collect()
}

fn filtered_state() -> State {
    state_with_named_worktrees(&[
        ("feat-abc-123_login", WorktreeStatus::Valid),
        ("fix-crash", WorktreeStatus::Valid),
        ("fix-def-9_thing", WorktreeStatus::Valid),
        ("my-experiment", WorktreeStatus::Valid),
    ])
}

#[test]
fn empty_filter_shows_all() {
    let state = filtered_state();
    assert_eq!(state.filtered_worktree_tree().len(), 4);
}

#[test]
fn type_filter_selects_only_that_type() {
    let mut state = filtered_state();
    state.update(Message::Sidebar(SidebarMsg::FilterToggled(
        TagFilter::Type(ConventionalType::Fix),
    )));
    assert_eq!(
        dirs(&state.filtered_worktree_tree()),
        vec!["fix-crash", "fix-def-9_thing"]
    );
}

#[test]
fn filtered_tree_is_unaffected_by_the_filter_panels_open_state() {
    // Feature 009 FR-007/FR-008: showing/hiding the filter panel is purely a display change and
    // must never affect which worktrees are considered filtered.
    let mut state = filtered_state();
    state.update(Message::Sidebar(SidebarMsg::FilterToggled(
        TagFilter::Type(ConventionalType::Fix),
    )));
    let expected = dirs(&state.filtered_worktree_tree());

    state.update(Message::Sidebar(SidebarMsg::FilterMenuToggled)); // open
    assert_eq!(dirs(&state.filtered_worktree_tree()), expected);
    state.update(Message::Sidebar(SidebarMsg::FilterMenuToggled)); // close
    assert_eq!(dirs(&state.filtered_worktree_tree()), expected);
}

#[test]
fn filters_combine_with_or() {
    let mut state = filtered_state();
    state.update(Message::Sidebar(SidebarMsg::FilterToggled(
        TagFilter::Type(ConventionalType::Feat),
    )));
    state.update(Message::Sidebar(SidebarMsg::FilterToggled(
        TagFilter::Untyped,
    )));
    // feat + untyped ⇒ the feat worktree and the non-conforming one.
    assert_eq!(
        dirs(&state.filtered_worktree_tree()),
        vec!["feat-abc-123_login", "my-experiment"]
    );
}

#[test]
fn has_issue_filter_selects_issue_bearing() {
    let mut state = filtered_state();
    state.update(Message::Sidebar(SidebarMsg::FilterToggled(
        TagFilter::HasIssue,
    )));
    assert_eq!(
        dirs(&state.filtered_worktree_tree()),
        vec!["feat-abc-123_login", "fix-def-9_thing"]
    );
}

#[test]
fn untyped_filter_selects_non_conforming() {
    let mut state = filtered_state();
    state.update(Message::Sidebar(SidebarMsg::FilterToggled(
        TagFilter::Untyped,
    )));
    assert_eq!(dirs(&state.filtered_worktree_tree()), vec!["my-experiment"]);
}

#[test]
fn available_filters_reflect_present_tags() {
    let filters = filtered_state().available_tag_filters();
    assert!(filters.contains(&TagFilter::Type(ConventionalType::Feat)));
    assert!(filters.contains(&TagFilter::Type(ConventionalType::Fix)));
    assert!(filters.contains(&TagFilter::HasIssue));
    assert!(filters.contains(&TagFilter::Untyped));
    // No `chore` worktree ⇒ no chore filter offered.
    assert!(!filters.contains(&TagFilter::Type(ConventionalType::Chore)));
}

#[test]
fn filter_recomputes_after_delete(/* FR-028 / C1 */) {
    let mut state = filtered_state();
    state.update(Message::Sidebar(SidebarMsg::FilterToggled(
        TagFilter::Type(ConventionalType::Fix),
    )));
    assert_eq!(state.filtered_worktree_tree().len(), 2);
    // Delete one fix worktree. Confirming only dismisses the dialog — the daemon performs the
    // removal and pushes git's refreshed truth, which the client adopts via `set_worktrees`.
    state.update(Message::Worktree(WorktreeMsg::DeleteRequested(
        "fix-crash".to_string(),
    )));
    state.update(Message::Worktree(WorktreeMsg::DeleteConfirmed));
    let surviving: Vec<_> = state
        .worktrees
        .iter()
        .filter(|w| w.dir_name != "fix-crash")
        .cloned()
        .collect();
    micold_client::app::drain(state.set_worktrees(surviving), |o| {
        micold_client::app::interpret(&mut state, o)
    });
    assert_eq!(
        dirs(&state.filtered_worktree_tree()),
        vec!["fix-def-9_thing"]
    );
}

// --- Feature 010 US1/US2: the "Default" (project-root) sidebar entry ---

// T014: the Default entry is always present for an open project, ahead of any worktree
// entries, and absent when no project is open (contracts/sidebar-default-entry.md
// invariant 1, previously untested).

#[test]
fn default_entry_present_and_first_even_with_no_worktrees() {
    let mut state = State::default();
    let path = PathBuf::from("/repo");
    state.workspace.projects.push(Project {
        path: path.clone(),
        display_name: "repo".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    state.workspace.active = Some(path);
    // No worktrees at all.
    state.worktrees = vec![];

    let entries = state.sidebar_entries();
    assert_eq!(entries.len(), 1, "Default entry alone, no worktrees yet");
    assert!(matches!(entries[0], SidebarEntry::Default(_)));
}

#[test]
fn default_entry_precedes_worktree_entries() {
    let state = state_with_active_project();
    let entries = state.sidebar_entries();
    assert!(matches!(entries[0], SidebarEntry::Default(_)));
    assert_eq!(entries.len(), 1 + 2, "Default + the 2 worktrees");
    assert!(entries[1..]
        .iter()
        .all(|e| matches!(e, SidebarEntry::Worktree(_))));
}

#[test]
fn no_default_entry_when_no_project_is_open() {
    let state = State::default();
    assert!(state.workspace.active.is_none());
    assert!(
        state.sidebar_entries().is_empty(),
        "no project open must yield no sidebar entries at all, not a stray Default"
    );
}

#[test]
fn default_sessions_are_attached_to_the_default_entry_only() {
    let mut state = state_with_active_project(); // has one Worktree("feat-a") session
    let path = state.workspace.active.clone().unwrap();
    state
        .workspace
        .sessions
        .get_mut(&path)
        .unwrap()
        .push(Session::start_new(SessionLocation::Default));

    let entries = state.sidebar_entries();
    let SidebarEntry::Default(default_node) = &entries[0] else {
        panic!("expected the Default entry first");
    };
    assert_eq!(default_node.sessions.len(), 1);
    assert!(default_node
        .sessions
        .iter()
        .all(|s| s.location == SessionLocation::Default));
}

// --- Feature 014 US1: agent-owned worktrees are hidden ---

/// An agent-owned worktree as Claude Code creates it: `agent-<hex>` bound to
/// `worktree-agent-<hex>` (feature 014, FR-005).
fn agent_worktree(hex: &str, status: WorktreeStatus) -> Worktree {
    let dir = format!("agent-{hex}");
    Worktree {
        dir_name: dir.clone(),
        path: PathBuf::from(format!("/repo/.claude/worktrees/{dir}")),
        branch: Some(format!("worktree-agent-{hex}")),
        status,
        included: false,
    }
}

fn state_with(worktrees: Vec<Worktree>) -> State {
    let mut state = State::default();
    let path = PathBuf::from("/repo");
    state.workspace.projects.push(Project {
        path: path.clone(),
        display_name: "repo".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    state.workspace.active = Some(path);
    state.worktrees = worktrees;
    state
}

const AGENT_HEXES: [&str; 3] = [
    "a885b42dc521fbda1",
    "abf6a58b16c3c9e6f",
    "ae474105b29fbeb68",
];

fn mixed_state() -> State {
    state_with(vec![
        worktree("feat-a", WorktreeStatus::Valid),
        worktree("feat-b", WorktreeStatus::Valid),
        worktree("fix-c", WorktreeStatus::Valid),
        agent_worktree(AGENT_HEXES[0], WorktreeStatus::Valid),
        agent_worktree(AGENT_HEXES[1], WorktreeStatus::Valid),
        agent_worktree(AGENT_HEXES[2], WorktreeStatus::Valid),
    ])
}

#[test]
fn tree_lists_only_user_worktrees_by_default() {
    // US1 acceptance #1: 3 user + 3 agent worktrees ⇒ exactly the 3 user rows.
    let tree = mixed_state().worktree_tree();
    assert_eq!(dirs(&tree), vec!["feat-a", "feat-b", "fix-c"]);
}

#[test]
fn agent_only_project_yields_no_worktree_nodes() {
    // US1 acceptance #2: the sidebar must fall through to its empty state, not list machine names.
    let state = state_with(
        AGENT_HEXES
            .iter()
            .map(|h| agent_worktree(h, WorktreeStatus::Valid))
            .collect(),
    );
    assert!(state.worktree_tree().is_empty());
    // Only the Default entry survives — no worktree entries at all.
    let entries = state.sidebar_entries();
    assert_eq!(entries.len(), 1);
    assert!(matches!(entries[0], SidebarEntry::Default(_)));
}

#[test]
fn hidden_worktrees_are_not_reachable_as_action_targets() {
    // FR-004: rows are the only entry point to start-session, rename, and delete, so proving no
    // agent worktree reaches `sidebar_entries()` is what pins that requirement.
    let state = mixed_state();
    let listed: Vec<String> = state
        .sidebar_entries()
        .iter()
        .filter_map(|e| match e {
            SidebarEntry::Worktree(n) => Some(n.worktree.dir_name.clone()),
            SidebarEntry::Default(_) => None,
        })
        .collect();
    assert!(
        listed.iter().all(|d| !d.starts_with("agent-")),
        "no agent worktree may be offered as an action target while hidden, got {listed:?}"
    );
}

// --- Feature 014 US4: revealing agent worktrees ---

#[test]
fn revealing_adds_agent_rows_in_unchanged_order() {
    // US4 acceptance #1: the user's own rows are unaffected and unmoved; the agent rows join them.
    let mut state = mixed_state();
    state.update(Message::Sidebar(SidebarMsg::ShowAgentWorktreesToggled));
    let listed = dirs(&state.worktree_tree());
    assert_eq!(listed.len(), 6);
    // Revealing must not reorder: the tree preserves `State::worktrees` order (which `reconcile()`
    // has already sorted by dir_name in production), so with the toggle on it equals that list
    // exactly.
    let all: Vec<String> = state.worktrees.iter().map(|w| w.dir_name.clone()).collect();
    assert_eq!(listed, all);
    for hex in AGENT_HEXES {
        assert!(listed.contains(&format!("agent-{hex}")));
    }
}

#[test]
fn revealed_rows_carry_the_agent_badge() {
    // FR-010b: every revealed row is badged, unconditionally — not depending on its health, name,
    // or session count — so it can never be mistaken for the user's own work.
    let mut state = mixed_state();
    state.update(Message::Sidebar(SidebarMsg::ShowAgentWorktreesToggled));
    let tree = state.worktree_tree();
    for hex in AGENT_HEXES {
        let n = node(&tree, &format!("agent-{hex}"));
        assert!(
            n.tags.contains(&Tag::Agent),
            "revealed agent row must carry Tag::Agent, got {:?}",
            n.tags
        );
    }
    // The user's own worktrees are never badged.
    assert!(!node(&tree, "feat-a").tags.contains(&Tag::Agent));
}

#[test]
fn tag_filters_apply_to_revealed_rows_the_same_way() {
    // FR-010d: revealed entries flow through the same `matches_filters()` call as everyone else.
    // An agent worktree carries no conventional type, so `Untyped` matches it and `feat` does not.
    let mut state = mixed_state();
    state.update(Message::Sidebar(SidebarMsg::ShowAgentWorktreesToggled));

    state.update(Message::Sidebar(SidebarMsg::FilterToggled(
        TagFilter::Type(ConventionalType::Feat),
    )));
    assert_eq!(
        dirs(&state.filtered_worktree_tree()),
        vec!["feat-a", "feat-b"]
    );

    state.update(Message::Sidebar(SidebarMsg::FilterToggled(
        TagFilter::Type(ConventionalType::Feat),
    ))); // clear it
    state.update(Message::Sidebar(SidebarMsg::FilterToggled(
        TagFilter::Untyped,
    )));
    let untyped = dirs(&state.filtered_worktree_tree());
    for hex in AGENT_HEXES {
        assert!(untyped.contains(&format!("agent-{hex}")));
    }
}

// --- Feature 014 US3: unhealthy agent worktrees are hidden too ---

#[test]
fn unhealthy_agent_worktrees_are_hidden_rather_than_shown_as_broken() {
    // FR-007 / US3 acceptance #3: an orphan directory git no longer registers (Invalid) and a
    // registration whose directory is gone (Missing) are still agent worktrees. Surfacing them as
    // broken entries would be worse than the original problem — a scary row for something the
    // user never created.
    let state = state_with(vec![
        worktree("feat-a", WorktreeStatus::Valid),
        agent_worktree(AGENT_HEXES[0], WorktreeStatus::Missing),
        agent_worktree(AGENT_HEXES[1], WorktreeStatus::Invalid),
    ]);
    assert_eq!(dirs(&state.worktree_tree()), vec!["feat-a"]);
    // A user's own broken worktree still surfaces — hiding is about ownership, not health.
    let user_broken = state_with(vec![worktree("feat-gone", WorktreeStatus::Missing)]);
    assert_eq!(dirs(&user_broken.worktree_tree()), vec!["feat-gone"]);
}

#[test]
fn a_session_in_a_hidden_worktree_renders_nowhere_but_is_not_pruned() {
    // FR-011 / research R8: no dedicated handling. The session is joined to its worktree in
    // `worktree_tree()`, so hiding the worktree hides the session with it — exactly what already
    // happens when a worktree is deleted outside the app. The record itself survives untouched.
    let mut state = state_with(vec![
        worktree("feat-a", WorktreeStatus::Valid),
        agent_worktree(AGENT_HEXES[0], WorktreeStatus::Valid),
    ]);
    let agent_dir = format!("agent-{}", AGENT_HEXES[0]);
    let path = state.workspace.active.clone().unwrap();
    state.workspace.sessions.insert(
        path,
        vec![Session::start_new(SessionLocation::Worktree(
            agent_dir.clone(),
        ))],
    );

    // Rendered nowhere: no row carries it.
    assert_eq!(dirs(&state.worktree_tree()), vec!["feat-a"]);
    assert!(state.worktree_tree().iter().all(|n| n.sessions.is_empty()));

    // Not pruned, and still resolvable by dir_name — visibility is irrelevant to which sessions
    // must be terminated before a worktree is removed.
    assert_eq!(state.active_sessions().len(), 1);
    assert_eq!(state.sessions_in_worktree(&agent_dir).len(), 1);
}

// --- Feature 014 US2: user worktrees are never hidden by mistake ---

#[test]
fn user_worktrees_sharing_the_reserved_prefix_stay_listed() {
    // SC-002 / FR-006: a naming corpus that deliberately brushes up against the reserved
    // convention. Every one of these is the user's own work and must survive.
    let corpus = [
        "agent-foo",                        // ordinary word after the prefix
        "agent-face",                       // hex, but far too short
        "agent-deadbeefdeadbeef-parser",    // long enough, tail is not hex
        "agent-deadbeefdeadbee",            // 15 hex digits — one below the bound
        "feat-1234-agent-runner",           // reserved word in the middle
        "worktree-agent-a885b42dc521fbda1", // branch prefix in the directory position
    ];
    let state = state_with(
        corpus
            .iter()
            .map(|d| worktree(d, WorktreeStatus::Valid))
            .collect(),
    );
    let listed = dirs(&state.worktree_tree());
    for name in corpus {
        assert!(
            listed.iter().any(|d| d == name),
            "{name} is a user worktree and must stay visible, got {listed:?}"
        );
    }
}

#[test]
fn hiding_does_not_disturb_the_underlying_worktree_list() {
    // FR-008: hiding is presentation-only — `State::worktrees` still holds everything discovered.
    let state = mixed_state();
    assert_eq!(state.worktrees.len(), 6);
}

#[test]
fn filter_recomputes_after_rename(/* FR-028 / C1 */) {
    let mut state = filtered_state();
    state.update(Message::Sidebar(SidebarMsg::FilterToggled(
        TagFilter::Type(ConventionalType::Fix),
    )));
    // Renaming changes only the display name; tags (and thus the filter result) are unchanged.
    state.update(Message::Worktree(WorktreeMsg::RenameStarted(
        "fix-crash".to_string(),
    )));
    state.update(Message::Worktree(WorktreeMsg::RenameTextChanged(
        "Hotfix".to_string(),
    )));
    state.update(Message::Worktree(WorktreeMsg::RenameConfirmed));
    assert_eq!(state.filtered_worktree_tree().len(), 2);
    let renamed = node(&state.filtered_worktree_tree(), "fix-crash")
        .display_name
        .clone();
    assert_eq!(renamed, "Hotfix");
}

// --- Feature 024: the row holding the current session ----------------------------------------
//
// Contract §1's clauses, asked of `State` rather than of the predicate: which location holds the
// current session, and what happens when the answer is unavailable.

/// The state above, with its one session made current.
fn state_with_current_session() -> State {
    let mut state = state_with_active_project();
    state.active_session = Some(state.active_sessions()[0].id);
    state
}

#[test]
fn the_current_sessions_location_is_open_though_nobody_expanded_it() {
    let state = state_with_current_session();

    let feat_a = state
        .worktree_tree()
        .into_iter()
        .find(|n| n.worktree.dir_name == "feat-a")
        .unwrap();
    assert!(
        feat_a.expanded,
        "the location holding the current session is listed open — the reported bug is that it \
         was not (FR-001)"
    );
    assert!(
        state.expanded.is_empty(),
        "and it is open without anything being written to the user's own expansion set: \
         open-ness is derived, so a worktree-list replacement has nothing to lose (FR-001b)"
    );
}

#[test]
fn no_other_location_is_opened_on_the_users_behalf() {
    let state = state_with_current_session();

    let feat_b = state
        .worktree_tree()
        .into_iter()
        .find(|n| n.worktree.dir_name == "feat-b")
        .unwrap();
    assert!(
        !feat_b.expanded,
        "exactly one location is forced open, because there is at most one current session \
         (FR-004, invariant I1)"
    );
}

#[test]
fn replacing_the_worktree_list_does_not_close_the_current_sessions_row() {
    let mut state = state_with_current_session();

    micold_client::app::drain(
        state.set_worktrees(vec![
            worktree("feat-a", WorktreeStatus::Valid),
            worktree("feat-c", WorktreeStatus::Valid),
        ]),
        |o| micold_client::app::interpret(&mut state, o),
    );

    let feat_a = state
        .worktree_tree()
        .into_iter()
        .find(|n| n.worktree.dir_name == "feat-a")
        .unwrap();
    assert!(
        feat_a.expanded,
        "creating, deleting or re-discovering a worktree replaces the whole list; the row holding \
         the current session survives it (SC-008)"
    );
}

#[test]
fn a_current_session_whose_worktree_is_gone_opens_nothing() {
    let mut state = state_with_current_session();

    micold_client::app::drain(
        state.set_worktrees(vec![worktree("feat-b", WorktreeStatus::Valid)]),
        |o| micold_client::app::interpret(&mut state, o),
    );

    assert!(
        state.worktree_tree().into_iter().all(|n| !n.expanded),
        "the location that held the current session no longer exists, so there is nothing to \
         open — and no unrelated row may be opened in its place (FR-013)"
    );
}

#[test]
fn a_current_session_in_the_project_root_opens_the_default_row() {
    let mut state = state_with_active_project();
    let path = state.workspace.active.clone().unwrap();
    let default_session = Session::start_new(SessionLocation::Default);
    let id = default_session.id;
    state
        .workspace
        .sessions
        .get_mut(&path)
        .unwrap()
        .push(default_session);
    state.active_session = Some(id);

    let default_open = state
        .sidebar_entries()
        .into_iter()
        .any(|entry| match entry {
            SidebarEntry::Default(node) => node.expanded,
            SidebarEntry::Worktree(_) => false,
        });
    assert!(
        default_open,
        "FR-001 is not a worktree-only promise — the project root holds sessions too \
         (constitution Principle III's Default exception)"
    );
}

// --- Feature 024: the one location that escapes the filters -----------------------------------
//
// US4. Filters exist so the user sees less; this exists so the panel never stops answering "where
// am I". Contract §5 is the balance between those two, and its whole weight is on "one".

/// A project whose worktrees are `feat-a` (typed `feat`), `fix-b` (typed `fix`), and a hidden
/// agent one, with the current session placed in `dir`.
fn state_with_filterable_worktrees(dir: &str) -> State {
    let mut state = State::default();
    let path = PathBuf::from("/repo");
    state.workspace.projects.push(Project {
        path: path.clone(),
        display_name: "repo".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    state.workspace.active = Some(path.clone());
    state.worktrees = vec![
        Worktree {
            dir_name: "feat-a".to_string(),
            path: PathBuf::from("/repo/.claude/worktrees/feat-a"),
            branch: Some("feat/a".to_string()),
            status: WorktreeStatus::Valid,
            included: false,
        },
        Worktree {
            dir_name: "fix-b".to_string(),
            path: PathBuf::from("/repo/.claude/worktrees/fix-b"),
            branch: Some("fix/b".to_string()),
            status: WorktreeStatus::Valid,
            included: false,
        },
        // A real agent id: 16+ hex characters, which is what classifies a worktree as
        // agent-owned and so hidden by default (feature 014).
        agent_worktree("00112233445566aa", WorktreeStatus::Valid),
    ];
    let session = Session::start_new(SessionLocation::Worktree(dir.to_string()));
    let id = session.id;
    state.workspace.sessions.insert(path, vec![session]);
    state.active_session = Some(id);
    state
}

fn listed(state: &State) -> Vec<String> {
    state
        .filtered_worktree_tree()
        .into_iter()
        .map(|n| n.worktree.dir_name)
        .collect()
}

#[test]
fn a_filter_that_would_hide_the_current_session_does_not_hide_it() {
    let mut state = state_with_filterable_worktrees("fix-b");
    state
        .sidebar_filters
        .insert(TagFilter::Type(ConventionalType::Feat));

    assert_eq!(
        listed(&state),
        vec!["feat-a".to_string(), "fix-b".to_string()],
        "the filter admits feat-a; fix-b is there only because it holds the current session. \
         Without this the panel goes quiet in exactly the situation the filter was set up for \
         (FR-011, SC-005)"
    );
}

#[test]
fn the_exempt_row_sits_where_it_would_sit_unfiltered() {
    let mut state = state_with_filterable_worktrees("feat-a");
    state
        .sidebar_filters
        .insert(TagFilter::Type(ConventionalType::Fix));

    assert_eq!(
        listed(&state),
        vec!["feat-a".to_string(), "fix-b".to_string()],
        "the exemption changes membership, never order — a row pinned to the top would be a \
         second thing to explain, and would move as the current session moved (FR-012a)"
    );
}

#[test]
fn only_the_current_sessions_location_escapes_the_filter() {
    let mut state = state_with_filterable_worktrees("fix-b");
    state.show_agent_worktrees = true;
    state
        .sidebar_filters
        .insert(TagFilter::Type(ConventionalType::Feat));

    let listed = listed(&state);
    assert!(
        !listed.contains(&"agent-00112233445566aa".to_string()),
        "one exemption, not a filter bypass — every other excluded location stays hidden (FR-012)"
    );
}

#[test]
fn a_hidden_agent_worktree_holding_the_current_session_is_shown() {
    let state = state_with_filterable_worktrees("agent-00112233445566aa");

    assert!(
        listed(&state).contains(&"agent-00112233445566aa".to_string()),
        "the hidden-agent setting excludes rows earlier than the tag filters do — in \
         `visible_worktrees`, before the tree is built — so the exemption has to resolve against \
         all worktrees rather than the visible ones (contract §5.1, US4 scenario 3)"
    );
    assert_eq!(
        listed(&state).len(),
        3,
        "the two user worktrees plus the exempt agent one, and nothing else"
    );
}

#[test]
fn the_exempt_row_says_why_it_is_there_and_others_do_not() {
    let mut state = state_with_filterable_worktrees("fix-b");
    state
        .sidebar_filters
        .insert(TagFilter::Type(ConventionalType::Feat));

    let tree = state.filtered_worktree_tree();
    let exempt = tree
        .iter()
        .find(|n| n.worktree.dir_name == "fix-b")
        .unwrap();
    let admitted = tree
        .iter()
        .find(|n| n.worktree.dir_name == "feat-a")
        .unwrap();

    assert!(
        exempt.shown_for_current_session,
        "a row that survived a filter it does not match is otherwise unexplained — the user set \
         that filter and is owed a reason (FR-012a)"
    );
    assert!(
        !admitted.shown_for_current_session,
        "and a row the filter admits on its own claims no exemption it did not need"
    );
}

#[test]
fn a_row_the_filters_allow_is_not_marked_as_exempt_merely_for_being_current() {
    let mut state = state_with_filterable_worktrees("feat-a");
    state
        .sidebar_filters
        .insert(TagFilter::Type(ConventionalType::Feat));

    let tree = state.filtered_worktree_tree();
    let current = tree
        .iter()
        .find(|n| n.worktree.dir_name == "feat-a")
        .unwrap();

    assert!(
        !current.shown_for_current_session,
        "holding the current session is not itself the reason this row is listed — the filter \
         admits it. Saying otherwise would put a chip on a row that needs no explanation"
    );
}

#[test]
fn the_exemption_ends_when_the_location_stops_holding_the_current_session() {
    let mut state = state_with_filterable_worktrees("fix-b");
    state
        .sidebar_filters
        .insert(TagFilter::Type(ConventionalType::Feat));
    assert!(listed(&state).contains(&"fix-b".to_string()));

    let moved = Session::start_new(SessionLocation::Worktree("feat-a".to_string()));
    let moved_id = moved.id;
    let path = state.workspace.active.clone().unwrap();
    state.workspace.sessions.get_mut(&path).unwrap().push(moved);
    set_current(&mut state, Some(moved_id));

    assert!(
        !listed(&state).contains(&"fix-b".to_string()),
        "the row returns to being hidden, because the filter still excludes it and the reason it \
         was exempt has gone (FR-012, US4 scenario 4)"
    );
    assert!(
        state.expanded.contains("fix-b"),
        "its *open* state survives the commit, though — only its presence goes (contract §5.3)"
    );
}

#[test]
fn an_exempt_row_conjures_no_filter_chip() {
    let state = state_with_filterable_worktrees("agent-00112233445566aa");

    assert!(
        !state.available_tag_filters().contains(&TagFilter::Untyped),
        "an agent worktree's machine name has no conventional type, so listing it as exempt must \
         not offer an `Untyped` chip matching nothing else the user can see — the same rule a \
         hidden agent worktree already obeys (contract §5.6, feature 014 R7)"
    );
}

#[test]
fn exactly_one_session_row_carries_the_mark_when_a_location_holds_several() {
    let mut state = state_with_active_project();
    let path = state.workspace.active.clone().unwrap();
    let sibling = Session::start_new(SessionLocation::Worktree("feat-a".to_string()));
    let sibling_id = sibling.id;
    state
        .workspace
        .sessions
        .get_mut(&path)
        .unwrap()
        .push(sibling);
    let current = state.active_sessions()[0].id;
    state.active_session = Some(current);

    let node = state
        .worktree_tree()
        .into_iter()
        .find(|n| n.worktree.dir_name == "feat-a")
        .unwrap();
    assert_eq!(node.sessions.len(), 2, "two sessions share the location");

    let marked: Vec<_> = node
        .sessions
        .iter()
        .filter(|s| state.active_session == Some(s.id))
        .map(|s| s.id)
        .collect();
    assert_eq!(
        marked,
        vec![current],
        "exactly one row carries the mark, and it is the current session's — not its sibling's \
         (FR-002)"
    );
    assert_ne!(current, sibling_id);
}

#[test]
fn nothing_is_marked_when_no_session_is_current() {
    let mut state = state_with_active_project();
    set_current(&mut state, None);

    let node = state
        .worktree_tree()
        .into_iter()
        .find(|n| n.worktree.dir_name == "feat-a")
        .unwrap();
    assert!(
        node.sessions
            .iter()
            .all(|s| state.active_session != Some(s.id)),
        "and none carries it when there is no current session — the panel must not claim \
         otherwise (FR-002, FR-013)"
    );
}
