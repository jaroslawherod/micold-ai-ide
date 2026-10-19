# Contract: Changes view changes (#1003)

Extends `specs/775-diff-view-session-prs/contracts/changes-view.md`.

## Messages and effects (`features::changes`)

- `Msg::PullRequestStepped(Step)`, `Step::{Previous, Next}`: select the adjacent entry; nothing at
  an end or while the working tree is shown.
- `Effect::ReadPullRequests { seq, project, entry, current }`: `entry` names the worktree whose
  directory the branch set is read in; `current` its branch, if any.
- `Msg::PullRequestsRead` unchanged; `Err(Unavailable)` now keeps the last list (or none).

## Section (`ui/changes.rs`, drawn from `section(..)`)

| `PrSection` | Drawn |
|---|---|
| `Absent` | nothing |
| `Loading` | "Pull requests" title, muted "Reading pull requests…" |
| `Unavailable` | title, muted "Pull requests can't be shown for this worktree" |
| `Empty` | title, "No pull requests from this worktree yet" |
| `List` | title, "Working tree" row, one `PullRequestRow` per entry (`#number title`, state mark with `.checks(..)`), "possibly stale" caption when `stale`; rows in a scrollable of bounded height |

## Header (while a pull request is shown)

Above the diff: `PullRequestIndicator` (state + checks), `#number title`, the base line
"Pull request #N: <base> ... <head_branch>", and two `IconButton`s, previous and next, each
disabled when `adjacent(..)` is `None`. New `Icon::ChevronLeft`/`ChevronRight` glyphs.

## Shared components

`PullRequestRow` gains `.checks(CheckMark)` (builder, Principle VIII), passed to its
`PullRequestIndicator`; the showcase's review section shows a row with each check mark.
