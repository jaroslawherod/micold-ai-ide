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

    /// Rebuild the element over the **same** tree, the way a frame does after the application's
    /// state has moved on.
    ///
    /// This is the only way to pose the question these tests exist for: a supplied flag is read
    /// when the widget is *built*, so "the application changed its mind" is not an event that can
    /// be sent — it is a rebuild carrying a different answer. `Mounted::new` would allocate a fresh
    /// tree and lose exactly the state under test.
    fn rebuild(&mut self, element: impl Into<Element<'a, String>>) {
        let mut element = element.into();
        self.tree.diff(element.as_widget());
        self.node = element.as_widget_mut().layout(
            &mut self.tree,
            &self.renderer,
            &layout::Limits::new(Size::ZERO, WINDOW),
        );
        self.element = element;
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

    /// Type one character, which only a focused input answers — so this reads as "does this still
    /// have the keyboard?" without asking the widget to confess.
    fn type_char(&mut self, c: char) -> Vec<String> {
        let key = iced::keyboard::Key::Character(c.to_string().into());
        self.send(
            Event::Keyboard(iced::keyboard::Event::KeyPressed {
                key: key.clone(),
                modified_key: key,
                physical_key: iced::keyboard::key::Physical::Unidentified(
                    iced::keyboard::key::NativeCode::Unidentified,
                ),
                location: iced::keyboard::Location::Standard,
                modifiers: iced::keyboard::Modifiers::default(),
                text: Some(c.to_string().into()),
                repeat: false,
            }),
            mouse::Cursor::Unavailable,
        )
    }

    /// The frame tick every widget sees. iced delivers `RedrawRequested` through `update` like any
    /// other event, which is what gives a widget somewhere to publish from after an *operation* has
    /// changed something — an operation carries no shell and can say nothing itself.
    fn redraw(&mut self) -> Vec<String> {
        self.send(
            Event::Window(iced::window::Event::RedrawRequested(
                std::time::Instant::now(),
            )),
            mouse::Cursor::Unavailable,
        )
    }

    /// Move the keyboard forward through this element, the way `Message::FocusMoved` does.
    ///
    /// The loop is not ceremony. `focus_next` is two passes chained — count the focusables, then
    /// move to the one after the focused — and a single `operate` call runs only the first, so a
    /// test that made one call would prove nothing and pass either way. The runtime drives the
    /// chain; here that is this loop.
    fn focus_next(&mut self) {
        use iced::advanced::widget::operation::{focusable, Operation, Outcome};

        let mut current: Box<dyn Operation<()>> = Box::new(focusable::focus_next::<()>());
        loop {
            self.element.as_widget_mut().operate(
                &mut self.tree,
                Layout::new(&self.node),
                &self.renderer,
                current.as_mut(),
            );
            match current.finish() {
                Outcome::Chain(next) => current = next,
                Outcome::None | Outcome::Some(_) => break,
            }
        }
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
// The application's answer is the authoritative one (BUG-004, FR-035)
// -------------------------------------------------------------------------------------------
//
// Focus is *observed* inside the control and *held* by the application, which leaves two copies of
// one fact. BUG-003 made the control tell the application; these make the application able to tell
// the control, so a screen that takes the keyboard back — `focus_terminal()` does exactly this —
// does not leave a control drawing itself at rest while it still answers keys.
//
// The reconciliation is deliberately conditional on the caller having asked to be told, because a
// supplied flag means two different things in this library: for a text field it is focus, and for a
// picker it is *open* (§7.7 — the active indicator follows the list, not the keyboard).
// [`a_picker_that_closes_its_list_keeps_the_keyboard`] is the fence around that difference.

#[test]
fn the_application_can_take_the_keyboard_back_from_a_field() {
    let r = roles();
    let mut mounted = Mounted::new(field("", r));
    mounted.press(mounted.slot(1).center());
    assert!(
        !mounted.type_char('a').is_empty(),
        "precondition: a focused input must answer the keyboard, or the assertion below proves \
         nothing",
    );

    // The frame after the press: the application heard `focus=true` and now says so too. Written
    // out rather than skipped, because what the control watches for is the application *changing
    // its mind* — not a standing disagreement, which is what an unreported traversal focus looks
    // like and which it must not undo.
    mounted.rebuild(field("", r).active(true));

    // And now the application changes its mind: `focus_terminal()` clears `focused_field` with no
    // press landing anywhere near this field, and the next frame carries that answer back.
    mounted.rebuild(field("", r).active(false));

    assert!(
        mounted.type_char('b').is_empty(),
        "a field the application says is unfocused must not still be typing into. The flag it is \
         drawn from and the focus it acts on are one fact, and a control that keeps the keyboard \
         after the application has given it away is a field at rest that swallows every keystroke \
         (BUG-004)",
    );
}

#[test]
fn the_application_can_take_the_keyboard_back_from_a_checkbox() {
    let r = roles();
    let mut box_ = Mounted::new(checkbox(false, r));
    box_.press(box_.node.bounds().center());

    // The application agrees, then takes it back — the same two frames as the field above.
    box_.rebuild(checkbox(false, r).focused(true));
    box_.rebuild(checkbox(false, r).focused(false));

    assert!(
        box_.press_key(iced::keyboard::key::Named::Space).is_empty(),
        "a checkbox the application says is unfocused must not still answer Space — it would be a \
         box drawn at rest that toggles under a keystroke aimed at whatever really has the \
         keyboard (BUG-004)",
    );
}

#[test]
fn a_picker_that_closes_its_list_keeps_the_keyboard() {
    let r = roles();
    // A search picker's field: `active` follows **open**, and nobody is tracking its focus — so
    // there is no `on_focus_change` here, exactly as `typeahead.rs` builds it.
    let built = |open: bool| {
        TextField::new("", "", r)
            .label("Branch name")
            .on_input(|typed| typed)
            .active(open)
    };
    let mut mounted = Mounted::new(built(true));
    mounted.press(mounted.slot(1).center());

    // The list closes. The keyboard has nothing to do with it.
    mounted.rebuild(built(false));

    assert!(
        !mounted.type_char('a').is_empty(),
        "closing a picker's list must not take the keyboard out of its search field. `active` is \
         focus for a text field and *open* for a picker (§7.7), so the application's answer is \
         authoritative only where the application asked to be told it — which is what pairs it \
         with `on_focus_change` (BUG-004)",
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

/// A focus traversal is not an event, so a control it lands on has no `update` to notice it in.
/// The checkbox took the keyboard from the traversal, answered Space, and told the application
/// nothing — so `focused_field` stayed empty and the box drew itself at rest. Found by the T075
/// visual pass: tabbing onto a credential opt-in changed not one pixel, and Space then toggled it.
///
/// FR-030 asks for the focused element to be *visible*, and a control that holds the keyboard in
/// secret is the half of that requirement nothing else here was checking.
#[test]
fn a_checkbox_reached_by_the_traversal_says_so() {
    let r = roles();
    let mut mounted =
        Mounted::new(checkbox(false, r).on_focus_change(|focused| format!("focus={focused}")));

    mounted.focus_next();
    assert_eq!(
        mounted.redraw(),
        vec!["focus=true".to_string()],
        "the traversal gave the checkbox the keyboard and nothing was published, so nothing in the \
         application knows to draw it focused"
    );
}

/// And only once. A report on every frame would be a message per repaint for as long as the box
/// holds the keyboard — the defect `focus_is_reported_when_it_changes_and_not_on_every_event`
/// guards against for the field beside it.
#[test]
fn the_checkbox_reports_the_traversal_once_and_not_every_frame() {
    let r = roles();
    let mut mounted =
        Mounted::new(checkbox(false, r).on_focus_change(|focused| format!("focus={focused}")));

    mounted.focus_next();
    assert_eq!(mounted.redraw(), vec!["focus=true".to_string()]);
    assert!(
        mounted.redraw().is_empty(),
        "the checkbox re-announced focus it had already reported"
    );
}

// -------------------------------------------------------------------------------------------
// The button and the select, which had no keyboard either (feature 027, FR-030)
// -------------------------------------------------------------------------------------------
//
// The checkbox above was given a keyboard by feature 022 and the text field has always had one, so
// a settings form of fields and opt-ins could be tabbed through. Feature 027's Settings is not that
// form: it is a surface whose sections are chosen from a rail of *buttons*, whose theme is a
// *select*, and whose Save and Cancel are buttons. Eight Tab presses on its Appearance section
// changed zero pixels, because none of those controls is focusable at all — so the section could be
// reached by pointer only, and FR-030 ("Sections and every control within them MUST be reachable by
// keyboard alone, with the focused element visible") failed on its first clause.
//
// These drive both controls the way a person does. What they cannot see is the *indicator* — a
// renderer here draws into nothing that can be read back — so its geometry is asserted beside it in
// `keyboard_focus.rs` and its appearance is the visual pass's to confirm.

/// A button that says what it was pressed for.
fn button<'a>(r: Roles) -> super::Button<'a, String> {
    super::Button::filled("Save", r).on_press("saved".to_string())
}

/// The options a test select offers. `static` rather than `const` so the slice outlives the
/// builder's borrow of it.
static OPTIONS: [&str; 2] = ["Light", "Dark"];

/// A select over [`OPTIONS`], with nothing chosen yet.
fn select<'a>(r: Roles) -> super::Select<'a, &'static str, String> {
    super::Select::new(&OPTIONS, None, |chosen: &str| format!("picked={chosen}"), r).label("Theme")
}

#[test]
fn a_button_reached_by_the_traversal_answers_enter() {
    let r = roles();
    let mut mounted = Mounted::new(button(r));

    mounted.focus_next();

    assert_eq!(
        mounted.press_key(iced::keyboard::key::Named::Enter),
        vec!["saved".to_string()],
        "the rendering stack's button holds no focus and answers no key, so Save, Cancel and every \
         row of the settings rail were reachable by pointer only (FR-030)",
    );
}

#[test]
fn a_button_answers_space_as_well() {
    let r = roles();
    let mut mounted = Mounted::new(button(r));

    mounted.focus_next();

    assert_eq!(
        mounted.press_key(iced::keyboard::key::Named::Space),
        vec!["saved".to_string()],
        "a button answers both keys everywhere it exists — unlike the checkbox, which answers only \
         Space because Enter is the dialog's",
    );
}

#[test]
fn an_unfocused_button_leaves_both_keys_alone() {
    let r = roles();
    let mut mounted = Mounted::new(button(r));

    assert!(
        mounted
            .press_key(iced::keyboard::key::Named::Enter)
            .is_empty()
            && mounted
                .press_key(iced::keyboard::key::Named::Space)
                .is_empty(),
        "a button nobody has focused must not fire on a keystroke aimed at whatever has — a form \
         of eight buttons would save itself eight times on one Enter",
    );
}

#[test]
fn a_disabled_button_is_not_a_tab_stop() {
    let r = roles();
    // No `on_press`, which is how a button renders disabled everywhere in this library — beside a
    // checkbox, so the traversal has somewhere else to land and the test can tell "skipped" from
    // "nothing happened at all".
    let row: Element<'_, String> = iced::widget::column![
        Element::from(super::Button::<String>::filled("Save", r)),
        Element::from(checkbox(false, r)),
    ]
    .into();
    let mut mounted = Mounted::new(row);

    mounted.focus_next();

    assert_eq!(
        mounted.redraw(),
        vec!["focus=true".to_string()],
        "the first Tab must reach the checkbox: a disabled control that takes the keyboard draws a \
         focus indicator on something that cannot be operated and stands as a dead stop in the tab \
         order",
    );
}

#[test]
fn a_select_reached_by_the_traversal_opens_and_can_be_chosen_from() {
    let r = roles();
    let mut mounted = Mounted::new(select(r));

    mounted.focus_next();
    // Enter opens the list. Nothing is published by opening — the list is the component's own
    // state, which is the whole reason `Select::active` does not exist (FR-013).
    assert!(mounted
        .press_key(iced::keyboard::key::Named::Enter)
        .is_empty());
    mounted.press_key(iced::keyboard::key::Named::ArrowDown);
    let published = mounted.press_key(iced::keyboard::key::Named::Enter);

    assert_eq!(
        published,
        vec!["picked=Light".to_string()],
        "a select the keyboard can reach but not open is a tab stop with nothing behind it — the \
         Theme picker was exactly that (FR-030)",
    );
}

#[test]
fn an_unfocused_select_leaves_enter_alone() {
    let r = roles();
    let mut mounted = Mounted::new(select(r));

    assert!(
        mounted
            .press_key(iced::keyboard::key::Named::Enter)
            .is_empty(),
        "a closed select nobody has focused must not open on a keystroke meant for something else \
         — every select on the surface would drop its list at once",
    );
}

#[test]
fn a_select_that_loses_the_keyboard_closes_its_list() {
    let r = roles();
    // Two focusables, so the traversal has somewhere to go: a `focus_next` in a tree with one stop
    // wraps back onto it and moves nothing.
    let form: Element<'_, String> =
        iced::widget::column![Element::from(select(r)), Element::from(checkbox(false, r))].into();
    let mut mounted = Mounted::new(form);
    mounted.focus_next();
    mounted.press_key(iced::keyboard::key::Named::Enter);
    mounted.press_key(iced::keyboard::key::Named::ArrowDown);

    // Tab moves on, which unfocuses the select.
    mounted.focus_next();
    // The checkbox the keyboard landed on reports that on the next frame; flushing it here keeps
    // the assertion below about the select and nothing else.
    mounted.redraw();

    let published = mounted.press_key(iced::keyboard::key::Named::Enter);

    assert!(
        !published.iter().any(|m| m.starts_with("picked=")),
        "the list was still open and still taking rows after the keyboard had moved on — and a \
         list nothing holds is a list nothing can dismiss: Escape now reaches whatever has the \
         focus, and it is not this — published {published:?}",
    );
}
