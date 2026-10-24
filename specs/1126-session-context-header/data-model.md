# Data model: Worktree context header and session tabs above the terminal (#1126)

Nothing is stored or sent. Every entity is derived from the window's state on each render, except
`HeaderCounts`, which holds the last answer of an in-memory read (R5). No setting, no persisted
field, no wire message changes.

## ChangeSummary (new, `micold_core::context_header`)

| Field | Type | Rule |
|---|---|---|
| `files` | `u32` | `list.files.len()` |
| `added` | `u32` | sum of `ChangedFile.added` (0 for a binary file, as the view's rows) |
| `removed` | `u32` | sum of `ChangedFile.removed` |

`ChangeSummary::of(&ChangeList)`; saturating sums.

## CountsReading (new, `micold_core::context_header`)

What the header knows of the counts: `Unknown` (no read answered yet, a read under way, or a read
that failed or cannot run here) or `Known(ChangeSummary)`. Built by the client from
`HeaderCounts.load`: `Ready(s)` → `Known(s)`; `Idle`, `Loading { .. }` (whatever `last` holds) and
`Failed(_)` → `Unknown` (US3 scenario 4).

## PrChipInput (new, `micold_core::context_header`)

`{ number: u64, state: PrState, review: ReviewState, conflict: bool, stale: bool }`, built by the
client from `pr_status.statuses.get(branch)` only while `pr_status.enabled`, with
`stale = pull_request::is_stale(read_at, now)`. `None`: no branch, status off, or no current pull
request (FR-005).

## HeaderText (new, `micold_core::context_header`)

`HeaderText::of(&CardModel, Option<&PrChipInput>, CountsReading)`. Rules:
[contracts/context-header.md](contracts/context-header.md).

| Field | Type | Rule |
|---|---|---|
| `title` | `String` | `model.title` (feature 1120's rule, FR-002) |
| `title_tooltip` | `String` | the title, then the display name on a second line when it differs |
| `state` | `CardState` | `model.state`; its word `CardState::label`, its reason `model.reason.label()` as tooltip (FR-003) |
| `branch` | `Option<Chip>` | `None` without a branch; text `short_branch(b)` else `b`, tooltip the full branch (FR-004) |
| `pull_request` | `Option<Chip>` | `None` without input; text `PR #n · <state word>[ · <phrase>]`, tooltip the same plus *as of the last reading* when stale (FR-005) |
| `changes` | `String` | `Changes`, then `· N file(s) · +A −R`, or `· no changes`, or nothing while `Unknown` (FR-006) |
| `changes_tooltip` | `String` | *Open the Changes view* / *Close the Changes view*, chosen by the client from whether the view is open on this location (FR-007) |

`Chip { text: String, tooltip: String }`.

## StatusPhrase (changed, `micold_core::pr_list`)

New constructor `StatusPhrase::of_status(&PullRequestStatus) -> Option<StatusPhrase>`: the same
candidates, in the same order, as `StatusPhrase::of(&ListedPr)`, read from the status's `state`
(checks inside `Open`/`Draft`), `review` and `conflict`; `None` for merged or closed.

## HeaderCounts (new, `client::features::changes`)

| Field | Type | Notes |
|---|---|---|
| `entry` | `Option<(PathBuf, SessionLocation)>` | the project and location the counts are for; `None` while no location is shown |
| `load` | `Load<ChangeSummary>` | the view's `Load` type: `Idle`, `Loading { seq, again, last }`, `Ready`, `Failed` |
| `unwatched` | `bool` | set by the follow step while the Overview hides the header (no watch); cleared, with a `HeaderRecount`, when it shows again |

Transitions:

| Message | From | To | Effect |
|---|---|---|---|
| `HeaderFollowed { project, Some(entry) }`, entry differs | any | `entry` set, `Loading { seq }` | `ReadCounts { seq, entry }` |
| `HeaderFollowed { .., None }` | any | `entry: None`, `Idle` | none |
| `HeaderFollowed` with the same entry | any | unchanged | none |
| `HeaderCountsRead { seq, Ok(s) }`, `seq` current | `Loading` | `Ready(s)`; a queued `again` reads once more | `ReadCounts` if `again` |
| `HeaderCountsRead { seq, Err(_) }`, `seq` current | `Loading` | `Failed`; a queued `again` reads once more | `ReadCounts` if `again` |
| `HeaderCountsRead` with an old `seq` | any | unchanged (dropped) | none |
| `Changed` | `Ready`/`Failed`/`Idle` with an entry | `Loading { last }` | `ReadCounts` (besides the view's own re-read) |
| `Changed` | `Loading` | `again = true` | none |
| `WorktreesListed(dirs)`, entry's dir gone | any | `entry: None`, `Idle` | none |
| `HeaderRecount` (the root's follow step: the Overview, which turns the header's watch off, closed) | `Ready`/`Failed`/`Idle` with an entry | `Loading { last }` | `ReadCounts` |

## StripModel (new, `client::features::session_strip`)

`StripModel::of(&State) -> Option<StripModel>`; `None` without an active session. Rules:
[contracts/session-area.md](contracts/session-area.md).

| Field | Type | Notes |
|---|---|---|
| `members` | `Vec<Member>` | in order: session tabs; the shown session's shell tabs and `Plus` right after its own tab |
| `status` | `String` | the shown session's status words (`session_status`) |
| `restart` | `Option<Message>` | the restart press, while the shown session's attached process is stopped |
| `location` | `Option<SessionLocation>` | the shown location, for **New session**; `None` when it no longer resolves |

`Member`:

| Variant | Fields | Press | Width |
|---|---|---|---|
| `Session` | `id`, `label` (`<command> · <name>`), `tooltip`, `shown: bool`, `marked: bool`, `stopped: bool` | shown: `TerminalAiCliSelected(id)`, right-click `StripTabMenuRequested(StripTab::Ai, ..)`; other: `Session(Selected(id))`, no menu | `tab::SESSION_WIDTH` |
| `Shell` | `session`, `instance`, `label` (ordinal), `marked`, `stopped` | `ShellInstanceSelected(session, instance)`; close: `ShellInstanceCloseRequested`; right-click `StripTabMenuRequested(StripTab::Instance(instance), ..)` | `tab::WIDTH` |
| `Plus` | — | `ShellInstanceOpenRequested` | the icon button's size |

`MemberKey`: `Session(SessionId)`, `Shell(SessionId, ShellInstanceId)`, `Plus` — the keyed row's
keys (R10). Exactly one member is marked (`marked_tab` is total). A stopped mark is that member's own
process's (`process_stopped(state, member's session, member's tab)`).

## StripTab (moved, `client::features::session_strip`)

Unchanged enum (`Instance(ShellInstanceId)`, `Ai`), moved from `ui/terminal.rs` with `marked_tab`,
`process_stopped` and the overflow arithmetic (R7). `ui/terminal.rs` re-exports them
(`pub use crate::features::session_strip::{StripTab, process_stopped, ..}`), so `ui/mod.rs`'s menu
and the integration tests that import `micold_client::ui::terminal::StripTab` are unchanged;
`features/session.rs` (`shell_instance_menu`, `strip_tab_menu_requested`, `Msg::StripTabMenuRequested`)
switches to `crate::features::session_strip::StripTab`.
