//! **New session** and the split buttons hold the strip's right end (feature 1126 FR-008, S9).
//!
//! It replaced feature 027's `tabs_anchor_the_trailing_edge`, whose tabs anchored the bar's
//! trailing edge: FR-008 starts the tabs at the strip's leading edge and puts the fixed controls at
//! its right end instead. This reads the place, where `strip_controls_hold_their_size` reads the
//! size: New session and the splits are the strip row's last two children, in that order, and the
//! splits finish flush with the strip's trailing padding at every member count.
//!
//! Compiled into the `layout_snapshot` binary beside the other geometry gates.

use crate::support::covered_states::covered_states;
use crate::support::layout::{self as lay, LayoutRecord};
use micold_core::theme::ColorScheme;
use micold_core::tokens::spacing;

const RECORDED_SCHEME: ColorScheme = ColorScheme::Light;
const TOLERANCE: f32 = 0.5;

fn at<'r>(
    covered: &lay::CoveredState,
    records: &'r [LayoutRecord],
    name: &str,
) -> Option<&'r LayoutRecord> {
    let anchor = covered.anchors.iter().find(|a| a.name == name)?;
    records
        .iter()
        .find(|r| r.layer == lay::Layer::Base && r.path == anchor.path)
}

#[test]
fn new_session_and_the_splits_are_the_strips_last_children_at_its_trailing_edge() {
    let renderer = lay::renderer();
    let all = lay::cached_records(covered_states(), &renderer, RECORDED_SCHEME);
    let mut checked = 0usize;
    for (covered, records) in covered_states().iter().zip(all.iter()) {
        let Some(strip) = at(covered, records, "session.strip") else {
            continue;
        };
        let name = covered.name;
        let new_session = at(covered, records, "session.strip.new_session")
            .unwrap_or_else(|| panic!("{name}: New session is anchored"));
        let split = at(covered, records, "session.strip.split")
            .unwrap_or_else(|| panic!("{name}: the split buttons are anchored"));
        let row_path = &split.path[..split.path.len() - 1];
        let siblings = records
            .iter()
            .filter(|r| {
                r.layer == lay::Layer::Base
                    && r.path.len() == split.path.len()
                    && r.path.starts_with(row_path)
            })
            .count();
        assert_eq!(
            (new_session.path.last(), split.path.last()),
            (Some(&(siblings - 2)), Some(&(siblings - 1))),
            "{name}: New session and the splits must be the strip row's last two children, in \
             order (FR-008)"
        );
        assert!(
            new_session.x + new_session.width <= split.x + TOLERANCE,
            "{name}: New session {:.1}..{:.1} must sit before the splits at {:.1}",
            new_session.x,
            new_session.x + new_session.width,
            split.x,
        );
        let trailing = strip.x + strip.width - spacing::SM;
        assert!(
            ((split.x + split.width) - trailing).abs() <= TOLERANCE,
            "{name}: the splits end at {:.1}, not flush with the strip's trailing padding at \
             {trailing:.1} (FR-008: they sit at the strip's right end)",
            split.x + split.width,
        );
        checked += 1;
    }
    assert!(
        checked >= 3,
        "fewer than the three strip states were checked ({checked}), so this gate proved little"
    );
}
