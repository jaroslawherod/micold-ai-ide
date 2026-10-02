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
use crate::process::RunOutcome;

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

/// Shared by both connections of a branch: the 10 newest pull requests with that head branch.
const NEWEST_TEN: &str = "first: 10, orderBy: {field: CREATED_AT, direction: DESC}";

/// What is read of each pull request. Check counts by state, never the checks themselves: one
/// request stays small however many checks a pull request has.
const FRAGMENT: &str = "fragment pr on PullRequest { number title url state isDraft createdAt \
isCrossRepository headRefOid reviewDecision commits(last: 1) { nodes { commit { \
statusCheckRollup { contexts(first: 1) { checkRunCount checkRunCountsByState { state count } \
statusContextCount statusContextCountsByState { state count } } } } } } }";

/// The GraphQL document that asks about `n` branches (1 to [`BRANCHES_PER_REQUEST`]), on one line.
///
/// Branch `i` is the variable `$b<i>`; `o<i>` holds its open pull requests and `r<i>` its newest of
/// any state. No branch name is part of the document (FR-031): names travel as variables, so no
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
             r{i}: pullRequests(headRefName: $b{i}, {NEWEST_TEN}) {{ nodes {{ ...pr }} }} "
        );
    }
    format!(
        "query($owner: String!, $name: String!{variables}) {{ \
         repository(owner: $owner, name: $name) {{ {aliases}}} \
         rateLimit {{ remaining resetAt }} }} {FRAGMENT}"
    )
}

/// The arguments after `gh` for one request: the query, the repository's owner and name, and the
/// branch names leave the machine, and nothing else (FR-031). No credential is passed (FR-028).
///
/// Every variable is a raw string (`-f`): `-F` would turn a branch named `true` or `123` into a
/// boolean or a number, and read `@file` from disk. `--hostname` keeps a `GH_HOST` in the user's
/// environment from redirecting the request (FR-005).
pub fn status_args(repo: &GithubRepo, branches: &[String]) -> Vec<String> {
    let mut args: Vec<String> = ["api", "graphql", "--hostname", "github.com", "--include"]
        .map(str::to_string)
        .to_vec();
    let mut field = |value: String| {
        args.push("-f".into());
        args.push(value);
    };
    field(format!("query={}", status_query(branches.len())));
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
    let checks = reduce_checks(node.checks.as_ref());
    let state = match (node.state.as_str(), node.is_draft) {
        ("OPEN", true) => PrState::Draft { checks },
        ("OPEN", false) => PrState::Open { checks },
        ("MERGED", _) => PrState::Merged,
        ("CLOSED", _) => PrState::Closed,
        _ => return Err(Unreadable),
    };
    let review = match node.review_decision.as_deref() {
        Some("APPROVED") => ReviewState::Approved,
        Some("CHANGES_REQUESTED") => ReviewState::ChangesRequested,
        Some("REVIEW_REQUIRED") => ReviewState::ReviewRequired,
        _ => ReviewState::None,
    };
    Ok(Some(PullRequestStatus {
        number: node.number,
        title: node.title,
        url: node.url,
        state,
        review,
        head: node.head,
    }))
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
}

#[derive(serde::Deserialize)]
struct Connection {
    nodes: Vec<RawNode>,
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
    commits: RawCommits,
}

#[derive(serde::Deserialize)]
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

impl From<RawNode> for PrNode {
    fn from(raw: RawNode) -> Self {
        let counts = |list: Option<Vec<RawStateCount>>| {
            list.unwrap_or_default()
                .into_iter()
                .map(|entry| (entry.state, entry.count))
                .collect()
        };
        let checks = raw
            .commits
            .nodes
            .into_iter()
            .next_back()
            .and_then(|node| node.commit.status_check_rollup)
            .map(|rollup| CheckCounts {
                check_runs: counts(rollup.contexts.check_run_counts_by_state),
                status_contexts: counts(rollup.contexts.status_context_counts_by_state),
            });
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
        }
    }
}

/// One status per branch that has a pull request, from the body of an answer to
/// [`status_query`]`(branches.len())`.
///
/// The key of an entry is `branches[i]`, never a name taken from the answer. An answer that cannot
/// be read whole — an `errors` entry, a missing alias, a `null` repository, a pull request state
/// this version does not know, JSON that does not parse — is [`ReadingFailure::Passing`], never a
/// partial map (FR-019). [`reading_failure`] names the real kind from the whole `gh` run.
pub fn parse_status(
    body: &[u8],
    branches: &[String],
) -> Result<BTreeMap<String, PullRequestStatus>, ReadingFailure> {
    const UNREADABLE: ReadingFailure = ReadingFailure::Passing;
    let answer: Answer = serde_json::from_slice(body).map_err(|_| UNREADABLE)?;
    if !answer.errors.is_empty() {
        return Err(UNREADABLE);
    }
    let mut repository = answer
        .data
        .and_then(|data| data.repository)
        .ok_or(UNREADABLE)?;
    let mut statuses = BTreeMap::new();
    for (i, branch) in branches.iter().enumerate() {
        let mut nodes = |alias: String| -> Result<Vec<PrNode>, ReadingFailure> {
            let connection = repository.remove(&alias).ok_or(UNREADABLE)?;
            Ok(connection.nodes.into_iter().map(PrNode::from).collect())
        };
        let open = nodes(format!("o{i}"))?;
        let recent = nodes(format!("r{i}"))?;
        if let Some(status) = select_pull_request(&open, &recent).map_err(|_| UNREADABLE)? {
            statuses.insert(branch.clone(), status);
        }
    }
    Ok(statuses)
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

/// The answer of one `gh` run that asked about `branches`: the statuses when `gh` exited 0 with an
/// HTTP 200 that [`parse_status`] reads, else the kind [`reading_failure`] names.
pub fn read_outcome(
    outcome: &RunOutcome,
    branches: &[String],
    now: u64,
) -> Result<BTreeMap<String, PullRequestStatus>, ReadingFailure> {
    if let RunOutcome::Exited {
        code: 0, stdout, ..
    } = outcome
    {
        let read = split_response(stdout)
            .filter(|response| response.status == 200)
            .map(|response| parse_status(response.body, branches));
        if let Some(Ok(statuses)) = read {
            return Ok(statuses);
        }
    }
    Err(reading_failure(outcome, now))
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
    ) -> Result<BTreeMap<String, PullRequestStatus>, ReadingFailure>;
}

/// Read `branches` in chunks of [`BRANCHES_PER_REQUEST`], one after another (FR-023).
///
/// No branch, no call: a project without worktrees makes no request. The first chunk that fails
/// ends the reading with that failure, and nothing of the earlier chunks is returned: a reading is
/// whole or it is a failure (FR-019).
pub fn read_in_chunks(
    branches: &[String],
    mut read_chunk: impl FnMut(&[String]) -> Result<BTreeMap<String, PullRequestStatus>, ReadingFailure>,
) -> Result<BTreeMap<String, PullRequestStatus>, ReadingFailure> {
    let mut statuses = BTreeMap::new();
    for chunk in branches.chunks(BRANCHES_PER_REQUEST) {
        statuses.extend(read_chunk(chunk)?);
    }
    Ok(statuses)
}

/// One scripted answer of a [`FakePullRequestSource`].
type Reading = Result<BTreeMap<String, PullRequestStatus>, ReadingFailure>;

/// A scripted [`PullRequestSource`] for tests: answers each `read` with the next scripted answer
/// and records every call. Public (not `#[cfg(test)]`) so every crate's tests can use it, like
/// [`crate::github::FakeIssueSource`]. Nothing in a test runs `gh`.
#[derive(Debug, Default)]
pub struct FakePullRequestSource {
    script: std::sync::Mutex<std::collections::VecDeque<Reading>>,
    calls: std::sync::Mutex<Vec<(String, Vec<String>)>>,
}

impl FakePullRequestSource {
    /// A source with nothing scripted.
    pub fn new() -> Self {
        Self::default()
    }

    /// Answer the next unanswered `read` with `statuses`.
    pub fn with_answer(self, statuses: BTreeMap<String, PullRequestStatus>) -> Self {
        self.lock_script().push_back(Ok(statuses));
        self
    }

    /// Answer the next unanswered `read` with `failure`.
    pub fn with_failure(self, failure: ReadingFailure) -> Self {
        self.lock_script().push_back(Err(failure));
        self
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
    ) -> Result<BTreeMap<String, PullRequestStatus>, ReadingFailure> {
        self.calls
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push((repo.to_string(), branches.to_vec()));
        self.lock_script()
            .pop_front()
            .unwrap_or(Err(ReadingFailure::Passing))
    }
}
