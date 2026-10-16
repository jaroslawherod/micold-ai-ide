# Research: issue relationship metadata (837)

Each entry: Decision, Rationale, Alternatives considered.

## R1. Where the read lives

- **Decision**: in a new module, `micold-core/src/issue_details.rs`. Both the daemon (tool) and the
  client (form) call it, through a trait the existing `GhCli` and `FakeIssueSource` implement.
- **Rationale**: the spec requires one shared reading (Assumptions; 550 FR-016). 550 put the
  single-issue lookup in core for the same reason, and the daemon already depends on core's `GhCli`
  and `locate_gh_on_host`. `github.rs` is 1,800 lines, so a separate module keeps it readable. The
  new module reuses `GhCli::run`, `classify` and the GraphQL error typing instead of copying them.
- **Alternatives**: read in the daemon and let the form ask the daemon over the protocol. Rejected:
  the form already runs `gh` itself (034), a protocol round trip adds a message type and a version
  bump, and the form works the same with a remote daemon. Extending `ISSUE_QUERY` (550's lookup)
  with the relationships was also rejected: a slow or partly refused heavy query would then refuse
  the worktree, which breaks FR-006.

## R2. One query, errors mapped to kinds by path

- **Decision**: one `gh api graphql` document (contract
  [details-read.md](contracts/details-read.md)) fetches every kind. GitHub answers a field-level
  failure (for example `INSUFFICIENT_SCOPES` on `projectItems` when the token lacks `read:project`)
  with partial `data` and an `errors[]` entry whose `path` names the field. The parser marks the
  kind that owns the first field after `issue` in that path as `NotReadable(reason)`, and keeps
  every other kind. An error with no path into the issue, or with `data.repository.issue` absent,
  is a whole failure (`IssueLoadError`), typed by the existing GraphQL error typing and `classify`.
- **Rationale**: one process run keeps the read well inside 10 s, and per-path mapping gives US3-1
  ("projects reported as not readable, every other kind shown") without one query per kind.
- **Alternatives**: one query per kind (6 to 8 `gh` runs per pick, each with its own process start
  and rate-limit cost) was rejected. All-or-nothing was rejected because it violates FR-007.

## R3. Which GitHub fields

- **Decision**:
  - `blockedBy(first: 20)` and `blocking(first: 20)`, with `issueDependenciesSummary { blockedBy totalBlockedBy blocking totalBlocking }`.
    `blockedBy` there is the open-blocker count; the totals count open and closed.
  - `parent`.
  - `subIssues(first: 20)` with `subIssuesSummary { total completed }`.
  - `labels(first: 20) { totalCount }`, `assignees(first: 20) { totalCount }`, `milestone { title }`.
  - `projectItems(first: 20) { project { title } fieldValues(first: 20) }`, with inline fragments for
    the text, number, date, single-select and iteration value types and their `field { ... on ProjectV2FieldCommon { name } }`.
  - `closedByPullRequestsReferences(first: 20, includeClosedPrs: true)`.
  - `timelineItems(itemTypes: [CROSS_REFERENCED_EVENT], first: 100)` with `source { ... on PullRequest }`.

  Each issue or PR node carries `number title state repository { nameWithOwner }`, plus `isDraft`
  and `merged` for PRs.
- **Rationale**: these are GitHub's public GraphQL fields for issue dependencies, sub-issues,
  Projects (v2) and PR links. The summaries give exact totals and the open-blocker count for any
  list length (FR-003, FR-008).
- **Verification step for implementation**: the first core task records a live answer for an issue
  on github.com that has each kind (and one with none), and stores it as a fixture with logins and
  titles replaced. A field name that differs from this list is corrected in the contract in that
  task. A field GitHub does not offer is answered by an `undefinedField` error. That fails the whole
  query, so the query must contain only fields confirmed by that recording.
- **Confirmed (M1, T001)**: `gh` was not signed in where M1 was built, so the fields were checked
  against GitHub's published GraphQL schema (`github/docs` `src/graphql/data/fpt/schema.docs.graphql`,
  2026-10-16): `Issue` has `blockedBy`, `blocking`, `issueDependenciesSummary { blockedBy blocking
  totalBlockedBy totalBlocking }`, `parent`, `subIssues`, `subIssuesSummary { total completed }`,
  `projectItems`, `closedByPullRequestsReferences(includeClosedPrs:)` and `timelineItems`; the value
  types carry `text`, `number` (a float), `date`, `name` (single select) and `title` (iteration),
  each with `field: ProjectV2FieldConfiguration`, whose members implement `ProjectV2FieldCommon {
  name }`. No name differs from this list. The fixtures in
  `crates/micold-core/tests/fixtures/issue_details/` are written to that schema, not recorded.
- **Enterprise**: `GithubRepo` is constructed only for github.com remotes (034 FR-022), so a GitHub
  Enterprise repository never reaches the read. 550 already refuses `github_issue` there and the
  form offers no GitHub source. The spec's "server without dependencies" edge case reduces to the
  per-kind error path of R2.

## R4. Linked pull requests and their count

- **Decision**:
  - **Closing PRs**: `closedByPullRequestsReferences`, whose `totalCount` gives the count.
  - **Linked PRs**: PR sources of cross-reference events, without the closing ones (spec Edge Cases)
    and without repeats. The first page has 100 events. While `pageInfo.hasNextPage` and the deadline
    leaves time, further timeline-only pages are read, up to 10 pages (1,000 events).
  - A count that rests on an incomplete timeline is marked a lower bound: JSON `"total_is_lower_bound": true`,
    form "at least N".
- **Rationale**: GitHub has no count of "PRs that mention this issue". The timeline's `totalCount`
  also counts issue sources. Paging within the deadline gives an exact count for every realistic
  issue. The lower-bound mark keeps the rare remainder from being understated silently, which is
  FR-008's purpose.
- **Alternatives**: a search query (`is:pr <N>`) was rejected because it matches numbers in text
  and misses other repositories. Reporting only the first 100 with no mark was rejected because it
  understates silently.

## R5. Hidden items

- **Decision**: for blockers, blocked issues and sub-issues, the hidden count is the summary total
  minus the connection's `totalCount`. A connection whose nodes include `null`, which is how GitHub
  answers an item the token cannot see, adds those nulls to the hidden count. PR connections use
  null nodes only. A hidden parent shows as no parent, because GitHub gives no signal for it. This
  is recorded as a known limit in the user guide.
- **Rationale**: the spec requires hidden items to be counted so "blocked by" is not understated
  (Edge Cases, FR-008). The summary counts include every item, which gives that count directly.
- **Alternatives**: none viable without extra API calls.

## R6. The 10-second bound and the tool's latency

- **Decision**: the daemon starts the read on the blocking pool once the remote is resolved. It
  runs in parallel with 550's lookup and with the create, and is awaited afterwards with
  `tokio::time::timeout(details_bound - elapsed_since_start)`. `GhCli` bounds each `gh` run by the
  time left before the same deadline, so no process outlives it. On timeout the result reports
  "GitHub didn't answer within 10 seconds" (the existing `IssueLoadError::Timeout` sentence).
- **Rationale**: SC-005 allows a tool call at most 10 s more than without the feature. Starting
  early means the extra wait is usually zero, because the create takes longer than the read.
  Awaiting after the create keeps the worktree independent of the read (FR-006, FR-011).
- **Alternatives**:
  - Read after the create: correct, but adds the whole read latency to every call.
  - Read inside the lookup: rejected in R1.
  - Return without waiting: rejected by the clarification (the tool waits).

## R7. Form timing and stale answers

- **Decision**: the read starts at the pick. A `details_seq` on the form, bumped on every pick
  change, and the picked number both tag the request. The answer applies only when both match the
  `Loading` state. While the read runs the form shows nothing extra: no spinner, so an issue with no
  relationships never flickers a row in and out (US2-3, SC-002). **Create** is independent of the
  state.
- **Rationale**: this is the same `seq` pattern the issue list, search and description pass already
  use (034, 038), so the shell's "request changed, start a task" logic is reused. Showing nothing
  while loading is a choice; the spec asks for no loading indicator.
- **Alternatives**: a "Reading details…" caption was rejected because it adds a line for every issue,
  including those with nothing to show. Reading on hover was rejected because the spec says at pick.

## R8. Display text and neutralising

- **Decision**: `PlainText::new` applies `description_from`'s rule, minus whitespace folding and the
  length cut (titles are short, and `gh` caps them at 256 characters). Control, zero-width and
  direction-override characters are dropped; the joiners are kept between visible characters. The
  character tests (`is_invisible`, `is_joiner`) are already in `github.rs`. The shared loop becomes
  one `pub(crate) fn visible_text`, used by both.
- **Rationale**: FR-013 points to 038's tooltip behaviour. A type that can only be built through
  the neutraliser makes an un-neutralised title unrepresentable in the model (Principle V).

## R9. Nothing persisted, nothing logged

- **Decision**: `IssueDetails` derives no `Serialize`. `to_json` builds the tool's JSON value by hand,
  and the value lives only in that one reply. `Debug` is hand-written to print the number and counts
  only, as `Issue` redacts its reporter (038 FR-025). The form state is dropped with the form.
- **Rationale**: Principle IV, and 034's "held only in the open form" rule for GitHub data.

## R10. Sidebar extensibility (not planned)

- **Decision**: no sidebar change (spec US2-6, decided by the user, 2026-10-16). The model sits in core
  with a pure `is_blocked()` and no dependence on the form, so a later sidebar marker needs only a
  holder for an `IssueDetails` per worktree and a refresh policy. The read and the model would not
  change.
