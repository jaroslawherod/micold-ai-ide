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
    let mut messages = publish_for(&[
        Event::Mouse(mouse::Event::CursorMoved {
            position: Point::new(pointer.0, pointer.1),
        }),
        Event::Window(window::Event::FileDropped(PathBuf::from(path))),
    ]);
    // The view also reports its pane sizes; only the drops matter here.
    messages.retain(|m| dropped(m).is_some());
    messages
}

/// What the real pane view publishes for `events`, in order.
fn publish_for(events: &[Event]) -> Vec<Message> {
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
    for event in events {
        element.as_widget_mut().update(
            &mut tree,
            event,
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &renderer,
            &mut clipboard,
            &mut shell,
            &viewport,
        );
    }
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

/// Feature 941 (FR-026, R11): what a pane shows decides where its drop goes. A structured
/// session's primary is drawn as its transcript over the composer, so the composer takes it; a
/// terminal stays a terminal, an empty pane stays empty, and a pane gone by the settle is outside.
#[test]
fn a_pane_showing_a_structured_primary_is_the_composers_drop_target() {
    use micold_client::features::session::{target_of, DropTarget};
    use micold_core::session::{AiCli, Session, SessionKind, SessionLocation};
    let start = |kind| {
        Session::start_new_of_kind(
            SessionLocation::Worktree("feat-00".into()),
            AiCli::ClaudeCode,
            kind,
        )
    };
    let (structured, terminal) = (start(SessionKind::Structured), start(SessionKind::Terminal));
    let (s, t) = (structured.id, terminal.id);
    let mut workspace =
        support::workspace_with(vec![("/fixture/project", vec![structured, terminal])]);
    workspace.active = workspace.projects.first().map(|p| p.path.clone());
    let state = State {
        workspace,
        ..State::default()
    };
    let primary = |session| TerminalRef {
        session,
        process: SessionProcess::Primary,
    };
    assert_eq!(
        target_of(&state, Some(Some(primary(s)))),
        DropTarget::Composer(s)
    );
    assert_eq!(
        target_of(&state, Some(Some(primary(t)))),
        DropTarget::Terminal(primary(t))
    );
    assert_eq!(target_of(&state, Some(None)), DropTarget::EmptyPane);
    assert_eq!(target_of(&state, None), DropTarget::Outside);
}

fn hovering(m: &Message) -> Option<bool> {
    match m {
        Message::Session(SessionMsg::Pane(PaneMsg::FilesHovering(on))) => Some(*on),
        _ => None,
    }
}

/// #1464 FR-005, SC-005: the pane view reports the window's file hover, and the end of it by the
/// drag leaving or by the drop, so the flag the cue reads is never left set.
#[test]
fn the_pane_view_reports_a_hover_start_and_its_end_by_leave_or_drop() {
    let hovered = |p: &str| Event::Window(window::Event::FileHovered(PathBuf::from(p)));
    let left = || Event::Window(window::Event::FilesHoveredLeft);
    let dropped = |p: &str| Event::Window(window::Event::FileDropped(PathBuf::from(p)));
    let hovers = |events: &[Event]| -> Vec<bool> {
        publish_for(events).iter().filter_map(hovering).collect()
    };
    assert_eq!(hovers(&[hovered("/a")]), [true]);
    assert_eq!(hovers(&[hovered("/a"), left()]), [true, false]);
    assert_eq!(hovers(&[hovered("/a"), dropped("/a")]), [true, false]);
    // One event per dragged file, one message per change.
    assert_eq!(
        hovers(&[hovered("/a"), hovered("/b"), dropped("/a"), dropped("/b")]),
        [true, false]
    );
}

/// #1464 FR-005, US2 scenario 3: only a structured session that accepts input, while files hover,
/// wears the cue; a terminal pane and a session that cannot take input do not.
#[test]
fn only_a_structured_pane_that_accepts_input_wears_the_cue_while_files_hover() {
    use micold_client::features::composer::{InputState, TurnPhase};
    use micold_client::features::session::shows_drop_cue;
    use micold_core::session::{AiCli, Session, SessionKind, SessionLocation};
    let start = |kind| {
        Session::start_new_of_kind(
            SessionLocation::Worktree("feat-00".into()),
            AiCli::ClaudeCode,
            kind,
        )
    };
    let (structured, ended, plain) = (
        start(SessionKind::Structured),
        start(SessionKind::Structured),
        start(SessionKind::Terminal),
    );
    let (s, e, t) = (structured.id, ended.id, plain.id);
    let mut workspace =
        support::workspace_with(vec![("/fixture/project", vec![structured, ended, plain])]);
    workspace.active = workspace.projects.first().map(|p| p.path.clone());
    let mut state = State {
        workspace,
        ..State::default()
    };
    let report = |state: &mut State, id, phase| {
        state.composers.input_reported(
            id,
            InputState {
                phase,
                hold: false,
                queue: Vec::new(),
            },
        );
    };
    report(&mut state, s, TurnPhase::Idle);
    report(&mut state, e, TurnPhase::NotRunning);
    report(&mut state, t, TurnPhase::Idle);
    let primary = |session| TerminalRef {
        session,
        process: SessionProcess::Primary,
    };
    for id in [s, e, t] {
        assert!(!shows_drop_cue(&state, primary(id)), "nothing hovers yet");
    }
    state.session.files_hovering = true;
    assert!(shows_drop_cue(&state, primary(s)));
    assert!(!shows_drop_cue(&state, primary(e)), "cannot accept input");
    assert!(!shows_drop_cue(&state, primary(t)), "a terminal");
    state.session.files_hovering = false;
    assert!(
        !shows_drop_cue(&state, primary(s)),
        "gone after leave or drop"
    );
}
