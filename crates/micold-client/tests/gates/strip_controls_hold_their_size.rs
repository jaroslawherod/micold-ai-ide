//! No member or fixed control of the tab strip is squeezed (feature 1126 S9, FR-010, SC-004;
//! carried over from feature 026 FR-002c's `bar_controls_hold_their_size`).
//!
//! The strip lays its controls out in one row, and a fixed width is a **budget**: iced settles a
//! shortfall by shrinking the trailing children rather than by overflowing, so a control at the
//! strip's trailing end can be laid out at a fraction of its size, or at zero, with nothing reported.
//! The scrolling region is the strip's one flexible member; this gate reads what every member and
//! fixed control was actually given against what it asked for, at every member count.
//!
//! Compiled into the `layout_snapshot` binary beside the other geometry gates.

use crate::support::covered_states::{covered_states, STRIP_MEMBERS};
use crate::support::layout::{self as lay, LayoutRecord};
use micold_client::features::session_strip::StripModel;
use micold_core::theme::ColorScheme;

const RECORDED_SCHEME: ColorScheme = ColorScheme::Light;
const TOLERANCE: f32 = 0.5;

/// The record an anchor named `name` points at in a covered state.
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

/// The immediate base children of `parent`.
fn children<'r>(records: &'r [LayoutRecord], parent: &LayoutRecord) -> Vec<&'r LayoutRecord> {
    records
        .iter()
        .filter(|r| {
            r.layer == lay::Layer::Base
                && r.path.len() == parent.path.len() + 1
                && r.path.starts_with(&parent.path)
        })
        .collect()
}

/// S9: every member is laid out at its own fixed width — session tab, shell tab or "+" — and the
/// row holds exactly the model's members.
#[test]
fn every_member_is_laid_out_at_its_own_width() {
    let renderer = lay::renderer();
    let all = lay::cached_records(covered_states(), &renderer, RECORDED_SCHEME);
    let mut checked = 0usize;
    for (name, count) in STRIP_MEMBERS {
        let (covered, records) = covered_states()
            .iter()
            .zip(all.iter())
            .find(|(c, _)| c.name == *name)
            .unwrap_or_else(|| panic!("the covered state {name} exists"));
        let model = StripModel::of(&(covered.build)().state).expect("a session is shown");
        let row = at(covered, records, "session.strip.tabs")
            .unwrap_or_else(|| panic!("{name}: the strip's members are anchored"));
        let laid = children(records, row);
        assert_eq!(laid.len(), *count, "{name}: the keyed row's members");
        assert_eq!(model.members.len(), *count, "{name}: the model's members");
        for (record, member) in laid.iter().zip(&model.members) {
            checked += 1;
            assert!(
                (record.width - member.width()).abs() < TOLERANCE,
                "{name}: {:?} laid out {:.1}dp wide against its fixed {:.1}dp — the strip settled \
                 a shortfall by shrinking a member instead of scrolling it (FR-010)",
                member.key(),
                record.width,
                member.width(),
            );
        }
    }
    assert!(checked > 0, "no strip member was located");
}

/// S9, FR-010: the status cluster, **New session** and the split buttons are each laid out wider
/// than nothing, and New session and the splits measure the same at every member count.
#[test]
fn the_fixed_controls_hold_their_size_at_every_member_count() {
    let renderer = lay::renderer();
    let all = lay::cached_records(covered_states(), &renderer, RECORDED_SCHEME);
    let mut seen: Vec<(&str, f32, f32)> = Vec::new();
    for (covered, records) in covered_states().iter().zip(all.iter()) {
        let Some(strip) = at(covered, records, "session.strip") else {
            continue;
        };
        let row = children(records, strip);
        let row = row.first().expect("the strip's surface holds its row");
        for control in children(records, row) {
            assert!(
                control.width > TOLERANCE,
                "{}: strip child {} laid out {:.1}dp wide, which is nothing a user can press",
                covered.name,
                lay::path_token(&control.path),
                control.width,
            );
        }
        let new_session = at(covered, records, "session.strip.new_session")
            .unwrap_or_else(|| panic!("{}: New session is anchored", covered.name));
        let split = at(covered, records, "session.strip.split")
            .unwrap_or_else(|| panic!("{}: the split buttons are anchored", covered.name));
        seen.push((covered.name, new_session.width, split.width));
    }
    assert!(
        seen.len() >= STRIP_MEMBERS.len(),
        "fewer strip states than member counts: {seen:?}"
    );
    let (first, new_session, split) = seen[0];
    for (name, n, s) in &seen {
        assert!(
            (n - new_session).abs() < TOLERANCE && (s - split).abs() < TOLERANCE,
            "{name}: New session {n:.1}dp and splits {s:.1}dp against {first}'s \
             {new_session:.1}/{split:.1}dp — a fixed control whose size depends on how many tabs \
             are open is not laid out; it is left over"
        );
    }
}
