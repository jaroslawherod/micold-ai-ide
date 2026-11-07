//! The three composer selectors share one trigger (feature 1703 T026 — FR-015, FR-016, SC-004).
//!
//! Model, effort and permission are all a [`MenuSelect`]. Their geometry is measured off the laid
//! out trigger, so a selector that grew its own box would show; their state treatments are read
//! from the trigger's style in each of the five states, which is where hover, pressed, open and
//! disabled are decided. In-crate because `material` is `pub(crate)`.

use iced::widget::button;
use iced::{Background, Element, Length, Size, Theme};

use super::test_support::{bounds_at, roles};
use super::{style, MenuSelect, MenuSelectRow};
use crate::showcase::state::Message;
use micold_core::tokens::{shape, state};

const ROOM: Size = Size::new(500.0, 400.0);
const TOLERANCE: f32 = 0.5;

/// A closed selector reading `label`, `width` wide, optionally disabled.
fn selector(label: &str, width: impl Into<Length>, disabled: bool) -> Element<'static, Message> {
    let r = roles();
    let menu = MenuSelect::new(
        label.to_string(),
        vec![MenuSelectRow::note("nothing")],
        false,
        Message::NoOp,
        Message::NoOp,
        r,
    )
    .title(Some("A value long enough to be cut by a narrow trigger"))
    .width(width);
    if disabled {
        menu.disabled("Why it is off").into()
    } else {
        menu.into()
    }
}

/// The trigger button's box: the first node under the root.
fn trigger(label: &str, width: impl Into<Length>, disabled: bool) -> iced::Rectangle {
    bounds_at(selector(label, width, disabled), ROOM, &[])
}

fn status_style(open: bool, status: button::Status) -> button::Style {
    style::selector_trigger(roles(), open)(&Theme::Dark, status)
}

#[test]
fn the_three_selectors_are_32dp_high_with_equal_geometry() {
    let boxes =
        ["Model:", "Effort:", "Permissions:"].map(|l| trigger(l, Length::Fixed(200.0), false));
    for b in boxes {
        assert!(
            (b.height - 32.0).abs() < TOLERANCE,
            "a selector trigger is {:.1}dp high, not 32 (FR-015)",
            b.height
        );
        assert!((b.width - 200.0).abs() < TOLERANCE, "width {:.1}", b.width);
    }
    assert_eq!(boxes[0].height, boxes[1].height);
    assert_eq!(boxes[1].height, boxes[2].height);
}

#[test]
fn a_fixed_width_below_140dp_is_raised_to_140dp() {
    let b = trigger("Model:", Length::Fixed(60.0), false);
    assert!(
        (b.width - 140.0).abs() < TOLERANCE,
        "a 60dp selector is {:.1}dp, not the 140dp minimum (FR-015)",
        b.width
    );
}

#[test]
fn a_disabled_selector_keeps_its_geometry() {
    let on = trigger("Model:", Length::Fixed(200.0), false);
    let off = trigger("Model:", Length::Fixed(200.0), true);
    assert_eq!((on.width, on.height), (off.width, off.height));
}

#[test]
fn every_state_has_the_8dp_radius_and_an_outline_variant_border_except_open() {
    let r = roles();
    for status in [
        button::Status::Active,
        button::Status::Hovered,
        button::Status::Pressed,
        button::Status::Disabled,
    ] {
        let s = status_style(false, status);
        assert_eq!(s.border.radius, shape::SMALL.into(), "{status:?} radius");
        assert_eq!(s.border.width, 1.0, "{status:?} border width");
        if status != button::Status::Disabled {
            assert_eq!(
                s.border.color,
                style::color(r.outline_variant),
                "{status:?}"
            );
        }
    }
    let open = status_style(true, button::Status::Active);
    assert_eq!(open.border.width, 2.0, "open: a 2dp border");
    assert_eq!(open.border.color, style::color(r.primary), "open: primary");
    assert_eq!(open.border.radius, shape::SMALL.into());
}

#[test]
fn hover_and_pressed_layer_the_content_colour_and_disabled_dims_it() {
    let layer = |s: button::Style| match s.background {
        Some(Background::Color(c)) => c.a,
        other => panic!("expected a colour layer, got {other:?}"),
    };
    assert!(status_style(false, button::Status::Active)
        .background
        .is_none());
    assert!((layer(status_style(false, button::Status::Hovered)) - state::HOVER).abs() < 0.01);
    assert!((layer(status_style(false, button::Status::Pressed)) - state::PRESSED).abs() < 0.01);
    let off = status_style(false, button::Status::Disabled);
    assert!((layer(off) - state::DISABLED_CONTAINER).abs() < 0.01);
    assert!((off.text_color.a - state::DISABLED_CONTENT).abs() < 0.01);
    assert_eq!(
        status_style(false, button::Status::Active).text_color,
        style::color(roles().on_surface)
    );
}
