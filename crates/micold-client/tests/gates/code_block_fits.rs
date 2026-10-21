//! A code block's header holds its parts, and a long line scrolls inside the block (feature 940,
//! T025, FR-010, research R9).
//!
//! The showcase's code block — the one the gallery poses — is laid out in every copy state at a
//! narrow width. In each, the `rust` tag, the copy glyph and the state's words ("Copied", "Copy
//! failed") are drawn on one row inside the block, and the block is no wider than the room it was
//! given. The sample's long line is wider than that room: it is clipped to the block, never
//! wrapped, so it scrolls sideways inside the block rather than widening the transcript.
//!
//! Compiled into the `layout_snapshot` binary beside `disclosure_card_header_fits`.

use crate::support::layout as lay;
use iced::Size;
use micold_client::icons::Icon;
use micold_client::showcase::sections::surfaces::{code_block_sample, CodePose};
use micold_client::showcase::state::Message;
use micold_core::theme::ColorScheme;
use micold_core::tokens;

/// Half a pixel, matching every geometry gate beside this one.
const TOLERANCE: f32 = 0.5;

/// Narrower than the sample's long line, as a phone-width transcript is.
const ROOM: Size = Size::new(280.0, 400.0);

/// How far apart two parts of one row may sit vertically: a label line.
const ROW: f32 = 24.0;

#[test]
fn a_code_blocks_header_parts_stay_in_its_header_row_and_a_long_line_scrolls_inside() {
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let roles = tokens::roles(scheme);
        for pose in [CodePose::Idle, CodePose::Copied, CodePose::Failed] {
            let what = format!("{scheme:?}, {pose:?}");
            let mut renderer = lay::renderer();
            let (painted, records) = lay::painted_text_settled_at(
                code_block_sample(pose, Message::NoOp, roles),
                &mut renderer,
                ROOM,
            );
            let root = records
                .iter()
                .find(|r| r.path.is_empty())
                .unwrap_or_else(|| panic!("{what}: no root record in {records:?}"));
            assert!(
                root.x + root.width <= ROOM.width + TOLERANCE,
                "{what}: the block is wider than its room: {root:?}"
            );
            let glyph = match pose {
                CodePose::Copied => Icon::CardDone,
                _ => Icon::Copy,
            };
            let mut header = vec!["rust".to_string(), glyph.glyph().to_string()];
            match pose {
                CodePose::Copied => header.push("Copied".into()),
                CodePose::Failed => header.push("Copy failed".into()),
                _ => {}
            }
            let parts: Vec<_> = header
                .iter()
                .map(|content| {
                    painted
                        .iter()
                        .find(|p| &p.content == content)
                        .unwrap_or_else(|| panic!("{what}: `{content}` is drawn: {painted:?}"))
                })
                .collect();
            let top = parts[0].origin.y;
            for p in &parts {
                let width = p.natural_width.min(p.allowed_width);
                assert!(
                    (p.origin.y - top).abs() <= ROW
                        && p.origin.x >= root.x - TOLERANCE
                        && p.origin.x + width <= root.x + root.width + TOLERANCE,
                    "{what}: `{}` at {:?} ({width} wide) leaves the header row at y {top} or \
                     the block {root:?}",
                    p.content,
                    p.origin
                );
            }
            let long = painted
                .iter()
                .find(|p| p.content.contains("long enough"))
                .unwrap_or_else(|| panic!("{what}: the long line is drawn: {painted:?}"));
            assert!(
                long.natural_width > ROOM.width,
                "{what}: the sample's long line is wider than the room: {long:?}"
            );
            assert!(
                long.natural_height <= 20.0,
                "{what}: the long line is one line, not wrapped: {long:?}"
            );
            assert!(
                long.clip.x + long.clip.width <= root.x + root.width + TOLERANCE,
                "{what}: the long line is clipped to the block, not drawn past it: {long:?}"
            );
        }
    }
}
