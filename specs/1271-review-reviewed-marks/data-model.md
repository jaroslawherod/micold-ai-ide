# Data model: Review UI — reviewed marks and progress (#1271)

Extends 482's, 946's and 1149's data models. Only what is new or changed is listed. All core types
live in the new module `crates/micold-core/src/review/marks.rs` unless stated.

## Core: `review::marks` (new)

### `ChangeId`

A file's change's identity (research R1): 20 bytes, written as 40 lowercase hex characters in JSON
and on the wire (`#[serde(try_from = "String", into = "String")]`, so postcard and JSON share one
shape). Built only by:

| Constructor | Input | Record digested |
|---|---|---|
| `ChangeId::of_worktree(&WholeFile)` | a working-tree file (below) | `w`, kind tag, path, old mode, old id, new mode, new id |
| `ChangeId::of_patch(&ChangedFile, &[u8])` | a pull request file and its patch section | `p`, kind tag, path, patch bytes |

Kind tags: `A` added, `M` modified, `D` deleted, `R` + old path renamed, `T` mode only, `U`
untracked. Fields are separated by `\0` (a path never holds one). `ChangeId::parse(&str)` accepts
exactly 40 hex characters (stored lower-cased); anything else is refused.

### `WholeFile`, `WholeChange`

The entry's whole working-tree change, whatever the toggles (R2).

| Field | Type | Rule |
|---|---|---|
| `WholeFile.path` | `RelPath` | the path now (new path of a rename) |
| `WholeFile.kind` | `ChangeKind` | as `ChangedFile.kind` |
| `WholeFile.origin` | `Origin` | Committed, Uncommitted or Both; Uncommitted for the Default entry and an entry without a base |
| `WholeFile.id` | `Option<ChangeId>` | `None` when a side's object id could not be read (R2 step 3) |
| `WholeChange.files` | `BTreeMap<RelPath, WholeFile>` | one per path |

`WholeChange::assemble(names: Vec<NameStatus>, committed, uncommitted, hashed: BTreeMap<RelPath,
String>, untracked: Vec<RelPath>) -> WholeChange` is the pure part; `GitCli::whole_change` only
runs git and calls it.

### `ReviewMark`

| Field | Type | Rule |
|---|---|---|
| `scope` | `Scope` (1149's) | `WorkingTree` or `PullRequest(n)`, `n > 0` |
| `path` | `RelPath` | worktree-relative, `/`-separated, as comments (Principle VI) |
| `change` | `ChangeId` | the file's change the mark was made on |

### `EntryMarks`

All marks of one entry: `BTreeMap<(Scope, RelPath), ChangeId>`, at most one mark per file per
source (FR-006). Operations:

| Operation | Rule |
|---|---|
| `set(scope, path, Some(id)) -> bool` | inserts or replaces (re-mark, FR-016); `false` when it already held `id` |
| `set(scope, path, None) -> bool` | removes (unmark); `false` when there was none |
| `get(scope, &path) -> Option<&ChangeId>` | |
| `of_scope(scope)` | the scope's marks, for the shown source |
| `to_vec()` / `from_vec(Vec<ReviewMark>)` | wire and store order: by scope, then path; a duplicate key keeps the last |
| `is_empty()` | |

Nothing else removes a mark: not a file leaving the list, a differing change, a pull request
merged or closed or leaving the list, a send, Clear sent or Discard drafts (FR-007, FR-016). An
entry's marks go only with the entry (R3).

### `ReviewState`, `FileReview` (derived, never stored)

`ReviewState = Unseen | HasDrafts | Reviewed`. `FileReview { state: ReviewState, changed_since:
bool }`.

`file_review(mark: Option<&ChangeId>, current: Option<&ChangeId>, drafts: usize) -> FileReview`:

| mark | current | drafts | state | changed_since |
|---|---|---|---|---|
| `Some(m)` | `Some(c)`, `c == m` | any | Reviewed | false |
| `Some(m)` | `Some(c)`, `c != m` | 0 / >0 | Unseen / HasDrafts | true |
| `Some(_)` | `None` (never read) | 0 / >0 | Unseen / HasDrafts | false |
| `None` | any | 0 / >0 | Unseen / HasDrafts | false |

Reviewed wins over drafts (FR-001, D2). With marks unavailable (R6) the caller passes `mark: None`.

### `progress`

`progress(files: impl Iterator<Item = FileReview>) -> Option<Ratio>` (`task_progress::Ratio`):
`None` for no files; otherwise `done` = the Reviewed ones, `total` = all listed (FR-008, FR-009).
The caller passes only the files the list shows now (*m* follows the toggles).

### `MarkBlock` and `mark_block` (D4, provisional: research R7)

`MarkBlock = Disconnected | NotRead | HiddenKind`, each with one tooltip constant.
`mark_block(origin: Option<Origin>, toggles: Toggles, scope: &ReviewScope) -> Option<MarkBlock>`
answers `NotRead` for `origin == None` (no id yet) and `HiddenKind` when `origin` includes a kind
the toggles hide (Committed hidden and origin Committed or Both; Uncommitted hidden and origin
Uncommitted or Both; the Default entry and a worktree without a base never hide their only kind).
`Disconnected` is decided by the client (R6). Only **Mark reviewed** is blocked; unmarking never is.

## Core: `review::comment` (changed)

`EntryReview` gains `marks: EntryMarks` and `from_parts(comments, marks)`; `from_comments` stays
(no marks). `is_empty()` is true only with no comments **and** no marks. New
`set_reviewed(scope, path, change) -> bool` delegates to `EntryMarks::set`. No other operation
touches `marks`.

## Core: `review::changes` (changed)

`NameStatus` gains `old_mode`, `new_mode`, `old_id`, `new_id` (`String`, as git printed them),
filled by the same `parse_name_status`. Existing callers ignore them.

## Core: `review::pr_diff` (changed)

`pr_change_ids(files: &[ChangedFile], patches: &BTreeMap<RelPath, Vec<u8>>) -> BTreeMap<RelPath,
ChangeId>`: one id per file with a patch, by `ChangeId::of_patch`.

## Core: `review::store` (changed)

`ReviewFile` gains `marks: BTreeMap<String, Vec<ReviewMark>>` (keyed like `entries`). Version 4,
written only while some entry has marks; else 3 or 2 as today; reads 1 to 4 (research R5, S4 in
contracts/marks-wire.md). Stored per entry:

```json
{ "location": { "worktree": "feat-x" },
  "comments": [],
  "marks": [ { "scope": "WorkingTree", "path": "src/a.rs", "change": "<40 hex>" },
             { "scope": { "PullRequest": 12 }, "path": "src/a.rs", "change": "<40 hex>" } ] }
```

## Wire (`protocol::messages`)

- `ReviewEditOp::SetReviewed { scope: Scope, path: String, change: Option<ChangeId> }`.
- `DaemonMsg::ReviewChanged` gains `marks: Vec<ReviewMark>`.
- `PROTOCOL_VERSION` +1. Contract: contracts/marks-wire.md.

## Client: `features::changes` (render-free)

| Item | Change | Milestone |
|---|---|---|
| `ReviewView.marks: EntryMarks` | filled from `Msg::ReviewChanged.marks` | before #1269 |
| `Msg::ReviewChanged.marks` | new field | before #1269 |
| `OpenView.whole: Load<WholeChange>` | read with the list, kept while a re-read runs (`Loading { last }`) | after #1269 |
| `Msg::ListRead.whole: Result<WholeChange, String>` | new field | after #1269 |
| `Msg::MarkToggled` | the control was pressed: `Effect::ReviewEdit(SetReviewed { .. })` with the current id, or `None` when Reviewed | after #1269 |
| `current_ids(view) -> BTreeMap<RelPath, ChangeId>` | working tree: from `whole`; pull request: `pr_change_ids` of the ready diff | after #1269 |
| `file_reviews(state, view, connected) -> BTreeMap<RelPath, FileReview>` | for the listed files of the shown source | after #1269 |
| `MarkControl { label, pressed, block: Option<MarkBlock> }`, `mark_control(state, view, connected)` | `None` with no file selected | after #1269 |
| `header_progress(state, view, connected) -> Option<Ratio>` | `progress` of the listed files; `None` while disconnected | after #1269 |

## Shell (`micold-client/src/shell`)

- `daemon_sync.rs`: `DaemonMsg::ReviewChanged.marks` into `Msg::ReviewChanged`; `PendingOp::ReviewMark`
  with failure text "save the reviewed mark" (before #1269).
- `changes.rs`: `read_list` also runs `git.whole_change(dir, &scope)`; `Effect::ReviewEdit` for
  `SetReviewed` uses `PendingOp::ReviewMark` (after #1269).
