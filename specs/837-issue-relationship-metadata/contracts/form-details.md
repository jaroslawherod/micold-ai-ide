# Contract: the New worktree form's issue details

Extends 034-github-issue-worktree contracts/issue-picker-ui.md.

## 1. Reducer (`features::worktree_form`)

- **New field**: `WorktreeForm.issue_details: IssueDetailsState` (data-model §6), with
  `details_seq: u64`.
- **One writer of the pick**: `set_picked_issue(Option<u64>)`. Every write of `picked_issue` goes
  through it: `issue_picked`, `reset_issues`, and any other path that clears the pick. It moves
  `issue_details` as data-model §6 says. A Jira pick, or a pick while the provider is not GitHub,
  goes to `Idle`.
- **New message**: `Msg::IssueDetailsLoaded { seq: u64, number: u64, result: Result<IssueDetails, IssueLoadError> }`.
  It applies only to `Loading { seq, number }` with both equal.
- **New query**: `State::issue_details_request() -> Option<DetailsRequest { seq, number, repo: GithubRepo, gh: PathBuf }>`.
  It is `Some` only while `Loading`, the list is `IssueList::Loaded { gh, .. }`, and `github_repo()`
  is `Some`.
- **Create**: `submitted` and `create_started` do not read `issue_details` (US2-4, FR-006).

## 2. Shell (`shell::issues`)

- **Starting the read**: after every form message that may change the pick (`IssuePicked`,
  `IssueQueryChanged`, `SourceChanged`, `Cancelled`, …), compare `issue_details_request()` before
  and after. When the new value is `Some` and different, start
  `Task::perform(spawn_blocking(source(gh).read_details(repo, number, now + 10 s)))`, mapped to
  `IssueDetailsLoaded { seq, number, result }`. This is the `update_then_follow` pattern the
  description pass uses. The comparison is made once, in a helper shared by those handlers, not
  repeated in each.
- **Number**: `read_details` takes `u32`, as `IssueLookup` does. `issue_details_request()` converts
  the picked `u64` with `u32::try_from`; a number that does not fit gives `None`, so no read starts.
- **Never started**: for the New or Existing source, for a Jira pick, or before a pick (SC-006).

## 3. View (`ui::worktree_form::issue_details`, GUI glue)

Placed directly below the issue picker, only while the GitHub source is shown and an issue is
picked.

| State | Shown |
|---|---|
| `Idle`, `Loading` | nothing |
| `Failed` | one muted caption: "Couldn't read this issue's details. <message>" (US3-3) |
| `Loaded` | one row per `display_lines()` entry: a caption-role label, then the text in body role. A warning line has its label tinted `r.error` and its text led by "Blocked:" |

Line texts (`IssueDetails::display_lines`, core, tested):

| Label | Text |
|---|---|
| Blocked by | `#10 Parser rewrite (open), #9 Old spike (closed)`, then ` and N more`. With hidden items: `; N you can't see`. Warning when `is_blocked()`, and the text then starts `1 open of 2: …` |
| Blocking | the list, as above |
| Parent | `acme/roadmap#3 Q4 epic (open)` |
| Sub-issues | `1 of 2 closed: #15 … (closed), #16 … (open)`, then ` and N more` |
| Closing pull requests | `#20 … (draft)`, then ` and N more` |
| Linked pull requests | as above. A lower bound reads `at least N: …` |
| Assignees | `octocat, hubot`, then ` and N more` |
| Milestone | `v1.4` |
| Projects | `Roadmap: Status In progress, Estimate 3; Backlog` |
| any kind, not readable | `Not readable: <reason>` (US3-1) |

- **Order and omission**: lines come in the table's order. A kind with nothing in it has no line,
  so an issue without relationships adds no row (US2-3, SC-002).
- **Text**: every text is `PlainText`, drawn with the existing `ui::material::Text`. No new widget
  (Principle VIII).
