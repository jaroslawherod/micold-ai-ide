# Data model: Review changes reaches every pull request of the worktree (#1003)

All in memory; nothing is stored or serialised (040 FR-032). `Debug` of anything holding a title
or address prints number and state only, as 775.

## 1. micold-core

### `worktree_branches` (new module, pure)

| Item | Shape | Note |
|---|---|---|
| `ReflogEntry` | `{ refname: String, time: u64, commit: String, subject: String }` | one line of `%gD\t%H\t%gs`; `refname` without the `@{…}` |
| `parse_reflog(text) -> Vec<ReflogEntry>` | newest first, as git prints | a line that does not split into 3 fields, or whose `@{…}` is not a number, is skipped |
| `candidates(head_log, current) -> Vec<String>` | dedup, in first-seen order | research R2 rule 1 |
| `branch_set(head_log, ref_logs, current) -> Vec<String>` | `ref_logs`: entries of `refs/heads/*` for the candidates | research R2 rules 2 to 6, without the default-branch rule |
| `MATCH_WINDOW_SECS` | `2` | R2 rule 2 |
| `copies_remote(subject, name) -> bool` | `branch: Created from refs/remotes/<r>/<name>` or `<r>/<name>` | R2 rule 3 |

A commit name is 40 or 64 lowercase hex characters; `HEAD` is not a branch.

### `Git` trait (`git.rs`): one new method

`fn reflogs(&self, dir: &Path, refs: &[String]) -> io::Result<String>`: runs
`git log -g --ignore-missing --date=unix --format=%gD%x09%H%x09%gs <refs> --` in `dir`. Called
twice by `worktree_branches::read_branch_set(git, dir, current)`: once with `["HEAD"]`, once with
the candidates as `refs/heads/<n>`. A failed HEAD run gives `[current]`; a failed refs run treats
every candidate's record as gone (R2 rule 4). `FakeGit` gains scripted reflog text.

### `pull_request::BranchPullRequest` (extended)

| Field | Change |
|---|---|
| `head_branch: String` | new: `headRefName`, for the header's base line and the default-branch drop |
| `checks: CheckStatus` | new: `reduce_checks` of the last commit's rollup, every state (R4) |

Ordering (FR-004): `created_at` descending, then `number` descending.

### `pull_request` reads (replacing 775's single-branch read)

- `LIST_BRANCHES_PER_REQUEST: usize = 25`
- `list_query(n) -> String`, `list_args(repo, branches) -> Vec<String>` (contracts/pull-request-list.md)
- `parse_list(body, branches) -> Result<Vec<BranchPullRequest>, ReadingFailure>`: per alias, forks
  dropped, the default branch's alias dropped.
- `PullRequestSource::list_branches(repo, branches, now) -> Result<Vec<..>, ReadingFailure>`:
  chunks of 25, first failure fails all, merged and sorted. Empty `branches`: `Ok(vec![])`, no
  request. `list_branch` is removed; `FakePullRequestSource` records `(repo, branches)` per call.

## 2. micold-client `features/changes.rs`

- `Effect::ReadPullRequests { seq, project, entry: SessionLocation, current: Option<String> }`
  (was `branch: String`): the shell reads the branch set in the entry's directory, then the list.
- `request_pull_requests`: reads for every worktree entry while enabled, `current` may be `None`
  (detached HEAD: the set is the created branches alone). The project root (`Default`) stays
  `Absent` (FR-012).
- `PullRequestsRead(Err(Unavailable))` keeps `PrList::Unavailable { last }` (was `Absent`) (R5),
  the same arm as every other error. `view.source` is kept when `last` is `Some` (the shown pull
  request is still in the listed `last`, FR-008); with `last: None` it resets to `Source::Changes`,
  so a shown pull request is never outside any list.
- `Msg::PullRequestStepped(Step)`, `enum Step { Previous, Next }`.
- `pub fn adjacent(prs, number, step) -> Option<u64>`.
- `pub enum PrSection<'a> { Absent, Loading, Unavailable, Empty, List { prs: &'a [BranchPullRequest], stale: bool } }`
  and `pub fn section(view, enabled, now) -> PrSection`, the spec's section-states table:

| `PrList` | section |
|---|---|
| `Absent` | `Absent` |
| `Loading { last: None }` | `Loading` |
| `Unavailable { last: None }` | `Unavailable` |
| any with a list, empty | `Empty` |
| any with a list, not empty | `List`, `stale` by `possibly_stale` |

- `pub fn pr_header(view) -> Option<PrHeader>`: `{ number, title, mark, checks, base_line }` of
  the shown pull request, looked up in the current list; `None` while the working tree is shown.
- `pr_mark` gains a sibling `check_mark(CheckStatus) -> Option<CheckMark>` (moved from
  `ui/sidebar.rs::indicator_marks`, so both draw from one mapping).

### Transitions (new or changed)

- `PullRequestStepped(step)`: shown pull request `n`; `adjacent(list, n, step)` → `Some(m)` runs
  `SourceChosen(Some(m))`; `None` (an end, or the working tree shown) → nothing.
- `PullRequestsRead(Ok(prs))`: unchanged from 775 (selection kept unless its number left).
- `PullRequestsRead(Err(_))`: `Unavailable { last }` for every failure kind.
