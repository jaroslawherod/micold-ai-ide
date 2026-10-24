//! The monospace face the application ships, and how `Font::MONOSPACE` is made to reach it
//! (feature 1260, contract `fonts.md` C-1 to C-3).
//!
//! Code-like text — code blocks, inline code, diffs, tool output and terminal cells — asks for
//! `Font::MONOSPACE`, which is `Family::Monospace`. Left alone, cosmic-text resolves that on the host
//! (`Noto Sans Mono` when installed, otherwise whatever its lookup lands on), so code looked
//! different on every machine. [`install_monospace`] loads the bundled Fira Mono, removes any
//! other face of that family (a host copy, any version) and points the monospace family at it, so
//! every call site draws with the shipped face without changing.

use std::borrow::Cow;

use iced::advanced::graphics::text::cosmic_text::fontdb;
use iced::advanced::graphics::text::FontSystem;

/// The family both bundled faces report (name ID 1), and the database's monospace family.
pub const MONO_FAMILY: &str = "Fira Mono";

/// Fira Mono Regular (400), as shipped in `assets/fonts/` (provenance: `assets/fonts/PROVENANCE.md`).
pub const FIRA_MONO_REGULAR_BYTES: &[u8] =
    include_bytes!("../../../../../assets/fonts/FiraMono-Regular.ttf");

/// Fira Mono Bold (700): bold code and bold terminal cells (FR-011).
pub const FIRA_MONO_BOLD_BYTES: &[u8] =
    include_bytes!("../../../../../assets/fonts/FiraMono-Bold.ttf");

/// Make `Family::Monospace` resolve to the bundled faces only, and return how many other faces of
/// [`MONO_FAMILY`] it removed.
///
/// Removes every face of [`MONO_FAMILY`] that was not loaded from memory (`Source::Binary`, which is
/// how the bundled bytes are loaded): `Database::query` breaks a tie between a host copy and the
/// bundled one in favour of the face found first, and host faces are loaded first (FR-003). Faces of
/// every other family stay, so characters Fira Mono lacks still fall back to a system face (FR-004).
/// Idempotent: a second call removes nothing.
pub fn pin_monospace(db: &mut fontdb::Database) -> usize {
    let foreign: Vec<fontdb::ID> = db
        .faces()
        .filter(|face| {
            face.families.iter().any(|(name, _)| name == MONO_FAMILY)
                && !matches!(face.source, fontdb::Source::Binary(_))
        })
        .map(|face| face.id)
        .collect();
    for &id in &foreign {
        db.remove_face(id);
    }
    db.set_monospace_family(MONO_FAMILY);
    foreign.len()
}

/// Register the bundled monospace faces on `fonts` and pin the monospace family to them.
///
/// Called once, before any text is shaped (cosmic-text caches font matches): by the application and
/// the showcase at startup, and by the test renderers. A face whose bytes do not parse is logged and
/// skipped by `load_font`; code then falls back to a host face and the tests below fail (research R8).
pub fn install_monospace(fonts: &mut FontSystem) {
    fonts.load_font(Cow::Borrowed(FIRA_MONO_REGULAR_BYTES));
    fonts.load_font(Cow::Borrowed(FIRA_MONO_BOLD_BYTES));
    pin_monospace(fonts.raw().db_mut());
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{is_bundled, resolves_to_bundled};
    use super::*;
    use fontdb::{Family, Query, Source, Style, Weight};
    use std::sync::Arc;

    /// C-1: the face parses, names itself `Fira Mono`, has the given weight, and is fixed-pitch at
    /// 600 units per 1000-unit em for every printable ASCII character.
    fn assert_bundled_face(bytes: &[u8], weight: u16, what: &str) {
        let face = ttf_parser::Face::parse(bytes, 0)
            .unwrap_or_else(|e| panic!("bundled Fira Mono {what} does not parse: {e}"));
        let family = face
            .names()
            .into_iter()
            .filter(|n| n.name_id == ttf_parser::name_id::FAMILY && n.is_unicode())
            .find_map(|n| n.to_string());
        assert_eq!(
            family.as_deref(),
            Some(MONO_FAMILY),
            "bundled Fira Mono {what}: family (name ID 1)"
        );
        assert_eq!(
            face.weight().to_number(),
            weight,
            "bundled Fira Mono {what}: weight"
        );
        assert!(
            face.is_monospaced(),
            "bundled Fira Mono {what} is not fixed-pitch"
        );
        let em = face.units_per_em();
        for c in ' '..='~' {
            let glyph = face
                .glyph_index(c)
                .unwrap_or_else(|| panic!("bundled Fira Mono {what} lacks {c:?}"));
            let advance = face.glyph_hor_advance(glyph).unwrap_or(0);
            assert_eq!(
                u32::from(advance) * 1000,
                600 * u32::from(em),
                "bundled Fira Mono {what}: advance of {c:?} is not 600/1000 em"
            );
        }
    }

    #[test]
    fn the_regular_bytes_are_fira_mono_regular() {
        assert_bundled_face(FIRA_MONO_REGULAR_BYTES, 400, "Regular");
    }

    #[test]
    fn the_bold_bytes_are_fira_mono_bold() {
        assert_bundled_face(FIRA_MONO_BOLD_BYTES, 700, "Bold");
    }

    fn monospace(db: &fontdb::Database, weight: u16, style: Style) -> fontdb::ID {
        db.query(&Query {
            families: &[Family::Monospace],
            weight: Weight(weight),
            style,
            ..Query::default()
        })
        .unwrap_or_else(|| panic!("Family::Monospace resolves to no face (weight {weight})"))
    }

    #[test]
    fn pin_monospace_drops_a_host_copy_and_resolves_to_the_bundled_faces() {
        let dir = tempfile::tempdir().expect("temp dir");
        let host_copy = dir.path().join("FiraMono-Regular.ttf");
        std::fs::write(&host_copy, FIRA_MONO_REGULAR_BYTES).expect("write host copy");

        let mut db = fontdb::Database::new();
        // The host copy is loaded first, as `load_system_fonts` does before the bundled faces.
        db.load_font_file(&host_copy).expect("load host copy");
        db.load_font_source(Source::Binary(Arc::new(
            crate::ui::material::ROBOTO_REGULAR_BYTES,
        )));
        db.load_font_source(Source::Binary(Arc::new(FIRA_MONO_REGULAR_BYTES)));
        db.load_font_source(Source::Binary(Arc::new(FIRA_MONO_BOLD_BYTES)));
        let before = db.len();

        assert_eq!(
            pin_monospace(&mut db),
            1,
            "the host Fira Mono copy is removed"
        );
        assert!(
            db.faces()
                .filter(|f| f.families.iter().any(|(name, _)| name == MONO_FAMILY))
                .all(|f| matches!(f.source, Source::Binary(_))),
            "a Fira Mono face not loaded from memory survived"
        );
        assert_eq!(db.family_name(&Family::Monospace), MONO_FAMILY);
        assert!(
            is_bundled(
                &db,
                monospace(&db, 400, Style::Normal),
                FIRA_MONO_REGULAR_BYTES
            ),
            "Family::Monospace weight 400 is not the bundled Fira Mono Regular"
        );
        assert!(
            is_bundled(
                &db,
                monospace(&db, 700, Style::Normal),
                FIRA_MONO_BOLD_BYTES
            ),
            "Family::Monospace weight 700 is not the bundled Fira Mono Bold"
        );
        assert!(
            is_bundled(
                &db,
                monospace(&db, 400, Style::Italic),
                FIRA_MONO_REGULAR_BYTES
            ),
            "italic weight 400 is not the upright bundled Fira Mono Regular"
        );
        assert!(
            is_bundled(
                &db,
                monospace(&db, 700, Style::Italic),
                FIRA_MONO_BOLD_BYTES
            ),
            "italic weight 700 is not the upright bundled Fira Mono Bold"
        );
        assert!(
            db.faces()
                .any(|f| f.families.iter().any(|(name, _)| name == "Roboto")),
            "a face of another family was removed (FR-004 needs it for fallback)"
        );

        assert_eq!(pin_monospace(&mut db), 0, "a second call removes nothing");
        assert_eq!(db.len(), before - 1, "a second call changed the face count");
    }

    /// FR-002/FR-003 through the renderer every in-crate test shares: the global font system
    /// resolves `Font::MONOSPACE` to the bundled bytes, Regular and Bold.
    #[test]
    fn the_global_font_system_resolves_monospace_to_bundled_fira_mono() {
        let _ = super::super::test_support::renderer();
        let bold = iced::Font {
            weight: iced::font::Weight::Bold,
            ..iced::Font::MONOSPACE
        };
        assert!(
            resolves_to_bundled(iced::Font::MONOSPACE, FIRA_MONO_REGULAR_BYTES),
            "Font::MONOSPACE does not resolve to the bundled Fira Mono Regular"
        );
        assert!(
            resolves_to_bundled(bold, FIRA_MONO_BOLD_BYTES),
            "bold Font::MONOSPACE does not resolve to the bundled Fira Mono Bold"
        );
    }

    /// FR-004 through the global font system: a character Fira Mono lacks (a Material Symbols
    /// private-use icon, bundled by the app itself, so host-independent) is shaped with a face that
    /// is not Fira Mono and a real glyph, while the ASCII letter stays on the bundled Fira Mono.
    #[test]
    fn a_character_fira_mono_lacks_falls_back_to_another_face_not_a_box() {
        use iced::advanced::graphics::text::cosmic_text::{Buffer, Metrics, Shaping};
        const ICON: char = '\u{e5cd}';
        let _ = super::super::test_support::renderer();
        let has = |bytes: &[u8], c: char| {
            ttf_parser::Face::parse(bytes, 0)
                .expect("bundled font parses")
                .glyph_index(c)
                .is_some()
        };
        assert!(
            !has(FIRA_MONO_REGULAR_BYTES, ICON),
            "Fira Mono has {ICON:?}: pick a character it lacks (FR-004)"
        );
        assert!(
            has(super::super::glyph::MATERIAL_SYMBOLS_BYTES, ICON),
            "the bundled icon font lacks {ICON:?}: pick one it has (FR-004)"
        );

        let mut fonts = iced::advanced::graphics::text::font_system()
            .write()
            .expect("the global font system lock was poisoned");
        let attrs = iced::advanced::graphics::text::to_attributes(iced::Font::MONOSPACE);
        let fs = fonts.raw();
        let mut buffer = Buffer::new(fs, Metrics::new(14.0, 20.0));
        buffer.set_text(fs, &format!("a{ICON}"), &attrs, Shaping::Advanced, None);
        buffer.shape_until_scroll(fs, false);
        let glyphs: Vec<_> = buffer
            .layout_runs()
            .flat_map(|run| run.glyphs.to_vec())
            .collect();
        assert_eq!(glyphs.len(), 2, "expected one glyph per character (FR-004)");
        let db = fs.db();
        assert!(
            is_bundled(db, glyphs[0].font_id, FIRA_MONO_REGULAR_BYTES),
            "the ASCII letter is not shaped with the bundled Fira Mono (FR-004)"
        );
        assert!(
            !is_bundled(db, glyphs[1].font_id, FIRA_MONO_REGULAR_BYTES),
            "{ICON:?} stayed on Fira Mono, which lacks it: no fallback face (FR-004)"
        );
        assert_ne!(
            glyphs[1].glyph_id, 0,
            "{ICON:?} fell back to a .notdef box, not another face (FR-004)"
        );
    }
}
