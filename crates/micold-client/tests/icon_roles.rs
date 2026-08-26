//! US2: icon tint correctness (FR-004, FR-007, SC-004). Pure — no iced, runs under
//! `cargo test --no-default-features`.
//!
//! `icon_role` selects the foreground tint for each surface. Asserting the chosen color has
//! sufficient contrast against the *actual background the icon sits on* guarantees an icon is
//! never tinted invisibly, in BOTH schemes — proving the SC-004 legibility outcome directly
//! (including the `error`-tinted unavailable marker, which the text-only pairs don't cover).

use micold_client::icons::{icon_role, IconSurface};
use micold_core::theme::ColorScheme;
// `contrast` from the crate that owns the colours, not from a copy here — see the note in
// `micold-core/tests/tokens_contrast.rs`.
use micold_core::tokens::{contrast, roles, AA_NON_TEXT};

/// WCAG 1.4.11 minimum contrast for non-text graphics (icons are >= 18px graphical marks).
const ICON_MIN: f64 = AA_NON_TEXT;

#[test]
fn every_icon_tint_is_legible_on_its_surface() {
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let r = roles(scheme);
        // The background each surface's icon is actually drawn on.
        let cases = [
            (IconSurface::AppBarAction, r.surface),
            (IconSurface::PrimaryButton, r.primary),
            (IconSurface::Badge, r.surface_variant),
            (IconSurface::Unavailable, r.surface),
            // A menu item's glyph sits on the menu panel, which is `surface_container` (§7.5).
            (IconSurface::MenuItem, r.surface_container),
        ];
        for (surface, background) in cases {
            let ratio = contrast(icon_role(surface, r), background);
            assert!(
                ratio >= ICON_MIN,
                "{scheme:?}: {surface:?} icon contrast {ratio:.2} < {ICON_MIN} on its background"
            );
        }
    }
}

#[test]
fn icon_role_maps_each_surface_to_its_expected_foreground() {
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let r = roles(scheme);
        assert_eq!(icon_role(IconSurface::AppBarAction, r), r.on_surface);
        assert_eq!(icon_role(IconSurface::PrimaryButton, r), r.on_primary);
        assert_eq!(icon_role(IconSurface::Badge, r), r.on_surface_variant);
        assert_eq!(icon_role(IconSurface::Unavailable, r), r.error);
        assert_eq!(icon_role(IconSurface::MenuItem, r), r.on_surface_variant);
    }
}

#[test]
fn all_surfaces_are_covered() {
    assert_eq!(IconSurface::ALL.len(), 5, "surface contexts");
}

/// BUG-001 (feature 012 FR-011a, SC-007): a glyph nested inside a **filled** container must take
/// that container's foreground, not the bar's.
///
/// `IconButton::new` defaults its tint to the roles' `on_surface` — correct on a surface, and the
/// wrong half of a pair the moment the button is nested inside something that paints its own fill.
/// The terminal's instance-switcher put a close `IconButton` inside a `ButtonVariant::Filled` tab,
/// which `style::filled` paints `primary`/`on_primary`, and the glyph kept `on_surface`. The tab's
/// *label* was fine — plain `Text` inherits the button's `text_color` — so only the icon opted out,
/// and the close control all but vanished on the one tab a user is most likely to want to close.
///
/// The second assertion is the one that makes this a standing gate rather than a fixed bug: it
/// pins the *wrong* pairing as measurably wrong, so re-introducing the default tint inside a filled
/// container fails here instead of needing an eye at a display. If a future palette ever makes
/// `on_surface` legible on `primary`, this assertion inverts — revisit it then rather than deleting
/// it, because the roles would still be wrong even where the contrast happened to survive.
#[test]
fn a_glyph_in_a_filled_container_takes_the_containers_foreground() {
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let r = roles(scheme);
        let right = contrast(icon_role(IconSurface::PrimaryButton, r), r.primary);
        assert!(
            right >= ICON_MIN,
            "{scheme:?}: a nested glyph tinted `on_primary` must be legible on the `primary` fill \
             its container paints — contrast {right:.2} < {ICON_MIN}"
        );

        let default_tint = icon_role(IconSurface::AppBarAction, r);
        assert_eq!(default_tint, r.on_surface, "IconButton's default tint");
        let wrong = contrast(default_tint, r.primary);
        assert!(
            wrong < ICON_MIN,
            "{scheme:?}: `on_surface` on a `primary` fill reads {wrong:.2} — at or above \
             {ICON_MIN}, so this assertion no longer describes the BUG-001 defect. The pairing is \
             still wrong (a nested control must take its container's foreground, FR-011a); revisit \
             this gate rather than deleting it"
        );
    }
}
