//! A component lays out at the size its anatomy entry states, whatever room it is offered
//! (feature 018 — FR-027, FR-030, BUG-002).
//!
//! # The gap this closes
//!
//! Four checks already read something about a component's box, and none of them reads its **size**:
//!
//! - `micold-core/tests/tokens_anatomy.rs` compares the *constants* against the contract. It was
//!   green throughout BUG-002: `button::MIN_TOUCH_TARGET` is 48.0 and always was — the defect was
//!   that nothing used it.
//! - `tests/layout_snapshot.rs` byte-compares a fixture. It **recorded** BUG-002 as the expected
//!   value (a 499.9 × 64.0 icon button in the app bar) and was green on it from the day it landed.
//!   A snapshot catches change, not defect; a defect older than the fixture becomes the baseline.
//! - `tests/gates/containment.rs` asks whether a child escapes its parent. A component that grows
//!   to fill its parent is contained by definition.
//! - `content_placement.rs` asks where a component put its *content* inside its box. BUG-002's
//!   glyph was perfectly centred — in the middle of a box twenty-five times too wide.
//!
//! So this asks the remaining question: is the box the size the contract says?
//!
//! # How, and why twice
//!
//! Each component is laid out under two differently-sized limits. That is what separates the three
//! outcomes an axis can have, which no single measurement can:
//!
//! - **`Fixed`** — the same number under both, equal to the anatomy constant. §7.3's 48dp target,
//!   §7.6's 32dp chip, §7.1's 64dp bar.
//! - **`Fill`** — tracks the limit. Intended for the app bar's width; a defect anywhere a contract
//!   states a number.
//! - **`Content`** — the same number under both, and smaller than the room offered. A chip's width,
//!   a compact icon button.
//!
//! One measurement cannot tell `Fixed(48)` from `Fill` (offer exactly 48 and they agree) nor `Fill`
//! from `Content` (offer a tight box and they agree). Two, at different sizes, tell all three apart.
//!
//! # In-crate, like its neighbours
//!
//! `material` is `pub(crate)`, so an `IconButton` cannot be constructed from `tests/` at all — the
//! same reason `content_placement`, `form_field_anatomy` and `text_field_anatomy` live here.

use iced::advanced::layout;
use iced::advanced::widget::Tree;
use iced::{Element, Length, Size};
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, anatomy, Roles};

use super::{IconButton, ToggleChip, Toolbar};
use crate::icons::Icon;
use crate::showcase::state::Message;

/// The two boxes every component is offered. Both are far larger than any component here, and
/// differ on both axes, so a `Fill` axis reports two different numbers and a sized one reports the
/// same number twice.
const ROOMY: Size = Size::new(1200.0, 900.0);
const SNUG: Size = Size::new(400.0, 300.0);

/// Layout arithmetic accumulates over a nested tree; this is well below anything a person could
/// see and far below the defect being separated from correct (which is hundreds of pixels).
const TOLERANCE: f32 = 0.5;

fn roles() -> Roles {
    tokens::roles(ColorScheme::Light)
}

/// What an axis of a component is allowed to do.
#[derive(Clone, Copy)]
enum Extent {
    /// Exactly this many dp, whatever room is offered. The anatomy constant, never a literal.
    Fixed(f32),
    /// As much room as it is given — correct only where the contract asks for it.
    Fill,
    /// Sized by what it holds: the same under both limits, and smaller than either.
    Content,
}

/// The size `build` resolves to inside `bounds`.
fn laid_out(element: Element<'_, Message>, bounds: Size) -> Size {
    let mut element = element;
    let renderer = super::test_support::renderer();
    let mut tree = Tree::new(element.as_widget());
    let limits = layout::Limits::new(Size::ZERO, bounds);
    element
        .as_widget_mut()
        .layout(&mut tree, &renderer, &limits)
        .bounds()
        .size()
}

/// Why `measured` is wrong for an axis declared `expected`, or `None` if it is right.
///
/// Returned rather than asserted so [`the_gate_can_fail`] can watch it fire against a deliberately
/// broken control. A check that cannot be shown to fail is decoration.
fn violation(
    component: &str,
    axis: &str,
    expected: Extent,
    measured: (f32, f32),
    offered: (f32, f32),
) -> Option<String> {
    let (roomy, snug) = measured;
    let (roomy_limit, snug_limit) = offered;
    let seen = format!(
        "{component}'s {axis} measured {roomy}dp in a {roomy_limit}dp box and {snug}dp in a \
         {snug_limit}dp one"
    );
    match expected {
        Extent::Fixed(dp) => ((roomy - dp).abs() > TOLERANCE || (snug - dp).abs() > TOLERANCE)
            .then(|| {
                let filling = (roomy - roomy_limit).abs() < TOLERANCE
                    && (snug - snug_limit).abs() < TOLERANCE;
                let diagnosis = if filling {
                    " — it is taking whatever room it is offered, so the stated size is not being \
                     applied at all. That is BUG-002's shape: `Container::center_x`/`center_y` set \
                     the length as well as aligning, so a `.width(Fixed(n)).center_x(Fill)` chain \
                     silently discards `n`."
                } else {
                    "."
                };
                format!("{seen}, but its anatomy entry states {dp}dp{diagnosis}")
            }),
        Extent::Fill => ((roomy - roomy_limit).abs() > TOLERANCE
            || (snug - snug_limit).abs() > TOLERANCE)
            .then(|| {
                format!(
                    "{seen}, but it is supposed to span the room it is given ({roomy_limit}dp and \
                     {snug_limit}dp respectively)."
                )
            }),
        Extent::Content => ((roomy - snug).abs() > TOLERANCE || roomy >= snug_limit).then(|| {
            format!(
                "{seen}, but it is supposed to be sized by what it holds — the same under both, \
                 and smaller than either box."
            )
        }),
    }
}

/// Lay `build` out under both limits and assert each axis does what it is declared to do.
fn assert_anatomy_size(
    component: &str,
    build: impl Fn() -> Element<'static, Message>,
    width: Extent,
    height: Extent,
) {
    let roomy = laid_out(build(), ROOMY);
    let snug = laid_out(build(), SNUG);

    let complaints: Vec<String> = [
        violation(
            component,
            "width",
            width,
            (roomy.width, snug.width),
            (ROOMY.width, SNUG.width),
        ),
        violation(
            component,
            "height",
            height,
            (roomy.height, snug.height),
            (ROOMY.height, SNUG.height),
        ),
    ]
    .into_iter()
    .flatten()
    .collect();

    assert!(complaints.is_empty(), "{}", complaints.join("\n"));
}

/// §7.3: an icon button presents a 48 × 48 interactive target — a *fixed* one, not "the rest of the
/// row". This is BUG-002 itself, and the reason every other component here is checked too.
#[test]
fn an_icon_buttons_touch_target_is_the_48dp_the_contract_states() {
    assert_anatomy_size(
        "an icon button",
        || {
            IconButton::new(Icon::Menu, roles())
                .on_press(Message::NoOp)
                .into()
        },
        Extent::Fixed(anatomy::button::MIN_TOUCH_TARGET),
        Extent::Fixed(anatomy::button::MIN_TOUCH_TARGET),
    );
}

/// A disabled icon button takes no target container at all (there is nothing to hit), so it is
/// content-sized — but it must still not grow into the room it is offered.
#[test]
fn a_disabled_icon_button_is_sized_by_its_glyph() {
    assert_anatomy_size(
        "a disabled icon button",
        || IconButton::<Message>::new(Icon::Menu, roles()).into(),
        Extent::Content,
        Extent::Content,
    );
}

/// FR-045's recorded deviation: inside the sidebar the target stays at the glyph's own size. That
/// is a *smaller* box, not an elastic one — this is what would notice `compact()` acquiring the
/// same defect the full-size path had.
#[test]
fn a_compact_icon_button_is_sized_by_its_glyph_not_by_the_room_it_is_given() {
    assert_anatomy_size(
        "a compact icon button",
        || {
            IconButton::new(Icon::Menu, roles())
                .compact()
                .on_press(Message::NoOp)
                .into()
        },
        Extent::Content,
        Extent::Content,
    );
}

/// §7.6: a chip is 32dp tall and as wide as its label plus its 12dp ends.
#[test]
fn a_chip_is_32dp_tall_and_sized_by_its_label() {
    assert_anatomy_size(
        "a chip",
        || ToggleChip::new("feat", Message::NoOp, roles()).into(),
        Extent::Content,
        Extent::Fixed(anatomy::chip::HEIGHT),
    );
}

/// §7.1: the small app bar is a fixed 64dp that spans its window. The one component here with a
/// deliberately elastic axis — which is the point of declaring the axes separately rather than
/// asserting "nothing fills".
///
/// Raised, because at rest the `Toolbar` element is the bar *and* its 1dp separator, and the 64dp
/// under test is the bar's.
#[test]
fn an_app_bar_is_64dp_tall_and_spans_its_window() {
    assert_anatomy_size(
        "the app bar",
        || {
            Toolbar::<Message>::new("Micold AI IDE", roles())
                .elevated(true)
                .into()
        },
        Extent::Fill,
        Extent::Fixed(anatomy::app_bar::HEIGHT),
    );
}

/// The gate can fail, shown against the exact chain BUG-002 was: a container that states 48dp and
/// then hands both axes to `center_x`/`center_y`, which set the length as well as aligning.
///
/// Built here rather than by reverting the component, so it holds after the fix and would still
/// hold if `IconButton` were rewritten around a different primitive.
#[test]
fn the_gate_can_fail() {
    let sabotage = || -> Element<'static, Message> {
        iced::widget::container(iced::widget::Space::new())
            .width(Length::Fixed(anatomy::button::MIN_TOUCH_TARGET))
            .height(Length::Fixed(anatomy::button::MIN_TOUCH_TARGET))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    };

    let roomy = laid_out(sabotage(), ROOMY);
    let snug = laid_out(sabotage(), SNUG);
    let complaint = violation(
        "the sabotage",
        "width",
        Extent::Fixed(anatomy::button::MIN_TOUCH_TARGET),
        (roomy.width, snug.width),
        (ROOMY.width, SNUG.width),
    )
    .expect(
        "a container whose stated 48dp is overwritten by `center_x(Fill)` was reported as \
         correctly sized — this gate cannot see the defect it exists for",
    );

    assert!(
        complaint.contains("taking whatever room it is offered"),
        "the failure names the size but not the cause, so it does not tell the next reader what to \
         look for: {complaint}"
    );
}
