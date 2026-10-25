//! The review bar holds its place and the comments drawer covers the diff without moving it
//! (feature 1270, T015, T041 — FR-001, FR-010, FR-017, SC-001, SC-002, SC-004).
//!
//! At the fixture's 1280×800 window: **Send review (n)** has the same bounds with 0 and 99 drafts,
//! while sending, with a send error and without, and with the drawer closed and open (SC-001); the
//! bar is `review_bar::HEIGHT` high, directly above the status bar, at the Review tab's full width
//! (FR-001). Opening the drawer changes neither the diff pane's bounds nor what the diff paints
//! left of the drawer (SC-002); with the drawer closed the diff area spans the tab body beside the
//! file list, at least as wide as the old diff and review columns together (SC-004); and the
//! drawer's left edge is at or right of the diff area's middle, so a jumped-to line starts left
//! of it (FR-017).
//!
//! **Compiled into the `layout_snapshot` binary** for the `support::layout` passes; it builds its
//! own states from the covered states' builders.

use crate::support::covered_states::{
    feat_short_drafts, review_bar, review_shown, REVIEW_BAR, REVIEW_BAR_SEND, REVIEW_BODY,
    REVIEW_DIFF_AREA, REVIEW_DIFF_PANE, REVIEW_DRAWER,
};
use crate::support::layout::{self as lay, LayoutRecord, StateUnderTest, WINDOW};
use iced::{Rectangle, Size};
use micold_client::app::State;
use micold_client::features::changes::{self, Effect, Load, Msg};
use micold_core::review::base::{Base, ReviewScope};
use micold_core::review::changes::{
    ChangeIdentity, ChangeKind, ChangeList, ChangedFile, Content, Origin,
};
use micold_core::review::diff::{parse_unified, LoadedDiff, SideLines};
use micold_core::review::drawer::DrawerEvent;
use micold_core::review::RelPath;
use micold_core::tokens::{anatomy, spacing};

/// Half a pixel, matching every geometry gate beside this one.
const TOLERANCE: f32 = 0.5;

/// The lines of the diff [`with_diff`] shows: more than the pane holds.
const DIFF_LINES: u32 = 60;

fn rect(record: &LayoutRecord) -> Rectangle {
    Rectangle {
        x: record.x,
        y: record.y,
        width: record.width,
        height: record.height,
    }
}

/// The base layer of `under` laid out at the fixture's window.
fn records(under: &StateUnderTest) -> Vec<LayoutRecord> {
    lay::resolve(lay::view_of(under), &lay::renderer())
        .into_iter()
        .filter(|r| r.layer == lay::Layer::Base)
        .collect()
}

/// The bounds of the node at `path` in `records`.
fn at(records: &[LayoutRecord], path: &[usize], what: &str) -> Rectangle {
    records
        .iter()
        .find(|r| r.path == path)
        .map(rect)
        .unwrap_or_else(|| panic!("the Review tab lays out {what} at {path:?}"))
}

/// The bounds of **Send review (n)** in `under`.
fn send_bounds(under: &StateUnderTest) -> Rectangle {
    at(&records(under), REVIEW_BAR_SEND, "Send review")
}

fn apply(under: &mut StateUnderTest, msg: Msg) {
    changes::update(&mut under.state.changes, msg);
}

fn drawer_open(mut under: StateUnderTest) -> StateUnderTest {
    apply(&mut under, Msg::Drawer(DrawerEvent::Toggle));
    assert!(
        under
            .state
            .changes
            .tabs
            .shown_view()
            .is_some_and(|v| v.drawer.state.open),
        "the drawer opened"
    );
    under
}

fn with_error(mut under: StateUnderTest) -> StateUnderTest {
    let view = under
        .state
        .changes
        .tabs
        .shown_view_mut()
        .expect("a Review tab is shown");
    view.send_error = Some(micold_core::review::bar::error_text(
        "no session could be started for this worktree",
    ));
    under
}

fn sending(mut under: StateUnderTest) -> StateUnderTest {
    for review in under.state.changes.reviews.values_mut() {
        review.sending = true;
    }
    under
}

fn with_drafts(drafts: usize) -> StateUnderTest {
    let mut under = review_shown();
    feat_short_drafts(&mut under.state, drafts);
    under
}

/// `under` with `src/lib.rs` listed, selected and its diff of [`DIFF_LINES`] added lines read.
fn with_diff(mut under: StateUnderTest) -> StateUnderTest {
    let path = RelPath::from_native("src/lib.rs").expect("a relative path");
    let seq = match &shown(&under.state).list {
        Load::Loading { seq, .. } => *seq,
        other => panic!("the Review tab reads its list first, got {other:?}"),
    };
    apply(
        &mut under,
        Msg::ListRead {
            seq,
            result: Ok(ChangeList {
                files: vec![ChangedFile {
                    path: path.clone(),
                    kind: ChangeKind::Modified,
                    content: Content::Text,
                    added: DIFF_LINES,
                    removed: 0,
                    large: false,
                    origin: Origin::Uncommitted,
                    identity: ChangeIdentity::Unknown,
                }],
                scope: ReviewScope::Worktree {
                    base: Base::MergeBase {
                        branch: "main".into(),
                        commit: "0123456789abcdef0123456789abcdef01234567".into(),
                    },
                },
            }),
            whole: Ok(Default::default()),
        },
    );
    let seq = match changes::update(&mut under.state.changes, Msg::FileSelected(path)) {
        Effect::ReadDiff { seq, .. } => seq,
        other => panic!("selecting a file reads its diff, got {other:?}"),
    };
    let lines: String = (1..=DIFF_LINES)
        .map(|n| format!("+let line_{n} = {n};\n"))
        .collect();
    let unified = format!("@@ -0,0 +1,{DIFF_LINES} @@\n{lines}");
    let new: String = (1..=DIFF_LINES)
        .map(|n| format!("let line_{n} = {n};\n"))
        .collect();
    apply(
        &mut under,
        Msg::DiffRead {
            seq,
            result: Ok(LoadedDiff {
                diff: parse_unified(unified.as_bytes()),
                old: SideLines::from_bytes(b""),
                new: SideLines::from_bytes(new.as_bytes()),
                spans: Default::default(),
            }),
        },
    );
    under
}

fn shown(state: &State) -> &changes::OpenView {
    state
        .changes
        .tabs
        .shown_view()
        .expect("a Review tab is shown")
}

/// FR-001, B1: the bar is one `review_bar::HEIGHT` row, directly above the status bar, spanning
/// the Review tab.
#[test]
fn the_bar_sits_directly_above_the_status_bar_at_the_tabs_full_width() {
    let records = records(&review_bar());
    let bar = at(&records, REVIEW_BAR, "the bar");
    let tab = at(&records, &REVIEW_BAR[..REVIEW_BAR.len() - 1], "the tab");
    assert_eq!(bar.height, anatomy::review_bar::HEIGHT, "{bar:?}");
    let status_top = WINDOW.height - anatomy::status_bar::HEIGHT;
    assert!(
        (bar.y + bar.height - status_top).abs() <= TOLERANCE,
        "the bar ends where the status bar starts ({status_top}): {bar:?}"
    );
    assert!(
        (bar.x - tab.x).abs() <= TOLERANCE && (bar.width - tab.width).abs() <= TOLERANCE,
        "the bar spans the tab: {bar:?} in {tab:?}"
    );
    assert!(
        (tab.x + tab.width - WINDOW.width).abs() <= TOLERANCE,
        "the tab reaches the window's right edge: {tab:?}"
    );
}

/// SC-001: **Send review (n)** keeps its bounds across 0 and 99 drafts, a send under way and a
/// send error.
#[test]
fn send_review_keeps_its_bounds_across_counts_sending_and_an_error() {
    let reference = send_bounds(&with_drafts(0));
    assert_eq!(reference.width, anatomy::review_bar::SEND_WIDTH);
    for (case, under) in [
        ("99 drafts", with_drafts(99)),
        ("sending", sending(with_drafts(3))),
        ("an error", with_error(with_drafts(3))),
        ("an error and 99 drafts", with_error(with_drafts(99))),
        ("mixed statuses", review_bar()),
    ] {
        assert_eq!(send_bounds(&under), reference, "{case}");
    }
}

/// SC-001: **Send review (n)** keeps its bounds with the drawer closed and open.
#[test]
fn send_review_keeps_its_bounds_with_the_drawer_open() {
    assert_eq!(
        send_bounds(&drawer_open(review_bar())),
        send_bounds(&review_bar())
    );
    assert_eq!(
        send_bounds(&drawer_open(with_error(with_drafts(99)))),
        send_bounds(&with_drafts(0))
    );
}

/// SC-002: opening the drawer changes neither the diff pane's bounds nor where the diff paints
/// its lines (the first visible one included) left of the drawer.
#[test]
fn opening_the_drawer_neither_resizes_nor_moves_the_diff() {
    let closed = with_diff(review_bar());
    let open = drawer_open(with_diff(review_bar()));
    let (closed_records, open_records) = (records(&closed), records(&open));
    let pane = at(&closed_records, REVIEW_DIFF_PANE, "the diff pane");
    assert_eq!(at(&open_records, REVIEW_DIFF_PANE, "the diff pane"), pane);
    let drawer = at(&open_records, REVIEW_DRAWER, "the drawer");

    let diff_text = |under: &StateUnderTest| -> Vec<(String, f32, f32)> {
        lay::painted_text(lay::view_of(under), &mut lay::renderer())
            .into_iter()
            .filter(|p| pane.contains(p.origin) && p.origin.x < drawer.x)
            .map(|p| (p.content, p.origin.x, p.origin.y))
            .collect()
    };
    let before = diff_text(&closed);
    assert!(
        before.iter().any(|(text, _, _)| text.contains("line_1 ")),
        "the diff's first line is painted: {before:?}"
    );
    assert_eq!(diff_text(&open), before);
}

/// SC-004: with the drawer closed the diff area spans the tab body beside the file list, at
/// least as wide as the old diff and review columns together.
#[test]
fn with_the_drawer_closed_the_diff_spans_the_body_beside_the_file_list() {
    let records = records(&review_bar());
    let body = at(&records, REVIEW_BODY, "the body");
    let list = at(&records, &[REVIEW_BODY, &[0]].concat(), "the file list");
    let area = at(&records, REVIEW_DIFF_AREA, "the diff area");
    assert!(
        (area.x - (list.x + list.width + spacing::LG)).abs() <= TOLERANCE,
        "the diff area starts one gap right of the list: {area:?} after {list:?}"
    );
    assert!(
        (area.x + area.width - (body.x + body.width)).abs() <= TOLERANCE,
        "the diff area reaches the body's right edge: {area:?} in {body:?}"
    );
    // The old layout: list, diff and review column at 2 : 3 : 2 in the padded view.
    let old = (body.width - 4.0 * spacing::LG) * 5.0 / 7.0;
    assert!(
        area.width >= old - TOLERANCE,
        "the diff area ({}) is at least the old diff and review columns ({old})",
        area.width
    );
    assert!(
        !records.iter().any(|r| r.path == REVIEW_DRAWER),
        "no drawer is laid out while it is closed"
    );
}

/// The drawer's full width (FR-010), by value rather than the token that builds it.
const DRAWER_PX: f32 = 360.0;

/// A window narrow enough that half the diff area is under [`DRAWER_PX`].
const NARROW: Size = Size::new(1000.0, WINDOW.height);

/// FR-010, FR-017: the open drawer is 360 wide (at most half the diff area) at the area's right
/// edge, from under the header to the bar, and its left edge is at or right of the area's middle.
#[test]
fn the_open_drawer_covers_the_right_of_the_diff_area_down_to_the_bar() {
    let records = records(&drawer_open(review_bar()));
    let area = at(&records, REVIEW_DIFF_AREA, "the diff area");
    assert!(
        area.width / 2.0 >= DRAWER_PX,
        "at the fixture's window the drawer takes its full width: {area:?}"
    );
    assert_drawer_covers_the_right_of(&records, DRAWER_PX);
}

/// FR-010: when half the diff area is under 360, the drawer is that half.
#[test]
fn in_a_narrow_window_the_drawer_is_half_the_diff_area() {
    let mut renderer = lay::renderer();
    let (_, all) = lay::painted_text_settled_at(
        lay::view_of(&drawer_open(review_bar())),
        &mut renderer,
        NARROW,
    );
    let records: Vec<LayoutRecord> = all
        .into_iter()
        .filter(|r| r.layer == lay::Layer::Base)
        .collect();
    let area = at(&records, REVIEW_DIFF_AREA, "the diff area");
    assert!(
        area.width / 2.0 < DRAWER_PX,
        "the narrow window leaves the drawer less than its full width: {area:?}"
    );
    assert_drawer_covers_the_right_of(&records, area.width / 2.0);
}

/// The drawer is `wanted` wide, at the area's right edge, from its top to the bar, and starts at or
/// right of the area's middle.
fn assert_drawer_covers_the_right_of(records: &[LayoutRecord], wanted: f32) {
    let area = at(records, REVIEW_DIFF_AREA, "the diff area");
    let drawer = at(records, REVIEW_DRAWER, "the drawer");
    let bar = at(records, REVIEW_BAR, "the bar");
    assert!(
        (drawer.width - wanted).abs() <= TOLERANCE,
        "the drawer is {wanted} wide: {drawer:?} over {area:?}"
    );
    assert!(
        (drawer.x + drawer.width - (area.x + area.width)).abs() <= TOLERANCE,
        "the drawer's right edge is the area's: {drawer:?} in {area:?}"
    );
    assert!(
        (drawer.y - area.y).abs() <= TOLERANCE
            && (drawer.y + drawer.height - bar.y).abs() <= TOLERANCE,
        "the drawer runs from the area's top to the bar: {drawer:?}, bar {bar:?}"
    );
    assert!(
        drawer.x >= area.x + area.width / 2.0 - TOLERANCE,
        "a jumped-to line starts left of the drawer: {drawer:?} over {area:?}"
    );
}
