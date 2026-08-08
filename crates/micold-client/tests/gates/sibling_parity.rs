//! Two components of the same kind measure the same (BUG-007, SC-008e).
//!
//! The fifth gate, and the first to read two components **of one kind against each other** rather
//! than either against a figure.
//!
//! Every check before it compares a component to something written down:
//!
//! - `tokens_anatomy` compares the *constants* — and a constant only covers a component the
//!   contract names. §7.1 describes *the* app-bar action; nothing said the project switcher was
//!   one, so its 28dp target answered to no row and passed.
//! - `anatomy_size` reads one component's laid-out box against its own declared extent. The
//!   switcher's trigger declared none, being a `button` assembled at its call site.
//! - `content_placement` rasterises one component and asks where its content sits inside it. A
//!   fork's content is perfectly placed inside the fork.
//! - `panel_placement` reads two components of *different* kinds — a panel against the bar it hangs
//!   from. Two panels differing from each other by 20dp both clear the bar.
//!
//! So a fork is invisible to all of them: both copies are internally consistent, and the second
//! answers to no figure at all. The missing question is the symmetric one — the app bar has two
//! action controls and two panels hang from it, and each pair should be one component drawn twice
//! (FR-029c).
//!
//! **Compiled into the `layout_snapshot` binary** for the reason `containment` and
//! `panel_placement` give: cargo makes one process per file directly under `tests/`, and a separate
//! process cannot reach the record cache that has already resolved every covered state.

use crate::support::covered_states::covered_states;
use crate::support::layout::{self as lay, LayoutRecord};
use micold_core::theme::ColorScheme;
use micold_core::tokens::anatomy;

/// Geometry is a structural property; one scheme establishes it. The dark pass exists for colour.
const RECORDED_SCHEME: ColorScheme = ColorScheme::Light;

/// Half a pixel, matching `containment`, `panel_placement` and the text-overflow gate.
const TOLERANCE: f32 = 0.5;

/// The app bar's action row: shell → bar → row. Its children are the title, a filling spacer, then
/// one node per action added by `Toolbar::action`.
///
/// Five indices, not the six the fixture prints: a record's path is relative to the root, and
/// `path_token` prepends the root's own `0` for display.
const APP_BAR_ACTION_ROW: &[usize] = &[0, 0, 0, 0, 0];

/// The action controls on the app bar's trailing edge, in order.
///
/// Structural rather than a list of paths: anything the toolbar is given past its title and spacer
/// is an action, so an action added later is covered without anyone remembering to add it here.
fn app_bar_actions(records: &[LayoutRecord]) -> Vec<&LayoutRecord> {
    records
        .iter()
        .filter(|r| {
            r.layer == lay::Layer::Base
                && r.path.len() == APP_BAR_ACTION_ROW.len() + 1
                && r.path.starts_with(APP_BAR_ACTION_ROW)
                // Index 0 is the title, index 1 the `Length::Fill` spacer that pushes the rest to
                // the trailing edge. Everything after them was pushed by `Toolbar::action`.
                && r.path[APP_BAR_ACTION_ROW.len()] >= 2
        })
        .collect()
}

/// The glyph a control draws: the deepest, last-laid-out descendant of `root`.
///
/// A glyph is a leaf, and an icon button's leaf is its glyph — the chain is container → ripple →
/// button → text. Read this way rather than by a fixed path so the two controls may differ in how
/// many wrappers they carry and still be compared on the thing a person sees.
fn glyph_of<'a>(records: &'a [LayoutRecord], root: &LayoutRecord) -> Option<&'a LayoutRecord> {
    records
        .iter()
        .filter(|r| {
            r.layer == root.layer
                && r.path.len() > root.path.len()
                && r.path.starts_with(&root.path)
        })
        .max_by_key(|r| r.path.len())
}

/// Every anchored panel in a state: a layer's own child, smaller than the window on both axes.
/// The same rule `panel_placement` uses, and for the same reason — a panel added later is covered.
fn anchored_panels(records: &[LayoutRecord]) -> impl Iterator<Item = &LayoutRecord> {
    records.iter().filter(|r| {
        r.layer == lay::Layer::Base
            && r.path.len() == 2
            && r.path[0] >= 1
            && r.path[1] == 0
            && r.width < lay::WINDOW.width - TOLERANCE
            && r.height < lay::WINDOW.height - TOLERANCE
    })
}

/// Two figures agree within the gate's tolerance.
fn same(a: f32, b: f32) -> bool {
    (a - b).abs() <= TOLERANCE
}

/// The app bar's action controls are one component drawn twice (FR-029c, SC-008e).
///
/// Target **and** glyph, because they fail independently: a hand-built control can reach 48×48 by
/// padding a 14dp glyph, which satisfies §7.1's target row and still reads as a different control
/// beside a 24dp one.
#[test]
fn the_app_bars_action_controls_measure_the_same() {
    let renderer = lay::renderer();
    let all = lay::cached_records(covered_states(), &renderer, RECORDED_SCHEME);
    let mut differing: Vec<String> = Vec::new();

    for (covered, records) in covered_states().iter().zip(all.iter()) {
        let actions = app_bar_actions(records);
        let Some(first) = actions.first() else {
            continue;
        };
        for other in actions.iter().skip(1) {
            if !same(first.width, other.width) || !same(first.height, other.height) {
                differing.push(format!(
                    "{}: the action at {} is {:.1} × {:.1} where the one at {} is {:.1} × {:.1}",
                    covered.name,
                    lay::path_token(&other.path),
                    other.width,
                    other.height,
                    lay::path_token(&first.path),
                    first.width,
                    first.height,
                ));
                continue;
            }
            let (Some(a), Some(b)) = (glyph_of(records, first), glyph_of(records, other)) else {
                continue;
            };
            if !same(a.width, b.width) || !same(a.height, b.height) {
                differing.push(format!(
                    "{}: the action at {} draws a {:.1} × {:.1} glyph where the one at {} draws \
                     {:.1} × {:.1}",
                    covered.name,
                    lay::path_token(&other.path),
                    b.width,
                    b.height,
                    lay::path_token(&first.path),
                    a.width,
                    a.height,
                ));
            }
        }
    }

    assert!(
        differing.is_empty(),
        "{} app-bar action control(s) differ from the one beside them. Every trailing action is \
         the shared icon button at §7.1's {} × {} target (FR-029c) — a control that assembles its \
         own button, style, ripple and target answers to no contract row, which is how BUG-007 \
         shipped a 28dp trigger with a 14dp glyph next to a 48dp one with a 24dp glyph.\n  {}",
        differing.len(),
        anatomy::app_bar::ICON_TARGET,
        anatomy::app_bar::ICON_TARGET,
        differing.join("\n  "),
    );
}

/// Every panel hanging from the app bar is the same panel (FR-029c, SC-008e).
///
/// Read across states rather than within one: only one bar-anchored panel is open at a time, so a
/// within-state check would never see the pair. Identified by their top edge — a panel that begins
/// at the bar's bottom edge is anchored to the bar, which `panel_placement` already establishes is
/// where they all begin.
#[test]
fn the_panels_hanging_from_the_app_bar_are_the_same_panel() {
    let renderer = lay::renderer();
    let all = lay::cached_records(covered_states(), &renderer, RECORDED_SCHEME);
    let mut seen: Vec<(String, String, f32, f32)> = Vec::new();

    for (covered, records) in covered_states().iter().zip(all.iter()) {
        for panel in anchored_panels(records) {
            if same(panel.y, anatomy::app_bar::BOTTOM_EDGE) {
                seen.push((
                    covered.name.to_string(),
                    lay::path_token(&panel.path),
                    panel.width,
                    panel.x + panel.width,
                ));
            }
        }
    }

    let Some((first_state, first_path, first_width, first_end)) = seen.first().cloned() else {
        panic!(
            "no panel anchored to the app bar's bottom edge in any covered state — this gate has \
             nothing to compare, which means the states it reads have stopped covering the menus"
        );
    };
    let differing: Vec<String> = seen
        .iter()
        .skip(1)
        .filter(|(_, _, width, end)| !same(*width, first_width) || !same(*end, first_end))
        .map(|(state, path, width, end)| {
            format!(
                "{}: the panel at {} is {:.1} wide ending at x={:.1}, where {}'s at {} is {:.1} \
                 wide ending at x={:.1}",
                state, path, width, end, first_state, first_path, first_width, first_end,
            )
        })
        .collect();

    assert!(
        differing.is_empty(),
        "{} panel(s) hanging from the app bar differ from the first. §7.5 states one width and one \
         trailing inset for the panel of this kind, and a panel that restates either is a second \
         implementation of the first (FR-029c) — BUG-007's switcher panel was 260 against the \
         overflow menu's 240, from the same edge, so their leading edges sat 20dp apart.\n  {}",
        differing.len(),
        differing.join("\n  "),
    );
}
