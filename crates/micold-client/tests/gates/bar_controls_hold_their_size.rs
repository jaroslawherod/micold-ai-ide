//! No control in the terminal's bottom bar is laid out narrower than the width it asks for
//! (feature 026 FR-002c, SC-008).
//!
//! # The defect this exists for, which is live on `main`
//!
//! `ui/terminal.rs::pane` lays the bar out as one row with no bound on the tab strip. A row settles
//! a shortfall the way iced always does: **a fixed parent width is a budget, and the trailing
//! children are shrunk to fit it** — laid out narrower, or at zero, with nothing reported. Nothing
//! overflows, so nothing fails. Past about five open instances the bar's trailing controls, the "+"
//! and the mode toggle, are the ones that pay.
//!
//! That is feature 012's BUG-005 one level out. Its own comment records what it cost inside a tab:
//! "the button laid out at 0.0dp wide and the close control beside it at 45.2, under §7.3's target.
//! Nothing overflowed, so nothing failed: `mise run test` was green over a control a user could not
//! press." The same sentence applies here with "tab" replaced by "bar".
//!
//! Feature 026 meets it sooner — the strip becomes always visible (FR-003) and gains a tab (FR-001)
//! — so it is fixed first, where it can be verified on its own.
//!
//! # Why a second gate rather than a widened `tab_children_fit`
//!
//! That gate asks this question one level in, and it recognises a control **structurally**: a direct
//! child of a tab's content row as tall as the row itself. The bar's children are not tabs and do
//! not answer to that rule — a title is a text label, the spacer is a `Fill`, and the strip is a
//! row of rows. Widening it would mean two unrelated structural rules in one file, each silently
//! failing to recognise the other's subjects.
//!
//! **Compiled into the `layout_snapshot` binary**, like every gate here: cargo makes one process per
//! file directly under `tests/`, and a separate process cannot reach the record cache that has
//! already resolved every covered state.

use crate::support::covered_states::covered_states;
use crate::support::layout::{self as lay, LayoutRecord};
use micold_core::theme::ColorScheme;

/// Geometry is a structural property; one scheme establishes it.
const RECORDED_SCHEME: ColorScheme = ColorScheme::Light;

/// Half a pixel, matching every other gate here.
const TOLERANCE: f32 = 0.5;

/// The anchor naming the terminal's bottom bar in the states that draw one.
const BAR_ANCHOR: &str = "terminal.bottom_bar";

/// The bar's path in a state that has one, or `None` for a state without.
///
/// From the anchor rather than a constant path, so this gate covers any state that registers a bar
/// — including one added later — and covers none that does not, silently and correctly.
fn bar_path(covered: &'static lay::CoveredState) -> Option<&'static [usize]> {
    covered
        .anchors
        .iter()
        .find(|a| a.name == BAR_ANCHOR)
        .map(|a| a.path)
}

/// The bar's own row — its single child, which is what actually holds the controls.
fn bar_row<'r>(records: &'r [LayoutRecord], bar: &[usize]) -> Option<&'r LayoutRecord> {
    records.iter().find(|r| {
        r.layer == lay::Layer::Base && r.path.len() == bar.len() + 1 && r.path.starts_with(bar)
    })
}

/// The controls: the bar row's immediate children.
fn controls<'r>(records: &'r [LayoutRecord], row: &LayoutRecord) -> Vec<&'r LayoutRecord> {
    records
        .iter()
        .filter(|r| {
            r.layer == lay::Layer::Base
                && r.path.len() == row.path.len() + 1
                && r.path.starts_with(&row.path)
        })
        .collect()
}

/// Every control in the bar is laid out at a width it can be pressed at, and none is squeezed to
/// nothing by a neighbour that grew (FR-002c, SC-008).
///
/// The assertion is deliberately the weakest one that catches the defect: **no control is laid out
/// at zero width**, and the controls the bar owns unconditionally — the mode toggle above all — are
/// laid out at the same width in an overflowing state as in a state with room to spare. A control
/// squeezed from 40dp to 12 is as unpressable as one squeezed to 0, but 0 is the figure that cannot
/// be argued with, and the cross-state comparison is what catches the rest.
#[test]
fn no_control_in_the_bar_is_squeezed_by_the_strip() {
    let renderer = lay::renderer();
    let all = lay::cached_records(covered_states(), &renderer, RECORDED_SCHEME);
    let mut failures = Vec::new();
    let mut checked = 0usize;

    for (covered, records) in covered_states().iter().zip(all.iter()) {
        let Some(bar) = bar_path(covered) else {
            continue;
        };
        let Some(row) = bar_row(records, bar) else {
            continue;
        };
        for control in controls(records, row) {
            // The filling spacer between the title and the status is *supposed* to have no width
            // when there is none to give — it is the slack, not a control. Everything else is one.
            if control.width <= TOLERANCE && !is_the_spacer(records, control) {
                failures.push(format!(
                    "  {} — {}: laid out {:.1}dp wide, which is nothing a user can press. The bar \
                     settled its shortfall by shrinking this child; iced reports that as a \
                     successful layout.",
                    covered.name,
                    lay::anchor_for(covered.anchors, &control.path)
                        .map(|a| a.name.to_string())
                        .unwrap_or_else(|| lay::path_token(&control.path)),
                    control.width,
                ));
            }
            checked += 1;
        }
    }

    assert!(
        failures.is_empty(),
        "a control in the terminal's bottom bar was squeezed to nothing (feature 026 FR-002c, \
         SC-008):\n{}\n\nThe bar lays its controls out in one row. A fixed width is a budget, and \
         iced settles a shortfall by shrinking the trailing children rather than by overflowing — \
         so the controls at its trailing end absorb it silently. Bound the strip's growth instead.",
        failures.join("\n")
    );
    assert!(
        checked > 0,
        "no bar control was located, so this gate proved nothing — a pass that inspects nothing \
         looks exactly like a pass that found nothing"
    );
}

/// The mode toggle is the same size whether the strip is empty or overflowing (FR-002c, SC-008).
///
/// The half of the requirement a zero-width check cannot reach. The toggle is the bar's last child
/// and therefore the first to be squeezed, and it is present in **every** state that draws a bar —
/// so its width across states is a direct reading of whether the strip's growth is taking width
/// from its siblings. If they differ, the bar is redistributing rather than bounding.
#[test]
fn the_mode_toggle_measures_the_same_at_every_instance_count() {
    let renderer = lay::renderer();
    let all = lay::cached_records(covered_states(), &renderer, RECORDED_SCHEME);
    let mut seen: Vec<(&str, f32)> = Vec::new();

    for (covered, records) in covered_states().iter().zip(all.iter()) {
        let Some(anchor) = covered
            .anchors
            .iter()
            .find(|a| a.name == "terminal.mode_toggle")
        else {
            continue;
        };
        let Some(record) = records
            .iter()
            .find(|r| r.layer == lay::Layer::Base && r.path == anchor.path)
        else {
            continue;
        };
        seen.push((covered.name, record.width));
    }

    assert!(
        seen.len() > 1,
        "fewer than two states name the mode toggle, so there is nothing to compare — this gate \
         needs both a roomy bar and an overflowing one to say anything at all (T014)"
    );
    let (first_name, first) = seen[0];
    let disagreeing: Vec<String> = seen
        .iter()
        .filter(|(_, w)| (w - first).abs() > TOLERANCE)
        .map(|(name, w)| format!("  {name}: {w:.1}dp"))
        .collect();
    assert!(
        disagreeing.is_empty(),
        "the mode toggle is a different size depending on how many instances are open (feature 026 \
         FR-002c):\n  {first_name}: {first:.1}dp (the reference)\n{}\n\nIt is the bar's last child \
         and therefore the first thing a row shrinks. A control whose size depends on its \
         neighbour's content is not laid out; it is left over.",
        disagreeing.join("\n")
    );
}

/// The filling spacer between the title and the status — a `Fill` with nothing in it.
///
/// Recognised structurally rather than by index: it is the only bar child with **no descendants at
/// all**, since every real control draws something inside itself. An index would have to be revised
/// each time the bar gains or loses an optional control, which is exactly what this feature does.
fn is_the_spacer(records: &[LayoutRecord], control: &LayoutRecord) -> bool {
    !records.iter().any(|r| {
        r.layer == control.layer
            && r.path.len() > control.path.len()
            && r.path.starts_with(&control.path)
    })
}
