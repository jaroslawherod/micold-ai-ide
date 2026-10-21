//! An inline diff's `+`/`−` markers lie inside their cells, and it has no number gutter (feature
//! 940, T025, FR-015, FR-016, SC-006).
//!
//! The showcase's Edit, MultiEdit and Write diffs are laid out at a narrow width in both schemes.
//! Every marker glyph drawn is inside a marker cell — the diff view's own `MARKER_WIDTH` column,
//! one row tall — so a sign is never clipped or pushed into the line's text, and a greyscale copy
//! still tells an added line from a removed one by its sign. No text drawn is a bare number: the
//! tool call says nothing of where in the file its text is, so the diff shows no line numbers.
//!
//! Compiled into the `layout_snapshot` binary beside `code_block_fits`.

use crate::support::layout as lay;
use iced::Size;
use micold_client::showcase::sections::review::{inline_diff_sample, DiffPose, MARKER_WIDTH};
use micold_core::theme::ColorScheme;
use micold_core::tokens;

/// Half a pixel, matching every geometry gate beside this one.
const TOLERANCE: f32 = 0.5;

/// A narrow transcript.
const ROOM: Size = Size::new(280.0, 800.0);

/// The diff view's row height (`material::DIFF_ROW_HEIGHT`).
const ROW: f32 = 20.0;

#[test]
fn every_marker_lies_inside_its_cell_and_no_number_is_drawn() {
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let roles = tokens::roles(scheme);
        for pose in [DiffPose::Edit, DiffPose::MultiEdit, DiffPose::Write] {
            let what = format!("{scheme:?}, {pose:?}");
            let mut renderer = lay::renderer();
            let (painted, records) =
                lay::painted_text_settled_at(inline_diff_sample(pose, roles), &mut renderer, ROOM);
            let cells: Vec<_> = records
                .iter()
                .filter(|r| {
                    (r.width - MARKER_WIDTH).abs() <= TOLERANCE && r.height <= ROW + TOLERANCE
                })
                .collect();
            let markers: Vec<_> = painted
                .iter()
                .filter(|p| p.content == "+" || p.content == "\u{2212}")
                .collect();
            assert!(
                markers.iter().any(|p| p.content == "+"),
                "{what}: an added line is marked: {painted:?}"
            );
            if pose != DiffPose::Write {
                assert!(
                    markers.iter().any(|p| p.content == "\u{2212}"),
                    "{what}: a removed line is marked: {painted:?}"
                );
            }
            for m in &markers {
                let width = m.natural_width.min(m.allowed_width);
                let inside = cells.iter().any(|c| {
                    m.origin.x >= c.x - TOLERANCE
                        && m.origin.x + width <= c.x + c.width + TOLERANCE
                        && m.origin.y >= c.y - TOLERANCE
                        && m.origin.y < c.y + ROW
                });
                assert!(
                    inside,
                    "{what}: the marker `{}` at {:?} ({width} wide) is in no {MARKER_WIDTH} dp \
                     marker cell: {cells:?}",
                    m.content, m.origin
                );
            }
            let numbers: Vec<_> = painted
                .iter()
                .filter(|p| !p.content.is_empty() && p.content.chars().all(|c| c.is_ascii_digit()))
                .collect();
            assert!(
                numbers.is_empty(),
                "{what}: line numbers are drawn: {numbers:?}"
            );
        }
    }
}
