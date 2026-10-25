//! The Jira filters of the add-worktree form's issue picker (feature 1185): their state, their
//! messages and their reducer. Render-free, like every module here.
//!
//! While the Jira source shows, the picker offers a row of filter fields above the issue search
//! (contracts/filter-ui.md). Each field is a typeahead over the filter's choices, whose first row
//! is "Any <filter>" and clears it. A filter's choices are asked for only when the field first
//! takes focus ([`ChoiceLoad`], research R7), under a seq of their own, so a late answer for an
//! earlier request is dropped. Choosing, changing or clearing a filter hands out a fresh issue-load
//! seq and puts the list back to loading (research R6): the shell then loads the listing with the
//! filters now chosen, and the existing seq check drops every older listing.
//!
//! Nothing here is serialised or kept between openings of the form (FR-006, FR-016): the state
//! lives on [`WorktreeForm`] and is reset with its issues.

use crate::features::worktree_form::{IssueList, WorktreeForm, WorktreeFormStatus};
use micold_core::github::{Issue, IssueLoadError, NoticeSource};
use micold_core::jira_filter::{FilterKind, JiraFilters};
use micold_core::typeahead::{clamp_highlight, move_highlight, rank, Direction, Match, Query};

/// A filter's choices, from never asked to loaded (data-model §6).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ChoiceLoad<T> {
    /// Not asked for yet: the field never took focus on this form.
    #[default]
    NotRequested,
    /// Asked for; only the answer carrying `seq` applies.
    Loading { seq: u64 },
    /// The request failed; **Retry** asks again.
    Failed { error: IssueLoadError },
    /// The choices, in the order they are offered.
    Loaded(Vec<T>),
}

/// One filter field's typeahead: its own typed text, ranked matches, highlight and open list.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FilterField {
    /// The field's text: what was typed, or the chosen value's label.
    pub query: String,
    /// Which choices match `query`, in order; indices address the filter's loaded choices.
    pub matches: Vec<(usize, Match)>,
    /// Where the keyboard is, as an index into the field's rows ([`FilterState::rows`]).
    pub highlight: Option<usize>,
    /// Whether the field's list is showing.
    pub open: bool,
}

/// The filters of one open form (data-model §7).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FilterState {
    /// What the listing is filtered by.
    pub chosen: JiraFilters,
    /// The project's components.
    pub components: ChoiceLoad<String>,
    /// Each filter's field, in [`FilterKind::ALL`] order.
    fields: [FilterField; 4],
    /// The picked issue as it was when a filter change reloaded the list, so the form keeps its
    /// facts (the first prompt's title and body) when the new list does not hold it.
    pub kept_pick: Option<Issue>,
}

/// The filters this build offers in the row, in order (M1: the component).
pub const OFFERED: [FilterKind; 1] = [FilterKind::Component];

/// The position of `kind` in [`FilterKind::ALL`].
fn slot(kind: FilterKind) -> usize {
    match kind {
        FilterKind::Reporter => 0,
        FilterKind::Label => 1,
        FilterKind::Component => 2,
        FilterKind::FixVersion => 3,
    }
}

/// What picking a row of a filter's list does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowAction {
    /// Clear the filter ("Any <filter>").
    Any,
    /// Choose the choice at this index of the loaded choices.
    Choice(usize),
    /// Ask for the choices again after a failure.
    Retry,
    /// Nothing: a state row (loading, failure cause, empty reason).
    Inert,
}

/// What a row of a filter's list says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowText {
    /// "Any component".
    Any,
    /// A choice's label.
    Choice(String),
    /// "Loading components…".
    Loading,
    /// Why the choices failed to load.
    Failed(IssueLoadError),
    /// "Retry".
    Retry,
    /// "This project has no components".
    Empty,
    /// "No component matches".
    NoMatch,
}

/// One row of a filter's list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterRow {
    /// What it says.
    pub text: RowText,
    /// What picking it does.
    pub action: RowAction,
    /// The emphasis of a choice row: the matched characters.
    pub spans: Vec<std::ops::Range<usize>>,
}

impl FilterRow {
    fn new(text: RowText, action: RowAction) -> Self {
        Self {
            text,
            action,
            spans: Vec::new(),
        }
    }

    /// Whether the row can be picked: state rows cannot.
    pub fn enabled(&self) -> bool {
        self.action != RowAction::Inert
    }

    /// The row's words for `kind`; a failure is named for `source`.
    pub fn label(&self, kind: FilterKind, source: Option<NoticeSource<'_>>) -> String {
        match &self.text {
            RowText::Any => format!("Any {}", kind.noun()),
            RowText::Choice(text) => text.clone(),
            RowText::Loading => format!("Loading {}…", plural(kind)),
            RowText::Failed(error) => match source {
                Some(source) => error.message(source),
                None => format!("Couldn't load the {}.", plural(kind)),
            },
            RowText::Retry => "Retry".to_string(),
            RowText::Empty => format!("This project has no {}", plural(kind)),
            RowText::NoMatch => format!("No {} matches", kind.noun()),
        }
    }
}

/// The filter's choices named in the plural: "components".
fn plural(kind: FilterKind) -> &'static str {
    match kind {
        FilterKind::Reporter => "people",
        FilterKind::Label => "labels",
        FilterKind::Component => "components",
        FilterKind::FixVersion => "versions",
    }
}

impl FilterState {
    /// `kind`'s field.
    pub fn field(&self, kind: FilterKind) -> &FilterField {
        &self.fields[slot(kind)]
    }

    fn field_mut(&mut self, kind: FilterKind) -> &mut FilterField {
        &mut self.fields[slot(kind)]
    }

    /// `kind`'s choice load, for the filters that ask Jira for their choices.
    pub fn choices(&self, kind: FilterKind) -> Option<&ChoiceLoad<String>> {
        match kind {
            FilterKind::Component => Some(&self.components),
            _ => None,
        }
    }

    fn choices_mut(&mut self, kind: FilterKind) -> Option<&mut ChoiceLoad<String>> {
        match kind {
            FilterKind::Component => Some(&mut self.components),
            _ => None,
        }
    }

    /// The seq of the choice request now awaited, with its filter.
    pub fn awaited_choices(&self) -> Option<(FilterKind, u64)> {
        OFFERED
            .into_iter()
            .find_map(|kind| match self.choices(kind)? {
                ChoiceLoad::Loading { seq } => Some((kind, *seq)),
                _ => None,
            })
    }

    /// The rows of `kind`'s list: "Any <filter>" first, then the matching choices, or the state the
    /// choices are in (contracts/filter-ui.md §2).
    pub fn rows(&self, kind: FilterKind) -> Vec<FilterRow> {
        let mut rows = vec![FilterRow::new(RowText::Any, RowAction::Any)];
        match self.choices(kind) {
            None | Some(ChoiceLoad::NotRequested) | Some(ChoiceLoad::Loading { .. }) => {
                rows.push(FilterRow::new(RowText::Loading, RowAction::Inert));
            }
            Some(ChoiceLoad::Failed { error }) => {
                rows.push(FilterRow::new(
                    RowText::Failed(error.clone()),
                    RowAction::Inert,
                ));
                rows.push(FilterRow::new(RowText::Retry, RowAction::Retry));
            }
            Some(ChoiceLoad::Loaded(choices)) if choices.is_empty() => {
                rows.push(FilterRow::new(RowText::Empty, RowAction::Inert));
            }
            Some(ChoiceLoad::Loaded(choices)) => {
                let field = self.field(kind);
                if field.matches.is_empty() {
                    rows.push(FilterRow::new(RowText::NoMatch, RowAction::Inert));
                }
                for (index, matched) in &field.matches {
                    if let Some(choice) = choices.get(*index) {
                        rows.push(FilterRow {
                            text: RowText::Choice(choice.clone()),
                            action: RowAction::Choice(*index),
                            spans: matched.spans.clone(),
                        });
                    }
                }
            }
        }
        rows
    }

    /// The row of `kind`'s list that holds the chosen value, if it is shown.
    pub fn selected_row(&self, kind: FilterKind) -> Option<usize> {
        let chosen = self.chosen.shown(kind);
        self.rows(kind).iter().position(|row| match &row.text {
            RowText::Any => chosen.is_none(),
            RowText::Choice(text) => chosen.as_ref() == Some(text),
            _ => false,
        })
    }

    /// The line naming the chosen filters, while any is chosen: "Filtered by component Payments".
    pub fn active_line(&self) -> Option<String> {
        let active = self.chosen.active();
        (!active.is_empty()).then(|| format!("Filtered by {}", described(&active)))
    }

    /// Re-rank `kind`'s choices against its field's text and re-seat its highlight. While the text
    /// is the chosen value's label every choice is offered: the text shows the choice, it is not a
    /// search.
    fn rematch(&mut self, kind: FilterKind) {
        let chosen = self.chosen.shown(kind);
        let choices: Vec<String> = match self.choices(kind) {
            Some(ChoiceLoad::Loaded(choices)) => choices.clone(),
            _ => Vec::new(),
        };
        let field = self.field_mut(kind);
        let text = if chosen.as_deref() == Some(field.query.as_str()) {
            ""
        } else {
            field.query.as_str()
        };
        field.matches = rank(&choices, |choice| choice.as_str(), &Query::new(text));
        let rows = self.rows(kind).len();
        let field = self.field_mut(kind);
        field.highlight = clamp_highlight(field.highlight, rows);
    }

    /// Choose `value` for `kind` (or clear it with `None`); `true` when the chosen filters changed.
    fn choose(&mut self, kind: FilterKind, value: Option<String>) -> bool {
        let before = self.chosen.clone();
        let accepted = match kind {
            FilterKind::Component => self.chosen.set_component(value),
            FilterKind::Label => self.chosen.set_label(value),
            FilterKind::FixVersion => self.chosen.set_fix_version(value),
            // The reporter is chosen as a user, not a text, and is not offered in this build.
            FilterKind::Reporter => {
                value.is_none() && {
                    self.chosen.clear(FilterKind::Reporter);
                    true
                }
            }
        };
        self.restore_query(kind);
        accepted && self.chosen != before
    }

    /// Put `kind`'s field text back to the chosen value's label, or empty.
    fn restore_query(&mut self, kind: FilterKind) {
        let shown = self.chosen.shown(kind).unwrap_or_default();
        self.field_mut(kind).query = shown;
        self.rematch(kind);
    }
}

/// "component Payments, fix version 2.4": the chosen filters as a sentence names them.
pub fn described(active: &[(FilterKind, String)]) -> String {
    active
        .iter()
        .map(|(kind, value)| format!("{} {value}", kind.noun()))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The filters' messages, wrapped as `worktree_form::Msg::JiraFilter` (data-model §8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterMsg {
    /// A filter field took focus: its list opens and, the first time, its choices are asked for.
    Focused(FilterKind),
    /// A filter field's text changed.
    QueryChanged(FilterKind, String),
    /// The keyboard moved through a filter's list.
    HighlightMoved(FilterKind, Direction),
    /// A filter's list closed without a pick (Escape, a press outside, Tab).
    Dismissed(FilterKind),
    /// Row `index` of a filter's list was picked; row 0 is "Any <filter>".
    Picked(FilterKind, usize),
    /// Clear one filter.
    Cleared(FilterKind),
    /// **Clear filters**: clear every filter.
    ClearedAll,
    /// Ask for a filter's choices again after a failure.
    ChoicesRetry(FilterKind),
    /// The project's components answered. Applies only while they are awaited under `seq`.
    ComponentsLoaded {
        seq: u64,
        result: Result<Vec<String>, IssueLoadError>,
    },
}

/// Whether the open form shows the filters: the Jira source is showing, its list asked for.
pub fn shows_filters(form: &WorktreeForm) -> bool {
    form.jira_site().is_some()
        && form.source == crate::features::worktree_form::BranchSource::Issue
        && form.issues != IssueList::NotRequested
}

/// The filters' reducer.
pub fn update(state: &mut crate::app::State, msg: FilterMsg) {
    let next_issue_seq = state.worktree_form.issue_request_seq + 1;
    let next_filter_seq = state.worktree_form.filter_request_seq + 1;
    let Some(form) = state.worktree_form.form.as_mut() else {
        return;
    };
    let editing = form.status == WorktreeFormStatus::Editing && !form.resolution.is_prompting();
    let mut reload = false;
    let mut asked = false;
    match msg {
        FilterMsg::ComponentsLoaded { seq, result } => {
            let filters = &mut form.jira_filters;
            if filters.components != (ChoiceLoad::Loading { seq }) {
                return;
            }
            filters.components = match result {
                Ok(names) => ChoiceLoad::Loaded(names),
                Err(error) => ChoiceLoad::Failed { error },
            };
            filters.rematch(FilterKind::Component);
        }
        // Closing a list is never refused: a list left open behind a prompt would reappear.
        FilterMsg::Dismissed(kind) => {
            let filters = &mut form.jira_filters;
            filters.field_mut(kind).open = false;
            filters.restore_query(kind);
        }
        _ if !editing || !shows_filters(form) => return,
        FilterMsg::Focused(kind) => {
            let filters = &mut form.jira_filters;
            filters.field_mut(kind).open = true;
            if let Some(load @ ChoiceLoad::NotRequested) = filters.choices_mut(kind) {
                *load = ChoiceLoad::Loading {
                    seq: next_filter_seq,
                };
                asked = true;
            }
            filters.rematch(kind);
        }
        FilterMsg::QueryChanged(kind, text) => {
            let filters = &mut form.jira_filters;
            let field = filters.field_mut(kind);
            field.query = text;
            field.open = true;
            filters.rematch(kind);
        }
        FilterMsg::HighlightMoved(kind, direction) => {
            let filters = &mut form.jira_filters;
            let rows = filters.rows(kind).len();
            let field = filters.field_mut(kind);
            if let Some(next) = move_highlight(field.highlight, direction, rows) {
                field.highlight = Some(next);
            }
        }
        FilterMsg::Picked(kind, index) => {
            let filters = &mut form.jira_filters;
            let Some(row) = filters.rows(kind).into_iter().nth(index) else {
                return;
            };
            match row.action {
                RowAction::Any => {
                    filters.field_mut(kind).open = false;
                    reload = filters.choose(kind, None);
                }
                RowAction::Choice(choice) => {
                    let Some(value) = (match filters.choices(kind) {
                        Some(ChoiceLoad::Loaded(choices)) => choices.get(choice).cloned(),
                        _ => None,
                    }) else {
                        return;
                    };
                    filters.field_mut(kind).open = false;
                    reload = filters.choose(kind, Some(value));
                }
                RowAction::Retry => asked = retry(filters, kind, next_filter_seq),
                RowAction::Inert => {}
            }
        }
        FilterMsg::Cleared(kind) => reload = form.jira_filters.choose(kind, None),
        FilterMsg::ClearedAll => {
            let filters = &mut form.jira_filters;
            let had = !filters.chosen.is_empty();
            filters.chosen.clear_all();
            for kind in FilterKind::ALL {
                filters.restore_query(kind);
            }
            reload = had;
        }
        FilterMsg::ChoicesRetry(kind) => {
            asked = retry(&mut form.jira_filters, kind, next_filter_seq);
        }
    }
    if reload {
        let picked = form.picked_issue.and_then(|number| {
            form.issues
                .held()
                .into_iter()
                .find(|issue| issue.number() == number)
                .cloned()
        });
        form.jira_filters.kept_pick = picked.or(form.jira_filters.kept_pick.take());
        form.issues = IssueList::Loading {
            seq: next_issue_seq,
        };
        form.issue_highlight = None;
        state.worktree_form.issue_request_seq = next_issue_seq;
    }
    if asked {
        state.worktree_form.filter_request_seq = next_filter_seq;
    }
}

/// Ask for `kind`'s choices again when they failed; `true` when a request was handed `seq`.
fn retry(filters: &mut FilterState, kind: FilterKind, seq: u64) -> bool {
    match filters.choices_mut(kind) {
        Some(load @ ChoiceLoad::Failed { .. }) => {
            *load = ChoiceLoad::Loading { seq };
            filters.rematch(kind);
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loaded(names: &[&str]) -> FilterState {
        let mut filters = FilterState {
            components: ChoiceLoad::Loaded(names.iter().map(|n| n.to_string()).collect()),
            ..FilterState::default()
        };
        filters.rematch(FilterKind::Component);
        filters
    }

    #[test]
    fn the_active_line_names_the_chosen_component() {
        let mut filters = loaded(&["Payments"]);
        assert_eq!(filters.active_line(), None);
        assert!(filters.choose(FilterKind::Component, Some("Payments".into())));
        assert_eq!(
            filters.active_line().as_deref(),
            Some("Filtered by component Payments")
        );
    }

    #[test]
    fn row_zero_is_any_and_the_choices_follow_in_order() {
        let filters = loaded(&["Payments", "Auth"]);
        let labels: Vec<String> = filters
            .rows(FilterKind::Component)
            .iter()
            .map(|row| row.label(FilterKind::Component, None))
            .collect();
        assert_eq!(labels, ["Any component", "Payments", "Auth"]);
    }

    #[test]
    fn the_field_showing_the_chosen_value_offers_every_choice() {
        let mut filters = loaded(&["Payments", "Auth"]);
        assert!(filters.choose(FilterKind::Component, Some("Payments".into())));
        assert_eq!(filters.field(FilterKind::Component).query, "Payments");
        assert_eq!(filters.rows(FilterKind::Component).len(), 3);
        assert_eq!(filters.selected_row(FilterKind::Component), Some(1));
    }

    #[test]
    fn choosing_the_same_value_again_changes_nothing() {
        let mut filters = loaded(&["Payments"]);
        assert!(filters.choose(FilterKind::Component, Some("Payments".into())));
        assert!(!filters.choose(FilterKind::Component, Some("Payments".into())));
    }

    #[test]
    fn a_choice_answer_for_another_seq_is_dropped() {
        let mut state = crate::app::State::default();
        let mut form = WorktreeForm::default();
        form.jira_filters.components = ChoiceLoad::Loading { seq: 2 };
        state.worktree_form.form = Some(form);
        update(
            &mut state,
            FilterMsg::ComponentsLoaded {
                seq: 1,
                result: Ok(vec!["Old".into()]),
            },
        );
        let filters = &state.worktree_form.form.as_ref().unwrap().jira_filters;
        assert_eq!(filters.components, ChoiceLoad::Loading { seq: 2 });
        update(
            &mut state,
            FilterMsg::ComponentsLoaded {
                seq: 2,
                result: Ok(vec!["New".into()]),
            },
        );
        let filters = &state.worktree_form.form.as_ref().unwrap().jira_filters;
        assert_eq!(filters.components, ChoiceLoad::Loaded(vec!["New".into()]));
    }
}
