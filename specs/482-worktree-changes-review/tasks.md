---

description: "Task list for feature 482: review a worktree's changes and send comments to its session"
---

# Tasks: Review a Worktree's Changes and Send Comments to Its Session

**Input**: Design documents from `specs/482-worktree-changes-review/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/ (review-wire,
review-prompt, changes-view), quickstart.md

**Tests**: Mandatory (Constitution I). Every test task comes before the implementation it covers and
must fail first for the right reason (TDD loop, cycle log in `tdd/cycle-log.md`).

**Documentation**: `docs/user-guide/reviewing-changes.md` grows with each story, in the same
milestone as the behaviour it describes (Constitution VII, CI user-guide gate).

**Cross-platform**: core logic is platform-free (`RelPath` with `/`, `\n` only); the watcher uses
notify's recommended backend per OS; quickstart §C covers macOS and Windows (Constitution VI).

**Wire**: every wire-visible change of this feature is made in **one** edit (T008), so
`PROTOCOL_VERSION` moves 29 → 30 once; a second bump later in the feature fails
`crates/micold-core/tests/schema_hash.rs` (see the doc comment in
`crates/micold-core/src/protocol/version.rs`).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: US1–US4 from spec.md

---

## Phase 1: Setup (Shared Infrastructure)

- [ ] T001 Enable iced's `highlighter` feature in the workspace `iced` dependency in `Cargo.toml` (features list gains `"highlighter"`, R10) and add `notify = { workspace = true }` to `crates/micold-client/Cargo.toml` (R9); check `deny.toml` accepts the new licences (MIT, MIT/Apache-2.0) and that `cargo tree -p micold-client -i onig_sys` finds nothing (pure-Rust `fancy-regex` engine only)
- [ ] T002 Create the empty `crates/micold-core/src/review/` module tree (`mod.rs` declaring `base`, `changes`, `diff`, `comment`, `prompt`, `target`, `watch`, `git`, `store`; one empty file each) and `pub mod review;` in `crates/micold-core/src/lib.rs`; make `local_only` and `run_git` `pub(crate)` in `crates/micold-core/src/git.rs` (plan D1)

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: the domain value types every story uses, and the feature's whole wire delta in one
edit. No user-visible behaviour yet.

- [ ] T003 [P] Write failing unit tests in `crates/micold-core/src/review/mod.rs`: `RelPath::from_native` turns `\` separators into `/` and refuses absolute paths; a `RelPath` never holds `\` as a separator; `LineRange::new` refuses `start == 0` and `start > end`, accepts `start == end` (one line), `len()` is `end - start + 1`; `limits::{MAX_CHANGED_LINES == 5_000, MAX_VERSION_BYTES == 2 * 1024 * 1024, MAX_QUOTED_LINES == 50}`
- [ ] T004 Implement `RelPath` ("`String`, `/`-separated, relative to the entry root … never absolute; never contains `\` as a separator"), `Side { New, Old }`, `LineRange { start: NonZeroU32, end: NonZeroU32 }` ("`start <= end` enforced by `LineRange::new`") and `limits` in `crates/micold-core/src/review/mod.rs` until T003 passes
- [ ] T005 [P] Write failing serde round-trip tests in `crates/micold-core/src/review/comment.rs` for `ReviewComment` (`id`, `path`, `side`, `range`, `quote`, `text`, `state`, `created`), `CommentState::{Pending, Sent { at }}` and `CommentId` (UUID v4), matching the JSON shape in data-model.md § Core: persistence (`"side": "new"`, `"start"`, `"end"`, `"state": { "pending": null }`)
- [ ] T006 Implement the `ReviewComment`, `CommentState` and `CommentId` types (data only, no operations) in `crates/micold-core/src/review/comment.rs` until T005 passes
- [ ] T007 Write failing wire tests: in `crates/micold-core/tests/protocol_roundtrip.rs` round-trip `ClientMsg::ReviewEdit` with each `ReviewEditOp` (`Add`, `SetText`, `Delete`, `ClearSent`, `DiscardPending`), `ClientMsg::ReviewSend { outdated }`, `DaemonMsg::ReviewChanged { comments, sending }`, `OperationResult::ReviewSent { session, started }` and `SettingsSet`/`SettingsChanged` with `diff_layout`; in `crates/micold-core/tests/settings_roundtrip.rs` a settings file without `diff_layout` loads as `DiffLayout::Unified` (serde default, R12)
- [ ] T008 Make the feature's whole wire delta in one edit (contracts/review-wire.md): `ClientMsg::ReviewEdit`, `ClientMsg::ReviewSend`, `ReviewEditOp`, `DaemonMsg::ReviewChanged`, `OperationResult::ReviewSent`, and `diff_layout` on `SettingsSet` (`Option<DiffLayout>`), `SettingsChanged` and `DaemonSettings` in `crates/micold-core/src/protocol/messages.rs`; `DiffLayout { Unified, SideBySide }` and `Settings::diff_layout` (serde default `Unified`) in `crates/micold-core/src/settings.rs`; `PROTOCOL_VERSION` 29 → 30 with its "Bumped 29 → 30" doc line in `crates/micold-core/src/protocol/version.rs`; the new pin in `crates/micold-core/tests/schema_hash.rs`. T007 passes
- [ ] T009 [P] Write a failing daemon test `crates/micold-daemon/tests/diff_layout_setting.rs` modelled on `crates/micold-daemon/tests/pr_status_setting.rs`: `SettingsSet { diff_layout: Some(SideBySide) }` is stored in the settings file, reported in `Welcome` after a restart, and pushed by `SettingsChanged` to a second attached client; and `ReviewEdit` / `ReviewSend` are answered `OperationError { kind: Refused }` (not yet served)
- [ ] T010 Serve `diff_layout` in the daemon as `pr_status_enabled` is served (`set_diff_layout` in `crates/micold-daemon/src/catalog.rs`, `crates/micold-daemon/src/state.rs`, the `SettingsSet` arm and `DaemonSettings`/`SettingsChanged` fill in `crates/micold-daemon/src/server.rs`); route `ReviewEdit` and `ReviewSend` in `server.rs` to an `OperationError { kind: Refused, message: "review comments are not available in this build" }` placeholder that T057 (`ReviewEdit`) and T072 (`ReviewSend`) replace, each retiring its T009 assertion. T009 passes

**Checkpoint**: core value types and protocol v30 in place; the client sends no review message yet.

---

## Phase 3: User Story 1 — See what changed in a worktree (Priority: P1) 🎯 MVP

**Goal**: a Changes view per entry with the changed-file list, both toggles, unified and
side-by-side diffs with syntax colouring, binary and large files handled.

**Independent Test**: spec US1 Independent Test; quickstart B1–B6, B15.

### Part A — the changed-file list (scenarios 1, 2, 6, 9)

- [ ] T011 [P] [US1] Write failing unit tests in `crates/micold-core/src/review/base.rs`: `default_branch_from(origin_head, has_main, has_master)` prefers `origin/HEAD`'s target, then `main`, then `master`, else `None`; `DiffRange::for_view` for every `Toggles` × `ReviewScope`: committed only → `BaseToHead`, uncommitted only → `HeadToWorktree`, both → `BaseToWorktree`, `Base::Unavailable` → `HeadToWorktree`, `RootUncommitted` never gives a committed range, both off → `None`
- [ ] T012 [P] [US1] Write failing unit tests in `crates/micold-core/src/review/changes.rs` on captured git output: `parse_numstat` and `parse_name_status` with `-z` (renames with old path, mode-only from `--summary`, binary `-\t-`, paths with spaces and non-ASCII kept verbatim); `merge_untracked` adds `Untracked` entries; one `ChangedFile` per path, list sorted by path; `classify` marks `large` at 5,001 changed lines but not 5,000, and above `MAX_VERSION_BYTES` but not at it; `origin` is `Committed`/`Uncommitted`/`Both`
- [ ] T013 [P] [US1] Write failing real-git integration tests in `crates/micold-core/tests/review_git.rs` (new; temp repo + worktree, as `crates/micold-core/tests/git_containment.rs` builds them): committed-only, uncommitted-only and both lists (a file changed in both is listed once); staged, unstaged and untracked-not-ignored all count as uncommitted, ignored files do not; rename; binary; base from `origin/HEAD`, then `main`, then `master`; no common history → `Base::Unavailable(NoCommonHistory)`; no default branch → `NoDefaultBranch`; the Default entry (`RootUncommitted`) lists only the root's uncommitted changes
- [ ] T014 [P] [US1] Write failing client state tests in `crates/micold-client/tests/features_changes.rs` (new) for the list half of contracts/changes-view.md: opening an entry gives `Effect::ReadList`, both toggles on (V1, L1); a toggle change re-reads; an answer with a stale `seq` is dropped; a read requested while one is in flight runs once more after it (`Loading { again }`); the selected path is kept across a re-read while still listed; selecting a session closes the view (V2); the entry leaving the catalog closes it (V3); for the Default entry the committed toggle is unavailable with the L1 note; empty states "No changes against <base>", "Committed and uncommitted changes are both hidden" and the base-reason line (L3), chosen by a view-model function in `features::changes` so the glue only renders it
- [ ] T015 [P] [US1] Write failing geometry tests in `crates/micold-client/src/ui/material/virtual_rows.rs` (new): `visible_range(offset, viewport, row_height, len, overscan)` arithmetic at the top, middle, end and for `len == 0`; with 2,000 rows only the visible rows plus overscan are built; spacer heights above and below keep the total height `len * row_height` (L4, R11)
- [ ] T016 [US1] Implement `ReviewScope`, `Base`, `BaseUnavailable`, `Toggles` ("Default both on"), `DiffRange::for_view` and `default_branch_from` in `crates/micold-core/src/review/base.rs` until T011 passes
- [ ] T017 [US1] Implement `ChangeKind`, `Content`, `ChangedFile`, `Origin`, `ChangeList`, `parse_numstat`, `parse_name_status`, `merge_untracked` and `classify` in `crates/micold-core/src/review/changes.rs` until T012 passes
- [ ] T018 [US1] Implement `GitCli::review_base` and `GitCli::change_list(scope, toggles)` in `crates/micold-core/src/review/git.rs` with the R1 commands (all with `-c core.quotepath=false --no-pager --no-ext-diff --no-textconv --no-color`, through `no_window`/`local_only`/`run_git`), version sizes via `git cat-file -s` and file metadata for `classify`, until T013 passes
- [ ] T019 [US1] Implement `VirtualRows::new(len, row_height, build_row).on_scroll(..)` and pure `visible_range` in `crates/micold-client/src/ui/material/virtual_rows.rs` over the library `Scrollable`, registered in `crates/micold-client/src/ui/material/mod.rs`, until T015 passes
- [ ] T020 [US1] Implement the list half of `features::changes` (`State`, `OpenView { entry, toggles, list: Load<ChangeList>, selected }`, `Msg`, `Effect::ReadList { seq }`) in `crates/micold-client/src/features/changes.rs` (new), register it in `crates/micold-client/src/features/mod.rs`, and wire `Message::Changes` in `crates/micold-client/src/app.rs` (selecting a session and a catalog without the entry close the view) until T014 passes
- [ ] T021 [US1] Run `Effect::ReadList` in `tokio::task::spawn_blocking` via `GitCli::change_list`, answering `Msg::ListRead { seq, .. }`, in `crates/micold-client/src/shell/changes.rs` (new; registered in `crates/micold-client/src/shell/mod.rs`), as `crates/micold-client/src/shell/pr_status.rs` runs its reads (R2)
- [ ] T022 [US1] Add the **Review changes** message to `crates/micold-client/src/features/sidebar.rs` and the menu item to the worktree row's and the Default row's menus in `crates/micold-client/src/ui/sidebar.rs`, extending `crates/micold-client/tests/features_sidebar.rs` with a failing test first that the message opens the Changes view of that entry (V1)
- [ ] T023 [US1] Compose the view in `crates/micold-client/src/ui/changes.rs` (new): header with the entry, `Compared with <branch> at <short sha>` or the base reason, and a close action (V2); `LabelledToggle`s **Committed**/**Uncommitted** with the Default note (L1); the file list in `VirtualRows` with path, kind `Tag`, `+a −r` counts (L2); the empty state `features::changes` selects (L3); a "Select a file" placeholder in the diff pane. Show it in place of the terminal pane in `crates/micold-client/src/ui/mod.rs`
- [ ] T024 [P] [US1] Add a `VirtualRows` long-list pose (2,000 rows) to `crates/micold-client/src/showcase/sections/review.rs` (new; registered in `crates/micold-client/src/showcase/sections/mod.rs`) with its entry in `crates/micold-client/src/showcase/catalogue.rs`
- [ ] T025 [P] [US1] Write `docs/user-guide/reviewing-changes.md` (new): opening the Changes view from a worktree or the Default row, what the list shows, the base line, the two toggles and the Default entry's uncommitted-only list; link it from `docs/SUMMARY.md`, `docs/README.md` and `docs/user-guide/worktrees-and-sessions.md` § Managing a worktree
- [ ] T026 [US1] Visual pass (visual-pass skill) of quickstart B1, B2 and B15 (list part), light and dark; save the evidence under `specs/482-worktree-changes-review/visual-pass/`

### Part B — the unified diff (scenarios 3, 5, 7, 8)

- [ ] T027 [P] [US1] Write failing unit tests in `crates/micold-core/src/review/diff.rs`: `parse_unified` reads hunk headers and lines with both numbers; CRLF and LF are stripped from `text`; a line-ending-only change is a `Removed` + `Added` pair; `\ No newline at end of file` is dropped from the lines and recorded on the hunk; `Binary files … differ` gives `FileDiff::Binary`; invalid UTF-8 gives `NotUtf8`; `added_file(bytes)` synthesises an all-added diff, a NUL byte gives `Binary`; a mode-only change gives `ModeOnly`; `unified_rows` yields hunk headers and lines in order
- [ ] T028 [P] [US1] Extend `crates/micold-core/tests/review_git.rs` with failing `GitCli::file_diff` cases: committed, uncommitted and both ranges for one path (both = base → file on disk, US1 s3), rename with old path, deleted file all removed, untracked file all added, binary, LF → CRLF one line, a 6,000-line file reported `TooLarge` unless `force_large`
- [ ] T029 [P] [US1] Extend `crates/micold-client/tests/features_changes.rs` with failing tests: selecting a file gives `Effect::ReadDiff { seq, path, force_large: false }`; a stale diff answer is dropped; a `TooLarge` answer shows counts and **Show diff**, which gives `ReadDiff { force_large: true }` and adds the path to `shown_large` (US1 s7, D4); binary, not-UTF-8 and mode-only answers allow no pick (D3); a list re-read keeps the selection and re-reads its diff
- [ ] T030 [P] [US1] Write failing geometry tests in `crates/micold-client/src/ui/material/diff_view.rs` (new) for the unified layout: both line-number columns align across rows; added/removed/context rows take their tint roles; only visible rows are built for 50,000 lines (D5); binary, not-UTF-8, mode-only and large-gate states show their D3/D4 message and no gutter
- [ ] T031 [P] [US1] Write failing contrast checks in `crates/micold-client/src/ui/material/composition_contrast.rs`: text on `diff_added` and on `diff_removed` meets 4.5:1 in the light and the dark scheme; extend the token CSS test in `crates/micold-core/src/tokens/css.rs` to list both roles (US1 s8)
- [ ] T032 [US1] Implement `DiffLine`, `LineKind`, `Hunk`, `FileDiff`, `UnifiedRow`, `SideLines`, `parse_unified`, `added_file` and `unified_rows` in `crates/micold-core/src/review/diff.rs` until T027 passes
- [ ] T033 [US1] Implement `GitCli::file_diff(scope, toggles, path, force_large)` in `crates/micold-core/src/review/git.rs` (`git diff -M -U3 <from> [HEAD] -- <path>`, old path for a rename, untracked read from disk) returning `FileDiff` plus the loaded `SideLines`, until T028 passes
- [ ] T034 [US1] Add `diff_added` and `diff_removed` container tints per scheme to `Roles` in `crates/micold-core/src/tokens/mod.rs` and the role list in `crates/micold-core/src/tokens/css.rs`, until T031 passes
- [ ] T035 [US1] Implement `DiffView::new(rows, layout, roles)` for the unified layout with tinted rows, both number columns and the D3/D4 messages in `crates/micold-client/src/ui/material/diff_view.rs`, rendered through `VirtualRows`, until T030 passes
- [ ] T036 [US1] Add the diff half to `crates/micold-client/src/features/changes.rs` (`diff: Load<LoadedDiff>`, `shown_large`, `Msg::Select`, `Msg::ShowLarge`, `Effect::ReadDiff`) and run `ReadDiff` in `crates/micold-client/src/shell/changes.rs` off the UI thread, until T029 passes
- [ ] T037 [US1] Show the diff pane in `crates/micold-client/src/ui/changes.rs`: `DiffView` for text, the D3 messages, the large gate with **Show diff**, and `StageProgress::new("Loading diff…", r)` while loading (D4)
- [ ] T038 [P] [US1] Add `DiffView` poses (unified, binary message, large-file gate, both schemes) to `crates/micold-client/src/showcase/sections/review.rs` and `crates/micold-client/src/showcase/catalogue.rs`
- [ ] T039 [P] [US1] Extend `docs/user-guide/reviewing-changes.md`: reading a diff, binary and non-text files, large files and Show diff
- [ ] T040 [US1] Visual pass of quickstart B1, B4, B5, B6 and B15 (diff part), light and dark, with a 2,000-file worktree and a 50,000-line diff timed for frames over 100 ms (SC-003); evidence under `specs/482-worktree-changes-review/visual-pass/`

### Part C — side by side, syntax colouring, kept layout (scenario 4, 8)

- [ ] T041 [P] [US1] Write failing unit tests in `crates/micold-core/src/review/diff.rs` for `side_by_side_rows`: context lines on both sides; each run of removed lines paired index-by-index with the following run of added lines, the shorter side padded with `None`; the set of changed lines and their numbers equals `unified_rows`' (US1 s4)
- [ ] T042 [P] [US1] Write failing tests: in `crates/micold-client/tests/features_settings.rs`, choosing a layout sends `SettingsSet { diff_layout }` and `SettingsChanged` updates it (R12); in `crates/micold-client/tests/features_changes.rs`, a span set for a line longer than 2,000 bytes is capped at 2,000 bytes and an unknown extension yields no spans (R10, via the pure helper `features::changes::cap_spans`)
- [ ] T043 [P] [US1] Write failing geometry tests in `crates/micold-client/src/ui/material/diff_view.rs` for the side-by-side layout: left and right number columns align, padded cells keep row heights, spans are painted over the row tint; extend `crates/micold-client/src/ui/material/composition_contrast.rs` so every foreground colour of the `InspiredGitHub` (light) and `Base16Ocean` (dark) highlighter themes meets 4.5:1 over the surface, `diff_added` and `diff_removed` of its scheme (plan: Contrast gate; US1 s8)
- [ ] T044 [US1] Implement `SideRow`, `Cell` and `side_by_side_rows` in `crates/micold-core/src/review/diff.rs` until T041 passes
- [ ] T045 [US1] Highlight each side's lines with `iced::highlighter::Highlighter` by file extension (`InspiredGitHub` light, `Base16Ocean` dark) in the background task of `crates/micold-client/src/shell/changes.rs`, store `Spans` in `LoadedDiff` through `cap_spans`, and wire the layout choice to `SettingsSet` in `crates/micold-client/src/features/changes.rs` and `crates/micold-client/src/features/settings.rs`, until T042 passes
- [ ] T046 [US1] Add the side-by-side layout and span painting to `DiffView` (`.spans(..)`) in `crates/micold-client/src/ui/material/diff_view.rs` until T043 passes; add the **Unified** / **Side by side** `ToggleChip` pair to the diff pane in `crates/micold-client/src/ui/changes.rs` (D1)
- [ ] T047 [P] [US1] Add side-by-side and syntax-coloured `DiffView` poses, both schemes, to `crates/micold-client/src/showcase/sections/review.rs` and `crates/micold-client/src/showcase/catalogue.rs`
- [ ] T048 [P] [US1] Extend `docs/user-guide/reviewing-changes.md`: the two layouts, that the choice is kept, syntax colouring; mention the layout in `docs/user-guide/settings.md` if the setting is shown there
- [ ] T049 [US1] Visual pass of quickstart B3 and B18 (DiffView, VirtualRows), light and dark; evidence under `specs/482-worktree-changes-review/visual-pass/`

**Checkpoint**: US1 complete; the Changes view is a full diff viewer.

---

## Phase 4: User Story 2 — Comment on lines and send the comments to the session (Priority: P1)

**Goal**: comments on lines and ranges, shared by every window and kept across restarts; Send to
session delivers one prompt to the entry's running session.

**Independent Test**: spec US2 Independent Test; quickstart B7, B8, B10.

### Part A — writing comments (scenarios 1, 2, 7; FR-020 persistence)

- [ ] T050 [P] [US2] Write failing unit tests in `crates/micold-core/src/review/comment.rs` for `EntryReview::add` (refuses a range mixing sides, `quote.len() != range.len()`, empty trimmed text; stores text trimmed), `set_text` and `delete` (unknown id → `NotFound`; on a sent comment → `Refused`)
- [ ] T051 [P] [US2] Write failing unit tests in `crates/micold-core/src/review/store.rs`: `ReviewFile` JSON round-trip in the data-model shape (`"version": 1`, `entries` with `location` and `comments`); entries with no comments are not written; a missing file loads empty; an unparseable file is kept aside as `<name>.corrupt` and an empty review loads (store.rs pattern)
- [ ] T052 [P] [US2] Write a failing daemon integration test `crates/micold-daemon/tests/review_edit.rs` (new; harness as `crates/micold-daemon/tests/pr_status_setting.rs`): W1 refusals (`InvalidInput` for an absolute path, a `\`, `start == 0`, `start > end`, quote length mismatch, empty text); W2 an unknown `worktree_dir` is `NotFound` and nothing is stored; W3 `SetText`/`Delete` on another entry's comment or an unknown id is `NotFound`; W5 the file is written before `OperationOk`; `ReviewChanged` reaches a second attached client after each edit and on attach; after a daemon restart the comments and their states are back (US4 s4)
- [ ] T053 [P] [US2] Write failing client state tests in `crates/micold-client/tests/features_changes.rs`: a gutter click picks one line, shift-click on the same side extends to a contiguous range, a click on the other side starts a new pick, context lines count as `New` (C1); Add comment opens the composer and Save sends `ReviewEdit::Add` with the quote taken from the loaded lines (C2); Edit and Delete send `SetText`/`Delete`; the composer and its text survive a list or diff re-read (C4); a `ReviewChanged` push replaces only that entry's comments in `reviews`; binary files allow no pick; the view model gives each listed file its pending-comment count and puts comments whose anchor is not in the current diff in the "Not in the current diff" group (C3, L2)
- [ ] T054 [P] [US2] Write failing geometry tests for the new components: `crates/micold-client/src/ui/material/text_area.rs` (empty with placeholder, focused, multi-line growth), `crates/micold-client/src/ui/material/review_comment.rs` (pending with Edit/Delete, sent without them, outdated tag, in-a-send without them), and slot rows and the picked-range state in `crates/micold-client/src/ui/material/diff_view.rs` (slot rows keep their measured heights in `VirtualRows`)
- [ ] T055 [US2] Implement `EntryReview`, `ReviewError` and `add`/`set_text`/`delete` in `crates/micold-core/src/review/comment.rs` until T050 passes
- [ ] T056 [US2] Implement `ReviewFile` and its (de)serialisation in `crates/micold-core/src/review/store.rs`, and `JsonFileStore::load_reviews`/`save_reviews` (temp + rename) beside a private `reviews_dir()` next to `project_state_dir()` in `crates/micold-core/src/store.rs` (the methods live in `store.rs`, so `reviews_dir` stays private), until T051 passes
- [ ] T057 [US2] Implement `crates/micold-daemon/src/review.rs` (new): `Reviews` (`HashMap<PathBuf, HashMap<SessionLocation, EntryState>>`), load on catalog adoption, `apply_edit` (W1–W3, W5, edits persisted through the catalog before answering), and the `ReviewChanged` push to every client of the project after each edit and after `Attached` (`crates/micold-daemon/src/state.rs`); replace the `ReviewEdit` placeholder in `crates/micold-daemon/src/server.rs`; until T052 passes. Logs name the entry and count, never comment text (W12)
- [ ] T058 [US2] Implement `TextArea` (over iced `text_editor`, styled like `FilledField`, `.on_action(..).placeholder(..)`) in `crates/micold-client/src/ui/material/text_area.rs`, `ReviewCommentCard::new(text, state, roles).outdated(b).on_edit(..).on_delete(..)` in `crates/micold-client/src/ui/material/review_comment.rs`, and `.pick(..)`, `.on_gutter(..)`, `.slot(row, element)` on `DiffView`, registered in `crates/micold-client/src/ui/material/mod.rs`, until T054 passes
- [ ] T059 [US2] Add `pick`, `composer`, `reviews` and the pick/composer/edit messages to `crates/micold-client/src/features/changes.rs`, apply `DaemonMsg::ReviewChanged` in `crates/micold-client/src/app.rs`, until T053 passes
- [ ] T060 [US2] Wire commenting in `crates/micold-client/src/ui/changes.rs`: gutter picks, **Add comment**, the composer under the last picked row (Save, Ctrl/Cmd+Enter, Cancel), comment cards under their anchors, and the "Not in the current diff" group and list-row pending counts as `features::changes` derives them (C1–C3, L2)
- [ ] T061 [P] [US2] Add `TextArea` (empty, focused, multi-line) and `ReviewCommentCard` (pending, sent, outdated, in a send) poses and a `DiffView` picked-range pose to `crates/micold-client/src/showcase/sections/review.rs` and `crates/micold-client/src/showcase/catalogue.rs`
- [ ] T062 [P] [US2] Extend `docs/user-guide/reviewing-changes.md`: picking a line or range, writing, editing and deleting comments, that comments are kept across restarts and shown in every window
- [ ] T063 [US2] Visual pass of quickstart B7, B8 and B14, light and dark; evidence under `specs/482-worktree-changes-review/visual-pass/`

### Part B — sending to the running session (scenarios 3–6)

- [ ] T064 [P] [US2] Write failing unit tests in `crates/micold-core/src/review/prompt.rs` for P1–P10 of contracts/review-prompt.md (exact string for two files and three comments; removed-line wording with the base line number; 51-line range elided with `49 lines not shown`, 50 not elided; four-backtick fence around a quoted ```` ``` ````; CRLF in text → LF and no `\r` anywhere; ordering ties; Default entry wording; outdated suffix; 20 comments across 5 files each exactly once; paths with spaces and non-ASCII verbatim)
- [ ] T065 [P] [US2] Write failing unit tests: in `crates/micold-core/src/review/comment.rs`, `begin_send` snapshots exactly the pending comments (sent ones excluded, US2 s6) and errs `NothingPending` with none and `Busy` while a snapshot is open; snapshot comments refuse `set_text`/`delete` with `InSend`; `finish_send(&snapshot, at)` marks only the snapshot ids `Sent { at }` and leaves comments added meanwhile pending; `abort_send` leaves all pending; in `crates/micold-core/src/review/target.rs`, `pick_target` returns the largest `Uptime`, breaks ties by the larger `SessionId`, and `None` for no candidates
- [ ] T066 [P] [US2] Write a failing daemon integration test `crates/micold-daemon/tests/review_send.rs` (new; fake CLI as `crates/micold-daemon/tests/mcp_create_session.rs` installs it): with one running session in the worktree, `ReviewSend` types exactly one prompt equal to `prompt::build`'s and answers `ReviewSent { started: false }`; the comments become sent and `ReviewChanged` shows `sending: true` then `false`; a second send carries only the new pending comment; `ReviewSend` with no pending comment is `InvalidInput`; a terminal without bracketed paste is refused and the comments stay pending (W9); a prompt never reaches another entry's running session (FR-021); with two running sessions in the entry the one whose `last_active` is later receives it, where `last_active` is moved by start, by `SessionInput` and by an activity change (US3 s4, R5)
- [ ] T067 [P] [US2] Write failing client state tests in `crates/micold-client/tests/features_changes.rs`: **Send to session (n)** is enabled iff `n ≥ 1` pending and `sending` is false, reads "Sending…" while sending (S1); pressing it sends `ReviewSend` with an empty `outdated` list (T083/T087 fill it in M7); `OperationOk(ReviewSent)` gives the S2 success snackbar text, `OperationError` the error snackbar
- [ ] T068 [US2] Implement `prompt::build(entry_label, comments, outdated)` in `crates/micold-core/src/review/prompt.rs` until T064 passes
- [ ] T069 [US2] Implement `SendSnapshot`, `begin_send`, `finish_send` and `abort_send` in `crates/micold-core/src/review/comment.rs` and `pick_target` in `crates/micold-core/src/review/target.rs` until T065 passes
- [ ] T070 [US2] Move the delivery steps out of `crates/micold-daemon/src/mcp/tools.rs` into `ops::type_submission(state, session, text) -> Result<(), Undelivered>` in `crates/micold-daemon/src/ops.rs` (trust check, bracketed-paste check, `encode_submission`, `write_input`); `mcp/tools.rs` calls it with unchanged behaviour (`crates/micold-daemon/tests/mcp_create_session.rs` and `mcp_cross_session.rs` stay green)
- [ ] T071 [US2] Add per-session `last_active: Uptime` (set on start, on `SessionInput`, on activity change) and `running_sessions_in(project, location)` in `crates/micold-daemon/src/state.rs`
- [ ] T072 [US2] Implement `review::send` for a running target in `crates/micold-daemon/src/review.rs` (W6: snapshot, prompt, `sending`, push; W7 running part: `pick_target` over `running_sessions_in`; W8; W9 for an undelivered running target) and replace the `ReviewSend` placeholder in `crates/micold-daemon/src/server.rs`; with no running session it answers `Refused` until T077; until T066 passes. The prompt text is never logged (W12)
- [ ] T073 [US2] Add the send state and messages to `crates/micold-client/src/features/changes.rs` and the toolbar **Send to session (n)** button and snackbars to `crates/micold-client/src/ui/changes.rs` (S1, S2), until T067 passes
- [ ] T074 [P] [US2] Extend `docs/user-guide/reviewing-changes.md`: Send to session, what the prompt holds, which session receives it, sent comments
- [ ] T075 [US2] Visual pass of quickstart B9 with a session already running in `wt`, and B10; evidence under `specs/482-worktree-changes-review/visual-pass/`

**Checkpoint**: the review loop works end to end when the entry's session runs.

---

## Phase 5: User Story 3 — Send when no session is running (Priority: P2)

**Goal**: Send starts a session in the entry with the default AI CLI and types the prompt as its
first input; failures keep comments pending; concurrent sends deliver once.

**Independent Test**: spec US3 Independent Test; quickstart B9, B11, B16.

- [ ] T076 [P] [US3] Extend `crates/micold-daemon/tests/review_send.rs` with failing cases: no running session → a session of `default_ai_cli` starts in that entry, its first input is the prompt, `ReviewSent { started: true }` (US3 s1); the CLI fails to start or is not ready within `first_prompt_bound` → `OperationError`, comments pending, `sending` cleared (US3 s2, W9); two clients send at once → one `Busy`, the prompt typed once (US3 s3, FR-018); an ended session in the entry and none running → a new session starts and the ended one is not resumed (US3 s4); the Default entry starts its session in the project root
- [ ] T077 [US3] Move `deliver_first_prompt` out of `crates/micold-daemon/src/mcp/tools.rs` into `ops::create_session_with_prompt(...)` in `crates/micold-daemon/src/ops.rs` (create with `state.default_ai_cli()`, `ops::start_session` with `LaunchMode::Fresh`, `wait_ready_for_input` bounded by `first_prompt_bound`, then `type_submission`); `mcp/tools.rs` calls it unchanged in behaviour; `review::send` in `crates/micold-daemon/src/review.rs` uses it when `pick_target` gives `None` (W7, W9: a session the send started stays); until T076 passes
- [ ] T078 [P] [US3] Extend `crates/micold-client/tests/features_changes.rs` with a failing test that `ReviewSent { started: true }` gives "Started a session and sent n comments", and implement it in `crates/micold-client/src/features/changes.rs` (S2)
- [ ] T079 [P] [US3] Extend `docs/user-guide/reviewing-changes.md`: sending when no session runs, what happens when the session cannot start, sending from two windows
- [ ] T080 [US3] Visual pass of quickstart B9 (no session running), B11 and B16; evidence under `specs/482-worktree-changes-review/visual-pass/`

**Checkpoint**: Send to session works whether or not a session runs.

---

## Phase 6: User Story 4 — Keep the view current and the comment list tidy (Priority: P3)

**Goal**: the view refreshes itself within 2 s, comments on changed lines are marked outdated, sent
comments can be cleared, pending ones discarded, and a removed worktree takes its comments with it.

**Independent Test**: spec US4 Independent Test; quickstart B12, B13, B17.

### Part A — refresh and outdated comments (scenarios 1, 2)

- [ ] T081 [P] [US4] Write failing unit tests in `crates/micold-core/src/review/watch.rs` for `relevant_paths(entry_root, git_dirs, paths)`: drops `.git/objects`, `.git/logs` and lock files; keeps `HEAD`, `index`, `refs/`, `packed-refs` and worktree files; for the Default entry drops everything under `.claude/worktrees/`; the pure `watch::Debouncer` (`push(paths, now)`, `ready(now) -> Option<Vec<PathBuf>>`) releases a batch only after 300 ms without a new event and merges repeated paths
- [ ] T082 [P] [US4] Write failing unit tests in `crates/micold-core/src/review/comment.rs` for `ReviewComment::is_outdated(lines: Option<&SideLines>)`: false when the lines at the range on its side equal the quote; true when they differ, when the range runs past the end, and for `None` (file gone)
- [ ] T083 [P] [US4] Extend `crates/micold-client/tests/features_changes.rs` with failing tests: `Msg::Changed` re-reads the list and the open diff; a change arriving during a read runs one more read after it (coalesced); the pick is dropped when its lines are gone, the composer is kept (R2, C4); comments whose quote no longer matches show Outdated and their ids go in `ReviewSend.outdated`
- [ ] T084 [US4] Implement `relevant_paths` and `Debouncer` in `crates/micold-core/src/review/watch.rs` and `is_outdated` in `crates/micold-core/src/review/comment.rs` until T081 and T082 pass
- [ ] T085 [US4] Implement `GitCli::ignored` (`git check-ignore -z --stdin`) in `crates/micold-core/src/review/git.rs` with a failing real-git case in `crates/micold-core/tests/review_git.rs` first (an ignored path is filtered, a tracked one kept)
- [ ] T086 [US4] Implement the watch subscription in `crates/micold-client/src/shell/changes_watch.rs` (new): keyed by entry root, `notify::RecommendedWatcher` over the entry directory and its git metadata (own git dir `HEAD`, `index`; common dir `refs/`, `packed-refs`), debounced by `watch::Debouncer`, filtered by `relevant_paths` and `GitCli::ignored`, emitting `Msg::Changed` (the subscription holds no decision of its own); registered in `crates/micold-client/src/shell/subscriptions.rs` only while the view is open (R9)
- [ ] T087 [US4] Add refresh handling and outdated derivation to `crates/micold-client/src/features/changes.rs` and the Outdated tag to the cards in `crates/micold-client/src/ui/changes.rs`, until T083 passes
- [ ] T088 [P] [US4] Extend `docs/user-guide/reviewing-changes.md`: automatic refresh and outdated comments
- [ ] T089 [US4] Visual pass of quickstart B12 on Linux, timing 20 file edits from a terminal and recording how many showed within 2 s (SC-004: at least 19); evidence under `specs/482-worktree-changes-review/visual-pass/`

### Part B — tidying and removal (scenarios 3, 5)

- [ ] T090 [P] [US4] Write failing unit tests in `crates/micold-core/src/review/comment.rs`: `clear_sent` removes sent comments only; `discard_pending` removes pending comments not inside an open send (W4)
- [ ] T091 [P] [US4] Extend `crates/micold-daemon/tests/review_edit.rs` with failing cases: `ClearSent` and `DiscardPending` per W4, persisted and pushed; deleting the worktree through `WorktreeDelete` removes its comments from memory and the file and pushes an empty `ReviewChanged` (W11); a worktree removed outside the app is pruned the same way when a catalog refresh no longer lists it; removing the project from the catalog deletes its `reviews/` file
- [ ] T092 [P] [US4] Extend `crates/micold-client/tests/features_changes.rs` with failing tests: **Clear sent** is enabled iff a sent comment exists and sends `ClearSent`; **Discard pending…** opens the confirmation and only its confirm sends `DiscardPending` (S3)
- [ ] T093 [US4] Implement `clear_sent` and `discard_pending` in `crates/micold-core/src/review/comment.rs` until T090 passes
- [ ] T094 [US4] Serve `ClearSent`/`DiscardPending` in `crates/micold-daemon/src/review.rs`, call `review::forget_worktree` from `ops::delete_worktree` on success in `crates/micold-daemon/src/ops.rs`, prune a worktree that disappears from the catalog on refresh, and `review::forget_project` on project removal, until T091 passes
- [ ] T095 [US4] Add **Clear sent** and **Discard pending…** with its `Modal` built with `dialog::body` as `crates/micold-client/src/ui/confirm_delete.rs` does ("Discard n pending comments? This cannot be undone.") to `crates/micold-client/src/features/changes.rs` and `crates/micold-client/src/ui/changes.rs`, until T092 passes
- [ ] T096 [P] [US4] Extend `docs/user-guide/reviewing-changes.md`: clearing sent comments, discarding pending ones, what happens to comments when a worktree is removed
- [ ] T097 [US4] Visual pass of quickstart B13 and B17; evidence under `specs/482-worktree-changes-review/visual-pass/`

**Checkpoint**: all user stories complete.

---

## Phase 7: Polish & Cross-Cutting Concerns

No code. Done by the close unit in the close PR.

- [ ] T098 Run quickstart §C on macOS and Windows (B1, B3, B7, B9, B12, B6 with `core.autocrlf=true`) and compare B9's prompt bytes with Linux's; record the results in `specs/482-worktree-changes-review/quickstart.md`
- [ ] T099 Run the full quickstart §B (B1–B18) once on the merged result and record it in `specs/482-worktree-changes-review/quickstart.md`
- [ ] T100 Review `docs/user-guide/reviewing-changes.md` end to end against FR-023 and fix wording across sections

---

## Dependencies & Execution Order

### Phase dependencies

- Setup (T001–T002) → Foundational (T003–T010) → stories.
- US1 parts run in order A → B → C (B needs A's view and list; C needs B's `DiffView`).
- US2 needs US1 Part B (comments anchor to diff rows); its Part B needs its Part A.
- US3 needs US2 Part B (it extends `review::send`).
- US4 Part A needs US2 Part A (outdated comments) and US1 Part B; US4 Part B needs US2 Part B
  (sent comments) and US2 Part A.
- Polish after all stories.

### Within each part

Tests first and failing (Constitution I), then core, then daemon, then client state, then shell
and glue; showcase and user guide in the same part; the visual pass last.

### Parallel opportunities

Tasks marked [P] in one part touch disjoint files: e.g. in US1 Part A, T011–T015 (five test files)
together; in US2 Part A, T050–T054 together; showcase and user-guide tasks alongside the glue.

## Parallel Example: User Story 1, Part A

```text
Task: "T011 base.rs unit tests"
Task: "T012 changes.rs unit tests"
Task: "T013 review_git.rs real-git tests"
Task: "T014 features_changes.rs list tests"
Task: "T015 virtual_rows.rs geometry tests"
```

## Implementation Strategy

MVP first: Setup, Foundational and US1 Part A give a Changes view with the file list; each later
part adds an observable step (diff, layouts and colour, comments, sending, starting a session,
refresh, tidying). Each part is one milestone below and merges on its own.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Changes view with the changed-file list 🎯 MVP

- **Tasks**: T001–T026
- **Deliverable**: right-click a worktree or the Default row → **Review changes** opens a Changes view listing the changed files with kind and `+a −r`, the base line, and working Committed/Uncommitted toggles (Default: uncommitted only)
- **Satisfies**: US1 acceptance scenarios 1, 2, 6, 9; FR-001, FR-002, FR-003, FR-004, FR-005, FR-022 (list part); protocol v30 and `diff_layout` stored by the daemon
- **Verify**: `cargo test -p micold-core --all-targets review`, `cargo test -p micold-core --test protocol_roundtrip --test schema_hash`, `cargo test -p micold-daemon --test diff_layout_setting` and `cargo test -p micold-client --test features_changes`; quickstart B1, B2, B15 (list part)
- **Depends on**: —
- **Tier**: full

### M2 — Unified diff of the selected file

- **Tasks**: T027–T040
- **Deliverable**: selecting a file shows its unified diff with both line numbers and tinted added/removed rows; binary, non-UTF-8 and mode-only files show a message; a large file shows counts and Show diff; the window stays responsive
- **Satisfies**: US1 acceptance scenarios 3, 5, 7, 8 (tints); FR-006 (unified), FR-008, FR-009, FR-022 (diff part); SC-005; Edge Cases large diff, binary, CRLF
- **Verify**: `cargo test -p micold-core --test review_git` and `cargo test -p micold-client --test features_changes`; quickstart B4, B5, B6
- **Depends on**: M1
- **Tier**: full

### M3 — Side-by-side layout, syntax colouring, kept layout

- **Tasks**: T041–T049
- **Deliverable**: the diff pane switches between Unified and Side by side, colours recognised languages, and keeps the chosen layout for the next file and after a restart
- **Satisfies**: US1 acceptance scenarios 4, 8; FR-006, FR-007
- **Verify**: `cargo test -p micold-client --test features_settings` and the `diff_view` geometry tests; quickstart B3, B18
- **Depends on**: M2
- **Tier**: full

### M4 — Comments on lines and ranges

- **Tasks**: T050–T063
- **Deliverable**: picking a line or range and writing a comment shows a pending card under it; comments can be edited and deleted, appear in every window of the project, and survive a restart
- **Satisfies**: US2 acceptance scenarios 1, 2, 7; US4 acceptance scenario 4; FR-011, FR-012, FR-020 (persistence), FR-021 (edits)
- **Verify**: `cargo test -p micold-daemon --test review_edit` and `cargo test -p micold-client --test features_changes`; quickstart B7, B8, B14
- **Depends on**: M2
- **Tier**: full

### M5 — Send comments to the running session

- **Tasks**: T064–T075
- **Deliverable**: Send to session types one prompt holding every pending comment (file, lines, quote, text) into the entry's most recently active running session and marks them sent
- **Satisfies**: US2 acceptance scenarios 3, 4, 5, 6; FR-014, FR-015, FR-017, FR-018, FR-021; SC-001, SC-002, SC-006
- **Verify**: `cargo test -p micold-core --all-targets review::prompt` and `cargo test -p micold-daemon --test review_send`; quickstart B9 (session running), B10
- **Depends on**: M4
- **Tier**: full

### M6 — Send when no session is running

- **Tasks**: T076–T080
- **Deliverable**: with no session running in the entry, Send to session starts one with the default AI CLI and delivers the prompt as its first input; a failed start keeps the comments pending with an error
- **Satisfies**: US3 acceptance scenarios 1–4; FR-016, FR-017, FR-018
- **Verify**: `cargo test -p micold-daemon --test review_send`; quickstart B9 (no session), B11, B16
- **Depends on**: M5
- **Tier**: full

### M7 — Live refresh and outdated comments

- **Tasks**: T081–T089
- **Deliverable**: with the view open, editing, creating, deleting or committing a file updates the list and the open diff within 2 s; comments on changed lines show Outdated and are sent with their recorded quote
- **Satisfies**: US4 acceptance scenarios 1, 2; FR-010, FR-013; SC-004
- **Verify**: `cargo test -p micold-core --all-targets review::watch` and `cargo test -p micold-client --test features_changes`; quickstart B12
- **Depends on**: M5
- **Tier**: full

### M8 — Clear, discard, and removal with the worktree

- **Tasks**: T090–T097
- **Deliverable**: Clear sent removes sent comments, Discard pending… removes pending ones after confirmation, and deleting a worktree removes its comments everywhere
- **Satisfies**: US4 acceptance scenarios 3, 5; FR-019, FR-020 (removal); Edge Case removed worktree
- **Verify**: `cargo test -p micold-daemon --test review_edit`; quickstart B13, B17
- **Depends on**: M5
- **Tier**: full

Polish (T098–T100) changes no code: the close unit does it in the close PR.
