//! The command palette fits the composer's narrowest width (#943 T055, FR-007, FR-008, FR-010).
//!
//! The showcase's palette is laid out in both schemes at the narrowest pane a split leaves
//! (`MIN_PANE_COLS` cells of 8 px). Every group header, every command's name and every second line
//! is drawn inside the panel, every command's name whole, and a terminal-only row's "opens in
//! terminal" whole: a tag pushed past the edge, or cut away with the description behind it, is a
//! warning the user never sees (FR-010). A second line too long for the row is cut with an
//! ellipsis, the tag at its head surviving.
//!
//! Compiled into the `layout_snapshot` binary beside `composer_parts_fit`.

use crate::support::layout as lay;
use iced::Size;
use micold_client::showcase::sections::composer::{command_palette_sample, PALETTE_TERMINAL_TAG};
use micold_core::pane_layout::MIN_PANE_COLS;
use micold_core::theme::ColorScheme;

/// Half a pixel, matching every geometry gate beside this one.
const TOLERANCE: f32 = 0.5;

/// The narrowest pane: `MIN_PANE_COLS` terminal cells of 8 px, as `split_view` measures a pane.
const NARROWEST: Size = Size::new(MIN_PANE_COLS as f32 * 8.0, 4_000.0);

fn root_of(records: &[lay::LayoutRecord]) -> &lay::LayoutRecord {
    records
        .iter()
        .find(|r| r.path.is_empty())
        .unwrap_or_else(|| panic!("no root record in {records:?}"))
}

#[test]
fn every_row_header_and_tag_is_drawn_inside_the_palette_at_the_narrowest_pane() {
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let roles = micold_core::tokens::roles(scheme);
        let mut renderer = lay::renderer();
        let (painted, records) =
            lay::painted_text_settled_at(command_palette_sample(roles), &mut renderer, NARROWEST);
        let panel = root_of(&records);
        assert!(
            panel.width <= NARROWEST.width + TOLERANCE,
            "{scheme:?}: the palette fits the pane: {panel:?}"
        );
        let contents: Vec<&str> = painted.iter().map(|p| p.content.as_str()).collect();
        // A matched name is drawn in runs (the match emphasised), so the row's words are read
        // from the whole list in order.
        let all = contents.concat();
        for part in [
            "Skills",
            "Built-in commands",
            "In this app",
            "/speckit-plan",
            "/speckit-specify",
            "/mcp",
            "/model",
        ] {
            assert!(
                all.contains(part),
                "{scheme:?}: `{part}` is drawn whole: {contents:?}"
            );
        }
        for text in &painted {
            let right = text.origin.x + text.natural_width.min(text.allowed_width);
            assert!(
                text.origin.x >= panel.x - TOLERANCE && right <= panel.x + panel.width + TOLERANCE,
                "{scheme:?}: `{}` stays inside the palette {panel:?}: {text:?}",
                text.content
            );
            assert!(
                text.origin.y >= panel.y - TOLERANCE
                    && text.origin.y + text.natural_height <= panel.y + panel.height + TOLERANCE,
                "{scheme:?}: `{}` is inside the palette's height {panel:?}: {text:?}",
                text.content
            );
        }
        let tag = painted
            .iter()
            .find(|p| p.content.starts_with(PALETTE_TERMINAL_TAG))
            .unwrap_or_else(|| {
                panic!("{scheme:?}: the terminal-only tag is drawn first: {contents:?}")
            });
        assert!(
            !tag.content[..PALETTE_TERMINAL_TAG.len()].contains('…'),
            "{scheme:?}: the tag is whole: {:?}",
            tag.content
        );
    }
}
