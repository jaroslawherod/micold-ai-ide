# Data Model: Review a Worktree's Changes and Send Comments to Its Session

All types below are new unless marked. Core types live in `crates/micold-core/src/review/` (no iced,
no PTY); client view state in `crates/micold-client/src/features/changes.rs`; daemon state in
`crates/micold-daemon/src/review.rs`.

## Core: the entry and its scope

| Type | Shape | Rules |
|---|---|---|
| `ReviewEntry` | `SessionLocation` (existing: `Default` or `Worktree(dir_name)`) | The key for comments and for the view. Comments of one entry are never shown in, or sent to a session of, another (FR-021). |
| `ReviewScope` | `enum { Worktree { base: Base }, RootUncommitted }` | `RootUncommitted` for the Default entry: it has no committed toggle by construction (FR-003, US1 s9). |
| `Base` | `enum { MergeBase { branch: String, commit: String }, Unavailable(BaseUnavailable) }` | Resolved per read, never persisted (R7). |
| `BaseUnavailable` | `enum { NoDefaultBranch, NoCommonHistory }` | Shown as the reason committed changes are missing. |
| `Toggles` | `struct { committed: bool, uncommitted: bool }` | Default both on. For `RootUncommitted` only `uncommitted` is read. Both off: empty list with the "both hidden" message. |
| `DiffRange` | `enum { BaseToHead, HeadToWorktree, BaseToWorktree }` | Derived from `Toggles` × `Base`: committed only → `BaseToHead`; uncommitted only → `HeadToWorktree`; both → `BaseToWorktree`; base unavailable → `HeadToWorktree`. Pure: `DiffRange::for_view(&ReviewScope, Toggles) -> Option<DiffRange>` (`None` when nothing is switched on). |

## Core: changed files and diffs

| Type | Fields | Rules |
|---|---|---|
| `RelPath` | `String`, `/`-separated, relative to the entry root | Built only from git's `-z` output or by `RelPath::from_native`; never absolute; never contains `\` as a separator (Principle VI, FR-015). |
| `ChangeKind` | `Added`, `Modified`, `Deleted`, `Renamed { from: RelPath }`, `ModeOnly`, `Untracked` | `Untracked` is shown as added. |
| `Content` | `Text`, `Binary`, `NotUtf8` | `Binary`/`NotUtf8`: listed and marked, no diff body, no comments (FR-008). |
| `ChangedFile` | `path: RelPath`, `kind: ChangeKind`, `content: Content`, `added: u32`, `removed: u32`, `large: bool`, `origin: Origin` | `large` per R8 limits. `origin: Committed / Uncommitted / Both` (Key Entities). One entry per path even when changed in both (FR-004). List sorted by path. |
| `ChangeList` | `files: Vec<ChangedFile>`, `base: Base` | Built by pure `parse_numstat` + `parse_name_status` + `merge_untracked`. |
| `DiffLine` | `kind: LineKind { Context, Added, Removed }`, `old: Option<u32>`, `new: Option<u32>`, `text: String` | `text` has no line ending (CRLF and LF stripped). A line-ending-only change is a Removed + Added pair (Edge Cases). `\ No newline at end of file` is dropped from the lines and recorded on the hunk. |
| `Hunk` | `old_start, old_len, new_start, new_len: u32`, `lines: Vec<DiffLine>` | From `@@` headers. |
| `FileDiff` | `enum { Text(Vec<Hunk>), Binary, NotUtf8, ModeOnly, TooLarge { added, removed } }` | `TooLarge` until the user asks (US1 s7). |
| `UnifiedRow` | `Hunk header` or `Line(&DiffLine)` | `unified_rows(&FileDiff)`. |
| `SideRow` | `left: Option<Cell>`, `right: Option<Cell>`; `Cell { number: u32, text, kind }` | `side_by_side_rows(&FileDiff)`: context lines on both sides; each run of removed lines paired index-by-index with the following run of added lines, the shorter side padded with `None`. Both layouts show the same changed lines (US1 s4). |
| `SideLines` | the full text of one side's version as lines, when loaded | Used for outdated checks and quotes. |

## Core: review comments

| Type | Fields | Rules |
|---|---|---|
| `CommentId` | `Uuid` (v4) | Assigned by the daemon. |
| `Side` | `New`, `Old` | `Old` = a removed line, numbered in the base version (US2 s4). |
| `LineRange` | `start: NonZeroU32`, `end: NonZeroU32` | `start <= end` enforced by `LineRange::new`; one line is `start == end`. |
| `ReviewComment` | `id`, `path: RelPath`, `side`, `range`, `quote: Vec<String>`, `text: String`, `state: CommentState`, `created: u64` (Unix seconds) | `quote.len() == range.len()` at creation; `text` trimmed and non-empty; context or added lines are `New`, removed lines `Old`; a range never mixes sides (FR-011, FR-012). |
| `CommentState` | `Pending`, `Sent { at: u64 }` | Moves `Pending → Sent` only on delivery (FR-017). No way back. |
| outdated | derived: `ReviewComment::is_outdated(&self, lines: Option<&SideLines>) -> bool` | True when the lines at `range` on `side` differ from `quote`, or the file is gone (`None`). Not stored (R14). |
| `EntryReview` | `comments: Vec<ReviewComment>` | Pure operations, each returning `Result<_, ReviewError>`: `add`, `set_text`, `delete`, `clear_sent`, `discard_pending`, `begin_send -> SendSnapshot`, `finish_send(&SendSnapshot, at)`, `abort_send`. While a snapshot is open its comments refuse `set_text`/`delete` (`ReviewError::InSend`), and a second `begin_send` is `ReviewError::Busy` (R6). `begin_send` with no pending comment is `ReviewError::NothingPending` (US2 s5). |
| `SendSnapshot` | `ids: Vec<CommentId>`, `prompt: String` | Built from exactly the pending comments at `begin_send` (FR-014, US2 s6). |

State transitions of one comment:

```text
          add                 begin_send            finish_send
 (none) ------> Pending ------------------> Pending* ------------> Sent
                 |  ^ set_text                |  abort_send (failure)
                 |  |                         +--------------------> Pending
                 v
        delete / discard_pending -> (none)          Sent --clear_sent--> (none)

 Pending* = pending and inside the open send: no set_text, no delete.
```

## Core: prompt and target

| Item | Shape | Rules |
|---|---|---|
| `prompt::build(entry_label, comments: &[&ReviewComment], outdated: &BTreeSet<CommentId>) -> String` | pure | Format in [contracts/review-prompt.md](contracts/review-prompt.md). |
| `target::pick_target(candidates: &[(SessionId, Uptime)]) -> Option<SessionId>` | pure | Largest `Uptime`; ties by `SessionId` order; empty → `None` (start a new one). Candidates are the entry's running sessions only (R5). |

## Core: persistence

`reviews/<project id>.json` (project id = `store.rs`'s `project_id`), beside `projects/`:

```json
{
  "version": 1,
  "entries": [
    { "location": { "worktree": "feature-x" },
      "comments": [ { "id": "…", "path": "src/a.rs", "side": "new",
                      "start": 12, "end": 14, "quote": ["…","…","…"],
                      "text": "…", "state": { "pending": null }, "created": 1790000000 } ] },
    { "location": "default", "comments": [] }
  ]
}
```

A missing file is an empty review; an unparseable one is kept aside as `<name>.corrupt` and an empty
review loaded (store.rs pattern, catalog C4). Entries with no comments are not written.
`sending` is never persisted.

## Client: the Changes view (render-free)

`features::changes::State` (one per window):

| Field | Meaning |
|---|---|
| `open: Option<OpenView>` | `None`: the view is closed. |
| `OpenView.entry` | `SessionLocation` of the entry shown; the project is the window's active project. |
| `OpenView.toggles: Toggles` | Per view, reset on open (FR-004 defaults). |
| `OpenView.list: Load<ChangeList>` | `Load { Idle, Loading { seq, again }, Ready(T), Failed(String) }` (pr_status `Phase` pattern). |
| `OpenView.selected: Option<RelPath>` | Kept across refreshes while the path is still listed. |
| `OpenView.diff: Load<LoadedDiff>` | `LoadedDiff { diff: FileDiff, old: Option<SideLines>, new: Option<SideLines>, spans: Spans }`; `Spans` per side per line `Vec<(Range<usize>, Rgb)>` (R10). |
| `OpenView.shown_large: BTreeSet<RelPath>` | Files whose large diff the user asked to show. |
| `OpenView.pick: Option<Pick>` | Gutter selection: `Pick { side, anchor: u32, head: u32 }`, one side only. |
| `OpenView.composer: Option<Composer>` | `Composer { target: ComposerTarget::New(Pick) / Edit(CommentId), text: String }`; survives a refresh (Edge Cases). |
| `OpenView.confirm_discard: bool` | The discard-pending confirmation is open (FR-019). |
| `reviews: BTreeMap<SessionLocation, ReviewView>` | Last `ReviewChanged` per entry of the active project: `comments`, `sending: bool`. |

The diff layout is not here: it is the service-owned `Settings::diff_layout` (R12).

## Daemon

`review::Reviews` inside `DaemonState` (behind the existing mutex):
`HashMap<PathBuf /* project */, HashMap<SessionLocation, EntryState>>`, `EntryState { review:
EntryReview, sending: Option<SendSnapshot> }`. Per session: `last_active: Uptime` (in-memory, R5).
Removing a worktree through `ops::delete_worktree` removes its `EntryState` and rewrites the file
(FR-020, US4 s5); a worktree that disappears from the catalog on refresh is pruned the same way.
