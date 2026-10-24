//! The session area's context header and tab strip fit (feature 1126, T011, T028, contracts H10,
//! S9, A1, A2, FR-013, SC-004, SC-005).
//!
//! Every covered state that shows a session is laid out at the smallest window the application
//! allows beside a 300 px sidebar, and at the layout window, in light and in dark: the header's
//! title and each of its chips, and the strip's scrolling region, status, New session and split
//! buttons, lie inside the session area and inside the window, and none of them overlaps another.
//! The strip states hold 1, 6 and 12 tabs. Top to bottom the area is header, strip, pane area, and
//! nothing is laid out under the pane area (A1); with two panes the header and the strip are each
//! laid out once, above the pane area (A2). A stressed variant shows the longest worktree name with
//! a pull request, so the title and the chips have to give way with an ellipsis rather than push
//! each other out.
//!
//! Compiled into the `layout_snapshot` binary beside the other geometry gates.

use crate::support::covered_states::covered_states;
use crate::support::layout::{self as lay, LayoutRecord, StateUnderTest, WINDOW};
use iced::advanced::widget::Tree;
use iced::advanced::{layout, Layout};
use iced::{Rectangle, Size};
use micold_client::app::{State, MIN_WINDOW_SIZE};
use micold_core::pull_request::{CheckStatus, PrState, PullRequestStatus, ReviewState};
use micold_core::session::SessionLocation;
use micold_core::theme::ThemePreference;

/// Half a pixel, matching every geometry gate beside this one.
const TOLERANCE: f32 = 0.5;

/// The session area: the parent of the anchor `session.header` (`SESSION_HEADER`).
const SESSION_AREA: &[usize] = &[0, 0, 1, 1];

/// The covered states that show a session, and so the header.
const SESSION_STATES: &[&str] = &[
    "session-area-one-tab",
    "session-transcript-pane",
    "session-transcript-rich",
    "session-transcript-insights",
    "session-area-six-tabs",
    "session-area-twelve-tabs",
    "status-bar-split-panes",
];

/// The sidebar width H10 measures the smallest window beside.
const SIDEBAR: u16 = 300;

fn rect(record: &LayoutRecord) -> Rectangle {
    Rectangle {
        x: record.x,
        y: record.y,
        width: record.width,
        height: record.height,
    }
}

/// The base tree of `state` laid out in a window `window` large, single pane.
fn records_at(state: State, window: Size) -> Vec<LayoutRecord> {
    records_of(StateUnderTest::new(state), window)
}

/// The base tree of `under` laid out in a window `window` large.
fn records_of(under: StateUnderTest, window: Size) -> Vec<LayoutRecord> {
    let renderer = lay::renderer();
    let mut element = lay::view_of(&under);
    let mut tree = Tree::new(element.as_widget());
    let limits = layout::Limits::new(Size::ZERO, window);
    let node = element
        .as_widget_mut()
        .layout(&mut tree, &renderer, &limits);
    lay::walk(Layout::new(&node), lay::Layer::Base)
}

/// The context header's height in `state` at the layout window: chrome above the pane area that
/// other gates measuring the pane leave out of the pane's room.
pub(crate) fn header_height(state: State) -> f32 {
    let records = records_at(state, WINDOW);
    let header = [SESSION_AREA, &[0]].concat();
    at(&records, &header).map_or(0.0, |r| r.height)
}

/// The record at `path`.
fn at<'r>(records: &'r [LayoutRecord], path: &[usize]) -> Option<&'r LayoutRecord> {
    records.iter().find(|r| r.path == path)
}

/// The header's Changes button: the title line's second child (H9).
const CHANGES_BUTTON: &[usize] = &[0, 0, 0, 1];

/// The header's title and chips in `records`: the title line, each chip of the chips row, then
/// the Changes button.
fn header_parts(records: &[LayoutRecord]) -> Vec<(String, Rectangle)> {
    let header = [SESSION_AREA, &[0]].concat();
    let title = [header.as_slice(), &[0, 0, 0]].concat();
    let chips = [header.as_slice(), &[0, 1]].concat();
    let button = [SESSION_AREA, CHANGES_BUTTON].concat();
    let mut parts = vec![
        (
            "title".to_string(),
            rect(at(records, &title).expect("the header draws its title")),
        ),
        (
            "Changes button".to_string(),
            rect(at(records, &button).expect("the header draws its Changes button")),
        ),
    ];
    parts.extend(
        records
            .iter()
            .filter(|r| r.path.len() == chips.len() + 1 && r.path.starts_with(&chips))
            .map(|r| (format!("chip {}", r.path[chips.len()]), rect(r))),
    );
    parts
}

fn inside(part: &Rectangle, host: &Rectangle) -> bool {
    part.x >= host.x - TOLERANCE
        && part.y >= host.y - TOLERANCE
        && part.x + part.width <= host.x + host.width + TOLERANCE
        && part.y + part.height <= host.y + host.height + TOLERANCE
}

fn overlap(a: &Rectangle, b: &Rectangle) -> bool {
    a.x + TOLERANCE < b.x + b.width
        && b.x + TOLERANCE < a.x + a.width
        && a.y + TOLERANCE < b.y + b.height
        && b.y + TOLERANCE < a.y + a.height
}

/// The strip's own row's children: its scrolling region, status cluster, New session and splits
/// (S7). The members inside the region scroll, so they are not parts that must fit the window.
fn strip_parts(records: &[LayoutRecord]) -> Vec<(String, Rectangle)> {
    let row = [SESSION_AREA, &[1, 0]].concat();
    const NAMES: [&str; 4] = ["tab region", "status", "New session", "split buttons"];
    records
        .iter()
        .filter(|r| r.path.len() == row.len() + 1 && r.path.starts_with(&row))
        .map(|r| (format!("strip {}", NAMES[r.path[row.len()]]), rect(r)))
        .collect()
}

/// A1, A2: the session area holds the header, the strip and the pane area, in that order, each
/// once, and nothing under the pane area.
fn assert_order(name: &str, records: &[LayoutRecord], area: &Rectangle) {
    let children: Vec<&LayoutRecord> = records
        .iter()
        .filter(|r| r.path.len() == SESSION_AREA.len() + 1 && r.path.starts_with(SESSION_AREA))
        .collect();
    assert_eq!(
        children.len(),
        3,
        "{name}: the session area is header, strip and pane area, each laid out once (A1, A2)"
    );
    let [header, strip, panes] = [children[0], children[1], children[2]].map(rect);
    assert!(
        header.y + header.height <= strip.y + TOLERANCE
            && strip.y + strip.height <= panes.y + TOLERANCE,
        "{name}: header {header:?}, strip {strip:?} and pane area {panes:?} are not top to bottom"
    );
    assert!(
        (panes.y + panes.height - (area.y + area.height)).abs() <= TOLERANCE,
        "{name}: something is laid out under the pane area {panes:?} in the session area \
         {area:?} (SC-005)"
    );
}

/// H10 and S9 for `under` named `name`, in both schemes, at both windows.
fn assert_fits(name: &str, state: &State) {
    assert_fits_under(name, || StateUnderTest::new(state.clone()));
}

/// The width the header's Changes button asks for in `state`, laid out on its own with no bound.
fn changes_button_request(state: &State) -> f32 {
    let location = state
        .current_session_location()
        .expect("a session is shown");
    let label = micold_core::context_header::changes_label(
        micold_client::ui::session_area::counts_of(state, &location),
    );
    let roles = micold_core::tokens::roles(state.color_scheme());
    let mut element =
        micold_client::ui::session_area::changes_button(state, &location, label, roles);
    let renderer = lay::renderer();
    let mut tree = Tree::new(element.as_widget());
    let limits = layout::Limits::new(Size::ZERO, Size::INFINITE);
    element
        .as_widget_mut()
        .layout(&mut tree, &renderer, &limits)
        .size()
        .width
}

/// H10: the Changes button is laid out at the width it asks for, never squeezed by the title.
fn assert_button_unsqueezed(what: &str, records: &[LayoutRecord], state: &State) {
    let button = [SESSION_AREA, CHANGES_BUTTON].concat();
    let laid = at(records, &button).expect("the header draws its Changes button");
    let wanted = changes_button_request(state);
    assert!(
        (laid.width - wanted).abs() <= TOLERANCE,
        "{what}: the Changes button is laid out {} wide, it asks for {wanted} (H10)",
        laid.width
    );
}

fn assert_fits_under(name: &str, build: impl Fn() -> StateUnderTest) {
    for theme in [ThemePreference::Light, ThemePreference::Dark] {
        for window in [MIN_WINDOW_SIZE, WINDOW] {
            let mut under = build();
            // The application always passes its pane area; a state that names none is one pane.
            under.panes = under
                .panes
                .or_else(|| Some(micold_core::pane_layout::PaneLayout::single()));
            under.state.settings.theme_pref = theme;
            under.state.sidebar.width = SIDEBAR;
            let state = under.state.clone();
            let records = records_of(under, window);
            assert_button_unsqueezed(&format!("{name} {theme:?} {window:?}"), &records, &state);
            let area = rect(at(&records, SESSION_AREA).expect("the session area is drawn"));
            let screen = Rectangle::with_size(window);
            assert_order(name, &records, &area);
            let mut parts = header_parts(&records);
            assert!(
                parts.len() >= 2,
                "{name} {theme:?} {window:?}: the title and at least the state chip: {parts:?}"
            );
            let strip = strip_parts(&records);
            assert_eq!(
                strip.len(),
                4,
                "{name} {theme:?} {window:?}: the strip's four children: {strip:?}"
            );
            parts.extend(strip);
            for (what, part) in &parts {
                assert!(
                    part.width > 0.0 && inside(part, &area) && inside(part, &screen),
                    "{name} {theme:?} {window:?}: the {what} {part:?} lies inside the session \
                     area {area:?} and the window"
                );
            }
            for (i, (a_name, a)) in parts.iter().enumerate() {
                for (b_name, b) in &parts[i + 1..] {
                    assert!(
                        !overlap(a, b),
                        "{name} {theme:?} {window:?}: the {a_name} {a:?} overlaps the {b_name} {b:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn the_header_and_the_strip_fit_in_every_session_state_at_both_windows() {
    for name in SESSION_STATES {
        let covered = covered_states()
            .iter()
            .find(|s| s.name == *name)
            .unwrap_or_else(|| panic!("the covered state {name} exists"));
        assert_fits_under(name, covered.build);
    }
}

/// The longest worktree name with a pull request whose chip is long too: title and chips give way
/// with an ellipsis and still fit (FR-013).
#[test]
fn a_long_title_and_a_pull_request_chip_still_fit() {
    let covered = covered_states()
        .iter()
        .find(|s| s.name == "session-area-one-tab")
        .expect("the covered state exists");
    let mut state = (covered.build)().state;
    let long = state
        .worktree
        .worktrees
        .iter()
        .max_by_key(|w| w.dir_name.len())
        .expect("the fixture has worktrees")
        .clone();
    for session in state.workspace.sessions.values_mut().flatten() {
        session.location = SessionLocation::Worktree(long.dir_name.clone());
    }
    state.pr_status.enabled = true;
    state.pr_status.statuses.insert(
        long.branch.clone().expect("the long worktree has a branch"),
        PullRequestStatus {
            number: 123_456,
            title: "A pull request".into(),
            url: "https://github.com/o/r/pull/123456".into(),
            state: PrState::Draft {
                checks: CheckStatus::Pending,
            },
            review: ReviewState::ChangesRequested,
            head: "abc".into(),
            conflict: false,
        },
    );
    // A branch's worth of counts, so the button asks for more than `Changes` alone. The title line
    // has about 286 px at the smallest window beside the 300 px sidebar: counts in the thousands
    // would wrap the button's label there (recorded in the ledger's follow-ups).
    let location = state
        .current_session_location()
        .expect("a session is shown");
    state.changes.header.entry = Some((state.workspace.active.clone().unwrap(), location));
    state.changes.header.load =
        micold_client::features::changes::Load::Ready(micold_core::context_header::ChangeSummary {
            files: 12,
            added: 345,
            removed: 67,
        });
    let parts = header_parts(&records_at(state.clone(), WINDOW));
    assert_eq!(
        parts.len(),
        5,
        "title, Changes button, state, branch and pull request chips: {parts:?}"
    );
    assert_fits("long title with a pull request", &state);
}

/// T053 (R12, A1, H10): with the Changes view open the session area is the header, then the view;
/// the header's title, chips and Changes button fit at both windows and themes, the button at the
/// width it asks for.
#[test]
fn the_header_stands_over_the_open_changes_view() {
    let covered = covered_states()
        .iter()
        .find(|s| s.name == "session-area-changes-open")
        .expect("the covered state exists");
    for theme in [ThemePreference::Light, ThemePreference::Dark] {
        for window in [MIN_WINDOW_SIZE, WINDOW] {
            let mut under = (covered.build)();
            under.state.settings.theme_pref = theme;
            under.state.sidebar.width = SIDEBAR;
            let state = under.state.clone();
            let records = records_of(under, window);
            let what = format!("changes open {theme:?} {window:?}");
            let area = rect(at(&records, SESSION_AREA).expect("the session area is drawn"));
            let children: Vec<Rectangle> = records
                .iter()
                .filter(|r| {
                    r.path.len() == SESSION_AREA.len() + 1 && r.path.starts_with(SESSION_AREA)
                })
                .map(rect)
                .collect();
            assert_eq!(
                children.len(),
                2,
                "{what}: the header, then the view: {children:?}"
            );
            assert!(
                children[0].y + children[0].height <= children[1].y + TOLERANCE,
                "{what}: the header {:?} is above the view {:?}",
                children[0],
                children[1]
            );
            let parts = header_parts(&records);
            for (part, r) in &parts {
                assert!(
                    r.width > 0.0 && inside(r, &area),
                    "{what}: the {part} {r:?} lies inside the session area {area:?}"
                );
            }
            for (i, (a_name, a)) in parts.iter().enumerate() {
                for (b_name, b) in &parts[i + 1..] {
                    assert!(
                        !overlap(a, b),
                        "{what}: the {a_name} {a:?} overlaps the {b_name} {b:?}"
                    );
                }
            }
            assert_button_unsqueezed(&what, &records, &state);
        }
    }
}
