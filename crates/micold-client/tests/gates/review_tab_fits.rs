//! The finished review tab fits at the smallest window and at 1280×800 (feature 1276, M2, T018 —
//! FR-006, FR-007, contracts/review-states.md).
//!
//! Every review state the contract lists, by the name its slice registered it under, in light and
//! in dark, at `MIN_WINDOW_SIZE` and at the fixture's window: no text is painted wider than its
//! clip; no node escapes its parent beyond the overhangs `gates/containment.rs` declares (and, at
//! the smallest window, those [`SMALLEST_WINDOW`] declares, filed as #1474, and the cut header
//! title of #1501); the review bar ends at or above the status bar and both lie inside the
//! window; an open drawer lies inside the diff pane, which is as wide as with the drawer closed (an
//! overlay, not a push); the status bar keeps its invariants (`status_bar_fits.rs`); and the
//! session area's header and strip fit (`session_area_fits.rs`).
//!
//! **Compiled into the `layout_snapshot` binary** beside the gates whose checks it calls.

use crate::containment::clips_deliberately;
use crate::session_area_fits::assert_fits_under;
use crate::status_bar_fits::assert_the_bar_holds_its_place;
use crate::support::covered_states::{covered_states, REVIEW_BAR, REVIEW_DIFF_PANE, REVIEW_DRAWER};
use crate::support::layout::{self as lay, CoveredState, LayoutRecord, StateUnderTest, WINDOW};
use iced::{Rectangle, Size};
use micold_client::app::MIN_WINDOW_SIZE;
use micold_client::features::changes::{self, Msg};
use micold_core::review::drawer::DrawerEvent;
use micold_core::theme::ThemePreference;

/// Half a pixel, matching every geometry gate beside this one.
const TOLERANCE: f32 = 0.5;

/// The windows FR-007 names.
const WINDOWS: [Size; 2] = [MIN_WINDOW_SIZE, WINDOW];

/// Each review state of the contract: its name there, the covered state that records it (a
/// slice's own name stands), and the anchors the contract asks of it, by the names the fixture
/// uses (`review.tab` is the strip's `session.strip.tabs.review`).
const REVIEW_STATES: &[(&str, &str, &[&str])] = &[
    (
        "session-area-changes-open",
        "session-area-review-shown",
        &[
            "session.header",
            "session.header.changes",
            "session.strip",
            "session.strip.tabs.review",
        ],
    ),
    (
        "review-tab-beside-sessions",
        "review-tab-beside-sessions",
        &["session.strip.tabs.review", "review.tab.badge"],
    ),
    (
        "review-tab-hidden-changed",
        "session-area-review-hidden",
        &["session.strip.tabs.review", "review.tab.changed"],
    ),
    (
        "review-bar-counts",
        "review-bar",
        &[
            "review.bar",
            "review.bar.send",
            "review.bar.overflow",
            "status_bar",
        ],
    ),
    (
        "review-drawer-drafts",
        "drawer-drafts",
        &["review.drawer", "review.drawer.tabs", "review.diff"],
    ),
    (
        "review-drawer-replies",
        "drawer-replies",
        &["review.drawer", "review.diff"],
    ),
    (
        "review-source-open",
        "review-source-open",
        &["review.source", "review.source.list"],
    ),
    (
        "review-tree-filtered-sorted",
        "changes-tree-filter",
        &["review.tree", "review.tree.filter", "review.tree.sort"],
    ),
    (
        "review-reviewed-progress",
        "changes-review-marks",
        &["review.progress", "review.tree.row.mark"],
    ),
    (
        "review-shortcut-sheet",
        "review-shortcut-sheet",
        &["review.sheet"],
    ),
    (
        "review-stale-notice",
        "changes-review-held",
        &["review.stale"],
    ),
    (
        "review-drag-composer",
        "review-drag-composer",
        &["review.composer"],
    ),
    (
        "review-empty",
        "review-empty",
        &["review.tree", "review.bar", "review.bar.send"],
    ),
    (
        "review-two-worktrees",
        "review-two-worktrees",
        &["session.strip.tabs.review", "review.tree"],
    ),
];

/// Overhangs allowed at the smallest window only, as `(states, parent, edge, layer, why)`: in one
/// of `states`, a child escaping `parent` past `edge` in `layer`. At 1280×800 none fires and none
/// is allowed. Each entry must keep firing in each of its states, or it is widening the gate
/// silently: drop a state once its node fits there, and the entry once no state is left.
///
/// The first three are scrolling, not defects: content taller than the `Scrollable` that shows it,
/// as `gates/containment.rs`'s scroll exemptions. The rest are the review tab squeezed at 640×480,
/// filed as #1474 rather than fixed here (feature 1276 changes no review layout, spec § Out of
/// Scope).
const SMALLEST_WINDOW: &[(&[&str], &str, &str, &str, &str)] = &[
    (
        &[
            "changes-review-held",
            "changes-review-marks",
            "changes-tree-filter",
            "review-drag-composer",
            "review-shortcut-sheet",
            "review-source-open",
            "review-two-worktrees",
        ],
        "0/0/0/1/1/2/1/0/0/0/2",
        "bottom",
        "base",
        "the tree's VirtualRows: rows past its viewport",
    ),
    (
        &[
            "changes-review-held",
            "changes-review-marks",
            "review-drag-composer",
            "review-shortcut-sheet",
            "review-source-open",
        ],
        "0/0/0/1/1/2/1/1/0/0/1",
        "bottom",
        "base",
        "the diff's Scrollable: lines past its viewport",
    ),
    (
        &[
            "changes-review-held",
            "changes-review-marks",
            "review-shortcut-sheet",
            "review-source-open",
        ],
        "0/0/0/1/1/2/1/1/0/0/1",
        "right",
        "base",
        "the diff's Scrollable: a line wider than its viewport, scrolled sideways (#1466)",
    ),
    (
        &[
            "changes-review-marks",
            "changes-tree-filter",
            "review-drag-composer",
            "review-shortcut-sheet",
            "review-source-open",
            "review-two-worktrees",
        ],
        "0/0/0/1/1/2/0/0/2/0/0",
        "right",
        "base",
        "#1474: the progress bar past its slot",
    ),
    (
        &["changes-review-held"],
        "0/0/0/1/1/2/0/0/2/0/0/0",
        "right",
        "base",
        "#1474: the progress words past their box",
    ),
    (
        &["changes-review-held"],
        "0/0/0/1/1/2/0/0/3/0/0",
        "right",
        "base",
        "#1474: the progress bar past its slot, beside the stale notice",
    ),
    (
        &[
            "changes-review-held",
            "changes-review-marks",
            "review-drag-composer",
            "review-shortcut-sheet",
            "review-source-open",
        ],
        "0/0/0/1/1/2/1/0/0/0/1",
        "bottom",
        "base",
        "#1474: the wrapped sort chips below their row",
    ),
    (
        &[
            "changes-review-held",
            "changes-review-marks",
            "changes-tree-filter",
            "review-drag-composer",
            "review-shortcut-sheet",
            "review-source-open",
            "review-two-worktrees",
        ],
        "0/0/0/1/1/2/1/1/0/0/0",
        "bottom",
        "base",
        "#1474: the diff header's wrapped controls below their row",
    ),
    (
        &["changes-tree-filter", "review-two-worktrees"],
        "0/0/0/1/1/2/1/1/0/0/1/0/0",
        "bottom",
        "base",
        "#1474: the diff's loading placeholder past its box",
    ),
    (
        &["drawer-drafts", "drawer-replies", "review-empty"],
        "0/0/0/1/1/2/1/1/1/0/0/0/0",
        "right",
        "base",
        "#1474: the drawer's header past a narrow drawer",
    ),
    (
        &["drawer-drafts", "drawer-replies", "review-empty"],
        "0/0/0/1/1/2/1/1/1/0/0/1/0/0/0/0/0/0/1/0",
        "right",
        "base",
        "#1474: a draft or reply row's controls past a narrow drawer",
    ),
    (
        &["drawer-drafts", "drawer-replies", "review-empty"],
        "0/0/0/1/1/2/1/1/1/0/0/1/0/0/0/1/0/0/1/0",
        "right",
        "base",
        "#1474: a draft or reply row's controls past a narrow drawer",
    ),
    (
        &["drawer-drafts", "drawer-replies"],
        "0/0/0/1/1/2/1/1/1/0/0/1/0/0/1/0/0/0/0",
        "right",
        "base",
        "#1474: a draft or reply row past a narrow drawer",
    ),
    (
        &["drawer-drafts", "drawer-replies"],
        "0/0/0/1/1/2/1/1/1/0/0/1/0/0/1/0/0/0/2",
        "right",
        "base",
        "#1474: a draft or reply row's controls past a narrow drawer",
    ),
    (
        &["drawer-drafts"],
        "0/0/0/1/1/2/1/1/1/0/0/1/0/0/1/0/1/0/2",
        "right",
        "base",
        "#1474: a draft row's Edit and Delete past a narrow drawer",
    ),
    (
        &["drawer-drafts"],
        "0/0/0/1/1/2/1/1/1/0/0/1/0/0/1/0/2/0/2",
        "right",
        "base",
        "#1474: a draft row's Edit and Delete past a narrow drawer",
    ),
    (
        &["drawer-drafts", "drawer-replies"],
        "0/0/0/1/1/2/1/1/1/0/0/1/0/0/1/0/1/0/0",
        "right",
        "base",
        "#1474: a draft or reply row past a narrow drawer",
    ),
    (
        &["drawer-drafts", "drawer-replies"],
        "0/0/0/1/1/2/1/1/1/0/0/1/0/0/1/0/2/0/0",
        "right",
        "base",
        "#1474: a draft or reply row past a narrow drawer",
    ),
];

/// The session header's title text (`session.header`'s first line, feature 1463), whose ellipsis
/// is painted past a title squeezed to a few pixels at the smallest window: filed as #1501, not a
/// review layout. Allowed only there, only for the ellipsis, and it must keep firing.
const HEADER_TITLE_TEXT: &str = "0/0/0/1/1/0/0/0/0";

/// Whether `overflow`, in a review state at `window`, is the cut header title of #1501.
fn header_title_cut(window: Size, overflow: &lay::Overflow) -> bool {
    window == MIN_WINDOW_SIZE && overflow.content == "…" && overflow.node_path == HEADER_TITLE_TEXT
}

/// The [`SMALLEST_WINDOW`] entry `escape` falls under, in the state `name` at `window`.
fn known_at(name: &str, window: Size, escape: &lay::Escape) -> Option<&'static str> {
    if window != MIN_WINDOW_SIZE {
        return None;
    }
    SMALLEST_WINDOW
        .iter()
        .find(|(states, parent, edge, layer, _)| {
            states.contains(&name)
                && escape.parent_path == *parent
                && escape.edge == *edge
                && escape.layer.token() == *layer
        })
        .map(|(_, parent, ..)| *parent)
}

/// The registered state named `name`.
fn covered(name: &str) -> &'static CoveredState {
    covered_states()
        .iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("the covered state {name} exists"))
}

fn rect(record: &LayoutRecord) -> Rectangle {
    Rectangle {
        x: record.x,
        y: record.y,
        width: record.width,
        height: record.height,
    }
}

fn inside(part: &Rectangle, host: &Rectangle) -> bool {
    part.x >= host.x - TOLERANCE
        && part.y >= host.y - TOLERANCE
        && part.x + part.width <= host.x + host.width + TOLERANCE
        && part.y + part.height <= host.y + host.height + TOLERANCE
}

/// The base record at `path`.
fn at<'r>(records: &'r [LayoutRecord], path: &[usize]) -> Option<&'r LayoutRecord> {
    records
        .iter()
        .find(|r| r.layer == lay::Layer::Base && r.path == path)
}

/// `under` in `theme`, laid out in `window` with entrances settled: what overflowed and the boxes
/// of both layers.
fn laid_out(
    mut under: StateUnderTest,
    theme: ThemePreference,
    window: Size,
) -> (Vec<lay::Overflow>, Vec<LayoutRecord>) {
    under.state.settings.theme_pref = theme;
    under.state.window.window_size = (window.width as u16, window.height as u16);
    lay::text_overflows_settled_at(lay::view_of(&under), &mut lay::renderer(), window)
}

/// What one review state named `name` breaks, in `theme` at `window`, against its own build: one
/// line per finding, empty when it fits. The status bar's own check panics on its own.
fn findings(
    name: &str,
    theme: ThemePreference,
    window: Size,
    fired: &mut std::collections::BTreeSet<(String, &'static str)>,
) -> Vec<String> {
    let what = format!("{name} {theme:?} {window:?}");
    let build = covered(name).build;
    let (mut overflows, records) = laid_out(build(), theme, window);
    let before = overflows.len();
    overflows.retain(|o| !header_title_cut(window, o));
    if overflows.len() < before {
        fired.insert((name.to_string(), "#1501"));
    }
    let mut found = Vec::new();
    let mut check = |holds: bool, finding: String| {
        if !holds {
            found.push(finding);
        }
    };

    check(
        overflows.is_empty(),
        format!(
            "{what}: text is painted wider than its clip: {:?}",
            overflows
                .iter()
                .map(|o| format!(
                    "{:?} wants {:.1}px in {:.1}px at {}",
                    o.content, o.natural_width, o.allowed_width, o.node_path
                ))
                .collect::<Vec<_>>()
        ),
    );

    let escapes: Vec<String> = lay::escapes(&records, TOLERANCE)
        .into_iter()
        .filter(|e| clips_deliberately(name, e).is_none())
        .filter(|e| match known_at(name, window, e) {
            Some(entry) => {
                fired.insert((name.to_string(), entry));
                false
            }
            None => true,
        })
        .map(|e| {
            format!(
                "{} escapes {} by {:.1}px past its {} edge ({})",
                e.child_path,
                e.parent_path,
                e.overhang,
                e.edge,
                e.layer.token()
            )
        })
        .collect();
    check(
        escapes.is_empty(),
        format!("{what}: nodes are laid out outside the parent that owns them: {escapes:#?}"),
    );

    let status = rect(assert_the_bar_holds_its_place(&what, &records, window));
    let screen = Rectangle::with_size(window);
    if let Some(bar) = at(&records, REVIEW_BAR) {
        let bar = rect(bar);
        check(
            bar.y + bar.height <= status.y + TOLERANCE,
            format!("{what}: the review bar {bar:?} ends below the status bar's top {status:?}"),
        );
        check(
            inside(&bar, &screen) && inside(&status, &screen),
            format!(
                "{what}: the review bar {bar:?} and the status bar {status:?} lie inside the \
                 window"
            ),
        );
    }

    if let Some(drawer) = at(&records, REVIEW_DRAWER) {
        let drawer = rect(drawer);
        let pane = rect(at(&records, REVIEW_DIFF_PANE).expect("an open drawer lies over a diff"));
        check(
            inside(&drawer, &pane),
            format!("{what}: the drawer {drawer:?} lies inside the diff pane {pane:?}"),
        );
        let mut closed = build();
        changes::update(&mut closed.state.changes, Msg::Drawer(DrawerEvent::Toggle));
        let (_, closed) = laid_out(closed, theme, window);
        assert!(
            at(&closed, REVIEW_DRAWER).is_none(),
            "{what}: premise: the toggle closed the drawer"
        );
        let without = rect(at(&closed, REVIEW_DIFF_PANE).expect("the diff pane is laid out"));
        check(
            (pane.width - without.width).abs() <= TOLERANCE,
            format!(
                "{what}: the drawer changed the diff pane's width from {} to {}",
                without.width, pane.width
            ),
        );
    }
    found
}

/// FR-007: every review state, both themes, both windows.
#[test]
fn every_review_state_fits_at_both_windows_in_both_themes() {
    let mut found = Vec::new();
    let mut fired = std::collections::BTreeSet::new();
    for (_, name, _) in REVIEW_STATES {
        for theme in [ThemePreference::Light, ThemePreference::Dark] {
            for window in WINDOWS {
                found.extend(findings(name, theme, window, &mut fired));
            }
        }
    }
    assert!(found.is_empty(), "{}", found.join("\n"));
    let silent: Vec<_> = SMALLEST_WINDOW
        .iter()
        .flat_map(|(states, parent, ..)| states.iter().map(move |state| (*state, *parent)))
        .filter(|(state, parent)| !fired.contains(&(state.to_string(), *parent)))
        .collect();
    assert!(
        silent.is_empty(),
        "these smallest-window exemptions, as (state, parent), no longer fire; drop them: \
         {silent:?}"
    );
    assert!(
        fired.iter().any(|(_, entry)| *entry == "#1501"),
        "the cut header title of #1501 no longer overflows at the smallest window; drop \
         HEADER_TITLE_TEXT"
    );
    for (_, name, _) in REVIEW_STATES {
        assert_fits_under(name, covered(name).build);
    }
}

/// FR-006, SC-002: each review state of the contract is a covered state, anchored on the elements
/// it introduces.
#[test]
fn every_review_state_of_the_contract_is_covered_with_its_anchors() {
    for (contract, name, anchors) in REVIEW_STATES {
        let state = covered(name);
        for anchor in *anchors {
            assert!(
                state.anchors.iter().any(|a| a.name == *anchor),
                "{contract} ({name}) anchors {anchor}"
            );
        }
    }
}
