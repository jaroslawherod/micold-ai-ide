//! A picker's list outlives its own closing (feature 022, T003–T005 — FR-019, FR-022).
//!
//! `cdk::picker` returned `None` from `overlay()` the instant `open` went false, which removes the
//! widget and takes its animation state with it. A closing list therefore vanished between frames:
//! there was nothing left on screen to fade. This file pins the fix, and the two things that make it
//! safe rather than merely pretty.
//!
//! **The invariant**: `progress > 0` ⟺ `overlay()` returns `Some`. Stated as an equivalence because
//! both halves can fail and they fail differently. If a settled picker still yields an overlay, an
//! invisible surface sits over the page swallowing presses. If a closing one stops yielding
//! immediately, the exit animation has nothing to animate.
//!
//! **The two dangers of the fix**, each with a test below:
//!
//! - A list on its way out must accept **no** input (FR-022). It is on screen only in the sense that
//!   it is still fading, and a press landing where a row used to be must do nothing. "Draws nothing"
//!   is not enough — the overlay has to refuse.
//! - A settled picker must ask for **no** frames. An exit track that never finishes looks perfectly
//!   fine and burns a core forever; `idle_requests_no_frames.rs` guards the mechanism, and this
//!   guards this widget's use of it.
//!
//! `ui::cdk` is `pub`, so all of this is reachable from an integration test. `ui::material` is not,
//! which is why the select's own gates live in-crate instead.

#[path = "support/mod.rs"]
mod support;

use std::time::{Duration, Instant};

use iced::advanced::widget::Tree;
use iced::advanced::{mouse, Shell, Widget};
use iced::widget::{container, text};
use iced::{keyboard, window, Element, Event, Length, Point, Rectangle, Size, Vector};

use micold_client::ui::cdk::motion::FRAME;
use micold_client::ui::cdk::picker::Picker;

use support::layout::renderer;

/// Long enough that a handful of frames does not finish it, short enough that a loop can.
const EXIT: Duration = Duration::from_millis(100);

/// The gap between field and list. Any value; this one is realistic.
const GAP: f32 = 4.0;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Msg {
    Picked,
    Dismissed,
}

/// A picker with a one-row list, at `open`.
fn picker<'a>(open: bool) -> Picker<'a, Msg> {
    let field = container(text("field")).width(Length::Fixed(200.0));
    let menu = container(text("row")).width(Length::Fill);
    Picker::new(field, menu, open, GAP)
        .exit(EXIT)
        .keyboard(Some(0), 1, true)
        .on_pick(Msg::Picked)
        .on_dismiss(Msg::Dismissed)
}

/// Lay the picker out and hand back everything an `overlay()` call needs.
///
/// The tree is built once and reused across frames on purpose: the whole subject here is state that
/// must *survive* `open` going false, and a fresh tree each frame would hide exactly that.
struct Harness<'a> {
    picker: Picker<'a, Msg>,
    tree: Tree,
    renderer: iced::Renderer,
    /// A monotonic frame clock, one nominal [`FRAME`] apart.
    ///
    /// A track advances by the wall-clock time between the frame it is handed and the one before it
    /// (007 BUG-001), so the redraws a test hands over have to be spaced the way a display spaces
    /// them. Two `Instant::now()`s taken inside a loop are microseconds apart, and six hundred of
    /// them would not add up to one frame of a 100 ms exit.
    origin: Instant,
    clock: u32,
}

impl<'a> Harness<'a> {
    fn new(open: bool) -> Self {
        let picker = picker(open);
        let tree = Tree {
            tag: picker.tag(),
            state: picker.state(),
            children: picker.children(),
        };
        Self {
            picker,
            tree,
            renderer: renderer(),
            origin: Instant::now(),
            clock: 0,
        }
    }

    /// Close the list, keeping the tree — this is the transition under test.
    fn close(&mut self) {
        self.picker = picker(false);
        self.picker.diff(&mut self.tree);
    }

    /// Whether an overlay is produced right now.
    fn has_overlay(&mut self) -> bool {
        let limits = iced::advanced::layout::Limits::new(Size::ZERO, Size::new(400.0, 400.0));
        let node = self
            .picker
            .layout(&mut self.tree, &self.renderer, &limits)
            .move_to(Point::ORIGIN);
        let viewport = Rectangle::with_size(Size::new(400.0, 400.0));
        let present = {
            let layout = iced::advanced::Layout::new(&node);
            self.picker
                .overlay(
                    &mut self.tree,
                    layout,
                    &self.renderer,
                    &viewport,
                    Vector::ZERO,
                )
                .is_some()
        };
        present
    }

    /// Advance one frame, returning whether a redraw was requested.
    fn frame(&mut self) -> bool {
        self.clock += 1;
        let mut messages = Vec::new();
        let mut shell = Shell::new(&mut messages);
        let limits = iced::advanced::layout::Limits::new(Size::ZERO, Size::new(400.0, 400.0));
        let node = self.picker.layout(&mut self.tree, &self.renderer, &limits);
        let layout = iced::advanced::Layout::new(&node);
        let viewport = Rectangle::with_size(Size::new(400.0, 400.0));
        self.picker.update(
            &mut self.tree,
            &Event::Window(window::Event::RedrawRequested(
                self.origin + FRAME * self.clock,
            )),
            layout,
            mouse::Cursor::Unavailable,
            &self.renderer,
            &mut iced::advanced::clipboard::Null,
            &mut shell,
            &viewport,
        );
        shell.redraw_request() != window::RedrawRequest::Wait
    }

    /// Run frames until the exit settles, or give up. Returns the frame count.
    fn settle(&mut self) -> usize {
        for i in 0..600 {
            if !self.has_overlay() {
                return i;
            }
            self.frame();
        }
        panic!("the exit never settled — a track that never finishes burns a core forever");
    }
}

// ---------------------------------------------------------------------------------------------
// T003 — the invariant
// ---------------------------------------------------------------------------------------------

/// An open picker shows its list. The trivial half, here so the interesting half means something.
#[test]
fn an_open_picker_yields_an_overlay() {
    let mut h = Harness::new(true);
    assert!(h.has_overlay(), "an open picker must show its list");
}

/// A picker that has never been opened shows nothing — and, in particular, does not animate one
/// into existence on mount.
#[test]
fn a_picker_that_was_never_opened_yields_nothing() {
    let mut h = Harness::new(false);
    assert!(
        !h.has_overlay(),
        "a picker built closed must not produce a list at all — an element built already-closed \
         has no transition to play"
    );
}

/// The heart of it: closing does not remove the list, it starts it leaving.
#[test]
fn a_just_closed_picker_still_yields_its_overlay() {
    let mut h = Harness::new(true);
    assert!(h.has_overlay(), "precondition: it was open");

    h.close();
    assert!(
        h.has_overlay(),
        "the list disappeared the instant it was closed, so there is nothing left to fade out — \
         which is the defect this feature exists to fix"
    );
}

/// And it does eventually go. An overlay that outlived its exit would sit invisible over the page,
/// swallowing every press that landed on it.
#[test]
fn the_overlay_goes_once_the_exit_has_settled() {
    let mut h = Harness::new(true);
    h.close();
    let frames = h.settle();
    assert!(
        frames > 0,
        "the exit finished before a single frame ran, which means it did not animate at all"
    );
    assert!(
        !h.has_overlay(),
        "the list is still there after its exit settled"
    );
}

// ---------------------------------------------------------------------------------------------
// T004 — a leaving list takes no input (FR-022)
// ---------------------------------------------------------------------------------------------

/// A press where a row used to be must choose nothing while the list is on its way out.
///
/// This is the failure mode that makes the fix dangerous rather than merely incomplete: without it,
/// the feature adds a 100 ms window in which the application looks closed and still acts open.
#[test]
fn a_leaving_list_publishes_nothing_for_a_press() {
    let mut h = Harness::new(true);
    h.close();
    assert!(h.has_overlay(), "precondition: it is still leaving");

    let mut messages: Vec<Msg> = Vec::new();
    let mut shell = Shell::new(&mut messages);
    let limits = iced::advanced::layout::Limits::new(Size::ZERO, Size::new(400.0, 400.0));
    let node = h.picker.layout(&mut h.tree, &h.renderer, &limits);
    let layout = iced::advanced::Layout::new(&node);
    let viewport = Rectangle::with_size(Size::new(400.0, 400.0));

    let mut overlay = h
        .picker
        .overlay(&mut h.tree, layout, &h.renderer, &viewport, Vector::ZERO)
        .expect("precondition: an overlay while leaving");

    let overlay_node = overlay
        .as_overlay_mut()
        .layout(&h.renderer, Size::new(400.0, 400.0));
    let overlay_layout = iced::advanced::Layout::new(&overlay_node);
    let inside = overlay_layout.bounds().center();

    overlay.as_overlay_mut().update(
        &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        overlay_layout,
        mouse::Cursor::Available(inside),
        &h.renderer,
        &mut iced::advanced::clipboard::Null,
        &mut shell,
    );

    assert!(
        messages.is_empty(),
        "a list on its way out accepted a press and published {messages:?} — it is on screen only \
         in the sense that it is still fading"
    );
}

/// The same for the keyboard. Enter must not take a row out of a list that is leaving.
#[test]
fn a_leaving_list_publishes_nothing_for_a_key() {
    let mut h = Harness::new(true);
    h.close();
    assert!(h.has_overlay(), "precondition: it is still leaving");

    let mut messages: Vec<Msg> = Vec::new();
    let mut shell = Shell::new(&mut messages);
    let limits = iced::advanced::layout::Limits::new(Size::ZERO, Size::new(400.0, 400.0));
    let node = h.picker.layout(&mut h.tree, &h.renderer, &limits);
    let layout = iced::advanced::Layout::new(&node);
    let viewport = Rectangle::with_size(Size::new(400.0, 400.0));

    let mut overlay = h
        .picker
        .overlay(&mut h.tree, layout, &h.renderer, &viewport, Vector::ZERO)
        .expect("precondition: an overlay while leaving");

    let overlay_node = overlay
        .as_overlay_mut()
        .layout(&h.renderer, Size::new(400.0, 400.0));
    let overlay_layout = iced::advanced::Layout::new(&overlay_node);

    overlay.as_overlay_mut().update(
        &Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(keyboard::key::Named::Enter),
            modified_key: keyboard::Key::Named(keyboard::key::Named::Enter),
            physical_key: keyboard::key::Physical::Unidentified(
                keyboard::key::NativeCode::Unidentified,
            ),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::default(),
            text: None,
            repeat: false,
        }),
        overlay_layout,
        mouse::Cursor::Unavailable,
        &h.renderer,
        &mut iced::advanced::clipboard::Null,
        &mut shell,
    );

    assert!(
        messages.is_empty(),
        "a list on its way out took a row from the keyboard and published {messages:?}"
    );
}

// ---------------------------------------------------------------------------------------------
// T005 — nothing asks for a frame at rest
// ---------------------------------------------------------------------------------------------

/// A picker that has never opened asks for nothing. This is the case that decides whether a
/// backgrounded window burns CPU, because most pickers on a page are in it.
#[test]
fn a_closed_picker_asks_for_no_frames() {
    let mut h = Harness::new(false);
    for _ in 0..10 {
        assert!(
            !h.frame(),
            "a picker that was never opened asked for a frame"
        );
    }
}

/// An open, settled picker asks for nothing. Being visible is not being animated.
#[test]
fn a_settled_open_picker_asks_for_no_frames() {
    let mut h = Harness::new(true);
    for _ in 0..10 {
        h.frame();
    }
    for _ in 0..10 {
        assert!(
            !h.frame(),
            "an open picker that had finished arriving kept asking for frames"
        );
    }
}

/// And once the exit has run, the picker goes quiet again rather than spinning forever on a track
/// that never quite reaches zero.
#[test]
fn a_picker_asks_for_no_frames_once_its_exit_has_settled() {
    let mut h = Harness::new(true);
    h.close();
    h.settle();
    for _ in 0..10 {
        assert!(
            !h.frame(),
            "the picker kept asking for frames after its exit settled — an exit track that never \
             finishes looks fine and burns a core forever"
        );
    }
}

/// The converse, so the three tests above cannot pass by the picker never animating at all.
#[test]
fn a_leaving_picker_does_ask_for_frames() {
    let mut h = Harness::new(true);
    h.close();
    assert!(
        h.frame(),
        "a picker in the middle of leaving asked for no frame, so nothing will advance it"
    );
}

/// A dummy so the unused-import lint stays honest about `Element` if the harness changes shape.
#[allow(dead_code)]
fn _element_type_is_used<'a>(p: Picker<'a, Msg>) -> Element<'a, Msg> {
    p.into()
}
