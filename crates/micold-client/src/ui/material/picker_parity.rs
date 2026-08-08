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
