//! T017 — sidebar tree building + expand/collapse (FR-002/003).

use micold_client::app::{Message, SidebarEntry, State, TagFilter};
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
    state.update(Message::WorktreeExpansionToggled("feat-a".to_string()));
    let expanded = state
        .worktree_tree()
        .into_iter()
        .find(|n| n.worktree.dir_name == "feat-a")
        .unwrap()
        .expanded;
    assert!(expanded);

    state.update(Message::WorktreeExpansionToggled("feat-a".to_string()));
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
    state.update(Message::WorktreeExpansionToggled("feat-a".to_string()));
    // Reload without feat-a.
    state.update(Message::WorktreesLoaded(vec![worktree(
        "feat-b",
        WorktreeStatus::Valid,
    )]));
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
    state: &'a [micold_client::app::WorktreeNode],
    dir: &str,
) -> &'a micold_client::app::WorktreeNode {
    state.iter().find(|n| n.worktree.dir_name == dir).unwrap()
}

#[test]
fn worktree_node_exposes_type_and_issue_tags() {
    let state = state_with_named_worktrees(&[("feat-abc-123-login-page", WorktreeStatus::Valid)]);
    let tree = state.worktree_tree();
    let n = node(&tree, "feat-abc-123-login-page");
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
        ("feat-abc-123-login-page", WorktreeStatus::Valid),
        ("my-experiment", WorktreeStatus::Valid),
    ]);
    let tree = state.worktree_tree();
    assert_eq!(
        node(&tree, "feat-abc-123-login-page").display_name,
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
        micold_client::app::worktree_location_label(&root, &wt),
        ".claude/worktrees/feat-a"
    );
}

#[test]
fn default_location_label_is_a_fixed_project_root_string() {
    assert_eq!(micold_client::app::DEFAULT_LOCATION_LABEL, "Project root");
}

// --- Feature 008 US4: tag filtering ---

fn dirs(tree: &[micold_client::app::WorktreeNode]) -> Vec<String> {
    tree.iter().map(|n| n.worktree.dir_name.clone()).collect()
}

fn filtered_state() -> State {
    state_with_named_worktrees(&[
        ("feat-abc-123-login", WorktreeStatus::Valid),
        ("fix-crash", WorktreeStatus::Valid),
        ("fix-def-9-thing", WorktreeStatus::Valid),
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
    state.update(Message::SidebarFilterToggled(TagFilter::Type(
        ConventionalType::Fix,
    )));
    assert_eq!(
        dirs(&state.filtered_worktree_tree()),
        vec!["fix-crash", "fix-def-9-thing"]
    );
}

#[test]
fn filtered_tree_is_unaffected_by_the_filter_panels_open_state() {
    // Feature 009 FR-007/FR-008: showing/hiding the filter panel is purely a display change and
    // must never affect which worktrees are considered filtered.
    let mut state = filtered_state();
    state.update(Message::SidebarFilterToggled(TagFilter::Type(
        ConventionalType::Fix,
    )));
    let expected = dirs(&state.filtered_worktree_tree());

    state.update(Message::SidebarFilterMenuToggled); // open
    assert_eq!(dirs(&state.filtered_worktree_tree()), expected);
    state.update(Message::SidebarFilterMenuToggled); // close
    assert_eq!(dirs(&state.filtered_worktree_tree()), expected);
}

#[test]
fn filters_combine_with_or() {
    let mut state = filtered_state();
    state.update(Message::SidebarFilterToggled(TagFilter::Type(
        ConventionalType::Feat,
    )));
    state.update(Message::SidebarFilterToggled(TagFilter::Untyped));
    // feat + untyped ⇒ the feat worktree and the non-conforming one.
    assert_eq!(
        dirs(&state.filtered_worktree_tree()),
        vec!["feat-abc-123-login", "my-experiment"]
    );
}

#[test]
fn has_issue_filter_selects_issue_bearing() {
    let mut state = filtered_state();
    state.update(Message::SidebarFilterToggled(TagFilter::HasIssue));
    assert_eq!(
        dirs(&state.filtered_worktree_tree()),
        vec!["feat-abc-123-login", "fix-def-9-thing"]
    );
}

#[test]
fn untyped_filter_selects_non_conforming() {
    let mut state = filtered_state();
    state.update(Message::SidebarFilterToggled(TagFilter::Untyped));
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
    state.update(Message::SidebarFilterToggled(TagFilter::Type(
        ConventionalType::Fix,
    )));
    assert_eq!(state.filtered_worktree_tree().len(), 2);
    // Remove one fix worktree via the delete reducer path.
    state.update(Message::WorktreeDeleteRequested("fix-crash".to_string()));
    state.update(Message::WorktreeDeleteConfirmed);
    assert_eq!(
        dirs(&state.filtered_worktree_tree()),
        vec!["fix-def-9-thing"]
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
    state.update(Message::ShowAgentWorktreesToggled);
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
    state.update(Message::ShowAgentWorktreesToggled);
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
    state.update(Message::ShowAgentWorktreesToggled);

    state.update(Message::SidebarFilterToggled(TagFilter::Type(
        ConventionalType::Feat,
    )));
    assert_eq!(
        dirs(&state.filtered_worktree_tree()),
        vec!["feat-a", "feat-b"]
    );

    state.update(Message::SidebarFilterToggled(TagFilter::Type(
        ConventionalType::Feat,
    ))); // clear it
    state.update(Message::SidebarFilterToggled(TagFilter::Untyped));
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
    state.update(Message::SidebarFilterToggled(TagFilter::Type(
        ConventionalType::Fix,
    )));
    // Renaming changes only the display name; tags (and thus the filter result) are unchanged.
    state.update(Message::WorktreeRenameStarted("fix-crash".to_string()));
    state.update(Message::WorktreeRenameTextChanged("Hotfix".to_string()));
    state.update(Message::WorktreeRenameConfirmed);
    assert_eq!(state.filtered_worktree_tree().len(), 2);
    let renamed = node(&state.filtered_worktree_tree(), "fix-crash")
        .display_name
        .clone();
    assert_eq!(renamed, "Hotfix");
}
