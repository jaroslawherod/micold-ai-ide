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
//! The project remembers its mode, basic filters and last run query (FR-021): the form opens with
//! them (`Msg::Restored`, read by the shell), and each change, switch and accepted run hands back
//! `Outcome::GithubSelectorRemembered`; a draft typed and not run is not remembered. A
//! remembered label the repository no longer has still applies, its chip saying so.
//!
//! US6 adds three single-value fields to the panel: Assignee ("Me", "No assignee", people), Author
//! ("Me", people, a typed login) and Milestone ("No milestone", the open milestones), each led by
//! "Any …", which clears it. The field shows the chosen value's words. The milestones load on the
//! milestone field's first opening; people come from one search of the repository's assignable
//! users per text typed into the assignee or author field, after the debounce (FR-005). A choice
//! reloads and is remembered like any basic change.
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
    Assignee, Author, BasicFilters, FilterFailure, GithubFilter, IssueState, LabelChoices,
    LabelName, Login, Milestone, MilestoneChoices, MilestoneTitle, Person, RememberedSelector,
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
    /// US6: the repository's open milestones, asked for on the milestone field's first opening.
    pub milestones: ChoiceLoad<MilestoneChoices>,
    /// US6: the people the last search found, for the field in `people_for`.
    pub people: ChoiceLoad<Person>,
    /// US6: the people search waiting out its debounce, by its seq.
    pub people_pending: Option<u64>,
    /// US6: the person field the people search is for, assignee or author.
    pub people_for: Option<ValueField>,
    /// US6: the Assignee field.
    pub assignee_field: FilterField,
    /// US6: the Author field.
    pub author_field: FilterField,
    /// US6: the Milestone field.
    pub milestone_field: FilterField,
}

/// Which of the single-value fields (US6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueField {
    /// Whom the issues are assigned to.
    Assignee,
    /// Who opened the issues.
    Author,
    /// Which milestone the issues are in.
    Milestone,
}

impl ValueField {
    /// The three, in the panel's order (contracts/selector-ui.md §2 rows 4–6).
    pub const ALL: [ValueField; 3] = [
        ValueField::Assignee,
        ValueField::Author,
        ValueField::Milestone,
    ];

    /// What the field is called, as its label and its placeholder.
    pub fn title(self) -> &'static str {
        match self {
            ValueField::Assignee => "Assignee",
            ValueField::Author => "Author",
            ValueField::Milestone => "Milestone",
        }
    }

    /// Whether typing into the field searches the repository's people.
    fn searches_people(self) -> bool {
        self != ValueField::Milestone
    }
}

/// What a row of a single-value field does when picked (US6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueAction {
    /// "Any …": clear the field's value.
    Any,
    /// Choose this assignee.
    Assignee(Assignee),
    /// Choose this author.
    Author(Author),
    /// Choose this milestone.
    Milestone(Milestone),
    /// Ask again after a failure.
    Retry,
    /// A state row: nothing to pick.
    Inert,
}

/// What a row of a single-value field says (contracts/selector-ui.md §2 rows 4–6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueText {
    /// "Any assignee", "Any author", "Any milestone".
    Any(ValueField),
    /// The signed-in user.
    Me,
    /// No assignee.
    NoAssignee,
    /// No milestone.
    NoMilestone,
    /// A person the search found.
    Person(Person),
    /// The typed text, a valid login, as the author.
    UseLogin(Login),
    /// An open milestone.
    Milestone(MilestoneTitle),
    /// The milestones are on their way.
    Loading,
    /// The people search waits out its debounce or runs.
    Searching,
    /// The milestones could not be read.
    MilestonesFailed(IssueLoadError),
    /// The people search failed.
    PeopleFailed(IssueLoadError),
    /// **Retry** after a failure.
    Retry,
    /// The repository has no open milestone.
    NoMilestones,
    /// No milestone matches the typed text.
    NoMatch,
    /// The search found no one.
    NoPerson,
    /// This many milestone titles hold a quote and cannot be offered.
    NotOffered(usize),
}

/// One row of a single-value field's list (US6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueRow {
    /// What it says.
    pub text: ValueText,
    /// What picking it does.
    pub action: ValueAction,
    /// The typed text's matches in a milestone title, for emphasis.
    pub spans: Vec<std::ops::Range<usize>>,
}

impl ValueRow {
    fn new(text: ValueText, action: ValueAction) -> ValueRow {
        ValueRow {
            text,
            action,
            spans: Vec::new(),
        }
    }

    fn inert(text: ValueText) -> ValueRow {
        ValueRow::new(text, ValueAction::Inert)
    }

    /// Whether picking the row does anything.
    pub fn enabled(&self) -> bool {
        self.action != ValueAction::Inert
    }

    /// The row's words; a failure is named as 034 names it for `source`.
    pub fn label(&self, source: NoticeSource<'_>) -> String {
        match &self.text {
            ValueText::Any(field) => format!("Any {}", field.title().to_lowercase()),
            ValueText::Me => "Me".to_string(),
            ValueText::NoAssignee => "No assignee".to_string(),
            ValueText::NoMilestone => "No milestone".to_string(),
            ValueText::Person(person) => match &person.name {
                Some(name) => format!("{} ({name})", person.login.as_str()),
                None => person.login.as_str().to_string(),
            },
            ValueText::UseLogin(login) => format!("Use login {}", login.as_str()),
            ValueText::Milestone(title) => title.as_str().to_string(),
            ValueText::Loading => "Loading milestones…".to_string(),
            ValueText::Searching => "Searching people…".to_string(),
            ValueText::MilestonesFailed(error) | ValueText::PeopleFailed(error) => {
                label_failure(error, source)
            }
            ValueText::Retry => "Retry".to_string(),
            ValueText::NoMilestones => "This repository has no open milestones".to_string(),
            ValueText::NoMatch => "No milestone matches".to_string(),
            ValueText::NoPerson => "No person matches".to_string(),
            ValueText::NotOffered(1) => {
                "1 milestone can't be offered because its title holds a quote".to_string()
            }
            ValueText::NotOffered(n) => {
                format!("{n} milestones can't be offered because their titles hold a quote")
            }
        }
    }

    /// A heading above the row's words, for a row whose words wrap in full under it.
    pub fn summary(&self) -> Option<&'static str> {
        match self.text {
            ValueText::MilestonesFailed(_) => Some("Couldn't load the milestones."),
            ValueText::PeopleFailed(_) => Some("Couldn't search people."),
            ValueText::NotOffered(_) => Some("Milestones not offered"),
            _ => None,
        }
    }
}

/// A choice load as `Debug` prints it: its state and how many it holds, never a value.
fn count<T>(load: &ChoiceLoad<T>, held: impl Fn(&T) -> usize) -> String {
    match load {
        ChoiceLoad::NotRequested => "not requested".to_string(),
        ChoiceLoad::Loading { seq } => format!("loading {seq}"),
        ChoiceLoad::Failed { .. } => "failed".to_string(),
        ChoiceLoad::Loaded(choices) => format!("{} held", choices.iter().map(held).sum::<usize>()),
    }
}

impl std::fmt::Debug for GithubSelector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let labels = count(&self.labels, |c| c.labels.len());
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
            .field(
                "milestones",
                &count(&self.milestones, |c| c.milestones.len()),
            )
            .field("people", &count(&self.people, |_| 1))
            .field("people_pending", &self.people_pending)
            .field("assignee_field", &self.assignee_field)
            .field("author_field", &self.author_field)
            .field("milestone_field", &self.milestone_field)
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
    /// The selector a form opens with, from what the project remembers (data-model §10): its mode,
    /// basic filters and last run query, applied as the GitHub source shows (US2 AS1, AS3). The
    /// input starts as the last run query; a query none ran, or an empty one, is no run (FR-013).
    pub fn from_remembered(remembered: &RememberedSelector) -> GithubSelector {
        let last_run = UserQuery::parse(&remembered.query)
            .ok()
            .filter(|query| remembered.ran && !query.as_str().is_empty());
        let mut selector = GithubSelector {
            mode: remembered.mode,
            basic: remembered.basic.clone(),
            draft: last_run
                .as_ref()
                .map(|query| query.as_str().to_string())
                .unwrap_or_default(),
            last_run,
            ..GithubSelector::default()
        };
        for field in ValueField::ALL {
            selector.restore_value(field);
        }
        selector
    }

    /// The mode whose filter the list shown was asked with: `listed_for` once a list was asked
    /// for, else basic mode's (before the source shows). Switching to Advanced leaves the basic list
    /// in place until a query runs (US3 AS1), so what that list is searched beyond with and what it
    /// says follow the list, not the switch. `None` in advanced mode with nothing listed: the prompt
    /// shows, whatever query ran before (review A F3).
    fn listing_mode(&self) -> Option<QueryMode> {
        match (self.listed_for, self.mode) {
            (Some(listed), _) => Some(listed),
            (None, QueryMode::Basic) => Some(QueryMode::Basic),
            (None, QueryMode::Advanced) => None,
        }
    }

    /// The filter a request is made with (data-model §10): the basic filters for a basic list; for
    /// a query's list, the last run query; none while the prompt shows (nothing sent).
    pub fn active_filter(&self) -> Option<GithubFilter> {
        match self.listing_mode()? {
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

    /// Whether the list shown is a query's that holds a `sort:` qualifier: GitHub's order for it,
    /// not the most recently updated first (FR-011, FR-014).
    pub fn sorted_by_query(&self) -> bool {
        self.listing_mode() == Some(QueryMode::Advanced)
            && self
                .last_run
                .as_ref()
                .is_some_and(|query| !query.sorts().is_empty())
    }

    /// The state the list shows: the chosen one for a basic list, none for a query's, where the
    /// query says it.
    pub fn listed_state(&self) -> Option<IssueState> {
        match self.listing_mode() {
            Some(QueryMode::Basic) => Some(self.basic.state()),
            _ => None,
        }
    }

    /// The State row's chips, in order: the state, its words and whether it is the chosen one.
    pub fn state_choices(&self) -> Vec<(IssueState, &'static str, bool)> {
        let chosen = self.basic.state();
        [
            (IssueState::Open, "Open"),
            (IssueState::Closed, "Closed"),
            (IssueState::All, "All"),
        ]
        .into_iter()
        .map(|(state, words)| (state, words, state == chosen))
        .collect()
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
        for field in ValueField::ALL {
            self.value_field_mut(field).open = false;
            self.restore_value(field);
        }
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

// US6: the single-value fields
impl GithubSelector {
    /// `field`'s typeahead state.
    pub fn value_field(&self, field: ValueField) -> &FilterField {
        match field {
            ValueField::Assignee => &self.assignee_field,
            ValueField::Author => &self.author_field,
            ValueField::Milestone => &self.milestone_field,
        }
    }

    fn value_field_mut(&mut self, field: ValueField) -> &mut FilterField {
        match field {
            ValueField::Assignee => &mut self.assignee_field,
            ValueField::Author => &mut self.author_field,
            ValueField::Milestone => &mut self.milestone_field,
        }
    }

    /// The words of `field`'s chosen value, as the field shows it; empty when none is chosen.
    pub fn shown(&self, field: ValueField) -> String {
        let shown = match field {
            ValueField::Assignee => self.basic.assignee().map(|assignee| match assignee {
                Assignee::Me => "Me",
                Assignee::Nobody => "No assignee",
                Assignee::User(login) => login.as_str(),
            }),
            ValueField::Author => self.basic.author().map(|author| match author {
                Author::Me => "Me",
                Author::User(login) => login.as_str(),
            }),
            ValueField::Milestone => self.basic.milestone().map(|milestone| match milestone {
                Milestone::Nobody => "No milestone",
                Milestone::Titled(title) => title.as_str(),
            }),
        };
        shown.unwrap_or_default().to_string()
    }

    /// The text typed into `field` as a search: none while it shows the chosen value's words.
    fn typed(&self, field: ValueField) -> String {
        let query = &self.value_field(field).query;
        if *query == self.shown(field) {
            String::new()
        } else {
            query.trim().to_string()
        }
    }

    fn milestone_choices(&self) -> Option<&MilestoneChoices> {
        match &self.milestones {
            ChoiceLoad::Loaded(choices) => choices.first(),
            _ => None,
        }
    }

    /// The seq the milestones are awaited under, while in flight.
    pub fn awaited_milestones(&self) -> Option<u64> {
        self.milestones.awaited()
    }

    /// The people search in flight (not one waiting out its debounce): its seq and the text it
    /// sends, its field's typed text.
    pub fn awaited_people(&self) -> Option<(u64, String)> {
        match (&self.people, self.people_pending, self.people_for) {
            (ChoiceLoad::Loading { seq }, None, Some(field)) => Some((*seq, self.typed(field))),
            _ => None,
        }
    }

    /// The rows of `field`'s list: "Any …" first, then its fixed choices, then the people found
    /// or the milestones matching the typed text, or the state they are in (selector-ui §2).
    pub fn value_rows(&self, field: ValueField) -> Vec<ValueRow> {
        let mut rows = vec![ValueRow::new(ValueText::Any(field), ValueAction::Any)];
        match field {
            ValueField::Assignee => {
                rows.push(ValueRow::new(
                    ValueText::Me,
                    ValueAction::Assignee(Assignee::Me),
                ));
                rows.push(ValueRow::new(
                    ValueText::NoAssignee,
                    ValueAction::Assignee(Assignee::Nobody),
                ));
                rows.extend(self.people_rows(field));
            }
            ValueField::Author => {
                rows.push(ValueRow::new(
                    ValueText::Me,
                    ValueAction::Author(Author::Me),
                ));
                let people = self.people_rows(field);
                // A typed valid login is offered too, unless the search found that person.
                // GitHub logins are case-insensitive: `OctoCat` typed is `octocat` found.
                let typed = Login::parse(&self.typed(field)).filter(|login| {
                    !people.iter().any(|row| {
                        matches!(&row.action, ValueAction::Author(Author::User(found))
                            if found.as_str().eq_ignore_ascii_case(login.as_str()))
                    })
                });
                rows.extend(people);
                if let Some(login) = typed {
                    rows.push(ValueRow::new(
                        ValueText::UseLogin(login.clone()),
                        ValueAction::Author(Author::User(login)),
                    ));
                }
            }
            ValueField::Milestone => rows.extend(self.milestone_rows()),
        }
        rows
    }

    /// The row of `field`'s list that holds its chosen value, to mark it as 1185's Jira fields
    /// do: "Any …" (row 0) while none is chosen; none while the chosen value is not listed.
    pub fn selected_value_row(&self, field: ValueField) -> Option<usize> {
        let chosen = match field {
            ValueField::Assignee => self.basic.assignee().cloned().map(ValueAction::Assignee),
            ValueField::Author => self.basic.author().cloned().map(ValueAction::Author),
            ValueField::Milestone => self.basic.milestone().cloned().map(ValueAction::Milestone),
        }
        .unwrap_or(ValueAction::Any);
        self.value_rows(field)
            .iter()
            .position(|row| row.action == chosen)
    }

    /// What a person field's list holds after its fixed choices: the people the search for this
    /// field found, or its state.
    fn people_rows(&self, field: ValueField) -> Vec<ValueRow> {
        if self.people_for != Some(field) {
            return Vec::new();
        }
        if self.people_pending.is_some() {
            return vec![ValueRow::inert(ValueText::Searching)];
        }
        let choose = |login: Login| match field {
            ValueField::Author => ValueAction::Author(Author::User(login)),
            _ => ValueAction::Assignee(Assignee::User(login)),
        };
        match &self.people {
            ChoiceLoad::NotRequested => Vec::new(),
            ChoiceLoad::Loading { .. } => vec![ValueRow::inert(ValueText::Searching)],
            ChoiceLoad::Failed { error } => vec![
                ValueRow::inert(ValueText::PeopleFailed(error.clone())),
                ValueRow::new(ValueText::Retry, ValueAction::Retry),
            ],
            ChoiceLoad::Loaded(people) if people.is_empty() => {
                vec![ValueRow::inert(ValueText::NoPerson)]
            }
            ChoiceLoad::Loaded(people) => people
                .iter()
                .map(|person| {
                    ValueRow::new(
                        ValueText::Person(person.clone()),
                        choose(person.login.clone()),
                    )
                })
                .collect(),
        }
    }

    /// What the milestone field's list holds after "Any milestone".
    fn milestone_rows(&self) -> Vec<ValueRow> {
        let choices = match &self.milestones {
            ChoiceLoad::NotRequested | ChoiceLoad::Loading { .. } => {
                return vec![ValueRow::inert(ValueText::Loading)];
            }
            ChoiceLoad::Failed { error } => {
                return vec![
                    ValueRow::inert(ValueText::MilestonesFailed(error.clone())),
                    ValueRow::new(ValueText::Retry, ValueAction::Retry),
                ];
            }
            ChoiceLoad::Loaded(choices) => choices.first().cloned().unwrap_or_default(),
        };
        let mut rows = vec![ValueRow::new(
            ValueText::NoMilestone,
            ValueAction::Milestone(Milestone::Nobody),
        )];
        if choices.milestones.is_empty() {
            rows.push(ValueRow::inert(ValueText::NoMilestones));
        } else {
            let field = &self.milestone_field;
            if field.matches.is_empty() {
                rows.push(ValueRow::inert(ValueText::NoMatch));
            }
            for (index, matched) in &field.matches {
                if let Some(title) = choices.milestones.get(*index) {
                    rows.push(ValueRow {
                        text: ValueText::Milestone(title.clone()),
                        action: ValueAction::Milestone(Milestone::Titled(title.clone())),
                        spans: matched.spans.clone(),
                    });
                }
            }
        }
        if choices.not_offered > 0 {
            rows.push(ValueRow::inert(ValueText::NotOffered(choices.not_offered)));
        }
        rows
    }

    /// Re-rank the milestones against the milestone field's typed text (the people are already
    /// those GitHub found for it), and re-seat `field`'s highlight.
    fn rematch_value(&mut self, field: ValueField) {
        if field == ValueField::Milestone {
            let titles = self
                .milestone_choices()
                .map(|c| c.milestones.clone())
                .unwrap_or_default();
            let typed = self.typed(field);
            self.milestone_field.matches = rank(&titles, |t| t.as_str(), &Query::new(&typed));
        }
        let rows = self.value_rows(field).len();
        let state = self.value_field_mut(field);
        state.highlight = clamp_highlight(state.highlight, rows);
    }

    /// Put `field`'s text back to the chosen value's words, or empty, its highlight starting over;
    /// a people search for it is dropped with the text it was for.
    fn restore_value(&mut self, field: ValueField) {
        let shown = self.shown(field);
        let state = self.value_field_mut(field);
        state.query = shown;
        state.highlight = None;
        self.drop_people(field);
        self.rematch_value(field);
    }

    /// Forget the people found or sought for `field`.
    fn drop_people(&mut self, field: ValueField) {
        if self.people_for == Some(field) {
            self.people = ChoiceLoad::NotRequested;
            self.people_pending = None;
            self.people_for = None;
        }
    }

    /// Open `field`'s list; the milestone field's first opening asks for the milestones under
    /// `seq`. `true` when a request was handed `seq`.
    fn open_value(&mut self, field: ValueField, seq: u64) -> bool {
        self.value_field_mut(field).open = true;
        field == ValueField::Milestone && self.milestones.start(seq, false)
    }

    /// `field`'s text changed: a person field's non-empty text other than the chosen value's
    /// words waits out the debounce under `seq` before it is searched. `true` when `seq` was
    /// handed out (a milestone load or a pending search).
    fn type_value(&mut self, field: ValueField, text: String, seq: u64) -> bool {
        let mut asked = self.open_value(field, seq);
        self.value_field_mut(field).query = text;
        if field.searches_people() {
            if self.typed(field).is_empty() {
                self.drop_people(field);
            } else {
                self.people_for = Some(field);
                self.people_pending = Some(seq);
                asked = true;
            }
        }
        self.rematch_value(field);
        asked
    }

    /// Pick `action` in `field`: choose or clear its value, the list closing on the value's
    /// words; `true` when the filter changed.
    fn choose_value(&mut self, field: ValueField, action: ValueAction) -> bool {
        let before = self.basic.clone();
        match (field, action) {
            (ValueField::Assignee, ValueAction::Any) => self.basic.set_assignee(None),
            (ValueField::Author, ValueAction::Any) => self.basic.set_author(None),
            (ValueField::Milestone, ValueAction::Any) => self.basic.set_milestone(None),
            (_, ValueAction::Assignee(assignee)) => self.basic.set_assignee(Some(assignee)),
            (_, ValueAction::Author(author)) => self.basic.set_author(Some(author)),
            (_, ValueAction::Milestone(milestone)) => self.basic.set_milestone(Some(milestone)),
            (_, ValueAction::Retry | ValueAction::Inert) => return false,
        }
        self.value_field_mut(field).open = false;
        self.restore_value(field);
        self.basic != before
    }
}

impl WorktreeForm {
    /// Whether the open form shows the GitHub filters: the GitHub source is showing with GitHub
    /// issues on, its list asked for (FR-025), or advanced mode's prompt standing in for it.
    pub fn shows_github_filters(&self) -> bool {
        self.jira_site().is_none()
            && self.github_repo().is_some()
            && self.source == BranchSource::Issue
            && (self.issues != IssueList::NotRequested
                || self.github_selector.mode == QueryMode::Advanced)
    }

    /// Whether advanced mode shows on the GitHub source (US3).
    pub fn shows_github_advanced(&self) -> bool {
        self.shows_github_filters() && self.github_selector.mode == QueryMode::Advanced
    }

    /// The GitHub source was shown (data-model §10, "GitHub source shown"): the list then loads
    /// with the mode's filter, so it is the one listed. `true` when advanced mode has no query run
    /// yet and takes it instead: the prompt shows and nothing is sent (FR-013).
    pub(crate) fn github_source_shown(&mut self) -> bool {
        if self.jira_site().is_some() || self.github_repo().is_none() {
            return false;
        }
        let selector = &mut self.github_selector;
        if selector.mode == QueryMode::Advanced && selector.last_run.is_none() {
            selector.listed_for = None;
            return true;
        }
        selector.listed_for = Some(selector.mode);
        false
    }

    /// The switch was pressed (data-model §10). To Advanced: nothing is sent (FR-002); a basic list
    /// still loading is dropped for the prompt, and a search beyond it stops. To Basic: the basic
    /// filters' list loads again when a query's list (or nothing) showed; else the list stays.
    /// `true` when the basic list is to load. The draft is kept either way.
    fn change_github_mode(&mut self, mode: QueryMode) -> bool {
        if self.github_selector.mode == mode {
            return false;
        }
        self.github_selector.mode = mode;
        self.github_selector.close_lists();
        match mode {
            QueryMode::Advanced => {
                if self.stop_basic_work() {
                    self.github_selector.listed_for = None;
                }
                false
            }
            QueryMode::Basic => {
                self.github_selector.listed_for == Some(QueryMode::Advanced)
                    || self.issues == IssueList::NotRequested
            }
        }
    }

    /// Run the draft (US3 AS2, US4 AS5): a query refused is shown in place of the list and never
    /// sent (FR-012); an empty one sends nothing and the prompt shows, as before any run (spec Edge
    /// "Empty advanced query"); one accepted becomes the last run, the draft shows it as cleaned,
    /// and the list loads it. `true` when it is to load.
    fn run_github_query(&mut self) -> bool {
        let selector = &mut self.github_selector;
        selector.listed_for = Some(QueryMode::Advanced);
        self.issue_highlight = None;
        match UserQuery::parse(&selector.draft) {
            Ok(query) if query.as_str().is_empty() => {
                selector.draft.clear();
                selector.last_run = None;
                selector.listed_for = None;
                self.issues = IssueList::NotRequested;
                self.issue_list_open = false;
                false
            }
            Err(refusal) => {
                self.issues = IssueList::Refused(refusal);
                self.issue_list_open = false;
                false
            }
            Ok(query) => {
                selector.draft = query.as_str().to_string();
                selector.last_run = Some(query);
                true
            }
        }
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
    /// A state chip of the State row was pressed (US5).
    StateChosen(IssueState),
    /// **Clear filters**: the default filter.
    ClearedAll,
    /// The Basic/Advanced switch was pressed (US3).
    ModeChanged(QueryMode),
    /// The Query input's text changed; nothing is sent (FR-010).
    DraftChanged(String),
    /// Enter in the Query input, or **Run**: run the draft (US3 AS2, FR-010).
    Run,
    /// US6: a single-value field took focus: its list opens and, the milestone field's first
    /// time, the milestones are asked for.
    ValueFocused(ValueField),
    /// US6: a single-value field's text changed.
    ValueQueryChanged(ValueField, String),
    /// US6: the keyboard moved through a single-value field's list.
    ValueHighlightMoved(ValueField, Direction),
    /// US6: a single-value field's list closed without a pick.
    ValueDismissed(ValueField),
    /// US6: row `index` of a single-value field's list was picked; row 0 is "Any …".
    ValuePicked(ValueField, usize),
    /// US6: the repository's open milestones answered. Applies only while awaited under `seq`.
    MilestonesLoaded {
        seq: u64,
        result: Result<MilestoneChoices, IssueLoadError>,
    },
    /// US6: the people search pending under `seq` waited out its debounce: it starts, when still
    /// pending.
    PeopleSearchDue { seq: u64 },
    /// US6: the people search answered. Applies only while it is the one in flight under `seq`.
    PeopleLoaded {
        seq: u64,
        result: Result<Vec<Person>, IssueLoadError>,
    },
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

/// The filters' reducer. A change of the basic filters, a switch of mode and an accepted run hand
/// back `Outcome::GithubSelectorRemembered` for the shell to write (FR-021).
pub fn update(state: &mut crate::app::State, msg: Msg) -> Vec<crate::features::Outcome> {
    let next_issue_seq = state.worktree_form.issue_request_seq + 1;
    let next_filter_seq = state.worktree_form.filter_request_seq + 1;
    let Some(form) = state.worktree_form.form.as_mut() else {
        return Vec::new();
    };
    let editing = form.status == WorktreeFormStatus::Editing && !form.resolution.is_prompting();
    let mut reload = false;
    let mut run = false;
    let mut asked = false;
    let mut remember = None;
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
        Msg::MilestonesLoaded { seq, result } => {
            let selector = &mut form.github_selector;
            if selector.milestones.answer(seq, result.map(|c| vec![c])) {
                selector.rematch_value(ValueField::Milestone);
            }
        }
        Msg::PeopleLoaded { seq, result } => {
            let selector = &mut form.github_selector;
            // A search typed over while it ran is stale: the newer one is pending.
            if selector.people_pending.is_none() && selector.people.answer(seq, result) {
                if let Some(field) = selector.people_for {
                    selector.rematch_value(field);
                }
            }
        }
        Msg::ValueDismissed(field) => {
            let selector = &mut form.github_selector;
            selector.value_field_mut(field).open = false;
            selector.restore_value(field);
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
        Msg::PeopleSearchDue { seq } => {
            let selector = &mut form.github_selector;
            if selector.people_pending == Some(seq) {
                selector.people_pending = None;
                selector.people = ChoiceLoad::Loading { seq };
            }
        }
        Msg::ValueFocused(field) => {
            let selector = &mut form.github_selector;
            asked = selector.open_value(field, next_filter_seq);
            selector.rematch_value(field);
        }
        Msg::ValueQueryChanged(field, text) => {
            asked = form
                .github_selector
                .type_value(field, text, next_filter_seq);
        }
        Msg::ValueHighlightMoved(field, direction) => {
            let selector = &mut form.github_selector;
            asked = selector.open_value(field, next_filter_seq);
            selector.rematch_value(field);
            let rows = selector.value_rows(field).len();
            let state = selector.value_field_mut(field);
            if let Some(next) = move_highlight(state.highlight, direction, rows) {
                state.highlight = Some(next);
            }
        }
        Msg::ValuePicked(field, index) => {
            let selector = &mut form.github_selector;
            let Some(row) = selector.value_rows(field).into_iter().nth(index) else {
                return Vec::new();
            };
            match (row.action, field) {
                (ValueAction::Retry, ValueField::Milestone) => {
                    asked = selector.milestones.start(next_filter_seq, true);
                    selector.rematch_value(field);
                }
                (ValueAction::Retry, _) => {
                    asked = selector.people.start(next_filter_seq, true);
                    selector.rematch_value(field);
                }
                (action, _) => reload = selector.choose_value(field, action),
            }
        }
        Msg::StateChosen(state) => {
            let selector = &mut form.github_selector;
            reload = selector.basic.state() != state;
            selector.basic.set_state(state);
        }
        Msg::ModeChanged(mode) => {
            let before = form.github_selector.mode;
            remember = (before != mode).then_some(SelectorChange::Mode(mode));
            if form.change_github_mode(mode) {
                // Not remembered as a basic change: the basic filters did not change.
                form.issues = IssueList::Loading {
                    seq: next_issue_seq,
                };
                form.issue_highlight = None;
                form.github_selector.listed_for = Some(QueryMode::Basic);
                state.worktree_form.issue_request_seq = next_issue_seq;
            }
        }
        Msg::DraftChanged(text) => form.github_selector.draft = text,
        Msg::Run if form.github_selector.mode == QueryMode::Advanced => {
            // A refusal takes a fresh seq too, so an answer still in flight cannot replace it.
            state.worktree_form.issue_request_seq = next_issue_seq;
            run = form.run_github_query();
            // FR-021: an accepted run only; a refused or an empty one is not a run.
            if run {
                if let Some(query) = &form.github_selector.last_run {
                    remember = Some(SelectorChange::Query(query.as_str().to_string()));
                }
            }
        }
        Msg::Run => {}
        Msg::ClearedAll => {
            let selector = &mut form.github_selector;
            reload = !selector.basic.is_default();
            selector.basic = selector.basic.cleared();
            selector.rematch_both();
            for field in ValueField::ALL {
                selector.restore_value(field);
            }
        }
    }
    if run {
        form.issues = IssueList::Loading {
            seq: next_issue_seq,
        };
    }
    let mut outcomes = Vec::new();
    if let (Some(change), Some(project)) = (remember, state.workspace.active.clone()) {
        outcomes.push(crate::features::Outcome::GithubSelectorRemembered { project, change });
    }
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
