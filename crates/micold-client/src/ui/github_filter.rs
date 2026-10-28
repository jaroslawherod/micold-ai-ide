//! The GitHub filters of the issue picker, drawn (feature 1400, contracts/selector-ui.md §1–§4).
//!
//! View glue only: `features::worktree_form::github_filter` decides what the filters are, which
//! labels a field offers and what each row says; this lays it out. The **Filters** button and its
//! panel are the frame the Jira filters use (`worktree_form::filters_panel`, research R13); inside
//! it, each label list is a [`material::Typeahead`] over the labels on offer, above its chosen
//! labels as chips that remove them. Its look is held by `tests/gates/github_filters_fit.rs` and
//! quickstart §B.

use crate::app::Message;
use crate::features::worktree_form::github_filter::{GithubSelector, LabelList, Msg as GithubMsg};
use crate::features::worktree_form::Msg as FormMsg;
use crate::features::worktree_form::WorktreeForm;
use crate::icons::Icon;
use crate::ui::material::{self, Button, ToggleChip, TypeaheadRow};
use iced::widget::{column, row, Row, Space};
use iced::{Alignment, Element, Length};
use micold_core::github::NoticeSource;
use micold_core::github_filter::NOT_ACCEPTED;
use micold_core::tokens::{spacing, Roles};

fn github(msg: GithubMsg) -> Message {
    Message::WorktreeForm(FormMsg::GithubFilter(msg))
}

/// The GitHub **Filters** button and its panel: the Include labels and Exclude labels fields, each
/// with its chosen labels under it (contracts/selector-ui.md §2).
fn filters_control<'a>(
    form: &'a WorktreeForm,
    source: NoticeSource<'_>,
    r: Roles,
) -> Element<'a, Message> {
    let selector = &form.github_selector;
    let mut rows: Vec<_> = [LabelList::Include, LabelList::Exclude]
        .into_iter()
        .map(|list| material::MenuSelectRow::controls(label_field(form, list, source, r)))
        .collect();
    rows.push(material::MenuSelectRow::controls(state_row(selector, r)));
    super::worktree_form::filters_panel(
        selector.basic.chosen_count(),
        rows,
        selector.panel_open,
        github(GithubMsg::PanelToggled),
        github(GithubMsg::PanelDismissed),
        r,
    )
}

/// The State row: one chip each for Open, Closed and All, the chosen one active (US5).
fn state_row<'a>(selector: &GithubSelector, r: Roles) -> Element<'a, Message> {
    let chips: Vec<Element<'a, Message>> = selector
        .state_choices()
        .into_iter()
        .map(|(state, words, active)| {
            ToggleChip::new(words, github(GithubMsg::StateChosen(state)), r)
                .active(active)
                .takes_the_keyboard()
                .into()
        })
        .collect();
    Row::with_children(chips).spacing(spacing::XS).wrap().into()
}

/// What a label field is called, as its label and its placeholder.
fn title(list: LabelList) -> &'static str {
    match list {
        LabelList::Include => "Include labels",
        LabelList::Exclude => "Exclude labels",
    }
}

/// One label list: the field over the labels on offer, its list's state as rows, then the chosen
/// labels as chips, wrapping inside the panel. Pressing a chip, or Enter or Space on it, removes
/// the label. A remembered label the repository lacks says so on its chip.
fn label_field<'a>(
    form: &'a WorktreeForm,
    list: LabelList,
    source: NoticeSource<'_>,
    r: Roles,
) -> Element<'a, Message> {
    let selector = &form.github_selector;
    let field = selector.field(list);
    let rows: Vec<TypeaheadRow> = selector
        .rows(list)
        .iter()
        .map(|row| {
            let label = row.label(source);
            let built = match row.summary() {
                // A failure's cause, or why labels are not offered, is the row's second line,
                // wrapped in full: a cut reason says nothing (as 1185's fields).
                Some(summary) => TypeaheadRow::new(summary, Vec::new()).details(label, Vec::new()),
                None => TypeaheadRow::new(label.clone(), row.spans.clone())
                    .single_line()
                    .tooltip(label),
            };
            if row.enabled() {
                built
            } else {
                built.disabled()
            }
        })
        .collect();
    let typeahead = material::Typeahead::new(
        &field.query,
        rows,
        move |text| github(GithubMsg::QueryChanged(list, text)),
        r,
    )
    .placeholder(title(list))
    .label(title(list))
    .open(field.open)
    .highlighted(field.highlight)
    .on_focus(github(GithubMsg::Focused(list)))
    .on_move(move |direction| github(GithubMsg::HighlightMoved(list, direction)))
    .on_dismiss(github(GithubMsg::Dismissed(list)))
    .on_pick(move |index| github(GithubMsg::Picked(list, index)));
    let chips: Vec<Element<'a, Message>> = selector
        .chosen(list)
        .iter()
        .map(|label| {
            ToggleChip::new(
                selector.chip_text(label),
                github(GithubMsg::Removed(label.clone())),
                r,
            )
            .active(true)
            .trailing_icon(Icon::Close)
            .takes_the_keyboard()
            .into()
        })
        .collect();
    let mut section = column![typeahead].spacing(spacing::XS);
    if !chips.is_empty() {
        section = section.push(Row::with_children(chips).spacing(spacing::XS).wrap());
    }
    section.into()
}

/// The row heading the picker while the GitHub filters show: **Clear filters** while a filter is
/// chosen (FR-007), then the **Filters** button, both trailing (contracts/selector-ui.md §1, where
/// feature 1400's switch will lead).
pub(super) fn controls_row<'a>(form: &'a WorktreeForm, r: Roles) -> Option<Element<'a, Message>> {
    if !form.shows_github_filters() {
        return None;
    }
    let source = form.notice_source()?;
    let mut controls = row![Space::new().width(Length::Fill)]
        .spacing(spacing::SM)
        .align_y(Alignment::Center);
    if !form.github_selector.basic.is_default() {
        controls =
            controls.push(Button::text("Clear filters", r).on_press(github(GithubMsg::ClearedAll)));
    }
    Some(controls.push(filters_control(form, source, r)).into())
}

/// The line naming the active filter under the search, while the GitHub filters show (FR-007).
pub(super) fn active_line(form: &WorktreeForm) -> Option<String> {
    form.shows_github_filters()
        .then(|| form.github_selector.active_line())
        .flatten()
}

/// What a refusal shows in place of the list (contracts/selector-ui.md §3): a heading, then each
/// of GitHub's messages on its own line, in GitHub's order; with none, one plain sentence.
pub(super) fn rejection_lines(messages: &[String]) -> Vec<String> {
    if messages.is_empty() {
        return vec![NOT_ACCEPTED.to_string()];
    }
    std::iter::once("GitHub did not accept the search query:".to_string())
        .chain(messages.iter().cloned())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_refusal_without_messages_says_so_plainly() {
        assert_eq!(
            rejection_lines(&[]),
            vec!["GitHub did not accept the search query.".to_string()]
        );
    }

    #[test]
    fn a_refusal_lists_githubs_messages_in_order_under_its_heading() {
        assert_eq!(
            rejection_lines(&["one".into(), "two".into()]),
            vec![
                "GitHub did not accept the search query:".to_string(),
                "one".to_string(),
                "two".to_string()
            ]
        );
    }

    #[test]
    fn the_fields_are_named_for_their_list() {
        assert_eq!(title(LabelList::Include), "Include labels");
        assert_eq!(title(LabelList::Exclude), "Exclude labels");
    }
}
