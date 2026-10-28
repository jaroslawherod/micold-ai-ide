//! The GitHub Filters button, its panel, the chips and the active line fit the New worktree dialog
//! (feature 1400, T027, contracts/selector-ui.md §5).
//!
//! The dialog is 520 px wide and the tallest one the application shows. A **Filters** button trails
//! the row heading the picker and opens a floating panel with the Include and Exclude label fields,
//! the chosen labels as chips under each; while a filter is chosen, **Clear filters** sits before
//! it on that row and the active line under the search (research R13; ledger M2 deviation: not
//! beside the search, which would narrow the search's list). Asked of
//! what the renderer painted, in the covered states `add-worktree-dialog-github-filtered` (panel
//! closed) and `add-worktree-dialog-github-filters-panel` (panel open, six labels excluded). That
//! no floating panel leaves the window is `panel_placement`'s scan, which reads the same states;
//! here the dialog's footer is asked for too.
//!
//! **Compiled into the `layout_snapshot` binary**, beside the other gates that read covered states.

use crate::support::covered_states::covered_states;
use crate::support::github_picker::{ADVANCED_QUERY, LABELS};
use crate::support::layout::{self as lay, StateUnderTest};

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

fn dialog_span() -> (f32, f32) {
    let left = (lay::WINDOW.width - DIALOG_WIDTH) / 2.0;
    (left, left + DIALOG_WIDTH)
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

fn assert_in_window(text: &lay::Overflow) {
    assert!(
        text.origin.y >= 0.0 && text.origin.y + text.natural_height <= lay::WINDOW.height,
        "{:?} ({:.1}..{:.1}) runs past the window",
        text.content,
        text.origin.y,
        text.origin.y + text.natural_height
    );
}

#[test]
fn the_filters_button_clear_filters_and_the_active_line_fit_the_dialog() {
    let drawn = painted(&covered("add-worktree-dialog-github-filtered"));
    let filters = find(&drawn, "Filters · 2");
    let clear = find(&drawn, "Clear filters");
    for text in [filters, clear] {
        assert_inside(text);
        assert_whole(text);
    }
    // No field shows while the panel is closed.
    assert!(
        drawn.iter().all(|d| d.content != "Exclude labels"),
        "a label field shows with the panel closed"
    );
    // Clear filters and the Filters button share the row heading the picker, trailing it.
    let middle = |t: &lay::Overflow| t.origin.y + t.natural_height / 2.0;
    assert!(
        (middle(filters) - middle(clear)).abs() <= 2.0 && clear.origin.x < filters.origin.x,
        "Clear filters ({:.1}, {:.1}) and Filters ({:.1}, {:.1}) are not one row, in that order",
        clear.origin.x,
        middle(clear),
        filters.origin.x,
        middle(filters)
    );
    let (left, right) = dialog_span();
    assert!(
        filters.origin.x > (left + right) / 2.0,
        "the Filters button ({:.1}) does not trail its row",
        filters.origin.x
    );
    // The statement under that row names the filter; the active line, with the cap caption in its
    // paragraph, keeps one line further down, under the search.
    let statement = drawn
        .iter()
        .find(|d| {
            d.content
                .starts_with("Reads issues of o/r from GitHub, sending the filter")
        })
        .expect("the statement names the filter");
    assert_inside(statement);
    let line = drawn
        .iter()
        .find(|d| {
            d.content
                .starts_with("Excluding labels in-progress, blocked")
        })
        .expect("the active line is painted");
    assert_inside(line);
    assert!(
        line.natural_height <= clear.natural_height + TOLERANCE,
        "the active line wraps to {:.1}px, past one line ({:.1})",
        line.natural_height,
        clear.natural_height
    );
    assert!(
        filters.origin.y < statement.origin.y && statement.origin.y < line.origin.y,
        "the Filters row, the statement and the active line are not in that order"
    );
    // The footer stays reachable with all of it shown (the tallest GitHub form).
    for action in ["Create", "Cancel"] {
        let button = find(&drawn, action);
        assert_in_window(button);
        assert!(
            button.origin.y > line.origin.y,
            "{action} is not under the picker"
        );
    }
}

#[test]
fn the_panel_stacks_its_fields_and_wraps_six_chips_inside_it() {
    let drawn = painted(&covered("add-worktree-dialog-github-filters-panel"));
    let button = find(&drawn, "Filters · 6");
    assert_inside(button);
    assert_whole(button);
    let include = drawn
        .iter()
        .filter(|d| d.content == "Include labels")
        .min_by(|a, b| a.origin.y.total_cmp(&b.origin.y))
        .expect("the Include field is painted");
    let exclude = drawn
        .iter()
        .filter(|d| d.content == "Exclude labels")
        .min_by(|a, b| a.origin.y.total_cmp(&b.origin.y))
        .expect("the Exclude field is painted");
    assert!(
        include.origin.y > button.origin.y && exclude.origin.y > include.origin.y,
        "the fields are not stacked under the button: button {:.1}, include {:.1}, exclude {:.1}",
        button.origin.y,
        include.origin.y,
        exclude.origin.y
    );
    // Six chips, each whole, inside the dialog and the window, under the Exclude field; at least
    // two rows of them, so the row wrapped rather than running out of the panel.
    let chips: Vec<&lay::Overflow> = LABELS[..6].iter().map(|l| find(&drawn, l)).collect();
    for chip in &chips {
        assert_whole(chip);
        assert_inside(chip);
        assert_in_window(chip);
        assert!(
            chip.origin.y > exclude.origin.y,
            "{:?} is not under the Exclude field",
            chip.content
        );
    }
    let first = chips[0].origin.y;
    assert!(
        chips.iter().any(|c| c.origin.y > first + 8.0),
        "six chips stayed on one row: the row did not wrap"
    );
}

/// T059 (US6, selector-ui §2 rows 4–6, §5): the panel's six rows stack in order under the button,
/// the Assignee, Author and Milestone fields after the labels and the State row, each whole inside
/// the dialog and the window.
#[test]
fn the_six_row_panel_stacks_assignee_author_and_milestone_inside_the_window() {
    let drawn = painted(&covered("add-worktree-dialog-github-filters-panel"));
    let top = |content: &str| {
        drawn
            .iter()
            .filter(|d| d.content == content)
            .min_by(|a, b| a.origin.y.total_cmp(&b.origin.y))
            .unwrap_or_else(|| panic!("{content:?} is painted"))
    };
    let mut above = top("Exclude labels").origin.y;
    let state = top("Closed");
    assert!(
        state.origin.y > above,
        "the State row is not under Exclude labels"
    );
    above = state.origin.y;
    for title in ["Assignee", "Author", "Milestone"] {
        let field = top(title);
        assert!(
            field.origin.y > above,
            "{title:?} ({:.1}) is not under the row above it ({above:.1})",
            field.origin.y
        );
        assert_whole(field);
        assert_inside(field);
        assert_in_window(field);
        above = field.origin.y;
    }
}

/// T043 (US3 AS1, selector-ui §1, §5): in advanced mode the switch heads the picker, the Query input
/// and **Run** share one row in place of the Filters button and the active line, and the cap
/// caption of a sorted query keeps one line under the search; the footer stays reachable.
#[test]
fn the_query_row_and_the_cap_caption_fit_the_dialog() {
    let drawn = painted(&covered("add-worktree-dialog-github-advanced"));
    for gone in ["Clear filters", "Filters"] {
        assert!(
            drawn.iter().all(|d| d.content != gone),
            "{gone:?} shows in advanced mode"
        );
    }
    let basic = find(&drawn, "Basic");
    let advanced = find(&drawn, "Advanced");
    let run = find(&drawn, "Run");
    let query = find(&drawn, ADVANCED_QUERY);
    for text in [basic, advanced, run, query] {
        assert_inside(text);
        assert_whole(text);
    }
    let middle = |t: &lay::Overflow| t.origin.y + t.natural_height / 2.0;
    assert!(
        (middle(query) - middle(run)).abs() <= 8.0 && query.origin.x < run.origin.x,
        "the query ({:.1}, {:.1}) and Run ({:.1}, {:.1}) are not one row, in that order",
        query.origin.x,
        middle(query),
        run.origin.x,
        middle(run)
    );
    // The field's text is set a size up from the button's, so one line of it is a little taller.
    assert!(
        query.natural_height < 1.5 * run.natural_height,
        "the query wraps to {:.1}px, past one line ({:.1})",
        query.natural_height,
        run.natural_height
    );
    assert!(
        advanced.origin.y < run.origin.y,
        "the switch does not head the picker"
    );
    let caption = drawn
        .iter()
        .find(|d| d.content.starts_with("Showing the first 2 of 1,234 issues"))
        .expect("the cap caption names the first issues");
    assert_inside(caption);
    assert_whole(caption);
    assert!(
        caption.origin.y > run.origin.y,
        "the cap caption is not under the query row"
    );
    for action in ["Create", "Cancel"] {
        let button = find(&drawn, action);
        assert_in_window(button);
        assert!(
            button.origin.y > caption.origin.y,
            "{action} is not under the picker"
        );
    }
}
