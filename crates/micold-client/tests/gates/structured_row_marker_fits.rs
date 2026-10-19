//! A structured session's sidebar row keeps its marker inside the row (feature 938, T040, FR-022).
//!
//! The same sidebar is laid out twice at the narrowest sidebar width, once with the session a
//! terminal session and once structured. What differs between the two is the marker: it must be
//! there, it must sit inside the row that holds it, and the row must keep its size (a session row
//! is one line; the marker changes its text, never its height).
//!
//! Compiled into the `layout_snapshot` binary beside `context_menu_anchor`, whose fixture it
//! reuses.

use crate::context_menu_anchor::with_worktrees;
use crate::support::layout::{self as lay, LayoutRecord};
use micold_client::app::{State, SIDEBAR_MIN_WIDTH};
use micold_client::features::connection::ConnectionStatus;
use micold_core::session::{AiCli, Session, SessionKind, SessionLocation};

/// Half a pixel, matching every geometry gate beside this one.
const TOLERANCE: f32 = 0.5;

fn records_with(kind: SessionKind) -> Vec<LayoutRecord> {
    let session = Session::start_new_of_kind(
        SessionLocation::Worktree("feat-00".into()),
        AiCli::ClaudeCode,
        kind,
    );
    let mut state: State = with_worktrees(vec![session], 3, crate::support::layout::WINDOW);
    state.sidebar.width = SIDEBAR_MIN_WIDTH;
    let element = micold_client::ui::view(
        &state,
        None,
        None,
        0,
        None,
        &micold_core::env_include::EnvIncludeOutcome::Disabled,
        &ConnectionStatus::Connected,
        &micold_client::features::sandbox::Sandbox::default(),
    );
    lay::resolve(element, &lay::renderer())
}

fn same(a: &LayoutRecord, b: &LayoutRecord) -> bool {
    (a.x - b.x).abs() <= TOLERANCE
        && (a.y - b.y).abs() <= TOLERANCE
        && (a.width - b.width).abs() <= TOLERANCE
        && (a.height - b.height).abs() <= TOLERANCE
}

#[test]
fn a_structured_rows_marker_stays_inside_its_row_at_the_narrowest_sidebar() {
    let plain = records_with(SessionKind::Terminal);
    let marked = records_with(SessionKind::Structured);
    assert_eq!(
        plain.len(),
        marked.len(),
        "the marker is text on an existing row, not a new element"
    );

    // The sidebar's session rows are the nodes the marker lives in; what it changed is the
    // records that differ. Their deepest common ancestor is the row.
    let changed: Vec<&LayoutRecord> = marked
        .iter()
        .filter(|m| {
            plain
                .iter()
                .find(|p| p.path == m.path)
                .is_none_or(|p| !same(p, m))
        })
        .collect();
    // The annotation is clipped to its slot, so a longer caption may move nothing at all: then the
    // marker trivially fits. The caption's text is pinned by `command_caption`'s own test.
    if changed.is_empty() {
        return;
    }
    let common = changed
        .iter()
        .skip(1)
        .fold(changed[0].path.clone(), |acc, r| {
            acc.iter()
                .zip(&r.path)
                .take_while(|(a, b)| a == b)
                .map(|(a, _)| *a)
                .collect()
        });
    // The row is the nearest ancestor that did not move: walk up to a record equal in both.
    let mut path = common;
    let row = loop {
        let before = plain.iter().find(|r| r.path == path);
        let after = marked.iter().find(|r| r.path == path);
        match (before, after) {
            (Some(b), Some(a)) if same(a, b) => break a,
            _ => {
                assert!(
                    path.pop().is_some(),
                    "no unchanged ancestor: the whole window moved"
                );
            }
        }
    };
    for r in &changed {
        assert!(
            r.x >= row.x - TOLERANCE
                && r.y >= row.y - TOLERANCE
                && r.x + r.width <= row.x + row.width + TOLERANCE
                && r.y + r.height <= row.y + row.height + TOLERANCE,
            "a node the marker changed ({:?}) lies outside the row that holds it ({:?})",
            (r.x, r.y, r.width, r.height),
            (row.x, row.y, row.width, row.height)
        );
    }
}
