//! The covered states, declared in exactly one place (feature 019, T019/T020, FR-016).
//!
//! Adding a screen to the gate means adding one entry to [`covered_states`] and nothing else. A
//! second registration site would make FR-016's "one place" claim false, which is why
//! `tests/layout_coverage_registry.rs` scans for one.
//!
//! [`revealing_states`] is a second *list*, not a second *site*: states pinned partway through an
//! animation, which the geometry fixture deliberately does not record. Both live in this file, and
//! the requirement is that this file is the only one either kind is declared in.
//!
//! Every state is built from fixed, invented data (FR-007). Nothing here reads the developer's
//! worktrees, configuration or session store — a fixture that recorded the author's own workspace
//! would be unreproducible anywhere else, including on the same machine tomorrow.

use std::path::PathBuf;

use micold_client::app::{BranchSource, Overlay, SettingsDraft, State, WorktreeForm};
use micold_client::ui::ConnectionStatus;
use micold_core::worktree::{Worktree, WorktreeStatus};

use super::layout::{Anchor, CoveredState, RevealingState, StateUnderTest};

/// A fixed project path. Never canonicalised against a real directory.
const PROJECT: &str = "/fixture/project";

/// Deliberately long, so the label/close-button relationship this gate was built to watch is under
/// real pressure at the canonical window size (FR-008b, FR-018).
const LONG_NAME: &str = "feat-a-deliberately-long-worktree-name-that-crowds-its-controls";

fn worktree(dir_name: &str, branch: &str) -> Worktree {
    Worktree {
        dir_name: dir_name.to_string(),
        path: PathBuf::from(PROJECT).join(".claude/worktrees").join(dir_name),
        branch: Some(branch.to_string()),
        status: WorktreeStatus::Valid,
    }
}

/// A project open, with a stable set of worktrees — the base every non-empty state builds on.
fn with_project() -> State {
    let mut state = State::default();
    state.workspace = super::workspace_with(vec![(PROJECT, vec![])]);
    // `workspace_with` ends by clearing `active`, which every other caller wants. Here it is the
    // difference between rendering the project surface and rendering "Open a folder to set it as
    // your working space." — four covered states were byte-identical until this line existed.
    state.workspace.active = state.workspace.projects.first().map(|p| p.path.clone());
    assert!(
        state.workspace.active_project().is_some(),
        "the covered state must have a project open, or it is not covering what it claims"
    );
    state.worktrees = vec![
        worktree("feat-short", "feat/short"),
        worktree(LONG_NAME, "feat/long"),
        worktree("fix-a-bug", "fix/a-bug"),
    ];
    state.sidebar_width = 260;
    state
}

/// The registry. **This is the one place a covered state is declared** (FR-016).
pub fn covered_states() -> &'static [CoveredState] {
    &[
        // --- Feature 017's reduced parity set (FR-008) ------------------------------------------
        CoveredState {
            name: "main-shell-sidebar-expanded",
            build: || StateUnderTest::new(with_project()),
            anchors: &[
                Anchor { name: "shell.root", path: &[] },
                Anchor { name: "shell.body", path: &[0] },
                // The two elements feature 017's defect was *between* — a name drawn across the
                // control beside it. T023 asked for these and they were missing until quickstart
                // Part B4 was run against a reintroduced defect and reported a bare path.
                //
                // Both are identified from the running gate rather than by reading the tree:
                // the label is the node B4's overflow names (187.6px allowed), and the cluster
                // beside it is `row_actions_cluster`, whose destructive control is Delete, not a
                // close — named for what it is.
                Anchor {
                    name: "sidebar.row.label",
                    path: &[0, 0, 2, 0, 0, 0, 2, 0, 0, 2, 0, 1],
                },
                Anchor {
                    name: "sidebar.row.delete_button",
                    path: &[0, 0, 2, 0, 0, 0, 2, 0, 0, 2, 0, 2, 1],
                },
            ],
        },
        CoveredState {
            name: "main-shell-sidebar-collapsed",
            build: || {
                let mut state = with_project();
                state.sidebar_hidden = true;
                StateUnderTest::new(state)
            },
            anchors: &[Anchor { name: "shell.root", path: &[] }],
        },
        CoveredState {
            name: "add-worktree-dialog-new-branch",
            build: || {
                let mut state = with_project();
                state.overlay = Overlay::AddWorktree;
                state.worktree_form = Some(WorktreeForm {
                    source: BranchSource::New,
                    name: "example".to_string(),
                    ..WorktreeForm::default()
                });
                StateUnderTest::new(state)
            },
            anchors: &[Anchor { name: "dialog.root", path: &[] }],
        },
        CoveredState {
            name: "add-worktree-dialog-existing-branch",
            build: || {
                let mut state = with_project();
                state.overlay = Overlay::AddWorktree;
                state.worktree_form = Some(WorktreeForm {
                    source: BranchSource::Existing,
                    ..WorktreeForm::default()
                });
                StateUnderTest::new(state)
            },
            anchors: &[Anchor { name: "dialog.root", path: &[] }],
        },
        CoveredState {
            name: "worktree-menu-open",
            build: || {
                let mut state = with_project();
                state.worktree_menu_open = Some(LONG_NAME.to_string());
                StateUnderTest::new(state)
            },
            anchors: &[Anchor { name: "shell.root", path: &[] }],
        },
        // --- The empty and error layouts (FR-008c) ----------------------------------------------
        // The screens least often looked at by eye, which is exactly where a gate earns most over
        // the human inspection feature 017 had to close on.
        CoveredState {
            name: "empty-no-project-open",
            build: || StateUnderTest::new(State::default()),
            anchors: &[Anchor { name: "shell.root", path: &[] }],
        },
        CoveredState {
            name: "empty-project-without-worktrees",
            build: || {
                let mut state = with_project();
                state.worktrees.clear();
                StateUnderTest::new(state)
            },
            anchors: &[Anchor { name: "shell.root", path: &[] }],
        },
        CoveredState {
            name: "error-daemon-disconnected",
            build: || StateUnderTest::new(with_project()).connection(ConnectionStatus::Disconnected),
            anchors: &[Anchor { name: "shell.root", path: &[] }],
        },
        // `worktree_error` is rendered by the add-worktree modal and nowhere else
        // (`ui/mod.rs:357`), so setting it on the main shell covered nothing — this state was
        // byte-identical to `main-shell-sidebar-expanded` until the dialog was opened with it.
        CoveredState {
            name: "error-add-worktree-failed",
            build: || {
                let mut state = with_project();
                state.overlay = Overlay::AddWorktree;
                state.worktree_form = Some(WorktreeForm {
                    source: BranchSource::New,
                    name: "example".to_string(),
                    ..WorktreeForm::default()
                });
                state.worktree_error =
                    Some("could not create the worktree: branch already checked out".to_string());
                StateUnderTest::new(state)
            },
            anchors: &[Anchor { name: "dialog.root", path: &[] }],
        },
        // --- The one state that exercises the overlay pass (FR-009) -----------------------------
        // Until this existed the fixture contained **zero** `over` records. The overlay pass was
        // written for `material::Select`, which wraps `pick_list` and lays its dropdown out through
        // `Widget::overlay` where the base walk cannot see it — and then no covered state ever
        // opened one. A pass that runs on every state and records nothing looks exactly like a pass
        // that found nothing, which is the failure this feature exists to correct.
        //
        // `pick_list`'s open flag is private widget state with no accessor, so it is *caused*
        // rather than set: `pressing` dispatches a left press at the control's centre, the way a
        // person opens it. See `resolve_pressing` for why the entrance transition has to be settled
        // first.
        CoveredState {
            name: "add-worktree-dialog-type-menu-open",
            build: || {
                let mut state = with_project();
                state.overlay = Overlay::AddWorktree;
                state.worktree_form = Some(WorktreeForm {
                    source: BranchSource::New,
                    name: "example".to_string(),
                    ..WorktreeForm::default()
                });
                StateUnderTest::new(state).pressing(&[3, 0, 0, 3])
            },
            anchors: &[
                Anchor { name: "dialog.root", path: &[] },
                Anchor { name: "dialog.type-select", path: &[3, 0, 0, 3] },
            ],
        },
        // --- Added to exercise FR-016 end-to-end (T032) ------------------------------------------
        // The Settings dialog: the tallest form the application shows, and the only covered state
        // with a checkbox, a text field carrying a validation error, and a control row that has to
        // fit four labelled inputs into a modal. Nothing about registering it needed a change
        // outside this file, which is the point of adding it.
        //
        // Every value is invented and fixed (FR-007) rather than taken from `SettingsDraft::
        // default()`, whose fields track the shipped defaults and would silently re-record the
        // fixture the day one of them changes.
        CoveredState {
            name: "settings-dialog-with-validation-error",
            build: || {
                let mut state = with_project();
                state.overlay = Overlay::Settings;
                state.settings_draft = Some(SettingsDraft {
                    scrollback_lines: "12000".to_string(),
                    env_include_enabled: true,
                    env_include_script_path: "~/.config/micold/session-env.sh".to_string(),
                    env_include_timeout: "5".to_string(),
                    error: Some("the scrollback limit must be between 100 and 100000".to_string()),
                });
                StateUnderTest::new(state)
            },
            anchors: &[Anchor { name: "dialog.root", path: &[] }],
        },
    ]
}

/// States pinned partway through a reveal. **A second list in this file, not a second site.**
///
/// These are not covered states and must not become them: the geometry fixture excludes
/// mid-animation geometry (T030), and recording a frame partway through a reveal would churn the
/// fixture on any change to a duration or an easing curve — motion's business, not layout's. They
/// exist to be asserted *about*, by the containment invariant, and are recorded nowhere.
///
/// FR-016's "one place" is about where a *covered state* is declared, and this file is still that
/// place. A registry scan (T029) should expect both lists here and neither elsewhere.
pub fn revealing_states() -> &'static [RevealingState] {
    &[
        // BUG-001: the reveal that paints over what moved up. Two frames of a 90ms reveal at
        // ~0.178 per frame lands at ~0.356 — far enough in that `Expand::draw` is past its
        // early-return, and far enough from the end that the parent is still visibly short of
        // its child.
        RevealingState {
            name: "sidebar-filter-panel-mid-reveal",
            build: || StateUnderTest::new(with_project()),
            toward: |state| state.sidebar_filter_open = true,
            frames: 2,
            node: "0/0/0/2/0/0/0/1",
            expect_between: (0.2, 0.5),
        },
    ]
}
