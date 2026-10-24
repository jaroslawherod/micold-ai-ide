//! Advanced JQL in the Jira issue selector (feature 1186): the render-free rules for a query the
//! user types, the only JQL advanced mode sends, and the load that lists its issues.
//!
//! A user query is cleaned, checked and scoped here before anything is sent
//! (`specs/1186-jira-advanced-jql/contracts/jql-rules.md`): an unclosed quoted string, parentheses
//! that do not balance outside quoted strings and a text over 2,000 characters are refused, so a
//! query can never leave the `project = "<KEY>" AND (…)` scope. Pure functions, no I/O.

use serde::{Deserialize, Serialize};

use crate::github::{load_pages, Issue, IssueListing, IssueLoadError, IssuePage};
use crate::jira::{ApiToken, JiraConnection, JiraSource, ProjectKey};

/// Characters a user query holds at most, counted after cleaning (FR-007).
pub const MAX_QUERY_CHARS: usize = 2_000;

/// The sort every scoped query without one of the user's ends with (FR-005).
const DEFAULT_SORT: &str = "ORDER BY updated DESC";

/// Shown when Jira refused the query and gave no message of its own (FR-010).
pub const NOT_VALID_JQL: &str = "Jira did not accept the query: it is not valid JQL.";

/// Shown when the query ran and matched no issue (FR-012).
pub const EMPTY: &str = "No issue matches this query.";

/// Shown in advanced mode before a query has run (FR-004a).
pub const PROMPT: &str = "Type a JQL query and press Enter to list issues.";

/// Shown under a list that holds the load cap while more issues match (FR-006).
pub const CAPPED: &str = "Showing the first 1,000 issues this query returns.";

/// Why the app did not send a query (FR-007, data-model §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryRefusal {
    /// The query ends inside a quoted string.
    UnclosedQuote,
    /// The parentheses outside quoted strings do not balance.
    UnbalancedParentheses,
    /// The cleaned query has more than [`MAX_QUERY_CHARS`] characters.
    TooLong,
}

impl QueryRefusal {
    /// The selector's text for this refusal; it never contains the query (FR-016).
    pub fn message(&self) -> &'static str {
        match self {
            Self::UnclosedQuote => {
                "The query has a quote that is not closed. Close it and run the query again."
            }
            Self::UnbalancedParentheses => {
                "The query's parentheses do not match. Fix them and run the query again."
            }
            Self::TooLong => {
                "The query is longer than 2,000 characters. Shorten it and run the query again."
            }
        }
    }
}

/// A cleaned user query that passed the checks (data-model §1). Constructed only by
/// [`UserQuery::parse`], so a query that could leave the project scope cannot be held.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserQuery {
    text: String,
    sort_at: Option<usize>,
}

impl UserQuery {
    /// Clean `raw`, refuse it when too long, then scan it once for quotes, parentheses and the
    /// sort clause (contracts/jql-rules.md §1, §2).
    pub fn parse(raw: &str) -> Result<UserQuery, QueryRefusal> {
        let text = clean(raw);
        if text.chars().count() > MAX_QUERY_CHARS {
            return Err(QueryRefusal::TooLong);
        }
        let sort_at = scan(&text)?;
        Ok(UserQuery { text, sort_at })
    }

    /// The query up to its sort clause, trimmed; all of it without one.
    pub fn condition(&self) -> &str {
        self.text[..self.sort_at.unwrap_or(self.text.len())].trim()
    }

    /// The sort clause as typed, from its `ORDER` on; `None` without one.
    pub fn sort_clause(&self) -> Option<&str> {
        self.sort_at.map(|at| self.text[at..].trim())
    }

    /// The cleaned text: what runs, what the input shows after a run and what is remembered.
    pub fn as_str(&self) -> &str {
        &self.text
    }
}

/// `\n`, `\r` and `\t` become a space, every other control character goes, and the ends are
/// trimmed (research R3).
fn clean(raw: &str) -> String {
    raw.chars()
        .filter_map(|c| match c {
            '\n' | '\r' | '\t' => Some(' '),
            c if c.is_control() => None,
            c => Some(c),
        })
        .collect::<String>()
        .trim()
        .to_string()
}

/// The one left-to-right pass of research R2: the byte offset of the first sort clause outside
/// quoted strings and parentheses, or why the query is refused. An unclosed quote outranks
/// unbalanced parentheses. An `ORDER BY` inside parentheses stays in the condition, so the split
/// never cuts a group and both halves balance on their own.
fn scan(text: &str) -> Result<Option<usize>, QueryRefusal> {
    let mut quote: Option<char> = None;
    let mut escaped = false;
    let mut depth: i32 = 0;
    let mut went_below = false;
    let mut sort_at = None;
    for (at, c) in text.char_indices() {
        if let Some(open) = quote {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == open {
                quote = None;
            }
            continue;
        }
        match c {
            '"' | '\'' => quote = Some(c),
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                went_below |= depth < 0;
            }
            _ if sort_at.is_none() && depth == 0 && sort_clause_at(text, at) => sort_at = Some(at),
            _ => {}
        }
    }
    if quote.is_some() {
        Err(QueryRefusal::UnclosedQuote)
    } else if went_below || depth != 0 {
        Err(QueryRefusal::UnbalancedParentheses)
    } else {
        Ok(sort_at)
    }
}

fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Whether `ORDER`, whitespace and `BY` start at byte `at`, as whole words (any case).
fn sort_clause_at(text: &str, at: usize) -> bool {
    if text[..at].chars().next_back().is_some_and(is_word) {
        return false;
    }
    let Some(rest) = strip_word(&text[at..], "ORDER") else {
        return false;
    };
    let after_space = rest.trim_start();
    if after_space.len() == rest.len() {
        return false;
    }
    let Some(rest) = strip_word(after_space, "BY") else {
        return false;
    };
    !rest.chars().next().is_some_and(is_word)
}

/// `text` after its leading `word` (ASCII case-insensitive).
fn strip_word<'a>(text: &'a str, word: &str) -> Option<&'a str> {
    let head = text.get(..word.len())?;
    head.eq_ignore_ascii_case(word).then(|| &text[word.len()..])
}

/// The only JQL advanced mode sends (FR-005, data-model §3): the user's condition inside the
/// connected project's scope, then the user's sort clause or [`DEFAULT_SORT`]. Constructed only by
/// [`ScopedQuery::new`], so a refused query cannot be sent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopedQuery {
    jql: String,
}

impl ScopedQuery {
    /// Scope `query` to `key` (contracts/jql-rules.md §3). The key is validated, so it cannot
    /// carry a quote; no `statusCategory` clause is ever added (US1 AS5).
    pub fn new(key: &ProjectKey, query: &UserQuery) -> ScopedQuery {
        let sort = query.sort_clause().unwrap_or(DEFAULT_SORT);
        let jql = match query.condition() {
            "" => format!("project = \"{key}\" {sort}"),
            condition => format!("project = \"{key}\" AND ({condition}) {sort}"),
        };
        ScopedQuery { jql }
    }

    /// The JQL as sent.
    pub fn as_str(&self) -> &str {
        &self.jql
    }
}

/// Why a sent query listed nothing (data-model §4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryFailure {
    /// Jira answered 400: its messages, possibly none.
    Rejected(Vec<String>),
    /// Any other failure, classified as basic mode's (533 FR-018).
    Load(IssueLoadError),
}

/// Which way the selector lists Jira issues (FR-001).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QueryMode {
    /// Search text over the open issues.
    #[default]
    Basic,
    /// A JQL query.
    Advanced,
}

/// List the issues `scoped` selects, page by page up to the load cap, through the paging rule both
/// providers share (contracts/jql-rules.md §6). Only issues keyed `<project key>-<digits>` are
/// kept, before the cap counts them (FR-008). Jira has no total, so `total_open` is the number
/// held, and the listing is `complete` only when the last page left no cursor. The first error
/// aborts the load.
pub fn load(
    source: &dyn JiraSource,
    conn: &JiraConnection,
    token: &ApiToken,
    scoped: &ScopedQuery,
) -> Result<IssueListing, QueryFailure> {
    let paged = load_pages(|cursor| {
        source
            .query_page(conn, token, scoped, cursor)
            .map(|page| keep_project(&conn.project_key, page))
    })?;
    Ok(IssueListing {
        total_open: paged.issues.len() as u64,
        complete: paged.exhausted,
        issues: paged.issues,
    })
}

/// `page` without the issues whose key is not `<key>-<digits>`.
fn keep_project(key: &ProjectKey, mut page: IssuePage) -> IssuePage {
    page.issues.retain(|issue| is_of_project(key, issue));
    page
}

fn is_of_project(key: &ProjectKey, issue: &Issue) -> bool {
    issue
        .key()
        .and_then(|k| k.strip_prefix(key.as_str()))
        .and_then(|rest| rest.strip_prefix('-'))
        .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}
