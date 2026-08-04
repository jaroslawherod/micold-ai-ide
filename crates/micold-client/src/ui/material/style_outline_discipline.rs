//! Depth comes from tone and shadow, never from a border (feature 018, T002 — FR-002, FR-003).
//!
//! Feature 003 drew a 1px outline around cards, dialogs and menus to suggest an edge. Material does
//! not: a container is defined by its surface tone and its elevation, and an outline is reserved for
//! three specific jobs — a divider, an outlined control's border, and a focus ring (contract §1.5).
//!
//! The failure this prevents is a *combination*, which is what makes it easy to miss in review: a
//! surface that gains a shadow but keeps its old border reads as both, and looks like a sticker
//! rather than a raised plane. Each half looks reasonable on its own line of the diff.
//!
//! Inside the crate for the reason `style_snapshot` states — the style layer is `pub(crate)` by
//! design (017 FR-002), so `tests/` cannot reach it.

use super::style;
use iced::widget::container;
use iced::Theme;
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, Roles};

/// A boxed container style function; see `style_elevation`'s alias for why.
type StyleFn = Box<dyn Fn(&Theme) -> container::Style>;

/// Every container style that carries an elevation, and therefore owes no border.
fn elevated(r: Roles) -> Vec<(&'static str, StyleFn)> {
    vec![
        ("surface", Box::new(style::surface(r)) as StyleFn),
        ("dialog", Box::new(style::dialog(r))),
        ("menu_surface", Box::new(style::menu_surface(r))),
        ("sidebar_surface", Box::new(style::sidebar_surface(r))),
        ("toolbar_surface", Box::new(style::toolbar_surface(r))),
    ]
}

#[test]
fn no_elevated_surface_also_draws_a_border() {
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let r = tokens::roles(scheme);
        let theme = style::theme(scheme);
        for (name, style_fn) in elevated(r) {
            let s = style_fn(&theme);
            let visible = s.border.width > 0.0 && s.border.color.a > 0.0;
            assert!(
                !visible,
                "{scheme:?} {name} draws a {}px border at alpha {} as well as carrying an \
                 elevation. Material defines a container by tone and elevation; an outline is only \
                 a divider, an outlined control's border, or a focus ring (contract §1.5). Drawn \
                 together they read as a sticker rather than a raised plane.",
                s.border.width, s.border.color.a
            );
        }
    }
}

/// The scan has to actually cover something. If the surface list is ever emptied, this fails rather
/// than reporting discipline over nothing.
#[test]
fn the_scan_covers_every_elevated_surface() {
    let r = tokens::roles(ColorScheme::Light);
    assert!(
        elevated(r).len() >= 5,
        "only {} surfaces checked — contract §4 assigns a level to more than that",
        elevated(r).len()
    );
}
