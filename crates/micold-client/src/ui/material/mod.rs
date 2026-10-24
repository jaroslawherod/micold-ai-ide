//! `material` — the shared, reusable UI component library (Constitution Principle VIII).
//!
//! All custom widgets/components live here and mimic Angular Material: a flat toolbar, an
//! overflow menu, a segmented/tree navigation, icon buttons, tooltips, and the [`fade`] /
//! [`expand`] animation wrappers. Features MUST reuse or extend these rather than fork bespoke
//! one-off widgets. Every component is theme-aware (draws from the active [`Roles`]) and
//! cross-platform. This module is the living catalog Principle VIII refers to.
//!
//! [`Roles`]: micold_core::tokens::Roles
//!
//! Feature 1262 split the catalog in two. The generic Material building blocks — buttons, fields,
//! menus, motion, the styling layer, the CDK behaviour layer and the icon vocabulary — live in the
//! `micold-material` crate, which names no Micold domain type. What remains here composes them into
//! Micold's own widgets (sessions, overview cards, diffs, pull requests, the terminal pane), and the
//! glob re-export below keeps every `material::X` path the client already names.

pub use micold_material::*;

mod activity_badge;
/// Whether a component lays out at the size its anatomy entry states, or grows into whatever room
/// it is offered. The constants gate reads the numbers, the snapshot records the geometry, and
/// `content_placement` reads the drawing — none of them asks whether the box is the stated size
/// (BUG-002).
#[cfg(test)]
mod anatomy_size;
/// §7.3's horizontal paddings, read off the laid-out button. `anatomy_size` covers the height and
/// the touch target; a button is content-sized across, so no size check can see its inset.
#[cfg(test)]
mod button_anatomy;
mod card_grid;
mod card_lift;
mod code_block;
mod command_palette;
/// A component read against the container it is *rendered in*, not against the surfaces its own
/// role was enumerated for. The first check here that reads a **composition** rather than the
/// token set (BUG-009).
#[cfg(test)]
mod composition_contrast;
mod connection_banner;
/// Where a component puts its content inside a height taller than that content. In-crate for the
/// same reason as `form_field_anatomy`, and the first check that reads what a component *drew*
/// rather than the constants it was built from (BUG-001).
#[cfg(test)]
mod content_placement;
/// §7.4's spatial figures, read off a laid-out dialog. Every one of them is a *gap*, which no
/// size check can see.
#[cfg(test)]
mod dialog_anatomy;
mod diff_view;
/// An expanded section stays inside the scrollable that holds it, in pixels (feature 1092).
#[cfg(test)]
mod expand_clipping;
/// A field reporting and taking the keyboard, driven rather than posed — the gap the anatomy
/// modules structurally cannot see (BUG-003).
#[cfg(test)]
mod field_focus;
/// `FormField`'s composition and chrome, checked in-crate because `material` is `pub(crate)` and a
/// `FormField` cannot be constructed from `tests/` at all.
#[cfg(test)]
mod form_field_anatomy;
mod host_picker;
mod inline_diff;
mod markdown_view;
/// §7.5's *spatial* figures — the item's inset, the panel's padding, the leading glyph, and what
/// sits between two items. `anatomy_size` reads sizes; these are positions, and nothing read them
/// (BUG-003).
#[cfg(test)]
mod menu_anatomy;
mod menu_select;
mod overview_card;
mod picker;
/// The transition both lists arrive and leave by, asserted against **each other** and against
/// §6.3 rather than each against the contract. In-crate for the same reason as `picker_parity`
/// (feature 022, FR-021, SC-007).
#[cfg(test)]
mod picker_motion;
/// The two pickers' lists, compared against **each other** rather than each against the contract.
/// In-crate because neither control is reachable from `tests/`; see the module's own docs for why
/// the comparison is of geometry (feature 022, SC-001).
#[cfg(test)]
mod picker_parity;
/// Whether a press on an unavailable row stops at it — the half of "not pickable" that no
/// state-level test can see, and the one that closed the add-worktree form (016 BUG-002, FR-035).
#[cfg(test)]
mod picker_press;
mod progress_bar;
mod prompt_card;
mod pull_request_indicator;
mod pull_request_row;
mod queue_row;
mod review_bar;
mod review_comment;
mod review_panel_row;
/// The ripple's clipping, checked in rasterised pixels rather than in geometry. In-crate for the
/// same reason as the snapshots above.
#[cfg(test)]
mod ripple_clipping;
/// The reference scene's ripple, and the rule it presses by. In-crate for the same reason.
#[cfg(test)]
mod ripple_pulse;
mod select;
/// The select's own anatomy, and the two behaviours nothing outside it can observe — its indicator
/// answering for itself, and its highlight seeded from the current choice. In-crate for the same
/// reason as `form_field_anatomy` (feature 022, FR-013).
#[cfg(test)]
mod select_anatomy;
mod side_sheet;
mod split_view;
mod state_icon;
mod status_chip;
#[cfg(test)]
mod style_shape;
#[cfg(test)]
mod style_snapshot;
pub(crate) mod terminal_pane;
/// The filled field's anatomy, checked in-crate — `material` is `pub`.
#[cfg(test)]
mod text_field_anatomy;
pub(crate) mod toleration_list;
mod tree_view;
mod typeahead;
mod usage_indicator;
pub use activity_badge::{ActivityBadge, BadgeEmphasis};
pub use card_grid::{CardGrid, Zoom};
pub use code_block::CodeBlock;
pub use command_palette::CommandPalette;
pub use connection_banner::ConnectionBanner;
pub use diff_view::{
    DiffLayout, DiffView, MARKER_WIDTH as DIFF_MARKER_WIDTH, ROW_HEIGHT as DIFF_ROW_HEIGHT,
};
pub use host_picker::HostPicker;
pub use inline_diff::InlineDiff;
pub use markdown_view::MarkdownView;
pub use menu_select::{MenuSelect, MenuSelectRow};
pub(crate) use overview_card::{palette as card_palette, state_icon as card_state_icon};
pub use overview_card::{CardPrRow, OverviewCard, PrTone};
pub use picker::Row as TypeaheadRow;
pub use picker::PICKER_HIGHLIGHT;
pub use progress_bar::ProgressBar;
pub use prompt_card::PromptCard;
pub use pull_request_indicator::{CheckMark, PrMark, PullRequestIndicator};
pub use pull_request_row::PullRequestRow;
pub use queue_row::QueueRow;
pub use review_bar::ReviewBar;
pub use review_comment::{CardState, ReviewCommentCard};
pub use review_panel_row::{status_label, ReviewPanelRow, HEIGHT as PANEL_ROW_HEIGHT};
pub use select::Select;
pub use side_sheet::SideSheet;
pub use split_view::{pane_focus_mark, pane_notice_fill, SplitView};
pub use state_icon::StateIcon;
pub use status_chip::StatusChip;
#[cfg(test)]
pub(crate) use terminal_pane::scrollbar_metrics;
pub use terminal_pane::target_offset_delta;
pub use terminal_pane::GridSizeReporter;
pub use terminal_pane::TerminalPane;
pub use toleration_list::TolerationList;
pub use tree_view::{Selection, TreeItem, TreeView};
pub use typeahead::Typeahead;
pub use usage_indicator::UsageIndicator;

/// The tooltip of the unread count on the switcher's button (feature 039, FR-023): what the
/// number counts, which the button itself has no room to say.
pub fn unread_total_tooltip(unread: usize) -> String {
    if unread == 1 {
        "1 unread session in other projects".to_string()
    } else {
        format!("{unread} unread sessions in other projects")
    }
}

#[cfg(test)]
mod unread_total_tests {
    use super::unread_total_tooltip;

    /// U158 (FR-023).
    #[test]
    fn the_unread_totals_tooltip_names_the_other_projects() {
        assert_eq!(
            unread_total_tooltip(1),
            "1 unread session in other projects"
        );
        assert_eq!(
            unread_total_tooltip(3),
            "3 unread sessions in other projects"
        );
    }
}
