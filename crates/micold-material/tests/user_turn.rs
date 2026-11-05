//! `UserTurn` (#1663 FR-001, FR-002, FR-006): the tinted surface and the accent edge a user's
//! message is drawn on, from theme tokens only.

use iced::widget::text;
use iced::{Background, Element};
use micold_core::theme::ColorScheme;
use micold_core::tokens::{roles, shape};
use micold_material::style::{self, color};
use micold_material::UserTurn;

const SCHEMES: [ColorScheme; 2] = [ColorScheme::Light, ColorScheme::Dark];

#[test]
fn the_surface_is_surface_container_high_without_a_shadow_and_with_the_small_corner() {
    for scheme in SCHEMES {
        let r = roles(scheme);
        let s = style::user_turn_surface(r)(&iced::Theme::Light);
        assert_eq!(
            s.background,
            Some(Background::Color(color(r.surface_container_high))),
            "{scheme:?}"
        );
        assert_eq!(s.shadow, iced::Shadow::default(), "{scheme:?}: no shadow");
        assert_eq!(s.border.radius, shape::SMALL.into(), "{scheme:?}: corner");
        assert_eq!(s.border.width, 0.0, "{scheme:?}: no outline");
    }
}

#[test]
fn the_edge_is_primary() {
    for scheme in SCHEMES {
        let r = roles(scheme);
        let s = style::user_turn_edge(r)(&iced::Theme::Light);
        assert_eq!(
            s.background,
            Some(Background::Color(color(r.primary))),
            "{scheme:?}"
        );
    }
}

#[test]
fn a_user_turn_converts_to_an_element_in_both_schemes_even_when_empty() {
    for scheme in SCHEMES {
        let r = roles(scheme);
        let _: Element<'_, ()> = UserTurn::new(text("hello"), r).into();
        let _: Element<'_, ()> = UserTurn::new(text(""), r).into();
    }
}
