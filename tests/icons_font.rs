//! Font-asset integrity (SC-005): every `Icon` codepoint resolves to a real glyph in the
//! shipped font, and the font advertises the family name the GUI pins. Prevents "tofu" from
//! ever reaching the running UI and catches a font swap that drops a glyph or renames the
//! family. Parses the `.ttf` directly (via `ttf-parser`), so it needs no GUI/iced and runs
//! under `cargo test --no-default-features`.

use micold_ai_ide::icons::Icon;

const FONT: &[u8] = include_bytes!("../assets/fonts/MaterialSymbolsOutlined.ttf");

/// Must match the constant the GUI selects the font with (see `src/main.rs`).
const EXPECTED_FAMILY: &str = "Material Symbols Outlined";

#[test]
fn every_icon_codepoint_has_a_glyph() {
    let face = ttf_parser::Face::parse(FONT, 0).expect("shipped font must parse");
    for &icon in Icon::ALL {
        assert!(
            face.glyph_index(icon.glyph()).is_some(),
            "{icon:?} (U+{:04X}) has no glyph in the shipped font — would render as tofu",
            icon.glyph() as u32
        );
    }
}

#[test]
fn font_advertises_the_pinned_family_name() {
    let face = ttf_parser::Face::parse(FONT, 0).expect("shipped font must parse");
    let has_family = face.names().into_iter().any(|name| {
        name.name_id == ttf_parser::name_id::FAMILY
            && name.to_string().as_deref() == Some(EXPECTED_FAMILY)
    });
    assert!(
        has_family,
        "font must advertise family '{EXPECTED_FAMILY}' so the GUI can select it"
    );
}
