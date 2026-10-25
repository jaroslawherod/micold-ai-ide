//! The Jira Filters button, its panel, the active line and the lists fit the New worktree dialog
//! (feature 1185, T014, T034, contracts/filter-ui.md §2).
//!
//! The dialog is 520 px wide and the tallest one the application shows. A **Filters** button trails
//! the issue search's row and opens a floating panel of the filter fields; while a filter is
//! chosen, **Clear filters** trails feature 1186's Basic / Advanced switch and the active line sits
//! under the row (ledger D9, D12, D15). Asked of what the renderer painted, in the covered states
//! `add-worktree-dialog-jira-filtered` (panel closed) and `add-worktree-dialog-jira-filters-panel`
//! (panel open): the button and the fields lie inside the dialog uncut, the button shares the
//! search's row, the active line keeps one line, the panel hangs under the button with its fields
//! stacked, and the open list of a filter hangs under its own field. That the dialog's footer stays
//! reachable with all of it shown is `panel_placement`'s question, which reads the same states.
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

fn covered(name: &str) -> StateUnderTest {
    let covered = covered_states()
        .iter()
        .find(|c| c.name == name)
        .unwrap_or_else(|| panic!("{name} is covered"));
    (covered.build)()
}

fn filtered() -> StateUnderTest {
    covered("add-worktree-dialog-jira-filtered")
}

fn panel() -> StateUnderTest {
    covered("add-worktree-dialog-jira-filters-panel")
}

/// The loaded Jira state with the Filters panel open.
fn loaded_with_panel() -> StateUnderTest {
    let mut under = covered("add-worktree-dialog-jira-loaded");
    filter(&mut under, FilterMsg::PanelToggled);
    under
}

fn filter(under: &mut StateUnderTest, msg: FilterMsg) {
    under
        .state
        .update(Message::WorktreeForm(Msg::JiraFilter(msg)));
}

fn assert_whole(text: &lay::Overflow) {
    assert!(
        text.natural_width <= text.allowed_width + TOLERANCE,
        "{:?} is cut: it wants {:.1} and has {:.1}",
        text.content,
        text.natural_width,
        text.allowed_width
    );
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
fn the_filters_button_clear_filters_and_the_active_line_fit_the_dialog() {
    let drawn = painted(&filtered());
    let heading = find(&drawn, "New worktree");
    let (left, _) = dialog_span();
    assert!(
        heading.origin.x >= left && heading.origin.x <= left + 48.0,
        "the dialog is not where this gate assumes: its heading starts at {:.1}",
        heading.origin.x
    );
    // The button names how many filters are chosen, whole, inside the dialog; no field shows
    // while the panel is closed.
    let filters = find(&drawn, "Filters · 2");
    assert_inside(filters);
    assert_whole(filters);
    assert!(
        drawn.iter().all(|d| d.content != "Payments"),
        "a filter field shows with the panel closed"
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
        assert_whole(text);
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
    // The Filters button trails the issue search, on its row, between the switch's row and the
    // active line (ledger D15).
    let search = drawn
        .iter()
        .find(|d| d.content.starts_with("Search by key"))
        .expect("the issue search's placeholder is painted");
    assert!(
        filters.origin.x > search.origin.x,
        "the Filters button ({:.1}) does not trail the issue search ({:.1})",
        filters.origin.x,
        search.origin.x
    );
    assert!(
        button.origin.y < filters.origin.y && line.origin.y > filters.origin.y,
        "the switch's row sits above the Filters button, the active line under it"
    );
}

#[test]
fn the_filters_panel_stacks_every_field_under_the_button() {
    let drawn = painted(&panel());
    let button = find(&drawn, "Filters · 3");
    assert_inside(button);
    assert_whole(button);
    let fields: Vec<&lay::Overflow> = ["You (Jane Doe)", "Payments", "2.4"]
        .into_iter()
        .map(|value| {
            drawn
                .iter()
                .filter(|d| d.content == value)
                .min_by(|a, b| a.origin.y.total_cmp(&b.origin.y))
                .unwrap_or_else(|| panic!("{value:?} is painted"))
        })
        .collect();
    let mut above = button.origin.y;
    for field in fields {
        assert_inside(field);
        assert_whole(field);
        assert!(
            field.origin.y > above,
            "{:?} ({:.1}) is not under what comes before it ({above:.1})",
            field.content,
            field.origin.y
        );
        above = field.origin.y;
        assert!(
            field.origin.y + field.natural_height <= lay::WINDOW.height,
            "{:?} runs past the window",
            field.content
        );
    }
}

#[test]
fn an_open_filter_list_hangs_under_its_field() {
    let mut under = panel();
    filter(&mut under, FilterMsg::Focused(FilterKind::Component));
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

/// A reporter search that finds no one says so in full, the own user still offered (T033).
#[test]
fn a_people_search_with_no_match_says_so_whole() {
    let mut under = panel();
    filter(
        &mut under,
        FilterMsg::QueryChanged(FilterKind::Reporter, "zz".into()),
    );
    let seq = under
        .state
        .worktree_form
        .pending_people_search()
        .expect("the search waits out its debounce");
    filter(&mut under, FilterMsg::PeopleSearchDue { seq });
    filter(
        &mut under,
        FilterMsg::PeopleLoaded {
            seq,
            result: Ok(Vec::new()),
        },
    );
    let drawn = painted(&under);
    let none = find(&drawn, "No project user matches; you are still offered");
    assert_whole(none);
    assert_inside(none);
    find(&drawn, "You (Jane Doe)");
}

/// A failure to load the choices shows its cause in full under the list's heading: the list is as
/// narrow as its field, so the cause wraps rather than being cut (FR-012).
#[test]
fn a_failed_choice_list_shows_its_whole_cause() {
    let mut under = loaded_with_panel();
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
    let mut under = loaded_with_panel();
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
