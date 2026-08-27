//! A collapsed rail's icons form a column (feature 027, FR-026c).
//!
//! The eighth gate, and the one the §B.6 visual pass produced.
//!
//! `SectionList` draws the current row `Filled` and every other row `Text`, and those two variants
//! are inset by different padding — `PADDING_FILLED` against `PADDING_TEXT`. Expanded, that is
//! invisible: the labels are what the eye tracks, and the pill's own boundary explains the extra
//! inset. Collapsed, a row is its glyph and nothing else, and the difference shows as the current
//! section's icon sitting ~5dp right of the other three. The rail stops reading as a column.
//!
//! Nothing else here can see it:
//!
//! - `layout_snapshot` records those x-positions and always would have. A record compares against
//!   what it was shown, and the misalignment was there the first time the state was recorded.
//! - `containment` asks whether a child escapes its parent. Each glyph is comfortably inside its
//!   own button.
//! - `tab_children_fit` asks whether a child got the width it asked for. Every glyph did.
//! - `sibling_parity` is the closest relative and the reason this is phrased the way it is — it
//!   reads members of one set against each other. Its set is the app bar's actions.
//!
//! So the missing question is the symmetric one for a rail: the destinations are a set whose
//! members differ only in *which* is current, and "current" is not supposed to move a row sideways.
//! Asserted rather than recorded, for the reason the first bullet gives.
//!
//! **Compiled into the `layout_snapshot` binary** for the reason `containment` gives: cargo makes
//! one process per file directly under `tests/`, and a separate process cannot reach the record
//! cache that has already resolved every covered state.

use crate::support::covered_states::covered_states;
use crate::support::layout::{self as lay, LayoutRecord};
use micold_core::theme::ColorScheme;

/// Geometry is a structural property; one scheme establishes it. The dark pass exists for colour.
const RECORDED_SCHEME: ColorScheme = ColorScheme::Light;

/// Half a pixel, matching `containment`, `panel_placement` and `sibling_parity`.
const TOLERANCE: f32 = 0.5;

/// The state that collapses the rail, by the name it is registered under.
const COLLAPSED: &str = "settings-view-rail-collapsed";

/// The anchor every settings state names for the rail's own container.
const RAIL: &str = "settings.rail";

/// The rail's rows: the direct children of the column inside the anchored container.
///
/// Structural rather than a list of paths, so a destination added later is covered without anyone
/// remembering to come back here. The column is the container's only child; each of its children is
/// one row — the destinations, then the `Length::Fill` spacer, then the collapse control.
fn rows<'a>(records: &'a [LayoutRecord], rail: &LayoutRecord) -> Vec<&'a LayoutRecord> {
    let depth = rail.path.len() + 2;
    records
        .iter()
        .filter(|r| {
            r.layer == lay::Layer::Base && r.path.len() == depth && r.path.starts_with(&rail.path)
        })
        .collect()
}

/// The single glyph a collapsed row is made of: the deepest node under it that is narrower than the
/// row, which for a bare icon is the glyph's own box.
fn glyph<'a>(records: &'a [LayoutRecord], row: &LayoutRecord) -> Option<&'a LayoutRecord> {
    records.iter().rfind(|r| {
        r.layer == lay::Layer::Base
            && r.path.len() > row.path.len()
            && r.path.starts_with(&row.path)
            && r.width < row.width - TOLERANCE
    })
}

/// Collapsed, every row's icon sits on one vertical axis — the current one included.
///
/// The collapse control is deliberately in the set. It is drawn like a destination that is never
/// current, sits directly beneath them, and a control that missed the column by 5dp would look
/// exactly as wrong as a destination that did.
#[test]
fn a_collapsed_rails_icons_share_one_axis() {
    let renderer = lay::renderer();
    let all = lay::cached_records(covered_states(), &renderer, RECORDED_SCHEME);

    let (covered, records) = covered_states()
        .iter()
        .zip(all.iter())
        .find(|(c, _)| c.name == COLLAPSED)
        .unwrap_or_else(|| panic!("no covered state named {COLLAPSED}"));

    let rail_path = covered
        .anchors
        .iter()
        .find(|a| a.name == RAIL)
        .unwrap_or_else(|| panic!("{COLLAPSED} does not name the {RAIL} anchor"))
        .path;
    let rail = records
        .iter()
        .find(|r| r.layer == lay::Layer::Base && r.path == rail_path)
        .unwrap_or_else(|| panic!("{COLLAPSED}'s {RAIL} anchor does not resolve"));

    let centres: Vec<(usize, f32)> = rows(records, rail)
        .iter()
        .enumerate()
        .filter_map(|(i, row)| glyph(records, row).map(|g| (i, g.x + g.width / 2.0)))
        .collect();

    assert!(
        centres.len() >= 4,
        "expected the four destinations and the collapse control, found {} glyphs — the rail's \
         shape moved and this gate is now measuring something else",
        centres.len()
    );

    let first = centres[0].1;
    let strays: Vec<String> = centres
        .iter()
        .filter(|(_, c)| (c - first).abs() > TOLERANCE)
        .map(|(i, c)| format!("row {i} at {c:.1}"))
        .collect();

    assert!(
        strays.is_empty(),
        "collapsed, the rail's icons must form a column, but these sit off the axis at {first:.1}: \
         {}.\n\nThe current row is `Filled` and inset by `PADDING_FILLED` where the others are \
         `Text` and inset by `PADDING_TEXT`; centring an iconic row's content is what makes the \
         two land together.",
        strays.join(", ")
    );
}
