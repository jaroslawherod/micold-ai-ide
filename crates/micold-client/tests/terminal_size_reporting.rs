//! BUG-003 (`006-real-terminal-emulator` FR-014a, SC-011) — the terminal area reports its size in
//! cells from the frame a session is *displayed*, not from the frame its first output arrives.
//!
//! The first half of that bug was that a size was only ever sent to the service as a *change*. The
//! second half, found once the first was fixed, is narrower and was invisible to every existing
//! test: `TerminalPane` — the only thing that measured the terminal area — is mounted only once a
//! session is displayed **and** its first grid frame has arrived. Until then the same rectangle
//! holds a "Starting…" placeholder, which measured nothing. So on a cold start the app had never
//! measured anything at the moment the user clicked their first session, `App::last_grid` was
//! `None`, and no size could be stated ahead of the start — the very case FR-014a is about.
//!
//! The measurement now wraps the terminal *area* rather than living in its contents
//! (`GridSizeReporter`), so it exists during "Starting…" — a frame or more before the process is
//! spawned, which is what lets the service seed the spawn from it (`010` FR-020a).
//!
//! This resolves the real widget tree headlessly — no display, no GPU — and dispatches one redraw,
//! which is what the runtime does on every frame.

mod support;

use iced::advanced::widget::Tree;
use iced::advanced::{clipboard, layout, mouse, Shell};
use iced::{Element, Rectangle, Size};
use micold_client::features::sidebar;

use micold_client::app::{Message, State};
use micold_client::features::connection::ConnectionStatus;
use micold_client::features::session::Msg as SessionMsg;
use micold_core::env_include::EnvIncludeOutcome;

use support::layout::{self as lay, WINDOW};

fn view(state: &State) -> Element<'_, Message> {
    micold_client::ui::view(
        state,
        None,
        None,
        0,
        None,
        &EnvIncludeOutcome::Disabled,
        &ConnectionStatus::Connected,
    )
}

/// The messages the first frame of `state` publishes.
fn first_frame_messages(state: &State) -> Vec<Message> {
    let renderer = lay::renderer();
    let mut element = view(state);
    let mut tree = Tree::new(element.as_widget());
    let limits = layout::Limits::new(Size::ZERO, WINDOW);
    let node = element
        .as_widget_mut()
        .layout(&mut tree, &renderer, &limits);

    let mut messages = Vec::new();
    let mut clipboard = clipboard::Null;
    let mut shell = Shell::new(&mut messages);
    let redraw = iced::Event::Window(iced::window::Event::RedrawRequested(
        std::time::Instant::now(),
    ));

    element.as_widget_mut().update(
        &mut tree,
        &redraw,
        layout::Layout::new(&node),
        mouse::Cursor::Unavailable,
        &renderer,
        &mut clipboard,
        &mut shell,
        &Rectangle::with_size(WINDOW),
    );

    messages
}

/// The reported grid, if the frame reported one.
fn reported_grid(messages: &[Message]) -> Option<(u16, u16)> {
    messages.iter().find_map(|m| match m {
        Message::Session(SessionMsg::TerminalResized { cols, rows }) => Some((*cols, *rows)),
        _ => None,
    })
}

/// A session has just been selected in a freshly-launched app: it is the displayed session, and no
/// grid frame has arrived for it yet, so the terminal area holds the "Starting…" placeholder. This
/// is the exact frame at which a cold start must learn its size — the start request has gone out and
/// the service has not finished spawning.
fn session_displayed_before_its_first_frame() -> State {
    let session = support::running_default_session();
    let id = session.id;
    let mut workspace = support::workspace_with(vec![("/tmp/project", vec![session])]);
    workspace.active = workspace.projects.first().map(|p| p.path.clone());
    let state = State {
        sidebar: sidebar::State {
            width: 260,
            ..Default::default()
        },

        workspace,
        active_session: Some(id),
        ..State::default()
    };
    assert_eq!(
        state.active_session,
        Some(id),
        "the premise is that this session is displayed"
    );
    state
}

#[test]
fn the_terminal_area_reports_its_size_before_any_output_arrives() {
    let messages = first_frame_messages(&session_displayed_before_its_first_frame());

    let (cols, rows) = reported_grid(&messages).expect(
        "a displayed session's terminal area must report its size while it is still starting — \
         waiting for the first grid frame is what left a cold start with no size to state (FR-014a)",
    );

    // Not the daemon's 100×30 spawn seed, and not a degenerate 1×1: this is a real measurement of
    // the area beside a 260px sidebar in a 1280×800 window. The exact numbers follow from the cell
    // metrics and are pinned by `grid_size_fits_cells_and_floors`; what matters here is that a
    // measurement exists and describes that rectangle.
    assert!(
        cols > 100 && rows > 30,
        "reported {cols}×{rows}, which is no larger than the seed it exists to replace"
    );
    assert!(
        (cols as f32) < WINDOW.width && (rows as f32) < WINDOW.height,
        "reported {cols}×{rows}, which is more cells than the window has pixels"
    );
}

/// A second frame with nothing changed reports nothing: the size is state, and re-sending it would
/// put a `SessionResize` on the wire for every frame the application draws.
#[test]
fn an_unchanged_terminal_area_reports_nothing_on_later_frames() {
    let state = session_displayed_before_its_first_frame();
    let renderer = lay::renderer();
    let mut element = view(&state);
    let mut tree = Tree::new(element.as_widget());
    let limits = layout::Limits::new(Size::ZERO, WINDOW);
    let node = element
        .as_widget_mut()
        .layout(&mut tree, &renderer, &limits);
    let viewport = Rectangle::with_size(WINDOW);
    let mut clipboard = clipboard::Null;

    let mut frame = || {
        let mut messages = Vec::new();
        let mut shell = Shell::new(&mut messages);
        element.as_widget_mut().update(
            &mut tree,
            &iced::Event::Window(iced::window::Event::RedrawRequested(
                std::time::Instant::now(),
            )),
            layout::Layout::new(&node),
            mouse::Cursor::Unavailable,
            &renderer,
            &mut clipboard,
            &mut shell,
            &viewport,
        );
        messages
    };

    assert!(
        reported_grid(&frame()).is_some(),
        "the first frame reports the size"
    );
    assert!(
        reported_grid(&frame()).is_none(),
        "a frame that changed nothing must not report the size again"
    );
}
