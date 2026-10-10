//! BUG-748: Ctrl/Cmd held over a link in the terminal pane switches the window's cursor to the
//! hand.
//!
//! The pane's own tests drive the widget alone and pass; this drives the application's whole view,
//! the way the runtime does: events in, a rebuilt tree when the pane marks the tree stale, a
//! redraw, and the interaction read from the root.

mod support;

use iced::advanced::widget::Tree;
use iced::advanced::{clipboard, layout, mouse, Layout};
use iced::{keyboard, Element, Event, Point, Size};
use iced_runtime::user_interface::{Cache, State as UiState, UserInterface};

use micold_client::app::{Message, State};
use micold_client::features::connection::ConnectionStatus;
use micold_client::features::sandbox::Sandbox;
use micold_client::grid::GridCache;
use micold_client::ui::panes::PaneArea;
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::protocol::grid::{
    GridFrame, LineId, StyleRun, WireColor, WireCursor, WireCursorShape, WireLine, WireStyle,
};
use micold_core::protocol::messages::SessionProcess;
use micold_core::session::SessionId;

use support::layout::{renderer, WINDOW};

const COLS: u16 = 80;
const SENTENCE: &str = "See https://example.com/docs for details";

fn grid_showing(session: SessionId) -> GridCache {
    let text = format!("{SENTENCE:<width$}", width = COLS as usize);
    let mut grid = GridCache::new();
    grid.apply(&GridFrame {
        process: SessionProcess::Primary,
        session,
        seq: 1,
        generation: 1,
        full: true,
        viewport_top: LineId(0),
        oldest_available: LineId(0),
        cols: COLS,
        rows: 1,
        cursor: WireCursor {
            line: LineId(0),
            col: 0,
            shape: WireCursorShape::Block,
            visible: false,
            blinking: false,
        },
        styles: vec![WireStyle {
            fg: WireColor::Named(256),
            bg: WireColor::Named(257),
            flags: 0,
            underline_color: None,
        }],
        hyperlinks: Vec::new(),
        lines: vec![WireLine {
            id: LineId(0),
            runs: vec![StyleRun {
                len: COLS,
                style: 0,
            }],
            text,
            extras: Vec::new(),
            wrapped: false,
        }],
        mode: 0,
        input_serial: None,
    });
    grid
}

fn build_view<'a>(
    state: &'a State,
    grid: &'a GridCache,
    outcome: &'a EnvIncludeOutcome,
    connection: &ConnectionStatus,
    sandbox: &Sandbox,
    panes: Option<&PaneArea<'a>>,
) -> Element<'a, Message> {
    micold_client::ui::view_with(
        state,
        Some(grid),
        None,
        0,
        None,
        outcome,
        connection,
        sandbox,
        None,
        panes,
    )
}

fn origins(layout: Layout<'_>, found: &mut Vec<Point>) {
    let bounds = layout.bounds();
    let mut leaf = true;
    for child in layout.children() {
        leaf = false;
        origins(child, found);
    }
    // The pane is a leaf of the layout, and the largest one the view has.
    if leaf && bounds.width > 300.0 && bounds.height > 100.0 {
        found.push(bounds.position());
    }
}

/// Whether Ctrl over the link somewhere in the window shows the hand, through the real interface.
fn hand_shows(state: &State, grid: &GridCache, area: Option<&PaneArea<'_>>) -> bool {
    let sandbox = Sandbox::default();
    let outcome = EnvIncludeOutcome::Disabled;
    let connection = ConnectionStatus::Connected;
    let mut renderer = renderer();
    let mut cache = Cache::default();

    // One step as the runtime takes it: the event through the user interface, and a redraw on the
    // tree rebuilt from its cache; the pointer is what the interface reported last.
    macro_rules! step {
        ($cursor:expr, $event:expr $(,)?) => {{
            let cursor: Point = $cursor;
            let mut interaction = mouse::Interaction::None;
            for event in [
                $event,
                Event::Window(iced::window::Event::RedrawRequested(
                    std::time::Instant::now(),
                )),
            ] {
                let mut ui = UserInterface::build(
                    build_view(state, grid, &outcome, &connection, &sandbox, area),
                    WINDOW,
                    std::mem::take(&mut cache),
                    &mut renderer,
                );
                let mut messages = Vec::new();
                let (shown, _) = ui.update(
                    &[event],
                    mouse::Cursor::Available(cursor),
                    &mut renderer,
                    &mut clipboard::Null,
                    &mut messages,
                );
                if let UiState::Updated {
                    mouse_interaction, ..
                } = shown
                {
                    interaction = mouse_interaction;
                }
                cache = ui.into_cache();
            }
            interaction
        }};
    }

    // Let the content fade finish: it takes its frames from redraws.
    for _ in 0..3 {
        step!(
            Point::ORIGIN,
            Event::Mouse(mouse::Event::CursorMoved {
                position: Point::ORIGIN,
            }),
        );
        std::thread::sleep(std::time::Duration::from_millis(150));
    }

    // The pane's place is not known here: try the spots a cell of its first row could be at under
    // every large box the view laid out.
    let mut candidates = Vec::new();
    {
        let mut ui = UserInterface::build(
            build_view(state, grid, &outcome, &connection, &sandbox, area),
            WINDOW,
            std::mem::take(&mut cache),
            &mut renderer,
        );
        // Draw once so the layout is in the interface; read it through a scratch element tree.
        ui.draw(
            &mut renderer,
            &iced::Theme::Dark,
            &iced::advanced::renderer::Style {
                text_color: iced::Color::WHITE,
            },
            mouse::Cursor::Unavailable,
        );
        cache = ui.into_cache();
    }
    {
        let mut element = build_view(state, grid, &outcome, &connection, &sandbox, area);
        let mut tree = Tree::new(&element);
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, WINDOW),
        );
        origins(Layout::new(&node), &mut candidates);
    }
    candidates.dedup();
    assert!(
        !candidates.is_empty(),
        "the view laid out a leaf box as large as a pane"
    );

    let mut hand = false;
    'sweep: for origin in candidates {
        for dx in [40.0, 80.0, 120.0, 160.0, 200.0] {
            for dy in [6.0, 10.0, 14.0, 18.0] {
                let at = Point::new(origin.x + dx, origin.y + dy);
                // Ctrl first, then the pointer arrives on the link: the order a user takes.
                let plain = step!(at, Event::Mouse(mouse::Event::CursorMoved { position: at }),);
                if plain != mouse::Interaction::Text {
                    continue;
                }
                step!(
                    Point::ORIGIN,
                    Event::Keyboard(keyboard::Event::ModifiersChanged(
                        keyboard::Modifiers::COMMAND,
                    )),
                );
                let held = step!(at, Event::Mouse(mouse::Event::CursorMoved { position: at }),);
                step!(
                    at,
                    Event::Keyboard(keyboard::Event::ModifiersChanged(
                        keyboard::Modifiers::empty(),
                    )),
                );
                if held == mouse::Interaction::Pointer {
                    hand = true;
                    break 'sweep;
                }
            }
        }
    }
    hand
}

fn state_with_active_session(id: SessionId) -> State {
    let mut workspace = support::workspace_with(vec![("/fixture/project", vec![])]);
    workspace.active = workspace.projects.first().map(|p| p.path.clone());
    let mut state = State {
        workspace,
        ..State::default()
    };
    state.session.active = Some(id);
    state
}

#[test]
fn ctrl_held_over_a_link_shows_the_hand_in_a_single_pane() {
    let id = SessionId::new();
    let state = state_with_active_session(id);
    let grid = grid_showing(id);
    assert!(
        hand_shows(&state, &grid, None),
        "holding Ctrl over a link in the terminal pane shows the hand (BUG-748)"
    );
}

#[test]
fn ctrl_held_over_a_link_shows_the_hand_in_split_panes() {
    use micold_core::pane_layout::{Axis, PaneLayout};
    use micold_core::protocol::messages::TerminalRef;
    use std::collections::HashMap;

    let (a, b) = (SessionId::new(), SessionId::new());
    let state = state_with_active_session(b);
    let terminal = |session| TerminalRef {
        session,
        process: SessionProcess::Primary,
    };
    let mut layout = PaneLayout::single();
    let first = layout.focused();
    layout.show(first, terminal(a)).expect("first pane");
    layout
        .split(
            first,
            Axis::Vertical,
            (1000.0, 700.0),
            (100.0, 100.0),
            Some(terminal(b)),
        )
        .expect("split");
    let grids: HashMap<TerminalRef, GridCache> = [
        (terminal(a), grid_showing(a)),
        (terminal(b), grid_showing(b)),
    ]
    .into_iter()
    .collect();
    let area = PaneArea {
        layout: &layout,
        grids: &grids,
        refusal: None,
    };
    let attached = grid_showing(b);
    assert!(
        hand_shows(&state, &attached, Some(&area)),
        "holding Ctrl over a link in a split pane shows the hand (BUG-748)"
    );
}
