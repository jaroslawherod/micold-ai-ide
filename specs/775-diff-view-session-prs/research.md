# Research: PRs in the diff view (#775)

## R1: How to read every PR of a branch
- **Decision**: one `gh api graphql` request per branch, `repository.pullRequests(headRefName: $b,
  first: 100, orderBy: CREATED_AT DESC)`, fields number, title, url, state, isDraft, createdAt,
  isCrossRepository, baseRefName, headRefOid. Built like `status_args` (owner/name/branch as `-f`
  variables, `--hostname github.com`), classified by the existing `reading_failure` (via `read_outcome`'s `RunOutcome` handling).
- **Rationale**: reuses GraphQL auth, rate-limit, failure and `--hostname` handling of feature 040;
  `status_query` only returns the newest open PR and newest of any state, so it cannot list all.
- **Rejected**: `gh pr list --head` (a second output format and failure classification; also
  matches by a text search on some versions); extending `status_query` (grows every sidebar read
  by up to 100 PRs per branch for all worktrees).

## R2: How to read a PR's diff
- **Decision**: `gh api --hostname github.com -H "Accept: application/vnd.github.v3.diff"
  repos/<o>/<n>/pulls/<N>` once on selection; split per file in core (`split_pr_diff`) and feed each
  file's section to the existing `parse_unified`.
- **Rationale**: the PR's own base...head, works after the head branch is deleted (spec
  clarification), one request, no pagination, and the existing parser already reads `git diff`
  text. Per-file selection then needs no further read.
- **Rejected**: local `git diff base...head` (fails when the head or base ref is gone locally or
  not fetched, the exact merged-and-deleted case); `pulls/N/files` JSON (paginated, no patch for
  large files, needs `--slurp` from a newer `gh`); `gh pr diff` (same media type but resolves host
  from `GH_HOST`, FR-005 of 040).
- **Limit**: GitHub answers 406 above 300 files / 20 000 changed lines; shown as "too large to show
  here" with the PR address, never a notice (FR-008 spirit).

## R3: When to refresh
- **Decision**: the list is requested when the view opens and
  on every PR-status trigger already wired (`PR_STATUS_INTERVAL` 300 s, open/refresh events of
  `shell/pr_status.rs`) while a Changes view is open. Staleness uses `pull_request::is_stale`
  (600 s).
- **Rationale**: SC-004 says "within the existing PR status refresh interval"; a second timer would
  add `gh` load and a second set of "read only on named events" guarantees
  (`pr_status_is_read_only_on_named_events.rs`).
- **Rate limit**: the shell skips `ReadPullRequests` while `pr_status` is paused by `RateLimited{until}` (040 FR-024) and while a read is under way; `Unavailable` is re-asked only on a trigger, never in a loop. Tested in `main_tests.rs`.
- **Rejected**: a dedicated Changes-view timer; piggybacking by deriving the list from sidebar
  statuses (those hold one PR per branch).

## R4: Where the selected PR lives
- **Decision**: `Source` enum inside `OpenView` (the open view is per entry), plus an in-memory
  `remembered: BTreeMap<(project, SessionLocation), u64>` in changes `State` so FR-013's "that
  selection's own view" holds when the user switches away and back. A PR missing from a fresh list
  clears it.
- **Rejected**: persisting it (nothing about PRs is stored, 040 FR-032).

## R5: Entries with no branch
- **Decision**: Default (project root) and detached/renamed-away branches show no section. The
  branch comes from the listed worktree's `branch: Option<String>` (as `shell/pr_status.rs::branches`).

## R6: Several sessions, one worktree
- **Decision**: key the read by (project, branch); the reducer holds one `PrList` per open view, so
  sessions sharing an entry share it. No daemon state.
