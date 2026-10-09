//! Feature 487 (FR-005): a file dropped on the window goes to the pane under the pointer, focused
//! or not, and to none when the pointer is over no pane. The same gate style as feature 484's pane
//! geometry tests: the layout is the real one, only the pointer is a number.

mod support;

use std::collections::HashMap;
use std::path::PathBuf;

use iced::advanced::widget::Tree;
use iced::advanced::{layout, mouse, Layout, Shell};
use iced::{window, Event, Point, Rectangle, Size};

use micold_client::app::{Message, State};
use micold_client::features::session::{Msg as SessionMsg, PaneMsg};
use micold_client::ui::panes::{self, PaneArea};
use micold_core::link::LinkContext;
use micold_core::pane_layout::{Axis, PaneId, PaneLayout};
use micold_core::protocol::messages::{SessionProcess, TerminalRef};
use micold_core::session::SessionId;
use micold_core::theme::ColorScheme;
use support::layout::{renderer, WINDOW};

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

/// Drive the real pane view (`ui::panes::view`, so the `SplitView` and its `.on_file_drop(`
/// wiring) with a pointer move and a file drop, and return what it published.
fn drop_at(pointer: (f32, f32), path: &str) -> Vec<Message> {
    let (layout, _, _) = split();
    let state = State::default();
    let grids = HashMap::new();
    let area = PaneArea {
        layout: &layout,
        grids: &grids,
        refusal: None,
    };
    let links = LinkContext {
        host_names: Vec::new(),
        windows_host: false,
        sandbox: None,
    };
    let mut element = panes::view(&state, &area, None, 0, ColorScheme::Dark, &links);
    let renderer = renderer();
    let mut tree = Tree::new(element.as_widget());
    let limits = layout::Limits::new(Size::ZERO, WINDOW);
    let node = element
        .as_widget_mut()
        .layout(&mut tree, &renderer, &limits);
    let viewport = Rectangle::with_size(WINDOW);
    let mut messages = Vec::new();
    let mut shell = Shell::new(&mut messages);
    let mut clipboard = iced::advanced::clipboard::Null;
    for event in [
        Event::Mouse(mouse::Event::CursorMoved {
            position: Point::new(pointer.0, pointer.1),
        }),
        Event::Window(window::Event::FileDropped(PathBuf::from(path))),
    ] {
        element.as_widget_mut().update(
            &mut tree,
            &event,
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &renderer,
            &mut clipboard,
            &mut shell,
            &viewport,
        );
    }
    // The view also reports its pane sizes; only the drops matter here.
    messages.retain(|m| dropped(m).is_some());
    messages
}

fn dropped(m: &Message) -> Option<(PaneId, &PathBuf)> {
    match m {
        Message::Session(SessionMsg::Pane(PaneMsg::FileDropped(pane, path))) => Some((*pane, path)),
        _ => None,
    }
}

/// The widget is what carries the pointer to `pane_at`, and the pane list is what carries the
/// answer to the terminal: both are wired where the panes are drawn.
#[test]
fn a_file_dropped_on_the_pane_view_is_reported_with_the_pane_under_the_pointer() {
    let (_, left, right) = split();
    let got = drop_at((100.0, 300.0), "/tmp/a.png");
    let [m] = got.as_slice() else {
        panic!("{got:?}");
    };
    assert_eq!(dropped(m), Some((left, &PathBuf::from("/tmp/a.png"))));
    let got = drop_at((1000.0, 300.0), "/tmp/b.png");
    let [m] = got.as_slice() else {
        panic!("{got:?}");
    };
    assert_eq!(dropped(m), Some((right, &PathBuf::from("/tmp/b.png"))));
}
