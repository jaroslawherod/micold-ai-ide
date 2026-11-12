//! The session area's one-row tab strip fits (feature 1126, T011, T028, contracts S9, A1, A2,
//! FR-013, SC-004, SC-005; feature 1793, FR-001: the context header is gone).
//!
//! Every covered state that shows a session is laid out at the smallest window the application
//! allows beside a 300 px sidebar, and at the layout window, in light and in dark: the strip's
//! scrolling region and controls lie inside the session area and inside the window, and none of
//! them overlaps another. The strip states hold 1, 6 and 12 tabs. Top to bottom the area is strip,
//! pane area, and nothing is laid out under the pane area (A1); with two panes the strip is laid
//! out once, above the pane area (A2). `header_row_fits.rs` holds the strip's 40 px height.
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

/// The session area: the parent of the strip and the pane area.
pub(crate) const SESSION_AREA: &[usize] = &[0, 0, 1, 1];

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

/// The sidebar width H10 measures the smallest window beside.
pub(crate) const SIDEBAR: u16 = 300;

pub(crate) fn rect(record: &LayoutRecord) -> Rectangle {
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
pub(crate) fn records_of(under: StateUnderTest, window: Size) -> Vec<LayoutRecord> {
    let renderer = lay::renderer();
    let mut element = lay::view_of(&under);
    let mut tree = Tree::new(element.as_widget());
    let limits = layout::Limits::new(Size::ZERO, window);
    let node = element
        .as_widget_mut()
        .layout(&mut tree, &renderer, &limits);
    lay::walk(Layout::new(&node), lay::Layer::Base)
}

/// The strip's height in `state` at the layout window: chrome above the pane area that other gates
/// measuring the pane leave out of the pane's room.
pub(crate) fn strip_height(state: State) -> f32 {
    let records = records_at(state, WINDOW);
    at(&records, &[SESSION_AREA, &[0]].concat())
        .expect("the session area draws its strip")
        .height
}

/// The record at `path`.
pub(crate) fn at<'r>(records: &'r [LayoutRecord], path: &[usize]) -> Option<&'r LayoutRecord> {
    records.iter().find(|r| r.path == path)
}

pub(crate) fn inside(part: &Rectangle, host: &Rectangle) -> bool {
    part.x >= host.x - TOLERANCE
        && part.y >= host.y - TOLERANCE
        && part.x + part.width <= host.x + host.width + TOLERANCE
        && part.y + part.height <= host.y + host.height + TOLERANCE
}

pub(crate) fn overlap(a: &Rectangle, b: &Rectangle) -> bool {
    a.x + TOLERANCE < b.x + b.width
        && b.x + TOLERANCE < a.x + a.width
        && a.y + TOLERANCE < b.y + b.height
        && b.y + TOLERANCE < a.y + a.height
}

/// The strip's own row's children: its scrolling region and each control after it (restart, the
/// splits, the `N hidden` chip; S7). The members inside the region scroll, so they are not parts
/// that must fit the window.
pub(crate) fn strip_parts(records: &[LayoutRecord]) -> Vec<(String, Rectangle)> {
    let row = [SESSION_AREA, &[0, 0]].concat();
    let controls = [row.as_slice(), &[1]].concat();
    let mut parts: Vec<(String, Rectangle)> = Vec::new();
    parts.extend(
        at(records, &[row.as_slice(), &[0]].concat())
            .map(|r| ("strip tab region".to_string(), rect(r))),
    );
    parts.extend(
        children_of(records, &controls)
            .into_iter()
            .enumerate()
            .map(|(i, r)| (format!("strip control {i}"), rect(r))),
    );
    parts
}

/// A1, A2: the session area holds the strip and the pane area, in that order, each once, and
/// nothing under the pane area.
pub(crate) fn assert_order(name: &str, records: &[LayoutRecord], area: &Rectangle) {
    let children: Vec<&LayoutRecord> = records
        .iter()
        .filter(|r| r.path.len() == SESSION_AREA.len() + 1 && r.path.starts_with(SESSION_AREA))
        .collect();
    assert_eq!(
        children.len(),
        2,
        "{name}: the session area is strip and pane area, each laid out once (A1, A2)"
    );
    let [strip, panes] = [children[0], children[1]].map(rect);
    assert!(
        strip.y + strip.height <= panes.y + TOLERANCE,
        "{name}: strip {strip:?} and pane area {panes:?} are not top to bottom"
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

pub(crate) fn assert_fits_under(name: &str, build: impl Fn() -> StateUnderTest) {
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
            let parts = strip_parts(&records);
            assert!(
                parts.len() >= 2,
                "{name} {theme:?} {window:?}: the tab region and the controls: {parts:?}"
            );
            for (what, part) in &parts {
                // Only the strip's tab region may shrink to nothing: FR-012 lets the tab region
                // go before the controls are clipped.
                assert!(
                    (part.width > 0.0 || what == "strip tab region")
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

#[test]
fn the_strip_fits_in_every_session_state_at_both_windows() {
    for name in SESSION_STATES {
        let covered = covered_states()
            .iter()
            .find(|s| s.name == *name)
            .unwrap_or_else(|| panic!("the covered state {name} exists"));
        assert_fits_under(name, covered.build);
    }
}

/// T053 (R12, A1), feature 1269 T037 (U16): with a review shown the session area is the strip,
/// then the view; the strip's controls fit at both windows and themes.
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
                    2,
                    "{what}: the strip, then the view: {children:?}"
                );
                assert!(
                    children[0].y + children[0].height <= children[1].y + TOLERANCE,
                    "{what}: strip and view are not top to bottom: {children:?}"
                );
                let parts: Vec<(String, Rectangle)> = strip_parts(&records)
                    .into_iter()
                    .filter(|(_, r)| r.width > 0.0)
                    .collect();
                assert!(parts.len() >= 2, "{what}: the strip's parts: {parts:?}");
                for (part, r) in &parts {
                    assert!(
                        inside(r, &area),
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
            }
        }
    }
}

/// The Changes view's own header row, under the session area's view: its title column, the source
/// control (feature 1272), then the progress slot (feature 1270 moved the panel toggle and the review
/// buttons to the review bar).
const VIEW_HEADER: &[usize] = &[1, 0, 0];
/// The diff pane's header row: the layout and whole-file chips, then the mark control.
const DIFF_HEADER: &[usize] = &[1, 1, 1, 0, 0, 0];
/// The diff pane, the diff area's first layer, which the diff header row must stay inside.
const DIFF_PANE: &[usize] = &[1, 1, 1, 0];

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
                    [SESSION_AREA, &[1]].concat(),
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
const PROGRESS_BAR: &[usize] = &[1, 0, 0, 1];

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
const STRIP_TABS: &[usize] = &[0, 0, 0, 0, 0, 0, 0];

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

/// The Changes view's body: the file column beside the diff area (feature 1272, R8).
const VIEW_BODY: &[usize] = &[1, 1];
/// The file column: the body's first child, a fixed-width container.
const FILE_COLUMN: &[usize] = &[1, 1, 0];
/// What the file column holds: the container's column.
const FILE_COLUMN_CONTENT: &[usize] = &[1, 1, 0, 0];
/// The diff area, the body's second child.
const DIFF_AREA: &[usize] = &[1, 1, 1];
/// The source control's slot in the view header.
const SOURCE_CONTROL: &[usize] = &[1, 0, 0, 1];

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
            titles.push(part_at(&records, &[1, 0, 0, 0], &what).width);
        }
        assert!(
            titles[2] < titles[0],
            "{theme:?}: the title column gives way first at 900 px: {titles:?}"
        );
    }
}

// ---- Feature 1463, M2: the switch control in the strip -------------------------------------------

/// A Claude terminal session in its AI tab, its process in `lifecycle`, with `queued` messages
/// waiting for the structured view (feature 1463 FR-004, FR-012).
fn claude_terminal(stopped: bool, queued: usize) -> State {
    use micold_client::features::composer::InputState;
    use micold_core::protocol::messages::{QueueEntryId, QueuedSummary, TurnPhase};
    let (mut state, id) = crate::support::covered_states::session_pane(
        micold_core::session::SessionKind::Terminal,
        &[],
    );
    for session in state.workspace.sessions.values_mut().flatten() {
        session.lifecycle = if stopped {
            micold_core::session::SessionLifecycle::Idle
        } else {
            micold_core::session::SessionLifecycle::Running
        };
    }
    state.composers.input_reported(
        id,
        InputState {
            phase: TurnPhase::Idle,
            hold: true,
            queue: (0..queued as u64)
                .map(|n| QueuedSummary {
                    id: QueueEntryId(n),
                    text: "a message".into(),
                    attachments: Vec::new(),
                })
                .collect(),
        },
    );
    state
}

/// One row above the terminal: the strip (feature 1793, FR-001), at most this tall.
const ONE_ROW_MAX: f32 = micold_client::ui::session_area::ROW_HEIGHT;

/// The strip's controls after the tabs: restart while the process is stopped, the splits.
const CONTROLS: &[usize] = &[0, 0, 1];

fn controls_children(records: &[LayoutRecord]) -> Vec<Rectangle> {
    let path = [SESSION_AREA, CONTROLS].concat();
    children_of(records, &path).into_iter().map(rect).collect()
}

fn each_window(mut check: impl FnMut(String, ThemePreference, Size)) {
    for theme in [ThemePreference::Light, ThemePreference::Dark] {
        for window in [MIN_WINDOW_SIZE, WINDOW] {
            check(format!("{theme:?} {window:?}"), theme, window);
        }
    }
}

fn records_for(mut state: State, theme: ThemePreference, window: Size) -> Vec<LayoutRecord> {
    state.settings.theme_pref = theme;
    state.sidebar.width = SIDEBAR;
    let under = StateUnderTest::new(state).panes(micold_core::pane_layout::PaneLayout::single());
    records_of(under, window)
}

/// US2 scenarios 1, 2, 3 (FR-004, FR-005, FR-009, SC-001, SC-002; feature 1793): a Claude terminal
/// session in a single pane has one row above the terminal, restart comes first among the controls
/// while the process is stopped, nothing is laid out between the strip and the terminal, and no
/// switch is in the strip (it is in the worktree menu).
#[test]
fn a_claude_terminal_session_has_one_row_and_no_switch_in_the_strip() {
    for (stopped, queued) in [(false, 0), (true, 0), (true, 3)] {
        each_window(|what, theme, window| {
            let what = format!("stopped {stopped}, {queued} queued {what}");
            let records = records_for(claude_terminal(stopped, queued), theme, window);
            let area = rect(at(&records, SESSION_AREA).expect("the session area is drawn"));
            let [strip, panes] = [0, 1]
                .map(|i| rect(at(&records, &[SESSION_AREA, &[i]].concat()).expect("a row")));
            assert!(
                panes.y - area.y <= ONE_ROW_MAX + TOLERANCE,
                "{what}: the terminal starts {} px below the area, over {ONE_ROW_MAX} (FR-001): \
                 strip {strip:?}",
                panes.y - area.y
            );
            assert!(
                (panes.y - (strip.y + strip.height)).abs() <= TOLERANCE,
                "{what}: something sits between the strip {strip:?} and the pane area {panes:?} \
                 (no switch row, FR-004)"
            );
            let controls = controls_children(&records);
            let expected = if stopped { 2 } else { 1 };
            assert_eq!(controls.len(), expected, "{what}: restart?, splits {controls:?}");
            for control in &controls {
                assert!(
                    inside(control, &area) && inside(control, &Rectangle::with_size(window)),
                    "{what}: the control {control:?} lies inside the area"
                );
            }
        });
    }
}

/// US3 scenario 2 (FR-011): at the smallest window beside the sidebar, the strip with restart
/// shown lies inside the area and overlaps nothing, light and dark.
#[test]
fn the_filled_strip_fits_at_the_smallest_window() {
    assert_fits_under("claude terminal, queue held", || {
        StateUnderTest::new(claude_terminal(true, 3))
    });
}

/// FR-008, FR-005: restart keeps its distance from the strip's right end whether the Review tab is
/// shown or hidden.
#[test]
fn restart_keeps_its_place_when_the_review_is_shown_and_closed() {
    use micold_client::features::changes::{self, Msg};
    let shown = || {
        crate::support::covered_states::with_review_of(
            claude_terminal(true, 3),
            SessionLocation::Worktree("feat-short".to_string()),
        )
    };
    let hidden = || {
        let mut state = shown();
        changes::update(&mut state.changes, Msg::SessionSelected);
        state
    };
    for window in [MIN_WINDOW_SIZE, WINDOW] {
        let mut from_right = Vec::new();
        for (what, state) in [("shown", shown()), ("hidden", hidden())] {
            let records = records_for(state, ThemePreference::Light, window);
            let cluster = controls_children(&records);
            assert_eq!(
                cluster.len(),
                2,
                "{what} {window:?}: restart, splits: {cluster:?}"
            );
            let controls = at(&records, &[SESSION_AREA, CONTROLS].concat()).expect("controls");
            let restart = cluster[0];
            from_right.push((controls.x + controls.width) - (restart.x + restart.width));
        }
        assert!(
            (from_right[0] - from_right[1]).abs() <= TOLERANCE,
            "{window:?}: restart sits {} from the controls' end with the Review shown, {} with it \
             closed (FR-008)",
            from_right[0],
            from_right[1]
        );
    }
}

/// FR-007, SC-004: a tiled pane keeps its own switch row, directly under its pane header, at the
/// size and the offset from the pane area's origin it had before the switch moved into the strip
/// (recorded at 1280x800 with the sidebar open: 34.2 px pane header, then a 48 px row inset
/// `XS`/`SM` that holds the held-queue caption and the 40 px high **Open in structured view**).
#[test]
fn a_tiled_pane_still_has_its_switch_row() {
    use micold_core::pane_layout::{Axis, PaneLayout};
    use micold_core::protocol::messages::{SessionProcess, TerminalRef};
    let mut state = claude_terminal(true, 2);
    state.settings.theme_pref = ThemePreference::Light;
    state.sidebar.width = SIDEBAR;
    let session = state.session.active.expect("a shown session");
    let mut layout = PaneLayout::single();
    let first = layout.focused();
    layout
        .split(first, Axis::Vertical, (1000.0, 600.0), (80.0, 160.0), None)
        .expect("a 1000 px pane splits in two");
    layout
        .show(
            first,
            TerminalRef {
                session,
                process: SessionProcess::Primary,
            },
        )
        .expect("the first pane shows the Claude terminal");
    let records = records_of(StateUnderTest::new(state).panes(layout), WINDOW);
    let panes_path = [SESSION_AREA, &[1]].concat();
    let panes = rect(at(&records, &panes_path).expect("pane area"));
    // The pane area's tiles: [split, first tile]; a tile is the column pane header, switch row,
    // terminal (`ui::panes`).
    let tile = [panes_path.as_slice(), &[0, 0]].concat();
    let row = rect(at(&records, &[tile.as_slice(), &[1]].concat()).expect("the switch row"));
    let button = rect(at(&records, &[tile.as_slice(), &[1, 0, 2]].concat()).expect("the button"));
    let near = |what: &str, got: f32, recorded: f32| {
        assert!(
            (got - recorded).abs() <= TOLERANCE,
            "{what} is {got}, recorded {recorded} before the switch moved into the strip (FR-007, \
             SC-004)"
        );
    };
    near("the switch row's width", row.width, 487.0);
    near("the switch row's height", row.height, 48.0);
    near(
        "the switch row's x offset from the pane area",
        row.x - panes.x,
        0.0,
    );
    near(
        "the switch row's y offset from the pane area",
        row.y - panes.y,
        34.2,
    );
    near(
        "the button's inset from the row's top (XS)",
        button.y - row.y,
        4.0,
    );
    near(
        "the button's inset from the row's right (SM)",
        row.x + row.width - (button.x + button.width),
        8.0,
    );
    near("the button's height", button.height, 40.0);
}
