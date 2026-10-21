//! A `DisclosureCard`'s header holds its parts (feature 939, research R11, plan §Geometry gates).
//!
//! The showcase's tool-call card — the one the gallery poses — is laid out folded and open, failed
//! and not, at a narrow width. In each, the leading icon, the title, the one-line summary (folded
//! only), the trailing status and the chevron are drawn inside the header row, and the summary is
//! one line. The header is the card's full-width button: the first node 40 dp tall, the height
//! `AccordionSection`'s header has always had.
//!
//! Compiled into the `layout_snapshot` binary beside `structured_row_marker_fits`.

use crate::support::layout as lay;
use iced::Size;
use micold_client::icons::Icon;
use micold_client::showcase::sections::surfaces::{edit_card, tool_card, ToolPose, EDIT_META};
use micold_client::showcase::state::Message;
use micold_core::theme::ColorScheme;
use micold_core::tokens;

/// Half a pixel, matching every geometry gate beside this one.
const TOLERANCE: f32 = 0.5;

/// Narrow enough that the sample summary has to be cut.
const ROOM: Size = Size::new(280.0, 400.0);

/// `AccordionSection`'s header height (its `TODAYS_HEADER_HEIGHT`).
const HEADER: f32 = 40.0;

/// The caption role's line height: the summary's one line.
const CAPTION_LINE: f32 = 16.0;

#[test]
fn a_disclosure_cards_header_parts_stay_inside_its_header_row() {
    let roles = tokens::roles(ColorScheme::Light);
    for pose in [ToolPose::Done, ToolPose::Running, ToolPose::Failed] {
        for open in [false, true] {
            let what = format!("{pose:?}, open {open}");
            let mut renderer = lay::renderer();
            let (painted, records) = lay::painted_text_settled_at(
                tool_card(pose, open, Message::NoOp, roles),
                &mut renderer,
                ROOM,
            );
            let header = records
                .iter()
                .find(|r| (r.height - HEADER).abs() <= TOLERANCE && r.width > ROOM.width / 2.0)
                .unwrap_or_else(|| panic!("{what}: no {HEADER} dp header row in {records:?}"));
            let (top, bottom) = (header.y, header.y + header.height);
            let in_header: Vec<_> = painted
                .iter()
                .filter(|p| p.origin.y >= top - TOLERANCE && p.origin.y < bottom)
                .collect();
            let (status_icon, status) = match pose {
                ToolPose::Done => (Icon::ToolDone, "Done"),
                ToolPose::Running => (Icon::ToolRunning, "Running"),
                ToolPose::Failed => (Icon::ToolFailed, "Failed"),
            };
            let chevron = if open { Icon::MoveUp } else { Icon::MoveDown };
            let mut expected = vec![
                Icon::ToolCall.glyph().to_string(),
                "Bash".to_string(),
                status_icon.glyph().to_string(),
                status.to_string(),
                chevron.glyph().to_string(),
            ];
            let summary = in_header.iter().find(|p| p.content.starts_with("cargo"));
            match (open, summary) {
                (false, Some(s)) => {
                    assert!(
                        s.natural_height <= CAPTION_LINE + TOLERANCE,
                        "{what}: the summary is one line: {s:?}"
                    );
                    expected.push(s.content.clone());
                }
                (false, None) => panic!("{what}: a folded card shows its summary: {in_header:?}"),
                (true, Some(s)) => panic!("{what}: an open card shows no summary: {s:?}"),
                (true, None) => {}
            }
            for content in &expected {
                assert!(
                    in_header.iter().any(|p| &p.content == content),
                    "{what}: `{content}` is drawn in the header row: {in_header:?}"
                );
            }
            for p in &in_header {
                let width = p.natural_width.min(p.allowed_width);
                assert!(
                    p.origin.x >= header.x - TOLERANCE
                        && p.origin.x + width <= header.x + header.width + TOLERANCE
                        && p.origin.y + p.natural_height.min(HEADER) <= bottom + TOLERANCE,
                    "{what}: `{}` at {:?} ({width} wide, {} tall) runs out of the header row {:?}",
                    p.content,
                    p.origin,
                    p.natural_height,
                    (header.x, header.y, header.width, header.height)
                );
            }
        }
    }
}

/// Feature 940 (FR-017): an Edit card's `+12 −3` meta is drawn whole in the header row, beside the
/// summary and the trailing status, folded and open, at the narrow width — the summary gives way.
#[test]
fn an_edit_cards_meta_is_never_ellipsized() {
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let roles = tokens::roles(scheme);
        for open in [false, true] {
            let what = format!("{scheme:?}, open {open}");
            let mut renderer = lay::renderer();
            let (painted, records) = lay::painted_text_settled_at(
                edit_card(open, Message::NoOp, roles),
                &mut renderer,
                ROOM,
            );
            let header = records
                .iter()
                .find(|r| (r.height - HEADER).abs() <= TOLERANCE && r.width > ROOM.width / 2.0)
                .unwrap_or_else(|| panic!("{what}: no {HEADER} dp header row in {records:?}"));
            let meta = painted
                .iter()
                .find(|p| p.content == EDIT_META)
                .unwrap_or_else(|| panic!("{what}: the meta is drawn whole: {painted:?}"));
            assert!(
                meta.natural_width <= meta.allowed_width + TOLERANCE,
                "{what}: the meta is cut: {meta:?}"
            );
            assert!(
                meta.origin.y >= header.y - TOLERANCE
                    && meta.origin.y < header.y + header.height
                    && meta.origin.x + meta.natural_width <= header.x + header.width + TOLERANCE,
                "{what}: the meta leaves the header row {header:?}: {meta:?}"
            );
            let status = painted
                .iter()
                .find(|p| p.content == "Done")
                .unwrap_or_else(|| panic!("{what}: the status is drawn: {painted:?}"));
            assert!(
                meta.origin.x + meta.natural_width <= status.origin.x + TOLERANCE,
                "{what}: the meta sits before the status: {meta:?} {status:?}"
            );
            if !open {
                let summary = painted
                    .iter()
                    .find(|p| p.content.starts_with("crates/"))
                    .unwrap_or_else(|| panic!("{what}: the summary is drawn: {painted:?}"));
                assert!(
                    summary.content.ends_with('\u{2026}'),
                    "{what}: the summary gives way: {summary:?}"
                );
                assert!(
                    summary.origin.x + summary.natural_width.min(summary.allowed_width)
                        <= meta.origin.x + TOLERANCE,
                    "{what}: the summary runs into the meta: {summary:?} {meta:?}"
                );
            }
        }
    }
}
