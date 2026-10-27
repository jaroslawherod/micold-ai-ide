# Data model: Readable worktree titles

All state is in memory, per window; nothing is persisted (Principle IV). New items are marked
**new**, extended ones **extended**.

## Core (`crates/micold-core/src/location_title.rs`, new module)

### TitleTicket (new)

| Variant | Meaning |
|---|---|
| `Github(u64)` | An all-digit ticket. |
| `Jira(String)` | A `<LETTERS>-<digits>` key, uppercased. |

Built only by `title_ticket(dir_name, branch)` (research R2). Folder ticket first, then branch.

### TitleSource (new)

`GithubIssue(u64) | JiraIssue(String) | PullRequest(u64) | Spec | Session | Commit | Branch | Folder`,
in FR-001 order. `words(&self) -> Option<String>`: `From GitHub issue #1504`, `From Jira ABC-42`,
`From pull request #1520`, `From the feature spec`, `From the session title`,
`From the latest commit`, `Branch name`; `None` for `Folder` (FR-008: today's tooltip).

### TitleInputs<'a> (new)

| Field | Type | From |
|---|---|---|
| `ticket` | `Option<&TitleTicket>` | `title_ticket` |
| `issue_title` | `Option<&str>` | GitHub `TitleLookup::Known`, or Jira `StatusBoard::summary_of` |
| `pull_request` | `Option<(u64, &str)>` | open/draft `PullRequestStatus` of the branch (R9) |
| `spec` | `Option<&str>` | `TitleFacts.spec` |
| `session` | `Option<&str>` | main session's `Named`/`Derived` label (R8) |
| `commit` | `Option<&str>` | `TitleFacts.commit` |
| `branch` | `Option<&str>` | `Worktree.branch` |
| `display_name` | `&str` | `WorktreeNode.display_name` / card display name |

### LocationTitle (new)

`{ text: String, source: TitleSource }`. `choose(&TitleInputs) -> LocationTitle`: the first input in
FR-001 order whose `first_line` (R7) is `Some`; `branch` and `display_name` are used as they are
(today's behaviour). `Debug` omits `text`.

### Functions (new)

- `title_ticket(dir_name: &str, branch: Option<&str>) -> Option<TitleTicket>`
- `first_line(text: &str) -> Option<&str>`
- `spec_title(text: &str) -> Option<String>`
- `read_spec_title(root: &Path, ticket: Option<u64>) -> Option<String>` (blocking; `ticket` is the
  GitHub title ticket's number, R5; uses
  `task_progress::feature_dir` and `task_progress::read_text`, made `pub(crate)`)

### Removed

`overview_text::card_title`: callers move to `choose` (its tests move with it).

## Core (extended)

### `git::Git` trait (extended)

`fn own_commit_subject(&self, dir: &Path) -> Option<String>` — `GitCli`: merge-base from
`review_base(dir)`, then `git log -1 --format=%s <base>..HEAD` through the bounded review `read`;
`None` on `Base::Unavailable`, empty output, error or timeout. `FakeGit`:
`with_own_commit(dir, subject)`.

### `jira_status` (extended)

- `KeyedIssue { key: String, status: Option<TicketStatus>, summary: Option<String> }` replaces
  `KeyedStatus`; `parse_statuses` reads `fields.summary`; `JiraSource::statuses` returns it.
- `StatusBoard` (extended): `summaries: BTreeMap<String, String>`, `status_keys: BTreeSet<String>`
  (of the read in flight). `begin(status_keys, summary_keys)`: asks the union, records which are
  status keys. `apply`: on `Ok`, statuses only for status keys, summaries for every asked key; on
  `Err`, summaries kept like statuses (stale) until retired, then cleared. `clear()` clears both.
  `summary_of(key) -> Option<&str>`. `Debug` of the board omits summaries.

## Client (`crates/micold-client/src/features/overview.rs`, extended)

### TitleLookup (extended)

| State | Meaning | Re-asked |
|---|---|---|
| `Pending` | Asked, not answered. | Never while pending. |
| `Known(String)` | The title. | Never in the run. |
| `NoIssue` (new) | Not an issue, a pull request, or no GitHub repo. | Never in the run. |
| `Failed { at: u64 }` (new, replaces `Unavailable`) | Read failed at Unix second `at`. | On an occasion with `now - at >= 300`. |

Transitions: none → `Pending` (`TitlesRequested`); `Pending` → `Known | NoIssue | Failed`
(`TitleRead`); `Failed` → `Pending` (`TitlesRequested` after the interval); any → none on
`ProjectSwitched`.

### TitleFacts (new)

`{ spec: Option<String>, commit: Option<String> }`, per worktree dir in
`State.facts: BTreeMap<String, TitleFacts>`. `Debug` omits texts. Beside it,
`State.facts_round: u64` (bumped per request) and `State.facts_asked: BTreeMap<String, u64>` (the
round last asked per dir). All three cleared on `ProjectSwitched`; entries for dirs no longer listed
are pruned when facts are stored.

### Msg (extended)

- `TitleFactsRequested { project, dirs }` (new): bumps `facts_round`, sets `facts_asked[dir]` to it
  for each dir. Applied by `shell::location_titles::read_facts` when the worktree list or active
  project changes and when the Overview opens (R5).
- `TitleFactsRead { project, dir, round, facts }` (new): stored only when `round ==
  facts_asked[dir]` and `dir` is listed; `read_project` returns `project`.
- `TitleRead.result: TitleLookup` carries the new states.

### Functions

- `titles_to_fetch(state, tickets: &[u64], github_available, now) -> Vec<u64>` (extended).
- `State::issue_title(&self, ticket: &TitleTicket, jira: &StatusBoard) -> Option<&str>` (new,
  replaces `known_title`).
- `location_title(app: &app::State, worktree: &Worktree, display_name: &str) -> LocationTitle`
  (new): gathers `TitleInputs` from state and calls `choose`; the card, the row and the header call
  it.

## Client (other)

- `features::jira_status::State::plan` (extended): asks `linked_keys` as status keys plus
  `branch_title_keys(worktrees, project)` (new: Jira title tickets from branches of worktrees whose
  folder has no ticket) as summary-only keys.
- `micold_core::overview::CardModel.title_source: TitleSource` (new field).
- `overview_text::ExpandedText.source: Option<String>` (new), `context_header::HeaderText`
  title tooltip with the source line (R6).
- `features::sidebar::row_title` takes the `LocationTitle`; `with_title_line` adds the source line.
