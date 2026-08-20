//! No child of a tab is laid out narrower than the width it asked for (BUG-005, SC-010).
//!
//! The sixth gate, and the first to read a laid-out child against **what it requested** rather than
//! against a constant, against its parent's bounds, or against a sibling.
//!
//! That distinction is the whole reason it exists. A control that has been squeezed to nothing
//! satisfies every check this repository already has:
//!
//! - `tokens_anatomy` compares the *constants*. `anatomy::button::MIN_TOUCH_TARGET` is still 48 in
//!   the source, and `IconButton` still wraps itself in a box of that width — the figure is present
//!   and correct everywhere it is written down.
//! - `anatomy_size` reads a component's laid-out box against its own declared extent. A tab's
//!   children declare none; they are assembled at the call site, which is the same blind spot
//!   feature 018's BUG-007 found for the project-switcher trigger.
//! - `containment` asks whether a child escapes its parent. A 0.0-wide button contains its 0.0-wide
//!   child perfectly and escapes nothing.
//! - `panel_placement` and `sibling_parity` read *between* components. Both squeezed controls are in
//!   the right place relative to everything else; they are simply not there.
//! - `layout_snapshot` records it, and recorded it from the first regeneration — which is how it was
//!   found — but a snapshot compares against what it was shown, so once written down the zero is the
//!   expected value.
//!
//! So `mise run test` was **1914 passed, 0 failed** over a restart button laid out at 0.0dp wide and
//! a close control at 45.2, below the 48dp target. Nothing misbehaved. The missing question is the
//! one iced answers silently: a `Length::Fixed` on a composite is a budget for its children, and
//! when they ask for more than the budget iced settles the difference by shrinking the trailing
//! ones. There is no overflow and no error — the control disappears.
//!
//! # What is checked
//!
//! **Every interactive control inside a tab measures at least `MIN_TOUCH_TARGET` wide.** That is the
//! face a user meets: a button too small to hit, or gone. Feature 018 FR-027 sets the figure; 018's
//! own BUG-002 was that figure being *overwritten*, and this is the same figure being *competed
//! away*. A gate reading the constant catches neither, because the constant is intact both times.
//!
//! # What is deliberately not checked here
//!
//! "The children fit inside the tab" is the *cause*, and it cannot be asked of a layout record.
//! A first draft of this gate summed the tab's laid-out children and compared the span to the tab —
//! and that test could never fail, because the children have **already been shrunk to fit** by the
//! time they are recorded. The span equals the tab's width whether or not anyone was squeezed. It
//! passed on the very defect this file was written for, which is the "a pass that records nothing
//! looks like a pass that found nothing" shape feature 019 keeps meeting (its T041), so it was
//! removed rather than kept as reassurance.
//!
//! Answering it properly needs the widths the children *asked for*, which no record holds. That
//! question belongs to the derivation instead: `TAB_WIDTH` is built from the constants its widest
//! arrangement requires (FR-004c), and `ui/terminal.rs`'s own unit test asserts the sum. Between
//! them the two cover both faces — the derivation says the budget is big enough, and this says no
//! control ended up under its target.
//!
//! Read structurally rather than by fixed paths — a tab's children change with its instance's
//! lifecycle, and naming them by index would pin the arrangement this gate exists to let move.
//!
//! **Compiled into the `layout_snapshot` binary** for the reason `containment` gives: cargo makes
//! one process per file directly under `tests/`, and a separate process cannot reach the record
//! cache that has already resolved every covered state.

use crate::support::covered_states::covered_states;
use crate::support::layout::{self as lay, LayoutRecord};
use micold_core::theme::ColorScheme;
use micold_core::tokens::anatomy;

/// Geometry is a structural property; one scheme establishes it. The dark pass exists for colour.
const RECORDED_SCHEME: ColorScheme = ColorScheme::Light;

/// Half a pixel, matching `containment`, `panel_placement` and `sibling_parity`.
const TOLERANCE: f32 = 0.5;

/// The anchors naming a tab strip in the covered states that draw one.
///
/// **Two of them**, since feature 026 FR-002b pinned the AI tab outside the scrolling region: the
/// scrolling members are one strip and the pinned tab is another, and they are two nodes in the bar
/// rather than one. A gate that knew only the first would have stopped covering the tab this whole
/// feature adds, silently and while still passing — which is the shape feature 019 keeps meeting.
const STRIP_ANCHORS: &[&str] = &["terminal.tabs", "terminal.tabs.pinned"];

/// Every tab strip in a state, or an empty list for a state without one.
///
/// Taken from the anchors rather than from constant paths, so this gate covers any state that
/// registers a strip — including one added later — and covers none that does not, silently and
/// correctly. A state with no strip is not a failure; it is a screen without tabs.
fn strip_paths(covered: &'static lay::CoveredState) -> Vec<&'static [usize]> {
    covered
        .anchors
        .iter()
        .filter(|a| STRIP_ANCHORS.contains(&a.name))
        .map(|a| a.path)
        .collect()
}

/// The tabs: the strip's immediate children.
fn tabs<'r>(records: &'r [LayoutRecord], strip: &[usize]) -> Vec<&'r LayoutRecord> {
    records
        .iter()
        .filter(|r| {
            r.layer == lay::Layer::Base
                && r.path.len() == strip.len() + 1
                && r.path.starts_with(strip)
        })
        .collect()
}

/// Everything laid out inside `tab`, excluding the tab itself.
fn descendants<'r>(records: &'r [LayoutRecord], tab: &LayoutRecord) -> Vec<&'r LayoutRecord> {
    records
        .iter()
        .filter(|r| {
            r.layer == tab.layer && r.path.len() > tab.path.len() && r.path.starts_with(&tab.path)
        })
        .collect()
}

/// The interactive controls inside a tab.
///
/// A tab is a button wrapping a column — the active indicator, then a row of children. That row's
/// direct children are the leading spacer, the label, the close control, and (before BUG-005 moved
/// it out) a restart button. A **control is a direct child of that row as tall as the row itself**:
/// a button fills its row's cross axis, and a text label does not. In the recorded tabs the row is
/// 21.0 tall, the close and restart controls are 21.0, the label is 16.0 and the leading spacer is
/// 0.0 — so the rule separates them cleanly without naming any of them.
///
/// Structural on purpose, and it took two attempts. The first recognised a control by an absolute
/// height floor and by "no child shares my width", which let through the 20dp pill *inside* the
/// close button (its parent is the 48dp touch box, so their widths differ) and the 0dp indicator
/// spacer (a stray `|| width == 0.0` swallowed every zero-width node). Both are noise: the pill is
/// supposed to be smaller than its target, and the indicator is a rule, not a control. Asserting on
/// them would have made the gate fail for reasons that are not defects, which is how a gate gets
/// weakened rather than fixed.
///
/// Direct children only. `IconButton` nests container → ripple → button → glyph, each a different
/// box, and the one that claims the touch target is the outermost.
fn nested_controls<'r>(records: &'r [LayoutRecord], tab: &LayoutRecord) -> Vec<&'r LayoutRecord> {
    let inner = descendants(records, tab);
    let Some(row) = content_row(records, tab) else {
        return Vec::new();
    };
    let children: Vec<_> = inner
        .iter()
        .filter(|c| c.path.len() == row.path.len() + 1 && c.path.starts_with(&row.path))
        .copied()
        .collect();
    children
        .iter()
        // **Never the label.** A tab's content row is `[leading slot, label, trailing slot]` —
        // three children, by construction, since feature 026 promoted the tab into a component
        // (`material/tab.rs`). The middle one is what the tab shows and is not a control; the two
        // slots are where controls go, and they are what this gate is about.
        //
        // Excluded by **position** rather than by the height rule below, because the height rule
        // cannot see it in the case that matters. It works on a terminal tab by accident: the label
        // is 16.0dp tall against a 21.0dp row, and the row is 21 because the close control in the
        // trailing slot is. On the **AI tab** there is no close control — FR-004, the trailing slot
        // is reserved and empty — so the label is the tallest thing in the row and the row is
        // exactly as tall as the label. Every tab whose only visible child is its label therefore
        // read as a tab holding one under-target control, which is not a defect and not what this
        // gate was built to find.
        //
        // Naming the position is not a weakening. Before the promotion the arrangement inside a tab
        // was a call-site detail and had to be discovered; it is now the component's own contract,
        // and `a_tabs_content_sits_on_its_tabs_midline` below fails if the row stops having three
        // children in that order.
        .enumerate()
        .filter(|(i, _)| *i != children.len() / 2)
        .map(|(_, c)| c)
        .filter(|c| (c.height - row.height).abs() < TOLERANCE)
        .copied()
        .collect()
}

/// A tab's content row: the deepest node inside it holding more than one child of its own.
///
/// Found structurally for the same reason the controls are — the arrangement inside a tab is the
/// thing under test and must be free to change.
fn content_row<'r>(records: &'r [LayoutRecord], tab: &LayoutRecord) -> Option<&'r LayoutRecord> {
    let inner = descendants(records, tab);
    inner
        .iter()
        .filter(|r| {
            inner
                .iter()
                .filter(|c| c.path.len() == r.path.len() + 1 && c.path.starts_with(&r.path))
                .count()
                > 1
        })
        .max_by_key(|r| r.path.len())
        .copied()
}

/// FR-004b, the face a user meets: the selection mark spans the **whole** tab, edge to edge.
///
/// A tab is a region of the strip, and the indicator marks the region — so a rule that stops short
/// of either edge reads as a mark *near* a tab rather than as the tab being selected. It stopped
/// short for exactly as long as the tab had padding: `spacing::SM` on each side left the content
/// column, which the rule fills, 16dp narrower than the tab it belonged to.
///
/// Geometry, not a value, because the value was never wrong — `Divider::horizontal` has always
/// filled what it was given. What was wrong was what it was given, and only the laid-out boxes say
/// that. It is the same shape as this gate's other assertion one axis over: a figure intact in the
/// source, competed away by the box around it.
#[test]
fn the_active_indicator_spans_its_whole_tab() {
    let renderer = lay::renderer();
    let all = lay::cached_records(covered_states(), &renderer, RECORDED_SCHEME);
    let mut failures = Vec::new();
    let mut checked = 0usize;

    for (covered, records) in covered_states().iter().zip(all.iter()) {
        for strip in strip_paths(covered) {
            for tab in tabs(records, strip) {
                // The rule is the content column's first or last child (the edge it is drawn on is
                // `IndicatorEdge`'s business, and this gate must not pin it): recognised as the
                // descendant of a tab whose **height is the indicator's thickness**. Structural, so
                // the levels between a tab and its rule stay free to change.
                let Some(rule) = descendants(records, tab).into_iter().find(|r| {
                    (r.height - anatomy::tab::INDICATOR).abs() < TOLERANCE && r.width > TOLERANCE
                }) else {
                    // An inactive tab draws a transparent spacer of the same height and no width,
                    // which the `width > TOLERANCE` above excludes. There is nothing to span.
                    continue;
                };
                checked += 1;
                // Flush against **an** edge, top or bottom: which one is `IndicatorEdge`'s
                // business and this gate must not pin it. The rule floated ~8dp clear of both for
                // as long as the tab's column was content-sized inside a fixed-height button —
                // the same defect as the horizontal inset, on the other axis, and invisible for
                // the same reason: every node was exactly where its own layout said it was.
                let flush_top = (rule.y - tab.y).abs() <= TOLERANCE;
                let flush_bottom =
                    ((rule.y + rule.height) - (tab.y + tab.height)).abs() <= TOLERANCE;
                if !flush_top && !flush_bottom {
                    let name = lay::anchor_for(covered.anchors, &tab.path)
                        .map(|a| a.name.to_string())
                        .unwrap_or_else(|| lay::path_token(&tab.path));
                    failures.push(format!(
                        "  {} — {name}: the rule sits {:.1}..{:.1} in a tab {:.1}..{:.1}, touching \
                         neither edge — {:.1}dp clear of the top and {:.1}dp of the bottom",
                        covered.name,
                        rule.y,
                        rule.y + rule.height,
                        tab.y,
                        tab.y + tab.height,
                        rule.y - tab.y,
                        (tab.y + tab.height) - (rule.y + rule.height),
                    ));
                }
                if (rule.x - tab.x).abs() > TOLERANCE || (rule.width - tab.width).abs() > TOLERANCE
                {
                    let name = lay::anchor_for(covered.anchors, &tab.path)
                        .map(|a| a.name.to_string())
                        .unwrap_or_else(|| lay::path_token(&tab.path));
                    failures.push(format!(
                        "  {} — {name}: the rule spans {:.1}..{:.1} in a tab {:.1}..{:.1}, so it \
                         stops {:.1}dp short of the tab it marks",
                        covered.name,
                        rule.x,
                        rule.x + rule.width,
                        tab.x,
                        tab.x + tab.width,
                        tab.width - rule.width,
                    ));
                }
            }
        }
    }

    assert!(
        failures.is_empty(),
        "a tab's active indicator does not span its tab (feature 012 FR-004b):\n{}\n\nThe \
         indicator marks the **region** the strip is divided into, not something inside it. A rule \
         inset from both edges reads as a mark near a tab rather than as the tab being selected, \
         and the usual cause is padding on the tab: the rule fills the content column, and padding \
         is what makes that column narrower than the tab.",
        failures.join("\n")
    );
    assert!(
        checked > 0,
        "no active indicator was located, so this gate proved nothing — every covered state's \
         tabs are inactive, or the rule stopped being recognisable by its thickness, and both look \
         exactly like a pass that found nothing"
    );
}

/// SC-010, the face a user meets: a control too narrow to hit, or gone.
#[test]
fn every_control_inside_a_tab_holds_its_touch_target() {
    let renderer = lay::renderer();
    let all = lay::cached_records(covered_states(), &renderer, RECORDED_SCHEME);
    let mut failures = Vec::new();
    let mut checked = 0usize;

    for (covered, records) in covered_states().iter().zip(all.iter()) {
        for strip in strip_paths(covered) {
        for tab in tabs(records, strip) {
            for control in nested_controls(records, tab) {
                checked += 1;
                if control.width + TOLERANCE < anatomy::button::MIN_TOUCH_TARGET {
                    let name = lay::anchor_for(covered.anchors, &control.path)
                        .map(|a| a.name.to_string())
                        .unwrap_or_else(|| lay::path_token(&control.path));
                    failures.push(format!(
                        "  {} — {name} is {:.1}dp wide, under the {:.1}dp minimum interactive \
                         target; its tab is {:.1}dp",
                        covered.name,
                        control.width,
                        anatomy::button::MIN_TOUCH_TARGET,
                        tab.width,
                    ));
                }
            }
        }
        }
    }

    assert!(
        failures.is_empty(),
        "a control inside a tab was laid out under the minimum interactive target (feature 012 \
         SC-010, feature 018 FR-027):\n{}\n\niced settles a shortfall inside a `Length::Fixed` \
         parent by shrinking its trailing children, so this is what a tab too narrow for its \
         contents looks like from the outside: no overflow, no error, a control that is simply not \
         there. The figure is intact in the source both when it is overwritten (018 BUG-002) and \
         when it is competed away (012 BUG-005), which is why this reads the laid-out box.",
        failures.join("\n")
    );
    assert!(
        checked > 0,
        "no control inside any tab was checked, so this gate proved nothing. Either no covered \
         state registers a `{STRIP_ANCHORS:?}` anchor, or the way a nested control is recognised has \
         drifted from what the tabs actually draw. A pass that inspects nothing is \
         indistinguishable from a pass that found nothing — the same defect feature 019's overlay \
         pass had (its T041)."
    );
}

/// FR-004a and SC-008: a tab's content sits on the tab's own midline, whether or not it is active.
///
/// The centring clause and the no-reflow clause are the same measurement taken twice. A tab's column
/// holds the indicator above the content row; the active tab's indicator fills the tab, an inactive
/// tab's placeholder has no width. If the column shrinks to its widest child, the active tab measures
/// the whole content box and centres its row inside it while every inactive tab measures only the row
/// and pins it to the leading edge. The label is then off-centre on every tab but one, and it *slides*
/// across as the active tab changes — under the pointer, between a press and its release, which is
/// exactly what SC-008 forbids.
///
/// The 2026-08-19 visual pass measured that slide at 4.6dp in both schemes. It was 0.6dp before this
/// bugfix and invisible: the slack is half the difference between the tab's content box and its row,
/// so widening `TAB_WIDTH` 128 → 136 for FR-004c's derivation multiplied an existing defect by eight
/// without touching the code that caused it. Two visual passes had read the strip and called it
/// stable, both correctly at the magnification they used.
///
/// Asked per tab rather than by comparing tabs to each other: "every tab is wrong in the same way"
/// would pass a difference test, and it is still a defect.
#[test]
fn a_tabs_content_sits_on_its_tabs_midline() {
    let renderer = lay::renderer();
    let all = lay::cached_records(covered_states(), &renderer, RECORDED_SCHEME);
    let mut failures = Vec::new();
    let mut checked = 0usize;

    for (covered, records) in covered_states().iter().zip(all.iter()) {
        for strip in strip_paths(covered) {
        for tab in tabs(records, strip) {
            let Some(row) = content_row(records, tab) else {
                continue;
            };
            checked += 1;
            let offset = (row.x + row.width / 2.0) - (tab.x + tab.width / 2.0);
            if offset.abs() > TOLERANCE {
                let name = lay::anchor_for(covered.anchors, &tab.path)
                    .map(|a| a.name.to_string())
                    .unwrap_or_else(|| lay::path_token(&tab.path));
                failures.push(format!(
                    "  {} — {name}: its content row is {offset:+.1}dp off the tab's midline (row \
                     {:.1}..{:.1} in a tab {:.1}..{:.1})",
                    covered.name,
                    row.x,
                    row.x + row.width,
                    tab.x,
                    tab.x + tab.width,
                ));
            }
        }
        }
    }

    assert!(
        failures.is_empty(),
        "a tab's content is not centred on the tab (feature 012 FR-004a, SC-008):\n{}\n\nAn \
         off-centre row is also a moving one: the offset is half the tab's leftover width, and an \
         inactive tab has leftover width only because its indicator placeholder has none. The \
         label therefore shifts by that much the moment its tab becomes active, under the pointer.",
        failures.join("\n")
    );
    assert!(
        checked > 0,
        "no tab's content row was located, so this gate proved nothing — the same shape as the \
         other assertion here: a pass that inspects nothing looks exactly like a pass that found \
         nothing."
    );
}
