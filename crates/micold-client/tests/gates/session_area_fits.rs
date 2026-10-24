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

/// The header's title and chips in `records`: the title line, then each chip of the chips row.
fn header_parts(records: &[LayoutRecord]) -> Vec<(String, Rectangle)> {
    let header = [SESSION_AREA, &[0]].concat();
    let title = [header.as_slice(), &[0, 0]].concat();
    let chips = [header.as_slice(), &[0, 1]].concat();
    let mut parts = vec![(
        "title".to_string(),
        rect(at(records, &title).expect("the header draws its title")),
    )];
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
            let records = records_of(under, window);
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
    let parts = header_parts(&records_at(state.clone(), WINDOW));
    assert_eq!(
        parts.len(),
        4,
        "title, state, branch and pull request chips: {parts:?}"
    );
    assert_fits("long title with a pull request", &state);
}
