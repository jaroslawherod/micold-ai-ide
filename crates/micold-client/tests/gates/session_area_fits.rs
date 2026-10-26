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

use crate::support::covered_states::{covered_states, review_source, SourcePose, LIST_READ_AT};
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
    "session-area-review-hidden",
];

/// A branch's worth of counts: more than `Changes` alone, within H10's three-digit bound.
const A_BRANCHS_COUNTS: micold_core::context_header::ChangeSummary =
    micold_core::context_header::ChangeSummary {
        files: 12,
        added: 345,
        removed: 67,
    };

/// The widest counts a branch ever shows: five-digit files and six-digit lines (M3 review B F2).
const THE_WIDEST_COUNTS: micold_core::context_header::ChangeSummary =
    micold_core::context_header::ChangeSummary {
        files: 12_345,
        added: 987_654,
        removed: 876_543,
    };

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
    at(&records, &header)
        .expect("the session area draws its header")
        .height
}

/// The record at `path`.
fn at<'r>(records: &'r [LayoutRecord], path: &[usize]) -> Option<&'r LayoutRecord> {
    records.iter().find(|r| r.path == path)
}

/// The header's Changes button: the header line's third child (H9, feature 1463).
const CHANGES_BUTTON: &[usize] = &[0, 0, 2];

/// The header's title and chips in `records`: the title, each chip of the chips row, then the
/// Changes button.
fn header_parts(records: &[LayoutRecord]) -> Vec<(String, Rectangle)> {
    let header = [SESSION_AREA, &[0]].concat();
    let title = [header.as_slice(), &[0, 0]].concat();
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
    let location = state.shown_location().expect("a location is shown");
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

/// The title is at its minimum below this width: little more than its ellipsis (FR-002).
const TITLE_MINIMUM: f32 = 40.0;

/// H10' (feature 1463): the Changes button is laid out at the width it asks for while the title
/// has any room; once the title is gone the label is cut with an ellipsis and the button is no
/// wider than it asks for.
fn assert_button_unsqueezed(what: &str, records: &[LayoutRecord], state: &State) {
    let button = [SESSION_AREA, CHANGES_BUTTON].concat();
    let laid = at(records, &button).expect("the header draws its Changes button");
    let wanted = changes_button_request(state);
    let title = header_line(records)[0];
    if title.width >= TITLE_MINIMUM {
        assert!(
            (laid.width - wanted).abs() <= TOLERANCE,
            "{what}: the Changes button is laid out {} wide, it asks for {wanted} while the title \
             still has room (H10')",
            laid.width
        );
    } else {
        assert!(
            laid.width > 0.0 && laid.width <= wanted + TOLERANCE,
            "{what}: the cut Changes button is {} wide, it asks for {wanted} (H10')",
            laid.width
        );
    }
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
                    (part.width > 0.0 || what == "title")
                        && inside(part, &area)
                        && inside(part, &screen),
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

/// The header's line: its title, chips row and Changes button, in that order (FR-001, H9).
fn header_line(records: &[LayoutRecord]) -> [Rectangle; 3] {
    let header = [SESSION_AREA, &[0]].concat();
    [vec![0, 0], vec![0, 1], vec![0, 2]].map(|tail| {
        let path = [header.as_slice(), tail.as_slice()].concat();
        rect(at(records, &path).expect("the header line has three children"))
    })
}

/// FR-001, FR-003, SC-003: title, chips and Changes sit on one line in that order; the header is
/// about one line tall and the Changes button is no taller than the chips.
#[test]
fn the_header_is_one_line_with_a_compact_changes_button() {
    for name in SESSION_STATES {
        let covered = covered_states()
            .iter()
            .find(|s| s.name == *name)
            .unwrap_or_else(|| panic!("the covered state {name} exists"));
        for window in [MIN_WINDOW_SIZE, WINDOW] {
            let mut under = (covered.build)();
            under.state.sidebar.width = SIDEBAR;
            let records = records_of(under, window);
            let [title, chips, changes] = header_line(&records);
            let what = format!("{name} {window:?}");
            assert!(
                title.x + title.width <= chips.x + TOLERANCE
                    && chips.x + chips.width <= changes.x + TOLERANCE,
                "{what}: title {title:?}, chips {chips:?}, Changes {changes:?} are not left to right"
            );
            for part in [&title, &chips, &changes] {
                assert!(
                    part.y < title.y + title.height && part.y + part.height > title.y,
                    "{what}: {part:?} is not on the title's line {title:?}"
                );
            }
            let header = rect(at(&records, &[SESSION_AREA, &[0]].concat()).expect("the header"));
            assert!(
                header.height <= 40.0 + TOLERANCE,
                "{what}: the header is {} tall, about one line is 40 (SC-003)",
                header.height
            );
            assert!(
                changes.height <= title.height + TOLERANCE,
                "{what}: the Changes button {changes:?} is taller than the title line {title:?}, so \
                 it sets the header's height (FR-003)"
            );
        }
    }
}

/// FR-002, H10': at the smallest window the title shortens first, then the chips; the Changes
/// button keeps its requested width.
#[test]
fn the_title_shortens_before_the_chips_and_the_button() {
    let covered = covered_states()
        .iter()
        .find(|s| s.name == "session-area-one-tab")
        .expect("the covered state exists");
    for counts in [A_BRANCHS_COUNTS, THE_WIDEST_COUNTS] {
        let measure = |window: Size| {
            let mut under = (covered.build)();
            under.state.sidebar.width = SIDEBAR;
            let location = under
                .state
                .current_session_location()
                .expect("a session is shown");
            let project = under.state.workspace.active.clone().unwrap();
            under.state.changes.header.entry = Some((project, location));
            under.state.changes.header.load = micold_client::features::changes::Load::Ready(counts);
            let state = under.state.clone();
            let records = records_of(under, window);
            assert_button_unsqueezed(&format!("{counts:?} {window:?}"), &records, &state);
            header_line(&records)
        };
        let [wide_title, wide_chips, _] = measure(WINDOW);
        let [title, chips, changes] = measure(MIN_WINDOW_SIZE);
        assert!(
            title.width < wide_title.width,
            "{counts:?}: the title {title:?} gives way at the smallest window (was {wide_title:?})"
        );
        // The chips give way only once the title has nothing more to give.
        assert!(
            chips.width <= wide_chips.width + TOLERANCE
                && (chips.width >= wide_chips.width - TOLERANCE || title.width < TITLE_MINIMUM),
            "{counts:?}: the chips {chips:?} shortened (was {wide_chips:?}) while the title \
             {title:?} still had room (FR-002)"
        );
        assert!(
            changes.x + changes.width <= MIN_WINDOW_SIZE.width,
            "{counts:?}: the Changes button {changes:?} lies inside the window"
        );
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
    // A branch's worth of counts, so the button asks for more than `Changes` alone: H10's bound at
    // the smallest window (about 286 px on the title line beside the 300 px sidebar).
    let location = state
        .current_session_location()
        .expect("a session is shown");
    state.changes.header.entry = Some((state.workspace.active.clone().unwrap(), location));
    state.changes.header.load = micold_client::features::changes::Load::Ready(A_BRANCHS_COUNTS);
    let parts = header_parts(&records_at(state.clone(), WINDOW));
    assert_eq!(
        parts.len(),
        5,
        "title, Changes button, state, branch and pull request chips: {parts:?}"
    );
    assert_fits("long title with a pull request", &state);
}

/// T053 (R12, A1, H10), feature 1269 T037 (U16): with a review shown the session area is the
/// header, the strip, then the view; the header's title, chips and Changes button and the strip's
/// controls fit at both windows and themes, the button at the width it asks for. A sessionless
/// location's strip has no status words, so only its status may be empty.
#[test]
fn the_header_and_the_strip_stand_over_the_shown_review() {
    for name in [
        "session-area-review-shown",
        "session-area-review-sessionless",
        "changes-review-marks",
        "changes-review-marks-unavailable",
        "changes-review-held",
    ] {
        let covered = covered_states()
            .iter()
            .find(|s| s.name == name)
            .unwrap_or_else(|| panic!("the covered state {name} exists"));
        for theme in [ThemePreference::Light, ThemePreference::Dark] {
            for window in [MIN_WINDOW_SIZE, WINDOW] {
                let mut under = (covered.build)();
                under.state.settings.theme_pref = theme;
                under.state.sidebar.width = SIDEBAR;
                let state = under.state.clone();
                let records = records_of(under, window);
                let what = format!("{name} {theme:?} {window:?}");
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
                    3,
                    "{what}: the header, the strip, then the view: {children:?}"
                );
                assert!(
                    children[0].y + children[0].height <= children[1].y + TOLERANCE
                        && children[1].y + children[1].height <= children[2].y + TOLERANCE,
                    "{what}: header, strip and view are not top to bottom: {children:?}"
                );
                let mut parts = header_parts(&records);
                let strip = strip_parts(&records);
                assert_eq!(
                    strip.len(),
                    4,
                    "{what}: the strip's four children: {strip:?}"
                );
                for (part, r) in &strip {
                    assert!(
                        r.width > 0.0 || part == "strip status",
                        "{what}: the {part} {r:?} is laid out"
                    );
                }
                parts.extend(strip.into_iter().filter(|(_, r)| r.width > 0.0));
                for (part, r) in &parts {
                    assert!(
                        (r.width > 0.0 || part == "title") && inside(r, &area),
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
}

/// The Changes view's own header row, under the session area's view: its title column, the source
/// control (feature 1272), then the progress slot (feature 1270 moved the panel toggle and the review
/// buttons to the review bar).
const VIEW_HEADER: &[usize] = &[2, 0, 0];
/// The diff pane's header row: the layout and whole-file chips, then the mark control.
const DIFF_HEADER: &[usize] = &[2, 1, 1, 0, 0, 0];
/// The diff pane, the diff area's first layer, which the diff header row must stay inside.
const DIFF_PANE: &[usize] = &[2, 1, 1, 0];

/// Feature 1271 T033 (FR-009, SC-005): at 1280×800 in both themes the view header's title and *n
/// of m reviewed* slot, and the diff header's chips and **Mark reviewed**, each lie whole inside
/// their row and none overlaps another; the slot is as wide, and the title as wide, whether
/// progress is shown or not.
#[test]
fn the_view_header_and_the_diff_header_hold_the_mark_controls() {
    let mut placed = Vec::new();
    for name in [
        "changes-review-marks",
        "changes-review-marks-unavailable",
        // Feature 1274: with the hunk controls beside the chips.
        "review-current-hunk",
    ] {
        let covered = covered_states()
            .iter()
            .find(|s| s.name == name)
            .unwrap_or_else(|| panic!("the covered state {name} exists"));
        for theme in [ThemePreference::Light, ThemePreference::Dark] {
            let mut under = (covered.build)();
            under.state.settings.theme_pref = theme;
            under.state.sidebar.width = SIDEBAR;
            let records = records_of(under, WINDOW);
            let what = format!("{name} {theme:?}");
            for (row, host) in [
                (
                    [SESSION_AREA, VIEW_HEADER].concat(),
                    [SESSION_AREA, &[2]].concat(),
                ),
                (
                    [SESSION_AREA, DIFF_HEADER].concat(),
                    [SESSION_AREA, DIFF_PANE].concat(),
                ),
            ] {
                let row_rect = rect(
                    at(&records, &row)
                        .unwrap_or_else(|| panic!("{what}: the row {row:?} is laid out")),
                );
                let host_rect = rect(at(&records, &host).expect("the host is laid out"));
                assert!(
                    inside(&row_rect, &host_rect),
                    "{what}: the row {row_rect:?} lies inside {host_rect:?}"
                );
                let parts: Vec<Rectangle> =
                    children_of(&records, &row).into_iter().map(rect).collect();
                assert!(
                    parts.len() >= 2,
                    "{what}: the row {row:?} has its parts: {parts:?}"
                );
                for (i, a) in parts.iter().enumerate() {
                    assert!(
                        inside(a, &row_rect),
                        "{what}: part {i} {a:?} lies inside its row {row_rect:?}"
                    );
                    for b in &parts[i + 1..] {
                        assert!(!overlap(a, b), "{what}: {a:?} overlaps {b:?}");
                    }
                }
            }
            let header = [SESSION_AREA, VIEW_HEADER].concat();
            let part = |i: usize| {
                rect(at(&records, &[header.as_slice(), &[i]].concat()).expect("a header part"))
            };
            placed.push((what, part(2), part(0)));
        }
    }
    let (first, slot, title) = &placed[0];
    for (what, other_slot, other_title) in &placed[1..] {
        assert_eq!(
            (other_slot.x, other_slot.width, other_title.width),
            (slot.x, slot.width, title.width),
            "{what}: the slot and the title stand where {first} has them (FR-009)"
        );
    }
}

/// The progress slot's bar, under the view header: the slot's two containers, its row, then the
/// bar after the *n of m reviewed* words.
const PROGRESS_BAR: &[usize] = &[2, 0, 0, 1];

/// Feature 1271 (visual re-pass V2): the header's progress bar is drawn, at its full width where
/// the slot has room for it beside its words. A `Fill` bar in the slot's `Shrink` row was laid out
/// at zero width, so the words updated with no bar beside them.
#[test]
fn the_progress_bar_has_its_width_beside_its_words() {
    let covered = covered_states()
        .iter()
        .find(|s| s.name == "changes-review-marks")
        .expect("the covered state changes-review-marks exists");
    for theme in [ThemePreference::Light, ThemePreference::Dark] {
        let mut under = (covered.build)();
        under.state.settings.theme_pref = theme;
        under.state.sidebar.width = SIDEBAR;
        // The drawer open, as the visual pass saw it; since feature 1270 the header holds only the
        // title and the slot either way (the review buttons moved to the review bar).
        let _ = micold_client::features::changes::update(
            &mut under.state.changes,
            micold_client::features::changes::Msg::Drawer(
                micold_core::review::drawer::DrawerEvent::Toggle,
            ),
        );
        assert!(under
            .state
            .changes
            .tabs
            .shown_view()
            .is_some_and(|v| v.drawer.state.open));
        let records = records_of(under, WINDOW);
        let header = [SESSION_AREA, VIEW_HEADER].concat();
        let part = |path: &[usize]| {
            rect(
                at(&records, &[header.as_slice(), path].concat())
                    .unwrap_or_else(|| panic!("{theme:?}: the header part {path:?} is laid out")),
            )
        };
        let (slot, words, bar) = (part(&[2]), part(&[2, 0, 0, 0]), part(PROGRESS_BAR));
        assert!(
            (bar.width - micold_client::ui::PROGRESS_BAR_WIDTH).abs() <= TOLERANCE,
            "{theme:?}: the bar {bar:?} is {} px wide, not its {} beside {words:?}",
            bar.width,
            micold_client::ui::PROGRESS_BAR_WIDTH
        );
        assert!(
            bar.x >= words.x + words.width && inside(&bar, &slot) && inside(&words, &slot),
            "{theme:?}: the words {words:?} then the bar {bar:?} lie inside the slot {slot:?}"
        );
    }
}

/// Feature 1271 (M4 visual rounds 2 and 3, V4): a changed row's text is never painted in the
/// button's trailing padding, where the changed dot is drawn. In a narrow list the tags and counts
/// can be wider than the row once the path has shrunk to nothing; the row's content is clipped to
/// the button's content box, so what does not fit is cut at that edge and the dot stays clear.
/// Read from what is painted (each text's clip), not from layout nodes, which overflow either way.
#[test]
fn a_changed_rows_text_is_clipped_clear_of_the_dot() {
    let covered = covered_states()
        .iter()
        .find(|s| s.name == "changes-review-marks")
        .expect("the covered state changes-review-marks exists");
    let mut checked = 0;
    for drawer_open in [false, true] {
        for width in [640.0, 700.0, 800.0, 900.0, 1100.0] {
            let mut under = (covered.build)();
            under.state.sidebar.width = SIDEBAR;
            if drawer_open {
                let _ = micold_client::features::changes::update(
                    &mut under.state.changes,
                    micold_client::features::changes::Msg::Drawer(
                        micold_core::review::drawer::DrawerEvent::Toggle,
                    ),
                );
            }
            let mut renderer = lay::renderer();
            let (painted, records) = lay::painted_text_settled_at(
                lay::view_of(&under),
                &mut renderer,
                Size::new(width, 700.0),
            );
            // The dot is `<stack>/1/0`, 8 px square; the row's button is `<stack>/0`.
            let dots: Vec<_> = records
                .iter()
                .filter(|r| {
                    r.width == 8.0
                        && r.height == 8.0
                        && r.path.len() > 2
                        && r.path[r.path.len() - 2..] == [1, 0]
                })
                .collect();
            for dot in dots {
                let mut button_path = dot.path[..dot.path.len() - 2].to_vec();
                button_path.push(0);
                let button = rect(at(&records, &button_path).expect("the row's button"));
                let content_right = button.x + button.width - 12.0;
                for text in painted.iter().filter(|t| {
                    t.origin.y >= button.y
                        && t.origin.y < button.y + button.height
                        && t.origin.x >= button.x
                        && t.origin.x < button.x + button.width
                }) {
                    checked += 1;
                    assert!(
                        text.clip.x + text.clip.width <= content_right + TOLERANCE,
                        "{width} drawer {drawer_open}: {:?} is clipped at {}, past the content box's {content_right}",
                        text.content,
                        text.clip.x + text.clip.width
                    );
                }
            }
        }
    }
    assert!(checked > 0, "a changed row's text was read");
}

/// The scrolling members' keyed row (`SESSION_STRIP_TABS`), under the session area.
const STRIP_TABS: &[usize] = &[1, 0, 0, 0, 0, 0, 0];

/// The immediate children of `parent` in `records`.
fn children_of<'r>(records: &'r [LayoutRecord], parent: &[usize]) -> Vec<&'r LayoutRecord> {
    records
        .iter()
        .filter(|r| r.path.len() == parent.len() + 1 && r.path.starts_with(parent))
        .collect()
}

/// Feature 1269 T037 (U15, SC-006): the Review tab with a `99+` badge and the changed mark, and
/// shown with its badge, at the smallest window and the layout window in both themes: its mark,
/// label, badge and × are each laid out whole inside the tab, and none overlaps another.
#[test]
fn the_review_tab_holds_its_label_badge_mark_and_close_whole() {
    let build = |name: &str| {
        covered_states()
            .iter()
            .find(|s| s.name == name)
            .unwrap_or_else(|| panic!("the covered state {name} exists"))
            .build
    };
    for (name, marked) in [
        ("session-area-review-hidden", true),
        ("session-area-review-shown", false),
    ] {
        for theme in [ThemePreference::Light, ThemePreference::Dark] {
            for window in [MIN_WINDOW_SIZE, WINDOW] {
                let mut under = build(name)();
                crate::support::covered_states::feat_short_drafts(&mut under.state, 150);
                under.state.settings.theme_pref = theme;
                under.state.sidebar.width = SIDEBAR;
                let records = records_of(under, window);
                let what = format!("{name} 99+ {theme:?} {window:?}");
                let row = [SESSION_AREA, STRIP_TABS].concat();
                let members = children_of(&records, &row);
                let tab = *members.last().expect("the strip has members");
                let tab_rect = rect(tab);
                assert!(
                    (tab_rect.width - micold_material::tab::REVIEW_WIDTH).abs() <= TOLERANCE,
                    "{what}: the Review tab {tab_rect:?} is its fixed width"
                );
                // The tab's content row: the shallowest node inside it with three children — the
                // mark's slot, the label and the ×'s slot (`material::Tab`).
                let content = records
                    .iter()
                    .filter(|r| r.path.len() > tab.path.len() && r.path.starts_with(&tab.path))
                    .filter(|r| children_of(&records, &r.path).len() == 3)
                    .min_by_key(|r| r.path.len())
                    .unwrap_or_else(|| panic!("{what}: the tab lays out its content row"));
                let slots = children_of(&records, &content.path);
                // The label's ceiling container holds the row of `Review` and its badge.
                let mut label = children_of(&records, &slots[1].path);
                while label.len() == 1 {
                    label = children_of(&records, &label[0].path);
                }
                assert_eq!(label.len(), 2, "{what}: `Review` and its badge: {label:?}");
                let mut parts: Vec<(&str, Rectangle)> = vec![
                    ("label", rect(label[0])),
                    ("badge", rect(label[1])),
                    ("×", rect(slots[2])),
                ];
                if marked {
                    let mark = children_of(&records, &slots[0].path);
                    let mark = rect(mark.first().unwrap_or_else(|| {
                        panic!("{what}: the changed mark is drawn in the mark slot")
                    }));
                    parts.push(("changed mark", mark));
                }
                for (part, r) in &parts {
                    assert!(
                        r.width > 0.0 && r.height > 0.0 && inside(r, &tab_rect),
                        "{what}: the {part} {r:?} lies whole inside the tab {tab_rect:?}"
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
            }
        }
    }
}

/// H10's bound (M3 review B F2): at 1280×800 a label with counts as wide as a branch ever gets is
/// laid out whole beside the title and the chips. At the smallest window, where the bound is
/// three-digit counts (`a_long_title_and_a_pull_request_chip_still_fit`), it is cut rather than
/// pushing anything out: the title, chips and button still lie inside the area, overlapping nothing.
#[test]
fn the_widest_counts_fit_at_the_layout_window() {
    let covered = covered_states()
        .iter()
        .find(|s| s.name == "session-area-one-tab")
        .expect("the covered state exists");
    let mut one_line: Option<f32> = None;
    for (theme, window) in [ThemePreference::Light, ThemePreference::Dark]
        .into_iter()
        .flat_map(|theme| [(theme, WINDOW), (theme, MIN_WINDOW_SIZE)])
    {
        let mut under = (covered.build)();
        under.panes = Some(micold_core::pane_layout::PaneLayout::single());
        under.state.settings.theme_pref = theme;
        under.state.sidebar.width = SIDEBAR;
        let location = under
            .state
            .current_session_location()
            .expect("a session is shown");
        let project = under.state.workspace.active.clone().unwrap();
        under.state.changes.header.entry = Some((project, location));
        under.state.changes.header.load =
            micold_client::features::changes::Load::Ready(THE_WIDEST_COUNTS);
        let state = under.state.clone();
        let records = records_of(under, window);
        let what = format!("widest counts {theme:?} {window:?}");
        if window == WINDOW {
            assert_button_unsqueezed(&what, &records, &state);
        }
        let area = rect(at(&records, SESSION_AREA).expect("the session area is drawn"));
        let parts = header_parts(&records);
        for (part, r) in &parts {
            assert!(
                (r.width > 0.0 || part == "title") && inside(r, &area),
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
        // The label stays one line: its text is no taller than at the layout window, where it is
        // laid out whole (checked first).
        let button = [SESSION_AREA, CHANGES_BUTTON].concat();
        let inside_button: Vec<&LayoutRecord> = records
            .iter()
            .filter(|r| r.path.len() > button.len() && r.path.starts_with(&button))
            .collect();
        assert!(
            !inside_button.is_empty(),
            "{what}: the Changes button holds a glyph and a label"
        );
        // The deepest nodes are the glyph and the label's text, under the 40 dp button's fill.
        let deepest = inside_button
            .iter()
            .map(|r| r.path.len())
            .max()
            .unwrap_or(0);
        let inner = inside_button
            .iter()
            .filter(|r| r.path.len() == deepest)
            .map(|r| r.height)
            .fold(0.0_f32, f32::max);
        assert!(inner > 0.0, "{what}: the button's label has a height");
        // The baseline is the layout window in light, laid out whole; it comes first in the loop.
        if matches!(theme, ThemePreference::Light) && window == WINDOW {
            one_line = Some(inner);
        }
        let one_line = one_line.expect("the light layout window is measured first");
        assert!(
            inner <= one_line + TOLERANCE,
            "{what}: the button's label wraps ({inner} tall inside, one line is {one_line})"
        );
    }
}

/// The Changes view's body: the file column beside the diff area (feature 1272, R8).
const VIEW_BODY: &[usize] = &[2, 1];
/// The file column: the body's first child, a fixed-width container.
const FILE_COLUMN: &[usize] = &[2, 1, 0];
/// What the file column holds: the container's column.
const FILE_COLUMN_CONTENT: &[usize] = &[2, 1, 0, 0];
/// The diff area, the body's second child.
const DIFF_AREA: &[usize] = &[2, 1, 1];
/// The source control's slot in the view header.
const SOURCE_CONTROL: &[usize] = &[2, 0, 0, 1];

/// `pose` laid out in a window `window` large beside a 300 px sidebar in `theme`: what it painted
/// and the boxes of both layers, entrances finished.
fn painted_source(
    pose: SourcePose,
    theme: ThemePreference,
    window: Size,
) -> (Vec<lay::Overflow>, Vec<LayoutRecord>) {
    let mut under = review_source(pose, false);
    under.state.settings.theme_pref = theme;
    under.state.sidebar.width = SIDEBAR;
    let mut renderer = lay::renderer();
    lay::painted_text_settled_at(lay::view_of(&under), &mut renderer, window)
}

/// The base-layer box at `path` under the session area.
fn part_at(records: &[LayoutRecord], path: &[usize], what: &str) -> Rectangle {
    let path = [SESSION_AREA, path].concat();
    rect(
        records
            .iter()
            .find(|r| r.layer == lay::Layer::Base && r.path == path)
            .unwrap_or_else(|| panic!("{what}: {path:?} is laid out")),
    )
}

/// Whether `text` starts inside `area`.
fn starts_inside(text: &lay::Overflow, area: &Rectangle) -> bool {
    inside(
        &Rectangle {
            x: text.origin.x,
            y: text.origin.y,
            width: 0.0,
            height: 0.0,
        },
        area,
    )
}

/// Feature 1272 T019 (FR-003, SC-003, FR-001, FR-009): in every source state — the working tree
/// with each pull-request list state, a pull request shown, Files on, the Default entry, the setting
/// off — the left column holds one thing, starting at its top: the file list, its empty or loading
/// state, or the Files pane. No Pull requests section, no Committed, Uncommitted or Files toggle and
/// no Default-entry note is painted in it. The header holds the source control, which reads the
/// shown source, and the control's tooltip carries the base line.
#[test]
fn the_left_column_starts_with_the_file_list_and_the_header_holds_the_source_control() {
    use micold_client::features::changes::{base_line, source_menu_input};
    use micold_core::review::source_menu::{label, DEFAULT_ENTRY_NOTE, FILES, PULL_REQUESTS};
    // The chips' words, as `ui::source_menu` draws them.
    const COMMITTED: &str = "Committed";
    const UNCOMMITTED: &str = "Uncommitted";
    for pose in SourcePose::ALL {
        let under = review_source(pose, false);
        let changes = &under.state.changes;
        let view = changes.tabs.shown_view().expect("the review is shown");
        let now = LIST_READ_AT;
        let shown = label(&source_menu_input(view, changes.pr_enabled, now));
        let fixed = shown.fixed.clone();
        // The project root's review compares with no base; every other pose has a base line.
        let base = base_line(view);
        assert_eq!(
            base.is_none(),
            pose == SourcePose::DefaultEntry,
            "{pose:?}: the base line {base:?}"
        );
        // What the widget was given, read as a person reads it: the pointer on the trigger, the
        // tooltip it opens painted.
        let mut hovered = review_source(pose, false);
        hovered.state.sidebar.width = SIDEBAR;
        let mut renderer = lay::renderer();
        let (tip, _) = lay::painted_text_hovering_at(
            lay::view_of(&hovered),
            &mut renderer,
            &[SESSION_AREA, SOURCE_CONTROL].concat(),
            WINDOW,
        );
        let tooltip = |wanted: &str| {
            tip.iter()
                .any(|p| p.layer == lay::Layer::Overlay && p.content.contains(wanted))
        };
        assert!(
            tooltip(&shown.full()),
            "{pose:?}: the tooltip carries the full label {:?} (FR-009): {tip:?}",
            shown.full()
        );
        if let Some(base) = base {
            assert!(
                tooltip(&base),
                "{pose:?}: the tooltip carries the base line {base:?} (FR-009): {tip:?}"
            );
        }
        for theme in [ThemePreference::Light, ThemePreference::Dark] {
            let what = format!("{pose:?} {theme:?}");
            let (painted, records) = painted_source(pose, theme, WINDOW);
            let column = part_at(&records, FILE_COLUMN, &what);
            let content_path = [SESSION_AREA, FILE_COLUMN_CONTENT].concat();
            let content: Vec<Rectangle> = children_of(&records, &content_path)
                .into_iter()
                .filter(|r| r.layer == lay::Layer::Base)
                .map(rect)
                .collect();
            let holder = part_at(&records, FILE_COLUMN_CONTENT, &what);
            assert_eq!(
                content.len(),
                1,
                "{what}: the left column holds only the list, its state or the Files pane: \
                 {content:?}"
            );
            assert!(
                (content[0].y - holder.y).abs() <= TOLERANCE,
                "{what}: the list starts at the column's top {holder:?}: {content:?} (SC-003)"
            );
            for gone in [
                PULL_REQUESTS,
                COMMITTED,
                UNCOMMITTED,
                FILES,
                DEFAULT_ENTRY_NOTE,
            ] {
                let found = painted.iter().find(|p| {
                    p.layer == lay::Layer::Base && p.content == gone && starts_inside(p, &column)
                });
                assert!(
                    found.is_none(),
                    "{what}: `{gone}` is painted in the left column {column:?}: {found:?} (FR-003)"
                );
            }
            let header = part_at(&records, VIEW_HEADER, &what);
            let control = part_at(&records, SOURCE_CONTROL, &what);
            assert!(
                inside(&control, &header),
                "{what}: the source control {control:?} lies in the header {header:?}"
            );
            assert!(
                painted.iter().any(|p| p.layer == lay::Layer::Base
                    && p.content == fixed
                    && starts_inside(p, &control)),
                "{what}: the control reads `{fixed}` (FR-001)"
            );
        }
    }
}

/// What the review header's source control publishes for `gestures`, in a review posed as `pose`
/// with the list `open`, driven through the runtime's own `UserInterface`.
fn header_messages(
    pose: SourcePose,
    open: bool,
    gestures: &[lay::Gesture],
) -> Vec<Vec<micold_client::app::Message>> {
    let mut under = review_source(pose, open);
    under.state.sidebar.width = SIDEBAR;
    lay::runtime_messages(|| lay::view_of(&under), WINDOW, gestures)
}

/// Feature 1272 T028 (US1 s6, US4 s3, FR-011): through the real review header, a press on the
/// trigger toggles the list, a press outside it or `Esc` dismisses it, and an arrow key walks it —
/// each its own message, so a header that wired one to another's fails here.
#[test]
fn the_review_header_sends_each_gesture_to_its_own_message() {
    use iced::keyboard::key::Named;
    use micold_client::app::Message;
    use micold_client::features::changes::Msg;
    use micold_core::typeahead::Direction;
    let changes = |msg: Msg| vec![Message::Changes(msg)];

    let (_, records) = painted_source(SourcePose::Listed, ThemePreference::Light, WINDOW);
    let trigger = part_at(&records, SOURCE_CONTROL, "the closed header");
    let at = iced::Point::new(
        trigger.x + trigger.width / 2.0,
        trigger.y + trigger.height / 2.0,
    );
    assert_eq!(
        header_messages(SourcePose::Listed, false, &[lay::Gesture::Click(at)]),
        vec![changes(Msg::SourceMenuToggled)],
        "a press on the trigger toggles the list"
    );

    let outside = iced::Point::new(WINDOW.width - 8.0, WINDOW.height - 8.0);
    for gesture in [
        lay::Gesture::Click(outside),
        lay::Gesture::Key(Named::Escape),
    ] {
        assert_eq!(
            header_messages(SourcePose::Listed, true, &[gesture]),
            vec![changes(Msg::SourceMenuDismissed)],
            "{gesture:?} dismisses the open list"
        );
    }
    assert_eq!(
        header_messages(
            SourcePose::Listed,
            true,
            &[lay::Gesture::Key(Named::ArrowDown)]
        ),
        vec![changes(Msg::SourceMenuMoved(Direction::Next))],
        "the down arrow walks to the next stop"
    );
}

/// FR-008 ("about 260 px"): the file column's width, by value, not the constant that builds it.
const FILE_COLUMN_PX: f32 = 260.0;
/// The source control's width in the header, by value (R8).
const SOURCE_CONTROL_PX: f32 = 360.0;

/// Feature 1272 T020 (FR-008, SC-002, US4 s1–s2, Edge "Narrow window"): the body is the file column
/// at its fixed width and the diff area; at 1280×800 the diff area is at least 70% of the body; at
/// 1600 and 900 px wide the file column keeps its width; at 900 px the source control keeps its
/// width inside the header while the title column gives way.
#[test]
fn the_file_column_keeps_its_width_and_the_diff_takes_the_rest() {
    for theme in [ThemePreference::Light, ThemePreference::Dark] {
        let mut titles = Vec::new();
        for width in [1280.0, 1600.0, 900.0] {
            let window = Size::new(width, WINDOW.height);
            let what = format!("{theme:?} {width}");
            let (_, records) = painted_source(SourcePose::PullRequest, theme, window);
            let body = part_at(&records, VIEW_BODY, &what);
            let file = part_at(&records, FILE_COLUMN, &what);
            let diff = part_at(&records, DIFF_AREA, &what);
            assert_eq!(
                children_of(&records, &[SESSION_AREA, VIEW_BODY].concat())
                    .into_iter()
                    .filter(|r| r.layer == lay::Layer::Base)
                    .count(),
                2,
                "{what}: the body is two columns"
            );
            assert!(
                (file.width - FILE_COLUMN_PX).abs() <= TOLERANCE,
                "{what}: the file column {file:?} is {FILE_COLUMN_PX} px (US4 s2)"
            );
            assert!(
                diff.x >= file.x + file.width && inside(&diff, &body),
                "{what}: the diff area {diff:?} follows the file column {file:?} in {body:?}"
            );
            if width == 1280.0 {
                assert!(
                    diff.width >= 0.7 * body.width,
                    "{what}: the diff area {diff:?} is under 70% of the body {body:?} (SC-002)"
                );
            }
            let header = part_at(&records, VIEW_HEADER, &what);
            let control = part_at(&records, SOURCE_CONTROL, &what);
            assert!(
                (control.width - SOURCE_CONTROL_PX).abs() <= TOLERANCE
                    && inside(&control, &header)
                    && control.x + control.width <= width + TOLERANCE,
                "{what}: the control {control:?} keeps its width in the header {header:?}"
            );
            titles.push(part_at(&records, &[2, 0, 0, 0], &what).width);
        }
        assert!(
            titles[2] < titles[0],
            "{theme:?}: the title column gives way first at 900 px: {titles:?}"
        );
    }
}
