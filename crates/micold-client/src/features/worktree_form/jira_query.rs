//! Advanced JQL in the Jira issue selector (feature 1186): the selector's advanced state, one
//! field of the add-worktree form (`specs/1186-jira-advanced-jql/data-model.md` §6, §7).
//!
//! The rules for the query itself (cleaning, the checks, the scope) are `micold_core::jira_query`;
//! this module holds what the selector shows: the mode, the draft as typed, the last run query and
//! where the run is. A sub-state of the form rather than a feature of its own: its transitions are
//! applied by `worktree_form`, which owns the seq counter every run is numbered from, so a reset
//! that already makes a basic load stale makes a run stale too (research R9).
//!
//! Render-free, like every module here. Nothing in this module logs or notifies: the draft, the last
//! run query and the scoped query never leave the form's state except as the request (FR-016).

use micold_core::github::{Issue, IssueListing};
use micold_core::jira::ProjectKey;
use micold_core::jira_query::{QueryFailure, QueryMode, QueryRefusal, ScopedQuery, UserQuery};
use micold_core::typeahead::{move_highlight, Direction};

use super::{BranchSource, IssueList, JiraSite, SearchState, WorktreeForm};

/// The selector's advanced state (data-model §6).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct JiraQuery {
    /// The mode shown. Changed only by the switch.
    pub mode: QueryMode,
    /// The query input's text as typed (FR-003). Starts as `last_run`.
    pub draft: String,
    /// The project's last run query.
    pub last_run: String,
    /// Where the advanced list is.
    pub run: QueryRun,
    /// The keyboard row, an index into the shown results.
    pub highlight: Option<usize>,
    /// Whether the result list shows.
    pub list_open: bool,
}

/// Where the advanced list is (data-model §6).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum QueryRun {
    /// Nothing run since the Jira source was shown: the prompt, or the list shown before the
    /// switch until a query runs (US1 AS1).
    #[default]
    Idle,
    /// The query was not sent (FR-007).
    Refused(QueryRefusal),
    /// The scoped query is running; only the result carrying `seq` applies.
    Running { seq: u64, scoped: ScopedQuery },
    /// The scoped query listed nothing because it failed.
    Failed {
        scoped: ScopedQuery,
        failure: QueryFailure,
    },
    /// The issues the scoped query returned, in Jira's order (FR-006); empty is "no issue matches"
    /// (FR-012).
    Loaded {
        scoped: ScopedQuery,
        listing: IssueListing,
    },
}

/// The selector's advanced messages (data-model §7), as `worktree_form::Msg::JiraQuery`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Msg {
    /// The Basic/Advanced switch was pressed (FR-001). Never runs anything (FR-004a).
    ModeChanged(QueryMode),
    /// The query input changed. Never runs anything (FR-004).
    DraftChanged(String),
    /// Enter in the query input or the run control (FR-004).
    Run,
    /// A run answered. Applies only while the selector awaits `seq`.
    Ran {
        seq: u64,
        result: Result<IssueListing, QueryFailure>,
    },
    /// The query input took focus.
    Focused,
    /// The keyboard moved through the results.
    HighlightMoved(Direction),
    /// The result list closed without a pick.
    Dismissed,
    /// Row `index` of the shown results was picked; the shell resolves it to its issue through
    /// `worktree_form::State::issue_number_at`, as for a basic row (FR-009).
    RowPicked(usize),
}

/// The selector's notice of what advanced mode reads and sends (FR-015, contracts/selector-ui.md
/// §4). It names the query, never its text.
pub fn notice(site: &JiraSite) -> String {
    format!(
        "Reads issues of project {} matching your query from {}. Sends the project key, your query and your credentials, and, with this dialog closed, the keys of the tickets your worktrees are linked to, to show their status.",
        site.key, site.origin
    )
}

impl JiraQuery {
    /// Whether advanced mode is the mode shown.
    pub fn is_advanced(&self) -> bool {
        self.mode == QueryMode::Advanced
    }

    /// The seq and scoped query of the run in flight, if any.
    pub fn running(&self) -> Option<(u64, &ScopedQuery)> {
        match &self.run {
            QueryRun::Running { seq, scoped } => Some((*seq, scoped)),
            _ => None,
        }
    }

    /// The issues advanced mode shows in place of the basic list: the loaded results, none while a
    /// run is refused, running or failed, and `None` while `Idle`, when the list shown before the
    /// switch stays until a query runs (US1 AS1).
    pub fn results(&self) -> Option<&[Issue]> {
        match &self.run {
            QueryRun::Idle => None,
            QueryRun::Loaded { listing, .. } => Some(&listing.issues),
            _ => Some(&[]),
        }
    }

    /// The Jira source was shown (FR-004a, FR-003a): in advanced mode, run the last run query under
    /// `seq` when there is one, else show the prompt. `false` in basic mode, where the caller loads
    /// the basic list instead; while advanced mode shows the basic list is never loaded.
    pub fn source_shown(&mut self, site: &JiraSite, seq: u64) -> bool {
        if !self.is_advanced() {
            return false;
        }
        self.run = QueryRun::Idle;
        if !self.last_run.trim().is_empty() {
            if let Ok(query) = UserQuery::parse(&self.last_run) {
                self.start(site, &query, seq);
            }
        }
        true
    }

    /// The source was left or its list forgotten: a run in flight becomes stale, the results go,
    /// and the draft is the last run query again (FR-003a). The mode is kept.
    pub fn reset(&mut self) {
        self.run = QueryRun::Idle;
        self.draft = self.last_run.clone();
        self.highlight = None;
        self.list_open = false;
    }

    /// Run the draft (FR-004): a query the checks refuse is not sent (FR-007); otherwise the
    /// cleaned text becomes the draft and the last run query, and the scoped query runs under
    /// `seq`. `true` when a run started, so the caller hands `seq` out.
    pub fn run_draft(&mut self, site: &JiraSite, seq: u64) -> bool {
        self.highlight = None;
        self.list_open = true;
        match UserQuery::parse(&self.draft) {
            Err(refusal) => {
                self.run = QueryRun::Refused(refusal);
                false
            }
            Ok(query) => {
                self.draft = query.as_str().to_string();
                self.last_run = self.draft.clone();
                self.start(site, &query, seq)
            }
        }
    }

    /// A run answered: only the awaited one applies.
    pub fn ran(&mut self, seq: u64, result: Result<IssueListing, QueryFailure>) {
        let QueryRun::Running {
            seq: awaited,
            scoped,
        } = &self.run
        else {
            return;
        };
        if *awaited != seq {
            return;
        }
        let scoped = scoped.clone();
        self.highlight = None;
        self.run = match result {
            Ok(listing) => {
                self.list_open = true;
                QueryRun::Loaded { scoped, listing }
            }
            Err(failure) => QueryRun::Failed { scoped, failure },
        };
    }

    /// The keyboard moved through the shown results (saturating, as the basic list).
    pub fn move_highlight(&mut self, direction: Direction) {
        let shown = self.results().map_or(0, <[Issue]>::len);
        if let Some(next) = move_highlight(self.highlight, direction, shown) {
            self.highlight = Some(next);
        }
    }

    /// Run `query` scoped to `site`'s project under `seq`. A key that does not parse (never, for
    /// a connected site) runs nothing.
    fn start(&mut self, site: &JiraSite, query: &UserQuery, seq: u64) -> bool {
        let Ok(key) = ProjectKey::parse(&site.key) else {
            return false;
        };
        self.run = QueryRun::Running {
            seq,
            scoped: ScopedQuery::new(&key, query),
        };
        true
    }
}

/// Apply `msg` to the open form (data-model §6 transitions). `true` when it started a run or a
/// basic load under `next_seq`, or made one in flight stale, so the caller hands `next_seq` out.
pub fn update(form: &mut WorktreeForm, msg: Msg, next_seq: u64) -> bool {
    match msg {
        Msg::ModeChanged(mode) => form.change_query_mode(mode, next_seq),
        Msg::DraftChanged(text) => {
            if form.shows_advanced() {
                form.jira_query.draft = text;
                form.jira_query.list_open = true;
            }
            false
        }
        Msg::Run => match form.jira_site().cloned() {
            Some(site) if form.shows_advanced() => form.jira_query.run_draft(&site, next_seq),
            _ => false,
        },
        Msg::Ran { seq, result } => {
            form.jira_query.ran(seq, result);
            false
        }
        Msg::Focused => {
            form.jira_query.list_open = form.shows_advanced();
            false
        }
        Msg::HighlightMoved(direction) => {
            form.jira_query.move_highlight(direction);
            false
        }
        Msg::Dismissed => {
            form.jira_query.list_open = false;
            false
        }
        // Resolved by the shell into a pick, as a basic row is.
        Msg::RowPicked(_) => false,
    }
}

impl WorktreeForm {
    /// Whether the selector offers the Basic/Advanced switch: only while the Jira source is showing,
    /// never for GitHub or with the integration off (FR-001).
    pub fn offers_query_modes(&self) -> bool {
        self.source == BranchSource::Issue && self.jira_site().is_some()
    }

    /// Whether advanced mode is what the selector shows.
    pub fn shows_advanced(&self) -> bool {
        self.offers_query_modes() && self.jira_query.is_advanced()
    }

    /// The issues the selector shows, which a row index and a pick address: the advanced results
    /// while advanced mode shows them, the held basic issues otherwise (contracts/selector-ui.md §5).
    pub fn shown_issues(&self) -> Vec<&Issue> {
        match self.advanced_results() {
            Some(results) => results.iter().collect(),
            None => self.issues.held(),
        }
    }

    /// Every issue the form holds, basic and advanced, whichever list shows: what a pick made
    /// earlier is looked up in, so switching the mode does not lose its title (feature 1186).
    pub(crate) fn held_issues(&self) -> impl Iterator<Item = &Issue> {
        let advanced = match &self.jira_query.run {
            QueryRun::Loaded { listing, .. } => listing.issues.as_slice(),
            _ => &[],
        };
        self.issues.held().into_iter().chain(advanced)
    }

    /// The advanced results shown in place of the basic list; `None` in basic mode, and while no
    /// query has run since the switch, when the basic list stays (US1 AS1).
    pub(crate) fn advanced_results(&self) -> Option<&[Issue]> {
        if self.shows_advanced() {
            self.jira_query.results()
        } else {
            None
        }
    }

    /// The Jira source was shown under `seq` (FR-004a, FR-003a). `true` when advanced mode took it,
    /// so the basic list is not loaded.
    pub(crate) fn jira_query_shown(&mut self, seq: u64) -> bool {
        match self.jira_site().cloned() {
            Some(site) => self.jira_query.source_shown(&site, seq),
            None => false,
        }
    }

    /// The switch was pressed. To advanced: nothing runs (FR-004a), a basic load or search in flight
    /// is discarded, and the list keeps what it showed (US1 AS1). To basic: a run in flight is
    /// discarded and the basic list, search text and matches are as left (FR-003), loaded under
    /// `next_seq` when they were never requested. The draft is kept for the next switch.
    fn change_query_mode(&mut self, mode: QueryMode, next_seq: u64) -> bool {
        if !self.offers_query_modes() || self.jira_query.mode == mode {
            return false;
        }
        self.jira_query.mode = mode;
        self.jira_query.highlight = None;
        self.jira_query.list_open = false;
        self.issue_list_open = false;
        match mode {
            QueryMode::Advanced => {
                if let IssueList::Loaded { search, .. } = &mut self.issues {
                    if matches!(
                        search,
                        SearchState::Pending { .. } | SearchState::Searching { .. }
                    ) {
                        *search = SearchState::Idle;
                    }
                }
                let loading = matches!(self.issues, IssueList::Loading { .. });
                if loading {
                    self.issues = IssueList::NotRequested;
                }
                loading
            }
            QueryMode::Basic => {
                let running = self.jira_query.running().is_some();
                if running {
                    self.jira_query.run = QueryRun::Idle;
                }
                if self.issues == IssueList::NotRequested {
                    self.issues = IssueList::Loading { seq: next_seq };
                    return true;
                }
                running
            }
        }
    }
}
