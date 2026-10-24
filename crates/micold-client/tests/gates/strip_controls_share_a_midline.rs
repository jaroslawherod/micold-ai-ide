//! The strip's controls sit on one midline (feature 1126 S9, FR-008).
//!
//! It restores the alignment half of feature 027's `the_tabs_share_a_midline_with_the_controls_beside_them`,
//! which retired with the bar: the strip's tabs, **New session** and the split buttons are laid out
//! in one row centred vertically, and a control that lost its centring (a tab taller than its
//! neighbours, a button pushed to the row's top edge) reads as misplaced. `tab_children_fit` holds
//! a tab's content to its own tab's midline; this holds the tabs and the fixed controls to each
//! other's.
//!
//! Compiled into the `layout_snapshot` binary beside the other geometry gates.

use crate::support::covered_states::covered_states;
use crate::support::layout::{self as lay, LayoutRecord};
use micold_core::theme::ColorScheme;

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

fn midline(r: &LayoutRecord) -> f32 {
    r.y + r.height / 2.0
}

#[test]
fn the_tabs_and_the_controls_beside_them_share_a_midline() {
    let renderer = lay::renderer();
    let all = lay::cached_records(covered_states(), &renderer, RECORDED_SCHEME);
    let mut checked = 0usize;
    for (covered, records) in covered_states().iter().zip(all.iter()) {
        let Some(tabs) = at(covered, records, "session.strip.tabs") else {
            continue;
        };
        let name = covered.name;
        let new_session = at(covered, records, "session.strip.new_session")
            .unwrap_or_else(|| panic!("{name}: New session is anchored"));
        let split = at(covered, records, "session.strip.split")
            .unwrap_or_else(|| panic!("{name}: the split buttons are anchored"));
        let line = midline(new_session);
        let members: Vec<&LayoutRecord> = records
            .iter()
            .filter(|r| {
                r.layer == lay::Layer::Base
                    && r.path.len() == tabs.path.len() + 1
                    && r.path.starts_with(&tabs.path)
            })
            .collect();
        assert!(!members.is_empty(), "{name}: the strip holds tabs");
        let controls = members
            .iter()
            .map(|m| ("a tab", *m))
            .chain([("the split buttons", split)]);
        for (what, control) in controls {
            assert!(
                (midline(control) - line).abs() <= TOLERANCE,
                "{name}: {what} {control:?} is centred at {:.1}, not on New session's midline {line:.1}",
                midline(control),
            );
            checked += 1;
        }
    }
    assert!(
        checked > 0,
        "no strip state was checked, so this gate proved nothing"
    );
}
