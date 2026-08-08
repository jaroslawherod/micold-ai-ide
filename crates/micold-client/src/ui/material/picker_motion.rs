//! The two lists animate by one definition (feature 022, T026 — FR-021, SC-007).
//!
//! [`picker_parity`](super::picker_parity) compares the two lists' *geometry* and finds them
//! identical. It has to stop there: the transition is `scale` and `fade`, and both transform
//! drawing only — a list mid-flight occupies exactly the boxes it occupies at rest, which is
//! FR-023 holding and is also why no comparison of rectangles can see the animation at all.
//!
//! So this file asserts the transition the two other ways it is observable.
//!
//! # 1. How long a closed list stays
//!
//! [`the_two_lists_leave_over_the_same_number_of_frames`] is behavioural. `cdk::picker` keeps
//! floating a list for as long as its visibility track says there is any of it left, and that track
//! is driven by the `exit` duration the material layer hands over — so *how many frames a closed
//! list survives* is the exit duration made observable without a rasteriser. Both pickers must
//! survive the same number, and that number must be §6.3's `short_2`.
//!
//! The two controls are closed in the two different ways their designs allow: the select by a press
//! outside itself, the search picker by being rebuilt with `open(false)`. That asymmetry is
//! deliberate and is the feature's central design decision (data-model §2.2) — what is compared is
//! what happens *after* the close, which is the part they share.
//!
//! # 2. What the other half is, and why it is read rather than measured
//!
//! The enter duration and both curves reach only `draw`, and this crate's test renderer resolves
//! layout rather than rasterising, so there is nothing here that could measure them. What *can* be
//! established is the property the contract actually asks for: that there is **one** definition
//! rather than two that agree today. [`neither_picker_names_a_duration_or_a_curve_of_its_own`]
//! reads the two controls and fails on a duration, a token or a curve appearing in either — the
//! numbers live in `material::picker` and both controls ask for them.
//!
//! That is a weaker kind of proof than the one above, and saying so is the point: a source check
//! cannot know what a call site *means*. It is here because the alternative — restating 150 and 100
//! against each control in turn — is exactly the shape SC-007 exists to forbid, where two tests
//! agree with the contract and drift from each other.
//!
//! In-crate because `material` is `pub(crate)` and neither control can be constructed from `tests/`.

use std::path::Path;
use std::time::Instant;

use iced::advanced::widget::Tree;
use iced::advanced::{clipboard, layout, mouse, Layout, Shell};
use iced::{Element, Event, Point, Rectangle, Size, Vector};
use micold_core::tokens::motion::duration;
use micold_core::tokens::{self, Roles};

use super::picker::{Row, EXIT};
use super::{Select, Typeahead};
use crate::ui::cdk::motion::FRAME;

const LABELS: &[&str] = &["one", "two", "three"];
const WINDOW: Size = Size::new(400.0, 800.0);

/// Well past any exit this feature could reasonably introduce. A list still floating after this
/// many frames is not leaving slowly, it is not leaving.
const GIVE_UP_AFTER: u32 = 60;

fn roles() -> Roles {
    tokens::roles(micold_core::theme::ColorScheme::Light)
}

/// One picker, laid out, with a tree that survives across frames.
///
/// The tree is built once and reused on purpose, for the reason `tests/picker_visibility.rs` states
/// about its own: the subject is state that must outlive `open` going false, and a fresh tree each
/// frame would hide exactly that.
struct Harness {
    element: Element<'static, String>,
    tree: Tree,
    node: layout::Node,
    renderer: iced::Renderer,
    origin: Instant,
    clock: u32,
}

impl Harness {
    fn new(element: Element<'static, String>) -> Self {
        let renderer = super::test_support::renderer();
        let mut element = element;
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
            origin: Instant::now(),
            clock: 0,
        }
    }

    /// Deliver one event and re-resolve the layout, as the runtime does.
    fn deliver(&mut self, event: Event, cursor: mouse::Cursor) {
        let mut messages: Vec<String> = Vec::new();
        let mut shell = Shell::new(&mut messages);
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
        self.node = self.element.as_widget_mut().layout(
            &mut self.tree,
            &self.renderer,
            &layout::Limits::new(Size::ZERO, WINDOW),
        );
    }

    /// Advance one frame. Only `RedrawRequested` moves a `Progress`, so this is the only thing that
    /// makes an exit progress at all.
    fn frame(&mut self) {
        self.clock += 1;
        let at = self.origin + FRAME * self.clock;
        self.deliver(
            Event::Window(iced::window::Event::RedrawRequested(at)),
            mouse::Cursor::Unavailable,
        );
    }

    fn press_at(&mut self, at: Point) {
        self.deliver(
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            mouse::Cursor::Available(at),
        );
    }

    /// Rebuild from a new element, keeping the tree — how a caller-owned openness changes.
    fn rebuild(&mut self, element: Element<'static, String>) {
        self.element = element;
        self.element.as_widget().diff(&mut self.tree);
        self.node = self.element.as_widget_mut().layout(
            &mut self.tree,
            &self.renderer,
            &layout::Limits::new(Size::ZERO, WINDOW),
        );
    }

    fn floats(&mut self) -> bool {
        self.element
            .as_widget_mut()
            .overlay(
                &mut self.tree,
                Layout::new(&self.node),
                &self.renderer,
                &Rectangle::with_size(WINDOW),
                Vector::ZERO,
            )
            .is_some()
    }

    /// How many frames the list keeps being produced, counting from now.
    fn frames_until_gone(&mut self) -> u32 {
        assert!(
            self.floats(),
            "the list was already gone the frame it was closed, so there is no exit to measure — \
             either the control never opened, or `overlay()` is gated on `open` alone again"
        );
        for survived in 1..=GIVE_UP_AFTER {
            self.frame();
            if !self.floats() {
                return survived;
            }
        }
        panic!(
            "the list was still floating {GIVE_UP_AFTER} frames after it closed — an exit track \
             that never settles looks perfectly fine and burns a core forever"
        );
    }
}

/// The select: opened by a press on its trigger, closed by a press outside it. Its openness is its
/// own, so both are interactions and there is nothing to rebuild.
fn select_exit() -> u32 {
    let r = roles();
    let mut h = Harness::new(
        Select::new(LABELS, Some(LABELS[0]), |t: &str| t.to_string(), r)
            .label("Type")
            .into(),
    );

    h.press_at(h.node.children()[0].bounds().center());
    // Two frames, so the opening track has settled before the closing one is measured — an exit
    // interrupted mid-entry is FR-021's business and would measure something shorter.
    h.frame();
    h.frame();
    h.press_at(Point::new(WINDOW.width - 1.0, WINDOW.height - 1.0));

    h.frames_until_gone()
}

/// The search picker: built open, then rebuilt closed. Its openness is the caller's, so closing it
/// is a rebuild — which is what the application does when `on_dismiss` comes back.
fn typeahead_exit() -> u32 {
    let r = roles();
    let build = |open: bool| -> Element<'static, String> {
        let rows: Vec<Row> = LABELS.iter().map(|l| Row::new(*l, Vec::new())).collect();
        Typeahead::new("", rows, |s: String| s, r)
            .label("Type")
            .open(open)
            .on_pick(|i: usize| i.to_string())
            .into()
    };

    let mut h = Harness::new(build(true));
    h.frame();
    h.frame();
    h.rebuild(build(false));

    h.frames_until_gone()
}

/// Both lists take the same number of frames to leave, and it is `short_2`'s.
///
/// The comparison is of the two controls against each other **and** against §6.3, in one test,
/// because either alone permits the failure the other catches: two controls that agree on a wrong
/// number, or two right numbers written down twice.
#[test]
fn the_two_lists_leave_over_the_same_number_of_frames() {
    let select = select_exit();
    let typeahead = typeahead_exit();

    assert_eq!(
        select, typeahead,
        "the select's list leaves over {select} frames and the search picker's over {typeahead} — \
         they are two transitions rather than one, and a person switching between the controls \
         would see it (FR-021, SC-001)"
    );

    let expected = (EXIT.as_secs_f32() / FRAME.as_secs_f32()).ceil() as u32;
    assert_eq!(
        select,
        expected,
        "a closed list survives {select} frames against the {expected} that `short_2` ({}ms) buys \
         at one frame per {}ms — the exit is not the duration §6.3 publishes for a menu fading out",
        duration::SHORT_2,
        FRAME.as_millis(),
    );
}

/// Neither control names a duration, a motion token or a curve. There is one definition and both
/// ask for it.
///
/// Scoped to the two controls rather than to the whole layer: `material::picker` *must* name
/// `short_3` and `short_2`, because it is the one definition. A check that forbade the tokens
/// everywhere would forbid the thing it is trying to require.
#[test]
fn neither_picker_names_a_duration_or_a_curve_of_its_own() {
    /// What a control naming its own motion looks like. `Motion`'s own defaults are §6.3's menu
    /// curves, so a curve appearing at a call site is a restatement even when it restates the right
    /// one — a second definition that agrees today is still a second definition.
    const FORBIDDEN: &[(&str, &str)] = &[
        (
            "Duration::from_millis",
            "a duration spelled out rather than asked for",
        ),
        ("duration::", "a motion token named at a call site"),
        (
            "STANDARD_DECELERATE",
            "a curve restated over `Motion`'s default",
        ),
        (
            "STANDARD_ACCELERATE",
            "a curve restated over `Motion`'s default",
        ),
        ("easing::", "a curve named at a call site"),
    ];

    for control in ["typeahead.rs", "select.rs"] {
        let source = read_control(control);

        assert!(
            source.contains("animated_menu"),
            "{control} does not reach `picker::animated_menu`, so whatever brings its list in is \
             not the shared transition (FR-018, FR-019)"
        );

        for (needle, why) in FORBIDDEN {
            assert!(
                !source.contains(needle),
                "{control} contains `{needle}` — {why}. The enter and exit durations are \
                 `material::picker`'s `ENTER` and `EXIT`, and both curves are `Motion`'s own \
                 defaults, which is why neither is stated at a call site (FR-020, SC-007)"
            );
        }
    }
}

/// …and the check above is reading the files it thinks it is.
///
/// A path that stopped resolving would make every `contains` above vacuously false, and the test
/// would pass by finding nothing in nothing.
#[test]
fn the_source_check_reads_both_controls() {
    for control in ["typeahead.rs", "select.rs"] {
        let source = read_control(control);
        assert!(
            source.len() > 1_000,
            "{control} read as {} bytes, which is not a control — the path has stopped resolving \
             and the check above is passing over an empty string",
            source.len(),
        );
    }
}

fn read_control(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/ui/material")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}
