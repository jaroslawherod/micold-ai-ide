//! Tab walks the composer (feature 941, T061 visual pass F5): from the text area the keyboard
//! reaches **Send**, and Shift+Tab **Send queued** and each queued message's actions, in the
//! window's own focus traversal (`Message::FocusMoved` answers Tab with iced's `focus_next`); and
//! while a composer control holds the keyboard the window keeps it from the pane. Before the fix
//! the text area's blur gave the keyboard back to the pane: `KeyboardElsewhere` then took Send's
//! focus on the next event and the Tab listener closed, so Tab never visibly left the text area.

use crate::support;

use std::sync::{Arc, Mutex};

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::operation::{focusable, Focusable, Outcome};
use iced::advanced::widget::{Id, Operation, Tree};
use iced::advanced::{clipboard, mouse, Shell};
use iced::widget::text_editor::{Action, Edit};
use iced::{keyboard, Element, Event, Point, Rectangle};
use micold_client::app::{EditorAction, Message, State};
use micold_client::features::composer::InputState;
use micold_client::features::connection::ConnectionStatus;
use micold_client::features::window::FieldId;
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::protocol::messages::{QueueEntryId, QueuedSummary, TurnPhase};
use micold_core::session::SessionId;
use support::covered_states::structured_pane;
use support::layout::{self as lay, StateUnderTest, WINDOW};

/// Every focusable the traversal reaches, in order, with whether it holds the keyboard.
#[derive(Clone, Default)]
struct Reached(Arc<Mutex<Vec<(Rectangle, bool)>>>);

struct Record(Reached);

impl Operation for Record {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn focusable(&mut self, _id: Option<&Id>, bounds: Rectangle, state: &mut dyn Focusable) {
        (self.0)
            .0
            .lock()
            .unwrap()
            .push((bounds, state.is_focused()));
    }
}

/// Focus the focusable laid out at exactly `bounds`, and no other.
struct FocusAt(Rectangle);

impl Operation for FocusAt {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn focusable(&mut self, _id: Option<&Id>, bounds: Rectangle, state: &mut dyn Focusable) {
        if bounds == self.0 {
            state.focus();
        } else {
            state.unfocus();
        }
    }
}

/// A mounted view whose focus is moved and read the way the runtime does it.
struct Mounted<'a> {
    element: Element<'a, Message>,
    tree: Tree,
    node: layout::Node,
    renderer: iced::Renderer,
}

impl<'a> Mounted<'a> {
    fn new(mut element: Element<'a, Message>) -> Self {
        let renderer = lay::renderer();
        let mut tree = Tree::new(element.as_widget());
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(iced::Size::ZERO, WINDOW),
        );
        Self {
            element,
            tree,
            node,
            renderer,
        }
    }

    fn operate(&mut self, operation: Box<dyn Operation>) {
        let mut current = operation;
        loop {
            self.element.as_widget_mut().operate(
                &mut self.tree,
                Layout::new(&self.node),
                &self.renderer,
                current.as_mut(),
            );
            match current.finish() {
                Outcome::Chain(next) => current = next,
                Outcome::Some(()) | Outcome::None => break,
            }
        }
    }

    fn reached(&mut self) -> Vec<(Rectangle, bool)> {
        let reached = Reached::default();
        self.operate(Box::new(Record(reached.clone())));
        let out = reached.0.lock().unwrap().clone();
        out
    }

    /// Deliver `event` as the runtime does; the messages the view published.
    fn event(&mut self, event: &Event) -> Vec<Message> {
        let mut messages = Vec::new();
        let mut shell = Shell::new(&mut messages);
        self.element.as_widget_mut().update(
            &mut self.tree,
            event,
            Layout::new(&self.node),
            mouse::Cursor::Unavailable,
            &self.renderer,
            &mut clipboard::Null,
            &mut shell,
            &Rectangle::with_size(WINDOW),
        );
        messages
    }

    /// Focus the focusable drawn on top at `at`, as a click on it does: the last one the traversal
    /// reaches there, since a transcript card scrolled out of its viewport still has bounds
    /// beneath the composer.
    fn focus_at(&mut self, at: Point) {
        let top = self
            .reached()
            .into_iter()
            .map(|(b, _)| b)
            .rfind(|b| b.contains(at))
            .expect("a focusable under the point");
        self.operate(Box::new(FocusAt(top)));
    }

    fn focused(&mut self) -> Option<Rectangle> {
        self.reached().into_iter().find(|(_, f)| *f).map(|(b, _)| b)
    }
}

/// A structured pane, idle, holding two queued messages, with a draft typed.
fn crowded() -> (State, SessionId) {
    let (mut state, id) = structured_pane(&support::transcript::fixture_lines("tool-turn"));
    let queue = (1..=2)
        .map(|n| QueuedSummary {
            id: QueueEntryId(n),
            text: format!("queued message {n}"),
            attachments: Vec::new(),
        })
        .collect();
    state.composers.input_reported(
        id,
        InputState {
            phase: TurnPhase::Idle,
            hold: true,
            queue,
        },
    );
    let editor = state.composers.editor_mut(id);
    let shown = editor.perform(&EditorAction(Action::Edit(Edit::Paste(Arc::new(
        "a draft".to_owned(),
    )))));
    let caret = editor.caret();
    state.composers.edited(id, shown, caret);
    (state, id)
}

/// Where `content` is painted: the centre of its first line.
fn painted_at(under: &StateUnderTest, content: &str) -> Point {
    let mut renderer = lay::renderer();
    let (painted, _) = lay::painted_text_settled_at(lay::view_of(under), &mut renderer, WINDOW);
    let text = painted
        .iter()
        .find(|p| p.content == content)
        .unwrap_or_else(|| {
            panic!(
                "`{content}` is drawn: {:?}",
                painted.iter().map(|p| &p.content).collect::<Vec<_>>()
            )
        });
    Point::new(
        text.origin.x + text.natural_width.min(text.allowed_width) / 2.0,
        text.origin.y + 4.0,
    )
}

fn view(state: &State) -> Element<'_, Message> {
    micold_client::ui::view(
        state,
        None,
        None,
        0,
        None,
        &EnvIncludeOutcome::Disabled,
        &ConnectionStatus::Connected,
        &micold_client::features::sandbox::Sandbox::default(),
    )
}

/// Tab's release, the event that follows the focus move Tab's press asked for.
fn tab_released() -> Event {
    Event::Keyboard(keyboard::Event::KeyReleased {
        key: keyboard::Key::Named(keyboard::key::Named::Tab),
        modified_key: keyboard::Key::Named(keyboard::key::Named::Tab),
        physical_key: keyboard::key::Physical::Code(keyboard::key::Code::Tab),
        location: keyboard::Location::Standard,
        modifiers: keyboard::Modifiers::default(),
    })
}

/// Where **Send**, **Send queued** and the text area are, in the crowded pose.
fn places(state: &State) -> (Point, Point, Point) {
    let under = StateUnderTest::new(state.clone());
    let send = painted_at(&under, "Send");
    let send_queued = painted_at(&under, "Send queued");
    // The text area fills the row left of **Send** (its text is the editor's, not a paragraph).
    (send, send_queued, Point::new(send.x - 200.0, send.y))
}

/// Tab from the text area passes **Add attachment** (#1464 FR-010) and lands on **Send**, and the
/// window still holds the keyboard after the next event: a composer control has it, not the pane.
#[test]
fn tab_from_the_text_area_lands_on_send_and_the_window_keeps_the_keyboard() {
    let (mut state, id) = crowded();
    let (send, _, area) = places(&state);
    // A press on the text area: it takes the keyboard and says so on the event.
    let clicked = lay::messages_after(
        view(&state),
        &lay::renderer(),
        &[lay::Input::Press(area, mouse::Button::Left)],
    );
    for message in clicked {
        state.update(message);
    }
    assert_eq!(state.window.focused_field, Some(FieldId::Composer(id)));
    let messages = {
        let mut mounted = Mounted::new(view(&state));
        mounted.focus_at(area);
        mounted.event(&tab_released());
        // The area is focused, so its **Open large editor** button shows and Tab reaches it first
        // (feature 1703 FR-004).
        mounted.operate(Box::new(focusable::focus_next::<()>()));
        mounted.event(&tab_released());
        mounted.operate(Box::new(focusable::focus_next::<()>()));
        let attach = painted_at(&StateUnderTest::new(state.clone()), "\u{e226}");
        let at = mounted.focused().expect("Tab lands somewhere");
        assert!(
            at.contains(attach),
            "Tab lands on Add attachment first at {attach:?}: {at:?}"
        );
        mounted.event(&tab_released());
        mounted.operate(Box::new(focusable::focus_next::<()>()));
        let at = mounted.focused().expect("Tab lands somewhere");
        assert!(at.contains(send), "Tab lands on Send at {send:?}: {at:?}");
        mounted.event(&tab_released())
    };
    for message in messages {
        state.update(message);
    }
    assert!(
        !state.terminal_focused(),
        "Send holds the keyboard, so the pane must not take it back (focused field {:?})",
        state.window.focused_field
    );
}

/// Shift+Tab from the text area walks up through **Send queued** and the queued messages' actions.
#[test]
fn shift_tab_from_the_text_area_reaches_send_queued_and_the_queue_actions() {
    let (state, _) = crowded();
    let (_, send_queued, area) = places(&state);
    let mut mounted = Mounted::new(view(&state));
    mounted.focus_at(area);
    let mut landed = Vec::new();
    // The status row's stops (the model, #1554's effort trigger) come before the queue's.
    for _ in 0..7 {
        mounted.operate(Box::new(focusable::focus_previous::<()>()));
        landed.extend(mounted.focused());
    }
    assert!(
        landed.iter().any(|b| b.contains(send_queued)),
        "Shift+Tab reaches Send queued: {landed:?}"
    );
    let above = landed.iter().filter(|b| b.y < send_queued.y - 8.0).count();
    assert!(
        above >= 4,
        "Shift+Tab reaches both queued messages' Edit and Remove: {landed:?}"
    );
}

// --- feature 1703 M2: the activity icon and the session options row ------------------------------

use micold_client::features::composer::Msg as ComposerMsg;
use micold_client::icons::Icon;

/// Every string the view paints, in the order laid out.
fn painted_words(state: &State) -> Vec<(String, Point)> {
    let under = StateUnderTest::new(state.clone());
    let mut renderer = lay::renderer();
    let (painted, _) = lay::painted_text_settled_at(lay::view_of(&under), &mut renderer, WINDOW);
    painted
        .into_iter()
        .map(|p| (p.content.clone(), p.origin))
        .collect()
}

fn words(state: &State) -> Vec<String> {
    painted_words(state).into_iter().map(|(w, _)| w).collect()
}

/// FR-006, FR-007, SC-003: no state word is drawn, in any state; the icon is in the input row.
#[test]
fn the_activity_state_is_an_icon_with_its_words_in_the_tooltip() {
    use micold_client::ui::composer::{activity, BUSY, NOT_RUNNING, READY, WAITING};
    for (phase, waiting, label, busy) in [
        (TurnPhase::Idle, false, READY, false),
        (TurnPhase::NotRunning, false, NOT_RUNNING, false),
        (TurnPhase::Idle, true, WAITING, false),
    ] {
        assert_eq!(activity(phase, waiting), (label, busy));
    }
    let (mut state, id) = crowded();
    for phase in [TurnPhase::Idle, TurnPhase::NotRunning] {
        state.composers.input_reported(
            id,
            InputState {
                phase,
                hold: false,
                queue: Vec::new(),
            },
        );
        let painted = words(&state);
        for word in [READY, BUSY, NOT_RUNNING, WAITING] {
            assert!(
                !painted.iter().any(|w| w == word),
                "no status word `{word}` is drawn: {painted:?}"
            );
        }
    }
}

/// FR-008, FR-009: collapsed, the composer holds only the settings icon; opened, Model comes
/// first, Permission is last, and the selectors are not in the input row.
#[test]
fn the_options_row_holds_the_selectors_only_while_open() {
    let (mut state, _) = crowded();
    let settings = Icon::Settings.glyph().to_string();
    let collapsed = painted_words(&state);
    assert!(
        collapsed.iter().any(|(w, _)| *w == settings),
        "the settings icon is drawn: {collapsed:?}"
    );
    assert!(
        !collapsed.iter().any(|(w, _)| w.contains("Permissions")),
        "no permission selector in the input row: {collapsed:?}"
    );
    state.composers.options_open = true;
    let open = painted_words(&state);
    let model = state.model_header(crowded_id(&state));
    let at = |needle: &str| {
        open.iter()
            .find(|(w, _)| w.contains(needle))
            .map(|(_, p)| *p)
            .unwrap_or_else(|| panic!("`{needle}` is drawn: {open:?}"))
    };
    let (model, effort) = (at(&model), at("ffort"));
    assert!(model.x < effort.x, "Model before Effort");
    assert!((model.y - effort.y).abs() < 4.0, "one row");
}

fn crowded_id(state: &State) -> SessionId {
    state.session.active.expect("a session")
}

/// FR-009: the settings icon is a focusable button; Space or Enter on it toggles the row.
#[test]
fn the_settings_icon_toggles_from_the_keyboard() {
    let (state, id) = crowded();
    let under = StateUnderTest::new(state.clone());
    // The window's own Settings button draws the same glyph, higher up: the composer's is the last.
    let glyph = Icon::Settings.glyph().to_string();
    let mut renderer = lay::renderer();
    let (painted, _) = lay::painted_text_settled_at(lay::view_of(&under), &mut renderer, WINDOW);
    let at = painted
        .iter()
        .rfind(|p| p.content == glyph)
        .expect("the icon");
    let icon = Point::new(at.origin.x + 4.0, at.origin.y + 4.0);
    for named in [keyboard::key::Named::Space, keyboard::key::Named::Enter] {
        let mut mounted = Mounted::new(view(&state));
        mounted.focus_at(icon);
        let press = Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(named),
            modified_key: keyboard::Key::Named(named),
            physical_key: keyboard::key::Physical::Unidentified(
                keyboard::key::NativeCode::Unidentified,
            ),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::default(),
            text: None,
            repeat: false,
        });
        let messages = mounted.event(&press);
        assert!(
            messages.iter().any(|m| matches!(
                m,
                Message::Composer(ComposerMsg::OptionsToggled(s)) if *s == id
            )),
            "{named:?} presses the icon: {messages:?}"
        );
    }
}
