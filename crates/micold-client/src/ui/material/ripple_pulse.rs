//! The reference scene's ripple, and the rule it presses by (feature 018, T077 — FR-039b, FR-024e).
//!
//! In-crate for the same reason as `ripple_clipping.rs`: `material` is `pub(crate)`, and the state
//! this traverses is a `Ripple` widget's own tree state, which is not reachable from `tests/`.
//!
//! # Why this is tested at all
//!
//! [`super::ripple::pulse`] looks like glue and is not. It decides *which* ripple to press and *when*, and
//! Principle I's GUI-wiring exception explicitly does not cover "code with decision logic,
//! branching, or a business rule of its own". The rule has a right answer and several plausible
//! wrong ones, and the wrong ones are invisible: every one of them still produces a scene that
//! reports "a ripple is animating" and still yields a figure that looks exactly like a good one.
//! §B8's whole purpose is that a figure cannot be recorded against a scene nobody verified, so the
//! verification itself has to be checked.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use iced::advanced::widget::{Operation, Tree};
use iced::advanced::{layout, Layout};
use iced::widget::{column, Space};
use iced::{Element, Length, Size};
use micold_core::tokens::{self, shape};

use super::Ripple;
use crate::showcase::state::Message;

fn roles() -> tokens::Roles {
    tokens::roles(micold_core::theme::ColorScheme::Light)
}

/// A column of `n` rippling rows, laid out — the shape a sidebar or a menu presents.
fn rippling_rows<'a>(n: usize) -> Element<'a, Message> {
    let r = roles();
    let mut col = column![];
    for _ in 0..n {
        let row: Element<'_, Message> = Space::new()
            .width(Length::Fixed(200.0))
            .height(Length::Fixed(40.0))
            .into();
        col = col.push(Ripple::new(row, r.on_surface, shape::FULL));
    }
    col.into()
}

/// Run one traversal over `element`, as the frame probe does once per frame.
fn pulse_once(element: &mut Element<'_, Message>, tree: &mut Tree, found: &Arc<AtomicUsize>) {
    let renderer = super::test_support::renderer();
    let limits = layout::Limits::new(Size::ZERO, Size::new(400.0, 2000.0));
    let node = element.as_widget_mut().layout(tree, &renderer, &limits);
    let mut op = super::ripple::pulse(Arc::clone(found));
    element
        .as_widget_mut()
        .operate(tree, Layout::new(&node), &renderer, &mut op);
    // The count is published on `finish`, which the runtime calls once the traversal is done.
    op.finish();
}

/// One press, not one per ripple on screen.
///
/// The scene is "the baseline **plus a ripple** mid-animation" — singular. A traversal that pressed
/// every ripple it found would measure a scene the contract does not describe, and would do it
/// silently: `Scene::check` only asks whether *a* ripple is animating, so twenty of them satisfy it
/// exactly as well as one.
#[test]
fn a_pulse_presses_one_ripple_and_not_the_rest() {
    let mut element = rippling_rows(5);
    let mut tree = Tree::new(&element);
    let found = Arc::new(AtomicUsize::new(0));

    pulse_once(&mut element, &mut tree, &found);

    assert_eq!(
        found.load(Ordering::Relaxed),
        1,
        "the first traversal started {} ripples — the full scene is the baseline plus *a* ripple",
        found.load(Ordering::Relaxed)
    );
}

/// Repeated pulses do not accumulate ripples.
///
/// The frame probe pulses on **every** frame of a `full` run, for the whole 300 it counts, because a
/// ripple lives about half a second and one pressed during composition would have settled long
/// before the end. So the rule has to be stable under repetition rather than merely correct once:
/// a traversal that presses "the first *idle* one" starts a second ripple on the next frame while
/// the first is still running, a third on the frame after, and has every ripple on screen animating
/// within as many frames as there are rows. The scene check never notices — it only asks whether
/// *a* ripple is animating — so the figure would be recorded against a screen full of them.
#[test]
fn pulsing_every_frame_keeps_one_ripple_rather_than_gathering_them() {
    let mut element = rippling_rows(5);
    let mut tree = Tree::new(&element);
    let found = Arc::new(AtomicUsize::new(0));

    for frame in 0..5 {
        pulse_once(&mut element, &mut tree, &found);
        assert!(
            found.load(Ordering::Relaxed) <= 1,
            "after {} pulses {} ripples are animating at once — pressing whichever one happens to \
             be idle starts a fresh one every frame, so the scene fills up with them",
            frame + 1,
            found.load(Ordering::Relaxed)
        );
    }
}

/// A ripple already under way is left alone, not restarted.
///
/// Re-pressing one every frame holds it at zero expansion for ever: it is a ripple that never gets
/// anywhere, which is not "mid-animation" in any sense the scene means. Asserted through the press
/// count rather than by reading expansion, because what must not happen is the *press*.
#[test]
fn a_running_ripple_is_not_pressed_again() {
    let mut element = rippling_rows(1);
    let mut tree = Tree::new(&element);
    let found = Arc::new(AtomicUsize::new(0));

    pulse_once(&mut element, &mut tree, &found);
    assert_eq!(found.load(Ordering::Relaxed), 1, "nothing was pressed");

    // The lone ripple is now running. Pulse again: it must still be the same one.
    pulse_once(&mut element, &mut tree, &found);
    assert_eq!(
        found.load(Ordering::Relaxed),
        1,
        "the running ripple was disturbed rather than left to finish"
    );
}

/// A tree with nothing to press reports nothing, rather than reporting what it wanted.
///
/// This is the half that keeps the scene check honest. `Scene::Full` refuses to record a figure
/// until a ripple is actually animating, and that refusal is worth nothing if the traversal reports
/// its own intent instead of what it found.
#[test]
fn a_tree_with_no_ripples_reports_none() {
    let mut element: Element<'_, Message> = column![Space::new()
        .width(Length::Fixed(200.0))
        .height(Length::Fixed(40.0))]
    .into();
    let mut tree = Tree::new(&element);
    let found = Arc::new(AtomicUsize::new(7));

    pulse_once(&mut element, &mut tree, &found);

    assert_eq!(
        found.load(Ordering::Relaxed),
        0,
        "a traversal that found no ripple still reported one"
    );
}
