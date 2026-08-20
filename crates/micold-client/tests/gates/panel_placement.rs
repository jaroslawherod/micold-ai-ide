//! No floating panel is laid out over the app bar, and none extends past the window (BUG-003,
//! SC-008c).
//!
//! The fourth gate, and the first to read a relationship between **two independent components**.
//!
//! Every check this application already has is scoped to one component, or to a parent and its own
//! child:
//!
//! - `tokens_anatomy` compares the *constants*. §7.1 says 64 and always did; BUG-003's panels were
//!   positioned by a 52 that appears in no contract at all.
//! - `anatomy_size` reads one component's laid-out box under two limits. Both panels are exactly
//!   the size §7.5 asks for. Being in the wrong place is not a size.
//! - `content_placement` rasterises one component and asks where its content sits *inside* it. A
//!   panel drawn across the app bar has perfectly placed content.
//! - `containment` asks whether a child escapes its own parent. A panel's parent is the overlay
//!   host, which is the whole window; nothing escapes anything.
//! - `layout_snapshot` records the panel and always did — `1032, 52, 240 × 264`, in nearly every
//!   state, from the day the fixture landed. It was green throughout, because a snapshot compares
//!   against what it was shown and a defect older than the fixture *is* what it was shown.
//!
//! So the missing question is the one between components: the panel hangs below the app bar — does
//! it begin below the app bar? It must be **asserted** rather than recorded, for the reason the
//! last bullet gives. T093 had to correct the same class of blindness after BUG-002.
//!
//! # What counts as a panel
//!
//! `ui::view` returns a stack: the shell, then one layer per floating surface (`cdk::overlay`). A
//! layer is window-sized; what it holds is either the surface's panel, positioned by the anchor, or
//! a full-window node — a dismissal backdrop, or a closed menu that is still laid out because it has
//! to outlive the state that opened it in order to fade out.
//!
//! A node smaller than the window on **both** axes is therefore an anchored panel: a menu, a
//! switcher, a dialog, a snackbar. Full-window nodes are backdrops and scrims, which are supposed to
//! cover the bar. That distinction is structural rather than a list of paths, so a panel added later
//! is covered without anyone remembering to add it here.
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

/// Half a pixel, matching `containment` and the text-overflow gate.
const TOLERANCE: f32 = 0.5;

/// Every anchored panel in a state: a layer's own child, smaller than the window on both axes.
///
/// `pub(crate)` for `context_menu_anchor`, which asks a different question of the same set (BUG-008).
///
/// Records are depth-first with full paths, so a layer is a root child (`path == [i]`, `i >= 1`,
/// index 0 being the shell) and its panel is `[i, 0]`.
pub(crate) fn anchored_panels(records: &[LayoutRecord]) -> impl Iterator<Item = &LayoutRecord> {
    records.iter().filter(|r| {
        r.layer == lay::Layer::Base
            && r.path.len() == 2
            && r.path[0] >= 1
            && r.path[1] == 0
            && r.width < lay::WINDOW.width - TOLERANCE
            && r.height < lay::WINDOW.height - TOLERANCE
    })
}

/// A panel anchored below the app bar begins below the app bar (FR-029a, SC-008c).
#[test]
fn no_floating_panel_is_laid_out_over_the_app_bar() {
    let renderer = lay::renderer();
    let all = lay::cached_records(covered_states(), &renderer, RECORDED_SCHEME);
    let mut over_the_bar: Vec<String> = Vec::new();

    for (covered, records) in covered_states().iter().zip(all.iter()) {
        for panel in anchored_panels(records) {
            if panel.y < anatomy::app_bar::BOTTOM_EDGE - TOLERANCE {
                over_the_bar.push(format!(
                    "{}: the panel at {} starts at y={:.1}, which is {:.1}px inside the app bar \
                     (the bar and its divider end at {:.1})",
                    covered.name,
                    lay::path_token(&panel.path),
                    panel.y,
                    anatomy::app_bar::BOTTOM_EDGE - panel.y,
                    anatomy::app_bar::BOTTOM_EDGE,
                ));
            }
        }
    }

    assert!(
        over_the_bar.is_empty(),
        "{} floating panel(s) are laid out across the app bar, so each is drawn over the bar it \
         hangs from and over the trigger it was opened from. A panel's offset must be derived from \
         `anatomy::app_bar::BOTTOM_EDGE` rather than stated as its own constant (FR-029a) — a \
         restated figure is a copy nothing links to its original, which is what BUG-003 was, twice \
         over.\n  {}",
        over_the_bar.len(),
        over_the_bar.join("\n  "),
    );
}

/// A panel is inside the window it floats in — the other half of "anchored correctly".
///
/// Not a restatement of `containment`: a panel's parent is the window-sized overlay layer, so a
/// panel *is* contained by its parent right up until it is not, and the escape it would report is
/// the symptom rather than the anchor that caused it. Nothing clamps the toolbar-anchored panels at
/// all, and the two context menus are anchored at a literal point (T107), so this is the check that
/// notices when a panel grows past what its corner of the window can hold.
#[test]
fn no_floating_panel_extends_past_the_window() {
    let renderer = lay::renderer();
    let all = lay::cached_records(covered_states(), &renderer, RECORDED_SCHEME);
    let mut escaping: Vec<String> = Vec::new();

    for (covered, records) in covered_states().iter().zip(all.iter()) {
        for panel in anchored_panels(records) {
            let overhangs = [
                ("left", -panel.x),
                ("top", -panel.y),
                ("right", panel.x + panel.width - lay::WINDOW.width),
                ("bottom", panel.y + panel.height - lay::WINDOW.height),
            ];
            for (edge, past) in overhangs {
                if past > TOLERANCE {
                    escaping.push(format!(
                        "{}: the panel at {} hangs {:.1}px past the window's {} edge",
                        covered.name,
                        lay::path_token(&panel.path),
                        past,
                        edge,
                    ));
                }
            }
        }
    }

    assert!(
        escaping.is_empty(),
        "{} floating panel(s) extend past the window, so part of each is unreachable:\n  {}",
        escaping.len(),
        escaping.join("\n  "),
    );
}

/// The gate can fail, shown against a panel placed the way BUG-003's were.
///
/// Built from a synthetic record rather than by reverting a component, so it still holds once both
/// panels are fixed — a check that cannot be shown to fail is decoration (`anatomy_size`'s
/// `the_gate_can_fail`, same argument).
#[test]
fn the_gate_can_fail() {
    let panel_at_the_old_offset = vec![LayoutRecord {
        path: vec![2, 0],
        layer: lay::Layer::Base,
        x: 1032.0,
        y: 52.0,
        width: 240.0,
        height: 264.0,
    }];

    let panel = anchored_panels(&panel_at_the_old_offset)
        .next()
        .expect("a 240 × 264 node at [2, 0] is an anchored panel and must be recognised as one");

    assert!(
        panel.y < anatomy::app_bar::BOTTOM_EDGE - TOLERANCE,
        "a panel at y=52 is 13px inside a bar whose bottom edge is at {}, and this gate reported it \
         as correctly placed — it cannot see the defect it exists for",
        anatomy::app_bar::BOTTOM_EDGE,
    );
}
