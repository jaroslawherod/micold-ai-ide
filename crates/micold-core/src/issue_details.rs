//! One GitHub issue's relationships (feature 837): what blocks it and what it blocks, its parent
//! and sub-issues, labels, assignees, milestone, project fields and linked pull requests, read with
//! one `gh api graphql` query and held as a model whose invariants hold by type. Three contracts
//! govern it: `specs/837-issue-relationship-metadata/contracts/details-read.md` (the request, the
//! parse and the timeline paging), `contracts/tool-result.md` (the `create_worktree` result's
//! `issue` object) and `contracts/form-details.md` (the New worktree form's lines).

use std::fmt;
use std::time::{Duration, Instant};

use serde_json::{json, Map, Value};

use crate::github::{answer_of, graphql_error, visible_words, GithubRepo, IssueLoadError};
use crate::process::RunOutcome;

/// How long a details read may take (FR-011): the bound 550 sets for the issue lookup. The daemon's
/// `details_bound` defaults to it, and so does the form.
pub const DETAILS_BOUND: Duration = Duration::from_secs(10);

/// The most items a list shows (FR-008); the query asks for no more.
pub const LIST_CAP: usize = 20;

/// The most pages of the timeline one read runs, the first query's included (research R4).
const MAX_PAGES: usize = 10;

/// Text from GitHub after neutralising (FR-013, data-model §1): control, zero-width and
/// direction-override characters are dropped, a joiner is kept only between two visible
/// characters, and every run of whitespace becomes one space, by the rule the 038 description
/// follows ([`crate::github::description_from`]). An all-invisible input gives an empty text, which
/// callers treat as absent.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlainText(String);

impl PlainText {
    /// `raw` neutralised. The only constructor, so every `PlainText` holds neutralised text.
    pub fn new(raw: &str) -> Self {
        let mut text = String::new();
        for word in visible_words(raw) {
            if !text.is_empty() {
                text.push(' ');
            }
            text.extend(word);
        }
        Self(text)
    }

    /// The neutralised text.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether nothing visible was left.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The neutralised text of `raw`, or `None` when nothing visible is left.
    fn non_empty(raw: &str) -> Option<Self> {
        Some(Self::new(raw)).filter(|text| !text.is_empty())
    }
}

impl fmt::Display for PlainText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Where a related issue or pull request lives (data-model §2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reference {
    /// In the picked issue's repository.
    Here(u64),
    /// In another repository.
    Other {
        owner: String,
        name: String,
        number: u64,
    },
}

impl Reference {
    /// The reference of item `number` in `name_with_owner` (`owner/name`), seen from `repo`. GitHub
    /// compares repository names without ASCII case, and so does this.
    pub fn of(name_with_owner: &str, number: u64, repo: &GithubRepo) -> Self {
        let (owner, name) = name_with_owner
            .split_once('/')
            .unwrap_or((name_with_owner, ""));
        if owner.eq_ignore_ascii_case(repo.owner()) && name.eq_ignore_ascii_case(repo.name()) {
            Self::Here(number)
        } else {
            Self::Other {
                owner: owner.to_string(),
                name: name.to_string(),
                number,
            }
        }
    }
}

impl fmt::Display for Reference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Here(number) => write!(f, "#{number}"),
            Self::Other {
                owner,
                name,
                number,
            } => write!(f, "{owner}/{name}#{number}"),
        }
    }
}

/// A related issue (data-model §3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelatedIssue {
    pub reference: Reference,
    pub title: PlainText,
    pub open: bool,
}

/// A pull request's state as a list shows it (data-model §3). Separate from
/// `pull_request::PrState`, which carries check status the details do not read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrListState {
    Open,
    Draft,
    Merged,
    Closed,
}

impl PrListState {
    /// The state from GitHub's `state`, `isDraft` and `merged`: merged first, then closed, then
    /// draft; anything else is open.
    pub fn from_fields(state: &str, is_draft: bool, merged: bool) -> Self {
        if merged {
            Self::Merged
        } else if state == "CLOSED" {
            Self::Closed
        } else if is_draft {
            Self::Draft
        } else {
            Self::Open
        }
    }

    /// The word the tool result uses.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Draft => "draft",
            Self::Merged => "merged",
            Self::Closed => "closed",
        }
    }
}

/// A related pull request (data-model §3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelatedPr {
    pub reference: Reference,
    pub title: PlainText,
    pub state: PrListState,
}

/// A capped list with the count of all its items (data-model §4, FR-008).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelatedList<T> {
    shown: Vec<T>,
    total: u32,
    hidden: u32,
    total_is_lower_bound: bool,
}

impl<T> RelatedList<T> {
    /// The first [`LIST_CAP`] of `items` in GitHub's order, with `total` items in all of which
    /// `hidden` cannot be seen. The total never counts fewer than the shown and hidden items.
    pub fn new(mut items: Vec<T>, total: u32, hidden: u32) -> Self {
        items.truncate(LIST_CAP);
        let at_least = u32::try_from(items.len())
            .unwrap_or(u32::MAX)
            .saturating_add(hidden);
        Self {
            shown: items,
            total: total.max(at_least),
            hidden,
            total_is_lower_bound: false,
        }
    }

    /// The same list, its total marked as a lower bound (the timeline was not read to its end).
    pub fn lower_bound(self) -> Self {
        Self {
            total_is_lower_bound: true,
            ..self
        }
    }

    /// The items shown, at most [`LIST_CAP`].
    pub fn shown(&self) -> &[T] {
        &self.shown
    }

    /// How many items there are in all, the hidden ones included.
    pub fn total(&self) -> u32 {
        self.total
    }

    /// How many items the sign-in cannot see.
    pub fn hidden(&self) -> u32 {
        self.hidden
    }

    /// How many items are not shown.
    pub fn more(&self) -> u32 {
        self.total
            .saturating_sub(u32::try_from(self.shown.len()).unwrap_or(u32::MAX))
    }

    /// Whether the list has no item at all.
    pub fn is_empty(&self) -> bool {
        self.total == 0
    }

    /// Whether the total may count fewer than there are (the linked pull requests only).
    pub fn total_is_lower_bound(&self) -> bool {
        self.total_is_lower_bound
    }

    /// The list as the tool result writes it: `total`, `hidden` when above 0, `more` and `items`.
    fn to_json(&self, item: impl Fn(&T) -> Value) -> Map<String, Value> {
        let mut object = Map::new();
        object.insert("total".into(), self.total.into());
        if self.hidden > 0 {
            object.insert("hidden".into(), self.hidden.into());
        }
        object.insert("more".into(), self.more().into());
        object.insert(
            "items".into(),
            Value::Array(self.shown.iter().map(item).collect()),
        );
        if self.total_is_lower_bound {
            object.insert("total_is_lower_bound".into(), true.into());
        }
        object
    }
}

/// One kind of detail: read, or not readable with a plain reason (data-model §5, FR-007).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind<T> {
    Read(T),
    NotReadable(String),
}

impl<T> Kind<T> {
    /// The reason it could not be read, if it could not.
    pub fn reason(&self) -> Option<&str> {
        match self {
            Self::Read(_) => None,
            Self::NotReadable(reason) => Some(reason),
        }
    }
}

impl<T: Contents> Kind<T> {
    /// Whether the kind is output: read with something in it, or not readable (FR-004, FR-007).
    pub fn is_present(&self) -> bool {
        match self {
            Self::Read(value) => value.has_any(),
            Self::NotReadable(_) => true,
        }
    }
}

/// What a read kind holds, for [`Kind::is_present`].
pub trait Contents {
    /// Whether there is anything in it.
    fn has_any(&self) -> bool;
}

impl<T> Contents for RelatedList<T> {
    fn has_any(&self) -> bool {
        !self.is_empty() || self.total_is_lower_bound
    }
}

impl<T> Contents for Option<T> {
    fn has_any(&self) -> bool {
        self.is_some()
    }
}

impl Contents for Blockers {
    fn has_any(&self) -> bool {
        self.list.has_any()
    }
}

impl Contents for SubIssues {
    fn has_any(&self) -> bool {
        self.list.has_any()
    }
}

/// The issues blocking it, and how many of them are open (hidden ones included).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Blockers {
    pub list: RelatedList<RelatedIssue>,
    pub open: u32,
}

/// Its sub-issues, and how many of them are closed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubIssues {
    pub list: RelatedList<RelatedIssue>,
    pub closed: u32,
}

/// A project the issue is in, with its non-empty field values for the issue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectMembership {
    pub name: PlainText,
    pub fields: Vec<(PlainText, PlainText)>,
}

/// One issue's relationships (data-model §5).
#[derive(Clone, PartialEq, Eq)]
pub struct IssueDetails {
    pub number: u64,
    pub blocked_by: Kind<Blockers>,
    pub blocking: Kind<RelatedList<RelatedIssue>>,
    pub parent: Kind<Option<RelatedIssue>>,
    pub sub_issues: Kind<SubIssues>,
    pub labels: Kind<RelatedList<PlainText>>,
    pub assignees: Kind<RelatedList<PlainText>>,
    pub milestone: Kind<Option<PlainText>>,
    pub projects: Kind<RelatedList<ProjectMembership>>,
    pub closing_prs: Kind<RelatedList<RelatedPr>>,
    pub linked_prs: Kind<RelatedList<RelatedPr>>,
}

impl IssueDetails {
    /// Whether an open issue blocks it (FR-003, SC-003). A blocker kind that cannot be read is
    /// never blocked.
    pub fn is_blocked(&self) -> bool {
        matches!(&self.blocked_by, Kind::Read(b) if b.open > 0)
    }

    /// The tool result's `issue` object (contracts/tool-result.md): the number, then only the
    /// kinds present, a not-readable kind only under `not_readable`.
    pub fn to_json(&self) -> Value {
        let mut out = Map::new();
        let mut not_readable = Map::new();
        out.insert("number".into(), self.number.into());
        if self.is_blocked() {
            out.insert("blocked".into(), true.into());
        }
        let mut put = |key: &str, reason: Option<&str>, value: Option<Value>| {
            if let Some(reason) = reason {
                not_readable.insert(key.into(), reason.into());
            } else if let Some(value) = value {
                out.insert(key.into(), value);
            }
        };
        put(
            "blocked_by",
            self.blocked_by.reason(),
            read_present(&self.blocked_by).map(|b| {
                let mut object = b.list.to_json(issue_json);
                object.insert("open".into(), b.open.into());
                Value::Object(object)
            }),
        );
        put(
            "blocking",
            self.blocking.reason(),
            read_present(&self.blocking).map(|l| Value::Object(l.to_json(issue_json))),
        );
        put(
            "parent",
            self.parent.reason(),
            read_present(&self.parent)
                .and_then(Option::as_ref)
                .map(issue_json),
        );
        put(
            "sub_issues",
            self.sub_issues.reason(),
            read_present(&self.sub_issues).map(|s| {
                let mut object = s.list.to_json(issue_json);
                object.insert("closed".into(), s.closed.into());
                Value::Object(object)
            }),
        );
        let text_list = |l: &RelatedList<PlainText>| Value::Object(l.to_json(text_json));
        put(
            "labels",
            self.labels.reason(),
            read_present(&self.labels).map(text_list),
        );
        put(
            "assignees",
            self.assignees.reason(),
            read_present(&self.assignees).map(text_list),
        );
        put(
            "milestone",
            self.milestone.reason(),
            read_present(&self.milestone)
                .and_then(Option::as_ref)
                .map(text_json),
        );
        let projects = read_present(&self.projects);
        put(
            "projects",
            self.projects.reason(),
            projects.map(|l| Value::Array(l.shown().iter().map(project_json).collect())),
        );
        if let Some(more) = projects.map(RelatedList::more).filter(|more| *more > 0) {
            put("projects_more", None, Some(more.into()));
        }
        put(
            "closing_pull_requests",
            self.closing_prs.reason(),
            read_present(&self.closing_prs).map(|l| Value::Object(l.to_json(pr_json))),
        );
        put(
            "linked_pull_requests",
            self.linked_prs.reason(),
            read_present(&self.linked_prs).map(|l| Value::Object(l.to_json(pr_json))),
        );
        if !not_readable.is_empty() {
            out.insert("not_readable".into(), Value::Object(not_readable));
        }
        Value::Object(out)
    }
}

/// A kind's value when it was read and has something in it.
fn read_present<T: Contents>(kind: &Kind<T>) -> Option<&T> {
    match kind {
        Kind::Read(value) if value.has_any() => Some(value),
        _ => None,
    }
}

fn text_json(text: &PlainText) -> Value {
    text.as_str().into()
}

fn issue_json(issue: &RelatedIssue) -> Value {
    json!({
        "ref": issue.reference.to_string(),
        "title": issue.title.as_str(),
        "state": if issue.open { "open" } else { "closed" },
    })
}

fn pr_json(pr: &RelatedPr) -> Value {
    json!({
        "ref": pr.reference.to_string(),
        "title": pr.title.as_str(),
        "state": pr.state.as_str(),
    })
}

fn project_json(project: &ProjectMembership) -> Value {
    let fields: Map<String, Value> = project
        .fields
        .iter()
        .map(|(name, value)| (name.to_string(), value.as_str().into()))
        .collect();
    json!({"name": project.name.as_str(), "fields": fields})
}

/// Hand-written, so that a `{:?}` in a log line or a panic message shows the number and how many
/// of each kind, never a title, a label or a login.
impl fmt::Debug for IssueDetails {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn count<T>(kind: &Kind<T>, total: impl Fn(&T) -> usize) -> String {
            match kind {
                Kind::Read(value) => total(value).to_string(),
                Kind::NotReadable(_) => "not readable".to_string(),
            }
        }
        let list = |l: &RelatedList<_>| l.total() as usize;
        f.debug_struct("IssueDetails")
            .field("number", &self.number)
            .field("blocked_by", &count(&self.blocked_by, |b| list(&b.list)))
            .field("blocking", &count(&self.blocking, |l| l.total() as usize))
            .field("parent", &count(&self.parent, |p| usize::from(p.is_some())))
            .field("sub_issues", &count(&self.sub_issues, |s| list(&s.list)))
            .field("labels", &count(&self.labels, |l| l.total() as usize))
            .field("assignees", &count(&self.assignees, |l| l.total() as usize))
            .field(
                "milestone",
                &count(&self.milestone, |m| usize::from(m.is_some())),
            )
            .field("projects", &count(&self.projects, |l| l.total() as usize))
            .field(
                "closing_prs",
                &count(&self.closing_prs, |l| l.total() as usize),
            )
            .field(
                "linked_prs",
                &count(&self.linked_prs, |l| l.total() as usize),
            )
            .finish()
    }
}

/// The selection of a related issue, and of a related pull request.
macro_rules! related {
    () => {
        "number title state repository { nameWithOwner }"
    };
}
macro_rules! related_pr {
    () => {
        concat!(related!(), " isDraft merged")
    };
}
/// The timeline connection both queries read: cross-references from pull requests.
macro_rules! timeline {
    ($after:expr) => {
        concat!(
            "timelineItems(itemTypes: [CROSS_REFERENCED_EVENT], first: 100",
            $after,
            ") { pageInfo { hasNextPage endCursor } nodes { ... on CrossReferencedEvent { ",
            "source { ... on PullRequest { ",
            related_pr!(),
            " } } } } }"
        )
    };
}

/// The GraphQL document for one issue's details (details-read §2, research R3). One line, like
/// [`crate::github::ISSUE_QUERY`].
pub const DETAILS_QUERY: &str = concat!(
    "query($owner: String!, $name: String!, $n: Int!) { ",
    "repository(owner: $owner, name: $name) { issue(number: $n) { number ",
    "issueDependenciesSummary { blockedBy totalBlockedBy blocking totalBlocking } ",
    "blockedBy(first: 20) { totalCount nodes { ",
    related!(),
    " } } ",
    "blocking(first: 20) { totalCount nodes { ",
    related!(),
    " } } ",
    "parent { ",
    related!(),
    " } ",
    "subIssuesSummary { total completed } ",
    "subIssues(first: 20) { totalCount nodes { ",
    related!(),
    " } } ",
    "labels(first: 20) { totalCount nodes { name } } ",
    "assignees(first: 20) { totalCount nodes { login } } ",
    "milestone { title } ",
    "projectItems(first: 20) { totalCount nodes { project { title } ",
    "fieldValues(first: 20) { nodes { __typename ",
    "... on ProjectV2ItemFieldTextValue { text field { ... on ProjectV2FieldCommon { name } } } ",
    "... on ProjectV2ItemFieldNumberValue { number field { ... on ProjectV2FieldCommon { name } } } ",
    "... on ProjectV2ItemFieldDateValue { date field { ... on ProjectV2FieldCommon { name } } } ",
    "... on ProjectV2ItemFieldSingleSelectValue { name field { ... on ProjectV2FieldCommon { name } } } ",
    "... on ProjectV2ItemFieldIterationValue { title field { ... on ProjectV2FieldCommon { name } } } ",
    "} } } } ",
    "closedByPullRequestsReferences(first: 20, includeClosedPrs: true) { totalCount nodes { ",
    related_pr!(),
    " } } ",
    timeline!(""),
    " } } }"
);

/// The GraphQL document for one more timeline page (details-read §4): only the timeline, after
/// `$after`.
pub const TIMELINE_QUERY: &str = concat!(
    "query($owner: String!, $name: String!, $n: Int!, $after: String!) { ",
    "repository(owner: $owner, name: $name) { issue(number: $n) { ",
    timeline!(", after: $after"),
    " } } }"
);

/// The arguments after `gh` for the details of issue `number`: only the owner, the name and the
/// number leave the machine (FR-009), as for the lookup (550).
pub fn details_args(repo: &GithubRepo, number: u32) -> Vec<String> {
    crate::github::numbered_args(DETAILS_QUERY, repo, number)
}

/// The arguments after `gh` for one more timeline page: the details' variables and the cursor.
pub fn timeline_args(repo: &GithubRepo, number: u32, after: &str) -> Vec<String> {
    let mut args = crate::github::numbered_args(TIMELINE_QUERY, repo, number);
    args.push("-f".to_string());
    args.push(format!("after={after}"));
    args
}

/// The kinds, by the key the tool result uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KindKey {
    BlockedBy,
    Blocking,
    Parent,
    SubIssues,
    Labels,
    Assignees,
    Milestone,
    Projects,
    ClosingPrs,
    LinkedPrs,
}

impl KindKey {
    /// The kind an `errors[]` path `["repository", "issue", field, sub, …]` names.
    fn of_path(field: &str, sub: Option<&str>) -> Vec<KindKey> {
        match field {
            "blockedBy" => vec![Self::BlockedBy],
            "blocking" => vec![Self::Blocking],
            "issueDependenciesSummary" => match sub {
                Some(sub) if sub.contains("lockedBy") => vec![Self::BlockedBy],
                Some(sub) if sub.contains("locking") => vec![Self::Blocking],
                _ => vec![Self::BlockedBy, Self::Blocking],
            },
            "parent" => vec![Self::Parent],
            "subIssues" | "subIssuesSummary" => vec![Self::SubIssues],
            "labels" => vec![Self::Labels],
            "assignees" => vec![Self::Assignees],
            "milestone" => vec![Self::Milestone],
            "projectItems" => vec![Self::Projects],
            "closedByPullRequestsReferences" => vec![Self::ClosingPrs],
            "timelineItems" => vec![Self::LinkedPrs],
            _ => Vec::new(),
        }
    }

    /// What the reason calls the kind.
    fn noun(self) -> &'static str {
        match self {
            Self::BlockedBy => "the issues blocking it",
            Self::Blocking => "the issues it blocks",
            Self::Parent => "its parent issue",
            Self::SubIssues => "its sub-issues",
            Self::Labels => "labels",
            Self::Assignees => "assignees",
            Self::Milestone => "its milestone",
            Self::Projects => "projects",
            Self::ClosingPrs => "the pull requests that close it",
            Self::LinkedPrs => "linked pull requests",
        }
    }
}

/// Each kind an `errors[]` entry names, with its plain reason (details-read §3). The first entry
/// for a kind wins.
fn unreadable_kinds(json: &Value) -> Vec<(KindKey, String)> {
    let mut found: Vec<(KindKey, String)> = Vec::new();
    for error in json["errors"].as_array().into_iter().flatten() {
        let path: Vec<&str> = error["path"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        let ["repository", "issue", field, rest @ ..] = path.as_slice() else {
            continue;
        };
        for key in KindKey::of_path(field, rest.first().copied()) {
            if found.iter().any(|(k, _)| *k == key) {
                continue;
            }
            let reason = match error["type"].as_str() {
                Some("INSUFFICIENT_SCOPES") if key == KindKey::Projects => {
                    "your GitHub sign-in can't read projects (it lacks the read:project scope)"
                        .to_string()
                }
                Some("INSUFFICIENT_SCOPES") => {
                    format!("your GitHub sign-in can't read {}", key.noun())
                }
                Some("FORBIDDEN") => "GitHub refused access".to_string(),
                _ => "GitHub could not provide it".to_string(),
            };
            found.push((key, reason));
        }
    }
    found
}

/// What a kind the server left out without an error reads as.
const NOT_PROVIDED: &str = "GitHub could not provide it";

/// Reads a kind: not readable when an error names it, else `read` of the issue node, which
/// answers `None` when the node lacks what the kind needs.
fn kind_of<T>(
    unreadable: &[(KindKey, String)],
    key: KindKey,
    read: impl FnOnce() -> Option<T>,
) -> Kind<T> {
    if let Some((_, reason)) = unreadable.iter().find(|(k, _)| *k == key) {
        return Kind::NotReadable(reason.clone());
    }
    read().map_or_else(|| Kind::NotReadable(NOT_PROVIDED.to_string()), Kind::Read)
}

fn u32_of(value: &Value) -> Option<u32> {
    value.as_u64().map(|n| u32::try_from(n).unwrap_or(u32::MAX))
}

fn issue_of(node: &Value, repo: &GithubRepo) -> Option<RelatedIssue> {
    Some(RelatedIssue {
        reference: Reference::of(
            node["repository"]["nameWithOwner"].as_str()?,
            node["number"].as_u64()?,
            repo,
        ),
        title: PlainText::new(node["title"].as_str().unwrap_or_default()),
        open: node["state"] == "OPEN",
    })
}

fn pr_of(node: &Value, repo: &GithubRepo) -> Option<RelatedPr> {
    Some(RelatedPr {
        reference: Reference::of(
            node["repository"]["nameWithOwner"].as_str()?,
            node["number"].as_u64()?,
            repo,
        ),
        title: PlainText::new(node["title"].as_str().unwrap_or_default()),
        state: PrListState::from_fields(
            node["state"].as_str().unwrap_or_default(),
            node["isDraft"].as_bool().unwrap_or(false),
            node["merged"].as_bool().unwrap_or(false),
        ),
    })
}

/// A connection's items read by `item`, its `totalCount`, and how many of its nodes are `null`
/// (items the sign-in cannot see). `None` when it is not a connection.
fn connection<T>(value: &Value, item: impl Fn(&Value) -> Option<T>) -> Option<(Vec<T>, u32, u32)> {
    let nodes = value["nodes"].as_array()?;
    let total = u32_of(&value["totalCount"]).unwrap_or(0);
    let nulls = u32::try_from(nodes.iter().filter(|n| n.is_null()).count()).unwrap_or(u32::MAX);
    let items = nodes.iter().filter_map(&item).collect();
    Some((items, total, nulls))
}

/// An issue list whose summary counts `summary_total` items: the hidden ones are what the summary
/// counts beyond the connection, plus its `null` nodes (research R5).
fn issue_list(
    value: &Value,
    summary_total: Option<u32>,
    repo: &GithubRepo,
) -> Option<RelatedList<RelatedIssue>> {
    let (items, count, nulls) = connection(value, |n| issue_of(n, repo))?;
    let total = summary_total.unwrap_or(count).max(count);
    let hidden = total.saturating_sub(count).saturating_add(nulls);
    Some(RelatedList::new(items, total, hidden))
}

fn text_list(value: &Value, field: &str) -> Option<RelatedList<PlainText>> {
    let (items, total, _) = connection(value, |n| PlainText::non_empty(n[field].as_str()?))?;
    Some(RelatedList::new(items, total, 0))
}

/// A project field's value as text: the first of `text`, `number`, `date`, `name` or `title`; a
/// whole number without `.0`.
fn field_value(node: &Value) -> Option<String> {
    for key in ["text", "number", "date", "name", "title"] {
        match &node[key] {
            Value::String(text) => return Some(text.clone()),
            Value::Number(number) => {
                return Some(match number.as_f64() {
                    Some(n) if n.fract() == 0.0 && n.abs() < 1e15 => format!("{}", n as i64),
                    _ => number.to_string(),
                })
            }
            _ => {}
        }
    }
    None
}

fn project_of(node: &Value) -> Option<ProjectMembership> {
    let name = PlainText::new(node["project"]["title"].as_str()?);
    let fields = node["fieldValues"]["nodes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|value| {
            let field = PlainText::non_empty(value["field"]["name"].as_str()?)?;
            Some((field, PlainText::non_empty(&field_value(value)?)?))
        })
        .collect();
    Some(ProjectMembership { name, fields })
}

/// One timeline page's pull requests, and where the next page starts while GitHub has more. A
/// source without a number is an issue, and is skipped.
fn timeline_of(value: &Value, repo: &GithubRepo) -> Option<(Vec<RelatedPr>, Option<String>)> {
    let nodes = value["nodes"].as_array()?;
    let prs = nodes
        .iter()
        .filter_map(|n| pr_of(&n["source"], repo))
        .collect();
    let next = (value["pageInfo"]["hasNextPage"] == true)
        .then(|| value["pageInfo"]["endCursor"].as_str().map(str::to_string))
        .flatten();
    Some((prs, next))
}

/// The linked pull requests: repeats and the closing ones dropped (details-read §5).
fn linked_list(
    raw: Vec<RelatedPr>,
    closing: &Kind<RelatedList<RelatedPr>>,
    lower_bound: bool,
) -> RelatedList<RelatedPr> {
    let closing: &[RelatedPr] = match closing {
        Kind::Read(list) => list.shown(),
        Kind::NotReadable(_) => &[],
    };
    let mut kept: Vec<RelatedPr> = Vec::new();
    for pr in raw {
        let seen = |other: &RelatedPr| other.reference == pr.reference;
        if !closing.iter().any(seen) && !kept.iter().any(seen) {
            kept.push(pr);
        }
    }
    let total = u32::try_from(kept.len()).unwrap_or(u32::MAX);
    let list = RelatedList::new(kept, total, 0);
    if lower_bound {
        list.lower_bound()
    } else {
        list
    }
}

/// A parsed details answer, before the timeline is paged.
struct FirstPage {
    details: IssueDetails,
    /// The linked pull requests as the timeline listed them, before normalising.
    linked: Vec<RelatedPr>,
    /// Where the next timeline page starts, while GitHub has more.
    next: Option<String>,
}

impl FirstPage {
    /// The details with `more` timeline pull requests added; `lower_bound` when the timeline was
    /// not read to its end.
    fn finish(mut self, more: Vec<RelatedPr>, lower_bound: bool) -> IssueDetails {
        if let Kind::Read(_) = self.details.linked_prs {
            self.linked.extend(more);
            self.details.linked_prs = Kind::Read(linked_list(
                self.linked,
                &self.details.closing_prs,
                lower_bound,
            ));
        }
        self.details
    }
}

fn read_json(stdout: &[u8]) -> Result<Value, IssueLoadError> {
    serde_json::from_slice(stdout)
        .map_err(|e| IssueLoadError::Other(format!("GitHub's answer could not be read: {e}")))
}

/// The issue node of an answer (details-read §3): no `data` is the GraphQL error's typing, no
/// repository is no access, and no issue is an answer without one.
fn issue_node(json: &Value) -> Result<&Value, IssueLoadError> {
    let no_issue = || IssueLoadError::Other("GitHub's answer had no issue".into());
    let data = &json["data"];
    if !data.is_object() {
        return Err(graphql_error(json).unwrap_or_else(no_issue));
    }
    let repository = &data["repository"];
    if repository.is_null() {
        return Err(graphql_error(json).unwrap_or(IssueLoadError::NoAccess));
    }
    let issue = &repository["issue"];
    if issue.is_null() {
        return Err(no_issue());
    }
    Ok(issue)
}

fn parse_first_page(stdout: &[u8], repo: &GithubRepo) -> Result<FirstPage, IssueLoadError> {
    let json = read_json(stdout)?;
    let issue = issue_node(&json)?;
    let unreadable = unreadable_kinds(&json);
    let summary = &issue["issueDependenciesSummary"];
    let subs_summary = &issue["subIssuesSummary"];
    let number = issue["number"].as_u64().unwrap_or_default();
    let blocked_by = kind_of(&unreadable, KindKey::BlockedBy, || {
        let list = issue_list(
            &issue["blockedBy"],
            u32_of(&summary["totalBlockedBy"]),
            repo,
        )?;
        let open = u32_of(&summary["blockedBy"]).unwrap_or_else(|| {
            u32::try_from(list.shown().iter().filter(|i| i.open).count()).unwrap_or(u32::MAX)
        });
        Some(Blockers { list, open })
    });
    let blocking = kind_of(&unreadable, KindKey::Blocking, || {
        issue_list(&issue["blocking"], u32_of(&summary["totalBlocking"]), repo)
    });
    let parent = kind_of(&unreadable, KindKey::Parent, || {
        Some(issue_of(&issue["parent"], repo))
    });
    let sub_issues = kind_of(&unreadable, KindKey::SubIssues, || {
        let list = issue_list(&issue["subIssues"], u32_of(&subs_summary["total"]), repo)?;
        let closed = u32_of(&subs_summary["completed"]).unwrap_or_else(|| {
            u32::try_from(list.shown().iter().filter(|i| !i.open).count()).unwrap_or(u32::MAX)
        });
        Some(SubIssues { list, closed })
    });
    let labels = kind_of(&unreadable, KindKey::Labels, || {
        text_list(&issue["labels"], "name")
    });
    let assignees = kind_of(&unreadable, KindKey::Assignees, || {
        text_list(&issue["assignees"], "login")
    });
    let milestone = kind_of(&unreadable, KindKey::Milestone, || {
        Some(
            issue["milestone"]["title"]
                .as_str()
                .and_then(PlainText::non_empty),
        )
    });
    let projects = kind_of(&unreadable, KindKey::Projects, || {
        let (items, total, _) = connection(&issue["projectItems"], project_of)?;
        Some(RelatedList::new(items, total, 0))
    });
    let closing_prs = kind_of(&unreadable, KindKey::ClosingPrs, || {
        let (items, total, nulls) =
            connection(&issue["closedByPullRequestsReferences"], |n| pr_of(n, repo))?;
        Some(RelatedList::new(items, total, nulls))
    });
    let mut linked = Vec::new();
    let mut next = None;
    let linked_prs = kind_of(&unreadable, KindKey::LinkedPrs, || {
        let (prs, cursor) = timeline_of(&issue["timelineItems"], repo)?;
        linked = prs;
        next = cursor;
        Some(RelatedList::new(Vec::new(), 0, 0))
    });
    Ok(FirstPage {
        details: IssueDetails {
            number,
            blocked_by,
            blocking,
            parent,
            sub_issues,
            labels,
            assignees,
            milestone,
            projects,
            closing_prs,
            linked_prs,
        },
        linked,
        next,
    })
}

/// Parse `gh api graphql` stdout for [`DETAILS_QUERY`] (details-read §3, §5). A timeline that has
/// more pages leaves the linked pull requests' total a lower bound; [`read_details_with`] pages it.
pub fn parse_details(stdout: &[u8], repo: &GithubRepo) -> Result<IssueDetails, IssueLoadError> {
    let page = parse_first_page(stdout, repo)?;
    let lower_bound = page.next.is_some();
    Ok(page.finish(Vec::new(), lower_bound))
}

/// Parse one [`TIMELINE_QUERY`] page.
fn parse_timeline_page(
    stdout: &[u8],
    repo: &GithubRepo,
) -> Result<(Vec<RelatedPr>, Option<String>), IssueLoadError> {
    let json = read_json(stdout)?;
    let issue = issue_node(&json)?;
    timeline_of(&issue["timelineItems"], repo)
        .ok_or_else(|| IssueLoadError::Other("GitHub's answer had no timeline".into()))
}

/// The tool result's `issue` object when the whole read failed (contracts/tool-result.md).
pub fn details_error_json(number: u64, error: &IssueLoadError, repo: &GithubRepo) -> Value {
    json!({"number": number, "details_error": error.message(repo)})
}

/// The time left before `deadline`, or `None` once it has passed.
fn left_before(deadline: Instant) -> Option<Duration> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|left| !left.is_zero())
}

/// Read issue `number`'s details through `run`, which runs `gh` with the arguments it is given and
/// bounds the run by the time it is given (details-read §1, §4). No run starts after `deadline`:
/// past it before the first, the read fails with [`IssueLoadError::TimedOut`]. After the first
/// query it pages the timeline while GitHub has more, up to ten runs in all; a page that fails or
/// would start late leaves the linked pull requests' total a lower bound and never fails the read.
pub fn read_details_with(
    repo: &GithubRepo,
    number: u32,
    deadline: Instant,
    mut run: impl FnMut(Vec<String>, Duration) -> RunOutcome,
) -> Result<IssueDetails, IssueLoadError> {
    let left = left_before(deadline).ok_or(IssueLoadError::TimedOut)?;
    let first = answer_of(&run(details_args(repo, number), left), |stdout| {
        parse_first_page(stdout, repo)
    })?;
    let mut cursor = first.next.clone();
    let mut more = Vec::new();
    let mut runs = 1;
    while let Some(after) = cursor.as_deref().filter(|_| runs < MAX_PAGES) {
        let Some(left) = left_before(deadline) else {
            break;
        };
        runs += 1;
        let outcome = run(timeline_args(repo, number, after), left);
        match answer_of(&outcome, |stdout| parse_timeline_page(stdout, repo)) {
            Ok((prs, next)) => {
                more.extend(prs);
                cursor = next;
            }
            Err(_) => break,
        }
    }
    Ok(first.finish(more, cursor.is_some()))
}

/// Reading one issue's details (details-read §1).
pub trait IssueDetailsSource: Send + Sync {
    /// Issue `number`'s details, read by `deadline`.
    fn read_details(
        &self,
        repo: &GithubRepo,
        number: u32,
        deadline: Instant,
    ) -> Result<IssueDetails, IssueLoadError>;
}

impl IssueDetailsSource for crate::github::GhCli {
    fn read_details(
        &self,
        repo: &GithubRepo,
        number: u32,
        deadline: Instant,
    ) -> Result<IssueDetails, IssueLoadError> {
        read_details_with(repo, number, deadline, |args, left| {
            self.outcome_within(args, left)
        })
    }
}
