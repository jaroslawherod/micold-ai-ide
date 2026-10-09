//! Feature 487 (FR-005): a file dropped on the window goes to the pane under the pointer, focused
//! or not, and to none when the pointer is over no pane. The same gate style as feature 484's pane
//! geometry tests: the layout is the real one, only the pointer is a number.

use micold_core::pane_layout::{Axis, PaneLayout};
use micold_core::protocol::messages::{SessionProcess, TerminalRef};
use micold_core::session::SessionId;

const AREA: (f32, f32) = (1000.0, 600.0);
const MIN: (f32, f32) = (10.0, 10.0);

fn terminal() -> TerminalRef {
    TerminalRef {
        session: SessionId::new(),
        process: SessionProcess::Primary,
    }
}

/// `[left | right]` at 30/70, the right pane focused.
fn split() -> (
    PaneLayout,
    micold_core::pane_layout::PaneId,
    micold_core::pane_layout::PaneId,
) {
    let mut l = PaneLayout::single();
    let left = l.focused();
    l.show(left, terminal()).unwrap();
    l.split(left, Axis::Vertical, AREA, MIN, Some(terminal()))
        .unwrap();
    l.set_ratio(0, 0.3, AREA, MIN);
    let right = l.focused();
    assert_ne!(left, right);
    (l, left, right)
}

#[test]
fn the_drop_target_is_the_pane_under_the_pointer_even_when_unfocused() {
    let (layout, left, right) = split();
    assert_eq!(layout.focused(), right);
    assert_eq!(layout.pane_at(AREA, MIN, (100.0, 300.0)), Some(left));
    assert_eq!(layout.pane_at(AREA, MIN, (800.0, 300.0)), Some(right));
}

#[test]
fn a_pointer_outside_every_pane_has_no_target() {
    let (layout, _, _) = split();
    for p in [(-1.0, 10.0), (10.0, -1.0), (1000.0, 10.0), (10.0, 600.0)] {
        assert_eq!(layout.pane_at(AREA, MIN, p), None, "{p:?}");
    }
}

#[test]
fn a_single_pane_takes_every_drop_inside_the_area() {
    let layout = PaneLayout::single();
    assert_eq!(
        layout.pane_at(AREA, MIN, (500.0, 300.0)),
        Some(layout.focused())
    );
}

/// The widget is what carries the pointer to that function, and the pane list is what carries the
/// answer to the terminal: both are wired where the panes are drawn.
#[test]
fn the_pane_tiles_report_file_drops() {
    let src = include_str!("../src/ui/panes.rs");
    assert!(src.contains(".on_file_drop("), "ui/panes.rs");
    let split = include_str!("../src/ui/material/split_view.rs");
    assert!(split.contains("FileDropped"), "split_view.rs");
    assert!(split.contains(".pane_at("), "split_view.rs");
}
