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
    let button = find(&drawn, "Filters · 4");
    assert_inside(button);
    assert_whole(button);
    // All four, in the order FR-001 names them (M4).
    let fields: Vec<&lay::Overflow> = ["You (Jane Doe)", "backend", "Payments", "2.4"]
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

/// The label list opens under the Label field, its labels whole (M4, T041).
#[test]
fn the_label_list_hangs_under_its_field() {
    let mut under = panel();
    filter(&mut under, FilterMsg::Focused(FilterKind::Label));
    let drawn = painted(&under);
    let any = find(&drawn, "Any label");
    let field = drawn
        .iter()
        .filter(|d| d.content == "backend")
        .min_by(|a, b| a.origin.y.total_cmp(&b.origin.y))
        .expect("the field's value is painted");
    assert!(
        any.origin.y > field.origin.y && (any.origin.x - field.origin.x).abs() <= 48.0,
        "the list's first row ({:.1}, {:.1}) is not under its field ({:.1}, {:.1})",
        any.origin.x,
        any.origin.y,
        field.origin.x,
        field.origin.y
    );
    for label in ["Any label", "payments", "ui"] {
        let row = find(&drawn, label);
        assert_inside(row);
        assert_whole(row);
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

/// The pixels of `area` in an RGBA frame [`lay::WINDOW`] wide.
fn pixels_in(frame: &[u8], area: iced::Rectangle) -> Vec<[u8; 4]> {
    let width = lay::WINDOW.width as usize;
    let (x0, y0) = (area.x as usize, area.y as usize);
    let (x1, y1) = (x0 + area.width as usize, y0 + area.height as usize);
    (y0..y1)
        .flat_map(|y| (x0..x1).map(move |x| (y * width + x) * 4))
        .map(|i| [frame[i], frame[i + 1], frame[i + 2], frame[i + 3]])
        .collect()
}

/// Visual pass M3 B2/B3 (ledger D16): a field's list opened inside the Filters panel is a layer
/// floated by a layer, and the window must still show the dialog's surface, the panel over it and
/// the list over both — composed by the runtime, not by this harness's own overlay walk.
#[test]
fn a_list_opened_in_the_panel_keeps_the_dialog_and_the_panel_drawn() {
    let shut = filtered();
    let closed = panel();
    let mut open = panel();
    filter(&mut open, FilterMsg::Focused(FilterKind::Component));
    let before = lay::runtime_frame(|| lay::view_of(&closed), lay::WINDOW);
    let after = lay::runtime_frame(|| lay::view_of(&open), lay::WINDOW);

    // Where the panel's top row, Reporter, sits: under the Filters button, above the Component
    // field whose list opens (`the_filters_panel_stacks_every_field_under_the_button`).
    let drawn = painted(&closed);
    let reporter = find(&drawn, "You (Jane Doe)");
    let row = iced::Rectangle {
        x: reporter.origin.x - 8.0,
        y: reporter.origin.y - 4.0,
        width: 160.0,
        height: 24.0,
    };
    assert_eq!(
        pixels_in(&before, row),
        pixels_in(&after, row),
        "the panel's Reporter row changed when Component's list opened"
    );
    assert_ne!(
        pixels_in(
            &lay::runtime_frame(|| lay::view_of(&shut), lay::WINDOW),
            row
        ),
        pixels_in(&after, row),
        "the panel is not drawn over the dialog"
    );

    let (left, right) = dialog_span();
    let dialog = iced::Rectangle {
        x: left + 4.0,
        y: 80.0,
        width: right - left - 8.0,
        height: 600.0,
    };
    let dark = |frame: &[u8]| {
        let all = pixels_in(frame, dialog);
        let dark = all
            .iter()
            .filter(|[r, g, b, _]| u32::from(*r) + u32::from(*g) + u32::from(*b) < 180)
            .count();
        dark as f32 / all.len() as f32
    };
    assert!(
        dark(&after) < 0.1,
        "{:.0}% of the dialog is dark with the list open",
        dark(&after) * 100.0
    );
    let drawn_open = painted(&open);
    let any = find(&drawn_open, "Any component");
    let first_row = iced::Rectangle {
        x: any.origin.x,
        y: any.origin.y,
        width: 80.0,
        height: 14.0,
    };
    assert_ne!(
        pixels_in(&before, first_row),
        pixels_in(&after, first_row),
        "the list's first row is not drawn"
    );
}
