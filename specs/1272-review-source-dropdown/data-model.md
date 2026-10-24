# Data model: #1272

All types are in-memory only: nothing persisted, nothing on the wire (research R9).

## Core: `micold_core::review::source_menu` (NEW, research R1)

### 1. `Shown<'a>`: what the view shows

`Shown::{WorkingTree, PullRequest { number: u64, title: Option<&'a str> }}`. `title` is the listed
pull request's title, `None` while the list does not hold the number (R5).

### 2. `PrRows<'a>`: the pull-request list state (R2)

`PrRows::{Absent, Loading, Unavailable, Empty, List { prs: &'a [BranchPullRequest], stale: bool }}`.
Replaces the client's `features::changes::PrSection`, which becomes `pub use
micold_core::review::source_menu::PrRows as PrSection;` so `section(view, enabled, now)` and its
tests keep their names. `Absent` covers the PR status setting off and an entry with no branch (the
Default entry).

### 3. `MenuInput<'a>`

`MenuInput { shown: Shown<'a>, files_on: bool, toggles: Toggles, committed_available: bool, prs:
PrRows<'a> }`. `Toggles` is `review::base::Toggles` (both on by default). Built by the client's
`source_menu_input(view: &OpenView, enabled: bool, now: u64) -> MenuInput<'_>` (NEW in
`features/changes.rs`) from `view.source`, `view.files.shown`, `view.toggles`,
`committed_available(view)` and `section(view, enabled, now)`.

### 4. `Row<'a>`: one line of the open list, in order

`rows(&MenuInput) -> Vec<Row>`:

1. `Row::WorkingTree { chosen: bool, committed: Chip, uncommitted: Chip, note: Option<&'static str> }`
   — `chosen` when `shown` is the working tree; `Chip { on: bool, available: bool }`; Committed is
   `available: committed_available`, `on: toggles.committed && available`; `note` is
   `DEFAULT_ENTRY_NOTE` when Committed is unavailable.
2. Pull-request part, by `prs`:
   - `Absent`: nothing (no heading).
   - `Loading` / `Unavailable` / `Empty`: `Row::Heading` then `Row::Note(PRS_LOADING |
     PRS_UNAVAILABLE | PRS_EMPTY)`.
   - `List { prs, stale }`: `Row::Heading`, `Row::Stale` when `stale`, then one
     `Row::PullRequest { pr: &BranchPullRequest, chosen: bool }` per entry in list order (newest
     first, as read), `chosen` when `shown` has its number.
3. `Row::Files { on: files_on }`.

Invariants (unit-tested): exactly one Working tree row first and one Files row last; at most one
row is `chosen` among Working tree and pull requests, and exactly one when `shown` is listed or the
working tree; `Files.on == files_on` independently; no `Heading` without a following `Note` or
pull request.

### 5. `Stop` and the keyboard (R7)

`Stop::{WorkingTree, Kind(Kind), PullRequest(u64), Files}`, `Kind::{Committed, Uncommitted}`.
`stops(&[Row]) -> Vec<(Stop, bool /* enabled */)>` lists what the keyboard reaches in row order:
Working tree, Committed, Uncommitted, each pull request, Files. `Kind(Committed)` is disabled when
unavailable. `Stop::keeps_open(self) -> bool` is true only for `Kind(_)`.

### 6. `Label<'a>` (R5)

`label(&MenuInput) -> Label { fixed: String, title: Option<&'a str> }`, `Label::full() -> String`.

| shown | files_on | fixed | title |
|---|---|---|---|
| working tree, both listable kinds on | no | `Working tree` | none |
| working tree, only committed | no | `Working tree · committed` | none |
| working tree, only uncommitted | no | `Working tree · uncommitted` | none |
| working tree, no listable kind | no | `Working tree · nothing shown` | none |
| working tree, any kinds | yes | `Files · Working tree` | none |
| PR #n listed | no | `#n · ` | its title |
| PR #n listed | yes | `Files · #n · ` | its title |
| PR #n not listed | either | `#n` / `Files · #n` | none |

`tooltip(base_line: Option<&str>, &Label) -> String`: `Label::full()` (so a truncated title is
readable, Edge "Long PR title"), then on a second line the base line when there is one (FR-009,
Story 4 s3); `Label::full()` alone before the first read answers.

### 7. Wording constants (R2)

`WORKING_TREE`, `PULL_REQUESTS`, `FILES`, `PRS_LOADING`, `PRS_UNAVAILABLE`, `PRS_EMPTY`,
`POSSIBLY_STALE`, `DEFAULT_ENTRY_NOTE`, `KINDS_COMMITTED`, `KINDS_UNCOMMITTED`, `KINDS_NONE`. The
client's same-named constants in `ui/changes.rs` and `features/changes.rs` become re-exports.

## Client: `features::changes` (render-free reducer)

### 8. `SourceMenu` on `OpenView` (NEW field `source_menu`)

`SourceMenu { open: bool, highlight: Option<usize> }`, default closed. Reset by `Msg::Opened` and
whenever the view's entry changes.

### 9. Messages

| Message | NEW? | Effect on `source_menu` | Other effect |
|---|---|---|---|
| `SourceMenuToggled` | NEW | open ↔ closed; opening sets `highlight` to the chosen source's stop | none |
| `SourceMenuDismissed` | NEW | closed, highlight cleared | source unchanged (FR-011) |
| `SourceMenuMoved(Direction)` | NEW | `move_highlight` over `stops` | none |
| `SourceMenuPicked` | NEW | per the highlighted stop: closes unless `keeps_open` | dispatches `SourceChosen(None)`, `CommittedToggled`, `UncommittedToggled`, `SourceChosen(Some(n))` or `FilesPaneToggled` |
| `SourceChosen(_)` | changed | closed | as today (1003) |
| `FilesPaneToggled` | changed | closed | as today (1150) |
| `CommittedToggled`, `UncommittedToggled` | changed | unchanged (stays open) | with a pull request shown: back to the working tree first, then flip (R4) |
| `PullRequestsRead` | changed | unchanged; `highlight` re-seated with `clamp_highlight` | as today; a shown PR that disappeared falls back to the working tree (1003) |

## State transitions

Closed → (`SourceMenuToggled`) → Open → (pick of Working tree / PR / Files, `Esc`, outside click,
`Tab`) → Closed. Open → (chip, by mouse or keyboard) → Open. Source and Files transitions are the
existing ones of 1003 and 1150; this feature adds only the chip-while-PR edge (R4).
