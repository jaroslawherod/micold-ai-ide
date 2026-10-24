//! The review's source control, drawn: core's rows and label (`review::source_menu`) as a
//! [`MenuSelect`] (feature 1272, contract §3).
//!
//! The one place a [`Row`] becomes a [`MenuSelectRow`], so the review header and the showcase pose
//! the same control. What each stop sends is the caller's: `on_stop` maps it to a message.

use crate::ui::material::{MenuSelect, MenuSelectRow, PullRequestRow, Text, ToggleChip, TypeRole};
use micold_core::review::source_menu::{
    self, Kind, Label, MenuInput, Row, Stop, FILES, POSSIBLY_STALE, PULL_REQUESTS, WORKING_TREE,
};
use micold_core::tokens::Roles;

/// The Committed chip's text.
pub const COMMITTED: &str = "Committed";
/// The Uncommitted chip's text.
pub const UNCOMMITTED: &str = "Uncommitted";

/// The open list's rows for `rows`, each stop sending `on_stop(stop)` when pressed.
pub fn menu_rows<'a, M: Clone + 'a>(
    rows: &[Row<'a>],
    on_stop: &impl Fn(Stop) -> M,
    r: Roles,
) -> Vec<MenuSelectRow<'a, M>> {
    let chip = |label: &'static str, kind: Kind, chip: source_menu::Chip| {
        ToggleChip::new(label, on_stop(Stop::Kind(kind)), r)
            .active(chip.on)
            .disabled(!chip.available)
    };
    rows.iter()
        .map(|row| match *row {
            Row::WorkingTree {
                chosen,
                committed,
                uncommitted,
                note,
            } => MenuSelectRow::option_with_chips(
                Text::new(WORKING_TREE, TypeRole::Body, r),
                chosen,
                on_stop(Stop::WorkingTree),
                vec![
                    chip(COMMITTED, Kind::Committed, committed),
                    chip(UNCOMMITTED, Kind::Uncommitted, uncommitted),
                ],
                note,
            ),
            Row::Heading => MenuSelectRow::heading(PULL_REQUESTS),
            Row::Note(text) => MenuSelectRow::note(text),
            Row::Stale => MenuSelectRow::note(POSSIBLY_STALE),
            Row::PullRequest { pr, chosen } => {
                let mut content = PullRequestRow::<M>::new(
                    pr.number,
                    pr.title.as_str(),
                    crate::features::changes::pr_mark(pr.state),
                    &r,
                );
                if let Some(mark) = crate::features::changes::check_mark(pr.checks) {
                    content = content.checks(mark);
                }
                MenuSelectRow::option(content, chosen, on_stop(Stop::PullRequest(pr.number)))
            }
            Row::Files { on } => MenuSelectRow::option(
                Text::new(FILES, TypeRole::Body, r),
                on,
                on_stop(Stop::Files),
            ),
        })
        .collect()
}

/// The control for `input`: its label, its rows, and the keyboard over its stops. `highlight` is a
/// stop index; `Enter` sends `on_pick` when the highlighted stop is enabled.
pub struct Control<M> {
    /// Sent by a press on the trigger, `Enter` or `Space` on it.
    pub on_toggle: M,
    /// Sent by a press outside the open list, `Esc` or `Tab`.
    pub on_dismiss: M,
    /// Sent by `Enter` on the highlighted stop.
    pub on_pick: M,
}

/// The source control for `input`, open while `open` with the keyboard on `highlight`, sending
/// `on_stop` for a pressed row or chip and `on_move` for an arrow key.
#[allow(clippy::too_many_arguments)]
pub fn control<'a, M: Clone + 'a>(
    input: &MenuInput<'a>,
    open: bool,
    highlight: Option<usize>,
    messages: Control<M>,
    on_stop: impl Fn(Stop) -> M,
    on_move: impl Fn(micold_core::typeahead::Direction) -> M + 'a,
    tooltip: String,
    r: Roles,
) -> MenuSelect<'a, M> {
    let rows = source_menu::rows(input);
    let stops = source_menu::stops(&rows);
    let enabled = highlight
        .and_then(|i| stops.get(i))
        .is_some_and(|(_, enabled)| *enabled);
    let Label { fixed, title } = source_menu::label(input);
    MenuSelect::new(
        fixed,
        menu_rows(&rows, &on_stop, r),
        open,
        messages.on_toggle,
        messages.on_dismiss,
        r,
    )
    .title(title)
    .tooltip(tooltip)
    .keyboard(highlight, stops.len(), enabled)
    .on_move(on_move)
    .on_pick(messages.on_pick)
}
