//! A context menu opens at the press point that opened it (BUG-008, SC-008f, FR-029d).
//!
//! The sixth gate, and the first to read a panel against **the element it was opened from**.
//!
//! Every check before it asks whether a component matches a figure, or whether two components
//! collide:
//!
//! - `tokens_anatomy` compares the *constants*. §7.5 says the panel is 160 wide and it is.
//! - `anatomy_size` reads one component's laid-out box under two limits. The panel is exactly the
//!   size §7.5 asks for, wherever it happens to be.
//! - `content_placement` rasterises one component and asks where its content sits inside it. A
//!   panel opened at the wrong corner has perfectly placed content.
//! - `panel_placement` reads a panel against the app bar. BUG-008's menus clear the bar by 31dp.
//! - `sibling_parity` reads a component against its siblings of the same kind — by *size*. Two
//!   menus can agree on every dimension and both point at nothing.
//!
//! So a menu anchored at a constant passes all five. `SIDEBAR_MENU_ANCHOR = (24, 96)` did, for as
//! long as it existed: the panel was the right size, on the right surface, clear of the bar, inside
//! the window — and a right-click on the last row of a long sidebar was answered at the top of it,
//! over the header and two unrelated rows. The missing question is the one this file asks.
//!
//! # It drives the gesture, not the state
//!
//! The other gates build a state and measure what it renders. This one dispatches a **real
//! secondary press** at a real point over a real row, feeds whatever messages that publishes back
//! through `State::update`, and measures what the next render puts on screen. Three reasons:
//!
//! 1. The defect is in the seam between the widget and the message. A gate that set the menu's
//!    anchor itself would be asserting about the half of the chain that was never broken.
//! 2. It names no message variant, so it cannot be quietly satisfied by renaming one.
//! 3. Two distinct press points, not one. A single press cannot separate "anchored at the press"
//!    from "anchored at a constant that happens to be near it" — `anatomy_size`'s two-limits
//!    argument, one scope out.
//!
//! # Compiled into the `layout_snapshot` binary
//!
//! For a different reason than `containment`, `panel_placement` and `sibling_parity`, which share
//! this binary to reach its record cache. This one builds its own states and reads no cache; it is
//! here so that it can reuse `panel_placement`'s rule for *what counts as an anchored panel* rather
//! than restating it. A restated rule is a copy nothing links to its original, which is the shape
//! FR-029a exists to forbid.

use crate::panel_placement::anchored_panels;
use crate::support::layout::{self as lay, LayoutRecord};
use micold_client::app::{Message, State};
use micold_client::features::connection::ConnectionStatus;
use micold_core::session::{
    AiCli, Session, SessionId, SessionLabel, SessionLocation, TerminalMode,
};
use micold_core::theme::{ColorScheme, ThemePreference};
use micold_core::tokens::{anatomy, density};
use micold_core::worktree::{Worktree, WorktreeStatus};
use std::path::PathBuf;

/// Geometry is structural; one scheme establishes it, as in every gate beside this one.
const RECORDED_SCHEME: ColorScheme = ColorScheme::Light;

/// Half a pixel, matching `containment`, `panel_placement` and the text-overflow gate.
const TOLERANCE: f32 = 0.5;

/// A fixed project path, invented — never the developer's own (FR-007).
const PROJECT: &str = "/fixture/project";

/// Invented projects for the switcher's list. The first is [`PROJECT`], so the active project and
/// the sidebar are the same in every state here.
const PROJECTS: [&str; 2] = [PROJECT, "/fixture/other-project"];

/// Enough worktrees that the sidebar's **last** row is far from its first, and few enough that the
/// list still fits the window unscrolled — a row below the fold cannot be pressed. The defect was
/// invisible at the top of the list and obvious at the bottom, so the fixture has to have a bottom.
const WORKTREE_COUNT: usize = 8;

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

/// As [`with_project`], with `count` projects known to the switcher. Only the first is active, so
/// the sidebar — and every other surface this file presses — is unchanged by the rest.
fn with_projects(count: usize) -> State {
    let mut state = with_project(Vec::new());
    let mut workspace = crate::support::workspace_with(
        (0..count)
            .map(|i| (PROJECTS[i], Vec::new()))
            .collect::<Vec<_>>(),
    );
    workspace.active = workspace.projects.first().map(|p| p.path.clone());
    state.workspace = workspace;
    state
}

/// A project open, with a long worktree list and whatever sessions the caller wants in it.
fn with_project(sessions: Vec<Session>) -> State {
    let mut workspace = crate::support::workspace_with(vec![(PROJECT, sessions)]);
    workspace.active = workspace.projects.first().map(|p| p.path.clone());

    let mut state = State {
        workspace,
        worktrees: (0..WORKTREE_COUNT)
            .map(|i| worktree(&format!("feat-{i:02}"), &format!("feat/{i:02}")))
            .collect(),
        sidebar_width: 260,
        // Clamping is only meaningful against a window whose size the application knows.
        window_size: (lay::WINDOW.width as u16, lay::WINDOW.height as u16),
        ..State::default()
    };
    state.theme_pref = match RECORDED_SCHEME {
        ColorScheme::Light => ThemePreference::Light,
        ColorScheme::Dark => ThemePreference::Dark,
    };
    state
}

/// Render `state` and resolve its geometry.
fn records(state: &State) -> Vec<LayoutRecord> {
    let renderer = lay::renderer();
    let element = micold_client::ui::view(
        state,
        None,
        None,
        0,
        None,
        &micold_core::env_include::EnvIncludeOutcome::Disabled,
        &ConnectionStatus::Connected,
        &micold_client::features::sandbox::Sandbox::default(),
    );
    lay::resolve(element, &renderer)
}

/// Dispatch a secondary (right) press at `point` and apply whatever it publishes to `state`.
///
/// Returns the number of messages applied, so a test can fail on "the press reached nothing" with a
/// different message than "the press reached something and it opened in the wrong place".
fn right_press_at(state: &mut State, point: (f32, f32)) -> usize {
    use iced::advanced::widget::Tree;
    use iced::advanced::{clipboard, layout, mouse, Layout, Shell};
    use iced::{Rectangle, Size};

    let renderer = lay::renderer();
    let mut element = micold_client::ui::view(
        state,
        None,
        None,
        0,
        None,
        &micold_core::env_include::EnvIncludeOutcome::Disabled,
        &ConnectionStatus::Connected,
        &micold_client::features::sandbox::Sandbox::default(),
    );
    let mut tree = Tree::new(element.as_widget());
    let limits = layout::Limits::new(Size::ZERO, lay::WINDOW);
    let node = element
        .as_widget_mut()
        .layout(&mut tree, &renderer, &limits);

    // Settle whatever is already on screen before pressing into it. A panel mounts at opacity zero
    // and `Fade` returns early below `HIDDEN` for every event that is not a `Window` event, so a
    // press dispatched into a freshly built tree is swallowed — the switcher's panel, which the
    // project row lives in, declines the click and the gate reports "the row does not answer a
    // right-click" about a row that does. `support::layout::resolve_pressing` hands over the same
    // frames for the same reason; the count is its.
    const SETTLE_FRAMES: u32 = 8;
    let origin = std::time::Instant::now();
    let mut settling: Vec<Message> = Vec::new();
    for frame in 0..SETTLE_FRAMES {
        let mut shell = Shell::new(&mut settling);
        element.as_widget_mut().update(
            &mut tree,
            &iced::Event::Window(iced::window::Event::RedrawRequested(
                origin + lay::FRAME * frame,
            )),
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &renderer,
            &mut clipboard::Null,
            &mut shell,
            &Rectangle::with_size(lay::WINDOW),
        );
    }

    let mut messages: Vec<Message> = Vec::new();
    let mut shell = Shell::new(&mut messages);
    element.as_widget_mut().update(
        &mut tree,
        &iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)),
        Layout::new(&node),
        mouse::Cursor::Available(iced::Point::new(point.0, point.1)),
        &renderer,
        &mut clipboard::Null,
        &mut shell,
        &Rectangle::with_size(lay::WINDOW),
    );

    drop(shell);
    let applied = messages.len();
    // The element borrows `state`; nothing may touch it until the borrow ends.
    drop(element);
    for message in messages {
        state.update(message);
    }
    applied
}

/// The centre of the recorded node at `path`.
fn centre_of(records: &[LayoutRecord], path: &[usize], what: &str) -> (f32, f32) {
    let node = records
        .iter()
        .find(|r| r.layer == lay::Layer::Base && r.path == path)
        .unwrap_or_else(|| {
            panic!(
                "no node at {} to press — {what} moved, so re-point the path against \
                 layout_snapshot.txt",
                lay::path_token(path),
            )
        });
    assert!(
        node.width > 0.0 && node.height > 0.0,
        "the node at {} has no area, so a press lands on nothing while every assertion still \
         passes ({what})",
        lay::path_token(path),
    );
    (node.x + node.width / 2.0, node.y + node.height / 2.0)
}

/// The one anchored panel that differs between two renders.
///
/// Every state here carries the same panels whether or not they are showing — a `MenuOverlay` is
/// pushed either way, so that it can outlive the flag that opened it and fade — and a panel that is
/// merely *hidden* is laid out exactly where a shown one would be. Taking the difference therefore
/// names a panel without this file having to know its layer index, which is arithmetic
/// `covered_states.rs` has already had to restate twice.
///
/// The corollary, learned here: a diff cannot see the switcher's panel *open*, because opening it
/// moves nothing. What does move it is giving the switcher another project to list.
fn only_panel_that_changed(
    before: &[LayoutRecord],
    after: &[LayoutRecord],
    expected: &str,
    nothing_changed: &str,
) -> LayoutRecord {
    let existing: Vec<&LayoutRecord> = anchored_panels(before).collect();
    let opened: Vec<&LayoutRecord> = anchored_panels(after)
        .filter(|panel| {
            !existing.iter().any(|had| {
                had.path == panel.path
                    && (had.x - panel.x).abs() < TOLERANCE
                    && (had.y - panel.y).abs() < TOLERANCE
                    && (had.width - panel.width).abs() < TOLERANCE
                    && (had.height - panel.height).abs() < TOLERANCE
            })
        })
        .collect();

    assert_eq!(
        opened.len(),
        1,
        "{expected}; {} panel(s) changed. {}\n  before: {}\n  after:  {}",
        opened.len(),
        if opened.is_empty() {
            nothing_changed
        } else {
            "More than one moved, so the difference no longer names one panel."
        },
        describe(&existing),
        describe(&anchored_panels(after).collect::<Vec<_>>()),
    );
    opened[0].clone()
}

/// The panel a right-press opened.
fn menu_that_opened(before: &[LayoutRecord], after: &[LayoutRecord]) -> LayoutRecord {
    only_panel_that_changed(
        before,
        after,
        "the press should have opened exactly one panel",
        "None appeared, so the right-press reached no handler at all — which is a different \
         defect from opening in the wrong place, and this gate would otherwise report it as the \
         same one.",
    )
}

/// Every panel, as `path @ x,y size w×h` — what a failure needs in order to be actionable.
fn describe(panels: &[&LayoutRecord]) -> String {
    if panels.is_empty() {
        return "(none)".to_string();
    }
    panels
        .iter()
        .map(|p| {
            format!(
                "{} @ {:.0},{:.0} {:.0}×{:.0}",
                lay::path_token(&p.path),
                p.x,
                p.y,
                p.width,
                p.height
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// The shared body: right-click the element at `path`, and the menu it opens begins at the press.
///
/// `menu` names the surface in the failure, since a failure that says only "a panel" leaves the
/// reader to work out which of the four this was.
fn assert_menu_opens_at_the_press(state: State, path: &[usize], menu: &str) {
    let point = centre_of(&records(&state), path, menu);
    assert_menu_opens_at(state, point, menu);
}

/// As above, for a surface located by geometry rather than by path.
fn assert_menu_opens_at(mut state: State, point: (f32, f32), menu: &str) {
    let before = records(&state);
    let applied = right_press_at(&mut state, point);
    assert!(
        applied > 0,
        "a secondary press at ({:.1}, {:.1}) over {menu} published nothing, so the row does not \
         answer a right-click at all",
        point.0,
        point.1,
    );

    let after = records(&state);
    let panel = menu_that_opened(&before, &after);

    let (expected, clamped) = expected_origin(point, &panel);
    let dx = panel.x - expected.0;
    let dy = panel.y - expected.1;
    assert!(
        dx.abs() < TOLERANCE && dy.abs() < TOLERANCE,
        "{menu}: right-clicked at ({:.1}, {:.1}) and its menu opened at ({:.1}, {:.1}), where it \
         belongs at ({:.1}, {:.1}){} — {:.0}px away. A context menu opens at the press point that \
         opened it, and the press point must be carried by the gesture rather than replaced by a \
         constant (FR-029d). A menu positioned by anything else cannot say which element it acts \
         on, which for a menu offering Delete and Remove is what BUG-008 was.",
        point.0,
        point.1,
        panel.x,
        panel.y,
        expected.0,
        expected.1,
        if clamped {
            ", the press being close enough to an edge that the panel is slid back inside"
        } else {
            ""
        },
        (dx * dx + dy * dy).sqrt(),
    );
}

/// Where a panel of this size, opened at this press, belongs — and whether the window moved it.
///
/// Two corrections, both the application's own and neither of them slack in the assertion:
///
/// - The press point is `f32` at the widget and `u16` on the message, so a press at y=641.2 asks
///   for 641. Comparing against the float would fail by a fifth of a pixel and say nothing.
/// - A press within a panel's height of an edge is **clamped** back inside (FR-029d's second half,
///   feature 015's FR-006). Asserting the raw press point would forbid the clamp, which is the
///   behaviour the other half of this bug's requirement asks for. The panel's *measured* size is
///   what it is clamped against, so nothing about the panel's dimensions is restated here.
fn expected_origin(point: (f32, f32), panel: &LayoutRecord) -> ((f32, f32), bool) {
    let asked = (point.0 as u16, point.1 as u16);
    let clamped = micold_client::features::project::clamp_menu_anchor(
        asked,
        (panel.width as u16, panel.height as u16),
        (lay::WINDOW.width as u16, lay::WINDOW.height as u16),
    );
    ((clamped.0 as f32, clamped.1 as f32), clamped != asked)
}

// --- The four context menus ---------------------------------------------------------------------

/// The path to a sidebar row, by index among the tree's rows.
///
/// The sidebar's list is the third child of the sidebar column (header, filter accordion, body);
/// the body holds the scrollable whose content column holds one element per row.
///
/// **Row 0 is the "Default" project-root row** (`sidebar_entries`' first entry), which carries no
/// context menu — the worktrees start at 1. Getting this wrong reports "the press reached no
/// handler" rather than a wrong anchor, which is why those are two different failures here.
fn sidebar_row(index: usize) -> Vec<usize> {
    vec![0, 0, 1, 0, 0, 0, 2, 0, 0, index]
}

/// The last worktree row: past the Default row, past the other seven.
const LAST_WORKTREE_ROW: usize = WORKTREE_COUNT;
/// The first worktree row, immediately after the Default row.
const FIRST_WORKTREE_ROW: usize = 1;

/// A worktree row near the **bottom** of a long list (FR-029d, SC-008f).
#[test]
fn the_worktree_menu_opens_at_the_row_it_was_opened_from() {
    let state = with_project(Vec::new());
    assert_menu_opens_at_the_press(
        state,
        &sidebar_row(LAST_WORKTREE_ROW),
        "the last worktree row's context menu",
    );
}

/// The same row's menu **moves** when a different row is right-clicked (FR-029d's second clause).
///
/// The clause a fixed anchor could never fail: two presses, two answers, and the answers must
/// differ by what the presses differ by.
#[test]
fn the_worktree_menu_moves_to_the_next_row_right_clicked() {
    let mut state = with_project(Vec::new());

    let before = records(&state);
    let first = centre_of(
        &before,
        &sidebar_row(FIRST_WORKTREE_ROW),
        "the first worktree row",
    );
    right_press_at(&mut state, first);
    let at_first = menu_that_opened(&before, &records(&state));

    let last_point = centre_of(
        &before,
        &sidebar_row(LAST_WORKTREE_ROW),
        "the last worktree row",
    );
    right_press_at(&mut state, last_point);
    let at_last = menu_that_opened(&before, &records(&state));

    let moved = at_last.y - at_first.y;
    let expected =
        expected_origin(last_point, &at_last).0 .1 - expected_origin(first, &at_first).0 .1;
    assert!(
        (moved - expected).abs() < TOLERANCE,
        "right-clicking a different row must re-anchor the menu (FR-029d): the panel should have \
         moved {expected:.0}px between the two presses and moved {moved:.0}px. A menu that stays \
         put belongs to whichever row the user last looked at rather than to the one they pressed.",
    );
}

/// A session row's menu — the surface the bug was reported from (FR-029d, SC-008f).
#[test]
fn the_session_menu_opens_at_the_row_it_was_opened_from() {
    let session = Session::restored(
        SessionId::new(),
        SessionLocation::Worktree("feat-00".to_string()),
        SessionLabel::Named("feat/00".to_string()),
        TerminalMode::AiCli,
        AiCli::ClaudeCode,
    );
    let mut state = with_project(vec![session]);
    state.expanded.insert("feat-00".to_string());

    // Default row, then `feat-00`, then its session.
    assert_menu_opens_at_the_press(state, &sidebar_row(2), "a session row's context menu");
}

/// The switcher's project row — correct since feature 015, and asserted here so that the rule is
/// held for the **kind** rather than only for the two surfaces that were broken.
#[test]
fn the_project_menu_opens_at_the_row_it_was_opened_from() {
    // Locate the switcher's panel by what changes it: a second project to list. Opening it changes
    // nothing a layout can see, and its layer index is arithmetic `covered_states.rs` has had to
    // re-point twice already — a third copy here would be a third thing to re-point.
    let one = with_projects(1);
    let mut two = with_projects(2);
    let panel = only_panel_that_changed(
        &records(&one),
        &records(&two),
        "listing a second project should have changed exactly one panel — the switcher's",
        "Nothing changed, so the switcher's panel is not sized by what it lists and this gate can \
         no longer find it.",
    );
    two.project_switcher_open = true;

    // Inside the panel's first row: across the panel it was just measured at, and down past §7.5's
    // vertical padding into the middle of the item. The figures are the measured panel's own and
    // the contract's own tokens — nothing about the row's position is restated here.
    let point = (
        panel.x + panel.width / 2.0,
        panel.y + anatomy::menu::VERTICAL_PADDING + density::MENU_ITEM_BASE / 2.0,
    );
    assert_menu_opens_at(two, point, "a switcher project row's context menu");
}
