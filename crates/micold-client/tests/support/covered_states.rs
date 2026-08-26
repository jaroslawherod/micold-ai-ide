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

use micold_client::features::worktree;
use std::path::PathBuf;

use micold_client::app::State;
use micold_client::features::connection::ConnectionStatus;
use micold_client::features::session::SessionMenu;
use micold_client::features::settings::SettingsDraft;
use micold_client::features::worktree::WorktreeMenu;
use micold_client::features::worktree_form::{BranchSource, WorktreeForm};
use micold_core::project::Availability;
use micold_core::session::{Session, SessionId, SessionLabel, SessionLocation, TerminalMode};
use micold_core::typeahead::{rank, Query};
use micold_core::worktree::{BranchCandidate, BranchOrigin, Worktree, WorktreeStatus};

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
/// The project-switcher trigger, immediately left of it (feature 008, FR-004) — the same
/// `IconButton` since BUG-007, which is why the two paths now differ only in their last index.
const APP_BAR_SWITCHER_TRIGGER: &[usize] = &[0, 0, 0, 0, 0, 2];

// A layer index is **two per open-able surface**, and the paths below that begin at one were
// re-pointed twice on the way here. First 018's BUG-007 made the switcher a `MenuOverlay`, which is
// pushed whether or not it is open — it owns its own fade, so it must outlive the flag that opened
// it — giving every state one more layer. Then 017's BUG-002 made a surface's *backdrop*
// unconditional too, because a backdrop that came and went renumbered the panels above it and they
// inherited each other's transitions.
//
// (Both of those were written here as "BUG-007" and "BUG-008", unqualified, in a file whose other
// bug references are 018's. The second named no bug that existed: the backdrop fix is 017's
// BUG-002, per the commit that made it. 018's BUG-008 is a different bug entirely — the sidebar's
// context menus opening at a corner — and it arrived to find its number already spoken for.)
//
// So the arithmetic is now stated rather than discovered: base, then a backdrop and a panel for
// each surface `ui::view` pushes, in `stack_order`. A dialog sits above the two popovers, at layer
// 6; the overflow menu's panel is 2 and the switcher's is 4.

/// The terminal's bottom status bar, and the mode toggle that anchors its trailing edge — the two
/// nodes BUG-002 moved. Filled in from the recorded tree rather than derived by reading the view,
/// the way `sidebar.row.label` above was; an anchor that does not resolve fails by name
/// (`an_anchor_whose_path_does_not_resolve_fails_naming_it`), so a stale path here cannot go quiet.
const TERMINAL_BOTTOM_BAR: &[usize] = &[0, 0, 1, 1, 1];
/// The "+" that opens another instance, and the AI tab, the bar row's **last two** children.
///
/// Feature 027 deleted the mode toggle that used to sit past them (FR-001) and reordered what was
/// left: the trailing group reads "+", then AI tab, so the one control that is always present
/// anchors the row's trailing edge. Both indices moved — the "+" was 4, the AI tab 3.
///
/// The AI tab being **last** is load-bearing, not incidental: iced settles a row's shortfall by
/// shrinking trailing children, so the last child is the first thing squeezed when the bar runs out
/// of room. `gates/bar_controls_hold_their_size.rs` reads this anchor for exactly that reason.
const TERMINAL_ADD_INSTANCE: &[usize] = &[0, 0, 1, 1, 1, 0, 3];

/// The instance tab strip, and three of its tabs (feature 012 T057). The bar's row holds its title,
/// a filling spacer and the status text, then whichever optional controls the session's state calls
/// for; with every instance already running there is no session-level restart, so the strip is the
/// row's fourth child. Its own children are one tab per element of `Session.shells`, in order.
///
/// The strip sits inside the bar's child 2 — the `EdgeFade` wrapping the `Length::Fill` horizontal
/// `Scrollable` — so its own path is several levels down: fade → stack → layer → viewport → strip.
///
/// Deep, and deliberately not flattened. Each of those levels is a component doing one thing
/// (`gates/tab_children_fit.rs` and `containment.rs` both read the strip through anchors rather
/// than through these constants, so the depth costs them nothing), and an anchor that no longer
/// resolves fails **by name** — `an_anchor_whose_path_does_not_resolve_fails_naming_it` — so a
/// stale path here cannot go quiet.
/// The width the bar hands the scrolling tab region at the fixture's 1280dp window, as recorded.
///
/// See the two states that set it. It is a measurement pasted in, not a constant the view reads —
/// the view has no opinion about it at all; the running application measures it each frame.
/// The same two controls in a state whose session offers a **restart** (`session-terminal-bottom-
/// bar`): that control is a bar child of its own at index 2, so everything after it moves down one.
///
/// Two constants rather than an offset applied to the pair above, because an anchor that resolves
/// to the wrong node is worse than one that does not resolve at all — it is checked, named and
/// silently about something else. That is not hypothetical here: before feature 027 this state
/// borrowed `TERMINAL_TAB_AI_PINNED`, which in a bar with a restart control lands on the *scrolling
/// region*, and `gates/tab_children_fit.rs` had been measuring that region as though it were a tab.
const TERMINAL_RESTARTABLE_ADD_INSTANCE: &[usize] = &[0, 0, 1, 1, 1, 0, 4];
const TERMINAL_RESTARTABLE_TAB_AI_PINNED: &[usize] = &[0, 0, 1, 1, 1, 0, 5];

const TAB_STRIP_VIEWPORT: u32 = 703;

const TERMINAL_TAB_STRIP: &[usize] = &[0, 0, 1, 1, 1, 0, 2, 0, 0, 0, 0, 0];
const TERMINAL_TAB_LEADING: &[usize] = &[0, 0, 1, 1, 1, 0, 2, 0, 0, 0, 0, 0, 0];
const TERMINAL_TAB_ACTIVE: &[usize] = &[0, 0, 1, 1, 1, 0, 2, 0, 0, 0, 0, 0, 1];
const TERMINAL_TAB_EXITED: &[usize] = &[0, 0, 1, 1, 1, 0, 2, 0, 0, 0, 0, 0, 2];
/// The **AI tab** (feature 026 FR-001, FR-002), the strip's last child — one past the instances.
///
/// Two indices, because two covered states hold different numbers of instances and FR-002 pins this
/// tab to the *end* rather than to a position. Named so `gates/tab_children_fit.rs` reports on it by
/// name: its touch-target assertion catches this tab squeezed by the scrolling viewport, and
/// `a_tabs_content_sits_on_its_tabs_midline` is what actually holds FR-010a's centred icon — the
/// property that failed at 4.6dp on a terminal tab the morning before this feature was planned.
///
/// One path, not one per instance count: FR-002b pins this tab **outside** the scrolling region, so
/// it is the bar row's own child rather than the strip's last one, and its position no longer moves
/// with the number of instances. That is the requirement stated as a path.
const TERMINAL_TAB_AI_PINNED: &[usize] = &[0, 0, 1, 1, 1, 0, 4];

/// Inside a tab: the button's content column, whose first child is the active indicator (or, on an
/// inactive tab, the transparent rule reserving its height) and whose second is the tab's content
/// row — leading spacer, label and close.
///
/// It held a fourth child until BUG-005: a restart affordance on an instance that was not running,
/// which the tab was too narrow to hold and which is offered from the tab's context menu now
/// (FR-010b). Its anchor is deleted with it rather than left pointing at nothing —
/// `an_anchor_whose_path_does_not_resolve_fails_naming_it` would fail on a stale one, which is the
/// behaviour that makes an anchor worth writing.
const TERMINAL_TAB_ACTIVE_INDICATOR: &[usize] = &[0, 0, 1, 1, 1, 0, 2, 0, 0, 0, 0, 0, 1, 0, 0, 0];

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
        included: false,
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
        worktree: worktree::State {
            worktrees: vec![
                worktree("feat-short", "feat/short"),
                worktree(LONG_NAME, "feat/long"),
                worktree("fix-a-bug", "fix/a-bug"),
            ],
            ..Default::default()
        },
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
                state.worktree_form.form = Some(WorktreeForm {
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
                    path: &[6, 0, 0, 1],
                },
            ],
        },
        CoveredState {
            name: "add-worktree-dialog-existing-branch",
            build: || {
                let mut state = with_project();
                state.worktree_form.form = Some(WorktreeForm {
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
                    path: &[6, 0, 0, 1],
                },
            ],
        },
        // 016 BUG-003 item 1's own finding, closed. The state above renders the dialog *before* any
        // branch listing has arrived, so `candidates` is empty and the picker draws a caption
        // instead of a field — which meant the fixture had never contained the search field at all.
        // That is a second reason no gate saw the icon drawn over the label: not only is a layout
        // tree blind to overlap, this one was not looking at the control.
        CoveredState {
            name: "add-worktree-dialog-branch-picker",
            build: || {
                let mut state = with_project();
                let candidates = vec![
                    BranchCandidate {
                        name: "feat/login-page".to_string(),
                        origin: BranchOrigin::Local,
                        blocked_by: None,
                    },
                    BranchCandidate {
                        name: "fix/crash-on-open".to_string(),
                        origin: BranchOrigin::Local,
                        blocked_by: None,
                    },
                    BranchCandidate {
                        name: "chore/bump-deps".to_string(),
                        origin: BranchOrigin::Remote {
                            remote: "origin".to_string(),
                        },
                        blocked_by: None,
                    },
                ];
                // Derived the way the application derives it, rather than written out: an invented
                // `branch_matches` could disagree with `candidates` in a way no reducer can
                // produce, and the fixture would then pin a state that cannot happen.
                let branch_matches = rank(&candidates, |c| c.name.as_str(), &Query::new(""));
                state.worktree_form.form = Some(WorktreeForm {
                    source: BranchSource::Existing,
                    candidates,
                    branch_matches,
                    // Closed: the list is an overlay, and this state exists for the **field** —
                    // the node that was missing. `worktree-menu-open` is where an open overlay is
                    // covered.
                    branch_list_open: false,
                    ..WorktreeForm::default()
                });
                StateUnderTest::new(state)
            },
            anchors: &[
                Anchor {
                    name: "dialog.root",
                    path: &[],
                },
                Anchor {
                    name: "dialog.actions",
                    path: &[6, 0, 0, 1],
                },
            ],
        },
        CoveredState {
            name: "worktree-menu-open",
            build: || {
                let mut state = with_project();
                // Opened from the row, not from a corner (018 BUG-008). The point is well down the
                // sidebar precisely because the defect was invisible at the top of the list: the
                // panel used to be recorded at 24, 96 whichever row it belonged to, and a fixture
                // that only ever opened it near there could not tell the two apart.
                state.window.window_size = (1280, 800);
                state.worktree.menu_open = Some(WorktreeMenu {
                    dir_name: LONG_NAME.to_string(),
                    anchor: (120, 420),
                });
                StateUnderTest::new(state)
            },
            anchors: &[Anchor {
                name: "shell.root",
                path: &[],
            }],
        },
        // The other sidebar menu, and the clamp (018 BUG-008, FR-029d / 015 FR-006). Opened close
        // enough to the bottom edge that the panel does not fit below it, so the fixture records a
        // menu that has been slid back inside rather than one that merely happened to fit.
        CoveredState {
            name: "session-menu-open-at-the-bottom-edge",
            build: || {
                let session = Session::restored(
                    SessionId::new(),
                    SessionLocation::Worktree("feat-short".to_string()),
                    SessionLabel::Named("feat/short".to_string()),
                    TerminalMode::Regular,
                );
                let id = session.id;
                let mut workspace = super::workspace_with(vec![(PROJECT, vec![session])]);
                workspace.active = workspace.projects.first().map(|p| p.path.clone());

                let mut state = with_project();
                state.workspace = workspace;
                state.expanded.insert("feat-short".to_string());
                state.window.window_size = (1280, 800);
                state.session_menu_open = Some(SessionMenu {
                    id,
                    anchor: (120, 760),
                });
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
                state.worktree.worktrees.clear();
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
                state.worktree_form.form = Some(WorktreeForm {
                    source: BranchSource::New,
                    name: "example".to_string(),
                    ..WorktreeForm::default()
                });
                state.worktree_form.worktree_error =
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
                    path: &[6, 0, 0, 1],
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
                state.worktree.worktrees = (0..30)
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
        // written for `material::Select`, which lays its list out through `Widget::overlay` where
        // the base walk cannot see it — and then no covered state ever opened one. A pass that runs
        // on every state and records nothing looks exactly like a pass that found nothing, which is
        // the failure this feature exists to correct.
        //
        // The select's open flag is its own widget state with no accessor, so it is *caused* rather
        // than set: `pressing` dispatches a left press at the control's centre, the way a person
        // opens it. Still true after feature 022 replaced the `pick_list` this was written against
        // — the control changed, its openness stayed its own. See `resolve_pressing` for why the
        // entrance transition has to be settled first, and the list's arrival afterwards.
        CoveredState {
            name: "add-worktree-dialog-type-menu-open",
            build: || {
                let mut state = with_project();
                state.worktree_form.form = Some(WorktreeForm {
                    source: BranchSource::New,
                    name: "example".to_string(),
                    ..WorktreeForm::default()
                });
                StateUnderTest::new(state).pressing(&[6, 0, 0, 0, 2])
            },
            anchors: &[
                Anchor {
                    name: "dialog.root",
                    path: &[],
                },
                Anchor {
                    name: "dialog.type-select",
                    path: &[6, 0, 0, 0, 2],
                },
                // Same form as `add-worktree-dialog-new-branch`; opening the menu adds an overlay
                // layer, not a field.
                Anchor {
                    name: "dialog.actions",
                    path: &[6, 0, 0, 1],
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
                state.settings.settings_draft = Some(SettingsDraft {
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
                    path: &[6, 0, 0, 1],
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
                    name: "terminal.bottom_bar.add_instance",
                    path: TERMINAL_RESTARTABLE_ADD_INSTANCE,
                },
                // Feature 026 FR-003: this session has **no** instances, and until now that meant
                // no strip at all. It has one now, with a single member — the AI tab. This is where
                // the change lands for the user who never opens a second terminal, which is most of
                // them, and it is the state most likely to read as a stray control rather than as a
                // deliberate strip (T030 judges that; this makes the geometry visible).
                //
                // Only the pinned tab is named. The **scrolling** strip is genuinely empty here —
                // there are no instances — and an empty row lays out no node at all, so an anchor
                // for it would point at nothing. That is not a gap: FR-002b is what puts the AI tab
                // outside the viewport, so in a session with no instances the one tab there is is
                // exactly the one this anchor names.
                Anchor {
                    name: "terminal.tabs.pinned",
                    path: TERMINAL_RESTARTABLE_TAB_AI_PINNED,
                },
            ],
        },
        // --- BUG-002's tab strip (feature 012 T057) ---------------------------------------------
        //
        // The state above renders the bottom bar with **one** shell instance, and the switcher
        // returns `None` below two — so the instance tab strip was under no covered state at all.
        // That was found the hard way. Feature 012's BUG-002 rebuilt every tab in the strip —
        // containers became an indicator, each tab gained a row above its content, and the whole
        // set was given a fixed width — and `layout_snapshot.txt` came out byte-identical. Both
        // defects the visual passes caught along the way, a 12dp centring error and an active tab
        // several times wider than its neighbours, are *pure geometry*: in range of this fixture,
        // and out of range of every gate, because the control was never rendered into it.
        //
        // Three instances rather than two, and the active one in the middle. A uniform tab width
        // (feature 012 FR-004c) is a claim about neighbours, and with two tabs "both the same
        // width" and "each the width its own content wants" are indistinguishable whenever the two
        // contents want the same thing. The trailing instance is `Exited`, so it draws the
        // per-entry restart affordance its siblings do not: the tab whose contents differ most from
        // the rest is the one that shows whether the width follows them.
        //
        // **It found one on the first regeneration** — and BUG-005 then fixed it, so what follows is
        // the history rather than the current reading.
        //
        // A restartable tab does not fit inside `TAB_WIDTH`: the content row is given 112dp and its
        // children want 48 (leading spacer) + 4 + 6.8 (label) + 4 + 48 (close) + 4 + 51.5 (restart)
        // = 166.3, so iced shrinks the last two — the restart button collapses to **0.0 wide** and
        // the close control drops to 45.2, below `anatomy::button::MIN_TOUCH_TARGET`. An instance
        // that exits in the background therefore cannot be restarted from its own tab, which is
        // exactly the affordance feature 011 FR-010 put there, and `ui/terminal.rs` asserts the
        // opposite in a comment — "It widens its own tab, which SC-008 permits" — which is what a
        // fixed width makes impossible.
        //
        // It was pinned as the baseline first, on 019 spec.md's own precedent — a snapshot records
        // what it is shown — and the fix then moved it: BUG-005 took the affordance out of the tab
        // for a context menu on it (FR-010b), since sizing every tab to hold a child only a stopped
        // instance draws comes to 204dp against 136. Every other gate in the suite had been green
        // over that zero-width button, which is the measure of how far outside their reach this
        // control was, and `tests/gates/tab_children_fit.rs` is the gate that closes the gap.
        CoveredState {
            name: "session-terminal-instance-tabs",
            build: || {
                let mut session = Session::restored(
                    SessionId::new(),
                    SessionLocation::Worktree("feat-short".to_string()),
                    SessionLabel::Named("feat/short".to_string()),
                    TerminalMode::Regular,
                );
                let leading = session.open_shell_instance();
                let active_shell = session.open_shell_instance();
                let exited = session.open_shell_instance();
                session.mark_shell_running(leading);
                session.mark_shell_running(active_shell);
                session.mark_shell_exited(exited);
                // Not the last-opened one, which `open_shell_instance` would have left active —
                // an active *trailing* tab cannot tell "the indicator spans its own tab" from
                // "the indicator spans everything after the tab before it".
                session.select_shell(active_shell);

                let active = session.id;
                let mut workspace = super::workspace_with(vec![(PROJECT, vec![session])]);
                workspace.active = workspace.projects.first().map(|p| p.path.clone());

                let mut state = with_project();
                state.workspace = workspace;
                state.active_session = Some(active);
                // The width the bar actually gives the scrolling region, which the running
                // application learns from `Scrollable::on_viewport_resize` and a hand-built state
                // does not. Feature 027 FR-003 spends whatever of it the tabs do not need as
                // leading slack, so a state that left this at its default would render the strip
                // flush **left** — the one arrangement the feature exists to replace, and the one
                // `gates/tabs_anchor_the_trailing_edge.rs` could then never see.
                //
                // Not a magic number: that gate compares the strip's leading gap against the
                // recorded viewport, so a figure that has drifted from what the bar hands out fails
                // there with the current one in the message.
                state.tab_strip_viewport_width = TAB_STRIP_VIEWPORT;
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
                Anchor {
                    name: "terminal.bottom_bar",
                    path: TERMINAL_BOTTOM_BAR,
                },
                Anchor {
                    name: "terminal.tabs",
                    path: TERMINAL_TAB_STRIP,
                },
                Anchor {
                    name: "terminal.tabs.leading",
                    path: TERMINAL_TAB_LEADING,
                },
                Anchor {
                    name: "terminal.tabs.active",
                    path: TERMINAL_TAB_ACTIVE,
                },
                Anchor {
                    name: "terminal.tabs.active.indicator",
                    path: TERMINAL_TAB_ACTIVE_INDICATOR,
                },
                Anchor {
                    name: "terminal.tabs.exited",
                    path: TERMINAL_TAB_EXITED,
                },
                Anchor {
                    name: "terminal.tabs.pinned",
                    path: TERMINAL_TAB_AI_PINNED,
                },
                // The reference width for `bar_controls_hold_their_size`'s cross-state comparison
                // (T015): this state has room to spare, and the overflowing one does not, so the
                // AI tab measuring the same in both is a direct reading of FR-002c. It is the bar
                // row's last child since feature 027, which makes it the first control iced shrinks
                // — the strongest position to read that requirement from.
                Anchor {
                    name: "terminal.add_instance",
                    path: TERMINAL_ADD_INSTANCE,
                },
            ],
        },
        // --- feature 026's overflowing bar (T014, FR-002c) --------------------------------------
        //
        // Every state above this line holds at most three instances, which the bar has room for.
        // Past about five it does not, and the way iced settles the shortfall is silent: a fixed
        // parent width is a *budget*, and the trailing children are shrunk to fit it — laid out
        // narrower, or at zero, with nothing reported. That is feature 012's BUG-005 one level out,
        // and it is live on `main` today, independent of this feature.
        //
        // Six instances at the fixture's 1280dp window is past the wall by a clear margin: the bar
        // is ~1014dp and a tab is 136 on a 144 pitch, so six tabs want 864 of it while the title,
        // the status, the "+" and the mode toggle want the rest. `gates/bar_controls_hold_their_
        // size.rs` is what reads the result; without this state it would inspect nothing, which is
        // the "a pass that records nothing looks like a pass that found nothing" shape feature 019
        // keeps meeting.
        CoveredState {
            name: "session-terminal-instance-tabs-overflowing",
            build: || {
                let mut session = Session::restored(
                    SessionId::new(),
                    SessionLocation::Worktree("feat-short".to_string()),
                    SessionLabel::Named("feat/short".to_string()),
                    TerminalMode::Regular,
                );
                let mut opened = Vec::new();
                for _ in 0..6 {
                    opened.push(session.open_shell_instance());
                }
                for id in &opened {
                    session.mark_shell_running(*id);
                }
                // The second of six, so the marked tab is neither the leading one nor the trailing
                // one — the two positions that cannot tell "the indicator spans its own tab" from
                // "it spans everything up to here".
                session.select_shell(opened[1]);

                let active = session.id;
                let mut workspace = super::workspace_with(vec![(PROJECT, vec![session])]);
                workspace.active = workspace.projects.first().map(|p| p.path.clone());

                let mut state = with_project();
                state.workspace = workspace;
                state.active_session = Some(active);
                // The width the bar actually gives the scrolling region, which the running
                // application learns from `Scrollable::on_viewport_resize` and a hand-built state
                // does not. Feature 027 FR-003 spends whatever of it the tabs do not need as
                // leading slack, so a state that left this at its default would render the strip
                // flush **left** — the one arrangement the feature exists to replace, and the one
                // `gates/tabs_anchor_the_trailing_edge.rs` could then never see.
                //
                // Not a magic number: that gate compares the strip's leading gap against the
                // recorded viewport, so a figure that has drifted from what the bar hands out fails
                // there with the current one in the message.
                state.tab_strip_viewport_width = TAB_STRIP_VIEWPORT;
                StateUnderTest::new(state)
            },
            anchors: &[
                Anchor {
                    name: "shell.root",
                    path: &[],
                },
                Anchor {
                    name: "terminal.bottom_bar",
                    path: TERMINAL_BOTTOM_BAR,
                },
                Anchor {
                    name: "terminal.tabs",
                    path: TERMINAL_TAB_STRIP,
                },
                Anchor {
                    name: "terminal.tabs.pinned",
                    path: TERMINAL_TAB_AI_PINNED,
                },
                Anchor {
                    name: "terminal.add_instance",
                    path: TERMINAL_ADD_INSTANCE,
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
                state.help.help_menu_open = true;
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
                state.project.switcher_open = true;
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
                    path: &[4, 0],
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
