//! Foundational icon-vocabulary invariants (FR-002, FR-003, SC-006). Pure — no iced, runs
//! under `cargo test --no-default-features`. The codepoints are pinned from the shipped font
//! (assets/fonts/PROVENANCE.md); this test regression-locks the name→glyph mapping so a font
//! swap that moves a glyph is caught.

use micold_client::icons::Icon;

/// The pinned codepoint for every icon (from assets/fonts/PROVENANCE.md).
fn expected(icon: Icon) -> char {
    match icon {
        Icon::Help => '\u{e8fd}',
        Icon::About => '\u{e88e}',
        Icon::OpenProject => '\u{e2c8}',
        Icon::Rename => '\u{f097}',
        Icon::Git => '\u{eaf5}',
        Icon::ActiveMarker => '\u{f0be}',
        Icon::Unavailable => '\u{f8b6}',
        Icon::NavigateUp => '\u{e5d8}',
        Icon::AddSession => '\u{e145}',
        Icon::AddWorktree => '\u{e97a}',
        Icon::Menu => '\u{e5d4}',
        Icon::LightMode => '\u{e518}',
        Icon::DarkMode => '\u{e51c}',
        Icon::AutoMode => '\u{e1ab}',
        Icon::HideSidebar => '\u{f717}',
        Icon::ShowSidebar => '\u{f716}',
        Icon::Settings => '\u{e8b8}',
        Icon::Delete => '\u{e872}',
        Icon::Copy => '\u{e14d}',
        Icon::Filter => '\u{e152}',
        Icon::AiCli => '\u{e65f}',
        Icon::RegularTerminal => '\u{eb8e}',
        Icon::ReleaseFocus => '\u{e31a}',
        Icon::ProjectRoot => '\u{e88a}',
        Icon::AddTerminalInstance => '\u{e146}',
        Icon::Close => '\u{e5cd}',
    }
}

#[test]
fn glyph_maps_every_variant_to_its_pinned_codepoint() {
    for &icon in Icon::ALL {
        assert_eq!(
            icon.glyph(),
            expected(icon),
            "{icon:?} must map to its pinned codepoint"
        );
    }
}

#[test]
fn all_covers_every_variant_without_duplicates() {
    // Curated icon set (research R5 / PROVENANCE.md); +1 for Settings (feature 006),
    // +1 for Delete (feature 008), +1 for Copy (cross-app clipboard), +1 for Filter
    // (feature 009), +3 for AiCli/RegularTerminal/ReleaseFocus, +1 for ProjectRoot (T012)
    // (all feature 010), +1 for Close (session row's trailing action, shared with feature 011's
    // terminal-instance close action rather than double-counted), +1 for AddTerminalInstance
    // (feature 011).
    assert_eq!(Icon::ALL.len(), 26, "curated set size");

    // No duplicate variants.
    for (i, &a) in Icon::ALL.iter().enumerate() {
        for &b in &Icon::ALL[i + 1..] {
            assert_ne!(a, b, "Icon::ALL must not contain duplicates");
        }
    }

    // No duplicate glyphs (two icons must never collide on one codepoint).
    for (i, &a) in Icon::ALL.iter().enumerate() {
        for &b in &Icon::ALL[i + 1..] {
            assert_ne!(
                a.glyph(),
                b.glyph(),
                "{a:?} and {b:?} must not share a codepoint"
            );
        }
    }
}
