//! Pull requests of a project's branches, read through the user's own `gh` (feature 040,
//! contracts/pull-request-source.md).
//!
//! [`status_args`] is what is sent, [`read_outcome`] turns one `gh` run into a status per branch or a
//! [`ReadingFailure`], and [`PullRequestSource`] is the seam the client reads through. All of it is
//! pure but the `GhCli` impl, which sits in [`crate::github`] beside the runner it shares.
//!
//! Everything here is held in memory only (FR-032): [`PullRequestStatus`] cannot be serialised, and
//! its `Debug` output leaves out the title and the address, so neither reaches a stored file or a
//! log. `tests/pull_request_is_never_stored.rs` holds both.

use std::collections::BTreeMap;
use std::fmt;

use crate::github::{classify, GithubRepo, IssueLoadError};
use crate::pr_list::{BranchPrList, ListedPr};
use crate::process::RunOutcome;
use crate::review::browse::FileReadFailure;
use crate::review::RelPath;

/// What one reading found for one branch (data-model §1).
#[derive(Clone, PartialEq, Eq)]
pub struct PullRequestStatus {
    /// The pull request's number in its repository.
    pub number: u64,
    /// Its title. Never logged, never serialised.
    pub title: String,
    /// Its address on GitHub. Never logged, never serialised.
    pub url: String,
    /// Open, draft, merged or closed; an open or draft one carries its check status.
    pub state: PrState,
    /// GitHub's review decision.
    pub review: ReviewState,
    /// The commit the pull request ends at (`headRefOid`).
    pub head: String,
    /// GitHub reports it cannot merge because of a conflict (`mergeable: CONFLICTING`). `MERGEABLE`
    /// and `UNKNOWN`, which GitHub answers while it computes, are `false`.
    pub conflict: bool,
}

/// Prints the number, the state and the review, and nothing else (FR-032).
impl fmt::Debug for PullRequestStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PullRequestStatus")
            .field("number", &self.number)
            .field("state", &self.state)
            .field("review", &self.review)
            .finish_non_exhaustive()
    }
}

/// The one state a pull request is shown in (FR-002). Only an open or a draft pull request has a
/// check status (FR-003): a merged or closed one with checks cannot be built.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrState {
    /// Open and ready for review.
    Open {
        /// The combined result of its checks.
        checks: CheckStatus,
    },
    /// Open, marked as a draft.
    Draft {
        /// The combined result of its checks.
        checks: CheckStatus,
    },
    /// Merged.
    Merged,
    /// Closed without merging.
    Closed,
}

/// The combined result of a pull request's checks (FR-008).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckStatus {
    /// The pull request has no checks at all.
    None,
    /// No check failed, and at least one is queued or running.
    Pending,
    /// Every check finished without a failure.
    Passing,
    /// At least one check failed, was cancelled, timed out or needs action.
    Failing,
}

/// GitHub's review decision for a pull request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewState {
    /// No decision: the repository asks for no review, or GitHub named one this version does not
    /// know.
    None,
    /// Approved.
    Approved,
    /// Changes were requested.
    ChangesRequested,
    /// A review is required and has not been given.
    ReviewRequired,
}

/// Why a reading produced no statuses (data-model §2). No variant carries text: nothing about a
/// failure is shown (FR-025).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadingFailure {
    /// Pull requests cannot be read at all: no GitHub remote, `gh` missing, not signed in, or the
    /// sign-in cannot see the repository. What a row shows is removed (FR-025).
    Unavailable,
    /// Waiting may help: offline, no answer in time, an answer that cannot be understood. What a
    /// row shows stays (FR-019).
    Passing,
    /// GitHub's request limit was reached; nothing is sent before `until` (FR-024).
    RateLimited {
        /// Unix seconds.
        until: u64,
    },
}

/// How old a reading may be before the indicator is drawn in its lower-emphasis form: two refresh
/// intervals of 5 minutes (FR-019, R13).
pub const STALE_AFTER_SECS: u64 = 600;

/// Whether a reading that started at `read_at` (Unix seconds) is stale at `now`: older than
/// [`STALE_AFTER_SECS`]. A clock that went backwards makes nothing stale.
pub fn is_stale(read_at: u64, now: u64) -> bool {
    now.saturating_sub(read_at) > STALE_AFTER_SECS
}

/// The most branches one request asks about (FR-023).
pub const BRANCHES_PER_REQUEST: usize = 50;

/// One pull request as the `pr` fragment of [`status_query`] names it. No `Debug`: it holds the
/// title and the address (FR-032).
#[derive(Clone, PartialEq, Eq)]
pub struct PrNode {
    /// `number`.
    pub number: u64,
    /// `title`.
    pub title: String,
    /// `url`.
    pub url: String,
    /// `state`, as GitHub names it (`OPEN`, `MERGED`, `CLOSED`).
    pub state: String,
    /// `isDraft`.
    pub is_draft: bool,
    /// `createdAt`: ISO-8601 in UTC with a fixed layout, so two of them compare as text.
    pub created_at: String,
    /// `isCrossRepository`: the head branch lives in another repository (a fork).
    pub is_cross_repository: bool,
    /// `headRefOid`.
    pub head: String,
    /// `reviewDecision`, as GitHub names it; `None` for `null`.
    pub review_decision: Option<String>,
    /// The check counts of the last commit; `None` for a `null` rollup or no commit.
    pub checks: Option<CheckCounts>,
    /// `mergeable` is `CONFLICTING`.
    pub conflict: bool,
}

/// How many checks of a pull request's last commit are in each state, by GitHub's state names.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CheckCounts {
    /// `checkRunCountsByState`: (state name, count).
    pub check_runs: Vec<(String, u64)>,
    /// `statusContextCountsByState`: (state name, count).
    pub status_contexts: Vec<(String, u64)>,
}

/// A pull request whose `state` this version does not know. The whole reading fails as
/// [`ReadingFailure::Passing`] rather than show a state that may be wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unreadable;

// ---- §2: the command and the query ----

/// Shared by the connections of a branch: the 10 newest pull requests with that head branch.
const NEWEST_TEN: &str = "first: 10, orderBy: {field: CREATED_AT, direction: DESC}";

/// The size of a [`NEWEST_TEN`] page: a full one means there may be more (feature 1123, FR-006).
const PAGE_SIZE: usize = 10;

/// The last commit's check counts by state, as a field list: one text for the sidebar's fragment and
/// the Review changes list (feature 1003, FR-003), so both mark a pull request the same way.
macro_rules! commits_rollup {
    () => {
        "commits(last: 1) { nodes { commit { statusCheckRollup { contexts(first: 1) { \
checkRunCount checkRunCountsByState { state count } statusContextCount \
statusContextCountsByState { state count } } } } } }"
    };
}

/// What is read of each pull request. Check counts by state, never the checks themselves: one
/// request stays small however many checks a pull request has.
const FRAGMENT: &str = concat!(
    "fragment pr on PullRequest { number title url state isDraft createdAt \
isCrossRepository headRefOid reviewDecision mergeable ",
    commits_rollup!(),
    " }"
);

/// What is read of each merged pull request for the card's list and counts (feature 1123, R2):
/// scalar fields only, no nested connection, so the `m<i>` page costs one request.
const MERGED_FRAGMENT: &str =
    "fragment merged on PullRequest { number url state isDraft createdAt isCrossRepository }";

/// The GraphQL document that asks about `n` branches (1 to [`BRANCHES_PER_REQUEST`]), on one line.
///
/// Branch `i` is the variable `$b<i>`; `o<i>` holds its open pull requests and `r<i>` its newest of
/// any state; `m<i>` its newest merged ones, so the card's list and counts come from the three
/// pages (feature 1123, FR-008). No branch name is part of the document (FR-031): names travel as variables, so no
/// name can change what is asked.
pub fn status_query(n: usize) -> String {
    use fmt::Write as _;
    let mut variables = String::new();
    let mut aliases = String::new();
    for i in 0..n {
        // Writing to a `String` cannot fail.
        let _ = write!(variables, ", $b{i}: String!");
        let _ = write!(
            aliases,
            "o{i}: pullRequests(headRefName: $b{i}, states: OPEN, {NEWEST_TEN}) \
             {{ nodes {{ ...pr }} }} \
             r{i}: pullRequests(headRefName: $b{i}, {NEWEST_TEN}) {{ nodes {{ ...pr }} }} \
             m{i}: pullRequests(headRefName: $b{i}, states: MERGED, {NEWEST_TEN}) \
             {{ nodes {{ ...merged }} }} "
        );
    }
    format!(
        "query($owner: String!, $name: String!{variables}) {{ \
         repository(owner: $owner, name: $name) {{ {aliases}}} \
         rateLimit {{ remaining resetAt }} }} {FRAGMENT} {MERGED_FRAGMENT}"
    )
}

/// The arguments after `gh` for one request: the query, the repository's owner and name, and the
/// branch names leave the machine, and nothing else (FR-031). No credential is passed (FR-028).
///
/// Every variable is a raw string (`-f`): `-F` would turn a branch named `true` or `123` into a
/// boolean or a number, and read `@file` from disk. `--hostname` keeps a `GH_HOST` in the user's
/// environment from redirecting the request (FR-005).
pub fn status_args(repo: &GithubRepo, branches: &[String]) -> Vec<String> {
    graphql_args(&status_query(branches.len()), repo, branches)
}

/// `gh api graphql` with `query`, the repository's owner and name, and `branches` as `$b0`… (raw
/// strings, `-f`), as [`status_args`] explains.
fn graphql_args(query: &str, repo: &GithubRepo, branches: &[String]) -> Vec<String> {
    let mut args: Vec<String> = ["api", "graphql", "--hostname", "github.com", "--include"]
        .map(str::to_string)
        .to_vec();
    let mut field = |value: String| {
        args.push("-f".into());
        args.push(value);
    };
    field(format!("query={query}"));
    field(format!("owner={}", repo.owner()));
    field(format!("name={}", repo.name()));
    for (i, branch) in branches.iter().enumerate() {
        field(format!("b{i}={branch}"));
    }
    args
}

// ---- §3 and §4: which pull request, and what its checks say ----

/// The pull request a branch is shown with (FR-004, FR-005): pull requests from another
/// repository are dropped; then the newest open one; with none open, the newest of any state.
pub fn select_pull_request(
    open: &[PrNode],
    recent: &[PrNode],
) -> Result<Option<PullRequestStatus>, Unreadable> {
    let newest = |nodes: &[PrNode]| {
        nodes
            .iter()
            .filter(|node| !node.is_cross_repository)
            .max_by(|a, b| a.created_at.cmp(&b.created_at))
            .cloned()
    };
    let Some(node) = newest(open).or_else(|| newest(recent)) else {
        return Ok(None);
    };
    let listed = listed(&node)?;
    let checks = listed.checks;
    let state = match listed.state {
        BranchPrState::Open => PrState::Open { checks },
        BranchPrState::Draft => PrState::Draft { checks },
        BranchPrState::Merged => PrState::Merged,
        BranchPrState::Closed => PrState::Closed,
    };
    Ok(Some(PullRequestStatus {
        number: node.number,
        title: node.title,
        url: node.url,
        state,
        review: listed.review,
        head: node.head,
        conflict: node.conflict,
    }))
}

/// GitHub's `state` and `isDraft` as a state this version knows.
fn branch_pr_state(state: &str, is_draft: bool) -> Result<BranchPrState, Unreadable> {
    match (state, is_draft) {
        ("OPEN", true) => Ok(BranchPrState::Draft),
        ("OPEN", false) => Ok(BranchPrState::Open),
        ("MERGED", _) => Ok(BranchPrState::Merged),
        ("CLOSED", _) => Ok(BranchPrState::Closed),
        _ => Err(Unreadable),
    }
}

/// One node of the `pr` fragment as the card lists it (feature 1123, data-model §1): its state, the
/// reduced checks and the review decision, read the one way [`select_pull_request`] reads them
/// (FR-013).
fn listed(node: &PrNode) -> Result<ListedPr, Unreadable> {
    let review = match node.review_decision.as_deref() {
        Some("APPROVED") => ReviewState::Approved,
        Some("CHANGES_REQUESTED") => ReviewState::ChangesRequested,
        Some("REVIEW_REQUIRED") => ReviewState::ReviewRequired,
        _ => ReviewState::None,
    };
    Ok(ListedPr {
        number: node.number,
        url: node.url.clone(),
        state: branch_pr_state(&node.state, node.is_draft)?,
        checks: reduce_checks(node.checks.as_ref()),
        review,
        conflict: node.conflict,
        created_at: node.created_at.clone(),
    })
}

/// The states of a check run that count as failed.
const FAILED_CHECK_RUNS: [&str; 5] = [
    "FAILURE",
    "CANCELLED",
    "TIMED_OUT",
    "ACTION_REQUIRED",
    "STARTUP_FAILURE",
];
/// The states of a check run that finished without failing. Any other name, known or new, is a
/// check that has not finished.
const FINISHED_CHECK_RUNS: [&str; 4] = ["SUCCESS", "NEUTRAL", "SKIPPED", "COMPLETED"];
/// The states of a commit status that count as failed.
const FAILED_STATUS_CONTEXTS: [&str; 2] = ["FAILURE", "ERROR"];
/// The one state of a commit status that finished without failing.
const FINISHED_STATUS_CONTEXTS: [&str; 1] = ["SUCCESS"];

/// The combined result of a pull request's checks (FR-008): failing when any check failed, else
/// pending when any has not finished, else passing; none when there is no check at all.
///
/// A state name this version does not know counts as not finished, so a new GitHub state is never
/// shown as a pass.
pub fn reduce_checks(rollup: Option<&CheckCounts>) -> CheckStatus {
    let Some(counts) = rollup else {
        return CheckStatus::None;
    };
    let (mut failed, mut unfinished, mut finished) = (0u64, 0u64, 0u64);
    let groups = [
        (
            &counts.check_runs,
            &FAILED_CHECK_RUNS[..],
            &FINISHED_CHECK_RUNS[..],
        ),
        (
            &counts.status_contexts,
            &FAILED_STATUS_CONTEXTS[..],
            &FINISHED_STATUS_CONTEXTS[..],
        ),
    ];
    for (list, failed_names, finished_names) in groups {
        for (state, count) in list {
            let group = if failed_names.contains(&state.as_str()) {
                &mut failed
            } else if finished_names.contains(&state.as_str()) {
                &mut finished
            } else {
                &mut unfinished
            };
            *group = group.saturating_add(*count);
        }
    }
    if failed > 0 {
        CheckStatus::Failing
    } else if unfinished > 0 {
        CheckStatus::Pending
    } else if finished > 0 {
        CheckStatus::Passing
    } else {
        CheckStatus::None
    }
}

// ---- §5: reading the answer ----

/// What `gh api --include` wrote to stdout: the HTTP status, the headers and the body. No `Debug`:
/// the body holds titles and addresses (FR-032).
pub struct Response<'a> {
    /// The HTTP status code.
    pub status: u16,
    headers: Vec<(&'a str, &'a str)>,
    /// Everything after the empty line.
    pub body: &'a [u8],
}

impl<'a> Response<'a> {
    /// The value of the first header called `name`, whatever the case of either.
    pub fn header(&self, name: &str) -> Option<&'a str> {
        self.headers
            .iter()
            .find(|(header, _)| header.eq_ignore_ascii_case(name))
            .map(|(_, value)| *value)
    }
}

/// Separate the status line, the headers and the body of what `gh api --include` printed. Line
/// ends may be `\r\n` or `\n`. `None` when the first line is no HTTP status line, or no empty line
/// ends the headers.
pub fn split_response(stdout: &[u8]) -> Option<Response<'_>> {
    let mut rest = stdout;
    let mut status = None;
    let mut headers = Vec::new();
    loop {
        let end = rest.iter().position(|byte| *byte == b'\n')?;
        let line = &rest[..end];
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        rest = &rest[end + 1..];
        // A header line that is not text cannot be one this module asks for.
        let text = std::str::from_utf8(line).unwrap_or_default();
        match status {
            None => {
                let mut words = text.split_ascii_whitespace();
                if !words.next()?.starts_with("HTTP/") {
                    return None;
                }
                status = Some(words.next()?.parse::<u16>().ok()?);
            }
            Some(status) if line.is_empty() => {
                return Some(Response {
                    status,
                    headers,
                    body: rest,
                });
            }
            Some(_) => {
                if let Some((name, value)) = text.split_once(':') {
                    headers.push((name.trim(), value.trim()));
                }
            }
        }
    }
}

/// The body of a GraphQL answer, as far as this module reads it.
#[derive(serde::Deserialize)]
struct Answer {
    data: Option<AnswerData>,
    #[serde(default)]
    errors: Vec<AnswerError>,
}

#[derive(serde::Deserialize)]
struct AnswerData {
    repository: Option<BTreeMap<String, Connection>>,
}

/// Only the `errors` of an answer. [`reading_failure`] reads them apart from [`Answer`], so that
/// data this version cannot read never hides the rate limit or the refusal reported beside it.
#[derive(serde::Deserialize)]
struct AnswerErrors {
    #[serde(default)]
    errors: Vec<AnswerError>,
}

/// One entry of GraphQL's `errors`: its `type` and where in the query it applies.
#[derive(serde::Deserialize)]
struct AnswerError {
    #[serde(rename = "type", default)]
    kind: Option<String>,
    #[serde(default)]
    path: Vec<serde_json::Value>,
}

impl AnswerError {
    fn is(&self, kind: &str) -> bool {
        self.kind.as_deref() == Some(kind)
    }

    /// GitHub's answer for a repository that does not exist or that the sign-in cannot see.
    fn is_repository_not_found(&self) -> bool {
        self.is("NOT_FOUND") && self.path.len() == 1 && self.path[0] == "repository"
    }

    /// GitHub's answer when the sign-in may read pull requests but not their checks: the field
    /// comes back `null` with a `FORBIDDEN` error at its path. Recorded in
    /// `tests/fixtures/gh/pr_checks_forbidden.txt`.
    fn is_checks_forbidden(&self) -> bool {
        self.is("FORBIDDEN")
            && self
                .path
                .last()
                .is_some_and(|last| last == "statusCheckRollup")
    }
}

#[derive(serde::Deserialize)]
struct Connection {
    nodes: Vec<serde_json::Value>,
}

/// One node of the `merged` fragment (feature 1123, R2).
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawMerged {
    number: u64,
    url: String,
    state: String,
    is_draft: bool,
    created_at: String,
    is_cross_repository: bool,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawNode {
    number: u64,
    title: String,
    url: String,
    state: String,
    is_draft: bool,
    created_at: String,
    is_cross_repository: bool,
    head_ref_oid: String,
    review_decision: Option<String>,
    /// Absent in answers recorded before the field was asked.
    #[serde(default)]
    mergeable: Option<String>,
    commits: RawCommits,
}

#[derive(serde::Deserialize, Default)]
struct RawCommits {
    nodes: Vec<RawCommitNode>,
}

#[derive(serde::Deserialize)]
struct RawCommitNode {
    commit: RawCommit,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawCommit {
    status_check_rollup: Option<RawRollup>,
}

#[derive(serde::Deserialize)]
struct RawRollup {
    contexts: RawContexts,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawContexts {
    check_run_counts_by_state: Option<Vec<RawStateCount>>,
    status_context_counts_by_state: Option<Vec<RawStateCount>>,
}

#[derive(serde::Deserialize)]
struct RawStateCount {
    state: String,
    count: u64,
}

impl RawCommits {
    /// The check counts of the last commit; `None` for a `null` rollup or no commit.
    fn into_counts(self) -> Option<CheckCounts> {
        let counts = |list: Option<Vec<RawStateCount>>| {
            list.unwrap_or_default()
                .into_iter()
                .map(|entry| (entry.state, entry.count))
                .collect()
        };
        self.nodes
            .into_iter()
            .next_back()
            .and_then(|node| node.commit.status_check_rollup)
            .map(|rollup| CheckCounts {
                check_runs: counts(rollup.contexts.check_run_counts_by_state),
                status_contexts: counts(rollup.contexts.status_context_counts_by_state),
            })
    }
}

impl From<RawNode> for PrNode {
    fn from(raw: RawNode) -> Self {
        let checks = raw.commits.into_counts();
        PrNode {
            number: raw.number,
            title: raw.title,
            url: raw.url,
            state: raw.state,
            is_draft: raw.is_draft,
            created_at: raw.created_at,
            is_cross_repository: raw.is_cross_repository,
            head: raw.head_ref_oid,
            review_decision: raw.review_decision,
            checks,
            conflict: raw.mergeable.as_deref() == Some("CONFLICTING"),
        }
    }
}

/// What one reading found: the pull request each branch is shown with, and the list of its pull
/// requests the card shows and counts (feature 1123).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StatusReading {
    /// Branch → the shown pull request; a branch without one has no entry.
    pub statuses: BTreeMap<String, PullRequestStatus>,
    /// Branch → its listed pull requests; **every** asked branch has an entry (empty for none), so
    /// "no pull request" differs from "unknown" (no entry).
    pub lists: BTreeMap<String, BranchPrList>,
}

impl StatusReading {
    /// Add the branches of `other`, as chunks of one reading are joined.
    fn extend(&mut self, other: Self) {
        self.statuses.extend(other.statuses);
        self.lists.extend(other.lists);
    }
}

/// One status and one list per asked branch, from the body of an answer to
/// [`status_query`]`(branches.len())`.
///
/// The key of an entry is `branches[i]`, never a name taken from the answer. An answer that cannot
/// be read whole — an `errors` entry (but a refused `statusCheckRollup`, whose checks read as none),
/// a missing alias (`m` too), a `null` repository, a pull request state
/// this version does not know, JSON that does not parse — is [`ReadingFailure::Passing`], never a
/// partial reading (FR-019). [`reading_failure`] names the real kind from the whole `gh` run.
pub fn parse_status(body: &[u8], branches: &[String]) -> Result<StatusReading, ReadingFailure> {
    const UNREADABLE: ReadingFailure = ReadingFailure::Passing;
    let answer: Answer = serde_json::from_slice(body).map_err(|_| UNREADABLE)?;
    // A refused `statusCheckRollup` leaves the pull request readable: its checks read as none.
    if !answer.errors.iter().all(AnswerError::is_checks_forbidden) {
        return Err(UNREADABLE);
    }
    let mut repository = answer
        .data
        .and_then(|data| data.repository)
        .ok_or(UNREADABLE)?;
    let mut reading = StatusReading::default();
    for (i, branch) in branches.iter().enumerate() {
        let mut alias = |name: String| -> Result<Vec<serde_json::Value>, ReadingFailure> {
            repository
                .remove(&format!("{name}{i}"))
                .map(|connection| connection.nodes)
                .ok_or(UNREADABLE)
        };
        let (open, recent) = (alias("o".into())?, alias("r".into())?);
        let merged = alias("m".into())?;
        let pull_requests =
            |values: Vec<serde_json::Value>| -> Result<Vec<PrNode>, ReadingFailure> {
                values
                    .into_iter()
                    .map(|value| {
                        serde_json::from_value::<RawNode>(value)
                            .map(PrNode::from)
                            .map_err(|_| UNREADABLE)
                    })
                    .collect()
            };
        let (open, recent) = (pull_requests(open)?, pull_requests(recent)?);
        let merged = merged
            .into_iter()
            .map(|value| serde_json::from_value::<RawMerged>(value).map_err(|_| UNREADABLE))
            .collect::<Result<Vec<_>, _>>()?;
        let (open_full, merged_full) = (open.len() >= PAGE_SIZE, merged.len() >= PAGE_SIZE);
        if let Some(status) = select_pull_request(&open, &recent).map_err(|_| UNREADABLE)? {
            reading.statuses.insert(branch.clone(), status);
        }
        // Forks are listed nowhere, the rule `select_pull_request` uses. The `o` and `r` nodes come
        // first, so a number also on the merged page keeps its checks and review (P7).
        let mut listed_prs = Vec::new();
        for node in open.iter().chain(&recent) {
            if !node.is_cross_repository {
                listed_prs.push(listed(node).map_err(|_| UNREADABLE)?);
            }
        }
        for node in merged {
            if !node.is_cross_repository {
                listed_prs.push(ListedPr {
                    number: node.number,
                    url: node.url,
                    state: branch_pr_state(&node.state, node.is_draft).map_err(|_| UNREADABLE)?,
                    checks: CheckStatus::None,
                    review: ReviewState::None,
                    conflict: false,
                    created_at: node.created_at,
                });
            }
        }
        reading.lists.insert(
            branch.clone(),
            BranchPrList::new(listed_prs, open_full, merged_full),
        );
    }
    Ok(reading)
}

/// How long nothing is sent when GitHub names no time (FR-024).
const DEFAULT_PAUSE_SECS: u64 = 60;

/// When the next request may be sent after a rate-limit answer, in Unix seconds (FR-024).
///
/// `Retry-After` first; else, when the primary limit is the one that was hit (`X-RateLimit-Remaining:
/// 0`), its reset time; else a minute. `X-RateLimit-Reset` is on every answer and is the primary
/// window's reset, so it says nothing about a secondary limit. A value that does not parse is
/// skipped as if absent. Never before `now + 1`.
pub fn rate_limit_pause(response: &Response<'_>, now: u64) -> u64 {
    let number = |name: &str| response.header(name).and_then(|v| v.parse::<u64>().ok());
    let fallback = now.saturating_add(DEFAULT_PAUSE_SECS);
    let until = if let Some(seconds) = number("retry-after") {
        now.saturating_add(seconds)
    } else if number("x-ratelimit-remaining") == Some(0) {
        number("x-ratelimit-reset").unwrap_or(fallback)
    } else {
        fallback
    };
    until.max(now.saturating_add(1))
}

/// Why a `gh` run produced no statuses (data-model §2), from its whole outcome: the HTTP status and
/// the GraphQL errors on stdout, and what [`classify`] reads from the exit status and stderr.
///
/// Rate limiting is checked before access, because GitHub reports both as HTTP 403.
pub fn reading_failure(outcome: &RunOutcome, now: u64) -> ReadingFailure {
    let reason = classify(outcome);
    let (stdout, stderr) = match outcome {
        RunOutcome::TimedOut { .. } => return ReadingFailure::Passing,
        RunOutcome::SpawnFailed(_) => {
            return if reason == IssueLoadError::ToolMissing {
                ReadingFailure::Unavailable
            } else {
                ReadingFailure::Passing
            };
        }
        RunOutcome::Exited { stdout, stderr, .. } => (stdout, stderr),
    };
    let response = split_response(stdout);
    let status = response.as_ref().map(|response| response.status);
    let body = response.as_ref().map_or(&[][..], |response| response.body);
    let errors = serde_json::from_slice::<AnswerErrors>(body)
        .map(|answer| answer.errors)
        .unwrap_or_default();
    let says_rate_limit = |text: &str| text.to_ascii_lowercase().contains("rate limit");

    let rate_limited = errors.iter().any(|error| error.is("RATE_LIMITED"))
        || status == Some(429)
        || (status == Some(403)
            && (says_rate_limit(&String::from_utf8_lossy(body)) || says_rate_limit(stderr)))
        || reason == IssueLoadError::RateLimited;
    if rate_limited {
        // With no answer to read headers from, GitHub named no time.
        let until = response.map_or(now.saturating_add(DEFAULT_PAUSE_SECS), |response| {
            rate_limit_pause(&response, now)
        });
        return ReadingFailure::RateLimited { until };
    }
    let no_access = errors.iter().any(AnswerError::is_repository_not_found)
        || status == Some(401)
        || matches!(
            reason,
            IssueLoadError::NotSignedIn | IssueLoadError::NoAccess
        );
    if no_access {
        ReadingFailure::Unavailable
    } else {
        ReadingFailure::Passing
    }
}

/// Whether the body's `errors` are all refusals of `statusCheckRollup` (and there is at least one).
fn only_checks_forbidden(body: &[u8]) -> bool {
    serde_json::from_slice::<AnswerErrors>(body).is_ok_and(|answer| {
        !answer.errors.is_empty() && answer.errors.iter().all(AnswerError::is_checks_forbidden)
    })
}

/// The answer of one `gh` run that asked about `branches`: the statuses when `gh` exited 0 with an
/// HTTP 200 that [`parse_status`] reads, else the kind [`reading_failure`] names.
pub fn read_outcome(
    outcome: &RunOutcome,
    branches: &[String],
    now: u64,
) -> Result<StatusReading, ReadingFailure> {
    if let RunOutcome::Exited { code, stdout, .. } = outcome {
        let read = split_response(stdout)
            .filter(|response| response.status == 200)
            // `gh` exits 1 for any `errors` entry; only a refused check rollup is read through.
            .filter(|response| *code == 0 || only_checks_forbidden(response.body))
            .map(|response| parse_status(response.body, branches));
        if let Some(Ok(reading)) = read {
            return Ok(reading);
        }
    }
    Err(reading_failure(outcome, now))
}

// ---- #775: every pull request of one branch ----

/// One pull request of a branch as the Changes pane lists it (feature 775, data-model §1). Held in
/// memory only; what it prints for a log is the number and the state (FR-032).
#[derive(Clone, PartialEq, Eq)]
pub struct BranchPullRequest {
    /// The pull request's number in its repository.
    pub number: u64,
    /// Its title. Never logged, never serialised.
    pub title: String,
    /// Its address on GitHub. Never logged, never serialised.
    pub url: String,
    /// Open, draft, merged or closed.
    pub state: BranchPrState,
    /// `createdAt`: ISO-8601 in UTC, ordering key (newest first).
    pub created_at: String,
    /// `baseRefName`.
    pub base: String,
    /// `headRefOid`.
    pub head: String,
    /// `headRefName`: the branch it was opened from (feature 1003), named by the header's base line.
    pub head_branch: String,
    /// The combined result of its last commit's checks (feature 1003, FR-003), whatever its state.
    pub checks: CheckStatus,
}

impl fmt::Debug for BranchPullRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BranchPullRequest")
            .field("number", &self.number)
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}

/// The state of a listed pull request. [`PrState`] is not reused: its open and draft carry a check
/// status this list does not read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BranchPrState {
    /// Open and ready for review.
    Open,
    /// Open, marked as a draft.
    Draft,
    /// Merged.
    Merged,
    /// Closed without merging.
    Closed,
}

/// How many branches one list request asks for (feature 1003, research R3): SC-007's 25 branches
/// are one request.
pub const LIST_BRANCHES_PER_REQUEST: usize = 25;

/// The GraphQL document that lists the pull requests of `n` branches, each newest first
/// (feature 1003, contracts/pull-request-list.md §2): alias `p<i>` reads branch `$b<i>`, and the
/// default branch's name comes back so its alias can be dropped. No branch name is in the document.
pub fn list_query(n: usize) -> String {
    use fmt::Write as _;
    const COMMITS_ROLLUP: &str = commits_rollup!();
    let mut variables = String::new();
    let mut aliases = String::new();
    for i in 0..n {
        // Writing to a `String` cannot fail.
        let _ = write!(variables, ", $b{i}: String!");
        let _ = write!(
            aliases,
            "p{i}: pullRequests(headRefName: $b{i}, first: 100, orderBy: {{field: CREATED_AT, direction: DESC}}) \
             {{ nodes {{ number title url state isDraft createdAt isCrossRepository baseRefName \
             headRefName headRefOid {COMMITS_ROLLUP} }} }} "
        );
    }
    format!(
        "query($owner: String!, $name: String!{variables}) {{ \
         repository(owner: $owner, name: $name) {{ defaultBranchRef {{ name }} {aliases}}} \
         rateLimit {{ remaining resetAt }} }}"
    )
}

/// The arguments after `gh` for [`list_query`] over `branches`: raw string variables (`-f`), like
/// [`status_args`].
pub fn list_args(repo: &GithubRepo, branches: &[String]) -> Vec<String> {
    graphql_args(&list_query(branches.len()), repo, branches)
}

#[derive(serde::Deserialize)]
struct ListAnswer {
    data: Option<ListData>,
    #[serde(default)]
    errors: Vec<serde_json::Value>,
}

#[derive(serde::Deserialize)]
struct ListData {
    repository: Option<serde_json::Map<String, serde_json::Value>>,
}

#[derive(serde::Deserialize)]
struct ListConnection {
    nodes: Vec<ListNode>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListNode {
    number: u64,
    title: String,
    url: String,
    state: String,
    is_draft: bool,
    created_at: String,
    is_cross_repository: bool,
    base_ref_name: String,
    head_ref_name: String,
    head_ref_oid: String,
    /// Absent in answers recorded before the counts were asked: no checks.
    #[serde(default)]
    commits: RawCommits,
}

/// Newest first by `createdAt`, then the higher number first (FR-004): `a` before `b` is `Less`.
/// Shared by the Changes pane's list and the card's (feature 1123, FR-003).
pub(crate) fn newest_first(a: (&str, u64), b: (&str, u64)) -> std::cmp::Ordering {
    b.0.cmp(a.0).then(b.1.cmp(&a.1))
}

/// Newest first by `createdAt`, then the higher number first (FR-004).
fn sort_newest_first(list: &mut [BranchPullRequest]) {
    list.sort_by(|a, b| newest_first((&a.created_at, a.number), (&b.created_at, b.number)));
}

/// The pull requests of an answer's body to [`list_query`] over `branches`: forks dropped, the
/// default branch's alias dropped, newest first. An answer that cannot be read whole (an `errors`
/// entry, a `null` repository, an alias missing, a state this version does not know, JSON that
/// does not parse) is [`ReadingFailure::Passing`]: no branch is silently left out (FR-013).
pub fn parse_list(
    body: &[u8],
    branches: &[String],
) -> Result<Vec<BranchPullRequest>, ReadingFailure> {
    const UNREADABLE: ReadingFailure = ReadingFailure::Passing;
    let answer: ListAnswer = serde_json::from_slice(body).map_err(|_| UNREADABLE)?;
    if !answer.errors.is_empty() {
        return Err(UNREADABLE);
    }
    let mut repository = answer
        .data
        .and_then(|data| data.repository)
        .ok_or(UNREADABLE)?;
    let default = repository
        .get("defaultBranchRef")
        .and_then(|r| r.get("name"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let mut list = Vec::new();
    for (i, branch) in branches.iter().enumerate() {
        let alias = repository.remove(&format!("p{i}")).ok_or(UNREADABLE)?;
        let connection: ListConnection = serde_json::from_value(alias).map_err(|_| UNREADABLE)?;
        if default.as_deref() == Some(branch.as_str()) {
            continue; // the default branch is never part of the set (FR-002)
        }
        for node in connection.nodes {
            if node.is_cross_repository {
                continue;
            }
            let state = branch_pr_state(&node.state, node.is_draft).map_err(|_| UNREADABLE)?;
            list.push(BranchPullRequest {
                number: node.number,
                title: node.title,
                url: node.url,
                state,
                created_at: node.created_at,
                base: node.base_ref_name,
                head: node.head_ref_oid,
                head_branch: node.head_ref_name,
                checks: reduce_checks(node.commits.into_counts().as_ref()),
            });
        }
    }
    sort_newest_first(&mut list);
    Ok(list)
}

/// The list of one `gh` run that asked [`list_args`] over `branches`: the pull requests when `gh`
/// exited 0 with an HTTP 200 that [`parse_list`] reads, else the kind [`reading_failure`] names.
pub fn read_list_outcome(
    outcome: &RunOutcome,
    branches: &[String],
    now: u64,
) -> Result<Vec<BranchPullRequest>, ReadingFailure> {
    if let RunOutcome::Exited {
        code: 0, stdout, ..
    } = outcome
    {
        let read = split_response(stdout)
            .filter(|response| response.status == 200)
            .map(|response| parse_list(response.body, branches));
        if let Some(Ok(list)) = read {
            return Ok(list);
        }
    }
    Err(reading_failure(outcome, now))
}

/// List `branches` in chunks of [`LIST_BRANCHES_PER_REQUEST`], one after another (feature 1003,
/// research R3), merged newest first. No branch, no call. The first chunk that fails ends the read
/// with that failure: whole or a failure, so no branch is silently dropped (FR-013).
pub fn list_in_chunks(
    branches: &[String],
    mut list_chunk: impl FnMut(&[String]) -> Result<Vec<BranchPullRequest>, ReadingFailure>,
) -> Result<Vec<BranchPullRequest>, ReadingFailure> {
    let mut list = Vec::new();
    for chunk in branches.chunks(LIST_BRANCHES_PER_REQUEST) {
        list.extend(list_chunk(chunk)?);
    }
    sort_newest_first(&mut list);
    Ok(list)
}

// ---- §1: the source ----

/// Where the pull requests of a project's branches are read from. [`crate::github::GhCli`] is the
/// production one, [`FakePullRequestSource`] the scripted one.
pub trait PullRequestSource {
    /// The pull request of each branch in the project's repository; a branch without one has no
    /// entry. One GitHub request per [`BRANCHES_PER_REQUEST`] branches. `now` is Unix seconds,
    /// used only to turn `Retry-After` into a time.
    fn read(
        &self,
        repo: &GithubRepo,
        branches: &[String],
        now: u64,
    ) -> Result<StatusReading, ReadingFailure>;

    /// Every pull request whose head branch is one of `branches`, newest first, forks and the
    /// default branch dropped (features 775, 1003). One GitHub request per
    /// [`LIST_BRANCHES_PER_REQUEST`] branches, no retry; none for no branch.
    fn list_branches(
        &self,
        repo: &GithubRepo,
        branches: &[String],
        now: u64,
    ) -> Result<Vec<BranchPullRequest>, ReadingFailure>;

    /// The unified diff of pull request `number` as GitHub computes it, base...head, so it also
    /// exists after the head branch is deleted (feature 775, FR-004). One request, no retry.
    fn diff(&self, repo: &GithubRepo, number: u64, now: u64) -> Result<String, DiffReadFailure>;

    /// The bytes of `path` at `commit`, for a head commit that is not in the local repository
    /// (feature 1150, FR-005). One request, no retry, the same `gh` and credentials as
    /// [`Self::diff`].
    fn file_at(
        &self,
        repo: &GithubRepo,
        commit: &str,
        path: &RelPath,
        now: u64,
    ) -> Result<Vec<u8>, FileReadFailure>;
}

/// Why a pull request's diff was not read (feature 775).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffReadFailure {
    /// GitHub refuses to diff a request above 300 files or 20 000 changed lines (HTTP 406).
    TooLarge,
    /// Any other reason.
    Failed(ReadingFailure),
}

/// The arguments after `gh` for a pull request's diff: owner, repository and number only.
pub fn diff_args(repo: &GithubRepo, number: u64) -> Vec<String> {
    [
        "api",
        "--hostname",
        "github.com",
        "-H",
        "Accept: application/vnd.github.v3.diff",
    ]
    .into_iter()
    .map(str::to_string)
    .chain([format!(
        "repos/{}/{}/pulls/{number}",
        repo.owner(),
        repo.name()
    )])
    .collect()
}

/// Why a diff run failed: HTTP 406 is [`DiffReadFailure::TooLarge`], the rest what
/// [`reading_failure`] names.
pub fn diff_failure(outcome: &RunOutcome, now: u64) -> DiffReadFailure {
    if let RunOutcome::Exited { stderr, .. } = outcome {
        if stderr.contains("HTTP 406") {
            return DiffReadFailure::TooLarge;
        }
    }
    DiffReadFailure::Failed(reading_failure(outcome, now))
}

/// `text` with every byte outside the unreserved set percent-encoded, as one URL path segment or
/// query value.
fn percent_encoded(text: &str) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

/// The arguments after `gh` for one file's raw content at a commit (feature 1150, R3): each path
/// segment percent-encoded, the commit as the `ref` query.
pub fn file_args(repo: &GithubRepo, commit: &str, path: &RelPath) -> Vec<String> {
    let path = path
        .as_str()
        .split('/')
        .map(percent_encoded)
        .collect::<Vec<_>>()
        .join("/");
    [
        "api",
        "--hostname",
        "github.com",
        "-H",
        "Accept: application/vnd.github.raw",
    ]
    .into_iter()
    .map(str::to_string)
    .chain([format!(
        "repos/{}/{}/contents/{path}?ref={}",
        repo.owner(),
        repo.name(),
        percent_encoded(commit)
    )])
    .collect()
}

/// The file's bytes when `gh` exited 0; HTTP 404 is [`FileReadFailure::Missing`], any other
/// failure [`FileReadFailure::HeadUnavailable`].
pub fn file_outcome(outcome: &RunOutcome) -> Result<Vec<u8>, FileReadFailure> {
    match outcome {
        RunOutcome::Exited {
            code: 0, stdout, ..
        } => Ok(stdout.clone()),
        RunOutcome::Exited { stderr, .. } if stderr.contains("HTTP 404") => {
            Err(FileReadFailure::Missing)
        }
        _ => Err(FileReadFailure::HeadUnavailable),
    }
}

/// The diff text when `gh` exited 0, else [`diff_failure`].
pub fn diff_outcome(outcome: &RunOutcome, now: u64) -> Result<String, DiffReadFailure> {
    match outcome {
        RunOutcome::Exited {
            code: 0, stdout, ..
        } => Ok(String::from_utf8_lossy(stdout).into_owned()),
        _ => Err(diff_failure(outcome, now)),
    }
}

/// Read `branches` in chunks of [`BRANCHES_PER_REQUEST`], one after another (FR-023).
///
/// No branch, no call: a project without worktrees makes no request. The first chunk that fails
/// ends the reading with that failure, and nothing of the earlier chunks is returned: a reading is
/// whole or it is a failure (FR-019).
pub fn read_in_chunks(
    branches: &[String],
    mut read_chunk: impl FnMut(&[String]) -> Result<StatusReading, ReadingFailure>,
) -> Result<StatusReading, ReadingFailure> {
    let mut reading = StatusReading::default();
    for chunk in branches.chunks(BRANCHES_PER_REQUEST) {
        reading.extend(read_chunk(chunk)?);
    }
    Ok(reading)
}

/// One scripted answer of a [`FakePullRequestSource`].
type Reading = Result<StatusReading, ReadingFailure>;

/// A scripted [`PullRequestSource`] for tests: answers each `read` with the next scripted answer
/// and records every call. Public (not `#[cfg(test)]`) so every crate's tests can use it, like
/// [`crate::github::FakeIssueSource`]. Nothing in a test runs `gh`.
#[derive(Debug, Default)]
pub struct FakePullRequestSource {
    script: std::sync::Mutex<std::collections::VecDeque<Reading>>,
    calls: std::sync::Mutex<Vec<(String, Vec<String>)>>,
    list_script: std::sync::Mutex<std::collections::VecDeque<ListReading>>,
    list_calls: std::sync::Mutex<Vec<(String, Vec<String>)>>,
    diff_script: std::sync::Mutex<std::collections::VecDeque<Result<String, DiffReadFailure>>>,
    diff_calls: std::sync::Mutex<Vec<(String, u64)>>,
    file_script: std::sync::Mutex<std::collections::VecDeque<Result<Vec<u8>, FileReadFailure>>>,
    file_calls: std::sync::Mutex<Vec<(String, String, RelPath)>>,
}

/// One scripted answer of [`FakePullRequestSource::list_branches`].
type ListReading = Result<Vec<BranchPullRequest>, ReadingFailure>;

impl FakePullRequestSource {
    /// A source with nothing scripted.
    pub fn new() -> Self {
        Self::default()
    }

    /// Answer the next unanswered `read` with `statuses` and no lists.
    pub fn with_answer(self, statuses: BTreeMap<String, PullRequestStatus>) -> Self {
        self.with_reading(StatusReading {
            statuses,
            lists: BTreeMap::new(),
        })
    }

    /// Answer the next unanswered `read` with `reading`.
    pub fn with_reading(self, reading: StatusReading) -> Self {
        self.lock_script().push_back(Ok(reading));
        self
    }

    /// Answer the next unanswered `read` with `failure`.
    pub fn with_failure(self, failure: ReadingFailure) -> Self {
        self.lock_script().push_back(Err(failure));
        self
    }

    /// Answer the next unanswered `list_branches` with `list`.
    pub fn with_list_answer(self, list: Vec<BranchPullRequest>) -> Self {
        self.lock_list_script().push_back(Ok(list));
        self
    }

    /// Answer the next unanswered `list_branches` with `failure`.
    pub fn with_list_failure(self, failure: ReadingFailure) -> Self {
        self.lock_list_script().push_back(Err(failure));
        self
    }

    /// Answer the next unanswered `diff` with `text`.
    pub fn with_diff_answer(self, text: String) -> Self {
        self.lock_diff_script().push_back(Ok(text));
        self
    }

    /// Answer the next unanswered `diff` with `failure`.
    pub fn with_diff_failure(self, failure: DiffReadFailure) -> Self {
        self.lock_diff_script().push_back(Err(failure));
        self
    }

    /// Answer the next unanswered `file_at` with `bytes`.
    pub fn with_file_answer(self, bytes: Vec<u8>) -> Self {
        self.lock_file_script().push_back(Ok(bytes));
        self
    }

    /// Answer the next unanswered `file_at` with `failure`.
    pub fn with_file_failure(self, failure: FileReadFailure) -> Self {
        self.lock_file_script().push_back(Err(failure));
        self
    }

    /// Every `file_at` so far, as (`owner/name`, commit, path).
    pub fn file_calls(&self) -> Vec<(String, String, RelPath)> {
        self.file_calls
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    fn lock_file_script(
        &self,
    ) -> std::sync::MutexGuard<'_, std::collections::VecDeque<Result<Vec<u8>, FileReadFailure>>>
    {
        self.file_script
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Every `diff` so far, as (`owner/name`, number).
    pub fn diff_calls(&self) -> Vec<(String, u64)> {
        self.diff_calls
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    fn lock_diff_script(
        &self,
    ) -> std::sync::MutexGuard<'_, std::collections::VecDeque<Result<String, DiffReadFailure>>>
    {
        self.diff_script
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Every `list_branches` so far, as (`owner/name`, branches).
    pub fn list_calls(&self) -> Vec<(String, Vec<String>)> {
        self.list_calls
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    fn lock_list_script(
        &self,
    ) -> std::sync::MutexGuard<'_, std::collections::VecDeque<ListReading>> {
        self.list_script
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Every `read` so far, as (`owner/name`, branches).
    pub fn calls(&self) -> Vec<(String, Vec<String>)> {
        self.calls
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    fn lock_script(&self) -> std::sync::MutexGuard<'_, std::collections::VecDeque<Reading>> {
        self.script
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

impl PullRequestSource for FakePullRequestSource {
    /// A call with nothing scripted answers [`ReadingFailure::Passing`], which changes nothing a
    /// row shows.
    fn read(
        &self,
        repo: &GithubRepo,
        branches: &[String],
        _now: u64,
    ) -> Result<StatusReading, ReadingFailure> {
        self.calls
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push((repo.to_string(), branches.to_vec()));
        self.lock_script()
            .pop_front()
            .unwrap_or(Err(ReadingFailure::Passing))
    }

    /// Nothing scripted answers [`ReadingFailure::Passing`].
    fn list_branches(
        &self,
        repo: &GithubRepo,
        branches: &[String],
        _now: u64,
    ) -> Result<Vec<BranchPullRequest>, ReadingFailure> {
        self.list_calls
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push((repo.to_string(), branches.to_vec()));
        self.lock_list_script()
            .pop_front()
            .unwrap_or(Err(ReadingFailure::Passing))
    }

    /// Nothing scripted answers [`ReadingFailure::Passing`] as a failure.
    fn diff(&self, repo: &GithubRepo, number: u64, _now: u64) -> Result<String, DiffReadFailure> {
        self.diff_calls
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push((repo.to_string(), number));
        self.lock_diff_script()
            .pop_front()
            .unwrap_or(Err(DiffReadFailure::Failed(ReadingFailure::Passing)))
    }

    /// Nothing scripted answers [`FileReadFailure::HeadUnavailable`].
    fn file_at(
        &self,
        repo: &GithubRepo,
        commit: &str,
        path: &RelPath,
        _now: u64,
    ) -> Result<Vec<u8>, FileReadFailure> {
        self.file_calls
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push((repo.to_string(), commit.to_string(), path.clone()));
        self.lock_file_script()
            .pop_front()
            .unwrap_or(Err(FileReadFailure::HeadUnavailable))
    }
}

#[cfg(test)]
mod file_at_tests {
    use super::*;

    fn repo() -> GithubRepo {
        GithubRepo::from_remote_url("https://github.com/acme/widgets").expect("a repo")
    }

    fn path(text: &str) -> RelPath {
        RelPath::from_native(text).expect("relative")
    }

    #[test]
    fn the_args_ask_for_the_raw_file_at_the_commit() {
        let args = file_args(&repo(), "abc123", &path("src/a.rs"));
        assert!(args.contains(&"Accept: application/vnd.github.raw".to_string()));
        assert_eq!(
            args.last().map(String::as_str),
            Some("repos/acme/widgets/contents/src/a.rs?ref=abc123")
        );
    }

    #[test]
    fn each_path_segment_is_percent_encoded() {
        let args = file_args(&repo(), "abc", &path("my dir/a#b?.rs"));
        assert_eq!(
            args.last().map(String::as_str),
            Some("repos/acme/widgets/contents/my%20dir/a%23b%3F.rs?ref=abc")
        );
    }

    fn exited(code: i32, stdout: &[u8], stderr: &str) -> RunOutcome {
        RunOutcome::Exited {
            code,
            stdout: stdout.to_vec(),
            stderr: stderr.to_string(),
        }
    }

    #[test]
    fn a_failed_run_maps_to_a_file_failure() {
        assert_eq!(file_outcome(&exited(0, b"x\n", "")), Ok(b"x\n".to_vec()));
        assert_eq!(
            file_outcome(&exited(1, b"", "gh: Not Found (HTTP 404)")),
            Err(FileReadFailure::Missing)
        );
        assert_eq!(
            file_outcome(&exited(1, b"", "gh: boom")),
            Err(FileReadFailure::HeadUnavailable)
        );
        assert_eq!(
            file_outcome(&RunOutcome::TimedOut {
                stderr: String::new()
            }),
            Err(FileReadFailure::HeadUnavailable)
        );
    }

    #[test]
    fn the_fake_answers_in_order_and_records_calls() {
        let fake = FakePullRequestSource::new()
            .with_file_answer(b"head\n".to_vec())
            .with_file_failure(FileReadFailure::Missing);
        let p = path("a.rs");
        assert_eq!(fake.file_at(&repo(), "c1", &p, 0), Ok(b"head\n".to_vec()));
        assert_eq!(
            fake.file_at(&repo(), "c2", &p, 0),
            Err(FileReadFailure::Missing)
        );
        assert_eq!(
            fake.file_at(&repo(), "c3", &p, 0),
            Err(FileReadFailure::HeadUnavailable)
        );
        assert_eq!(fake.file_calls().len(), 3);
        assert_eq!(fake.file_calls()[0].1, "c1");
    }
}

#[cfg(test)]
mod branch_pr_state_serde_tests {
    use super::BranchPrState;

    #[test]
    fn every_state_round_trips_with_its_lower_case_name() {
        for (state, name) in [
            (BranchPrState::Open, "open"),
            (BranchPrState::Draft, "draft"),
            (BranchPrState::Merged, "merged"),
            (BranchPrState::Closed, "closed"),
        ] {
            let json = serde_json::to_string(&state).expect("serialises");
            assert_eq!(json, format!("\"{name}\""));
            assert_eq!(
                serde_json::from_str::<BranchPrState>(&json).ok(),
                Some(state)
            );
        }
    }
}
