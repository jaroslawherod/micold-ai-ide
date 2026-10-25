//! The Jira filter row, its list and the active line fit the New worktree dialog (feature 1185,
//! T014, contracts/filter-ui.md §2).
//!
//! The dialog is 520 px wide and the tallest one the application shows. The filter fields trail
//! the issue search's row; while a filter is chosen, **Clear filters** trails feature 1186's Basic /
//! Advanced switch and the active line sits under the row (ledger D9, D12). Asked of what the
//! renderer painted, in the covered state `add-worktree-dialog-jira-filtered`: the field lies
//! inside the dialog, the button shares the switch's row, the active line keeps one line with
//! nothing cut, and the open list of a filter hangs under its own field. That the dialog's footer stays reachable with all of it shown is
//! `panel_placement`'s question, which reads the same covered state.
//!
//! **Compiled into the `layout_snapshot` binary**, beside the other gates that read covered states.

use crate::support::covered_states::covered_states;
use crate::support::layout::{self as lay, StateUnderTest};
use micold_client::app::Message;
use micold_client::features::worktree_form::jira_filters::FilterMsg;
use micold_client::features::worktree_form::Msg;
use micold_core::github::IssueLoadError;
use micold_core::jira_filter::FilterKind;

/// The dialog's fixed width.
const DIALOG_WIDTH: f32 = 520.0;
/// Half a pixel, as the other gates allow.
const TOLERANCE: f32 = 0.5;

fn filtered() -> StateUnderTest {
    let covered = covered_states()
        .iter()
        .find(|c| c.name == "add-worktree-dialog-jira-filtered")
        .expect("the filtered Jira state is covered");
    (covered.build)()
}

fn painted(under: &StateUnderTest) -> Vec<lay::Overflow> {
    let mut renderer = lay::renderer();
    lay::painted_text_settled_at(lay::view_of(under), &mut renderer, lay::WINDOW).0
}

fn find<'a>(drawn: &'a [lay::Overflow], content: &str) -> &'a lay::Overflow {
    drawn
        .iter()
        .find(|d| d.content == content)
        .unwrap_or_else(|| panic!("{content:?} is painted"))
}

/// The dialog's horizontal extent: centred in the window, as the modal places it.
fn dialog_span() -> (f32, f32) {
    let left = (lay::WINDOW.width - DIALOG_WIDTH) / 2.0;
    (left, left + DIALOG_WIDTH)
}

fn assert_inside(text: &lay::Overflow) {
    let (left, right) = dialog_span();
    let end = text.origin.x + text.natural_width.min(text.allowed_width);
    assert!(
        text.origin.x >= left - TOLERANCE && end <= right + TOLERANCE,
        "{:?} spans {:.1}..{:.1}, outside the dialog's {left:.1}..{right:.1}",
        text.content,
        text.origin.x,
        end,
    );
}

#[test]
fn the_filter_row_clear_filters_and_the_active_line_fit_the_dialog() {
    let drawn = painted(&filtered());
    let heading = find(&drawn, "New worktree");
    let (left, _) = dialog_span();
    assert!(
        heading.origin.x >= left && heading.origin.x <= left + 48.0,
        "the dialog is not where this gate assumes: its heading starts at {:.1}",
        heading.origin.x
    );
    // Each field shows its chosen value inside the dialog, the two side by side on one row.
    let value = find(&drawn, "Payments");
    assert_inside(value);
    let version = find(&drawn, "2.4");
    assert_inside(version);
    assert!(
        (value.origin.y - version.origin.y).abs() <= 2.0 && version.origin.x > value.origin.x,
        "Fix version ({:.1}, {:.1}) does not follow Component ({:.1}, {:.1}) on its row",
        version.origin.x,
        version.origin.y,
        value.origin.x,
        value.origin.y
    );
    // A capped list's caption follows the line in its paragraph, on one line.
    let line = drawn
        .iter()
        .find(|d| {
            d.content
                .starts_with("Filtered by component Payments, fix version 2.4")
        })
        .expect("the active line is painted");
    let button = find(&drawn, "Clear filters");
    for text in [line, button] {
        assert_inside(text);
        assert!(
            text.natural_width <= text.allowed_width + TOLERANCE,
            "{:?} is cut: it wants {:.1} and has {:.1}",
            text.content,
            text.natural_width,
            text.allowed_width
        );
    }
    assert!(
        line.natural_height <= button.natural_height + TOLERANCE,
        "the active line and the caption wrap to {:.1}px, past one line ({:.1}): a second line is \
         one the dialog lacks",
        line.natural_height,
        button.natural_height
    );
    // Clear filters trails the Basic / Advanced switch (feature 1186), on its row (ledger D12).
    let middle = |t: &lay::Overflow| t.origin.y + t.natural_height / 2.0;
    let basic = find(&drawn, "Basic");
    assert!(
        (middle(basic) - middle(button)).abs() <= 2.0,
        "the switch ({:.1}) and Clear filters ({:.1}) are not on one row",
        middle(basic),
        middle(button)
    );
    assert!(
        button.origin.x > basic.origin.x,
        "Clear filters trails the switch"
    );
    assert!(
        button.origin.y < value.origin.y && line.origin.y > value.origin.y,
        "the switch's row sits above the filter row, the active line under it"
    );
}

#[test]
fn an_open_filter_list_hangs_under_its_field() {
    let mut under = filtered();
    under
        .state
        .update(Message::WorktreeForm(Msg::JiraFilter(FilterMsg::Focused(
            FilterKind::Component,
        ))));
    let drawn = painted(&under);
    let any = find(&drawn, "Any component");
    let field = drawn
        .iter()
        .filter(|d| d.content == "Payments")
        .min_by(|a, b| a.origin.y.total_cmp(&b.origin.y))
        .expect("the field's value is painted");
    assert!(
        any.origin.y > field.origin.y,
        "the list's first row ({:.1}) is not under the field ({:.1})",
        any.origin.y,
        field.origin.y
    );
    assert!(
        (any.origin.x - field.origin.x).abs() <= 48.0,
        "the list ({:.1}) is not anchored at its field ({:.1})",
        any.origin.x,
        field.origin.x
    );
    assert_inside(any);
}

/// A failure to load the choices shows its cause in full under the list's heading: the list is as
/// narrow as its field, so the cause wraps rather than being cut (FR-012).
#[test]
fn a_failed_choice_list_shows_its_whole_cause() {
    let mut under = (covered_states()
        .iter()
        .find(|c| c.name == "add-worktree-dialog-jira-loaded")
        .expect("the loaded Jira state is covered")
        .build)();
    let filter = |under: &mut StateUnderTest, msg| {
        under
            .state
            .update(Message::WorktreeForm(Msg::JiraFilter(msg)))
    };
    filter(&mut under, FilterMsg::Focused(FilterKind::Component));
    let (_, seq) = under
        .state
        .worktree_form
        .awaited_choices()
        .expect("the first focus asks for the components");
    filter(
        &mut under,
        FilterMsg::ComponentsLoaded {
            seq,
            result: Err(IssueLoadError::TimedOut),
        },
    );
    let drawn = painted(&under);
    find(&drawn, "Couldn't load the components.");
    let retry = find(&drawn, "Retry");
    let cause: Vec<_> = drawn
        .iter()
        .filter(|d| d.content.contains("atlassian"))
        .collect();
    assert!(!cause.is_empty(), "the cause is painted");
    for line in cause {
        assert!(
            line.natural_width <= line.allowed_width + TOLERANCE,
            "the cause {:?} is cut: it wants {:.1} and has {:.1}",
            line.content,
            line.natural_width,
            line.allowed_width
        );
        assert!(
            line.origin.y < retry.origin.y,
            "the cause comes before Retry"
        );
    }
}

/// A filter with no choices says why in full: the reason row wraps in the field-wide list rather
/// than being cut to "This project ha…" (spec Edge Cases, "Empty choices"; visual pass M2).
#[test]
fn an_empty_choice_list_shows_its_whole_reason() {
    let mut under = (covered_states()
        .iter()
        .find(|c| c.name == "add-worktree-dialog-jira-loaded")
        .expect("the loaded Jira state is covered")
        .build)();
    let filter = |under: &mut StateUnderTest, msg| {
        under
            .state
            .update(Message::WorktreeForm(Msg::JiraFilter(msg)))
    };
    filter(&mut under, FilterMsg::Focused(FilterKind::FixVersion));
    let (_, seq) = under
        .state
        .worktree_form
        .awaited_choices()
        .expect("the first focus asks for the versions");
    filter(
        &mut under,
        FilterMsg::VersionsLoaded {
            seq,
            result: Ok(Vec::new()),
        },
    );
    let drawn = painted(&under);
    let reason = find(&drawn, "This project has no versions");
    assert!(
        reason.natural_width <= reason.allowed_width + TOLERANCE,
        "the reason is cut: it wants {:.1} and has {:.1}",
        reason.natural_width,
        reason.allowed_width
    );
    assert_inside(reason);
    find(&drawn, "No versions");
}
