//! `UserTurn` (#1663 FR-001, FR-002, FR-006): the tinted surface and the accent edge a user's
//! message is drawn on, from theme tokens only.

use iced::widget::text;
#[cfg(feature = "test-support")]
use iced::Size;
use iced::{Background, Element};
use micold_core::theme::ColorScheme;
use micold_core::tokens::{roles, shape, spacing};
use micold_material::style::{self, color};
#[cfg(feature = "test-support")]
use micold_material::test_support::Mounted;
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
fn the_edge_rounds_only_its_left_corners_like_the_surface() {
    for scheme in SCHEMES {
        let s = style::user_turn_edge(roles(scheme))(&iced::Theme::Light);
        let corner = shape::SMALL;
        assert_eq!(
            s.border.radius,
            iced::border::Radius {
                top_left: corner,
                bottom_left: corner,
                top_right: 0.0,
                bottom_right: 0.0,
            },
            "{scheme:?}"
        );
    }
}

/// The element draws the 4dp edge down the left and sets the content in from it. Needs the
/// headless renderer: `--features test-support`.
#[cfg(feature = "test-support")]
#[test]
fn a_user_turn_draws_a_4dp_edge_and_offsets_its_content() {
    const EDGE_DP: f32 = 4.0;
    const ROOM: Size = Size::new(200.0, 100.0);
    for scheme in SCHEMES {
        let r = roles(scheme);
        let element: Element<'_, String> = UserTurn::new(text("hello").size(10), r).into();
        let (buf, width) = Mounted::new_in(element, ROOM)
            .in_scheme(scheme)
            .drawn(iced::Color::TRANSPARENT);
        let width = width as usize;
        let at = |x: usize, y: usize| &buf[(y * width + x) * 4..][..3];
        let edge = [r.primary.r, r.primary.g, r.primary.b];
        let fill = [
            r.surface_container_high.r,
            r.surface_container_high.g,
            r.surface_container_high.b,
        ];
        let height = buf.len() / 4 / width;
        let corner = shape::SMALL as usize;
        let y = corner + 2; // clear of the rounded corners
        let edge_px = (0..width).take_while(|&x| at(x, y) == edge).count();
        assert_eq!(edge_px, EDGE_DP as usize, "{scheme:?}: edge width");
        assert_eq!(at(edge_px, y), fill, "{scheme:?}: surface follows the edge");
        let ink = |x: usize| (y..height - corner).any(|y| at(x, y) != fill && at(x, y) != edge);
        let first_ink = (edge_px..width).find(|&x| ink(x)).expect("text drawn");
        assert!(
            first_ink >= edge_px + spacing::SM as usize,
            "{scheme:?}: content starts {first_ink}, past the edge and padding"
        );
    }
}
