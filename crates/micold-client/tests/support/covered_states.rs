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
use micold_core::project::Availability;
use micold_core::worktree::{Worktree, WorktreeStatus};

use super::layout::{Anchor, CoveredState, RevealingState, StateUnderTest};

/// A fixed project path. Never canonicalised against a real directory.
const PROJECT: &str = "/fixture/project";

/// The toolbar's title, at the same path in every state: shell column → toolbar column → bar
/// container → bar row → leading child.
const TOOLBAR_TITLE: &[usize] = &[0, 0, 0, 0, 0, 0];

/// Deliberately long, so the label/close-button relationship this gate was built to watch is under
/// real pressure at the canonical window size (FR-008b, FR-018).
const LONG_NAME: &str = "feat-a-deliberately-long-worktree-name-that-crowds-its-controls";

fn worktree(dir_name: &str, branch: &str) -> Worktree {
    Worktree {
        dir_name: dir_name.to_string(),
        path: PathBuf::from(PROJECT)
            .join(".claude/worktrees")
            .join(dir_name),
        branch: Some(branch.to_string()),
        status: WorktreeStatus::Valid,
    }
}

/// A project open, with a stable set of worktrees — the base every non-empty state builds on.
fn with_project() -> State {
    let mut workspace = super::workspace_with(vec![(PROJECT, vec![])]);
    // `workspace_with` ends by clearing `active`, which every other caller wants. Here it is the
    // difference between rendering the project surface and rendering "Open a folder to set it as
    // your working space." — four covered states were byte-identical until this line existed.
    workspace.active = workspace.projects.first().map(|p| p.path.clone());

    let state = State {
        workspace,
        worktrees: vec![
            worktree("feat-short", "feat/short"),
            worktree(LONG_NAME, "feat/long"),
            worktree("fix-a-bug", "fix/a-bug"),
        ],
        sidebar_width: 260,
        ..State::default()
    };
    assert!(
        state.workspace.active_project().is_some(),
        "the covered state must have a project open, or it is not covering what it claims"
    );
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
                Anchor {
                    name: "shell.root",
                    path: &[],
                },
                Anchor {
                    name: "shell.body",
                    path: &[0],
                },
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
                // `Toolbar`'s leading `text(title)` (`material/toolbar.rs:46`). Named on both shell
                // states rather than on all eleven: the toolbar is behind every dialog too, and an
                // anchor repeated on a state whose failures are never about it is noise.
                Anchor {
                    name: "toolbar.title",
                    path: TOOLBAR_TITLE,
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
            anchors: &[
                Anchor {
                    name: "shell.root",
                    path: &[],
                },
                Anchor {
                    name: "toolbar.title",
                    path: TOOLBAR_TITLE,
                },
            ],
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
            anchors: &[
                Anchor {
                    name: "dialog.root",
                    path: &[],
                },
                // The Create/Cancel row. **The index differs per state and that is not avoidable**:
                // it is the last child of a column whose length depends on which fields the form
                // shows, so there is no single path that means "the actions" across all of them.
                // `the_action_row_anchors_are_action_rows` holds each one to the signature rather
                // than to the number.
                Anchor {
                    name: "dialog.actions",
                    path: &[3, 0, 0, 5],
                },
            ],
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
            anchors: &[
                Anchor {
                    name: "dialog.root",
                    path: &[],
                },
                // Three fewer fields than the new-branch form: one picker in place of label,
                // select, ticket and name.
                Anchor {
                    name: "dialog.actions",
                    path: &[3, 0, 0, 3],
                },
            ],
        },
        CoveredState {
            name: "worktree-menu-open",
            build: || {
                let mut state = with_project();
                state.worktree_menu_open = Some(LONG_NAME.to_string());
                StateUnderTest::new(state)
            },
            anchors: &[Anchor {
                name: "shell.root",
                path: &[],
            }],
        },
        // --- The empty and error layouts (FR-008c) ----------------------------------------------
        // The screens least often looked at by eye, which is exactly where a gate earns most over
        // the human inspection feature 017 had to close on.
        CoveredState {
            name: "empty-no-project-open",
            build: || StateUnderTest::new(State::default()),
            anchors: &[Anchor {
                name: "shell.root",
                path: &[],
            }],
        },
        CoveredState {
            name: "empty-project-without-worktrees",
            build: || {
                let mut state = with_project();
                state.worktrees.clear();
                StateUnderTest::new(state)
            },
            anchors: &[Anchor {
                name: "shell.root",
                path: &[],
            }],
        },
        CoveredState {
            name: "error-daemon-disconnected",
            build: || {
                StateUnderTest::new(with_project()).connection(ConnectionStatus::Disconnected)
            },
            anchors: &[Anchor {
                name: "shell.root",
                path: &[],
            }],
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
            anchors: &[
                Anchor {
                    name: "dialog.root",
                    path: &[],
                },
                // One past the new-branch form: the error sentence is a field-column child of its
                // own, and it pushes the actions down.
                Anchor {
                    name: "dialog.actions",
                    path: &[3, 0, 0, 6],
                },
            ],
        },
        // FR-008c's second required layout, and the last of the three to be covered. `shell.rs`
        // renders an unavailable folder as a plain `Button::filled("Unavailable")` where an
        // available one gets an icon-plus-label composite, so the two are different geometry.
        //
        // It has to be *set* rather than arranged, because `FakeScanner::default()` answers
        // `is_available: true` and every state built through `workspace_with` therefore takes the
        // available branch by construction. That is why this was missed: not an oversight about
        // which states to add, but a scaffold that could not produce the state at all.
        //
        // No project active, which is the honest form of it — the folder is gone, `Workspace::
        // activate` refuses to open an unavailable project, and this is what a restart shows.
        CoveredState {
            name: "error-project-unavailable",
            build: || {
                let mut state = with_project();
                state.workspace.active = None;
                for project in &mut state.workspace.projects {
                    project.availability = Availability::Unavailable;
                }
                StateUnderTest::new(state)
            },
            anchors: &[Anchor {
                name: "shell.root",
                path: &[],
            }],
        },
        // --- The one state that scrolls (FR-011) ------------------------------------------------
        // Until this existed, nothing in the fixture overflowed its viewport. FR-011 requires
        // scroll-dependent geometry to be recorded at a defined offset, and `a_fresh_tree_samples_
        // at_rest` proved a fresh tree reports every scrollable at the top — over `State::default()`,
        // where nothing scrolls. The guarantee held over a tree in which no element's geometry
        // depended on scroll position, which is the overlay pass's failure exactly.
        //
        // The sidebar's list is the only scrollable in the application whose content is driven by
        // state, so overflowing it is the way in. The count is deliberate rather than generous:
        // enough that the content exceeds a 764.8px viewport with margin, and no more.
        CoveredState {
            name: "main-shell-sidebar-scrolled-to-top",
            build: || {
                let mut state = with_project();
                state.worktrees = (0..30)
                    .map(|i| worktree(&format!("feat-{i:02}"), &format!("feat/{i:02}")))
                    .collect();
                StateUnderTest::new(state)
            },
            anchors: &[Anchor {
                name: "shell.root",
                path: &[],
            }],
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
                StateUnderTest::new(state).pressing(&[3, 0, 0, 2])
            },
            anchors: &[
                Anchor {
                    name: "dialog.root",
                    path: &[],
                },
                Anchor {
                    name: "dialog.type-select",
                    path: &[3, 0, 0, 2],
                },
                // Same form as `add-worktree-dialog-new-branch`; opening the menu adds an overlay
                // layer, not a field.
                Anchor {
                    name: "dialog.actions",
                    path: &[3, 0, 0, 5],
                },
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
            anchors: &[
                Anchor {
                    name: "dialog.root",
                    path: &[],
                },
                // The tallest form, so the furthest-down action row: seven children above it.
                // Was eight until §7.7 moved the scrollback field's free-standing label inside the
                // field's own container.
                Anchor {
                    name: "dialog.actions",
                    path: &[3, 0, 0, 7],
                },
            ],
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
