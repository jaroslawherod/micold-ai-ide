//! A status chip does not move the row's key and title (feature 1101, FR-009, edge case).

mod support;

use support::jira_picker::{form_with, state_with, ticket};
use support::layout::{painted_text_settled, renderer, view_of};

/// Where the row's title line is painted, and the chip's text and its widths, for a ticket with
/// `status`.
fn painted_for(status: Option<(&str, &str)>) -> (iced::Point, Vec<(String, f32, f32)>) {
    let form = form_with(vec![ticket(1, "Alpha", status)], "");
    let texts = painted_text_settled(view_of(&state_with(form)), &mut renderer());
    let title = texts
        .iter()
        .find(|t| t.content.contains("Alpha"))
        .expect("the title is painted")
        .origin;
    let others = texts
        .iter()
        .filter(|t| !t.content.contains("Alpha"))
        .map(|t| (t.content.clone(), t.natural_width, t.allowed_width))
        .collect();
    (title, others)
}

#[test]
fn a_long_status_name_is_truncated_and_leaves_the_title_where_it_was() {
    let long = "Waiting for customer confirmation of the fix";
    let (bare_title, _) = painted_for(None);
    let (title, others) = painted_for(Some((long, "indeterminate")));
    assert_eq!(title, bare_title, "the title did not move");
    let chip = others
        .iter()
        .find(|(content, _, _)| content.starts_with("Waiting") && content.ends_with('…'))
        .unwrap_or_else(|| panic!("the chip is cut with an ellipsis: {others:?}"));
    assert!(
        chip.1 <= chip.2 + 0.5,
        "the chip text fits its width: {chip:?}"
    );
}

#[test]
fn a_row_with_a_chip_has_the_layout_of_one_without() {
    let (a, _) = painted_for(None);
    let (b, others) = painted_for(Some(("To Do", "new")));
    assert_eq!(a, b);
    assert!(others.iter().any(|(c, _, _)| c == "To Do"));
}
