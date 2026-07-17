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

/// The shipped font must be a **static** instance at the pinned axis values (weight 400 /
/// FILL 0 / GRAD 0 / opsz 24 — see `assets/fonts/PROVENANCE.md`), not the full upstream
/// variable font. Regenerating via the documented pipeline without the `varLib.instancer`
/// step (research R6) would still pass every other test here — both only check glyph/name
/// presence, which the variable font also satisfies — so this guards specifically against
/// that regression, which would otherwise ship a several-times-larger binary silently.
#[test]
fn font_is_a_static_instance_not_the_variable_font() {
    let face = ttf_parser::Face::parse(FONT, 0).expect("shipped font must parse");
    assert!(
        !face.is_variable(),
        "shipped font must be a static instance (fonttools varLib.instancer), \
         not the upstream variable font — see assets/fonts/PROVENANCE.md"
    );
}

/// Loose upper bound on the shipped font's size: full upstream coverage as a static instance
/// is expected in the low single-digit-MB range (research R6). A much larger file would
/// indicate the variable font (or an unsubsetted/unstripped one) was shipped by mistake.
#[test]
fn font_size_is_within_the_expected_static_instance_range() {
    const MAX_BYTES: usize = 5 * 1024 * 1024;
    assert!(
        FONT.len() < MAX_BYTES,
        "shipped font is {} bytes, expected a static instance under {MAX_BYTES} bytes",
        FONT.len()
    );
}
