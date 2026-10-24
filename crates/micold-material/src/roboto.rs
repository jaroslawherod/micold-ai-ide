//! The Roboto faces the application ships, upright and italic, and how they are registered
//! (feature 018 FR-008, BUG-1298, feature 1259 contract `fonts.md` C-1 and C-3).
//!
//! Every interface role draws in [`ROBOTO`]. Three upright weights ship (400 and 500 for the
//! Material 3 type scale, 600 for the Overview card's semibold words), and each has an italic twin
//! so markdown emphasis draws in Roboto's own italic at the weight of the text around it: a role or
//! a bold run (`Weight::Bold`, which resolves to 600) that gains emphasis keeps its weight.
//! [`install_roboto`] registers all six; see `assets/fonts/PROVENANCE.md` for where they come from.

use std::borrow::Cow;

use iced::advanced::graphics::text::FontSystem;
use iced::Font;

/// The family every shipped face belongs to (asserted by `tests/roboto_font.rs`). The weighted
/// faces advertise e.g. `Roboto Medium` in their legacy family name and `Roboto` as their
/// typographic family, so all resolve here and are told apart by weight and style.
pub const ROBOTO_FAMILY: &str = "Roboto";

/// The interface font: [`ROBOTO_FAMILY`] at the weight and style a role or span asks for.
pub const ROBOTO: Font = Font::with_name(ROBOTO_FAMILY);

/// Roboto Regular (400): display, headline and body roles.
pub const ROBOTO_REGULAR_BYTES: &[u8] = include_bytes!("../../../assets/fonts/Roboto-Regular.ttf");
/// Roboto Medium (500): title medium/small and every label role.
pub const ROBOTO_MEDIUM_BYTES: &[u8] = include_bytes!("../../../assets/fonts/Roboto-Medium.ttf");
/// Roboto at weight 600, for the Overview card's semibold words (BUG-1298 FR-022, FR-024–FR-026)
/// and bold runs, which ask for 700 and resolve to the heaviest face shipped.
pub const ROBOTO_SEMIBOLD_BYTES: &[u8] =
    include_bytes!("../../../assets/fonts/Roboto-SemiBold.ttf");
/// Roboto Italic (400): emphasis in body text (feature 1259).
pub const ROBOTO_ITALIC_BYTES: &[u8] = include_bytes!("../../../assets/fonts/Roboto-Italic.ttf");
/// Roboto Medium Italic (500): emphasis in title and label roles.
pub const ROBOTO_MEDIUM_ITALIC_BYTES: &[u8] =
    include_bytes!("../../../assets/fonts/Roboto-MediumItalic.ttf");
/// Roboto SemiBold Italic (600): emphasis in bold runs, headings and bold header cells, so bold
/// italic is as heavy as bold (FR-003).
pub const ROBOTO_SEMIBOLD_ITALIC_BYTES: &[u8] =
    include_bytes!("../../../assets/fonts/Roboto-SemiBoldItalic.ttf");

/// Every shipped Roboto face, upright then italic.
pub const ROBOTO_FACES: [&[u8]; 6] = [
    ROBOTO_REGULAR_BYTES,
    ROBOTO_MEDIUM_BYTES,
    ROBOTO_SEMIBOLD_BYTES,
    ROBOTO_ITALIC_BYTES,
    ROBOTO_MEDIUM_ITALIC_BYTES,
    ROBOTO_SEMIBOLD_ITALIC_BYTES,
];

/// The weights a Roboto face ships at, each upright and italic.
pub const ROBOTO_WEIGHTS: [u16; 3] = [400, 500, 600];

/// Register every shipped Roboto face on `fonts`.
///
/// Called once, before any text is shaped (cosmic-text caches font matches): by the application and
/// the showcase at startup, and by the test renderers. A face whose bytes do not parse is logged and
/// skipped by `load_font`; `tests/roboto_font.rs` fails on such a file, so it cannot ship.
pub fn install_roboto(fonts: &mut FontSystem) {
    for bytes in ROBOTO_FACES {
        fonts.load_font(Cow::Borrowed(bytes));
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{is_bundled, resolves_to_bundled};
    use super::*;
    use fontdb::{Family, Query, Source, Style, Weight};
    use iced::advanced::graphics::text::cosmic_text::fontdb;
    use std::sync::Arc;

    /// The six bundled faces, loaded from memory as `install_roboto` loads them.
    fn bundled() -> fontdb::Database {
        let mut db = fontdb::Database::new();
        for bytes in ROBOTO_FACES {
            db.load_font_source(Source::Binary(Arc::new(bytes)));
        }
        db
    }

    fn roboto(db: &fontdb::Database, weight: u16, style: Style) -> fontdb::ID {
        db.query(&Query {
            families: &[Family::Name(ROBOTO_FAMILY)],
            weight: Weight(weight),
            style,
            ..Query::default()
        })
        .unwrap_or_else(|| panic!("Roboto resolves to no face (weight {weight}, {style:?})"))
    }

    const UPRIGHT: [(u16, &[u8], &str); 3] = [
        (400, ROBOTO_REGULAR_BYTES, "Roboto-Regular"),
        (500, ROBOTO_MEDIUM_BYTES, "Roboto-Medium"),
        (600, ROBOTO_SEMIBOLD_BYTES, "Roboto-SemiBold"),
    ];
    const ITALIC: [(u16, &[u8], &str); 3] = [
        (400, ROBOTO_ITALIC_BYTES, "Roboto-Italic"),
        (500, ROBOTO_MEDIUM_ITALIC_BYTES, "Roboto-MediumItalic"),
        (600, ROBOTO_SEMIBOLD_ITALIC_BYTES, "Roboto-SemiBoldItalic"),
    ];

    /// C-1 on a bare database: each shipped weight resolves to its own face, upright and italic.
    #[test]
    fn each_shipped_weight_resolves_to_its_bundled_face() {
        let db = bundled();
        for (weight, bytes, what) in UPRIGHT {
            assert!(
                is_bundled(&db, roboto(&db, weight, Style::Normal), bytes),
                "upright Roboto weight {weight} is not the bundled {what}"
            );
        }
        for (weight, bytes, what) in ITALIC {
            assert!(
                is_bundled(&db, roboto(&db, weight, Style::Italic), bytes),
                "italic Roboto weight {weight} is not the bundled {what}"
            );
        }
    }

    /// SC-002: bold (700, which no face ships) resolves to a 600 face upright and italic alike, so
    /// bold italic is exactly as heavy as bold.
    #[test]
    fn bold_and_bold_italic_resolve_to_faces_of_the_same_weight() {
        let db = bundled();
        let weight_of = |id| db.face(id).expect("resolved face exists").weight.0;
        let upright = roboto(&db, Weight::BOLD.0, Style::Normal);
        let italic = roboto(&db, Weight::BOLD.0, Style::Italic);
        assert_eq!(weight_of(upright), 600, "upright bold is not the 600 face");
        assert_eq!(weight_of(italic), 600, "bold italic is not the 600 face");
        assert!(
            is_bundled(&db, italic, ROBOTO_SEMIBOLD_ITALIC_BYTES),
            "bold italic is not the bundled Roboto-SemiBoldItalic"
        );
    }

    /// FR-001, FR-002, FR-003, FR-005 through the renderer every in-crate test shares: italic
    /// `ROBOTO` resolves to the bundled italic of each weight, and bold italic to the 600 italic.
    ///
    /// Resolves through the global font system, which also holds the host's fonts: on a machine
    /// with its own Roboto Italic installed this can fail until the pin joins `install_roboto`
    /// (feature 1259 T020).
    #[test]
    fn the_global_font_system_resolves_italic_roboto_to_the_bundled_italics() {
        let _ = super::super::test_support::renderer();
        let italic = |weight| Font {
            weight,
            style: iced::font::Style::Italic,
            ..ROBOTO
        };
        use iced::font::Weight as W;
        for (weight, bytes, what) in [
            (W::Normal, ROBOTO_ITALIC_BYTES, "Roboto-Italic"),
            (W::Medium, ROBOTO_MEDIUM_ITALIC_BYTES, "Roboto-MediumItalic"),
            (
                W::Semibold,
                ROBOTO_SEMIBOLD_ITALIC_BYTES,
                "Roboto-SemiBoldItalic",
            ),
            (
                W::Bold,
                ROBOTO_SEMIBOLD_ITALIC_BYTES,
                "Roboto-SemiBoldItalic",
            ),
        ] {
            assert!(
                resolves_to_bundled(italic(weight), bytes),
                "italic ROBOTO at {weight:?} does not resolve to the bundled {what}"
            );
        }
    }
}
