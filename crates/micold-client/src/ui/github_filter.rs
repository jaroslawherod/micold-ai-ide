//! The GitHub filters of the issue picker, drawn (feature 1400, contracts/selector-ui.md §1–§4).
//!
//! View glue only: `features::worktree_form::github_filter` decides what the filters are, which
//! labels a field offers and what each row says; this lays it out. The **Filters** button and its
//! panel are the frame the Jira filters use (`worktree_form::filters_panel`, research R13); inside
//! it, each label list is a [`material::Typeahead`] over the labels on offer, above its chosen
//! labels as chips that remove them. Its look is held by `tests/gates/github_filters_fit.rs` and
//! quickstart §B.

use crate::app::Message;
use crate::features::window::FieldId;
use crate::features::worktree_form::github_filter::{
    GithubSelector, LabelList, Msg as GithubMsg, ValueField,
};
use crate::features::worktree_form::Msg as FormMsg;
use crate::features::worktree_form::WorktreeForm;
use crate::icons::Icon;
use crate::ui::focus::TrackFocus;
use crate::ui::material::{self, Button, Text, TextField, ToggleChip, TypeRole};
use iced::widget::{column, container, row, Row, Space};
use iced::{Alignment, Element, Length, Padding};
use micold_core::github::NoticeSource;
use micold_core::github_filter::{QueryRefusal, NOT_ACCEPTED};
use micold_core::jira_query::QueryMode;
use micold_core::tokens::{spacing, Roles};

fn github(msg: GithubMsg) -> Message {
    Message::WorktreeForm(FormMsg::GithubFilter(msg))
}

/// The GitHub **Filters** button and its panel: the Include labels and Exclude labels fields, each
/// with its chosen labels under it, the State row, then the Assignee, Author and Milestone fields
/// (contracts/selector-ui.md §2).
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
    rows.extend(
        ValueField::ALL.into_iter().map(|field| {
            material::MenuSelectRow::controls(value_field(selector, field, source, r))
        }),
    );
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
    let rows = selector
        .rows(list)
        .iter()
        .map(|row| {
            super::worktree_form::typeahead_row(
                row.label(source),
                row.summary().map(str::to_string),
                row.spans.clone(),
                row.enabled(),
            )
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

/// A single-value field (US6, selector-ui §2 rows 4–6): a typeahead showing the chosen value's
/// words, over "Any …", its fixed choices and the people or milestones on offer.
fn value_field<'a>(
    selector: &'a GithubSelector,
    field: ValueField,
    source: NoticeSource<'_>,
    r: Roles,
) -> Element<'a, Message> {
    let state = selector.value_field(field);
    let rows = selector
        .value_rows(field)
        .iter()
        .map(|row| {
            super::worktree_form::typeahead_row(
                row.label(source),
                row.summary().map(str::to_string),
                row.spans.clone(),
                row.enabled(),
            )
        })
        .collect();
    material::Typeahead::new(
        &state.query,
        rows,
        move |text| github(GithubMsg::ValueQueryChanged(field, text)),
        r,
    )
    .placeholder(field.title())
    .label(field.title())
    .open(state.open)
    .highlighted(state.highlight)
    .on_focus(github(GithubMsg::ValueFocused(field)))
    .on_move(move |direction| github(GithubMsg::ValueHighlightMoved(field, direction)))
    .on_dismiss(github(GithubMsg::ValueDismissed(field)))
    .on_pick(move |index| github(GithubMsg::ValuePicked(field, index)))
    .into()
}

/// The Query input's placeholder (contracts/selector-ui.md §1), and the prompt in place of the list
/// before a query runs (FR-013).
pub const PROMPT: &str = "Type a GitHub search query, then press Enter or Run";

/// The row heading the picker while the GitHub filters show (contracts/selector-ui.md §1): the
/// Basic/Advanced switch leading (FR-001); in basic mode **Clear filters** while a filter is chosen
/// (FR-007), then the **Filters** button, both trailing. Advanced mode has neither (US3 AS1).
pub(super) fn controls_row<'a>(form: &'a WorktreeForm, r: Roles) -> Option<Element<'a, Message>> {
    if !form.shows_github_filters() {
        return None;
    }
    let source = form.notice_source()?;
    let mode = form.github_selector.mode;
    let switch = super::jira_query::mode_switch(mode, |m| github(GithubMsg::ModeChanged(m)), r);
    if mode == QueryMode::Advanced {
        return Some(row![switch, Space::new().width(Length::Fill)].into());
    }
    // The slot ahead of the Filters control is always there, empty or holding **Clear filters**:
    // focus is kept by position in the tree, so a button that came and went ahead of the control
    // moved it, and the chip a keyboard pick had focused lost the keyboard when the pick turned
    // the filter on or off. The gap sits inside the slot so an empty one adds none.
    let clear: Element<'a, Message> = if form.github_selector.basic.is_default() {
        Space::new().into()
    } else {
        container(Button::text("Clear filters", r).on_press(github(GithubMsg::ClearedAll)))
            .padding(Padding::default().right(spacing::SM))
            .into()
    };
    let controls = row![switch, Space::new().width(Length::Fill), clear]
        .align_y(Alignment::Center)
        .push(filters_control(form, source, r));
    Some(controls.into())
}

/// Advanced mode's query row, under the statement (contracts/selector-ui.md §1): the Query input,
/// where Enter runs the draft, beside **Run** (FR-010). The input keeps Enter: it is captured there,
/// so it reaches nothing behind the dialog.
pub(super) fn query_row<'a>(
    form: &'a WorktreeForm,
    r: Roles,
    focused: Option<FieldId>,
) -> Option<Element<'a, Message>> {
    if !form.shows_github_advanced() {
        return None;
    }
    let field = TextField::new(PROMPT, &form.github_selector.draft, r)
        .label("Query")
        .track_focus(FieldId::AddWorktreeGithubQuery, focused)
        .on_input(|text| github(GithubMsg::DraftChanged(text)))
        .on_submit(github(GithubMsg::Run));
    Some(
        row![
            container(field).width(Length::Fill),
            Button::text("Run", r).on_press(github(GithubMsg::Run)),
        ]
        .spacing(spacing::SM)
        .align_y(Alignment::Center)
        .into(),
    )
}

/// What stands in place of the list in advanced mode before a query runs (FR-013): the prompt.
pub(super) fn prompt<'a>(form: &WorktreeForm, r: Roles) -> Option<Element<'a, Message>> {
    form.shows_github_advanced()
        .then(|| Text::new(PROMPT, TypeRole::Caption, r).muted().into())
}

/// Why a query was not run (FR-012), error-tinted in place of the list.
pub(super) fn refusal<'a>(refusal: &QueryRefusal, r: Roles) -> Element<'a, Message> {
    Text::new(refusal.message(), TypeRole::Caption, r)
        .tint(r.error)
        .into()
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
    fn the_prompt_is_spelled_as_shipped() {
        assert_eq!(
            PROMPT,
            "Type a GitHub search query, then press Enter or Run"
        );
    }

    #[test]
    fn the_fields_are_named_for_their_list() {
        assert_eq!(title(LabelList::Include), "Include labels");
        assert_eq!(title(LabelList::Exclude), "Exclude labels");
    }

    /// A form showing the GitHub filters, with `chosen` as its state.
    fn showing(chosen: micold_core::github_filter::IssueState) -> crate::app::State {
        use crate::features::worktree_form::{BranchSource, Msg};
        let mut state = crate::app::State::default();
        state.workspace.active = Some(std::path::PathBuf::from("/work/acme"));
        let mut send = |msg| state.update(Message::WorktreeForm(msg));
        send(Msg::Opened);
        send(Msg::RemotesListed(Ok(vec![micold_core::git::GitRemote {
            name: "origin".into(),
            url: "git@github.com:o/r.git".into(),
        }])));
        send(Msg::SourceChanged(BranchSource::Issue));
        send(Msg::GithubFilter(GithubMsg::StateChosen(chosen)));
        state
    }

    /// A keyboard pick that turns **Clear filters** on or off must not move the **Filters**
    /// control (and the focus a chosen chip holds in it) to another slot of the row: the focus is
    /// kept by position in the tree, so a button that comes and goes ahead of it loses the focus.
    #[test]
    fn clear_filters_coming_and_going_keeps_the_filters_control_in_its_slot() {
        use micold_core::github_filter::IssueState;
        let r = micold_core::tokens::roles(micold_core::theme::ColorScheme::Light);
        let slots = |chosen| {
            let state = showing(chosen);
            let form = state.worktree_form.form.as_ref().unwrap();
            let row = controls_row(form, r).unwrap();
            let count = row.as_widget().children().len();
            count
        };
        assert_eq!(slots(IssueState::Open), slots(IssueState::Closed));
    }
}
