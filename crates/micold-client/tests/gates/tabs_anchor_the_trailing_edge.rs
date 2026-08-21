//! The tab strip and its two trailing controls sit against the bar's trailing edge, in order
//! (feature 027 FR-002, FR-003).
//!
//! # The requirement, and why geometry is the only place it can be read
//!
//! Feature 027 deleted the mode toggle and made the strip the sole switcher between a session's
//! panes. What replaced the toggle is an *arrangement*: the terminal tabs, then the "+", then the
//! AI tab, all pushed to the trailing end, so that the two controls a user reaches for by muscle
//! memory — "open another one" and "go back to the assistant" — stay put while the tabs grow
//! leftward away from them.
//!
//! None of that is visible to a unit test. `ui/terminal.rs` can only be read for the order it
//! *pushes* children in, and that is not the question: a row whose members are pushed in the right
//! order still lays them out wrong if one of them fills, and the strip is inside a horizontal
//! `Scrollable`, where `Length::Fill` resolves against an unbounded limit and pushes its sibling
//! clean out of the viewport. The arrangement is a fact about coordinates, so it is asserted about
//! coordinates.
//!
//! **Compiled into the `layout_snapshot` binary**, like every gate here, to share the record cache.

use crate::support::covered_states::covered_states;
use crate::support::layout::{self as lay, LayoutRecord};
use micold_core::theme::ColorScheme;

/// Geometry is a structural property; one scheme establishes it.
const RECORDED_SCHEME: ColorScheme = ColorScheme::Light;

/// Half a pixel, matching every other gate here.
const TOLERANCE: f32 = 0.5;

/// How far short of flush-right the strip may settle.
///
/// The covered state's viewport width is a rounded integer — the running application reports it
/// through `Scrollable::on_viewport_resize`, and `app::scroll_offset_px` rounds — so the slack
/// derived from it can miss the true figure by up to half a pixel either way. Two of those, and
/// nothing else: the reading this gate exists to catch is not 1dp of drift, it is a strip laid out
/// flush against the **other** end, which in the fixture's 1280dp window is hundreds.
const SLACK_TOLERANCE: f32 = 2.0;

/// The record an anchor names, or `None` in a state that does not register it.
fn at<'r>(
    covered: &'static lay::CoveredState,
    records: &'r [LayoutRecord],
    name: &str,
) -> Option<&'r LayoutRecord> {
    let anchor = covered.anchors.iter().find(|a| a.name == name)?;
    records
        .iter()
        .find(|r| r.layer == lay::Layer::Base && r.path == anchor.path)
}

/// The AI tab is the bar's **last** child, and the "+" the one before it (FR-002).
///
/// Read as coordinates rather than as indices: an index says where the view pushed them, and this
/// gate exists because pushing is not laying out. Two states register both anchors — a bar with
/// room to spare and one that overflows — and the order has to hold in each.
#[test]
fn the_ai_tab_is_the_last_thing_in_the_bar() {
    let renderer = lay::renderer();
    let all = lay::cached_records(covered_states(), &renderer, RECORDED_SCHEME);
    let mut checked = 0usize;

    for (covered, records) in covered_states().iter().zip(all.iter()) {
        let (Some(bar), Some(plus), Some(ai)) = (
            at(covered, records, "terminal.bottom_bar"),
            at(covered, records, "terminal.add_instance")
                .or_else(|| at(covered, records, "terminal.bottom_bar.add_instance")),
            at(covered, records, "terminal.tabs.pinned"),
        ) else {
            continue;
        };

        assert!(
            plus.x + plus.width <= ai.x + TOLERANCE,
            "{}: the \"+\" is not before the AI tab — \"+\" ends at {:.1}dp and the AI tab starts \
             at {:.1} (feature 027 FR-002). The trailing group reads tabs, \"+\", AI tab; a user \
             who reaches for the rightmost tab must land on the assistant.",
            covered.name,
            plus.x + plus.width,
            ai.x,
        );
        assert!(
            ai.x + ai.width <= bar.x + bar.width + TOLERANCE,
            "{}: the AI tab runs past the bar's trailing edge — it ends at {:.1}dp and the bar at \
             {:.1}. Nothing in iced reports an overflow; the tab is simply not where it is drawn \
             to be pressed.",
            covered.name,
            ai.x + ai.width,
            bar.x + bar.width,
        );
        checked += 1;
    }

    assert!(
        checked > 0,
        "no state registered both trailing controls, so this gate proved nothing — a pass that \
         inspects nothing looks exactly like a pass that found nothing"
    );
}

/// The terminal tabs hug the trailing end of their own viewport (FR-003).
///
/// The half that a right-alignment can silently lose. With the slack computed from the wrong
/// quantity — or from a viewport the state has not learned yet — the strip lands flush *left*
/// instead, leaving a gap between the last terminal tab and the "+". It still renders, still passes
/// every containment and touch-target rule, and still reads to a user as tabs that drift about the
/// bar as instances open and close.
///
/// Measured against the **scrolling region** rather than against the "+", because the two questions
/// separate once the tabs overflow: a strip wider than its viewport is *supposed* to run past it,
/// which is what scrolling means, and comparing it to the "+" would report that as an overlap. Where
/// there is no slack the property is vacuous and the state is skipped — with a count, so that a
/// fixture in which no state has any slack fails rather than passes quietly.
#[test]
fn the_terminal_tabs_meet_the_trailing_controls() {
    let renderer = lay::renderer();
    let all = lay::cached_records(covered_states(), &renderer, RECORDED_SCHEME);
    let mut checked = 0usize;

    for (covered, records) in covered_states().iter().zip(all.iter()) {
        let (Some(bar), Some(strip)) = (
            at(covered, records, "terminal.bottom_bar"),
            at(covered, records, "terminal.tabs"),
        ) else {
            continue;
        };
        // The scrolling region: the bar row's own child that contains the strip. Found by prefix
        // rather than by index, so the several wrappers between them — fade, stack, layer,
        // viewport — need not be counted here and may be restructured without touching this gate.
        let row_depth = bar.path.len() + 1;
        let Some(region) = records.iter().find(|r| {
            r.layer == lay::Layer::Base
                && r.path.len() == row_depth + 1
                && strip.path.starts_with(&r.path)
        }) else {
            continue;
        };

        // Overflowing: no slack to spend, the strip fills the region and scrolls. Nothing to say.
        if strip.width >= region.width - SLACK_TOLERANCE {
            continue;
        }

        let gap = strip.x - region.x;
        let available = region.width - strip.width;
        assert!(
            gap >= available - SLACK_TOLERANCE,
            "{}: the tabs are not against the trailing edge of their viewport — {gap:.1}dp of \
             leading gap where the arrangement calls for {available:.1} (feature 027 FR-003).\n\n\
             The strip is {:.1}dp wide in a {:.1}dp region, so all but {:.1} of that region is \
             slack; spending none of it is a strip laid out flush left, which still contains its \
             tabs, still meets every touch target, and still moves under the user every time an \
             instance opens.",
            covered.name,
            strip.width,
            region.width,
            strip.width,
        );
        assert!(
            strip.x + strip.width <= region.x + region.width + TOLERANCE,
            "{}: the tabs are pushed past the trailing edge of their own viewport — the strip ends \
             at {:.1}dp and the region at {:.1}. The slack overshot; nothing in iced reports that, \
             the last tab is simply clipped.",
            covered.name,
            strip.x + strip.width,
            region.x + region.width,
        );
        checked += 1;
    }

    assert!(
        checked > 0,
        "no covered state left the strip any slack, so this gate proved nothing — every state \
         either has no tabs or overflows, and a strip that cannot be pushed cannot be shown to have \
         been. A state needs `tab_strip_viewport_width` set to what the bar hands out."
    );
}
