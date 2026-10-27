//! The GitHub filters of the add-worktree form's issue picker (feature 1400): their state, their
//! messages and their reducer. Render-free, like every module here.
//!
//! While the GitHub source shows, a **Filters** button beside the issue search opens a panel with
//! an Include labels and an Exclude labels field (contracts/selector-ui.md §2). Each field is a
//! typeahead over the repository's labels, less those already in its own list; a pick adds the
//! label to its list (moving it out of the other) and clears the field, and each chosen label is a chip that removes it.
//! The labels are asked for once, when a label field first opens ([`ChoiceLoad`], FR-005), under a
//! filter seq of their own, so a late answer for an earlier request is dropped.
//!
//! Choosing, removing or clearing a label hands out a fresh issue-load seq and puts the list back
//! to loading (FR-008): the shell then loads the listing with the filter now chosen (the filtered
//! listing, or 034's listing once the filter is the default again, FR-009), and the seq check drops
//! every older answer.
//!
//! The project remembers its basic filters (FR-021): the form opens with them (`Msg::Restored`,
//! read by the shell), and each change hands back `Outcome::GithubSelectorRemembered`. A
//! remembered label the repository no longer has still applies, its chip saying so.
//!
//! The selector is kept while the form is open, across a change of source or provider (spec Edge
//! "Switching source to Jira and back"); its text fields and panel close with the issues.

use std::path::PathBuf;

use crate::features::worktree_form::jira_filters::{ChoiceLoad, FilterField};
use crate::features::worktree_form::{
    BranchSource, IssueList, SearchState, WorktreeForm, WorktreeFormStatus,
};
use micold_core::env_include::EnvIncludeSnapshot;
use micold_core::github::{Issue, IssueListing, IssueLoadError, NoticeSource};
use micold_core::github_filter::{
    BasicFilters, FilterFailure, GithubFilter, LabelChoices, LabelName, RememberedSelector,
    SelectorChange, UserQuery, NOT_ACCEPTED,
};
use micold_core::jira_query::QueryMode;
use micold_core::typeahead::{clamp_highlight, move_highlight, rank, Direction, Query};

/// Which of the two label fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelList {
    /// Every listed issue carries each of these labels.
    Include,
    /// No listed issue carries any of these labels.
    Exclude,
}

/// The selector's state, one field of [`WorktreeForm`] (data-model §10). `Debug` prints no filter
/// value, typed text or query (FR-024, research R15).
#[derive(Clone, Default, PartialEq, Eq)]
pub struct GithubSelector {
    /// Basic or advanced; changed only by the switch (advanced mode is feature 1400's US3).
    pub mode: QueryMode,
    /// The basic filters.
    pub basic: BasicFilters,
    /// The query input as typed (US3).
    pub draft: String,
    /// The last query run on the project; `None` when none ever ran (US3).
    pub last_run: Option<UserQuery>,
    /// Which mode's filter produced the list shown, or in flight (research R8).
    pub listed_for: Option<QueryMode>,
    /// The repository's labels, asked for on a label field's first opening. Held as one
    /// [`LabelChoices`] in `Loaded`, so [`ChoiceLoad`]'s seq rule is 1185's.
    pub labels: ChoiceLoad<LabelChoices>,
    /// The Include labels field.
    pub include_field: FilterField,
    /// The Exclude labels field.
    pub exclude_field: FilterField,
    /// Whether the Filters panel is open.
    pub panel_open: bool,
}

impl std::fmt::Debug for GithubSelector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let labels = match &self.labels {
            ChoiceLoad::NotRequested => "not requested".to_string(),
            ChoiceLoad::Loading { seq } => format!("loading {seq}"),
            ChoiceLoad::Failed { .. } => "failed".to_string(),
            ChoiceLoad::Loaded(choices) => {
                format!("{} label(s)", choices.first().map_or(0, |c| c.labels.len()))
            }
        };
        f.debug_struct("GithubSelector")
            .field("mode", &self.mode)
            .field("basic", &self.basic)
            .field("draft", &"<redacted>")
            .field("last_run", &self.last_run)
            .field("listed_for", &self.listed_for)
            .field("labels", &labels)
            .field("include_field", &self.include_field)
            .field("exclude_field", &self.exclude_field)
            .field("panel_open", &self.panel_open)
            .finish()
    }
}

/// What a label row does when picked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LabelAction {
    /// Add this label to the field's list.
    Choice(LabelName),
    /// Ask for the labels again after a failure.
    Retry,
    /// A state row: nothing to pick.
    Inert,
}

/// What a label row says (contracts/selector-ui.md §2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LabelText {
    /// The labels are on their way.
    Loading,
    /// The labels could not be read: 034's notice for the failure.
    Failed(IssueLoadError),
    /// **Retry** after a failure.
    Retry,
    /// The repository has no label to offer.
    NoLabels,
    /// Nothing offered matches the typed text.
    NoMatch,
    /// A label.
    Choice(LabelName),
    /// Only the first 1,000 labels were read.
    Partial,
    /// This many labels hold a quote and cannot be offered.
    NotOffered(usize),
}

/// One row of a label field's list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelRow {
    /// What it says.
    pub text: LabelText,
    /// What picking it does.
    pub action: LabelAction,
    /// The typed text's matches in the label, for emphasis.
    pub spans: Vec<std::ops::Range<usize>>,
}

impl LabelRow {
    fn state(text: LabelText, action: LabelAction) -> LabelRow {
        LabelRow {
            text,
            action,
            spans: Vec::new(),
        }
    }

    /// Whether picking the row does anything.
    pub fn enabled(&self) -> bool {
        self.action != LabelAction::Inert
    }

    /// The row's words; a failure is named as 034 names it for `source`.
    pub fn label(&self, source: NoticeSource<'_>) -> String {
        match &self.text {
            LabelText::Loading => "Loading labels…".to_string(),
            LabelText::Failed(error) => label_failure(error, source),
            LabelText::Retry => "Retry".to_string(),
            LabelText::NoLabels => "This repository has no labels".to_string(),
            LabelText::NoMatch => "No label matches".to_string(),
            LabelText::Choice(label) => label.as_str().to_string(),
            LabelText::Partial => "Only the first 1,000 labels are offered".to_string(),
            LabelText::NotOffered(1) => {
                "1 label can't be offered because its name holds a quote".to_string()
            }
            LabelText::NotOffered(n) => {
                format!("{n} labels can't be offered because their names hold a quote")
            }
        }
    }

    /// A heading above the row's words, for a row whose words wrap in full under it: a failure's
    /// cause, and why labels are not offered (visual pass M2 F3: cut, it said nothing).
    pub fn summary(&self) -> Option<&'static str> {
        match self.text {
            LabelText::Failed(_) => Some("Couldn't load the labels."),
            LabelText::NotOffered(_) => Some("Labels not offered"),
            _ => None,
        }
    }
}

/// A labels load's failure in words for the labels: the cause without 034's "Couldn't read
/// issues" opening, which would name the wrong thing under "Couldn't load the labels." (visual
/// pass M2 F5).
fn label_failure(error: &IssueLoadError, source: NoticeSource<'_>) -> String {
    match source {
        NoticeSource::Github(repo) => error.github_cause(repo),
        jira @ NoticeSource::Jira { .. } => error.message(jira),
    }
}

impl GithubSelector {
    /// The selector a form opens with, from what the project remembers (data-model §10): its basic
    /// filters, applied as the GitHub source shows (US2 AS1).
    pub fn from_remembered(remembered: &RememberedSelector) -> GithubSelector {
        GithubSelector {
            basic: remembered.basic.clone(),
            ..GithubSelector::default()
        }
    }

    /// The filter a request is made with: the basic filters in basic mode; in advanced mode the
    /// last run query, or none (the prompt, nothing sent).
    pub fn active_filter(&self) -> Option<GithubFilter> {
        match self.mode {
            QueryMode::Basic => Some(GithubFilter::Basic(self.basic.clone())),
            QueryMode::Advanced => self.last_run.clone().map(GithubFilter::Advanced),
        }
    }

    /// The line naming the active basic filter (FR-007): "Excluding labels in-progress, blocked".
    /// `None` with the default filter and in advanced mode.
    pub fn active_line(&self) -> Option<String> {
        match self.mode {
            QueryMode::Basic => self.basic.described(),
            QueryMode::Advanced => None,
        }
    }

    /// What `label`'s chip says: its name, and "not in this repository" when the repository's
    /// labels loaded in full without it (spec Edge "Remembered value no longer in the repository").
    /// GitHub matches labels case-insensitively, so neither does this; a partial list says nothing,
    /// the label may be further on.
    pub fn chip_text(&self, label: &LabelName) -> String {
        let name = label.as_str();
        let missing = self.choices().is_some_and(|choices| {
            !choices.partial
                && !choices
                    .labels
                    .iter()
                    .any(|l| l.as_str().to_lowercase() == name.to_lowercase())
        });
        if missing {
            format!("{name} · not in this repository")
        } else {
            name.to_string()
        }
    }

    /// `list`'s field.
    pub fn field(&self, list: LabelList) -> &FilterField {
        match list {
            LabelList::Include => &self.include_field,
            LabelList::Exclude => &self.exclude_field,
        }
    }

    fn field_mut(&mut self, list: LabelList) -> &mut FilterField {
        match list {
            LabelList::Include => &mut self.include_field,
            LabelList::Exclude => &mut self.exclude_field,
        }
    }

    /// The labels chosen in `list`, in chosen order.
    pub fn chosen(&self, list: LabelList) -> &[LabelName] {
        match list {
            LabelList::Include => self.basic.include(),
            LabelList::Exclude => self.basic.exclude(),
        }
    }

    fn choices(&self) -> Option<&LabelChoices> {
        match &self.labels {
            ChoiceLoad::Loaded(choices) => choices.first(),
            _ => None,
        }
    }

    /// Whether the labels loaded and the repository has none to offer: the fields are disabled.
    pub fn no_labels(&self) -> bool {
        self.choices().is_some_and(|c| c.labels.is_empty())
    }

    /// The seq the label choices are awaited under, while in flight.
    pub fn awaited_labels(&self) -> Option<u64> {
        self.labels.awaited()
    }

    /// The labels `list`'s field offers: the repository's, alphabetically, less those already in
    /// `list` (US1 AS1). One the other list holds is offered, and picking it moves it (spec Edge
    /// "Same label included and excluded", quickstart B4).
    fn offered(&self, list: LabelList) -> Vec<LabelName> {
        let own = self.chosen(list);
        self.choices()
            .map(|c| {
                c.labels
                    .iter()
                    .filter(|l| !own.contains(l))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The rows of `list`'s field: the matching labels, or the state the labels are in.
    pub fn rows(&self, list: LabelList) -> Vec<LabelRow> {
        let choices = match &self.labels {
            ChoiceLoad::NotRequested | ChoiceLoad::Loading { .. } => {
                return vec![LabelRow::state(LabelText::Loading, LabelAction::Inert)];
            }
            ChoiceLoad::Failed { error } => {
                return vec![
                    LabelRow::state(LabelText::Failed(error.clone()), LabelAction::Inert),
                    LabelRow::state(LabelText::Retry, LabelAction::Retry),
                ];
            }
            ChoiceLoad::Loaded(choices) => choices.first().cloned().unwrap_or_default(),
        };
        let mut rows = Vec::new();
        if choices.labels.is_empty() {
            rows.push(LabelRow::state(LabelText::NoLabels, LabelAction::Inert));
        } else {
            let offered = self.offered(list);
            let field = self.field(list);
            if field.matches.is_empty() {
                rows.push(LabelRow::state(LabelText::NoMatch, LabelAction::Inert));
            }
            for (index, matched) in &field.matches {
                if let Some(label) = offered.get(*index) {
                    rows.push(LabelRow {
                        text: LabelText::Choice(label.clone()),
                        action: LabelAction::Choice(label.clone()),
                        spans: matched.spans.clone(),
                    });
                }
            }
            if choices.partial {
                rows.push(LabelRow::state(LabelText::Partial, LabelAction::Inert));
            }
        }
        if choices.not_offered > 0 {
            rows.push(LabelRow::state(
                LabelText::NotOffered(choices.not_offered),
                LabelAction::Inert,
            ));
        }
        rows
    }

    /// Re-rank the offered labels against `list`'s text and re-seat its highlight.
    fn rematch(&mut self, list: LabelList) {
        let offered = self.offered(list);
        let field = self.field(list);
        let matches = rank(&offered, |l| l.as_str(), &Query::new(&field.query));
        self.field_mut(list).matches = matches;
        let rows = self.rows(list).len();
        let field = self.field_mut(list);
        field.highlight = clamp_highlight(field.highlight, rows);
    }

    fn rematch_both(&mut self) {
        self.rematch(LabelList::Include);
        self.rematch(LabelList::Exclude);
    }

    /// Close both fields' lists, their text cleared, and the panel, as a dismissal does: the
    /// issues were left, or the panel closed.
    pub fn close_lists(&mut self) {
        for list in [LabelList::Include, LabelList::Exclude] {
            let field = self.field_mut(list);
            field.open = false;
            field.query.clear();
            field.highlight = None;
        }
        self.rematch_both();
        self.panel_open = false;
    }

    /// Open `list`'s field; its first opening by any message asks for the labels under `seq`.
    /// `true` when a request was handed `seq`.
    fn open_field(&mut self, list: LabelList, seq: u64) -> bool {
        self.field_mut(list).open = true;
        self.labels.start(seq, false)
    }

    /// Add `label` to `list` (moving it out of the other) and clear the field; `true` when the
    /// filter changed. The field keeps the keyboard, so its list stays open on the labels left,
    /// and its highlight starts over: Enter adds nothing until a row is highlighted again (visual
    /// pass M2 round 2 F2, F3).
    fn choose(&mut self, list: LabelList, label: LabelName) -> bool {
        let before = self.basic.clone();
        match list {
            LabelList::Include => self.basic.include_label(label),
            LabelList::Exclude => self.basic.exclude_label(label),
        }
        let field = self.field_mut(list);
        field.query.clear();
        field.highlight = None;
        self.rematch_both();
        self.basic != before
    }
}

impl WorktreeForm {
    /// Whether the open form shows the GitHub filters: the GitHub source is showing with GitHub
    /// issues on, its list asked for (FR-025).
    pub fn shows_github_filters(&self) -> bool {
        self.jira_site().is_none()
            && self.github_repo().is_some()
            && self.source == BranchSource::Issue
            && self.issues != IssueList::NotRequested
    }

    /// The GitHub filter in force when it is not the default: what a GitHub request carries
    /// instead of 034's arguments (FR-009). `None` while Jira is the provider or GitHub issues are
    /// off.
    pub fn github_filter(&self) -> Option<GithubFilter> {
        if self.jira_site().is_some() || self.github_repo().is_none() {
            return None;
        }
        self.github_selector
            .active_filter()
            .filter(|filter| !filter.is_default())
    }
}

/// The filters' messages, wrapped as `worktree_form::Msg::GithubFilter`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Msg {
    /// The **Filters** button: open the panel, or close it when open.
    PanelToggled,
    /// The panel closed without the button (Escape, a press outside).
    PanelDismissed,
    /// A label field took focus: its list opens and, the first time, the labels are asked for.
    Focused(LabelList),
    /// A label field's text changed.
    QueryChanged(LabelList, String),
    /// The keyboard moved through a label field's list.
    HighlightMoved(LabelList, Direction),
    /// A label field's list closed without a pick.
    Dismissed(LabelList),
    /// Row `index` of a label field's list was picked.
    Picked(LabelList, usize),
    /// A chosen label's chip was pressed: remove it from its list.
    Removed(LabelName),
    /// **Clear filters**: the default filter.
    ClearedAll,
    /// The form opened: the project's remembered selector, read by the shell (FR-021).
    Restored(RememberedSelector),
    /// The repository's labels answered. Applies only while awaited under `seq`.
    LabelsLoaded {
        seq: u64,
        result: Result<LabelChoices, IssueLoadError>,
    },
    /// A filtered listing answered, with the `gh` it ran and the include snapshot it resolved.
    /// Applies only while the list awaits `seq`.
    Loaded {
        seq: u64,
        result: Result<(IssueListing, PathBuf), FilterFailure>,
        resolved_env: Option<(PathBuf, EnvIncludeSnapshot)>,
    },
    /// A filtered search beyond the list answered. Applies only while it runs under `seq`.
    Searched {
        seq: u64,
        result: Result<Vec<Issue>, FilterFailure>,
    },
}

/// A refused search beyond the list, as one status line: GitHub's messages after the heading, or
/// [`NOT_ACCEPTED`] alone when it gave none (contracts/selector-ui.md §3).
pub fn refusal_line(messages: &[String]) -> String {
    if messages.is_empty() {
        return NOT_ACCEPTED.to_string();
    }
    format!(
        "GitHub did not accept the search query: {}",
        messages.join(" ")
    )
}

/// The filters' reducer. A change of the basic filters hands back
/// `Outcome::GithubSelectorRemembered` for the shell to write (FR-021).
pub fn update(state: &mut crate::app::State, msg: Msg) -> Vec<crate::features::Outcome> {
    let next_issue_seq = state.worktree_form.issue_request_seq + 1;
    let next_filter_seq = state.worktree_form.filter_request_seq + 1;
    let Some(form) = state.worktree_form.form.as_mut() else {
        return Vec::new();
    };
    let editing = form.status == WorktreeFormStatus::Editing && !form.resolution.is_prompting();
    let mut reload = false;
    let mut asked = false;
    match msg {
        // Answers apply whatever the form is doing, as a load's answer does.
        Msg::Loaded { seq, result, .. } => {
            if form.issues != (IssueList::Loading { seq }) {
                return Vec::new();
            }
            match result {
                Ok((listing, gh)) => form.take_listing(seq, listing, gh),
                Err(FilterFailure::Rejected(messages)) => {
                    form.issues = IssueList::Rejected(messages);
                }
                Err(FilterFailure::Load(error)) => form.issues = IssueList::Failed { error },
            }
        }
        Msg::Searched { seq, result } => {
            let result = result.map_err(|failure| match failure {
                FilterFailure::Load(error) => SearchState::Failed { error },
                FilterFailure::Rejected(messages) => SearchState::Rejected(messages),
            });
            form.take_search_answer(seq, result);
        }
        Msg::LabelsLoaded { seq, result } => {
            let selector = &mut form.github_selector;
            if selector.labels.answer(seq, result.map(|c| vec![c])) {
                selector.rematch_both();
            }
        }
        // Closing a list or the panel is never refused: one left open behind a prompt would
        // reappear.
        Msg::Dismissed(list) => {
            let selector = &mut form.github_selector;
            let field = selector.field_mut(list);
            field.open = false;
            field.query.clear();
            selector.rematch(list);
        }
        // At form open, before anything loads: not a change, so nothing is remembered.
        Msg::Restored(remembered) => {
            form.github_selector = GithubSelector::from_remembered(&remembered);
        }
        Msg::PanelDismissed => form.github_selector.close_lists(),
        Msg::PanelToggled if form.github_selector.panel_open => form.github_selector.close_lists(),
        _ if !editing || !form.shows_github_filters() => return Vec::new(),
        Msg::PanelToggled => form.github_selector.panel_open = true,
        Msg::Focused(list) => {
            let selector = &mut form.github_selector;
            asked = selector.open_field(list, next_filter_seq);
            selector.rematch(list);
        }
        Msg::QueryChanged(list, text) => {
            let selector = &mut form.github_selector;
            asked = selector.open_field(list, next_filter_seq);
            selector.field_mut(list).query = text;
            selector.rematch(list);
        }
        Msg::HighlightMoved(list, direction) => {
            let selector = &mut form.github_selector;
            asked = selector.open_field(list, next_filter_seq);
            selector.rematch(list);
            let rows = selector.rows(list).len();
            let field = selector.field_mut(list);
            if let Some(next) = move_highlight(field.highlight, direction, rows) {
                field.highlight = Some(next);
            }
        }
        Msg::Picked(list, index) => {
            let selector = &mut form.github_selector;
            let Some(row) = selector.rows(list).into_iter().nth(index) else {
                return Vec::new();
            };
            match row.action {
                LabelAction::Choice(label) => reload = selector.choose(list, label),
                LabelAction::Retry => {
                    asked = selector.labels.start(next_filter_seq, true);
                    selector.rematch_both();
                }
                LabelAction::Inert => {}
            }
        }
        Msg::Removed(label) => {
            let selector = &mut form.github_selector;
            let before = selector.basic.clone();
            selector.basic.remove_label(&label);
            reload = selector.basic != before;
            selector.rematch_both();
        }
        Msg::ClearedAll => {
            let selector = &mut form.github_selector;
            reload = !selector.basic.is_default();
            selector.basic = selector.basic.cleared();
            selector.rematch_both();
        }
    }
    let mut outcomes = Vec::new();
    if reload {
        form.issues = IssueList::Loading {
            seq: next_issue_seq,
        };
        form.issue_highlight = None;
        form.github_selector.listed_for = Some(QueryMode::Basic);
        let basic = form.github_selector.basic.clone();
        state.worktree_form.issue_request_seq = next_issue_seq;
        // FR-021: the basic filters are remembered at each change.
        if let Some(project) = state.workspace.active.clone() {
            outcomes.push(crate::features::Outcome::GithubSelectorRemembered {
                project,
                change: SelectorChange::Basic(basic),
            });
        }
    }
    if asked {
        state.worktree_form.filter_request_seq = next_filter_seq;
    }
    outcomes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loaded(names: &[&str]) -> GithubSelector {
        let mut selector = GithubSelector {
            labels: ChoiceLoad::Loaded(vec![LabelChoices {
                labels: names.iter().filter_map(|n| LabelName::parse(n)).collect(),
                not_offered: 0,
                partial: false,
            }]),
            ..GithubSelector::default()
        };
        selector.rematch_both();
        selector
    }

    #[test]
    fn a_label_answer_for_another_seq_is_dropped() {
        let mut selector = GithubSelector::default();
        assert!(selector.open_field(LabelList::Include, 3));
        assert!(!selector.labels.answer(2, Ok(vec![LabelChoices::default()])));
        assert_eq!(selector.awaited_labels(), Some(3));
    }

    #[test]
    fn choosing_the_same_label_again_changes_nothing() {
        let mut selector = loaded(&["bug"]);
        let bug = LabelName::parse("bug").unwrap();
        assert!(selector.choose(LabelList::Include, bug.clone()));
        assert!(!selector.choose(LabelList::Include, bug));
    }

    /// Visual pass M2 F3: the note on labels not offered wraps under a heading, not cut.
    #[test]
    fn the_note_on_labels_not_offered_has_a_heading_to_wrap_under() {
        let row = LabelRow::state(LabelText::NotOffered(1), LabelAction::Inert);
        assert_eq!(row.summary(), Some("Labels not offered"));
        let label = LabelRow::state(
            LabelText::Choice(LabelName::parse("bug").unwrap()),
            LabelAction::Inert,
        );
        assert_eq!(label.summary(), None);
    }

    #[test]
    fn one_label_not_offered_is_named_in_the_singular() {
        let row = LabelRow::state(LabelText::NotOffered(1), LabelAction::Inert);
        let repo =
            micold_core::github::GithubRepo::from_remote_url("git@github.com:o/r.git").unwrap();
        assert_eq!(
            row.label(NoticeSource::from(&repo)),
            "1 label can't be offered because its name holds a quote"
        );
    }
}
