//! The select's own anatomy, and the two behaviours nothing outside it can observe (feature 022,
//! T012/T014 — FR-002, FR-003, FR-004, FR-013).
//!
//! In-crate for the reason `text_field_anatomy.rs` and `form_field_anatomy.rs` record: `material` is
//! `pub(crate)`, so a `Select` cannot be constructed from `tests/` at all. tasks.md names
//! `crates/micold-client/src/ui/material/select_anatomy.rs` for exactly that reason.
//!
//! # What is worth asserting here, and what is asserted elsewhere
//!
//! The chrome around the trigger is `FormField`'s and is already gated by `form_field_anatomy.rs`;
//! repeating it here would pin the same decision twice and make one of the two a liar the day it
//! changes. What is *this* component's, and is checked below:
//!
//! - the trigger is that shared chrome rather than a box of its own — asserted by measuring a select
//!   against a text field, not by restating 56dp;
//! - the label rests on the value's line while nothing is chosen and floats once something is;
//! - the trailing chevron is §7.7's 24dp glyph, drawn in the muted role rather than the value's;
//! - **the active indicator answers for itself** — it thickens because the component knows it is
//!   open, with nothing supplying that. This is accepted fidelity gap #3 closing structurally
//!   (FR-013), so it is measured in drawn pixels rather than inferred from a flag: a test that read
//!   the flag would pass against a build whose indicator never reached the screen.
//! - opening seeds the keyboard highlight from the current choice (feature 013's FR-003), asserted
//!   through what Enter *takes* rather than by reading private state — the row being reachable is
//!   the requirement, and the flag is only how it is met.
//!
//! The message type here is `String`, not the application's own: a `Select` is generic over its
//! message, and reporting the chosen option as its own text is what makes "which option did that
//! take?" answerable at all. `showcase::state::Message` maps every choice to `NoOp`.

use iced::advanced::renderer::Headless as _;
use iced::advanced::widget::Tree;
use iced::advanced::{clipboard, layout, mouse, renderer, Layout, Renderer as _};
use iced::{Color, Element, Event, Point, Rectangle, Size, Vector};
use micold_core::tokens::{self, anatomy, Roles};

use super::{style, Select, TextField};
/// One frame, named rather than restated — `cdk::motion` steps by exactly this, so a track ticked
/// at any other interval would be asked about a moment the runtime never produces.
use crate::ui::cdk::motion::FRAME;

/// The options every select below offers.
const OPTIONS: &[&str] = &["one", "two", "three"];

/// The width the field is laid out at, and the window it is laid out in.
const WIDTH: f32 = 400.0;
const WINDOW: Size = Size::new(WIDTH, 800.0);

/// One pixel of subpixel rounding, and nothing more. The indicator's two states differ by a whole
/// device pixel, so this separates them with room to spare.
const TOLERANCE: f32 = 0.6;

fn roles() -> Roles {
    tokens::roles(micold_core::theme::ColorScheme::Light)
}

/// A select over [`OPTIONS`], reporting the chosen option as its own text.
fn select<'a>(selected: Option<&'a str>, r: Roles) -> Select<'a, &'a str, String> {
    Select::new(OPTIONS, selected, |t: &str| t.to_string(), r)
        .placeholder("Choose one…")
        .label("Type")
}

/// A mounted element, its widget tree and its layout — the three things that have to stay in step
/// for an event to land where the test thinks it does.
///
/// A select owns its openness, so a test cannot pose it: it has to be *driven*, the way a person
/// drives it. That is the whole point of the design and it is what this harness exists for.
struct Mounted<'a> {
    element: Element<'a, String>,
    tree: Tree,
    node: layout::Node,
    renderer: iced::Renderer,
    /// The clock the frame ticks are stamped with. A track ignores a frame it has already seen, so
    /// two ticks at the same instant advance it once and the counter is what keeps them apart.
    origin: std::time::Instant,
    frame: u32,
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
            origin: std::time::Instant::now(),
            frame: 0,
        }
    }

    /// The whole field, including whatever sits beneath the container.
    fn bounds(&self) -> Rectangle {
        self.node.bounds()
    }

    /// The filled container alone — the first of `FormField`'s two bands.
    fn container(&self) -> Rectangle {
        self.node.children()[0].bounds()
    }

    /// The container's four slots: `[leading, control, trailing, label]`.
    fn slots(&self) -> Vec<Rectangle> {
        self.node.children()[0]
            .children()
            .iter()
            .map(|c| c.bounds())
            .collect()
    }

    /// Dispatch one event, settle the frame the list's visibility track needs, and return whatever
    /// was published.
    ///
    /// The settle is not padding. `cdk::picker` keeps the list on screen for as long as its own
    /// visibility track says there is any of it left, and that track only moves on a frame tick —
    /// so a list that has just been opened or closed is *between* the two states until one arrives.
    /// The rendering stack delivers that tick because the track asks for it; here it has to be
    /// handed over. Without it every open/close assertion below would read the previous frame.
    fn dispatch(&mut self, event: Event, cursor: mouse::Cursor) -> Vec<String> {
        let published = self.send(event, cursor);
        // The rendering stack re-lays out whenever a widget invalidates its layout, and a select
        // that just opened has done exactly that. Doing it unconditionally keeps the harness one
        // line instead of a branch that could disagree with the widget about when it matters.
        self.relayout();
        self.settle();
        published
    }

    /// Hand over the frames the visibility track needs to reach wherever the last event sent it.
    ///
    /// Two frames were enough until T027 gave the select the shared transition: opening is
    /// immediate, but a *closed* list now keeps being produced for the whole of `picker::EXIT` so
    /// that there is something left to fade (FR-019). So this settles the exit rather than a fixed
    /// couple of ticks — derived from the duration rather than written down as a number beside it,
    /// because a harness holding its own copy of a timing is how two definitions start.
    fn settle(&mut self) {
        let frames = (super::picker::EXIT.as_secs_f32() / FRAME.as_secs_f32()).ceil() as u32 + 1;
        for _ in 0..frames {
            self.frame += 1;
            let at = self.origin + FRAME * self.frame;
            let _ = self.send(
                Event::Window(iced::window::Event::RedrawRequested(at)),
                mouse::Cursor::Unavailable,
            );
        }
        self.relayout();
    }

    /// One event, with no settling — the raw dispatch [`Self::dispatch`] is built from.
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

    fn relayout(&mut self) {
        self.node = self.element.as_widget_mut().layout(
            &mut self.tree,
            &self.renderer,
            &layout::Limits::new(Size::ZERO, WINDOW),
        );
    }

    /// Press the trigger, the way a person opens the list.
    fn press_trigger(&mut self) -> Vec<String> {
        let at = self.container().center();
        self.dispatch(
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            mouse::Cursor::Available(at),
        )
    }

    fn key(&mut self, named: iced::keyboard::key::Named) -> Vec<String> {
        self.dispatch(
            Event::Keyboard(iced::keyboard::Event::KeyPressed {
                key: iced::keyboard::Key::Named(named),
                modified_key: iced::keyboard::Key::Named(named),
                physical_key: iced::keyboard::key::Physical::Unidentified(
                    iced::keyboard::key::NativeCode::Unidentified,
                ),
                location: iced::keyboard::Location::Standard,
                modifiers: iced::keyboard::Modifiers::empty(),
                text: None,
                repeat: false,
            }),
            mouse::Cursor::Unavailable,
        )
    }

    /// Whether the field is floating a list beneath it.
    fn has_list(&mut self) -> bool {
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

    /// One event, reporting whether anything asked for another frame as a result.
    ///
    /// The ripple never requests a redraw directly — handling the press already causes one, and its
    /// track then chains from that (see `ripple.rs`). So "did a press start a ripple?" is answered
    /// by the *frame after* it still asking for another, which is what this exposes.
    fn send_probing_frames(&mut self, event: Event, cursor: mouse::Cursor) -> bool {
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
        shell.redraw_request() != iced::window::RedrawRequest::Wait
    }

    /// Press at `at` and report whether a ripple is running afterwards.
    fn ripples_when_pressed_at(&mut self, at: Point) -> bool {
        let _ = self.send(
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            mouse::Cursor::Available(at),
        );
        self.frame += 1;
        let stamp = self.origin + FRAME * self.frame;
        self.send_probing_frames(
            Event::Window(iced::window::Event::RedrawRequested(stamp)),
            mouse::Cursor::Available(at),
        )
    }

    /// Draw the field over `backdrop` and return the buffer with the size it was drawn at.
    fn screenshot(&mut self, backdrop: Color, on: Color) -> (Vec<u8>, u32, u32) {
        self.screenshot_under(backdrop, on, mouse::Cursor::Unavailable)
    }

    /// The same, with the pointer somewhere — the container reads hover off the cursor when it
    /// draws (BUG-002), so a hover screenshot has to hand one over.
    fn screenshot_under(
        &mut self,
        backdrop: Color,
        on: Color,
        cursor: mouse::Cursor,
    ) -> (Vec<u8>, u32, u32) {
        let size = self.node.bounds().size();
        let viewport = Rectangle::with_size(size);
        self.renderer.reset(viewport);
        self.element.as_widget().draw(
            &self.tree,
            &mut self.renderer,
            &super::theme(micold_core::theme::ColorScheme::Light),
            &renderer::Style { text_color: on },
            Layout::new(&self.node),
            cursor,
            &viewport,
        );
        let (w, h) = (size.width.ceil() as u32, size.height.ceil() as u32);
        (
            self.renderer.screenshot(Size::new(w, h), 1.0, backdrop),
            w,
            h,
        )
    }
}

/// The pixel at `(x, y)` as `(r, g, b)`.
fn pixel(pixels: &[u8], width: u32, x: u32, y: u32) -> [u8; 3] {
    let i = ((y * width + x) * 4) as usize;
    [pixels[i], pixels[i + 1], pixels[i + 2]]
}

/// How far two colours are apart, in the largest channel. Above the rasteriser's noise and well
/// below the difference between any two roles this file compares.
fn distance(a: [u8; 3], b: [u8; 3]) -> u8 {
    (0..3).map(|c| a[c].abs_diff(b[c])).max().unwrap_or(0)
}

fn as_bytes(c: Color) -> [u8; 3] {
    let ch = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    [ch(c.r), ch(c.g), ch(c.b)]
}

/// The indicator, measured off the drawn field: how many rows of it there are at the container's
/// bottom edge, and what colour they are.
///
/// Sampled at `x = 2`, inside the container and well clear of §7.7's 16dp padding, so nothing but
/// the container's own background and the indicator can be in the column.
///
/// **The background is read from the field rather than from the token** (BUG-002). It used to be
/// `surface_container_highest` outright, which was true only while nothing else painted the
/// container: an open select now carries the pressed state layer over its whole width, so every row
/// in the column differed from the bare token and the indicator measured the full 56dp. The layer
/// is correct and the yardstick was wrong — so the reference is now a row from the container's
/// middle, which is whatever the field actually has behind its indicator.
fn indicator(field: &mut Mounted<'_>, r: Roles) -> (f32, [u8; 3]) {
    let fill = style::color(r.surface_container_highest);
    let box_bottom = field.container().height.round() as u32;
    let (pixels, width, _) = field.screenshot(fill, style::color(r.on_surface));

    let back = pixel(&pixels, width, 2, box_bottom / 2);
    let mut rows = 0.0;
    let mut colour = back;
    for y in (0..box_bottom).rev() {
        let p = pixel(&pixels, width, 2, y);
        if distance(p, back) <= 8 {
            break;
        }
        if rows == 0.0 {
            colour = p;
        }
        rows += 1.0;
    }
    (rows, colour)
}

// ---------------------------------------------------------------------------------------------
// The trigger is the shared chrome (C1.1)
// ---------------------------------------------------------------------------------------------

/// The select wears the filled field's container, the same one a text input wears.
///
/// Compared against a `TextField` rather than against §7.7's number: a container fixed at 56dp
/// measures 56dp no matter what is nested inside it, so the figure alone would pass for a select
/// carrying a box of its own as well.
#[test]
fn the_trigger_is_the_same_container_a_text_field_wears() {
    let r = roles();
    let chosen = Mounted::new(select(None, r));
    let typed = Mounted::new(TextField::new("", "", r).label("Type"));

    assert_eq!(
        chosen.container().height,
        typed.container().height,
        "a select's container is {:.1}dp against a text field's {:.1}dp — the two are meant to be \
         one shape drawn by one wrapper (FR-002)",
        chosen.container().height,
        typed.container().height,
    );
    assert_eq!(
        chosen.bounds().width,
        WIDTH,
        "the select does not fill the width it is offered, so it will not line up with the fields \
         beside it"
    );
}

/// Empty rests the label on the value's line; chosen floats it clear (C1.1's table).
#[test]
fn the_label_rests_while_nothing_is_chosen_and_floats_once_something_is() {
    let r = roles();
    let empty = Mounted::new(select(None, r));
    let filled = Mounted::new(select(Some(OPTIONS[1]), r));

    let middle = empty.container().height / 2.0;
    let resting = empty.slots()[3];
    let floating = filled.slots()[3];

    assert!(
        (resting.center_y() - middle).abs() < 1.0,
        "an unset select's label is centred at {:.1}dp in a {:.1}dp box rather than at {middle}dp — \
         resting means sitting where the value will appear, because the resting label *is* the \
         placeholder",
        resting.center_y(),
        empty.container().height,
    );
    assert!(
        floating.center_y() < middle - 4.0 && floating.height < resting.height,
        "a chosen select's label is at {:.1}dp and {:.1}dp tall against a resting {:.1}dp and \
         {:.1}dp — it has to leave the value's line free, in the smaller role",
        floating.center_y(),
        floating.height,
        resting.center_y(),
        resting.height,
    );
}

/// Opening floats the label too, even with nothing chosen — an open list is a field in use.
#[test]
fn opening_floats_the_label_before_anything_is_chosen() {
    let r = roles();
    let mut field = Mounted::new(select(None, r));
    let resting = field.slots()[3];
    field.press_trigger();
    let floating = field.slots()[3];

    assert!(
        floating.center_y() < resting.center_y() - 4.0,
        "the label sits at {:.1}dp open against {:.1}dp closed — an open select shows its \
         placeholder on the value's line, which the resting label would be printed on top of",
        floating.center_y(),
        resting.center_y(),
    );
}

// ---------------------------------------------------------------------------------------------
// The trailing chevron (C1.2, §7.7)
// ---------------------------------------------------------------------------------------------

/// §7.7's 24dp glyph, at the trailing edge of the control's own line.
#[test]
fn the_trigger_carries_a_trailing_chevron_at_the_contracts_size() {
    let r = roles();
    let field = Mounted::new(select(Some(OPTIONS[0]), r));
    let control = field.slots()[1];

    // The chevron is the last thing in the control's row, so the deepest node whose right edge is
    // the control's right edge and whose box is square is it. Found rather than indexed: the row
    // is composed, and an index would pin the composition rather than the anatomy.
    let want = anatomy::text_field::TRAILING_ICON;
    let found = square_nodes_at_the_trailing_edge(&field.node, control);
    assert!(
        found.iter().any(|b| (b.width - want).abs() < TOLERANCE),
        "no {want}dp glyph sits at the trailing edge of the select's control line — §7.7 gives the \
         select a trailing chevron and that is the only thing distinguishing it from a text field \
         at a glance. Found {found:?}",
    );
}

/// Every node flush with `control`'s trailing edge, as bounds. Helper for the check above.
fn square_nodes_at_the_trailing_edge(node: &layout::Node, control: Rectangle) -> Vec<Rectangle> {
    fn walk(node: &layout::Node, offset: iced::Vector, out: &mut Vec<Rectangle>) {
        // A node's own bounds are relative to its parent, so the parent's *absolute* origin is the
        // offset its children carry — not the offset it was handed, which is already in `bounds`.
        let bounds = node.bounds() + offset;
        out.push(bounds);
        for child in node.children() {
            walk(child, iced::Vector::new(bounds.x, bounds.y), out);
        }
    }
    let mut all = Vec::new();
    walk(node, iced::Vector::ZERO, &mut all);
    all.into_iter()
        .filter(|b| {
            b.width > 0.0
                && (b.x + b.width - (control.x + control.width)).abs() < 1.0
                && b.y >= control.y - 1.0
        })
        .collect()
}

/// The chevron is drawn in the muted role, not the value's (§7.7).
///
/// Read off the drawn field: it is a colour, and the only way to be sure a colour reached the
/// screen is to look at what was painted.
#[test]
fn the_chevron_is_drawn_in_the_muted_role() {
    let r = roles();
    let mut field = Mounted::new(select(Some(OPTIONS[0]), r));
    let control = field.slots()[1];
    let fill = style::color(r.surface_container_highest);
    let (pixels, width, height) = field.screenshot(fill, style::color(r.on_surface));

    // The trailing 24dp of the control's line: the chevron and nothing else.
    let x0 = (control.x + control.width - anatomy::text_field::TRAILING_ICON).max(0.0) as u32;
    let x1 = ((control.x + control.width) as u32).min(width);
    let y0 = control.y as u32;
    let y1 = ((control.y + control.height) as u32).min(height);

    let variant = as_bytes(style::color(r.on_surface_variant));
    let value = as_bytes(style::color(r.on_surface));
    let mut nearest = u8::MAX;
    let mut ink = 0;
    for y in y0..y1 {
        for x in x0..x1 {
            let p = pixel(&pixels, width, x, y);
            if distance(p, as_bytes(fill)) > 24 {
                ink += 1;
                nearest = nearest.min(distance(p, variant));
            }
        }
    }

    assert!(
        ink > 0,
        "nothing was drawn in the trailing 24dp of the select's control line, so there is no \
         chevron to colour"
    );
    assert!(
        nearest < distance(value, variant),
        "the chevron's darkest ink is {nearest} from `on_surface_variant` and the value role is \
         only {} away from it — §7.7 draws the chevron muted, so it does not compete with the value \
         beside it",
        distance(value, variant),
    );
}

// ---------------------------------------------------------------------------------------------
// The state layer covers the field it responds on (BUG-002 — FR-034, SC-011)
// ---------------------------------------------------------------------------------------------

/// Hovering shades the **whole** container, not an inner part of it.
///
/// Sampled at `x = 2` and at the container's top row: both are inside the field and both are
/// outside §7.7's 16dp padding, so before this was fixed neither could ever change colour. The
/// layer was painted on the control, which `FilledField` lays into one 24dp value line inside that
/// padding — 440×24 of a 472×56 field — while hover and press were read off the whole band. This is
/// the shape of the bug, and it is why the assertion is two corners rather than an area: an
/// implementation that shades the middle and not the edges is the one being ruled out.
#[test]
fn hovering_shades_the_whole_container_not_the_control_slot() {
    let r = roles();
    let fill = style::color(r.surface_container_highest);
    let on = style::color(r.on_surface);

    let mut field = Mounted::new(select(None, r));
    let box_ = field.container();
    // Inside the padding at the leading edge, vertically clear of the indicator.
    let probe = Point::new(box_.x + 4.0, box_.y + box_.height / 2.0);

    let (resting, width, _) = field.screenshot(fill, on);
    let (hovered, _, _) = field.screenshot_under(fill, on, mouse::Cursor::Available(probe));

    let y = (box_.y + box_.height / 2.0) as u32;
    let edge_at_rest = pixel(&resting, width, 2, y);
    let edge_hovered = pixel(&hovered, width, 2, y);
    assert!(
        distance(edge_at_rest, edge_hovered) > 0,
        "the container's leading edge is unchanged by a hover the field itself registers — the \
         layer is painted somewhere smaller than the rectangle the pointer is read against \
         (FR-034, SC-011)"
    );

    let top_at_rest = pixel(&resting, width, width / 2, (box_.y + 2.0) as u32);
    let top_hovered = pixel(&hovered, width, width / 2, (box_.y + 2.0) as u32);
    assert!(
        distance(top_at_rest, top_hovered) > 0,
        "the container's top edge is unchanged by a hover — the layer is inset vertically, which \
         is the 24dp-in-56dp half of BUG-002 (FR-034)"
    );
}

/// A press in the field's padding ripples, because that press already opens the list.
///
/// The contradiction this closes is the whole of BUG-002: `Select::update` read hover and press off
/// the 472×56 band while `Ripple` watched a 440×24 node inside it, so the pointer in the 16dp
/// gutter opened the select and rippled nothing.
#[test]
fn a_press_in_the_padding_ripples_because_it_also_opens_the_list() {
    let r = roles();
    let mut field = Mounted::new(select(None, r));
    let box_ = field.container();
    let gutter = Point::new(box_.x + 4.0, box_.y + box_.height / 2.0);

    assert!(
        field.ripples_when_pressed_at(gutter),
        "a press in the field's padding started no ripple, yet the same press toggles the list — \
         the rectangle that accepts the press and the one that answers it must be the same \
         (FR-010, FR-034)"
    );
}

// ---------------------------------------------------------------------------------------------
// The active indicator answers for itself (C1.3, FR-013)
// ---------------------------------------------------------------------------------------------

/// Closed, the indicator is §7.7's hairline in the muted role.
#[test]
fn a_closed_select_rests_its_indicator() {
    let r = roles();
    let mut field = Mounted::new(select(None, r));
    let (rows, colour) = indicator(&mut field, r);

    assert!(
        (rows - anatomy::text_field::INDICATOR).abs() < TOLERANCE,
        "a resting select's indicator measures {rows}dp against §7.7's {}dp",
        anatomy::text_field::INDICATOR,
    );
    assert!(
        distance(colour, as_bytes(style::color(r.on_surface_variant))) < 24,
        "a resting select's indicator is {colour:?} rather than `on_surface_variant`",
    );
}

/// **Open, it thickens and takes the accent — with nothing supplying that.**
///
/// The whole of accepted fidelity gap #3. `pick_list` reported its open state to its own style
/// closure and to nobody else, so `Select::active` had to be supplied by a caller that tracked
/// openness and none did; the indicator sat at rest forever. A component that holds the flag has
/// nobody to ask, so this is measurable rather than accepted (FR-013).
#[test]
fn an_open_select_thickens_its_indicator_from_its_own_knowledge() {
    let r = roles();
    let mut field = Mounted::new(select(None, r));
    field.press_trigger();

    let (rows, colour) = indicator(&mut field, r);
    assert!(
        (rows - anatomy::text_field::INDICATOR_ACTIVE).abs() < TOLERANCE,
        "an open select's indicator measures {rows}dp against §7.7's {}dp — nothing told it it was \
         open, which is the point: it holds the flag itself (FR-013, accepted gap #3)",
        anatomy::text_field::INDICATOR_ACTIVE,
    );
    assert!(
        distance(colour, as_bytes(style::color(r.primary))) < 24,
        "an open select's indicator is {colour:?} rather than the accent — an indicator that \
         thickened without recolouring reads as a rendering artefact rather than as a state",
    );
}

// ---------------------------------------------------------------------------------------------
// Opening (C1.3, §2) — the list is the component's own
// ---------------------------------------------------------------------------------------------

/// Pressing the trigger floats a list; pressing it again takes it away, choice unchanged.
#[test]
fn the_trigger_toggles_its_own_list() {
    let r = roles();
    let mut field = Mounted::new(select(None, r));
    assert!(
        !field.has_list(),
        "a select opened itself before it was pressed"
    );

    let published = field.press_trigger();
    assert!(
        field.has_list(),
        "pressing the trigger did not float a list — the select owns its openness, so nothing else \
         can open it"
    );
    assert!(
        published.is_empty(),
        "opening the list published {published:?} — opening is the component's own business and \
         `app.rs` gains nothing for it (data-model §2.2)"
    );

    field.press_trigger();
    assert!(
        !field.has_list(),
        "pressing an open select's trigger left the list up; §2 closes it, choice unchanged"
    );
}

/// Escape closes it, taking nothing.
#[test]
fn escape_closes_the_list_and_takes_nothing() {
    let r = roles();
    let mut field = Mounted::new(select(Some(OPTIONS[0]), r));
    field.press_trigger();

    let published = field.key(iced::keyboard::key::Named::Escape);
    assert!(!field.has_list(), "Escape left the list open");
    assert!(
        published.is_empty(),
        "Escape published {published:?} — it closes the list without taking anything (§2)"
    );
}

// ---------------------------------------------------------------------------------------------
// T014: the highlight is seeded from the current choice (feature 013's FR-003)
// ---------------------------------------------------------------------------------------------

/// Opening a select that already has a value puts the keyboard on that value.
///
/// Asserted through what Enter takes rather than by reading the flag: "the list opens with the
/// current value marked and **reachable**" is the requirement, and a seeded highlight the list
/// could not act on would satisfy a flag check and fail the user. `pick_list` gave this for free —
/// it seeded its own hovered option from the current value — and it must not leave with it.
#[test]
fn opening_seeds_the_highlight_from_the_current_choice() {
    let r = roles();
    let mut field = Mounted::new(select(Some(OPTIONS[2]), r));
    field.press_trigger();

    let published = field.key(iced::keyboard::key::Named::Enter);
    assert_eq!(
        published,
        vec![OPTIONS[2].to_string()],
        "Enter on a freshly opened select took {published:?} rather than the option it already \
         holds — the highlight was not seeded from the current choice (feature 013's FR-003)"
    );
    assert!(!field.has_list(), "taking a row left the list open (§2)");
}

/// …and one arrow press from there moves to the next row, not back to the top of the list.
///
/// The half that separates a genuinely seeded highlight from one that merely *looks* seeded: a
/// list that opened at row 0 while marking row 2 would pass the check above only if Enter read the
/// marking rather than the highlight, and would fail this.
#[test]
fn the_seeded_highlight_is_where_the_keyboard_carries_on_from() {
    let r = roles();
    let mut field = Mounted::new(select(Some(OPTIONS[0]), r));
    field.press_trigger();
    field.key(iced::keyboard::key::Named::ArrowDown);

    let published = field.key(iced::keyboard::key::Named::Enter);
    assert_eq!(
        published,
        vec![OPTIONS[1].to_string()],
        "Down from a select holding the first option took {published:?} — the keyboard has to carry \
         on from where the value is, not from wherever an unseeded list starts"
    );
}

/// With nothing chosen there is nothing to seed, so Enter takes nothing until an arrow lands
/// somewhere.
#[test]
fn an_unset_select_opens_with_the_keyboard_on_nothing() {
    let r = roles();
    let mut field = Mounted::new(select(None, r));
    field.press_trigger();

    let published = field.key(iced::keyboard::key::Named::Enter);
    assert!(
        published.is_empty(),
        "Enter took {published:?} from a list whose keyboard is on nothing — `intent_for` refuses \
         a pick without a highlight, and the select must answer the same keys the same way (FR-024)"
    );
    assert!(
        field.has_list(),
        "an Enter that took nothing still closed the list"
    );

    field.key(iced::keyboard::key::Named::ArrowDown);
    assert_eq!(
        field.key(iced::keyboard::key::Named::Enter),
        vec![OPTIONS[0].to_string()],
        "Down into an unseeded list has to enter it at the first row, as `move_highlight` says"
    );
}

/// Tab closes the list **and** passes through, so focus still moves — the one key where dismissing
/// and claiming come apart (`intent_for`'s own rule, FR-024).
#[test]
fn tab_closes_the_list_without_claiming_the_key() {
    let r = roles();
    let mut field = Mounted::new(select(Some(OPTIONS[1]), r));
    field.press_trigger();

    let mut messages = Vec::new();
    let mut shell = iced::advanced::Shell::new(&mut messages);
    field.element.as_widget_mut().update(
        &mut field.tree,
        &Event::Keyboard(iced::keyboard::Event::KeyPressed {
            key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Tab),
            modified_key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Tab),
            physical_key: iced::keyboard::key::Physical::Unidentified(
                iced::keyboard::key::NativeCode::Unidentified,
            ),
            location: iced::keyboard::Location::Standard,
            modifiers: iced::keyboard::Modifiers::empty(),
            text: None,
            repeat: false,
        }),
        Layout::new(&field.node),
        mouse::Cursor::Unavailable,
        &field.renderer,
        &mut clipboard::Null,
        &mut shell,
        &Rectangle::with_size(WINDOW),
    );
    let claimed = shell.is_event_captured();
    field.relayout();
    field.settle();

    assert!(!field.has_list(), "Tab left the list open");
    assert!(
        !claimed,
        "the select swallowed Tab, so a developer cannot tab past it — the key dismisses the list \
         and still belongs to whatever has focus"
    );
}

/// A press away from both the trigger and the list closes it.
#[test]
fn a_press_outside_closes_the_list() {
    let r = roles();
    let mut field = Mounted::new(select(None, r));
    field.press_trigger();
    assert!(field.has_list());

    let away = Point::new(
        field.bounds().x + 4.0,
        field.bounds().y + field.bounds().height + 400.0,
    );
    let published = field.dispatch(
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        mouse::Cursor::Available(away),
    );
    assert!(!field.has_list(), "a press outside left the list open (§2)");
    assert!(published.is_empty(), "dismissing published {published:?}");
}
