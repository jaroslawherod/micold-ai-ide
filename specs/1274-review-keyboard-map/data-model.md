# Data Model: Keyboard Map for Reviewing Changes (#1274)

All core types are render-free, in-memory and per window; nothing is persisted or sent on the wire
(Principle IV). Core types take plain inputs so they do not depend on the blockers (research R1).

## Core — `micold-core/src/review/keys.rs` (new, M1)

| Type | Fields / variants | Rules |
|---|---|---|
| `ReviewAction` | `NextFile`, `PrevFile`, `NextHunk`, `PrevHunk`, `NextComment`, `PrevComment`, `ToggleReviewed`, `NextUnreviewed`, `Comment`, `ToggleSheet`, `Send` | closed set of FR-001 + FR-010 |
| `KeyName` | `Char(char)`, `Enter` | the key an entry is on |
| `KeyMods` | `primary: bool` (Cmd on macOS, Ctrl elsewhere), `shift: bool` | `shift` is only compared for `primary` entries (R2) |
| `ReviewKey` | `key: KeyName`, `mods: KeyMods`, `action: Option<ReviewAction>`, `repeats: bool`, `sheet: &'static str` | `action` is `None` only for the save-comment chord (sheet row only); `labels() -> (String, String)`: Linux/Windows form, macOS form |
| `REVIEW_KEYS` | `pub const [ReviewKey; 12]` | the one place keys are chosen; order = sheet order; public and stable for #1276's guide test |
| `PressKey` | `Char(char)`, `Enter`, `Other` | `Char` from iced's produced `text` |
| `Press` | `key: PressKey`, `primary: bool`, `ctrl: bool`, `alt: bool`, `logo: bool`, `shift: bool`, `repeat: bool` | built by the client; `primary` from `keymap::primary_held` |
| `Holder` | `Review`, `Composer`, `Sheet`, `Other` | computed by the client (R7) |
| `resolve(press, holder) -> Option<ReviewAction>` | — | `Other` → none; `Sheet` → only `?` (`ToggleSheet`); `Composer` → only `Send`; `Review` → any entry of `REVIEW_KEYS` with an action. Matching per research R2 (modifiers, AltGr, repeat) |

## Core — `micold-core/src/review/navigate.rs` (new, M1)

| Type | Fields / variants | Rules |
|---|---|---|
| `NavFile` | `path: RelPath`, `has_hunks: bool` | one per file, in file order |
| `has_hunks(&ChangedFile, large_shown: bool) -> bool` | — | `Content::Text`, `added + removed > 0`, and not (`large` and not shown) |
| `SelectedDiff` | `Loading`, `Failed`, `Hunks(usize)` | the selected file's diff as the view holds it |
| `Edge` | `First`, `Last` | a hunk to make current once a file's diff loads |
| `HunkMove` | `Current(usize)`, `File { path: RelPath, edge: Edge }`, `Nothing` | result of `step_hunk` |
| `step_file(order, selected, dir) -> Option<RelPath>` | — | `navigation::step` over `order` paths (FR-003) |
| `step_hunk(order, selected, diff, current, dir) -> HunkMove` | — | FR-004 and R5; files with `has_hunks: false` are passed over |
| `resolve_edge(edge, count) -> Option<usize>` | — | `First` → 0, `Last` → count − 1, none for 0 |
| `CommentFact` | `id: CommentId`, `path: RelPath`, `line: u32`, `created: u64`, `in_diff: bool` | `line` = located first line; `in_diff` from the view's existing jump check |
| `comment_order(facts, order) -> Vec<CommentFact>` | — | file index in order, line, created, id; unlisted files last, by path |
| `Show` | `Card`, `Drawer` | |
| `CommentMove` | `id: CommentId`, `select: Option<RelPath>`, `show: Show` | `select` only for a listed file other than the selected one |
| `step_comment(ordered, order, selected, last, dir) -> Option<CommentMove>` | — | `navigation::step` by id (FR-006); no comments → `None` |
| `Unreviewed` | `Select(RelPath)`, `AllReviewed`, `Nothing` | |
| `next_unreviewed(order, selected, is_reviewed: impl Fn(&RelPath) -> bool) -> Unreviewed` | — | FR-008, R5 |
| `hunk_change_range(&Hunk) -> Option<(Side, LineRange)>` | — | first..last added new line, else first..last removed old line |
| `CommentOpen` | `FocusComposer`, `Picked`, `Hunk { side: Side, range: LineRange }`, `Refused` | |
| `comment_target(composer_open, picked: bool, current: Option<&Hunk>) -> CommentOpen` | — | FR-009, in that order of precedence |

Notes "All files reviewed" and "Pick lines or move to a hunk (n) to comment" are `pub const` strings
here, so the view and tests share them.

## Core — `micold-core/src/review/diff.rs` (extended, M1)

`header_rows(shaped, layout) -> Vec<usize>`: the row index of each hunk header in the unified or
side-by-side rows, in hunk order (R9).

## Core — `micold-core/src/quick_switch.rs` (extended, M1) and `review/jump.rs` (new, M1)

| Change | Rules |
|---|---|
| `EntryKey::ReviewFile(RelPath)`, `EntryKey::ReviewComment(CommentId)` | keys of review entries (FR-016) |
| `Candidate.detail: Option<String>` | second text, matched only when `name` does not match; existing builders pass `None` |
| `Via { Name, Detail }`, `Ranked.via` | sort `(via, rank, index)`; spans index the matched text (R6, FR-017) |
| `jump::review_candidates(files: &[RelPath], is_reviewed, comments: &[ReviewComment]) -> Vec<(Candidate, ReviewEntry)>` | file: name = file name, detail = path; comment: name = text joined by spaces, detail = path; `ReviewEntry` carries what the row shows (reviewed state; status, path, line, first text line) |

## Client — Review tab view state (M2, M3; on #1269's per-entry view)

| Field | Type | Rules |
|---|---|---|
| `current_hunk` | `Option<usize>` | cleared whenever the selected file changes, by any means (one `select_file` path) |
| `pending_edge` | `Option<(RelPath, Edge)>` | set by `HunkMove::File` for that path; resolved with `resolve_edge` only on a `DiffRead` of the same path; cleared on another selection |
| `last_comment` | `Option<CommentId>` | the comment the last `]`/`[` or jump-to choice reached |
| `sheet_open` | `bool` (window-level surface state) | the `ShortcutSheet` registry probe reads it |
| `note` | existing notice slot | shows the two core notes |

Kept with the tab's view, so hidden tabs keep their position (#1269); keys act on the shown tab's
entry only (FR-014).

## Client — `keymap.rs` (M2)

`primary_held(Mods) -> bool` (Cmd on macOS, Ctrl elsewhere) and `review_press(&KeyInput, repeat) ->
Press`; no key is matched here, `resolve` does it from `REVIEW_KEYS`. `TextArea::on_press(..)` and the
Shift exclusion on its submit (R7).
