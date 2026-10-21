//! A table in a formatted reply wraps a long cell inside its column (feature 940, T037, FR-003).
//!
//! The recorded `markdown-turn` reply is drawn through the window's rich renderers in a transcript
//! narrower than its table's natural width, as a phone-width pane is. Every cell is drawn inside the
//! width its column was given, and at least one wraps onto a second line inside its cell rather
//! than widening the table or the transcript.
//!
//! Compiled into the `layout_snapshot` binary beside `transcript_jump_control_fits`.

use crate::support::layout as lay;
use crate::support::transcript::{ctx, state_of};
use iced::Size;
use micold_client::ui::syntax::GRAMMAR;
use micold_client::ui::transcript::{view, BlockCache, BlockRenderers};
use micold_core::session::SessionId;

/// Half a pixel, matching every geometry gate beside this one.
const TOLERANCE: f32 = 0.5;

/// Narrower than the table's three columns laid side by side at their natural widths.
const ROOM: Size = Size::new(170.0, 6_000.0);

/// The table's cells in `markdown-turn`.
const CELLS: [&str; 12] = [
    "Tea", "Temp", "Time", "Green", "80 C", "2 min", "Black", "95 C", "3 min", "Herbal", "100 C",
    "5 min",
];

#[test]
fn a_table_cell_wraps_inside_its_column_at_a_narrow_width() {
    let session = SessionId::new();
    let state = state_of(session, "markdown-turn");
    let mut cache = BlockCache::default();
    let _jobs = cache.refresh(session, state.transcript(session).unwrap(), &GRAMMAR);
    let element = view(&state, session, &BlockRenderers::RICH, ctx(&cache));
    let (painted, _) = lay::painted_text_settled_at(element, &mut lay::renderer(), ROOM);

    let cells: Vec<_> = painted
        .iter()
        .filter(|p| CELLS.contains(&p.content.as_str()))
        .collect();
    assert_eq!(
        cells.len(),
        CELLS.len(),
        "every cell is drawn on its own: {painted:#?}"
    );
    let line = cells
        .iter()
        .map(|c| c.natural_height)
        .fold(f32::INFINITY, f32::min);
    for c in &cells {
        assert!(
            c.natural_width <= c.allowed_width + TOLERANCE,
            "`{}` wants {} but its column gives {}",
            c.content,
            c.natural_width,
            c.allowed_width
        );
        assert!(
            c.clip.x + c.clip.width <= ROOM.width + TOLERANCE,
            "`{}` is drawn past the transcript's width: {:?}",
            c.content,
            c.clip
        );
    }
    assert!(
        cells.iter().any(|c| c.natural_height > line * 1.5),
        "a long cell wraps inside its column: {cells:#?}"
    );
}
