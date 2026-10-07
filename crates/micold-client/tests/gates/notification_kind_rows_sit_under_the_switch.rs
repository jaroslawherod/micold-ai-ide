//! The Desktop notifications kind rows and the threshold field, by geometry (feature 613, T075).
//!
//! `settings_sections` reads the view's source for their order, and `layout_snapshot` records
//! where they are; the record was regenerated with the change that added them, so it compares
//! against what it was shown. This states the expected arrangement instead (S1, S5, FR-009):
//!
//! - one run of five rows in the Environment page is indented, each by one spacing step
//!   (`spacing::MD`): the four kind rows and the threshold field;
//! - the run follows the master switch's row directly, and its rows stack top to bottom without
//!   overlapping, inside the page's width;
//! - the four kind rows share one height, and the fourth row of the run, the one directly after
//!   **Long task finished** (third in `NotificationKind::ALL`), is the threshold field: the only row
//!   of another height (a labelled field with supporting text).
//!
//! **Compiled into the `layout_snapshot` binary** for the reason `containment` gives.

use crate::support::covered_states::covered_states;
use crate::support::layout::{self as lay, LayoutRecord};
use micold_core::theme::ColorScheme;
use micold_core::tokens::spacing;

const RECORDED_SCHEME: ColorScheme = ColorScheme::Light;

/// Half a pixel, matching the other gates.
const TOLERANCE: f32 = 0.5;

/// The covered state that shows the Environment page with the master switch on.
const ENVIRONMENT: &str = "settings-view-environment";

/// The direct children of the record at `parent`, in order.
fn children<'a>(records: &'a [LayoutRecord], parent: &[usize]) -> Vec<&'a LayoutRecord> {
    let mut found: Vec<&LayoutRecord> = records
        .iter()
        .filter(|r| {
            r.layer == lay::Layer::Base
                && r.path.len() == parent.len() + 1
                && r.path.starts_with(parent)
        })
        .collect();
    found.sort_by_key(|r| r.path[parent.len()]);
    found
}

/// A row is indented when its first child starts one spacing step inside it and spans the rest.
fn indented(records: &[LayoutRecord], row: &LayoutRecord) -> bool {
    children(records, &row.path).first().is_some_and(|inner| {
        (inner.x - (row.x + spacing::MD)).abs() <= TOLERANCE
            && (inner.width - (row.width - spacing::MD)).abs() <= TOLERANCE
    })
}

#[test]
fn the_kind_rows_and_the_threshold_sit_indented_under_the_master_switch() {
    let renderer = lay::renderer();
    let all = lay::cached_records(covered_states(), &renderer, RECORDED_SCHEME);
    let (_, records) = covered_states()
        .iter()
        .zip(all.iter())
        .find(|(c, _)| c.name == ENVIRONMENT)
        .unwrap_or_else(|| panic!("no covered state named {ENVIRONMENT}"));

    // Every run of consecutive indented siblings, under any parent.
    let mut runs: Vec<(Vec<usize>, usize, Vec<&LayoutRecord>)> = Vec::new();
    for parent in records.iter().filter(|r| r.layer == lay::Layer::Base) {
        let rows = children(records, &parent.path);
        let mut i = 0;
        while i < rows.len() {
            if indented(records, rows[i]) {
                let start = i;
                while i < rows.len() && indented(records, rows[i]) {
                    i += 1;
                }
                runs.push((parent.path.clone(), start, rows[start..i].to_vec()));
            } else {
                i += 1;
            }
        }
    }
    assert_eq!(
        runs.len(),
        1,
        "one run of indented rows in the Environment page, found {}: {:?}",
        runs.len(),
        runs.iter()
            .map(|(p, s, r)| (p, s, r.len()))
            .collect::<Vec<_>>()
    );
    let (column, start, run) = &runs[0];
    assert_eq!(run.len(), 5, "four kind rows and the threshold field");
    assert!(*start > 0, "the run follows a row");

    let rows = children(records, column);
    let master = rows[start - 1];
    let parent = records
        .iter()
        .find(|r| r.layer == lay::Layer::Base && &r.path == column)
        .expect("the column's own record");

    // Directly under the master switch, stacked in order, none overlapping, inside the page.
    let mut above = master;
    for row in run {
        assert!(
            row.y >= above.y + above.height - TOLERANCE,
            "row {:?} at y {} overlaps or precedes the row above, which ends at {}",
            row.path,
            row.y,
            above.y + above.height
        );
        assert!(
            (row.x - master.x).abs() <= TOLERANCE,
            "row {:?} starts where the master switch's row does; its content carries the indent",
            row.path
        );
        assert!(
            row.x + row.width <= parent.x + parent.width + TOLERANCE,
            "row {:?} stays inside the page's width",
            row.path
        );
        above = row;
    }

    // The fourth row is the threshold field: the only row whose height differs from the kinds'.
    let kind_height = run[0].height;
    for (index, row) in run.iter().enumerate() {
        if index == 3 {
            assert!(
                (row.height - kind_height).abs() > TOLERANCE,
                "the row after Long task finished is the threshold field, taller than a kind row \
                 ({} against {kind_height})",
                row.height
            );
        } else {
            assert!(
                (row.height - kind_height).abs() <= TOLERANCE,
                "kind row {index} is {} tall, the others {kind_height}",
                row.height
            );
        }
    }
}
