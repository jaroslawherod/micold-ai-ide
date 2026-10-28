//! Filters in the GitHub issue selector (feature 1400): the render-free rules for the basic filters
//! and the advanced query, the only search text a filtered request sends, the filtered load, the
//! label choices and the per-project remembered selector.
//!
//! Every filter becomes a [`ScopedSearch`] here before anything is sent
//! (`specs/1400-github-selector-filters/contracts/search-rules.md`): a basic value is quoted and
//! validated, a user query is cleaned and refused when it names a scope, leaves a quote open or
//! unbalances its parentheses, so no filter can leave `repo:<owner>/<name> is:issue`. Pure functions
//! but for [`load_filtered`] and [`load_label_choices`], which go through an [`IssueSource`].
//!
//! Nothing here logs: every type holding a filter value or query text has a `Debug` that prints
//! `<redacted>` for it (FR-024, research R15).

use std::fmt;
use std::ops::Range;

use serde::{Deserialize, Deserializer, Serialize};

use crate::github::{
    answer_of, graphql_error, graphql_error_of, issue_from_node, issue_node_selection, load_pages,
    next_cursor_of, GithubRepo, IssueListing, IssueLoadError, IssuePage, IssueSource, NamePage,
};
use crate::jira_query::QueryMode;
use crate::query_text::{clean, cut_message};

const REDACTED: &str = "<redacted>";

/// Characters a user query holds at most, counted after cleaning (FR-012).
pub const MAX_QUERY_CHARS: usize = 1_000;

/// The sort every scoped search without one of the user's ends with (FR-011).
const DEFAULT_SORT: &str = "sort:updated-desc";

/// Shown when GitHub refused the search and gave no message of its own (FR-018, US4 AS4).
pub const NOT_ACCEPTED: &str = "GitHub did not accept the search query.";

/// The qualifiers that would name another scope (research R5), matched ASCII case-insensitively at
/// a token start, with or without a leading `-`.
const SCOPE_QUALIFIERS: [&str; 4] = ["repo:", "org:", "user:", "owner:"];

/// GitHub's messages kept at most (research R7).
const MAX_MESSAGES: usize = 10;

/// Label pages [`load_label_choices`] reads at most: 1,000 labels (research R10).
pub const LABEL_PAGE_CAP: usize = 10;

// §5 Values

/// A label name that can be sent as `label:"<name>"` (data-model §1, research R6). Constructed
/// only by [`LabelName::parse`]: non-empty, no `"` (GitHub's search has no escape inside quotes),
/// no `\` (in case GitHub reads `\"` as an escape, which would leave the quote open, as typed
/// words drop it) and no control character.
#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct LabelName(String);

impl LabelName {
    /// `name` as a label, kept exactly as given; `None` when it cannot be quoted.
    pub fn parse(name: &str) -> Option<LabelName> {
        let quotable = !name.is_empty()
            && !name
                .chars()
                .any(|c| matches!(c, '"' | '\\') || c.is_control());
        quotable.then(|| LabelName(name.to_string()))
    }

    /// The name as GitHub spells it.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for LabelName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(REDACTED)
    }
}

/// Which issues by state a basic filter lists (data-model §1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IssueState {
    /// Open issues only: today's listing.
    #[default]
    Open,
    /// Closed issues only.
    Closed,
    /// Open and closed issues.
    All,
}

// Data-model §2: the basic filter set

/// The basic filters (data-model §2). Fields are private and change only through the setters, so
/// the two label lists stay free of duplicates and disjoint.
#[derive(Clone, PartialEq, Eq, Default, Serialize)]
pub struct BasicFilters {
    include: Vec<LabelName>,
    exclude: Vec<LabelName>,
    state: IssueState,
}

impl BasicFilters {
    /// Require `label`; it stops being excluded (spec Edge "Same label included and excluded").
    pub fn include_label(&mut self, label: LabelName) {
        self.exclude.retain(|held| *held != label);
        if !self.include.contains(&label) {
            self.include.push(label);
        }
    }

    /// Exclude `label`; it stops being required.
    pub fn exclude_label(&mut self, label: LabelName) {
        self.include.retain(|held| *held != label);
        if !self.exclude.contains(&label) {
            self.exclude.push(label);
        }
    }

    /// Drop `label` from whichever list holds it.
    pub fn remove_label(&mut self, label: &LabelName) {
        self.include.retain(|held| held != label);
        self.exclude.retain(|held| held != label);
    }

    /// List issues in `state`.
    pub fn set_state(&mut self, state: IssueState) {
        self.state = state;
    }

    /// The default filter, as **Clear filters** leaves it.
    pub fn cleared(&self) -> BasicFilters {
        BasicFilters::default()
    }

    /// The required labels, in chosen order.
    pub fn include(&self) -> &[LabelName] {
        &self.include
    }

    /// The excluded labels, in chosen order.
    pub fn exclude(&self) -> &[LabelName] {
        &self.exclude
    }

    /// The chosen state.
    pub fn state(&self) -> IssueState {
        self.state
    }

    /// No label and open issues: today's listing (FR-009).
    pub fn is_default(&self) -> bool {
        self.include.is_empty() && self.exclude.is_empty() && self.state == IssueState::Open
    }

    /// What the **Filters** control counts: one per label, one for a state other than open.
    pub fn chosen_count(&self) -> usize {
        self.include.len() + self.exclude.len() + usize::from(self.state != IssueState::Open)
    }

    /// The active line (FR-007); `None` for the default filter.
    pub fn described(&self) -> Option<String> {
        let names = |labels: &[LabelName]| {
            labels
                .iter()
                .map(LabelName::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        };
        let mut parts = Vec::new();
        if !self.include.is_empty() {
            parts.push(format!("Labels {}", names(&self.include)));
        }
        if !self.exclude.is_empty() {
            parts.push(format!("Excluding labels {}", names(&self.exclude)));
        }
        match self.state {
            IssueState::Open => {}
            IssueState::Closed => parts.push("Closed issues".to_string()),
            IssueState::All => parts.push("Open and closed issues".to_string()),
        }
        (!parts.is_empty()).then(|| parts.join("; "))
    }

    /// Read leniently (data-model §2): an entry that is not a quotable label is dropped, a
    /// duplicate kept once, a label in both lists kept in `include` only, an unknown state is open.
    fn from_value(value: &serde_json::Value) -> BasicFilters {
        let labels = |key: &str| {
            value[key]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(serde_json::Value::as_str)
                .filter_map(LabelName::parse)
                .collect::<Vec<_>>()
        };
        let mut filters = BasicFilters::default();
        for label in labels("exclude") {
            filters.exclude_label(label);
        }
        for label in labels("include") {
            filters.include_label(label);
        }
        filters.state = IssueState::deserialize(&value["state"]).unwrap_or_default();
        filters
    }
}

impl<'de> Deserialize<'de> for BasicFilters {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        Ok(BasicFilters::from_value(&value))
    }
}

impl fmt::Debug for BasicFilters {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BasicFilters")
            .field("include", &self.include)
            .field("exclude", &self.exclude)
            .field("state", &self.state)
            .finish()
    }
}

// Data-model §3: the advanced query

/// Why the app did not send a user query (FR-012, data-model §3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryRefusal {
    /// The cleaned query has more than [`MAX_QUERY_CHARS`] characters.
    TooLong,
    /// The query ends inside a quoted string.
    UnclosedQuote,
    /// The query holds `repo:`, `org:`, `user:` or `owner:` outside quoted strings.
    ScopeQualifier,
    /// The parentheses outside quoted strings do not balance.
    UnbalancedParentheses,
}

impl QueryRefusal {
    /// The selector's text for this refusal; it never contains the query (FR-024).
    pub fn message(&self) -> &'static str {
        match self {
            Self::TooLong => {
                "The query is longer than 1,000 characters. Shorten it and run it again."
            }
            Self::UnclosedQuote => {
                "The query has a quote that is not closed. Close it and run it again."
            }
            Self::ScopeQualifier => {
                "The query names a repository, organization or user. The selector already \
                 searches this repository's issues; remove repo:, org:, user: or owner: and run it \
                 again."
            }
            Self::UnbalancedParentheses => {
                "The query's parentheses do not match. Fix them and run it again."
            }
        }
    }
}

/// A cleaned user query that passed the checks (data-model §3). Constructed only by
/// [`UserQuery::parse`], so a query that could leave the repository scope cannot be held.
#[derive(Clone, PartialEq, Eq)]
pub struct UserQuery {
    text: String,
    /// Byte ranges of each `sort:` token outside quoted strings, in order.
    sorts: Vec<Range<usize>>,
}

impl UserQuery {
    /// Clean `raw`, refuse it when too long, then scan it once (research R5).
    pub fn parse(raw: &str) -> Result<UserQuery, QueryRefusal> {
        let text = clean(raw);
        if text.chars().count() > MAX_QUERY_CHARS {
            return Err(QueryRefusal::TooLong);
        }
        let sorts = scan(&text)?;
        Ok(UserQuery { text, sorts })
    }

    /// The query without its sort tokens, whitespace runs outside quoted strings collapsed to one
    /// space, trimmed.
    pub fn condition(&self) -> String {
        let mut cut = String::with_capacity(self.text.len());
        let mut from = 0;
        for sort in &self.sorts {
            cut.push_str(&self.text[from..sort.start]);
            from = sort.end;
        }
        cut.push_str(&self.text[from..]);
        let mut condition = String::with_capacity(cut.len());
        let mut quoted = false;
        for c in cut.chars() {
            if c == '"' {
                quoted = !quoted;
            }
            if !quoted && c.is_whitespace() {
                if !condition.ends_with(' ') {
                    condition.push(' ');
                }
            } else {
                condition.push(c);
            }
        }
        condition.trim().to_string()
    }

    /// The sort tokens as typed, in order.
    pub fn sorts(&self) -> Vec<&str> {
        self.sorts
            .iter()
            .map(|sort| &self.text[sort.clone()])
            .collect()
    }

    /// The cleaned text: what runs, what the input shows after a run and what is remembered.
    pub fn as_str(&self) -> &str {
        &self.text
    }
}

impl fmt::Debug for UserQuery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("UserQuery").field(&REDACTED).finish()
    }
}

/// The one left-to-right pass of research R5: the `sort:` tokens outside quoted strings, or why the
/// query is refused. GitHub's search has no escape inside quotes, so the next `"` closes. At each
/// position outside a string after any character but a letter, digit, `_` or `-` (so `(a)repo:x`
/// and `"x"repo:x` are caught, `myrepo:x` is not) it looks for a scope qualifier, with or without
/// `-`. At each token start (the start, or after whitespace or `(`) it looks for a `sort:` token,
/// which ends before whitespace, a parenthesis or a quote. Refusals rank `UnclosedQuote` > `ScopeQualifier` >
/// `UnbalancedParentheses`.
fn scan(text: &str) -> Result<Vec<Range<usize>>, QueryRefusal> {
    let mut quoted = false;
    let mut depth: i32 = 0;
    let mut went_below = false;
    let mut scope = false;
    let mut sorts = Vec::new();
    let mut previous: Option<char> = None;
    for (at, c) in text.char_indices() {
        if quoted {
            quoted = c != '"';
            previous = Some(c);
            continue;
        }
        let rest = &text[at..];
        if previous.is_none_or(|p| !(p.is_alphanumeric() || matches!(p, '_' | '-'))) {
            let unsigned = rest.strip_prefix('-').unwrap_or(rest);
            scope |= SCOPE_QUALIFIERS
                .iter()
                .any(|qualifier| starts_with_ignoring_case(unsigned, qualifier));
        }
        if previous.is_none_or(|p| p.is_whitespace() || p == '(')
            && starts_with_ignoring_case(rest, "sort:")
        {
            let len = rest
                .find(|c: char| c.is_whitespace() || matches!(c, '(' | ')' | '"'))
                .unwrap_or(rest.len());
            sorts.push(at..at + len);
        }
        match c {
            '"' => quoted = true,
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                went_below |= depth < 0;
            }
            _ => {}
        }
        previous = Some(c);
    }
    if quoted {
        Err(QueryRefusal::UnclosedQuote)
    } else if scope {
        Err(QueryRefusal::ScopeQualifier)
    } else if went_below || depth != 0 {
        Err(QueryRefusal::UnbalancedParentheses)
    } else {
        Ok(sorts)
    }
}

/// Whether `text` starts with `prefix`, ASCII case-insensitively.
fn starts_with_ignoring_case(text: &str, prefix: &str) -> bool {
    text.get(..prefix.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
}

// Data-model §4: what a request filters by

/// The filter a filtered request lists by (data-model §4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GithubFilter {
    /// The basic filters.
    Basic(BasicFilters),
    /// A run advanced query.
    Advanced(UserQuery),
}

impl GithubFilter {
    /// The default basic filter: the caller takes today's listing and search (FR-009) and never
    /// builds a [`ScopedSearch`] for it.
    pub fn is_default(&self) -> bool {
        matches!(self, GithubFilter::Basic(filters) if filters.is_default())
    }
}

/// The only search text a filtered request sends as `q` (research R4). Constructed only by
/// [`ScopedSearch::listing`] and [`ScopedSearch::beyond`], so every text starts with the
/// repository scope and holds the user's text only where it cannot widen it.
#[derive(Clone, PartialEq, Eq)]
pub struct ScopedSearch {
    text: String,
}

impl ScopedSearch {
    /// The text of a filtered listing (search-rules §4): the scope, the filter, then the user's
    /// sort qualifiers or `sort:updated-desc`. For the default basic filter the caller sends
    /// today's listing instead ([`GithubFilter::is_default`]).
    pub fn listing(repo: &GithubRepo, filter: &GithubFilter) -> ScopedSearch {
        let sort = match filter {
            GithubFilter::Advanced(query) if !query.sorts.is_empty() => query.sorts().join(" "),
            _ => DEFAULT_SORT.to_string(),
        };
        ScopedSearch {
            text: format!("{} {sort}", scope(repo, filter)),
        }
    }

    /// The text of a search beyond a filtered list (FR-016, search-rules §4): the listing's text
    /// without its sort, then each typed word quoted, with every `"` and `\` removed, so the typed
    /// text can only narrow the filter. `None` when no word is left: nothing is sent.
    pub fn beyond(repo: &GithubRepo, filter: &GithubFilter, typed: &str) -> Option<ScopedSearch> {
        let typed = clean(typed).replace(['"', '\\'], "");
        let mut words = typed.split_whitespace().peekable();
        words.peek()?;
        let mut text = scope(repo, filter);
        for word in words {
            text.push_str(&format!(" \"{word}\""));
        }
        Some(ScopedSearch { text })
    }

    /// The text as sent.
    pub fn as_str(&self) -> &str {
        &self.text
    }
}

impl fmt::Debug for ScopedSearch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ScopedSearch").field(&REDACTED).finish()
    }
}

/// `repo:<owner>/<name> is:issue` and the filter, without a sort. The repository is validated at
/// construction (no space, quote or colon), so the scope term cannot be broken by its name.
fn scope(repo: &GithubRepo, filter: &GithubFilter) -> String {
    let mut text = format!("repo:{repo} is:issue");
    match filter {
        GithubFilter::Basic(filters) => {
            text.push_str(match filters.state {
                IssueState::Open => " is:open",
                IssueState::Closed => " is:closed",
                IssueState::All => "",
            });
            for label in &filters.include {
                text.push_str(&format!(" label:\"{}\"", label.as_str()));
            }
            for label in &filters.exclude {
                text.push_str(&format!(" -label:\"{}\"", label.as_str()));
            }
        }
        GithubFilter::Advanced(query) => {
            let condition = query.condition();
            if !condition.is_empty() {
                text.push_str(&format!(" ({condition})"));
            }
        }
    }
    text
}

// Data-model §5, search-rules §6: answers

/// Why a filtered request listed nothing (data-model §5).
#[derive(Clone, PartialEq, Eq)]
pub enum FilterFailure {
    /// GitHub refused the search: its messages, cleaned and cut, possibly none.
    Rejected(Vec<String>),
    /// Any other failure, classified as the default listing's (034 FR-007).
    Load(IssueLoadError),
}

impl FilterFailure {
    /// The lines the selector shows for a refusal ([`NOT_ACCEPTED`] when GitHub gave none);
    /// `None` for a load failure, which shows 034's notice.
    pub fn rejection(&self) -> Option<Vec<String>> {
        match self {
            FilterFailure::Rejected(messages) if messages.is_empty() => {
                Some(vec![NOT_ACCEPTED.to_string()])
            }
            FilterFailure::Rejected(messages) => Some(messages.clone()),
            FilterFailure::Load(_) => None,
        }
    }
}

impl From<IssueLoadError> for FilterFailure {
    fn from(error: IssueLoadError) -> Self {
        FilterFailure::Load(error)
    }
}

/// GitHub's messages can quote the query, so `Debug` counts them (FR-024).
impl fmt::Debug for FilterFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FilterFailure::Rejected(messages) => f
                .debug_tuple("Rejected")
                .field(&format_args!("{} message(s)", messages.len()))
                .finish(),
            FilterFailure::Load(error) => f.debug_tuple("Load").field(error).finish(),
        }
    }
}

/// The search type of every filtered request (research R3): github.com's issue search syntax,
/// with `AND`, `OR` and parentheses. A macro, so `concat!` can put it into each query.
macro_rules! search_type {
    () => {
        "ISSUE_ADVANCED"
    };
}

/// What a filtered search selects: the count, the page, and each issue node with its body, state
/// and repository (search-rules §6).
macro_rules! search_selection {
    () => {
        concat!(
            "issueCount pageInfo { hasNextPage endCursor } nodes { ... on Issue { ",
            issue_node_selection!(),
            " bodyText state repository { nameWithOwner } } } } }"
        )
    };
}

/// The search type filtered requests send (research R3).
pub const SEARCH_TYPE: &str = search_type!();

/// The GraphQL document for one page of a filtered listing. One line, like every query.
pub const SEARCH_PAGE_QUERY: &str = concat!(
    "query($q: String!, $cursor: String) { search(type: ",
    search_type!(),
    ", query: $q, first: 100, after: $cursor) { ",
    search_selection!()
);

/// The GraphQL document for a search beyond a filtered list: the first 50, no cursor.
pub const SEARCH_FILTERED_QUERY: &str = concat!(
    "query($q: String!) { search(type: ",
    search_type!(),
    ", query: $q, first: 50) { ",
    search_selection!()
);

/// The arguments after `gh` for one page of a filtered listing: only the scoped text and the
/// cursor leave the machine, each a raw string (`-f`) (contracts/source-and-storage.md §1, §2).
pub fn search_page_args(search: &ScopedSearch, cursor: Option<&str>) -> Vec<String> {
    let mut args = crate::github::graphql_args(SEARCH_PAGE_QUERY);
    args.extend(["-f".to_string(), format!("q={}", search.as_str())]);
    if let Some(cursor) = cursor {
        args.extend(["-f".to_string(), format!("cursor={cursor}")]);
    }
    args
}

/// The arguments after `gh` for a search beyond a filtered list: only the scoped text.
pub fn search_filtered_args(search: &ScopedSearch) -> Vec<String> {
    let mut args = crate::github::graphql_args(SEARCH_FILTERED_QUERY);
    args.extend(["-f".to_string(), format!("q={}", search.as_str())]);
    args
}

/// Parse `gh api graphql` stdout for [`SEARCH_PAGE_QUERY`] or [`SEARCH_FILTERED_QUERY`]
/// (search-rules §6, research R7).
///
/// `RATE_LIMITED`, `FORBIDDEN` and `INSUFFICIENT_SCOPES` are load failures, classified as the
/// default listing classifies them (034's SAML and scope notices). Every other error at `["search"]`, or without a path when
/// there is no `data.search`, is GitHub refusing the query: their messages, in order, become
/// [`FilterFailure::Rejected`]. Any other error is classified as the default listing classifies
/// it. Nodes of another repository (compared ASCII case-insensitively) and empty nodes (pull
/// requests) are dropped (FR-015); `total_open` is GitHub's `issueCount`.
pub fn parse_search_page(stdout: &[u8], repo: &GithubRepo) -> Result<IssuePage, FilterFailure> {
    let json: serde_json::Value = serde_json::from_slice(stdout)
        .map_err(|e| IssueLoadError::Other(format!("GitHub's answer could not be read: {e}")))?;
    let search = &json["data"]["search"];
    if let Some(errors) = json["errors"]
        .as_array()
        .filter(|errors| !errors.is_empty())
    {
        if let Some(error) = errors.iter().find(|error| {
            matches!(
                error["type"].as_str(),
                Some("RATE_LIMITED" | "FORBIDDEN" | "INSUFFICIENT_SCOPES")
            )
        }) {
            return Err(graphql_error_of(error).into());
        }
        let refuses = |error: &&serde_json::Value| match &error["path"] {
            serde_json::Value::Null => search.is_null(),
            path => *path == serde_json::json!(["search"]),
        };
        let refusals: Vec<&serde_json::Value> = errors.iter().filter(refuses).collect();
        if refusals.is_empty() {
            return Err(graphql_error_of(&errors[0]).into());
        }
        return Err(FilterFailure::Rejected(
            refusals
                .into_iter()
                .filter_map(|error| error["message"].as_str())
                .map(clean)
                .filter(|message| !message.is_empty())
                .map(cut_message)
                .take(MAX_MESSAGES)
                .collect(),
        ));
    }
    let unexpected = || IssueLoadError::Other("GitHub's answer had no search results".into());
    let total_open = search["issueCount"].as_u64().ok_or_else(unexpected)?;
    let nodes = search["nodes"].as_array().ok_or_else(unexpected)?;
    let repo_name = repo.to_string();
    let in_repo = |node: &&serde_json::Value| {
        node["repository"]["nameWithOwner"]
            .as_str()
            .is_some_and(|name| name.eq_ignore_ascii_case(&repo_name))
    };
    let issues = nodes
        .iter()
        .filter(in_repo)
        .map(|node| {
            issue_from_node(node)
                .map(|issue| issue.closed(node["state"] == "CLOSED"))
                .ok_or_else(unexpected)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(IssuePage {
        issues,
        total_open,
        next_cursor: next_cursor_of(search),
    })
}

/// The answer of one filtered `gh` run, read with `parse` through [`answer_of`]: a
/// [`FilterFailure::Rejected`] stands whatever the exit status (`gh` exits 1 on any GraphQL
/// error), and a load failure is classified as every other request's (contracts/source-and-storage.md
/// §1).
pub(crate) fn filtered_answer_of<T>(
    outcome: &crate::process::RunOutcome,
    parse: impl Fn(&[u8]) -> Result<T, FilterFailure>,
) -> Result<T, FilterFailure> {
    let answer = answer_of(outcome, |stdout| match parse(stdout) {
        Ok(read) => Ok(Ok(read)),
        Err(FilterFailure::Rejected(messages)) => Ok(Err(FilterFailure::Rejected(messages))),
        Err(FilterFailure::Load(error)) => Err(error),
    });
    answer.unwrap_or_else(|error| Err(error.into()))
}

/// The issues `search` selects, page by page up to the load cap, through the paging rule every
/// listing shares (search-rules §6): the first failure aborts; `complete` when the last page left
/// no cursor and nothing was cut; `total_open` is GitHub's count of matches.
pub fn load_filtered(
    source: &dyn IssueSource,
    repo: &GithubRepo,
    search: &ScopedSearch,
) -> Result<IssueListing, FilterFailure> {
    let paged = load_pages(|cursor| source.search_page(repo, search, cursor))?;
    Ok(IssueListing {
        complete: paged.exhausted,
        total_open: paged.total_open,
        issues: paged.issues,
    })
}

// Data-model §6: choices

/// Parse `gh api graphql` stdout for a page of names: the connection at `path` under `data`, each
/// node's `field`. Errors are classified as the default listing classifies them.
pub fn parse_names_page(
    stdout: &[u8],
    path: &[&str],
    field: &str,
) -> Result<NamePage, IssueLoadError> {
    let json: serde_json::Value = serde_json::from_slice(stdout)
        .map_err(|e| IssueLoadError::Other(format!("GitHub's answer could not be read: {e}")))?;
    if let Some(error) = graphql_error(&json) {
        return Err(error);
    }
    let connection = path.iter().fold(&json["data"], |value, key| &value[*key]);
    let nodes = connection["nodes"]
        .as_array()
        .ok_or_else(|| IssueLoadError::Other("GitHub's answer had no list".into()))?;
    Ok(NamePage {
        names: nodes
            .iter()
            .filter_map(|node| node[field].as_str().map(str::to_string))
            .collect(),
        next_cursor: next_cursor_of(connection),
    })
}

/// Parse `gh api graphql` stdout for [`crate::github::LABELS_QUERY`].
pub fn parse_labels_page(stdout: &[u8]) -> Result<NamePage, IssueLoadError> {
    parse_names_page(stdout, &["repository", "labels"], "name")
}

/// The labels a label field offers (data-model §6).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LabelChoices {
    /// Sorted case-insensitively, each name once.
    pub labels: Vec<LabelName>,
    /// Names GitHub listed that cannot be quoted, so are not offered.
    pub not_offered: usize,
    /// [`LABEL_PAGE_CAP`] pages were read and GitHub had more.
    pub partial: bool,
}

/// The repository's labels, up to [`LABEL_PAGE_CAP`] pages (research R10). The first failure
/// aborts. A page that adds nothing or hands back the cursor it was asked with ends the load.
pub fn load_label_choices(
    source: &dyn IssueSource,
    repo: &GithubRepo,
) -> Result<LabelChoices, IssueLoadError> {
    let mut names = Vec::new();
    let mut cursor: Option<String> = None;
    let mut pages = 0;
    let partial = loop {
        let page = source.labels_page(repo, cursor.as_deref())?;
        pages += 1;
        let stalled = page.names.is_empty() || page.next_cursor == cursor;
        names.extend(page.names);
        match page.next_cursor {
            None => break false,
            Some(_) if stalled || pages >= LABEL_PAGE_CAP => break true,
            next => cursor = next,
        }
    };
    let mut labels: Vec<LabelName> = names.iter().filter_map(|n| LabelName::parse(n)).collect();
    let not_offered = names.len() - labels.len();
    labels.sort_by(|a, b| {
        a.as_str()
            .to_lowercase()
            .cmp(&b.as_str().to_lowercase())
            .then_with(|| a.as_str().cmp(b.as_str()))
    });
    labels.dedup();
    Ok(LabelChoices {
        labels,
        not_offered,
        partial,
    })
}

// Data-model §7: the remembered selector

/// What a project remembers of its GitHub selector (FR-021, data-model §7), in its owner-only file
/// `store::github_selector_path`. Never a draft: the query is a [`UserQuery::as_str`], set only by
/// a run.
#[derive(Clone, PartialEq, Eq, Default, Serialize)]
pub struct RememberedSelector {
    /// The mode last used.
    pub mode: QueryMode,
    /// The basic filters last chosen.
    pub basic: BasicFilters,
    /// The last run query, cleaned; empty only when none ran (an empty run is not a run).
    pub query: String,
    /// Whether any query ever ran on the project (FR-013).
    pub ran: bool,
}

/// One field of a [`RememberedSelector`] to change: the shell reads the file, applies it and
/// writes it back, so another window's write to another field stands (research R11).
#[derive(Clone, PartialEq, Eq)]
pub enum SelectorChange {
    /// The switch moved to this mode.
    Mode(QueryMode),
    /// The basic filters changed to these.
    Basic(BasicFilters),
    /// This query ran.
    Query(String),
}

impl RememberedSelector {
    /// Read the file's bytes leniently: JSON that is not an object is the default; an unknown mode
    /// is basic; invalid basic values are dropped (data-model §2); a query the checks refuse is the
    /// empty query that never ran.
    pub fn from_json(bytes: &[u8]) -> RememberedSelector {
        let Ok(serde_json::Value::Object(read)) = serde_json::from_slice(bytes) else {
            return RememberedSelector::default();
        };
        let field = |key: &str| read.get(key).unwrap_or(&serde_json::Value::Null);
        let query = UserQuery::parse(field("query").as_str().unwrap_or_default());
        let (query, ran) = match query {
            Ok(query) => (
                query.as_str().to_string(),
                field("ran").as_bool().unwrap_or_default(),
            ),
            Err(_) => (String::new(), false),
        };
        RememberedSelector {
            mode: QueryMode::deserialize(field("mode")).unwrap_or_default(),
            basic: BasicFilters::from_value(field("basic")),
            query,
            ran,
        }
    }

    /// The file's bytes: `{"mode":…,"basic":{…},"query":"…","ran":…}`.
    pub fn to_json(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("plain fields always serialize")
    }

    /// The same with `change` applied; a query the checks refuse changes nothing.
    pub fn with_change(self, change: &SelectorChange) -> RememberedSelector {
        match change {
            SelectorChange::Mode(mode) => RememberedSelector {
                mode: *mode,
                ..self
            },
            SelectorChange::Basic(basic) => RememberedSelector {
                basic: basic.clone(),
                ..self
            },
            SelectorChange::Query(text) => match UserQuery::parse(text) {
                Ok(query) if query.as_str().is_empty() => self,
                Ok(query) => RememberedSelector {
                    query: query.as_str().to_string(),
                    ran: true,
                    ..self
                },
                Err(_) => self,
            },
        }
    }
}

impl fmt::Debug for RememberedSelector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RememberedSelector")
            .field("mode", &self.mode)
            .field("basic", &self.basic)
            .field("query", &REDACTED)
            .field("ran", &self.ran)
            .finish()
    }
}

impl fmt::Debug for SelectorChange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SelectorChange::Mode(mode) => f.debug_tuple("Mode").field(mode).finish(),
            SelectorChange::Basic(basic) => f.debug_tuple("Basic").field(basic).finish(),
            SelectorChange::Query(_) => f.debug_tuple("Query").field(&REDACTED).finish(),
        }
    }
}
