# Contract: Changes view additions (#775)

## Messages (`features::changes::Msg`)
`PullRequestsRead { seq, result }`, `SourceChosen(Option<u64>)`, `PullRequestDiffRead { seq, number,
result }`, `PullRequestsWanted` (sent by the shell on the PR-status triggers while a view is open),
`PullRequestSettingChanged(bool)`.

## Effects
`ReadPullRequests { seq, project, branch }`, `ReadPullRequestDiff { seq, project, number }`. Neither
is returned together with `ReadList`/`ReadDiff` as a dependency: the list read is issued first and
never waits (FR-009).

## UI (`ui/changes.rs`)
Above the file list, when `pull_requests` is not `Absent`: a "Working tree" row and one row per PR
(`#number title`, state mark, "possibly stale" caption when stale). The selected row is highlighted.
Selected PR: the file list shows the PR's files with the existing `file_row`, the existing
`diff_pane` and layouts; the base line reads "Pull request #N: <base> ... <head>". "Too large"
shows one line and the PR address to open (existing open-address action of `sidebar`).
