//! The Roboto faces the application ships, upright and italic, and how they are registered
//! (feature 018 FR-008, BUG-1298, feature 1259 contract `fonts.md` C-1 and C-3).
//!
//! Every interface role draws in [`ROBOTO`]. Three upright weights ship (400 and 500 for the
//! Material 3 type scale, 600 for the Overview card's semibold words), and each has an italic twin
//! so markdown emphasis draws in Roboto's own italic at the weight of the text around it: a role or
//! a bold run (`Weight::Bold`, which resolves to 600) that gains emphasis keeps its weight.
//! [`install_roboto`] registers all six; see `assets/fonts/PROVENANCE.md` for where they come from.

use std::borrow::Cow;
use std::sync::LazyLock;

use iced::advanced::graphics::text::cosmic_text::fontdb;
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
    let db = fonts.raw().db_mut();
    alias_bold(db);
    pin_roboto_italics(db);
}

/// Make the interface family resolve to the bundled faces only where a host Roboto could win, and
/// return how many host faces it removed (feature 1259 contract C-2, FR-003, FR-004, FR-006).
///
/// Removes every face of [`ROBOTO_FAMILY`] that was not loaded from memory (`Source::Binary`) and
/// is either not upright or not at a shipped weight ([`ROBOTO_WEIGHTS`]). `Database::query` breaks
/// a tie between a host copy and a bundled face in favour of the face found first, and host faces
/// are loaded first, so a host Roboto Italic would beat the bundled italics. A host upright face at
/// 700 or 900 would beat the bold alias and the 600 face (cosmic-text matches the requested weight
/// exactly, see [`alias_bold`]), leaving bold upright heavier than bold italic. Host upright faces
/// at 400, 500 and 600 stay (FR-004). Faces of other families stay, so characters Roboto lacks
/// still fall back to a system face. Idempotent: a second call removes nothing.
pub fn pin_roboto_italics(db: &mut fontdb::Database) -> usize {
    super::mono::remove_host_faces(db, ROBOTO_FAMILY, |face| {
        face.style != fontdb::Style::Normal || !ROBOTO_WEIGHTS.contains(&face.weight.0)
    })
}

/// Register the bundled 600 faces a second time at weight 700, upright and italic.
///
/// cosmic-text (0.15) draws a run in the requested family only from a face of exactly the
/// requested weight; any other weight sends the run to the fallback list, where a host face that
/// is exactly 700 wins (DejaVu Sans Bold, or DejaVu Sans Mono Bold Oblique for bold italic, in the
/// feature 1259 visual pass). Markdown bold asks for `Weight::Bold` (700), which no Roboto face
/// ships, so without these aliases bold left Roboto altogether. Each alias shares its 600 face's
/// bytes, so bold draws in Roboto SemiBold and bold italic in Roboto SemiBold Italic. Idempotent.
fn alias_bold(db: &mut fontdb::Database) {
    let bundled_600 = |db: &fontdb::Database, face: &fontdb::FaceInfo| {
        [ROBOTO_SEMIBOLD_BYTES, ROBOTO_SEMIBOLD_ITALIC_BYTES]
            .iter()
            .any(|bytes| db.with_face_data(face.id, |data, _| data == *bytes) == Some(true))
    };
    let faces: Vec<fontdb::FaceInfo> = db
        .faces()
        .filter(|f| matches!(f.source, fontdb::Source::Binary(_)) && bundled_600(db, f))
        .cloned()
        .collect();
    let has_alias = |style| {
        faces
            .iter()
            .any(|f| f.weight == fontdb::Weight::BOLD && f.style == style)
    };
    for mut face in faces.iter().filter(|f| f.weight.0 == 600).cloned() {
        if has_alias(face.style) {
            continue;
        }
        face.weight = fontdb::Weight::BOLD;
        db.push_face_info(face);
    }
}

/// Whether Roboto's italic has a glyph for `c`. The three italics cover the same characters
/// (asserted below), so the 400 face answers for all of them.
pub fn italic_has_glyph(c: char) -> bool {
    static ITALIC: LazyLock<Option<ttf_parser::Face<'static>>> =
        LazyLock::new(|| ttf_parser::Face::parse(ROBOTO_ITALIC_BYTES, 0).ok());
    ITALIC
        .as_ref()
        .is_some_and(|face| face.glyph_index(c).is_some())
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{is_bundled, query, resolves_to_bundled};
    use super::*;
    use fontdb::{Family, Source, Style, Weight};
    use iced::font::Weight as W;
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
        query(db, Family::Name(ROBOTO_FAMILY), weight, style)
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

    /// C-3: `alias_bold` adds a weight-700 face over the bundled 600 bytes, upright and italic, so
    /// a bold request has a Roboto face at the exact weight cosmic-text matches on.
    #[test]
    fn bold_aliases_the_bundled_semibold_faces() {
        let mut db = bundled();
        alias_bold(&mut db);
        alias_bold(&mut db);
        let at_700: Vec<_> = db.faces().filter(|f| f.weight == Weight::BOLD).collect();
        assert_eq!(
            at_700.len(),
            2,
            "expected one 700 alias upright and one italic, once each"
        );
        let upright = roboto(&db, Weight::BOLD.0, Style::Normal);
        let italic = roboto(&db, Weight::BOLD.0, Style::Italic);
        assert!(
            is_bundled(&db, upright, ROBOTO_SEMIBOLD_BYTES),
            "bold is not Roboto-SemiBold"
        );
        assert!(
            is_bundled(&db, italic, ROBOTO_SEMIBOLD_ITALIC_BYTES),
            "bold italic is not Roboto-SemiBoldItalic"
        );
    }

    /// `ROBOTO_SEMIBOLD_BYTES` with the OS/2 `usWeightClass` set to 700 (glyphs unchanged), the
    /// host Roboto Bold the bundled faces do not include.
    fn bold_700_bytes() -> Vec<u8> {
        let mut bytes = ROBOTO_SEMIBOLD_BYTES.to_vec();
        let tables = usize::from(u16::from_be_bytes([bytes[4], bytes[5]]));
        let record = (0..tables)
            .map(|i| 12 + 16 * i)
            .find(|&r| &bytes[r..r + 4] == b"OS/2")
            .expect("the face has an OS/2 table");
        let at = u32::from_be_bytes(bytes[record + 8..record + 12].try_into().unwrap()) as usize;
        bytes[at + 4..at + 6].copy_from_slice(&700u16.to_be_bytes());
        bytes
    }

    /// C-2: a host Roboto Italic and a host Roboto Bold (700) never beat the bundled faces; a host
    /// Roboto Regular (400) and faces of other families stay; a second call removes nothing.
    #[test]
    fn pin_drops_host_italics_and_off_weight_faces_only() {
        let dir = tempfile::tempdir().expect("temp dir");
        let host = |name: &str, bytes: &[u8]| {
            let path = dir.path().join(name);
            std::fs::write(&path, bytes).expect("write host copy");
            path
        };
        let mut db = fontdb::Database::new();
        // Host copies load first, as `load_system_fonts` does before the bundled faces.
        db.load_font_file(host("Roboto-Italic.ttf", ROBOTO_ITALIC_BYTES))
            .expect("load host italic");
        db.load_font_file(host("Roboto-Regular.ttf", ROBOTO_REGULAR_BYTES))
            .expect("load host regular");
        db.load_font_file(host("Roboto-Bold.ttf", &bold_700_bytes()))
            .expect("load host bold");
        for bytes in ROBOTO_FACES {
            db.load_font_source(Source::Binary(Arc::new(bytes)));
        }
        db.load_font_source(Source::Binary(Arc::new(crate::FIRA_MONO_REGULAR_BYTES)));
        assert!(
            db.faces().any(|f| !matches!(f.source, Source::Binary(_))
                && f.weight == Weight::BOLD
                && f.style == Style::Normal),
            "the patched host face does not load with weight 700"
        );
        let upright_before: Vec<_> = [400, 500, 600]
            .map(|w| roboto(&db, w, Style::Normal))
            .to_vec();
        let faces_before = db.len();

        assert_eq!(pin_roboto_italics(&mut db), 2, "host italic and host bold");
        assert_eq!(db.len(), faces_before - 2);
        assert!(
            db.faces().any(|f| !matches!(f.source, Source::Binary(_))
                && f.weight == Weight::NORMAL
                && f.style == Style::Normal),
            "the host Roboto Regular was removed"
        );
        for (weight, bytes, _) in ITALIC {
            assert!(
                is_bundled(&db, roboto(&db, weight, Style::Italic), bytes),
                "italic {weight} is not the bundled face"
            );
        }
        assert!(is_bundled(
            &db,
            roboto(&db, 700, Style::Italic),
            ROBOTO_SEMIBOLD_ITALIC_BYTES
        ));
        assert!(
            is_bundled(&db, roboto(&db, 700, Style::Normal), ROBOTO_SEMIBOLD_BYTES),
            "upright bold is not the bundled SemiBold"
        );
        for (weight, before) in [400, 500, 600].into_iter().zip(upright_before) {
            assert_eq!(
                roboto(&db, weight, Style::Normal),
                before,
                "upright {weight} resolves differently after the pin"
            );
        }
        assert!(
            db.faces()
                .any(|f| is_bundled(&db, f.id, crate::FIRA_MONO_REGULAR_BYTES)),
            "a face of another family was removed"
        );
        assert_eq!(pin_roboto_italics(&mut db), 0, "a second call is a no-op");
    }

    /// Whether the global font system holds a Roboto face the application did not bundle. The pin
    /// leaves host upright faces at 400, 500 and 600 (FR-004), and one of them wins a match there.
    fn host_roboto_present() -> bool {
        let mut fonts = iced::advanced::graphics::text::font_system()
            .write()
            .expect("the global font system lock was poisoned");
        let host = fonts.raw().db().faces().any(|f| {
            !matches!(f.source, Source::Binary(_))
                && f.families.iter().any(|(name, _)| name == ROBOTO_FAMILY)
        });
        host
    }

    /// FR-001, FR-002, FR-003, FR-005, SC-002 through the renderer's own matching (cosmic-text
    /// shaping, not a fontdb `Query`): upright and italic `ROBOTO` draw in the bundled face of each
    /// weight, and bold (700) in the 600 faces, upright and italic alike.
    #[test]
    fn the_renderer_draws_each_weight_and_bold_in_the_bundled_faces() {
        let _ = super::super::test_support::renderer();
        if host_roboto_present() {
            eprintln!("skipped: the host has its own upright Roboto faces (FR-004 keeps them)");
            return;
        }
        for (style, faces) in [
            (iced::font::Style::Normal, &UPRIGHT),
            (iced::font::Style::Italic, &ITALIC),
        ] {
            let semibold = faces[2];
            for (weight, (_, bytes, what)) in [
                (W::Normal, faces[0]),
                (W::Medium, faces[1]),
                (W::Semibold, semibold),
                (W::Bold, semibold),
            ] {
                let font = Font {
                    weight,
                    style,
                    ..ROBOTO
                };
                assert!(
                    resolves_to_bundled(font, bytes),
                    "{style:?} ROBOTO at {weight:?} is not drawn in the bundled {what}"
                );
            }
        }
    }

    /// The three italics cover the same characters, so [`italic_has_glyph`] (which reads the 400
    /// italic) answers for every weight.
    #[test]
    fn the_italics_cover_the_same_characters() {
        let faces: Vec<_> = ITALIC
            .iter()
            .map(|(_, bytes, _)| ttf_parser::Face::parse(bytes, 0).expect("bundled font parses"))
            .collect();
        for c in (0u32..0x3_0000).filter_map(char::from_u32) {
            let covered: Vec<bool> = faces.iter().map(|f| f.glyph_index(c).is_some()).collect();
            assert!(
                covered.iter().all(|&h| h == covered[0]),
                "the italics disagree on {c:?}: {covered:?}"
            );
        }
        assert!(italic_has_glyph('a') && italic_has_glyph('é'));
        let emoji = char::from_u32(0x1F642).expect("U+1F642 is a scalar value");
        assert!(!italic_has_glyph('漢') && !italic_has_glyph(emoji));
    }
}
