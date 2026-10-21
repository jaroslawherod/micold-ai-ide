//! T039 (feature 939, FR-001, US1 scenario 6, SC-006): a structured session's primary draws the
//! transcript view in its pane, single and tiled, in place of the plain log; a terminal session's
//! pane is unchanged.
//!
//! Each case renders the whole window (`ui::view_with`) and reads back which scrollables it holds:
//! the transcript's is named `transcript-<session>` ([`transcript_scroll_id`]), so its presence
//! says which session's transcript a pane draws.

mod support;

use std::collections::HashMap;

use iced::advanced::widget::operation::scrollable::Scrollable;
use iced::advanced::widget::{Id, Operation, Tree};
use iced::advanced::{layout, Layout};
use iced::{Element, Rectangle, Size, Vector};
use micold_client::app::{Message, State};
use micold_client::features::connection::ConnectionStatus;
use micold_client::ui::panes::PaneArea;
use micold_client::ui::transcript::{transcript_scroll_id, NO_OUTPUT};
use micold_core::pane_layout::{Axis, PaneLayout};
use micold_core::protocol::messages::{SessionProcess, TerminalRef};
use micold_core::session::{AiCli, Session, SessionId, SessionKind, SessionLocation};
use support::layout::{painted_text, renderer, WINDOW};
use support::transcript::{fixture_lines, sequenced};

const PROJECT: &str = "/fixture/project";

/// The reply text of the recorded `text-turn` fixture's assistant message.
const REPLY: &str = "A clear daytime sky is blue.";

fn session(kind: SessionKind) -> Session {
    Session::start_new_of_kind(
        SessionLocation::Worktree("feat-00".into()),
        AiCli::ClaudeCode,
        kind,
    )
}

/// A window whose active project holds `sessions`, the first of them active.
fn window_with(sessions: Vec<Session>) -> State {
    let active = sessions.first().map(|s| s.id);
    let mut workspace = support::workspace_with(vec![(PROJECT, sessions)]);
    workspace.active = workspace.projects.first().map(|p| p.path.clone());
    let mut state = State {
        workspace,
        ..State::default()
    };
    state.session.active = active;
    state
}

/// `session`'s history as the recorded fixture `name`, as the wire hands it to the window.
fn feed(state: &mut State, session: SessionId, name: &str) {
    state.structured_events(session, 0, &sequenced(&fixture_lines(name), 0), false);
}

fn primary(session: SessionId) -> TerminalRef {
    TerminalRef {
        session,
        process: SessionProcess::Primary,
    }
}

/// `[left | right]`, showing `left` and `right`'s primaries.
fn tiled(left: SessionId, right: SessionId) -> PaneLayout {
    let area = (WINDOW.width, WINDOW.height);
    let min = (10.0, 10.0);
    let mut layout = PaneLayout::single();
    let first = layout.focused();
    layout.show(first, primary(left)).unwrap();
    layout
        .split(first, Axis::Vertical, area, min, Some(primary(right)))
        .unwrap();
    layout
}

fn view<'a>(state: &'a State, panes: Option<&PaneArea<'a>>) -> Element<'a, Message> {
    micold_client::ui::view_with(
        state,
        None,
        None,
        0,
        None,
        &micold_core::env_include::EnvIncludeOutcome::Disabled,
        &ConnectionStatus::Connected,
        &micold_client::features::sandbox::Sandbox::default(),
        None,
        panes,
    )
}

/// Every scrollable id the laid-out element holds.
struct ScrollIds(Vec<Id>);

impl<T> Operation<T> for ScrollIds {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<T>)) {
        operate(self);
    }

    fn scrollable(
        &mut self,
        id: Option<&Id>,
        _bounds: Rectangle,
        _content_bounds: Rectangle,
        _translation: Vector,
        _state: &mut dyn Scrollable,
    ) {
        self.0.extend(id.cloned());
    }
}

fn scroll_ids(mut element: Element<'_, Message>) -> Vec<Id> {
    let renderer = renderer();
    let mut tree = Tree::new(element.as_widget());
    let limits = layout::Limits::new(Size::ZERO, WINDOW);
    let node = element
        .as_widget_mut()
        .layout(&mut tree, &renderer, &limits);
    let mut ids = ScrollIds(Vec::new());
    element
        .as_widget_mut()
        .operate(&mut tree, Layout::new(&node), &renderer, &mut ids);
    ids.0
}

fn painted(element: Element<'_, Message>) -> Vec<String> {
    painted_text(element, &mut renderer())
        .into_iter()
        .map(|t| t.content)
        .collect()
}

fn shows_transcript(ids: &[Id], session: SessionId) -> bool {
    ids.contains(&transcript_scroll_id(session))
}

#[test]
fn a_structured_sessions_single_pane_draws_its_transcript() {
    let s = session(SessionKind::Structured);
    let id = s.id;
    let mut state = window_with(vec![s]);
    feed(&mut state, id, "text-turn");

    assert!(
        shows_transcript(&scroll_ids(view(&state, None)), id),
        "the pane holds the transcript's scrollable"
    );
    let text = painted(view(&state, None));
    assert!(
        text.iter().any(|t| t.contains(REPLY)),
        "the reply is drawn as an entry: {text:?}"
    );
}

#[test]
fn an_empty_structured_pane_says_no_output_not_the_plain_logs_line() {
    let s = session(SessionKind::Structured);
    let id = s.id;
    let state = window_with(vec![s]);

    assert!(shows_transcript(&scroll_ids(view(&state, None)), id));
    let text = painted(view(&state, None));
    assert!(text.iter().any(|t| t == NO_OUTPUT), "{text:?}");
    assert!(
        !text.iter().any(|t| t == "No events yet."),
        "the plain log's empty line is gone: {text:?}"
    );
}

#[test]
fn tiled_structured_sessions_each_draw_their_own_transcript() {
    let (a, b) = (
        session(SessionKind::Structured),
        session(SessionKind::Structured),
    );
    let (ia, ib) = (a.id, b.id);
    let mut state = window_with(vec![a, b]);
    feed(&mut state, ia, "text-turn");
    feed(&mut state, ib, "tool-turn");
    let layout = tiled(ia, ib);
    let grids = HashMap::new();
    let area = PaneArea {
        layout: &layout,
        grids: &grids,
        refusal: None,
    };

    let ids = scroll_ids(view(&state, Some(&area)));
    assert!(shows_transcript(&ids, ia), "left pane: {ids:?}");
    assert!(shows_transcript(&ids, ib), "right pane: {ids:?}");
}

#[test]
fn a_terminal_session_beside_a_structured_one_keeps_its_terminal_pane() {
    let (t, s) = (
        session(SessionKind::Terminal),
        session(SessionKind::Structured),
    );
    let (it, is) = (t.id, s.id);
    let state = window_with(vec![t, s]);
    let layout = tiled(it, is);
    let grids = HashMap::new();
    let area = PaneArea {
        layout: &layout,
        grids: &grids,
        refusal: None,
    };

    let ids = scroll_ids(view(&state, Some(&area)));
    assert!(!shows_transcript(&ids, it), "{ids:?}");
    assert!(shows_transcript(&ids, is), "{ids:?}");
    let text = painted(view(&state, Some(&area)));
    assert!(
        text.iter().any(|t| t == "Starting…"),
        "the terminal pane still shows its empty-terminal message: {text:?}"
    );
}

#[test]
fn a_terminal_sessions_single_pane_is_unchanged() {
    let t = session(SessionKind::Terminal);
    let id = t.id;
    let state = window_with(vec![t]);

    assert!(!shows_transcript(&scroll_ids(view(&state, None)), id));
    let text = painted(view(&state, None));
    assert!(text.iter().any(|t| t == "Starting…"), "{text:?}");
    assert!(!text.iter().any(|t| t == NO_OUTPUT), "{text:?}");
}
