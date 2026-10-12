# Data model: PRs in the diff view (#775)

All types are in-memory; none derive `Serialize` that reaches disk or the wire. `Debug` of anything
holding a title or address prints number and state only (as `PullRequestStatus`).

## 1. micold-core

### `pull_request::BranchPullRequest` (Session PR)
| Field | Type | Note |
|---|---|---|
| number | `u64` | |
| title | `String` | never logged |
| url | `String` | never logged |
| state | `BranchPrState` = `Open`, `Draft`, `Merged`, `Closed` | built from `state`+`isDraft`; unknown state makes the read `ReadingFailure::Passing` |
| created_at | `String` | ordering key, newest first (FR-011) |
| base | `String` | `baseRefName`, shown in the PR header |
| head | `String` | `headRefOid` |

Cross-repository (fork) PRs are dropped. `PrState` is not reused because its `Open`/`Draft` carry
check status this list does not read.

### `PullRequestSource` (extended)
- `list_branch(&self, repo, branch, now) -> Result<Vec<BranchPullRequest>, ReadingFailure>`
- `diff(&self, repo, number) -> Result<String, DiffReadFailure>`; `DiffReadFailure` = `TooLarge`,
  `Failed(ReadingFailure)`.
`FakePullRequestSource` gains scripted answers and recorded calls for both.

### `review::pr_diff`
- `PrDiff { files: Vec<PrFile> }`, `PrFile { file: ChangedFile, patch: Vec<u8> }`
- `split_pr_diff(raw: &str) -> PrDiff`: splits at `diff --git`, reads path, kind (new file, deleted
  file, `rename from`), binary marker and added/removed counts; `ChangedFile.origin = Committed`,
  `large` by `limits::MAX_CHANGED_LINES`. `patch` is handed to `parse_unified` on selection.

## 2. micold-client `features/changes.rs`

- `Source` = `Changes` | `PullRequest { number: u64, diff: Load<PrDiff> }` (field `OpenView.source`).
  While `PullRequest`, `list`/`selected`/`diff` of the working-tree view are kept, not read.
- `PrList` = `Absent` (setting off, no branch, no GitHub remote, no `gh`) | `Loading` |
  `Ready { prs: Vec<BranchPullRequest>, read_at: u64 }` | `Unavailable { last: Option<(Vec<..>, u64)> }`
  (field `OpenView.pull_requests`). Stale = `is_stale(read_at, now)` at draw time.
- `State.remembered: BTreeMap<(PathBuf, SessionLocation), u64>`.

### Transitions
- Opened/entry switch: `source = remembered.get(..)` if the list later contains it, else `Changes`;
  `pull_requests = Loading` and `Effect::ReadPullRequests` (only when enabled and entry has a branch).
- `PullRequestsRead(Ok)`: replace list; selected PR gone -> `Source::Changes`, forget it (FR-013).
- `PullRequestsRead(Err)`: keep last list; never raises a notice (FR-008, FR-012). A `RateLimited{until}` from a list read is fed to the existing `pr_status` `Outcome::Err` path (the shell sends it the same `finished`-style message), so `pause_until` is set and also gates the next list read (research R3).
- `SourceChosen(None | Some(n))` -> `Changes` | `PullRequest` + `Effect::ReadPullRequestDiff`.
- `PullRequestDiffRead` fills `Load<PrDiff>`; file selection inside a PR parses `patch` without I/O.
- `PullRequestSettingChanged(false)` -> `Absent`, `Source::Changes`.
