//! Every known project stays reachable when there are more of them than the window can show
//! (feature 002, BUG-005 — FR-011a, SC-011).
//!
//! Both surfaces that list the catalog were bare `column`s: the shell body's "Known projects" list
//! inside a `Fill` container, and the top-bar switcher's rows inside `MenuOverlay`'s panel with no
//! height cap. A row past the window's bottom edge was laid out, clipped, and never painted, and
//! nothing scrolled — so at the default 1280×800 window the body showed four of twenty projects and
//! the switcher fifteen, with no "Add project…" at all.
//!
//! These paint the real view with twenty projects, turn the mouse wheel over each list the way a
//! person would, and read what was drawn: the last project — and in the switcher the trailing "Add
//! project…" row — must then be on screen.

mod support;

use iced::{Element, Point};
use micold_client::app::{Message, State};
use micold_client::features::connection::ConnectionStatus;
use micold_client::features::project::{self, Msg as ProjectMsg};
use micold_client::features::sandbox::Sandbox;
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::tokens::{anatomy, density};
use support::layout as lay;

/// More projects than either list can show at the default window: the body fits four rows under
/// its empty-state header, the switcher fifteen below the app bar.
const MANY: usize = 20;
/// A catalog that fits in both lists.
const FEW: usize = 3;

/// The switcher panel's width and its inset from the window's trailing edge (`menu.rs`
/// `PANEL_WIDTH`, and `spacing::SM` for the `TopEnd` anchor).
const PANEL_WIDTH: f32 = 240.0;
const PANEL_END_INSET: f32 = micold_core::tokens::spacing::SM;

/// A line of text needs this much room above the window's bottom edge to be read, not glimpsed.
const LINE: f32 = 16.0;

fn panel_left() -> f32 {
    lay::WINDOW.width - PANEL_END_INSET - PANEL_WIDTH
}

fn state(projects: usize, switcher_open: bool) -> State {
    let paths: Vec<String> = (0..projects)
        .map(|i| format!("/fixture/proj-{i:02}"))
        .collect();
    let workspace = support::workspace_with(paths.iter().map(|p| (p.as_str(), vec![])).collect());
    State {
        workspace,
        project: project::State {
            switcher_open,
            ..Default::default()
        },
        ..State::default()
    }
}

fn view(state: &State) -> Element<'_, Message> {
    micold_client::ui::view(
        state,
        None,
        None,
        0,
        None,
        &EnvIncludeOutcome::Disabled,
        &ConnectionStatus::Connected,
        &Sandbox::default(),
    )
}

fn last_name(state: &State) -> String {
    state
        .workspace
        .projects
        .last()
        .expect("the fixture has projects")
        .display_name
        .clone()
}

/// The open switcher panel's box: the outermost node exactly [`PANEL_WIDTH`] wide at the panel's
/// trailing-edge position, and of those the one stacked last.
///
/// The closed overflow menu hangs from the same edge at the same width — it is laid out while
/// closed, because it fades out rather than vanishing — and `ui::view` pushes it onto the overlay
/// just before the switcher. Both are popovers, so the switcher is the later layer; no other
/// surface is open in these states.
fn panel_box(state: &State) -> Option<lay::LayoutRecord> {
    let panels: Vec<lay::LayoutRecord> = lay::resolve(view(state), &lay::renderer())
        .into_iter()
        .filter(|r| (r.width - PANEL_WIDTH).abs() < 0.5 && (r.x - panel_left()).abs() < 0.5)
        .collect();
    let outermost = panels.iter().map(|r| r.path.len()).min()?;
    panels
        .into_iter()
        .filter(|r| r.path.len() == outermost)
        .max_by(|a, b| a.path.cmp(&b.path))
}

fn on_screen(t: &lay::Overflow) -> bool {
    t.on_screen.y >= 0.0 && t.on_screen.y + LINE <= lay::WINDOW.height
}

fn contents(painted: &[lay::Overflow]) -> Vec<(&str, f32, f32)> {
    painted
        .iter()
        .map(|t| (t.content.as_str(), t.on_screen.x, t.on_screen.y))
        .collect()
}

#[test]
fn the_body_list_scrolls_to_its_last_project_and_its_actions_under_a_fixed_header() {
    let state = state(MANY, false);
    let last = last_name(&state);
    let mut renderer = lay::renderer();

    let at_rest = lay::painted_text_settled(view(&state), &mut renderer);
    // Over the list, low in the window: where the rows below the fold would be.
    let wheel_at = Point::new(lay::WINDOW.width / 2.0, lay::WINDOW.height - 100.0);
    let scrolled = lay::painted_text_scrolled(view(&state), &mut renderer, wheel_at);

    let name = scrolled
        .iter()
        .find(|t| t.content == last && on_screen(t))
        .unwrap_or_else(|| {
            panic!(
                "after scrolling the known-projects list to its end, the last project {last:?} \
                 must be painted inside the window — a row nobody can scroll to is a project \
                 nobody can reopen (FR-011a); painted: {:?}",
                contents(&scrolled)
            )
        });
    // Its row's actions, on the same line as its name at this width (one row is 80dp tall).
    for action in ["Open", "Rename", "Forget"] {
        assert!(
            scrolled.iter().any(|t| t.content == action
                && on_screen(t)
                && (t.on_screen.y - name.on_screen.y).abs() < 40.0),
            "the last project's {action:?} must be on screen with it after scrolling — reopen, \
             rename and forget are all reached from the row (FR-011a); painted: {:?}",
            contents(&scrolled)
        );
    }

    let header = |painted: &[lay::Overflow]| {
        painted
            .iter()
            .find(|t| t.content == "No project open")
            .map(|t| t.on_screen)
    };
    let before = header(&at_rest).expect("the empty-state header is painted at rest");
    assert_eq!(
        header(&scrolled),
        Some(before),
        "the header above the list stays in place while the list scrolls (FR-011a)"
    );
}

#[test]
fn the_switcher_panel_stays_in_the_window_and_scrolls_to_add_project() {
    let state = state(MANY, true);
    let last = last_name(&state);
    let mut renderer = lay::renderer();

    // The panel's own box: the outermost node exactly its width at its trailing-edge position.
    // Outermost, because the rows' column inside a scrolling panel is as tall as all the rows.
    //
    // The bound alone held on `origin/main` too — the panel's node was clamped to its container and
    // the rows overflowed *inside* it — so the scroll below is what tells the fix apart. The inset
    // is the fix's own: a panel that meets the window's edge loses its rounded corners and its
    // shadow there, so it stops as far short of the bottom edge as it hangs from the trailing one.
    let panel = panel_box(&state)
        .map(|r| r.y + r.height)
        .expect("the open switcher panel is laid out by the trailing edge");
    assert!(
        panel <= lay::WINDOW.height - PANEL_END_INSET + 0.5,
        "the switcher panel's bottom edge is at {panel:.1}; it must stop {PANEL_END_INSET}dp short \
         of the window's {} so the rows below are never cut off and the panel keeps its corners \
         (FR-011a)",
        lay::WINDOW.height
    );

    let wheel_at = Point::new(panel_left() + PANEL_WIDTH / 2.0, lay::WINDOW.height - 100.0);
    let scrolled = lay::painted_text_scrolled(view(&state), &mut renderer, wheel_at);
    let in_panel = |t: &&lay::Overflow| t.on_screen.x >= panel_left() - 0.5 && on_screen(t);
    for wanted in [last.as_str(), "Add project…"] {
        assert!(
            scrolled
                .iter()
                .filter(in_panel)
                .any(|t| t.content == wanted),
            "after scrolling the switcher panel to its end, {wanted:?} must be painted inside the \
             window (FR-011a, 008 FR-009); painted: {:?}",
            contents(&scrolled)
        );
    }
}

#[test]
fn a_switcher_panel_that_fits_keeps_the_height_it_always_had() {
    let state = state(FEW, true);
    let tallest = panel_box(&state)
        .map(|r| r.height)
        .expect("the open switcher panel is laid out by the trailing edge");
    // The projects plus the trailing "Add project…" row, in `menu_panel_size`'s own terms: §7.5's
    // padding above the first item and below the last, and one item height per row.
    let estimate =
        anatomy::menu::VERTICAL_PADDING * 2.0 + (FEW + 1) as f32 * density::MENU_ITEM_BASE;
    assert!(
        (tallest - estimate).abs() < 0.5,
        "a switcher panel that fits must stay exactly {estimate}dp tall — `menu_panel_size`'s \
         estimate, which context-menu clamping relies on — not {tallest:.1}dp (FR-011a)"
    );
}

/// A right-click on a switcher row reports where it landed **in the window**, however far the
/// panel is scrolled (feature 015's contract: the Forget menu opens at the click point).
///
/// A scrollable hands its content the cursor in *content* coordinates — shifted by the offset — so
/// a row that reads its press point from the cursor it was given reports a point the scrolled
/// distance below the click, and the menu opened at the window's bottom edge instead of at the row.
#[test]
fn a_right_click_on_a_scrolled_switcher_row_reports_where_it_landed() {
    let state = state(MANY, true);
    let last = last_name(&state);
    let wheel_at = Point::new(panel_left() + PANEL_WIDTH / 2.0, lay::WINDOW.height - 100.0);
    let scrolled = lay::painted_text_scrolled(view(&state), &mut lay::renderer(), wheel_at);
    let row = scrolled
        .iter()
        .find(|t| t.content == last && t.on_screen.x >= panel_left() - 0.5 && on_screen(t))
        .expect("the last project is on screen once the switcher is scrolled to its end");
    let click = Point::new(row.on_screen.x + 4.0, row.on_screen.y + 4.0);

    let published = lay::messages_after(
        view(&state),
        &lay::renderer(),
        &[
            lay::Input::Wheel(wheel_at),
            lay::Input::Move(click),
            lay::Input::Press(click, iced::mouse::Button::Right),
        ],
    );
    let anchors: Vec<(u16, u16)> = published
        .iter()
        .filter_map(|m| match m {
            Message::Project(ProjectMsg::MenuToggled(path, point)) if path.ends_with(&last) => {
                Some(*point)
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        anchors,
        vec![(click.x as u16, click.y as u16)],
        "a right-click on {last:?} in the scrolled switcher must open its menu at the click point, \
         in window pixels — not the point in the scrolled content (feature 015, FR-011a); \
         published: {published:?}"
    );
}

/// Scrolling the body's list is the ground moving under whatever floats over it, so it closes the
/// transient popovers — as scrolling the sidebar does (feature 017 FR-009). The switcher is one.
#[test]
fn scrolling_the_body_list_closes_the_switcher_floating_over_it() {
    let mut state = state(MANY, true);
    // Over the body's rows, clear of the switcher panel at the trailing edge.
    let wheel_at = Point::new(lay::WINDOW.width / 4.0, lay::WINDOW.height - 100.0);
    let published = lay::messages_after(
        view(&state),
        &lay::renderer(),
        &[lay::Input::Wheel(wheel_at)],
    );
    assert!(
        !published.is_empty(),
        "turning the wheel over the known-projects list must report the scroll"
    );
    for message in published {
        state.update(message);
    }
    assert!(
        !state.project.switcher_open,
        "the switcher must close when the list beneath it scrolls (017 FR-009)"
    );
}

/// Scrolling the switcher moves its rows out from under a row's context menu, so that menu closes —
/// and the switcher itself, which is what is being scrolled, stays open.
#[test]
fn scrolling_the_switcher_closes_a_rows_context_menu_and_keeps_the_switcher() {
    let mut state = state(MANY, true);
    let first = state.workspace.projects[0].path.clone();
    state.update(Message::Project(ProjectMsg::MenuToggled(first, (100, 100))));
    assert!(
        state.project.menu_open.is_some(),
        "the fixture opened the row's menu"
    );

    let wheel_at = Point::new(panel_left() + PANEL_WIDTH / 2.0, lay::WINDOW.height - 100.0);
    let published = lay::messages_after(
        view(&state),
        &lay::renderer(),
        &[lay::Input::Wheel(wheel_at)],
    );
    for message in published {
        state.update(message);
    }
    assert!(
        state.project.menu_open.is_none(),
        "a row's context menu must close when the switcher scrolls its row away (017 FR-009)"
    );
    assert!(
        state.project.switcher_open,
        "scrolling the switcher must not close the switcher"
    );
}
