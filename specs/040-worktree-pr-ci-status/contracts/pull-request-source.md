# Contract: the pull request source

**Feature**: [spec.md](../spec.md) | **Research**: [R1–R4, R9, R10](../research.md) |
**Data**: [data-model §1–2](../data-model.md)

Everything in this contract lives in `micold_core::pull_request` (render-free, `mise run
test-core`), except the `GhCli` impl, which sits beside 034's in `micold_core::github` so both
share one runner.

---

## 1. The trait, its production impl and its fake

```rust
pub trait PullRequestSource {
    /// The pull request of each branch in the project's repository. One GitHub request per
    /// 50 branches. `now` is Unix seconds, used only to turn `Retry-After` into a time.
    fn read(
        &self,
        repo: &GithubRepo,
        branches: &[String],
        now: u64,
    ) -> Result<BTreeMap<String, PullRequestStatus>, ReadingFailure>;
}
```

- **`GhCli`** implements it with the runner 034 gave it (`process::run_bounded`, 10 s, the
  non-interactive environment, the user's home as working directory). The bound is per `gh` run, so
  per chunk of 50 branches (FR-021).
- **`FakePullRequestSource`** (builder form, as `FakeIssueSource`): scripted answers per call and a
  record of `(owner/name, branches)` per call. The client's reducer and shell tests use it; nothing
  in a test runs `gh`.
- `branches` empty ⇒ `Ok(empty map)` and **no `gh` run** (spec Edge Cases "a project with no
  worktrees makes no request").
- More than 50 branches ⇒ chunks of 50, sent one after another; the first chunk that fails ends the
  reading with that failure, and nothing of the earlier chunks is returned (nothing half-read).
- The caller constructs a source only after `locate_gh` found `gh` and `choose_remote` found a
  github.com repository; either missing is `ReadingFailure::Unavailable` without constructing one
  (FR-026: no remote ⇒ `gh` is not run).

---

## 2. The command and the query

```text
gh api graphql --hostname github.com --include
   -f query=<status_query(n)>
   -f owner=<owner> -f name=<name>
   -f b0=<branch 0> … -f b<n-1>=<branch n-1>
```

built by the pure `status_args(repo, branches) -> Vec<String>`.

- **Only `-f`** (raw string), never `-F`: a branch named `true`, `123` or `@file` stays a string.
- **Nothing else about the project is sent** (FR-031): the arguments are the query text, the
  repository's owner and name, and branch names. No path, no worktree name, no commit id.
- **No credential is passed or read** (FR-028): no token argument, no `Authorization` header, no
  `GH_TOKEN` set by the application; `gh` uses the user's own sign-in, and when there is none it
  fails (`Unavailable`) rather than ask anonymously.
- **The only repository ever named is the project's own** (`choose_remote`), so a pull request in
  the repository a fork was made from is never asked for (FR-006).
- `--hostname github.com` always: a `GH_HOST` in the user's environment cannot redirect the request
  (FR-005, github.com only).

`status_query(n)` (1 ≤ n ≤ 50) returns one line:

```graphql
query($owner: String!, $name: String!, $b0: String!, … ) {
  repository(owner: $owner, name: $name) {
    o0: pullRequests(headRefName: $b0, states: OPEN, first: 10,
                     orderBy: {field: CREATED_AT, direction: DESC}) { nodes { ...pr } }
    r0: pullRequests(headRefName: $b0, first: 10,
                     orderBy: {field: CREATED_AT, direction: DESC}) { nodes { ...pr } }
    …
  }
  rateLimit { remaining resetAt }
}
fragment pr on PullRequest {
  number title url state isDraft createdAt isCrossRepository headRefOid reviewDecision
  commits(last: 1) { nodes { commit { statusCheckRollup { contexts(first: 1) {
    checkRunCount checkRunCountsByState { state count }
    statusContextCount statusContextCountsByState { state count } } } } } }
}
```

Tests: the document for n = 1 and n = 50 is pinned; no branch name ever appears in the document;
`status_args` for a branch holding `"`, `$`, a space and a leading `-` yields exactly one argument
`b<i>=<branch>` per branch.

**Known bound.** Each connection holds the 10 newest pull requests whose head branch has the name,
forks' included. A worktree's own pull request is missed only when more than 10 newer pull requests
with the same head-branch name exist in that connection. Not handled further.

---

## 3. `select_pull_request`

```rust
pub fn select_pull_request(open: &[PrNode], recent: &[PrNode]) -> Result<Option<PullRequestStatus>, Unreadable>
```

`PrNode` is the parsed fragment. Rule, in order (FR-004, FR-005):

1. drop every node with `isCrossRepository: true`;
2. `open` not empty ⇒ its node with the greatest `createdAt`;
3. else `recent` not empty ⇒ its node with the greatest `createdAt`;
4. else `Ok(None)`.

| `state` | `isDraft` | `PrState` |
|---|---|---|
| `OPEN` | `true` | `Draft { checks }` |
| `OPEN` | `false` | `Open { checks }` |
| `MERGED` | any | `Merged` |
| `CLOSED` | any | `Closed` |
| anything else | | `Err(Unreadable)` → §5 `Passing` |

`createdAt` is ISO-8601 in UTC with a fixed layout, so it is compared as a string. `checks` is §4
applied to the selected node's last commit.

Tests (each one case): open beats a newer merged; newest of two open; newest of merged and closed
when none open; a cross-repository open node is ignored and the same-repository closed one is
shown; only cross-repository nodes ⇒ `None`; a draft; an unknown `state`.

---

## 4. `reduce_checks`

```rust
pub fn reduce_checks(rollup: Option<&CheckCounts>) -> CheckStatus
```

| Check-run state | Status-context state | Group |
|---|---|---|
| `FAILURE`, `CANCELLED`, `TIMED_OUT`, `ACTION_REQUIRED`, `STARTUP_FAILURE` | `FAILURE`, `ERROR` | failed |
| `QUEUED`, `IN_PROGRESS`, `PENDING`, `WAITING`, `STALE` | `PENDING`, `EXPECTED` | not finished |
| `SUCCESS`, `NEUTRAL`, `SKIPPED`, `COMPLETED` | `SUCCESS` | finished without failure |
| any other name | any other name | not finished |

Result, first match wins:

1. `rollup` is `None` (`statusCheckRollup: null`, or the pull request has no commit), or every
   count is zero ⇒ `CheckStatus::None`;
2. any failed count > 0 ⇒ `Failing`;
3. any not-finished count > 0 ⇒ `Pending`;
4. otherwise ⇒ `Passing`.

Tests: one table-driven test with a row per state name of both lists (19 names) alone; failing
beside pending and passing; pending beside passing; only skipped and neutral ⇒ `Passing`; an
unknown name alone ⇒ `Pending`; `null` rollup; all zero.

---

## 5. Reading the answer: `split_response`, `parse_status`, `rate_limit_pause`, `reading_failure`

`gh api --include` writes the HTTP status line, the headers, an empty line and then the body to
stdout — for an error answer too, with exit status 1.

```rust
pub fn split_response(stdout: &[u8]) -> Option<Response<'_>>   // status: u16, headers, body
pub fn parse_status(body: &[u8], branches: &[String])
    -> Result<BTreeMap<String, PullRequestStatus>, ReadingFailure>
pub fn rate_limit_pause(response: &Response<'_>, now: u64) -> u64
pub fn reading_failure(outcome: &RunOutcome, now: u64) -> ReadingFailure
```

- **`split_response`**: accepts `\r\n` and `\n` line ends; header names compared without case;
  `None` when there is no status line or no empty line (⇒ `Passing`).
- **`parse_status`**: `data.repository` holding `o<i>` and `r<i>` for every `i` ⇒ §3 per branch.
  A GraphQL `errors` entry, a missing alias, a `null` repository or JSON that does not parse is a
  failure, never a partial map. The key of the result is `branches[i]`, never a name taken from the
  answer.
- **`rate_limit_pause`**, called only for a rate-limit answer (row 3 below):
  1. `Retry-After: <seconds>` ⇒ `now + seconds`;
  2. else `X-RateLimit-Remaining: 0` ⇒ the value of `X-RateLimit-Reset: <epoch>`;
  3. else `now + 60`.

  `X-RateLimit-Reset` is on every answer GitHub sends and is the primary window's reset, so it is
  used only when the primary limit is the one that was hit. A value that does not parse is skipped
  as if absent. The result is never before `now + 1`.
  Tests: `Retry-After: 30`; remaining 0 with a reset; a 403 secondary limit with remaining above 0
  and no `Retry-After` ⇒ `now + 60`; a reset in the past ⇒ `now + 1`.
- **`reading_failure`** maps evidence to data-model §2, checked in this order:

| # | Evidence | Result |
|---|---|---|
| 1 | `RunOutcome::TimedOut` | `Passing` |
| 2 | `SpawnFailed` | `Unavailable` when 034's `classify` says `ToolMissing`, else `Passing` |
| 3 | a rate-limit answer: a GraphQL error of type `RATE_LIMITED`; HTTP 429; HTTP 403 whose body or stderr says "rate limit" (primary or secondary); `classify` says `RateLimited` | `RateLimited { until: rate_limit_pause(…) }` |
| 4 | a GraphQL error of type `NOT_FOUND` on `repository`; HTTP 401; `classify` says `NotSignedIn` or `NoAccess` (403, SAML, missing scope) | `Unavailable` |
| 5 | `classify` says `Offline` | `Passing` |
| 6 | anything else, including HTTP 5xx, an answer `parse_status` cannot read and an unknown pull request `state` | `Passing` |

  Rate limiting is checked before access because GitHub reports both as HTTP 403 (the order 034's
  `classify` already keeps).

**Fixtures** — `crates/micold-core/tests/fixtures/gh/`, recorded from a real `gh` with `--include`
and only then trimmed of tokens and request ids; the recording command is the first line of
`pr_README.md` beside them:

| File | What it is |
|---|---|
| `pr_three_branches.txt` | HTTP 200: one branch with an open pull request, one merged, one without |
| `pr_checks_failing.txt`, `pr_checks_pending.txt`, `pr_checks_passing.txt`, `pr_no_checks.txt` | HTTP 200, one open pull request each |
| `pr_draft.txt`, `pr_closed.txt`, `pr_review_states.txt` | HTTP 200 |
| `pr_cross_repository.txt` | HTTP 200: a fork's pull request with the same head-branch name |
| `pr_repo_not_found.txt` | HTTP 200 with a `NOT_FOUND` error, exit 1 |
| `pr_rate_limited_graphql.txt` | `RATE_LIMITED` with `X-RateLimit-Remaining: 0` and `X-RateLimit-Reset` |
| `pr_rate_limited_secondary.txt` | HTTP 403 with `Retry-After` |
| `pr_rate_limited_secondary_no_retry_after.txt` | HTTP 403 secondary limit, `X-RateLimit-Remaining` above 0, no `Retry-After` |
| `pr_truncated.txt` | a 200 answer cut mid-body |

A state that cannot be produced on demand (a secondary limit) is recorded from GitHub's documented
answer and marked as such in `pr_README.md`. 034's stderr fixtures (`not signed in`, `offline`,
SAML) are reused through `classify`, not copied.

---

## 6. What is never done

- No `gh` run without a github.com remote, with the switch off, or with no branch to ask about.
- No request names a repository other than the project's.
- No title or address is logged: the one `debug` line of a reading is
  `pull request reading: <outcome kind>, <n> branches`.
- No retry inside a reading: the next attempt is the next start event
  ([reading-and-wire §1](./reading-and-wire.md)).
