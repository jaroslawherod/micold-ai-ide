---
description: "Task list for #1271 Review UI: reviewed marks and progress"
---

# Tasks: Review UI — reviewed marks and progress

**Input**: Design documents from `specs/1271-review-reviewed-marks/` (plan.md, spec.md, research.md,
data-model.md, contracts/marks-wire.md, quickstart.md)

**Tests**: Constitution Principle I — every behaviour has a failing test task before its
implementation task. Pure decisions live in `micold-core::review::marks` (FR-014).

**Documentation**: Constitution Principle VII — the user-guide tasks travel with the stories that
change what the user sees (T034 in US1, T038 in US2, T044 in US4).

**Cross-platform**: paths are `RelPath` (`/`); the change id record is bytes with no platform text
(research R1); git runs as argument vectors. CI runs Linux, macOS and Windows.

**D4 (provisional A)**: T025, T026, the s7 cases of T027 and the `HiddenKind` arm used by T028 implement answer A. If the user
picks B, drop the `HiddenKind` arm, `HIDDEN_KIND_TOOLTIP` and their tests (research R7); nothing
else moves. Answer C changes the data model and must be settled before M1.

**#1269**: Phases 1–3 (M1, M2) build on today's `main`. Phases 4–7 (M3–M5) change the review view
that #1269 rewrites and start only after #1269 merges (research R8).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: the user story a task belongs to (US1–US4)

## Path Conventions

Rust workspace: `crates/micold-core`, `crates/micold-daemon`, `crates/micold-client`,
`crates/micold-material`; docs under `docs/`.

---

## Phase 1: Setup

- [x] T001 Add `sha1 = "0.10"` as a direct dependency of `crates/micold-core/Cargo.toml` (already in `Cargo.lock` as 0.10.7, so `Cargo.lock` only gains the edge), and create the module `crates/micold-core/src/review/marks.rs` (module doc naming #1271 and research R1–R7) registered as `pub mod marks;` in `crates/micold-core/src/review/mod.rs`

---

## Phase 2: Foundational A — marks in `micold-core` (M1)

**Purpose**: change identity, the whole-change read, the per-entry mark set, the state and
progress derivations, and their storage. Every story builds on these.

- [x] T002 [P] Write failing unit tests for `ChangeId` in `crates/micold-core/src/review/marks.rs`: `of_worktree` over equal `WholeFile`s gives equal ids; changing any one of kind, path, rename source path, old mode, new mode, old id or new id changes the id; `of_patch` over the same `ChangedFile` and patch gives the same id and any patch byte change gives another; a working-tree id and a patch id over the same path and kind differ (source tags `w` / `p`); `parse` accepts exactly 40 hex characters and stores them lower-cased, refuses 39, 41 and non-hex; serde writes and reads the 40-hex string in JSON and in postcard (one shape, `#[serde(try_from = "String", into = "String")]`), and a non-hex string fails to decode
- [x] T003 Implement `ChangeId` (20 bytes; `of_worktree`, `of_patch`, `parse`, `Display` as 40 lowercase hex, serde via `String`), `WholeFile { path: RelPath, kind: ChangeKind, origin: Origin, id: Option<ChangeId> }` and `ReviewMark { scope: Scope, path: RelPath, change: ChangeId }` in `crates/micold-core/src/review/marks.rs`, with the record of data-model.md § `ChangeId`: source tag, kind tag (`A` added, `M` modified, `D` deleted, `R` + old path renamed, `T` mode only), path, old mode, old id, new mode, new id, fields separated by `\0`; digest with `sha1::Sha1`
- [x] T004 Write failing unit tests for `EntryMarks` in `crates/micold-core/src/review/marks.rs`: `set(scope, path, Some(id))` inserts and answers `true`, again with the same id answers `false`, with another id replaces and answers `true`; `set(.., None)` removes and answers `true`, with no mark answers `false`; `get`; `WorkingTree` and `PullRequest(12)` marks of the same path are kept apart and `of_scope` returns only the asked scope's; `to_vec` orders by scope, then path; `from_vec` with a duplicate key keeps the last; `is_empty`
- [x] T005 Implement `EntryMarks` (`BTreeMap<(Scope, RelPath), ChangeId>`; `set`, `get`, `of_scope`, `to_vec`, `from_vec`, `is_empty`) in `crates/micold-core/src/review/marks.rs`; no other operation removes a mark (FR-016)
- [x] T006 [P] Write failing unit tests in `crates/micold-core/src/review/changes.rs` that `parse_name_status` fills `old_mode`, `new_mode`, `old_id`, `new_id` exactly as git printed them for an `M`, `A` (old side all zeros), `D` (new side all zeros), `R100` and `T` record of `git diff --raw -z --no-abbrev`; the existing `parse_name_status` tests stay unchanged
- [x] T007 Add `old_mode`, `new_mode`, `old_id`, `new_id: String` to `NameStatus` and fill them in `parse_name_status` in `crates/micold-core/src/review/changes.rs` (no second parser); existing callers ignore them
- [x] T008 Write failing unit tests for `WholeChange::assemble(names, committed, uncommitted, hashed, untracked)` in `crates/micold-core/src/review/marks.rs`: origin Committed, Uncommitted or Both from the two path sets; every file Uncommitted when no committed set is given (the Default entry, no base); a new id of all zeros is replaced from `hashed`; a path missing from `hashed` gets `id: None`; an untracked path becomes kind added with old mode and id all zeros, new mode from disk and new id from `hashed`, and has the same id as git's added record of the same bytes (research R1); one `WholeFile` per path, keyed by the path now (new path of a rename)
- [x] T009 Implement `WholeChange { files: BTreeMap<RelPath, WholeFile> }` and the pure `WholeChange::assemble` in `crates/micold-core/src/review/marks.rs`
- [x] T010 [P] Write failing G2 tests in `crates/micold-core/tests/review_git.rs` against real temp repos (contracts/marks-wire.md § G2): committed, uncommitted and both origins; untracked file; rename (new path, old path in the id); mode-only change; binary file; the Default entry and a worktree without a base (all Uncommitted, read over `HEAD`); an unaltered commit of an uncommitted change in an entry with a base keeps the id (US2 s5); untracked → staged → committed keeps a new file's id; a reverted and redone change gets the same id (US2 s3, s6); the read is the same whatever `Toggles` the list read uses (FR-005)
- [x] T011 Implement `GitCli::whole_change(dir, &ReviewScope) -> io::Result<WholeChange>` in `crates/micold-core/src/review/git.rs` by G2 steps 1–3 (`git diff --no-ext-diff --no-textconv --no-color -z -M --raw --no-abbrev <rev>` over the merge-base or `HEAD`; `range_paths` for origins; `untracked` plus one `git hash-object --stdin-paths` over zero new ids and untracked paths, skipping a path holding `\n`; the review reads' deadline; a step-3 failure leaves `id: None`), and add `Git::whole_change` with its `GitCli` delegation and a configurable `FakeGit` answer in `crates/micold-core/src/git.rs`
- [x] T012 Write failing unit tests for `file_review(mark, current, drafts)` and `progress(files)` in `crates/micold-core/src/review/marks.rs`: every row of the data-model.md `file_review` table (Reviewed wins over drafts; a differing current id gives Unseen / HasDrafts with `changed_since`; no current id gives no `changed_since`; no mark gives Unseen / HasDrafts); `progress` of no files is `None`, of none, some and all Reviewed gives `Ratio { done, total }` with `total` the files passed
- [x] T013 Implement `ReviewState { Unseen, HasDrafts, Reviewed }`, `FileReview { state, changed_since }`, `file_review` and `progress` (returning `task_progress::Ratio`) in `crates/micold-core/src/review/marks.rs`
- [x] T014 [P] Write failing unit tests in `crates/micold-core/src/review/comment.rs`: `EntryReview::is_empty` is false with marks and no comments; `from_parts(comments, marks)` keeps both; `set_reviewed` delegates to `EntryMarks::set` and answers its bool; `clear_sent`, `discard_pending`, `begin_send`, `finish_send`, `abort_send`, `report` and `mark` (Mark applied) leave `marks` unchanged (FR-007, RM5)
- [x] T015 Add `marks: EntryMarks`, `from_parts` and `set_reviewed(scope, path, change) -> bool` to `EntryReview` and make `is_empty` require no comments and no marks in `crates/micold-core/src/review/comment.rs`; `from_comments` stays (no marks)
- [x] T016 [P] Write failing S4 tests in `crates/micold-core/src/review/store.rs` (contracts/marks-wire.md § S4): version 4 round-trip with working-tree and pull request marks; an entry with marks and no comments is written with `"comments": []`; no marks writes 3 (some PR comment) or 2; versions 1–3 load with no marks and every comment; a `change` that is not 40 hex characters and a `PullRequest(0)` mark set the file aside; the "unknown version" tests move from 4 to 5
- [x] T017 Implement review file version 4 in `crates/micold-core/src/review/store.rs`: `REVIEW_FILE_VERSION` 4 written only while some entry has marks, `READABLE_VERSIONS` 1–4, `StoredEntry.marks` (`#[serde(default, skip_serializing_if = "Vec::is_empty")]`), `ReviewFile.marks: BTreeMap<String, Vec<ReviewMark>>` keyed like `entries`, and `to_json` writing an entry with marks only

**Checkpoint**: `mise run test-core` passes; nothing outside `micold-core` uses marks yet.

---

## Phase 3: Foundational B — wire, session service, client receipt (M2)

**Purpose**: marks are set, kept, pushed and removed by the session service, and every window
holds the entry's marks. No view change.

- [ ] T018 [P] Write failing wire tests in `crates/micold-core/tests/protocol_roundtrip.rs`: `ReviewEditOp::SetReviewed` with `change: Some(id)` and `None` round-trips; `DaemonMsg::ReviewChanged` with `marks` of both scopes round-trips; a `SetReviewed` whose `change` is not 40 hex characters fails to decode; move the `SCHEMA_HASH` pin in `crates/micold-core/tests/schema_hash.rs` with the bump
- [ ] T019 Add `ReviewEditOp::SetReviewed { scope: Scope, path: String, change: Option<ChangeId> }` and `marks: Vec<ReviewMark>` on `DaemonMsg::ReviewChanged` in `crates/micold-core/src/protocol/messages.rs`, and bump `PROTOCOL_VERSION` in `crates/micold-core/src/protocol/version.rs` to the next free number when this milestone merges (68 → 69 at planning time; one bump for the feature, RM7)
- [ ] T020 [P] Write failing daemon tests in `crates/micold-daemon/tests/review_edit.rs` for RM1–RM4 and RM6: set, re-mark and unmark answer `Ack` and push `marks` (all sources, ordered by scope then path); refusals `InvalidInput` with memory and file unchanged for an unknown entry, an invalid path, `PullRequest(0)` and `PullRequest(_)` on the Default entry; a repeat writes and pushes nothing; accepted while a send of the entry is open; two attached clients both receive the push and two edits of the same file end equal in both; the attach push includes an entry with marks only; comments and `sending` in the push are unchanged by `SetReviewed`
- [ ] T021 Write failing daemon tests for RM5 in `crates/micold-daemon/tests/review_edit.rs` and `crates/micold-daemon/tests/review_send.rs`: Clear sent, Discard drafts, a send and a report leave marks; `forget_worktree`, `prune_worktrees` and `forget_project` remove them with the comments and push both lists empty
- [ ] T022 Implement `SetReviewed` in `apply_edit` (RM1 checks with 482's `checked_path`, RM2 no-change answer), `marks` in every `ReviewChanged` built there and in `pushes_on_attach`, `forget_worktree` and `prune_worktrees`, and loading and saving marks through `ReviewFile.marks` in `crates/micold-daemon/src/review.rs`; pass the new field where `crates/micold-daemon/src/server.rs` and `crates/micold-daemon/src/state.rs` build or forward `ReviewChanged`
- [ ] T023 [P] Write failing client reducer tests in `crates/micold-client/tests/features_changes.rs`: `Msg::ReviewChanged { marks, .. }` lands in `State.reviews[(project, worktree_dir)].marks`; marks of one entry never show for another entry; a later push replaces the marks (FR-006, FR-011)
- [ ] T024 Add `marks: Vec<ReviewMark>` to `Msg::ReviewChanged` and `marks: EntryMarks` to `ReviewView` in `crates/micold-client/src/features/changes.rs` (only the `marks` binding and the field in the `ReviewView` insert after the `state.open` block, research R8), and pass `DaemonMsg::ReviewChanged.marks` into it in `crates/micold-client/src/shell/daemon_sync.rs`

**Checkpoint**: `mise run gate` passes; two windows hold the same marks; no view change (the PR
carries `docs-not-needed`).

---

## Phase 4: User Story 1 — Mark a file reviewed and see my progress (Priority: P1) 🎯 MVP (M3)

**Goal**: every row shows ○ / ● / ✓, the diff header has the mark control, the header shows
*n of m reviewed* with a bar.

**Independent Test**: three changed files, mark one: ✓ on its row, *1 of 3 reviewed*, bar one
third; unmark returns both (spec US1).

- [ ] T025 [P] [US1] Write failing unit tests for `mark_block(origin, toggles, scope)` in `crates/micold-core/src/review/marks.rs` as one test group (research R7): `None` origin answers `NotRead`; Committed hidden with origin Committed or Both and Uncommitted hidden with origin Uncommitted or Both answer `HiddenKind`; a file whose changes are all of a shown kind answers `None`; the Default entry and a worktree without a base never answer `HiddenKind`
- [ ] T026 [US1] Implement `MarkBlock { Disconnected, NotRead, HiddenKind }` with one tooltip constant each (`HIDDEN_KIND_TOOLTIP` = "Turn on Committed and Uncommitted to see this file's whole change", the disconnected one "The session service is not connected") and `mark_block` in `crates/micold-core/src/review/marks.rs`
- [ ] T027 [US1] Write failing reducer tests in `crates/micold-client/tests/features_changes.rs` for US1 s1–s7: rows from marks, drafts and the whole change (○ for none, ● with a Draft, ✓ when the mark equals the current id, ✓ keeps the draft count, FR-001); `MarkToggled` emits `Effect::ReviewEdit` with `SetReviewed { change: Some(current id) }`, and `None` when Reviewed, with no state change before `ReviewChanged` arrives; `mark_control` label **Mark reviewed** / **Reviewed**, pressed, `HiddenKind` with Uncommitted off for a Both file and none for a Committed-only file (s7), `NotRead` before the whole change is read, `Disconnected` when not connected (unmark never blocked); `header_progress` *0 of 3*, *1 of 3*, *3 of 3*, `m` following the toggles, `None` for no files and while disconnected; `Msg::ListRead.whole` loads with the list, is kept during a re-read (`Loading { last }`), and a failed whole-change read keeps the last one
- [ ] T028 [US1] Implement in `crates/micold-client/src/features/changes.rs`: `OpenView.whole: Load<WholeChange>`, `Msg::ListRead.whole: Result<WholeChange, String>`, `current_ids` (working tree from `whole`), `file_reviews(state, view, connected)`, `MarkControl { label, pressed, block }` and `mark_control(state, view, connected)` (`None` with no file selected), `header_progress(state, view, connected)`, and `Msg::MarkToggled`
- [ ] T029 [US1] Run `git.whole_change(dir, &scope)` in `read_list` beside the list read and send it in `Msg::ListRead.whole`, and send `SetReviewed` edits through a new `PendingOp::ReviewMark` in `crates/micold-client/src/shell/changes.rs`; add `PendingOp::ReviewMark` with failure text "save the reviewed mark" on `PendingOp::ReviewEdit`'s path in `crates/micold-client/src/shell/daemon_sync.rs` (research R6). First write a failing test in `daemon_sync.rs`'s `tests` module: an error reply to a pending `ReviewMark` notifies "Couldn't save the reviewed mark: …" (where review edit failures appear) and leaves `State.reviews` and the row's state unchanged (Edge Cases: a failed mark request)
- [ ] T030 [P] [US1] Add `Icon::ReviewUnseen` (○), `Icon::ReviewDrafts` (●), `Icon::ReviewDone` (✓) with their glyphs and `icon_role` surfaces in `crates/micold-material/src/icons.rs`, extending `crates/micold-client/tests/icon_roles.rs` if it enumerates icons
- [ ] T031 [US1] Draw the row's leading state `Glyph` (draft count `Tag` unchanged) in `file_row`, the diff header `ToggleChip::new(label, Msg::MarkToggled, r).active(pressed).disabled(block.is_some())` inside a `Tooltip` beside **Whole file**, and the header slot with `Text` *n of m reviewed* and `ProgressBar::new(ratio, r).width(Length::Fixed(120.0))`, laid out at fixed width even when empty (FR-009), in `crates/micold-client/src/ui/changes.rs`
- [ ] T032 [US1] Add covered states `changes-review-marks` (○, ●, ✓ rows, the chip pressed, a partial bar) and `changes-review-marks-unavailable` (no progress; the header's other boxes in the same place) in `crates/micold-client/tests/support/covered_states.rs`, and regenerate `crates/micold-client/tests/fixtures/layout_snapshot.txt` with `UPDATE_LAYOUT_SNAPSHOT=1 cargo test -p micold-client --test layout_snapshot`
- [ ] T033 [US1] Check the containment and tooltip gates in `crates/micold-client/tests/gates/` cover the new header slot and chip tooltip (add the new states where a gate lists states), so no header control overlaps or is truncated at 1280×800 (SC-005)
- [ ] T034 [US1] Add *Marking files reviewed* to `docs/user-guide/reviewing-changes.md` (FR-015): the three row states, the **Mark reviewed** / **Reviewed** control, why it is unavailable (Committed / Uncommitted hidden, not read yet, service not connected), *n of m reviewed* and that *m* follows the toggles, marks shared by every window and kept across restarts, removed with the worktree

**Checkpoint**: US1's independent test passes in the app.

---

## Phase 5: User Story 2 — A file that changes after I marked it asks for another look (Priority: P1) (M4)

**Goal**: a marked file whose change differs shows ○ / ● with the changed dot and leaves *n*.

**Independent Test**: mark a file, change a line on disk, wait for the re-read: ○ with the dot,
*n* − 1; revert the line: Reviewed again (spec US2).

- [ ] T035 [US2] Write failing reducer tests in `crates/micold-client/tests/features_changes.rs` for US2 s1–s4 and s7: a re-read whose whole change gives another id makes the file Unseen (HasDrafts with drafts) with `changed_since` and *n* − 1; marking again clears `changed_since` and sends the new id; a re-read back to the marked id makes it Reviewed with no edit sent; another file changing, a new file appearing (*m* + 1) and a new id for another path leave it Reviewed; a re-read when the view is shown again (#1269's tab re-read) shows the change
- [ ] T036 [US2] Draw `UnreadMark` (dot only) inside a `Tooltip` "Changed since you marked it reviewed" at the trailing edge of a row whose `FileReview.changed_since` (from T028's `file_reviews`) is true, in `crates/micold-client/src/ui/changes.rs`
- [ ] T037 [US2] Add a changed-since row to the `changes-review-marks` covered state in `crates/micold-client/tests/support/covered_states.rs` and regenerate `crates/micold-client/tests/fixtures/layout_snapshot.txt`
- [ ] T038 [US2] Describe in `docs/user-guide/reviewing-changes.md` *Marking files reviewed* that a file whose change differs from the marked one drops back with the changed dot, that undoing the change brings the mark back, that committing unaltered keeps it in an entry with a base, and that renames start Unseen

**Checkpoint**: US2's independent test passes in the app.

---

## Phase 6: User Story 3 — Every window, and the next start, agrees (Priority: P2) (M4)

**Goal**: the view follows the marks pushed by another window; restart keeps them. The service
side is T020–T022, the store T016–T017.

**Independent Test**: two windows on one entry, mark in one, the other shows it within 2 s;
restart, the mark is still shown (spec US3).

- [ ] T039 [US3] Write a reducer test in `crates/micold-client/tests/features_changes.rs`: a `ReviewChanged` arriving with a mark set elsewhere changes that row to ✓ and raises `header_progress` with no action in this window; a push removing it returns the row; a `ReviewChanged` for the entry after a reconnect restores the marks (US3 s1–s3)

**Checkpoint**: US3's independent test passes with two windows.

---

## Phase 7: User Story 4 — Pull requests have their own marks (Priority: P3) (M5)

**Goal**: a pull request's files have their own marks and progress.

**Independent Test**: mark a file in pull request #12, switch to the working tree: the same path
is not marked; switch back: it is (spec US4).

- [ ] T040 [P] [US4] Write failing unit tests for `pr_change_ids(files, patches)` in `crates/micold-core/src/review/pr_diff.rs`: one id per file with a patch; a file's id changes only when its own patch section changes (US4 s3); a file without a patch has no id
- [ ] T041 [US4] Implement `pr_change_ids(files: &[ChangedFile], patches: &BTreeMap<RelPath, Vec<u8>>) -> BTreeMap<RelPath, ChangeId>` by `ChangeId::of_patch` in `crates/micold-core/src/review/pr_diff.rs`
- [ ] T042 [US4] Write failing reducer tests in `crates/micold-client/tests/features_pr_changes.rs`: with pull request #12 shown, rows, `mark_control` and `header_progress` use `PullRequest(12)` marks and the ids of the ready diff; marking sends `SetReviewed { scope: PullRequest(12), .. }`; the working tree's same path is unaffected; a mark of a pull request that left the list is kept and shown when its diff is shown again (US4 s1, s2); `NotRead` while the diff is not ready
- [ ] T043 [US4] Make `current_ids` answer `pr_change_ids` of `PrDiffLoad::Ready` for a pull request view, and `file_reviews` / `mark_control` / `header_progress` read `of_scope(view_scope(view))`, in `crates/micold-client/src/features/changes.rs`
- [ ] T044 [US4] Add a paragraph under *Pull requests* in `docs/user-guide/reviewing-changes.md`: a pull request's files have their own marks and progress, kept when it is merged, closed or leaves the list, and a new head commit that changes a file's diff drops its mark

**Checkpoint**: US4's independent test passes in the app.

---

## Phase 8: Polish & Cross-Cutting Concerns (close unit)

- [ ] T045 [P] Note slice R4 item 1 (#1271) as done in `docs/development/review-redesign.md`
- [ ] T046 Run quickstart.md Part A (`mise run test-core`, `mise run gate`)
- [ ] T047 Run quickstart.md Part B with the `visual-pass` skill at 1280×800 in both themes and record the screenshots in `specs/1271-review-reviewed-marks/visual-pass/` (SC-005)

---

## Dependencies & Execution Order

- **Setup (T001)** → **Foundational A (T002–T017)** → **Foundational B (T018–T024)** → user
  stories.
- **US1 (T025–T034)** needs Foundational A and B and #1269 merged.
- **US2 (T035–T038)** and **US3 (T039)** need US1 (they extend its view model and row).
- **US4 (T040–T044)** needs US1 (`current_ids`, `mark_control`, `header_progress`).
- **Polish** after all stories.

Within a phase, each test task precedes the implementation task after it (T002 → T003, T004 →
T005, …). Implementation tasks in `marks.rs` (T003, T005, T009, T013) run in order: one file. Of the `marks.rs` test tasks only T002 is `[P]`; T004, T008 and T012 follow the implementation before them. T020 and T021 share `tests/review_edit.rs` and run in order.

## Parallel Example: Foundational A

```text
T002 ChangeId tests (marks.rs)     T006 NameStatus tests (changes.rs)
T010 G2 tests (tests/review_git.rs) T014 EntryReview tests (comment.rs)
T016 store S4 tests (store.rs)
```

## Parallel Example: Foundational B

```text
T018 wire tests (tests/protocol_roundtrip.rs)   T020 daemon tests (then T021, same file)
T023 client receipt tests (tests/features_changes.rs)
```

## Implementation Strategy

1. M1 and M2 (Phases 1–3) merge while #1269 is open: they change no view.
2. After #1269 merges, M3 (US1) is the MVP: marking and progress in the working tree.
3. M4 adds the changed dot and the multi-window view test; M5 adds pull request marks.
4. The close unit runs Polish.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Reviewed marks in micold-core

- **Tasks**: T001–T017
- **Deliverable**: `micold-core` computes a file's change id from a toggle-independent whole-change read, keeps one mark per file per source per entry, derives each file's review state and *n of m* progress, and stores marks in review file version 4 (older files load unchanged); developers see it in `mise run test-core`.
- **Satisfies**: FR-003, FR-005, FR-006, FR-010 (storage), FR-012, FR-014, FR-016 (core); US2 s3, s5, s6 (ids); SC-003, SC-004 (store)
- **Verify**: `mise run test-core` (`review::marks`, `review::changes`, `review::comment`, `review::store`, `--test review_git`)
- **Depends on**: — (D4 answered A or B; C changes the data model, research R7)
- **Tier**: full

### M2 — Marks on the wire and in the session service

- **Tasks**: T018–T024
- **Deliverable**: the session service sets, keeps, pushes and removes marks (`SetReviewed`, `ReviewChanged.marks`, protocol +1) and every attached window holds each entry's marks; no view change (`docs-not-needed`).
- **Satisfies**: FR-007, FR-010, FR-011, FR-013 (service, RM7), FR-016; US3 s1–s4 (service side); contracts RM1–RM7
- **Verify**: `cargo test -p micold-core --test protocol_roundtrip --test schema_hash`, `cargo test -p micold-daemon --test review_edit --test review_send`, `cargo test -p micold-client --test features_changes`, via `mise run gate`
- **Depends on**: M1
- **Tier**: full

### M3 — Mark files reviewed and see progress 🎯 MVP

- **Tasks**: T025–T034
- **Deliverable**: in the review view every working-tree row shows ○ / ● / ✓, the diff header's **Mark reviewed** / **Reviewed** control marks and unmarks the file (unavailable with a tooltip when a toggle hides part of its change, it is not read yet, or the service is not connected), and the header shows *n of m reviewed* with a bar; the user guide describes it.
- **Satisfies**: US1 s1–s7; FR-001, FR-002, FR-004, FR-008, FR-009, FR-013 (view), FR-015; SC-001
- **Verify**: `cargo test -p micold-core review::marks`, `cargo test -p micold-client --test features_changes --test layout_snapshot --test icon_roles`, quickstart Part B steps 1–4 and 8
- **Depends on**: M2 and #1269 merged
- **Tier**: full

### M4 — Changed since reviewed, and windows agree in the view

- **Tasks**: T035–T039
- **Deliverable**: a marked file whose change differs drops back to ○ / ● with the changed dot and leaves *n*, returning to ✓ when the change is undone; another window's marks show in this view without action; the user guide describes changed files.
- **Satisfies**: US2 s1–s4, s7; US3 s1–s3 (view); FR-003 (dot), SC-002, SC-003
- **Verify**: `cargo test -p micold-client --test features_changes --test layout_snapshot`, quickstart Part B steps 2 (second window), 5, 6, 9
- **Depends on**: M3
- **Tier**: light

### M5 — Pull request marks

- **Tasks**: T040–T044
- **Deliverable**: a pull request's files show their own marks and *n of m reviewed*, separate from the working tree's, kept when the pull request leaves the list, and dropped back when a new head commit changes a file's diff; the user guide describes it.
- **Satisfies**: US4 s1–s3; FR-006 (sources)
- **Verify**: `cargo test -p micold-core review::pr_diff`, `cargo test -p micold-client --test features_pr_changes`, quickstart Part B step 7
- **Depends on**: M3
- **Tier**: full

Polish (T045–T047) changes no code: the close unit does it in the close PR.

## Notes

- [P] tasks touch different files and depend on no unfinished task.
- Commit after each test/implementation pair; run `mise run gate` before each milestone's PR.
