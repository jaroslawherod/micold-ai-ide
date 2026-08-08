//! The two lists are one list (feature 022, T013 — SC-001, FR-007, FR-008, FR-009).
//!
//! The feature's headline claim is that a person can open the select's list and the search picker's
//! side by side and find nothing to tell them apart. That claim is about *both* controls, so it
//! cannot be checked by reading either one: it is asserted here by **building both and comparing
//! them**, node for node, rather than by restating §7.5's figures against each in turn. Two tests
//! that each pinned the same number would agree with the contract and could still disagree with
//! each other the day one of them was updated.
//!
//! In-crate because `material` is `pub(crate)` and neither control can be constructed from `tests/`.
//!
//! # Why geometry, and why the same labels
//!
//! Row height, row padding, the marker's reserved slot and the panel's own inset are all *positions*,
//! and positions are what a comparison can be exact about. So both lists are built over the same
//! three labels at the same width, laid out, and their whole node trees compared. Anything that
//! differed — a row an item taller, a marker slot the select forgot to reserve, a panel padded to a
//! different step — moves a rectangle, and every rectangle is in the comparison.

use iced::advanced::widget::Tree;
use iced::advanced::{clipboard, layout, mouse, Layout};
use iced::{Element, Event, Rectangle, Size, Vector};
use micold_core::tokens::{self, Roles};

use super::picker::Row;
use super::{Select, Typeahead};
/// One frame, named rather than restated — see `select_anatomy`'s copy of this import.
use crate::ui::cdk::motion::FRAME;

/// The rows both lists are built over. Identical, because the comparison is of the list's anatomy
/// and a different label would move the same rectangle in both trees for a reason that is not the
/// component's.
const LABELS: &[&str] = &["one", "two", "three"];

const WIDTH: f32 = 400.0;
const WINDOW: Size = Size::new(WIDTH, 800.0);

fn roles() -> Roles {
    tokens::roles(micold_core::theme::ColorScheme::Light)
}

/// Lay `element` out, optionally press it at `press`, and return the node its floating list
/// resolves to.
///
/// The press is how the select is opened: its openness is its own, so there is no builder to pose
/// it with — which is the design, not an inconvenience (data-model §2.2).
fn floating_list(element: impl Into<Element<'static, String>>, press: bool) -> layout::Node {
    let mut element = element.into();
    let renderer = super::test_support::renderer();
    let mut tree = Tree::new(element.as_widget());
    let limits = layout::Limits::new(Size::ZERO, WINDOW);
    let mut node = element
        .as_widget_mut()
        .layout(&mut tree, &renderer, &limits);

    if press {
        let at = node.children()[0].bounds().center();
        let origin = std::time::Instant::now();
        // The press, then the frames the list's own visibility track needs to reach the state the
        // press sent it to — `cdk::picker` keeps a list on screen for as long as that track says
        // there is any of it left, and the track only moves on a tick.
        let mut events = vec![Event::Mouse(mouse::Event::ButtonPressed(
            mouse::Button::Left,
        ))];
        let mut cursors = vec![mouse::Cursor::Available(at)];
        for i in 1..=2u32 {
            events.push(Event::Window(iced::window::Event::RedrawRequested(
                origin + FRAME * i,
            )));
            cursors.push(mouse::Cursor::Unavailable);
        }
        for (event, cursor) in events.into_iter().zip(cursors) {
            let mut messages: Vec<String> = Vec::new();
            let mut shell = iced::advanced::Shell::new(&mut messages);
            element.as_widget_mut().update(
                &mut tree,
                &event,
                Layout::new(&node),
                cursor,
                &renderer,
                &mut clipboard::Null,
                &mut shell,
                &Rectangle::with_size(WINDOW),
            );
            node = element
                .as_widget_mut()
                .layout(&mut tree, &renderer, &limits);
        }
    }

    let mut overlay = element
        .as_widget_mut()
        .overlay(
            &mut tree,
            Layout::new(&node),
            &renderer,
            &Rectangle::with_size(WINDOW),
            Vector::ZERO,
        )
        .expect("the picker floated no list, so there is nothing to compare");
    overlay.as_overlay_mut().layout(&renderer, WINDOW)
}

/// Every node of `list`, as bounds **relative to the panel's own origin**, depth first.
///
/// Relative, because the two panels are anchored under two different fields and a shared absolute
/// origin would be an accident rather than a property. Rounded to a tenth of a device pixel: text
/// shaping resolves widths in floating point, and a comparison exact to the bit would fail on
/// arithmetic rather than on anatomy.
fn shape(list: &layout::Node) -> Vec<(f32, f32, f32, f32)> {
    fn walk(node: &layout::Node, offset: Vector, out: &mut Vec<Rectangle>) {
        let bounds = node.bounds() + offset;
        out.push(bounds);
        for child in node.children() {
            walk(child, Vector::new(bounds.x, bounds.y), out);
        }
    }
    let mut all = Vec::new();
    walk(list, Vector::ZERO, &mut all);
    let origin = all[0];
    let round = |v: f32| (v * 10.0).round() / 10.0;
    all.into_iter()
        .map(|b| {
            (
                round(b.x - origin.x),
                round(b.y - origin.y),
                round(b.width),
                round(b.height),
            )
        })
        .collect()
}

/// The select's open list.
fn select_list() -> layout::Node {
    let r = roles();
    floating_list(
        Select::new(LABELS, Some(LABELS[0]), |t: &str| t.to_string(), r).label("Type"),
        true,
    )
}

/// The search picker's open list, over the same rows in the same states.
fn typeahead_list() -> layout::Node {
    let r = roles();
    let rows: Vec<Row> = LABELS.iter().map(|l| Row::new(*l, Vec::new())).collect();
    floating_list(
        Typeahead::new("", rows, |s: String| s, r)
            .label("Type")
            .open(true)
            .selected(Some(0))
            .highlighted(Some(0))
            .on_pick(|i: usize| i.to_string()),
        false,
    )
}

/// Both lists resolve to the same shape, rectangle for rectangle.
///
/// This is SC-001 as a measurement. It covers the row's height and padding (every row's box), the
/// marker slot (the leading node each row reserves whether or not it is marked) and the panel's own
/// inset (the offset of the first row from the panel's origin) in one comparison, because all four
/// are positions and a list that got any of them wrong would put a rectangle somewhere else.
#[test]
fn the_select_and_the_search_picker_float_the_same_list() {
    let select = shape(&select_list());
    let typeahead = shape(&typeahead_list());

    assert_eq!(
        select.len(),
        typeahead.len(),
        "the select's list has {} nodes against the search picker's {} — one of them is assembling \
         a list of its own rather than the shared one (FR-007, FR-008)",
        select.len(),
        typeahead.len(),
    );

    let differing: Vec<String> = select
        .iter()
        .zip(typeahead.iter())
        .enumerate()
        .filter(|(_, (a, b))| a != b)
        .map(|(i, (a, b))| format!("  node {i}: select {a:?} vs search picker {b:?}"))
        .collect();

    assert!(
        differing.is_empty(),
        "the two lists do not resolve to the same shape:\n{}\n\nSC-001 is that a person can open \
         both and find nothing to tell them apart, and typography, row height, the marker's \
         reserved slot and the panel's inset are what they would notice first. Both lists come from \
         `material::picker`; a difference here means one control is no longer asking for it.",
        differing.join("\n"),
    );
}

/// …and the comparison is over something. A parity check between two empty lists passes and proves
/// nothing, which is the failure mode this whole file exists to avoid.
#[test]
fn the_comparison_is_over_a_list_with_rows_in_it() {
    let select = shape(&select_list());
    assert!(
        select.len() > LABELS.len(),
        "the select's list resolved to {} nodes over {} rows — there is not enough there for the \
         comparison above to be about anything",
        select.len(),
        LABELS.len(),
    );
    let panel = select[0];
    assert!(
        panel.3 > 0.0 && panel.2 > 0.0,
        "the panel has no area, so every rectangle inside it is zero and the comparison is vacuous"
    );
}

// ---------------------------------------------------------------------------------------------
// The shared behaviours, exercised through both controls (T029 — SC-008, FR-024, FR-025)
// ---------------------------------------------------------------------------------------------
//
// Everything above compares the two lists as *pictures*. What follows compares them as
// *behaviour*: anchoring, flipping and the keyboard rule, each asserted once and run against both
// controls.
//
// The point of one test body rather than two is the failure mode it forecloses. Two tests that
// each check "the list sits under the field" agree with the contract and can still come apart the
// day one of them is updated — which is precisely how a shared base stops being shared. Here there
// is one assertion and it names which control failed it.
//
// **What differs between the two, and is therefore a parameter rather than an assertion**: how the
// list is opened. The select owns its openness and opens by being pressed; the search picker's is
// its caller's and is posed. That asymmetry is the feature's central design decision
// (data-model §2.2), not an inconvenience, so it is stated once in `SUBJECTS` and nowhere else.

/// One picker under test, and the only thing that differs about reaching it.
struct Subject {
    name: &'static str,
    /// Build the control, open. Honoured by the control whose openness is its caller's; the other
    /// ignores it and is opened by [`Harness::open`] instead.
    build: fn() -> Element<'static, String>,
    opens_by_press: bool,
}

fn build_select() -> Element<'static, String> {
    Select::new(LABELS, Some(LABELS[0]), |t: &str| t.to_string(), roles())
        .label("Type")
        .into()
}

fn build_typeahead() -> Element<'static, String> {
    let rows: Vec<Row> = LABELS.iter().map(|l| Row::new(*l, Vec::new())).collect();
    Typeahead::new("", rows, |s: String| s, roles())
        .label("Type")
        .open(true)
        .selected(Some(0))
        .highlighted(Some(0))
        .on_pick(|i: usize| i.to_string())
        .on_move(|_| "moved".to_string())
        .on_dismiss("dismissed".to_string())
        .into()
}

const SUBJECTS: &[Subject] = &[
    Subject {
        name: "the select",
        build: build_select,
        opens_by_press: true,
    },
    Subject {
        name: "the search picker",
        build: build_typeahead,
        opens_by_press: false,
    },
];

/// One control, laid out with its field placed wherever the test needs it.
struct Harness {
    element: Element<'static, String>,
    tree: Tree,
    node: layout::Node,
    renderer: iced::Renderer,
    window: Size,
}

impl Harness {
    /// Build `subject`, place its field with its top-left at `at`, and open the list.
    fn open(subject: &Subject, window: Size, at: iced::Point) -> Self {
        let renderer = super::test_support::renderer();
        let mut element = (subject.build)();
        let mut tree = Tree::new(element.as_widget());
        let limits = layout::Limits::new(Size::ZERO, window);
        let mut node = element
            .as_widget_mut()
            .layout(&mut tree, &renderer, &limits)
            .move_to(at);

        if subject.opens_by_press {
            // Through `Layout` rather than off the node, because a child node's own bounds are
            // relative to its parent and this field has been moved away from the origin. Reading
            // them raw would press wherever the trigger *would* be in a window that starts at the
            // field — which is the origin case, and is why nothing noticed until now.
            let target = Layout::new(&node)
                .children()
                .next()
                .expect("the picker has a field")
                .bounds()
                .center();
            let origin = std::time::Instant::now();
            let send = |element: &mut Element<'static, String>,
                        tree: &mut Tree,
                        node: &mut layout::Node,
                        event: Event,
                        cursor: mouse::Cursor| {
                let mut messages: Vec<String> = Vec::new();
                let mut shell = iced::advanced::Shell::new(&mut messages);
                element.as_widget_mut().update(
                    tree,
                    &event,
                    Layout::new(node),
                    cursor,
                    &renderer,
                    &mut clipboard::Null,
                    &mut shell,
                    &Rectangle::with_size(window),
                );
                *node = element
                    .as_widget_mut()
                    .layout(tree, &renderer, &limits)
                    .move_to(at);
            };
            send(
                &mut element,
                &mut tree,
                &mut node,
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                mouse::Cursor::Available(target),
            );
            for i in 1..=2u32 {
                send(
                    &mut element,
                    &mut tree,
                    &mut node,
                    Event::Window(iced::window::Event::RedrawRequested(origin + FRAME * i)),
                    mouse::Cursor::Unavailable,
                );
            }
        }

        Self {
            element,
            tree,
            node,
            renderer,
            window,
        }
    }

    /// The field's own box — the thing the list anchors to.
    fn field(&self) -> Rectangle {
        self.node.bounds()
    }

    /// The floating list's box, in window coordinates.
    fn list(&mut self) -> Rectangle {
        let mut overlay = self
            .element
            .as_widget_mut()
            .overlay(
                &mut self.tree,
                Layout::new(&self.node),
                &self.renderer,
                &Rectangle::with_size(self.window),
                Vector::ZERO,
            )
            .expect("the picker floated no list");
        overlay
            .as_overlay_mut()
            .layout(&self.renderer, self.window)
            .bounds()
    }

    /// Send `key` the way the runtime does — the overlay first, the widget tree only if the
    /// overlay let it past — and report whether anything claimed it.
    ///
    /// The order is the whole reason this is worth simulating rather than asserting about either
    /// half alone: it is what lets a list claim Enter before the dialog behind it sees one, and it
    /// is why the select had to grow `ListWatch` at all.
    fn press_key(&mut self, named: iced::keyboard::key::Named) -> bool {
        let key = iced::keyboard::Key::Named(named);
        let event = Event::Keyboard(iced::keyboard::Event::KeyPressed {
            key: key.clone(),
            modified_key: key.clone(),
            physical_key: iced::keyboard::key::Physical::Unidentified(
                iced::keyboard::key::NativeCode::Unidentified,
            ),
            location: iced::keyboard::Location::Standard,
            modifiers: iced::keyboard::Modifiers::empty(),
            text: None,
            repeat: false,
        });
        let viewport = Rectangle::with_size(self.window);
        let mut messages: Vec<String> = Vec::new();
        let mut shell = iced::advanced::Shell::new(&mut messages);

        if let Some(mut overlay) = self.element.as_widget_mut().overlay(
            &mut self.tree,
            Layout::new(&self.node),
            &self.renderer,
            &viewport,
            Vector::ZERO,
        ) {
            let node = overlay.as_overlay_mut().layout(&self.renderer, self.window);
            overlay.as_overlay_mut().update(
                &event,
                Layout::new(&node),
                mouse::Cursor::Unavailable,
                &self.renderer,
                &mut clipboard::Null,
                &mut shell,
            );
        }
        if !shell.is_event_captured() {
            self.element.as_widget_mut().update(
                &mut self.tree,
                &event,
                Layout::new(&self.node),
                mouse::Cursor::Unavailable,
                &self.renderer,
                &mut clipboard::Null,
                &mut shell,
                &viewport,
            );
        }
        shell.is_event_captured()
    }
}

/// Half a pixel — the same tolerance the layout gates use, and far below anything visible.
const TOLERANCE: f32 = 0.5;

/// Both lists hang directly beneath their field, exactly as wide as it (contract §1.2).
#[test]
fn both_lists_anchor_under_their_field_at_its_width() {
    for subject in SUBJECTS {
        let mut h = Harness::open(subject, WINDOW, iced::Point::new(0.0, 40.0));
        let field = h.field();
        let list = h.list();

        assert!(
            (list.width - field.width).abs() < TOLERANCE,
            "{}'s list is {:.1}px wide against a {:.1}px field — a list that is not its field's \
             width reads as a different control rather than as that one's list",
            subject.name,
            list.width,
            field.width,
        );
        assert!(
            list.y >= field.y + field.height - TOLERANCE,
            "{}'s list starts at y={:.1} against a field ending at y={:.1}, so it is not beneath \
             it (contract §1.2)",
            subject.name,
            list.y,
            field.y + field.height,
        );
    }
}

/// With no room beneath, both lists flip above the field and stay inside the window (contract
/// §1.2, FR-005).
#[test]
fn both_lists_flip_above_the_field_when_there_is_no_room_below() {
    // A window barely taller than the field, placed so that a list hanging below would run off the
    // bottom. Anything that fits underneath is not testing the flip.
    let window = Size::new(WIDTH, 260.0);
    for subject in SUBJECTS {
        let mut h = Harness::open(subject, window, iced::Point::new(0.0, 170.0));
        let field = h.field();
        let list = h.list();

        assert!(
            list.y + list.height <= window.height + TOLERANCE,
            "{}'s list runs to y={:.1} in a {:.1}px window, so it left the screen rather than \
             flipping (FR-005)",
            subject.name,
            list.y + list.height,
            window.height,
        );
        assert!(
            list.y + list.height <= field.y + TOLERANCE,
            "{}'s list still hangs below a field at y={:.1} with no room for it — it should have \
             flipped above (contract §1.2)",
            subject.name,
            field.y,
        );
    }
}

/// Escape is the list's and is claimed; Tab dismisses and is **passed on** so focus still moves
/// (contract §1.4, FR-024).
///
/// The two keys are asserted together because the rule is that they differ: a control that claimed
/// both would leave a developer unable to tab past it, and one that claimed neither would let
/// Escape close the dialog behind the list as well.
#[test]
fn both_controls_claim_escape_and_pass_tab_on() {
    for subject in SUBJECTS {
        let mut h = Harness::open(subject, WINDOW, iced::Point::new(0.0, 40.0));
        assert!(
            h.press_key(iced::keyboard::key::Named::Escape),
            "{} let Escape past. It is the list's key, and a dialog behind an open list must not \
             also close on it (contract §1.4)",
            subject.name,
        );

        let mut h = Harness::open(subject, WINDOW, iced::Point::new(0.0, 40.0));
        assert!(
            !h.press_key(iced::keyboard::key::Named::Tab),
            "{} claimed Tab. Tab dismisses the list *and* belongs to the field — swallowing it \
             leaves a developer unable to tab past the picker at all (contract §1.4)",
            subject.name,
        );
    }
}

/// …and the parity above is over two genuinely different constructions.
///
/// Both `SUBJECTS` entries pointing at one control would make every assertion above pass twice
/// over the same code and prove nothing about sharing — the same vacuity
/// `the_comparison_is_over_a_list_with_rows_in_it` guards for the picture comparison.
#[test]
fn the_two_subjects_are_two_different_controls() {
    assert_eq!(SUBJECTS.len(), 2, "parity needs two controls to be parity");
    assert!(
        SUBJECTS[0].opens_by_press != SUBJECTS[1].opens_by_press,
        "both subjects open the same way, so one of them is not the control it claims to be — the \
         asymmetry of ownership is the thing that makes this a parity test (data-model §2.2)"
    );
}
