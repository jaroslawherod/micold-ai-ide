//! The covered states, declared in exactly one place (feature 019, T019/T020, FR-016).
//!
//! Adding a screen to the gate means adding one entry to [`covered_states`] and nothing else. A
//! second registration site would make FR-016's "one place" claim false, which is why
//! `tests/layout_coverage_registry.rs` scans for one.
//!
//! Every state is built from fixed, invented data (FR-007). Nothing here reads the developer's
//! worktrees, configuration or session store — a fixture that recorded the author's own workspace
//! would be unreproducible anywhere else, including on the same machine tomorrow.

use std::path::PathBuf;

use micold_client::app::{BranchSource, Overlay, State, WorktreeForm};
use micold_client::ui::ConnectionStatus;
use micold_core::worktree::{Worktree, WorktreeStatus};

use super::layout::{Anchor, CoveredState, StateUnderTest};

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
        CoveredState {
            name: "error-worktree-creation-failed",
            build: || {
                let mut state = with_project();
                state.worktree_error =
                    Some("could not create the worktree: branch already checked out".to_string());
                StateUnderTest::new(state)
            },
            anchors: &[Anchor { name: "shell.root", path: &[] }],
        },
    ]
}
