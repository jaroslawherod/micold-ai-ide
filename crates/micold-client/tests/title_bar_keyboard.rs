//! The title bar's keyboard reach (feature 1127, FR-003, FR-013, contract C1, C4, C5, C6, C7): its
//! controls are Tab stops in bar order — the project switcher, the Overview button, the jump-to
//! field, Settings, the overflow trigger — Enter on the field publishes its press and Enter on
//! Settings opens Settings, the field is skipped while Settings is open, and the app mark is neither
//! a Tab stop nor a control.

mod support;

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::operation::{focusable, Focusable, Outcome};
use iced::advanced::widget::{Id, Operation, Tree};
use iced::advanced::{clipboard, mouse, Shell};
use iced::keyboard::key::Named;
use iced::{Element, Event, Point, Rectangle, Size};
use micold_client::app::{Message, State};
use micold_client::features::connection::ConnectionStatus;
use micold_client::features::sandbox::Sandbox;
use micold_client::features::settings::Msg as SettingsMsg;
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::tokens::anatomy::app_bar;
use micold_material::test_support::named_key;
use std::sync::{Arc, Mutex};
use support::layout as lay;

const WINDOW: Size = Size::new(1280.0, 800.0);

/// Every focusable in traversal order: its bounds.
#[derive(Clone, Default)]
struct Record(Arc<Mutex<Vec<Rectangle>>>);

impl Operation for Record {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn focusable(&mut self, _id: Option<&Id>, bounds: Rectangle, _: &mut dyn Focusable) {
        self.0.lock().unwrap().push(bounds);
    }
}

/// The whole view, mounted, with its keyboard moved and fed the way the runtime does.
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
            &layout::Limits::new(Size::ZERO, WINDOW),
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

    /// The bounds of every Tab stop inside the bar, in traversal order.
    fn bar_stops(&mut self) -> Vec<Rectangle> {
        let record = Record::default();
        self.operate(Box::new(record.clone()));
        let all = record.0.lock().unwrap().clone();
        all.into_iter()
            .filter(|b| b.y + b.height <= app_bar::BOTTOM_EDGE)
            .collect()
    }

    fn tab(&mut self) {
        self.operate(Box::new(focusable::focus_next::<()>()));
    }

    fn send(&mut self, event: Event, cursor: mouse::Cursor) -> Vec<Message> {
        let mut published = Vec::new();
        let mut shell = Shell::new(&mut published);
        self.element.as_widget_mut().update(
            &mut self.tree,
            &event,
            Layout::new(&self.node),
            cursor,
            &self.renderer,
            &mut clipboard::Null,
            &mut shell,
            &Rectangle::with_size(WINDOW),
        );
        published
    }
}

/// A project open, Settings closed.
fn state() -> State {
    let mut workspace = support::workspace_with(vec![("/fixture/project", vec![])]);
    workspace.active = workspace.projects.first().map(|p| p.path.clone());
    let mut state = State {
        workspace,
        ..State::default()
    };
    state.window.window_size = (1280, 800);
    state
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
        &Sandbox::default(),
    )
}

/// C2/C4/C5/C6/C7: five Tab stops in the bar, left to right in traversal order, the first of them the
/// first Tab stop of the window; none of them is the app mark's square.
#[test]
fn the_bar_controls_are_tab_stops_in_bar_order() {
    let state = state();
    let mut m = Mounted::new(view(&state));
    let stops = m.bar_stops();
    assert_eq!(
        stops.len(),
        5,
        "the switcher, Overview, the field, Settings and the overflow trigger: {stops:?}"
    );
    for pair in stops.windows(2) {
        assert!(
            pair[0].x < pair[1].x,
            "Tab order is not bar order: {stops:?}"
        );
    }
    let mark_end = app_bar::PADDING + app_bar::MARK;
    assert!(
        stops.iter().all(|b| b.x >= mark_end),
        "the app mark is a Tab stop: {stops:?}"
    );
    let (settings, overflow) = (stops[3], stops[4]);
    for icon_only in [settings, overflow] {
        assert_eq!(
            (icon_only.width, icon_only.height),
            (app_bar::ICON_TARGET, app_bar::ICON_TARGET),
            "{icon_only:?}"
        );
    }
}

/// C6: Tab to Settings (the fourth stop) and press Enter: Settings opens.
#[test]
fn enter_on_settings_opens_settings() {
    let state = state();
    let mut m = Mounted::new(view(&state));
    for _ in 0..4 {
        m.tab();
    }
    let published = m.send(named_key(Named::Enter), mouse::Cursor::Unavailable);
    assert!(
        published
            .iter()
            .any(|msg| matches!(msg, Message::Settings(SettingsMsg::Opened))),
        "Enter on the fourth Tab stop sent {published:?}"
    );
}

/// T039 (FR-013, C5): the field is the third stop, between the Overview button and Settings, and
/// Enter on it publishes what `jump_press` says it sends.
#[test]
fn enter_on_the_jump_field_publishes_its_press() {
    let state = state();
    let mut m = Mounted::new(view(&state));
    for _ in 0..3 {
        m.tab();
    }
    let published = m.send(named_key(Named::Enter), mouse::Cursor::Unavailable);
    let pressed = micold_client::ui::jump_press(&state).expect("pressable");
    assert_eq!(published, vec![pressed], "Enter on the third Tab stop");
}

/// T039 (FR-009, C5): with Settings open the field is drawn disabled and Tab skips it.
#[test]
fn with_settings_open_tab_skips_the_jump_field() {
    let mut state = state();
    state.settings.settings_draft = Some(Default::default());
    let mut m = Mounted::new(view(&state));
    let field = lay::resolve(view(&state), &lay::renderer())
        .into_iter()
        .find(|r| r.layer == lay::Layer::Base && r.path == [0, 0, 0, 0, 0, 3])
        .expect("the bar lays out its jump-to field");
    let centre = Point::new(field.x + field.width / 2.0, field.y + field.height / 2.0);
    let stops = m.bar_stops();
    assert!(!stops.is_empty(), "the bar still has Tab stops");
    assert!(
        stops.iter().all(|b| !b.contains(centre)),
        "the disabled field at {field:?} is a Tab stop: {stops:?}"
    );
}

/// C1 (FR-003): a press and a release on the app mark publish nothing.
#[test]
fn the_app_mark_publishes_nothing_when_pressed() {
    let state = state();
    let mut m = Mounted::new(view(&state));
    let at = Point::new(
        app_bar::PADDING + app_bar::MARK / 2.0,
        app_bar::HEIGHT / 2.0,
    );
    let cursor = mouse::Cursor::Available(at);
    let mut published = m.send(
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        cursor,
    );
    published.extend(m.send(
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        cursor,
    ));
    assert!(published.is_empty(), "the mark published {published:?}");
}
