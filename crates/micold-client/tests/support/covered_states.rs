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
use micold_core::session::{Session, SessionId, SessionLabel, SessionLocation, TerminalMode};
use micold_core::worktree::{Worktree, WorktreeStatus};

use super::layout::{Anchor, CoveredState, RevealingState, StateUnderTest};

/// A fixed project path. Never canonicalised against a real directory.
const PROJECT: &str = "/fixture/project";

/// A second project, never active. Only the switcher's covered state needs one — it is what makes
/// an *unmarked* row exist to be aligned against a marked one.
const OTHER: &str = "/fixture/other-project";

/// The toolbar's title, at the same path in every state: shell column → toolbar column → bar
/// container → bar row → leading child.
const TOOLBAR_TITLE: &[usize] = &[0, 0, 0, 0, 0, 0];

/// The app bar itself — §7.1's 64dp box, one level above the row that holds the title and the
/// actions, and one below the column that adds its divider. It is the band BUG-003's panels were
/// drawn across, so `gates/panel_placement.rs` is about this node and the states that open a panel
/// name it (T100).
const APP_BAR: &[usize] = &[0, 0, 0, 0];
/// The overflow (⋮) trigger, trailing action of the bar. The panel that clipped it opens from here.
const APP_BAR_OVERFLOW_TRIGGER: &[usize] = &[0, 0, 0, 0, 0, 3];
/// The project-switcher trigger, immediately left of it (feature 008, FR-004).
const APP_BAR_SWITCHER_TRIGGER: &[usize] = &[0, 0, 0, 0, 0, 2];

/// The terminal's bottom status bar, and the mode toggle that anchors its trailing edge — the two
/// nodes BUG-002 moved. Filled in from the recorded tree rather than derived by reading the view,
/// the way `sidebar.row.label` above was; an anchor that does not resolve fails by name
/// (`an_anchor_whose_path_does_not_resolve_fails_naming_it`), so a stale path here cannot go quiet.
const TERMINAL_BOTTOM_BAR: &[usize] = &[0, 0, 1, 1, 1];
const TERMINAL_MODE_TOGGLE: &[usize] = &[0, 0, 1, 1, 1, 0, 5];

/// A **nested** sidebar row — the session under an expanded `feat-short`, at depth 1 in the tree
/// (BUG-005, T116). The sidebar's tree column is `…/2/0/0`, whose children are its rows in order:
/// the `Default` project row, then `feat-short`, then its session, then the two remaining
/// worktrees. Index 2 is that session.
///
/// Named because this is the node whose height differed from its siblings' for two days with no
/// fixture able to say so — every other covered state stops at depth 0.
const SIDEBAR_SESSION_ROW: &[usize] = &[0, 0, 1, 0, 0, 0, 2, 0, 0, 2];

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
                    // Re-pointed under BUG-005 (T115): the row gained one level when §7.2's height
                    // moved onto it, since a floor on a two-line row has to be a sibling of the
                    // whole body rather than of one of its lines. `…/2/0` is now the height spacer
                    // and `…/2/1` the content, so every path below the row gains a `1`. The gate
                    // named this anchor rather than letting it drift onto a neighbouring node,
                    // which is the whole reason anchors are named (019 FR-004).
                    path: &[0, 0, 1, 0, 0, 0, 2, 0, 0, 2, 1, 0, 1],
                },
                Anchor {
                    name: "sidebar.row.delete_button",
                    path: &[0, 0, 1, 0, 0, 0, 2, 0, 0, 2, 1, 0, 2, 1],
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
                    path: &[3, 0, 0, 1],
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
                // Two fewer fields than the new-branch form: one picker in place of the select,
                // the ticket and the name. It was three until §7.7 moved the select's
                // free-standing `Type` label inside the control's own container.
                Anchor {
                    name: "dialog.actions",
                    path: &[3, 0, 0, 1],
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
                    path: &[3, 0, 0, 1],
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
                StateUnderTest::new(state).pressing(&[3, 0, 0, 0, 2])
            },
            anchors: &[
                Anchor {
                    name: "dialog.root",
                    path: &[],
                },
                Anchor {
                    name: "dialog.type-select",
                    path: &[3, 0, 0, 0, 2],
                },
                // Same form as `add-worktree-dialog-new-branch`; opening the menu adds an overlay
                // layer, not a field.
                Anchor {
                    name: "dialog.actions",
                    path: &[3, 0, 0, 1],
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
                    path: &[3, 0, 0, 1],
                },
            ],
        },
        // --- Added by BUG-002 -------------------------------------------------------------------
        //
        // The terminal's bottom status bar had **no geometry coverage at all**, and BUG-002 broke
        // it: its two icon controls each claimed an equal share of the bar's free width, so the
        // mode toggle — which its own call site anchors to the bar's bottom-right corner — sat
        // hundreds of pixels inside that corner. The app bar's half of the same defect *was*
        // recorded here, as a 499.9 × 64.0 icon button, and the gate was green on it: a snapshot
        // records what it is shown. What it does do is fail the moment the geometry moves again,
        // which is the half of this that no other check offers for this screen.
        //
        // `Regular` mode and no shell started, because that is the arrangement with the most in the
        // bar: the restart action, the new-instance "+", and the mode toggle. A `Named` label
        // rather than `Pending`, so the title's width is a fixed string rather than a placeholder
        // that a future copy change would silently move.
        CoveredState {
            name: "session-terminal-bottom-bar",
            build: || {
                let session = Session::restored(
                    SessionId::new(),
                    SessionLocation::Worktree("feat-short".to_string()),
                    SessionLabel::Named("feat/short".to_string()),
                    TerminalMode::Regular,
                );
                let active = session.id;
                let mut workspace = super::workspace_with(vec![(PROJECT, vec![session])]);
                workspace.active = workspace.projects.first().map(|p| p.path.clone());

                let mut state = with_project();
                state.workspace = workspace;
                state.active_session = Some(active);
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
                // The bar itself, and the control BUG-002 moved. Named so a failure says "the mode
                // toggle" rather than a bare path — the whole point of FR-004.
                Anchor {
                    name: "terminal.bottom_bar",
                    path: TERMINAL_BOTTOM_BAR,
                },
                Anchor {
                    name: "terminal.bottom_bar.mode_toggle",
                    path: TERMINAL_MODE_TOGGLE,
                },
            ],
        },
        // --- BUG-005's nested rows (T116) -------------------------------------------------------
        //
        // Every state above this line holds **only depth-0 tree rows**. That is not a small gap: a
        // sidebar's whole reason to be a tree is that it nests, and the height of a nested row had
        // come to differ from the height of a top-level one — §7.2's floor rode on each row's
        // indent spacer, which is `Fixed(0)` wide at depth 0 and therefore void, so iced dropped it
        // and the floor applied to nested rows alone. The fixture could not see any of it. When the
        // floor was then deleted outright, taking 34% off every session row in the running
        // application, `layout_snapshot.txt` came out byte-identical — and the byte-identity was
        // reported as evidence that nothing had moved (T076).
        //
        // So this state exists to make a depth-1 row exist. 019's FR-008d is the requirement it
        // now answers: a screen's collapsed form is not a sample of its expanded one.
        CoveredState {
            name: "main-shell-worktree-expanded",
            build: || {
                let session = Session::restored(
                    SessionId::new(),
                    SessionLocation::Worktree("feat-short".to_string()),
                    SessionLabel::Named("feat/short".to_string()),
                    TerminalMode::Regular,
                );
                let mut workspace = super::workspace_with(vec![(PROJECT, vec![session])]);
                workspace.active = workspace.projects.first().map(|p| p.path.clone());

                let mut state = with_project();
                state.workspace = workspace;
                // The one line this state is about. Without it the session exists in the workspace
                // and the sidebar still draws a flat list of depth-0 rows.
                state.expanded.insert("feat-short".to_string());
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
                // The nested row itself. Named rather than left as a bare path because a failure
                // here should say "the session row" — it is the node this whole state exists for,
                // and the one whose height silently differed from its parent's for two days.
                Anchor {
                    name: "sidebar.session_row",
                    path: SIDEBAR_SESSION_ROW,
                },
            ],
        },
        // --- BUG-003's two panels (T100) --------------------------------------------------------
        //
        // Neither had a covered state, which is why nothing named the panel that opens over the app
        // bar. The *geometry* was not absent — a closed `MenuOverlay` still yields a surface, so the
        // overflow panel is laid out at `1032, 52, 240 × 264` in nearly every state already, and the
        // fixture has recorded it there from the day it landed. What was missing is a state in which
        // either panel is **open** (an open surface carries a dismissal backdrop, which is a layer of
        // its own and shifts the panel's path), and any assertion at all about where a panel sits.
        // The second half is `gates/panel_placement.rs`; this is the first.
        CoveredState {
            name: "toolbar-overflow-menu-open",
            build: || {
                let mut state = with_project();
                state.help_menu_open = true;
                StateUnderTest::new(state)
            },
            anchors: &[
                // The three elements BUG-003 is *between*: the bar, the trigger the panel hangs
                // from, and the panel. A failure that named only a path would say nothing about
                // which of them moved.
                Anchor {
                    name: "app_bar",
                    path: APP_BAR,
                },
                Anchor {
                    name: "app_bar.overflow_trigger",
                    path: APP_BAR_OVERFLOW_TRIGGER,
                },
                Anchor {
                    name: "menu.panel",
                    path: &[2, 0],
                },
            ],
        },
        CoveredState {
            name: "project-switcher-open",
            build: || {
                let mut state = with_project();
                // A second, *inactive* project, so the panel holds a marked row and an unmarked one
                // and the fixture records where each label starts (FR-006a of feature 008). With one
                // project the list is all-marked, and a marker that shifted its neighbours sideways
                // would have nothing to shift.
                let mut workspace = super::workspace_with(vec![(PROJECT, vec![]), (OTHER, vec![])]);
                workspace.active = workspace.projects.first().map(|p| p.path.clone());
                state.workspace = workspace;
                state.project_switcher_open = true;
                StateUnderTest::new(state)
            },
            anchors: &[
                Anchor {
                    name: "app_bar",
                    path: APP_BAR,
                },
                Anchor {
                    name: "app_bar.switcher_trigger",
                    path: APP_BAR_SWITCHER_TRIGGER,
                },
                // Layer 3, not 2: the closed overflow menu is still a surface (layer 1), and an open
                // surface's dismissal backdrop takes a layer of its own before the panel.
                Anchor {
                    name: "switcher.panel",
                    path: &[3, 0],
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
            node: "0/0/0/1/0/0/0/1",
            // Widened when the reveal took §6.2's easing (T064). A *decelerating* curve covers
            // most of its distance early, so two frames of a 150ms transition now sit at 0.64 of
            // the open height where a linear ramp put them at 0.21. The window still fails at 0 and
            // at 1, which is what "pinned mid-reveal" has to mean; it no longer assumes the value
            // and the elapsed time are the same number.
            expect_between: (0.2, 0.8),
        },
    ]
}
