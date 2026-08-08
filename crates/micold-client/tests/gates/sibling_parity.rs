//! An app-bar action is a shared button, and the panels hanging from the bar are one panel
//! (BUG-007, SC-008e).
//!
//! The fifth gate, and the first to read a component against **its siblings of the same kind**
//! rather than against a figure of its own.
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
//! answers to no figure at all. The missing question is the symmetric one — the app bar's actions
//! and the panels hanging from it are each a set whose members a *component* is supposed to decide,
//! so a member matching none of the component shapes is the fork showing (FR-029c).
//!
//! **Compiled into the `layout_snapshot` binary** for the reason `containment` and
//! `panel_placement` give: cargo makes one process per file directly under `tests/`, and a separate
//! process cannot reach the record cache that has already resolved every covered state.

use crate::support::covered_states::covered_states;
use crate::support::layout::{self as lay, LayoutRecord};
use micold_core::theme::ColorScheme;
use micold_core::tokens::{anatomy, density};

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

/// The glyph a control draws: its **leading** leaf — deepest, and leftmost among the deepest.
///
/// Read structurally rather than by a fixed path, so the two shapes may carry different numbers of
/// wrappers (an icon button nests container → ripple → button → glyph; a labelled button nests
/// button → container → row → glyph, text) and still be compared on the thing a person sees.
///
/// "Leftmost among the deepest" is the part that matters: a labelled action's glyph and its label
/// are siblings at the same depth, so depth alone would just as happily hand back the **text** and
/// measure a word where the contract states a glyph size.
fn glyph_of<'a>(records: &'a [LayoutRecord], root: &LayoutRecord) -> Option<&'a LayoutRecord> {
    let deepest = records
        .iter()
        .filter(|r| {
            r.layer == root.layer
                && r.path.len() > root.path.len()
                && r.path.starts_with(&root.path)
        })
        .map(|r| r.path.len())
        .max()?;
    records
        .iter()
        .filter(|r| {
            r.layer == root.layer && r.path.len() == deepest && r.path.starts_with(&root.path)
        })
        .min_by(|a, b| a.x.total_cmp(&b.x))
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

/// Every app-bar action is one of the contract's two action shapes (FR-029c, SC-008e).
///
/// **Not "both are the same size"**, which was this test's first form and was wrong: the bar
/// carries an icon-only action (the ⋮ menu) *and* a labelled one (the project switcher, which names
/// the project it switches away from), and §7 gives those two different figures on purpose. A rule
/// that flattened them would forbid a labelled action outright.
///
/// What both shapes share is that a **component** decides them. So the rule is conformance to the
/// closed set:
///
/// - an icon-only action is §7.1's [`anatomy::app_bar::ICON_TARGET`] square, drawing §7.3's
///   [`anatomy::button::ICON_BUTTON_GLYPH`] — what `IconButton` builds; and
/// - a labelled action stands at §7.3's button height and leads with
///   [`anatomy::button::LEADING_ICON`] — what `Button::leading` builds.
///
/// Height and glyph are checked separately because they fail separately, and BUG-007 failed both
/// at once: a hand-assembled 28dp box drawing its glyph at the *label's* 14dp role matched neither
/// shape, and no per-component gate could say so — §7.1 describes *the* app-bar action, and nothing
/// said that control was one.
#[test]
fn every_app_bar_action_is_one_of_the_contracts_action_shapes() {
    let renderer = lay::renderer();
    let all = lay::cached_records(covered_states(), &renderer, RECORDED_SCHEME);
    let mut wrong: Vec<String> = Vec::new();

    for (covered, records) in covered_states().iter().zip(all.iter()) {
        for action in app_bar_actions(records) {
            let Some(glyph) = glyph_of(records, action) else {
                continue;
            };
            // Which shape this action is claiming to be: a square at §7.1's target is the icon
            // button; anything wider carries a label beside its glyph.
            let icon_only = same(action.width, anatomy::app_bar::ICON_TARGET);
            let (height, glyph_size, shape) = if icon_only {
                (
                    anatomy::app_bar::ICON_TARGET,
                    anatomy::button::ICON_BUTTON_GLYPH,
                    "an icon-only action (§7.1's target, §7.3's icon-button glyph)",
                )
            } else {
                (
                    density::BUTTON_BASE,
                    anatomy::button::LEADING_ICON,
                    "a labelled action (§7.3's button height and leading-icon slot)",
                )
            };
            if !same(action.height, height) {
                wrong.push(format!(
                    "{}: the action at {} is {} and stands {:.1}dp tall, not {:.1}",
                    covered.name,
                    lay::path_token(&action.path),
                    shape,
                    action.height,
                    height,
                ));
            }
            if !same(glyph.width, glyph_size) {
                wrong.push(format!(
                    "{}: the action at {} is {} and draws a {:.1}dp glyph, not {:.1}",
                    covered.name,
                    lay::path_token(&action.path),
                    shape,
                    glyph.width,
                    glyph_size,
                ));
            }
        }
    }

    assert!(
        wrong.is_empty(),
        "{} app-bar action(s) match neither shape §7 defines for one. A trailing action is a \
         shared button — `IconButton` where it carries no label, `Button::text(..).leading(..)` \
         where it does — and never a `button` assembled at the call site with a style, a ripple \
         and a glyph size of its own (FR-029c). That is how BUG-007 shipped a 28dp control drawing \
         its glyph at its label's 14dp role, beside a 48dp one drawing 24dp.\n  {}",
        wrong.len(),
        wrong.join("\n  "),
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
