//! A field says when it has the keyboard, and the whole of it takes it (BUG-003; FR-034, FR-035).
//!
//! # What was missing, and why nothing caught it
//!
//! `form_field_anatomy.rs` and `text_field_anatomy.rs` build a field **with `active` set by the
//! test** and assert what it draws. That is the right way to check the component and it could not
//! see BUG-003: the component did exactly what it was told, and the defect was that nobody told it.
//! Focus was a supplied flag, no call site in the application supplied it, and for two features
//! every text field was drawn permanently at rest — label unfloated, indicator a hairline, focus
//! layer never once painted.
//!
//! So these tests are the other half. Nothing here poses a state: each one *drives* the field the
//! way a person does and reads what it published. A build where the report never fires fails them,
//! whatever the anatomy gates say.
//!
//! `tests/field_focus_call_sites.rs` is the third part, and the one that speaks to the actual bug —
//! it checks that the application's fields are joined to this at all.
//!
//! # The reach of a press
//!
//! A filled field is 56dp and its control is one 24dp line inside 16dp of padding, so most of the
//! box is not the input. FR-034 asks for one rectangle rather than two: the area that shades on
//! hover is the area that responds. [`a_press_in_the_padding_reaches_the_input_too`] is that rule
//! for the keyboard, and it is the same defect BUG-002 fixed for the select's pointer.

use iced::advanced::widget::Tree;
use iced::advanced::{clipboard, layout, mouse, Layout};
use iced::{Element, Event, Point, Rectangle, Size};
use micold_core::tokens::{self, Roles};

use super::TextField;

/// The width the field is laid out at, and the window it is laid out in.
const WINDOW: Size = Size::new(400.0, 800.0);

fn roles() -> Roles {
    tokens::roles(micold_core::theme::ColorScheme::Light)
}

/// An editable field that reports its focus as its own message.
///
/// `on_input` is not decoration: an input with nowhere to send its value renders disabled, and a
/// disabled input is not a fair test of what a live one reports.
fn field<'a>(value: &'a str, r: Roles) -> TextField<'a, String> {
    TextField::new("", value, r)
        .label("Branch name")
        .on_input(|typed| typed)
        .on_focus_change(|focused| format!("focus={focused}"))
}

/// A mounted element with its tree and layout kept in step, so an event lands where the test
/// thinks it does. The narrow cousin of `select_anatomy.rs`'s harness — no frame ticks are needed
/// here, because focus is not animated.
struct Mounted<'a> {
    element: Element<'a, String>,
    tree: Tree,
    node: layout::Node,
    renderer: iced::Renderer,
}

impl<'a> Mounted<'a> {
    fn new(element: impl Into<Element<'a, String>>) -> Self {
        let mut element = element.into();
        let renderer = super::test_support::renderer();
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

    /// The filled container — the first of the two bands `FormField` emits.
    fn container(&self) -> Rectangle {
        self.node.children()[0].bounds()
    }

    /// The container's four slots: `[leading, control, trailing, label]`.
    fn slot(&self, index: usize) -> Rectangle {
        self.node.children()[0].children()[index].bounds()
    }

    fn send(&mut self, event: Event, cursor: mouse::Cursor) -> Vec<String> {
        let mut messages = Vec::new();
        let mut shell = iced::advanced::Shell::new(&mut messages);
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
        messages
    }

    /// Press the left button at `at`.
    fn press(&mut self, at: Point) -> Vec<String> {
        self.send(
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            mouse::Cursor::Available(at),
        )
    }

    /// Move the pointer to `at` without pressing.
    fn hover(&mut self, at: Point) -> Vec<String> {
        self.send(
            Event::Mouse(mouse::Event::CursorMoved { position: at }),
            mouse::Cursor::Available(at),
        )
    }

    /// Press a named key, with the pointer nowhere near — a keyboard interaction has to work
    /// without the mouse resting helpfully on the control, or it is not a keyboard interaction.
    fn press_key(&mut self, key: iced::keyboard::key::Named) -> Vec<String> {
        self.send(
            Event::Keyboard(iced::keyboard::Event::KeyPressed {
                key: iced::keyboard::Key::Named(key),
                modified_key: iced::keyboard::Key::Named(key),
                physical_key: iced::keyboard::key::Physical::Unidentified(
                    iced::keyboard::key::NativeCode::Unidentified,
                ),
                location: iced::keyboard::Location::Standard,
                modifiers: iced::keyboard::Modifiers::default(),
                text: None,
                repeat: false,
            }),
            mouse::Cursor::Unavailable,
        )
    }
}

#[test]
fn a_press_on_the_input_reports_focus() {
    let r = roles();
    let mut field = Mounted::new(field("", r));

    let published = field.press(field.slot(1).center());

    assert!(
        published.contains(&"focus=true".to_string()),
        "clicking a text field must report that it took the keyboard, so the view can float the \
         label, thicken the indicator and shade the container (FR-031, FR-035) — published {published:?}",
    );
}

#[test]
fn a_press_in_the_padding_reaches_the_input_too() {
    let r = roles();
    let mut field = Mounted::new(field("", r));

    // Two dp below the container's top edge: inside the box, well clear of the 24dp value line the
    // control occupies. On the old field this was a press on nothing at all.
    let container = field.container();
    let published = field.press(Point::new(container.center_x(), container.y + 2.0));

    assert!(
        published.contains(&"focus=true".to_string()),
        "a press anywhere in the container must reach the control (FR-034): the box shades and \
         hovers as one rectangle, so it must respond as one — published {published:?}",
    );
}

#[test]
fn a_press_on_the_trailing_action_is_that_action_and_not_a_grab_for_the_keyboard() {
    let r = roles();
    let mut field = Mounted::new(
        field("", r).trailing_action(crate::icons::Icon::Close, "cleared".to_string()),
    );

    let published = field.press(field.slot(2).center());

    assert!(
        !published.contains(&"focus=true".to_string()),
        "a trailing icon button is an action of its own — pressing it must not also hand the \
         keyboard to the input beside it — published {published:?}",
    );
}

#[test]
fn losing_the_keyboard_is_reported_as_well_as_taking_it() {
    let r = roles();
    let mut field = Mounted::new(field("", r));
    field.press(field.slot(1).center());

    // Somewhere outside the field entirely, the way clicking anything else in a dialog is.
    let published = field.press(Point::new(WINDOW.width - 1.0, WINDOW.height - 1.0));

    assert!(
        published.contains(&"focus=false".to_string()),
        "focus must be dropped when it leaves (FR-035), or the field it left keeps drawing itself \
         focused for the rest of the dialog's life — published {published:?}",
    );
}

#[test]
fn focus_is_reported_when_it_changes_and_not_on_every_event() {
    let r = roles();
    let mut field = Mounted::new(field("", r));
    field.press(field.slot(1).center());

    let published = field.hover(field.container().center());

    assert!(
        published.is_empty(),
        "a field that re-announced \"still focused\" on every pointer move would put the \
         application into a message loop with itself — published {published:?}",
    );
}

// -------------------------------------------------------------------------------------------
// The checkbox, which had no keyboard at all (BUG-003, FR-035)
// -------------------------------------------------------------------------------------------

/// A checkbox that reports both what happened to it and what happened to its focus, so one harness
/// can tell a toggle from a focus change.
fn checkbox<'a>(checked: bool, r: Roles) -> super::Checkbox<'a, String> {
    super::Checkbox::new("Enabled", checked, r)
        .on_toggle(move |now| format!("toggled={now}"))
        .on_focus_change(|focused| format!("focus={focused}"))
}

#[test]
fn a_press_gives_the_checkbox_the_keyboard() {
    let r = roles();
    let mut box_ = Mounted::new(checkbox(false, r));

    let published = box_.press(box_.node.bounds().center());

    assert!(
        published.contains(&"focus=true".to_string()),
        "the rendering stack's checkbox cannot be focused at all — it holds no focus, joins no \
         traversal and answers no key. FR-035 asks every input to answer focus, and a control the \
         keyboard cannot reach can never answer it — published {published:?}",
    );
}

#[test]
fn space_toggles_a_focused_checkbox() {
    let r = roles();
    let mut box_ = Mounted::new(checkbox(false, r));
    box_.press(box_.node.bounds().center());

    let published = box_.press_key(iced::keyboard::key::Named::Space);

    assert!(
        published.contains(&"toggled=true".to_string()),
        "a focused checkbox must be operable from the keyboard — a focus ring on a control that \
         still needs the mouse is decoration (FR-035) — published {published:?}",
    );
}

#[test]
fn enter_is_the_dialogs_and_the_checkbox_leaves_it_alone() {
    let r = roles();
    let mut box_ = Mounted::new(checkbox(false, r));
    box_.press(box_.node.bounds().center());

    let published = box_.press_key(iced::keyboard::key::Named::Enter);

    assert!(
        published.is_empty(),
        "Space is the key a checkbox answers; Enter belongs to the dialog, which reaches \
         `TextField::on_submit` with it today and may grow a default action tomorrow. A control \
         that toggles on Enter is the thing that answers first, and nothing downstream of it ever \
         gets the chance — published {published:?}",
    );
}

#[test]
fn an_unfocused_checkbox_leaves_space_alone() {
    let r = roles();
    let mut box_ = Mounted::new(checkbox(false, r));

    let published = box_.press_key(iced::keyboard::key::Named::Space);

    assert!(
        published.is_empty(),
        "a checkbox nobody has focused must not swallow Space from whatever has — published \
         {published:?}",
    );
}

#[test]
fn a_press_elsewhere_takes_the_keyboard_back() {
    let r = roles();
    let mut box_ = Mounted::new(checkbox(false, r));
    box_.press(box_.node.bounds().center());

    let published = box_.press(Point::new(WINDOW.width - 1.0, WINDOW.height - 1.0));

    assert!(
        published.contains(&"focus=false".to_string()),
        "focus must be dropped when it leaves (FR-035) — published {published:?}",
    );
}

#[test]
fn a_disabled_checkbox_does_not_take_the_keyboard() {
    let r = roles();
    // No `on_toggle`, which is how a checkbox renders disabled everywhere in this library.
    let mut box_ = Mounted::new(
        super::Checkbox::<String>::new("Enabled", false, r)
            .on_focus_change(|focused| format!("focus={focused}")),
    );

    let published = box_.press(box_.node.bounds().center());

    assert!(
        published.is_empty(),
        "a disabled control must not take the keyboard: it would draw a focus ring on something \
         that cannot be operated, and stand as a dead stop in the tab order — published \
         {published:?}",
    );
}

#[test]
fn the_focused_checkbox_is_shaded_and_focus_outranks_hover() {
    use iced::widget::checkbox as checkbox_widget;
    use micold_core::theme::ColorScheme;

    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let r = tokens::roles(scheme);
        let theme = super::style::theme(scheme);
        let fill = |focused: bool, status| match super::style::checkbox(r, focused)(&theme, status)
            .background
        {
            iced::Background::Color(c) => c,
            _ => iced::Color::TRANSPARENT,
        };
        for is_checked in [false, true] {
            let rest = fill(false, checkbox_widget::Status::Active { is_checked });
            let hovered = fill(false, checkbox_widget::Status::Hovered { is_checked });
            let focused = fill(true, checkbox_widget::Status::Active { is_checked });
            let both = fill(true, checkbox_widget::Status::Hovered { is_checked });

            let delta = |a: iced::Color, b: iced::Color| {
                (a.r - b.r).abs() + (a.g - b.g).abs() + (a.b - b.b).abs()
            };
            assert!(
                delta(rest, focused) > 0.0,
                "{scheme:?} checked={is_checked}: a focused checkbox must carry the focused state \
                 layer (FR-035)",
            );
            assert!(
                delta(hovered, focused) > 0.0,
                "{scheme:?} checked={is_checked}: focus and hover are different states and must \
                 look different — §5 publishes two opacities",
            );
            assert_eq!(
                (both.r, both.g, both.b),
                (focused.r, focused.g, focused.b),
                "{scheme:?} checked={is_checked}: a focused, hovered checkbox must show **one** \
                 layer — the stronger — not two blended into a colour no token names",
            );
        }
    }
}
