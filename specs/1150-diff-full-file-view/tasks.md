---
description: "Task list for #1150 full-file view and repo browsing in the Changes view"
---

# Tasks: Show the full file and browse the repo's code while reviewing a diff

**Input**: `specs/1150-diff-full-file-view/` (spec.md, plan.md, research.md, data-model.md, contracts/changes-view.md, quickstart.md)

**Tests**: mandatory (Constitution I). Within each phase the test tasks come first and must fail before the implementation tasks they cover. Widget glue in `ui/` uses the GUI-glue exception (geometry gates + quickstart Part B).

**Documentation**: every story updates `docs/user-guide/reviewing-changes.md` in its own milestone (Constitution VII).

**Cross-platform**: `RelPath` is `/`-separated, `git` runs through the existing `no_window` wrapper, nothing is platform-specific (Constitution VI).

## Format: `[ID] [P?] [Story] Description`

Crate paths: `crates/micold-core/src/review/` (core), `crates/micold-client/src/` (client).

## Phase 1: Setup

- [x] T001 Register an empty `pub mod expand;` in `crates/micold-core/src/review/mod.rs` and create `crates/micold-core/src/review/expand.rs` with the module doc, so later tasks compile

## Phase 2: Foundational (gap computation and shaping, pure core)

**Blocks every story.** Tests first.

- [x] T002 [P] Write failing unit tests for `gaps(hunks, new_total)` in `crates/micold-core/src/review/expand.rs`: leading/between/trailing runs, zero-length runs omitted, hunk at line 1, hunk at EOF, pure-deletion hunk (`new_len == 0`), empty and one-line file, file fully covered (data-model §1)
- [x] T003 [P] Write failing property/unit tests for `shape` in `crates/micold-core/src/review/expand.rs`: whole-file covers every new-side line once in order, changed and removed lines preserved in place, numbering continuous across a revealed run, touching hunks merged (recomputed `old_len`/`new_len`, first `section`, last no-newline flags), closed expansion returns the input hunks with every run listed, collapse restores input, 5,000-line shape under 100 ms (SC-004)
- [x] T004 [P] Write failing unit tests for `Expansion` (`toggle`, `set_whole`, `is_open`) and `reconcile` in `crates/micold-core/src/review/expand.rs`: open gap kept when position and span are unchanged, dropped when it vanished, `whole` kept (FR-008)
- [x] T005 Implement `Gap`, `GapKey`, `Position`, `Expansion` and `gaps()` in `crates/micold-core/src/review/expand.rs` (T002, T004 gaps part green)
- [x] T006 Implement `shape()` and `Shaped` in `crates/micold-core/src/review/expand.rs` (T003 green)
- [x] T007 Implement `Expansion::reconcile` in `crates/micold-core/src/review/expand.rs` (T004 green)
- [x] T008 [P] Write failing tests for row builders over `&Shaped` in `crates/micold-core/src/review/diff.rs`: `UnifiedRow::Gap` / `SideRow::Gap` emitted where a gap is, revealed lines are ordinary context rows with the same numbers, indexes (`UnifiedIndex`, `SideIndex`) stay consistent in both layouts (FR-007)
- [x] T009 Make `unified_rows`, `side_by_side_rows` and their index types accept `&Shaped` and emit gap rows in `crates/micold-core/src/review/diff.rs`; update existing callers

**Checkpoint**: `mise run test-core` green.

## Phase 3: User Story 1 - Expand a hidden gap between hunks (P1) 🎯 MVP

**Goal**: gap markers on worktree-change diffs, expand and collapse in place.
**Independent test**: file with two distant hunks; expand and collapse each gap.

- [x] T010 [P] [US1] Write failing reducer tests in `crates/micold-client/tests/features_changes.rs`: `Msg::GapToggled` opens and closes a range for the selected file, state keyed `(SourceKey, RelPath)` never leaks across files, refresh reconciles (kept/dropped), added/deleted/binary files offer no markers (`Revealable`), over-2 MB version shows the large note and `Msg::ShowLargeFile` ("Show anyway") expands it, worktree new side comes from `LoadedDiff::new` (FR-001, FR-003, FR-008, FR-009, FR-010)
- [x] T011 [P] [US1] Write failing geometry-gate tests for the gap row (height, hit target, unified and side-by-side) in `crates/micold-client/src/ui/material/diff_view.rs`
- [x] T012 [US1] Add per-file `Expansion` map, `SourceKey`, `Revealable`, `Msg::GapToggled`, `Msg::ShowLargeFile`, shaping on `DiffRead` and reconcile on refresh in `crates/micold-client/src/features/changes.rs` (T010 green)
- [x] T013 [US1] Add `DiffView::gaps(..)` and `.on_gap(..)` builders and the gap row ("N hidden lines", expand and collapse controls) in `crates/micold-client/src/ui/material/diff_view.rs` (T011 green)
- [x] T014 [P] [US1] Pass shaped hunks and gaps to `DiffView`, show the large-file note with "Show anyway" in `crates/micold-client/src/ui/changes.rs`
- [x] T015 [P] [US1] Add `highlight_file` (highlight over the full new side so revealed lines are coloured like the diff's context lines; the one highlight path later milestones reuse) in `crates/micold-client/src/ui/syntax.rs`
- [x] T016 [P] [US1] Add a gap row to the review section of `crates/micold-client/src/showcase/sections/review.rs`
- [x] T017 [US1] Document "Reading hidden lines" (markers, expand, collapse, committed-only note, large files) in `docs/user-guide/reviewing-changes.md`

## Phase 4: User Story 2 - Show the whole file with one toggle (P1)

**Goal**: per-file whole-file mode.

- [x] T018 [P] [US2] Write failing reducer tests in `crates/micold-client/tests/features_changes.rs`: `Msg::WholeFileToggled` shows every line once and off restores hunks, toggle hidden for added files and disabled (with reason) when no text version, whole-file state per `(SourceKey, RelPath)`, survives refresh (FR-004, FR-008)
- [x] T019 [US2] Add `Msg::WholeFileToggled` and the added-file/unavailable gating in `crates/micold-client/src/features/changes.rs` (T018 green)
- [x] T020 [P] [US2] Add the per-file whole-file toggle (hidden for added files; for other files with no text version, disabled with a tooltip, since FR-009 forbids offering expansion) to the file header in `crates/micold-client/src/ui/changes.rs`
- [x] T021 [P] [US2] Run `highlight_file` under the existing `highlight_within` budget (uncoloured on budget, no UI freeze for large files) with a test in `crates/micold-client/src/ui/syntax.rs`
- [x] T022 [US2] Document "Showing the whole file" in `docs/user-guide/reviewing-changes.md`

## Phase 5: User Story 3 - Works for a pull request's diff (P1)

**Goal**: gaps and whole-file on a PR's files show the PR head version.

- [x] T023 [P] [US3] Write failing unit tests for `PullRequestSource::file_at` args builder (percent-encoded path segments, `Accept: application/vnd.github.raw`), failure mapping to `FileReadFailure`, and the fake's `with_file_answer` in `crates/micold-core/src/pull_request.rs`
- [x] T024 [P] [US3] Write failing real-git integration tests in `crates/micold-core/tests/review_browse.rs`: PR-head read equals the commit's blob while the worktree differs, `read_file` failures (Binary, NotUtf8, Missing, TooLarge, `force_large`, worktree symlink escape refused), `..`/absolute/symlink-escape refused, read-only worktree: files and `git status` unchanged and the fake `gh` unused for worktree reads (FR-005, FR-010, FR-014)
- [x] T025 [P] [US3] Write failing reducer tests in `crates/micold-client/tests/features_pr_changes.rs`: selecting a PR file returns `Effect::ReadFile` for the head, no markers until `Msg::FileRead` answers (caption "Reading file…"), stale `seq` or changed source dropped, failure shows the FR-009 message with the diff usable, expansion never applies across worktree/PR/PR
- [x] T026 [US3] Implement `PullRequestSource::file_at`, its pure args builder and the fake answer in `crates/micold-core/src/pull_request.rs` (T023 green)
- [x] T027 [US3] Create `crates/micold-core/src/review/browse.rs` (register in `review/mod.rs`) with `FileSource`, `Confined`, `FileText`, `FileReadFailure` and `read_file` (worktree via fs, PR head via local `git cat-file` first then `file_at`; 2 MB limit with `force_large`) (T024 green)
- [x] T028 [US3] Add `Effect::ReadFile`, `Msg::FileRead { seq, .. }` and PR-side shaping in `crates/micold-client/src/features/changes.rs` (T025 green)
- [x] T029 [US3] Run `Effect::ReadFile` on a blocking thread with the `shared_git()` locality gate in `crates/micold-client/src/shell/changes.rs`
- [x] T030 [P] [US3] Show the "Reading file…" caption and the FR-009 message in `crates/micold-client/src/ui/changes.rs`
- [x] T031 [US3] Document pull-request files (head version, offline limits) in `docs/user-guide/reviewing-changes.md`

## Phase 6: User Story 4 - Pick revealed lines for review comments (P2)

- [x] T032 [P] [US4] Write failing reducer tests in `crates/micold-client/tests/features_changes.rs`: `pickable` is true for revealed lines exactly where the diff's lines are pickable and addressed by the same file and line numbers, false where picking is not allowed (the rule is the existing line-pick handling in `update`, `features/changes.rs` near `Pick`; `pickable` mirrors it and adds none)
- [x] T033 [US4] Add the single gate `pickable(view, file)` and call it from the existing pick handling in `update` in `crates/micold-client/src/features/changes.rs` (T032 green)
- [x] T034 [US4] Route revealed lines through the diff's pick path in `crates/micold-client/src/ui/changes.rs`
- [x] T035 [US4] Document picking of revealed lines in `docs/user-guide/reviewing-changes.md`

## Phase 7: User Story 5 - Browse any file and return to the diff (P2)

### Worktree browsing, tree, picker, back

- [x] T036 [P] [US5] Write failing tests in `crates/micold-core/tests/review_browse.rs` and `crates/micold-core/src/review/diff.rs`: `list_tree` for worktree changes (tracked and untracked, ignored excluded), `plain_file` yields context-only lines with CR stripped
- [x] T037 [P] [US5] Write failing `BrowseStack` tests (push, back, empty back is a no-op, clear on source change) in `crates/micold-core/src/review/browse.rs`
- [x] T038 [P] [US5] Write failing reducer tests in `crates/micold-client/tests/features_changes.rs`: `pickable` is false for untouched browsed files (pinned until #1149; FR-015, US4 scenario 3), SC-006 action count (tree or picker, select: at most 3 actions; Back: 1), `BrowseOpen`, `PathPickerChanged`, `TreeRead`, `BrowseBack` restores file, scroll and expansion (against a refreshed diff), `OpenFileDiff` only when the change modifies the file, missing/binary/large messages with Back working, large "Show anyway"
- [x] T039 [US5] Implement `list_tree` (worktree: `git ls-files -z --cached --others --exclude-standard`; typed ignored path checked on disk) and `BrowseStack` in `crates/micold-core/src/review/browse.rs`
- [x] T040 [US5] Implement `plain_file(&SideLines) -> FileDiff` in `crates/micold-core/src/review/diff.rs`
- [x] T041 [US5] Add `BrowsedFile`, `Effect::ReadTree`, `Msg::{BrowseOpen, BrowseBack, OpenFileDiff, PathPickerChanged, TreeRead}` in `crates/micold-client/src/features/changes.rs` (T038 green)
- [x] T042 [US5] Run `Effect::ReadTree` on a blocking thread in `crates/micold-client/src/shell/changes.rs`
- [x] T043 [P] [US5] Write a failing geometry-gate test for `BrowseBar` and the tree/picker header, then create the shared `BrowseBar` component (Back, path, "Open diff") in `crates/micold-client/src/ui/material/browse_bar.rs` and register it
- [x] T044 [US5] Add the file tree (`TreeView`), path picker (`Typeahead`) and browse header to the list pane in `crates/micold-client/src/ui/changes.rs`
- [x] T045 [P] [US5] Add `BrowseBar` to `crates/micold-client/src/showcase/sections/review.rs`
- [x] T046 [US5] Document "Browsing the repository" in `docs/user-guide/reviewing-changes.md`

### Pull request head browsing

- [x] T047 [P] [US5] Write failing unit tests for `tree_at` (recursive trees args, `truncated` flag, failure mapping) and the fake's `with_tree_answer` in `crates/micold-core/src/pull_request.rs`
- [x] T048 [P] [US5] Write failing integration tests in `crates/micold-core/tests/review_browse.rs`: head tree vs worktree tree, untracked/ignored never listed for a PR, path missing at head gives the FR-009 message
- [x] T049 [P] [US5] Write failing reducer tests in `crates/micold-client/tests/features_pr_changes.rs`: browsing under a PR reads the head, switching PR/source clears the stack and re-reads the browsed path (or shows missing), truncated tree note and typed path still read
- [x] T050 [US5] Implement `PullRequestSource::tree_at` and the fake answer in `crates/micold-core/src/pull_request.rs` (T047 green)
- [x] T051 [US5] Extend `list_tree` for `FileSource::PrHead` (`git ls-tree -r -z --name-only`, else `tree_at`) in `crates/micold-core/src/review/browse.rs` (T048 green)
- [x] T052 [US5] Source-switch handling and truncated-tree note in `crates/micold-client/src/features/changes.rs` (T049 green)
- [x] T053 [US5] Document PR-head browsing in `docs/user-guide/reviewing-changes.md`

### References in diff lines

- [x] T054 [P] [US5] Write failing unit tests for `reference::find` in `crates/micold-core/src/review/reference.rs`: resolves only listed files, relative then root, extension and `index`/`mod.rs` probing, `:LINE` `#LLINE` `(LINE)` suffixes, nothing returned for unknown names
- [x] T055 [P] [US5] Write failing reducer tests in `crates/micold-client/tests/features_changes.rs`: `ReferenceActivated` opens the file at the line and pushes the diff frame, unresolved target shows a short notice only
- [x] T056 [P] [US5] Write failing geometry-gate tests for reference hit targets in `crates/micold-client/src/ui/material/diff_view.rs`
- [x] T057 [US5] Implement `Reference` and `find` in `crates/micold-core/src/review/reference.rs` (register in `review/mod.rs`) (T054 green)
- [x] T058 [US5] Add `Msg::ReferenceActivated` and per-line reference derivation in `crates/micold-client/src/features/changes.rs` (T055 green)
- [x] T059 [US5] Add `DiffView::references(..)` / `.on_reference(..)` (underline, click target) in `crates/micold-client/src/ui/material/diff_view.rs` (T056 green)
- [x] T060 [US5] Wire references in `crates/micold-client/src/ui/changes.rs`
- [x] T061 [P] [US5] Add a reference link to `crates/micold-client/src/showcase/sections/review.rs`
- [x] T062 [US5] Document following references in `docs/user-guide/reviewing-changes.md`

## Phase 8: Polish (no code; done by the close unit, not a milestone)

- [ ] T063 Record quickstart Part B results in `specs/1150-diff-full-file-view/quickstart.md`
- [ ] T064 Tick completed tasks and convergence in `specs/1150-diff-full-file-view/tasks.md`

## Dependencies and order

Setup, then Foundational, then US1 (MVP). US2, US3, US4 depend on US1; US5 worktree browsing depends on US1, US3 (`read_file`, `Effect::ReadFile`) and US4 (`pickable`); PR-head browsing on US3 and worktree browsing; references on worktree browsing. Within a phase: tests, core, reducer, shell, UI, docs.

## Implementation Strategy

MVP is M1 (T001-T017): gap markers on worktree diffs. Then one story slice per milestone, in the order below.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot). Polish (T063-T064) changes no code and is left to the close unit.

### M1 — Expand hidden lines in a diff 🎯 MVP

- **Tasks**: T001–T017
- **Deliverable**: In the Changes view, hidden unchanged lines show as "N hidden lines" markers that expand in place and collapse again, with continuous numbers and diff colouring in both layouts.
- **Satisfies**: US1 acceptance scenarios 1–3; FR-001, FR-002, FR-003, FR-007, FR-008, FR-009, FR-010; SC-002, SC-004
- **Verify**: `mise run test-core`; `cargo test -p micold-client --test features_changes`; quickstart Part B steps 1, 2 (gaps), 3, 5, 7, 10
- **Depends on**: —
- **Tier**: full

### M2 — Whole-file toggle

- **Tasks**: T018–T022
- **Deliverable**: Each file's header has a toggle showing every line of the file with changes still marked, and back to hunks.
- **Satisfies**: US2 acceptance scenarios 1–2; FR-004, FR-008
- **Verify**: `cargo test -p micold-client --test features_changes whole`; quickstart Part B steps 2, 3
- **Depends on**: M1
- **Tier**: light

### M3 — Pull request head version

- **Tasks**: T023–T031
- **Deliverable**: Expanding or toggling a file under a selected pull request shows the PR head's lines, not the worktree's.
- **Satisfies**: US3 acceptance scenarios 1–2; FR-005, FR-009, FR-010, FR-014; SC-003
- **Verify**: `cargo test -p micold-core --test review_browse`; `cargo test -p micold-client --test features_pr_changes`; quickstart Part B steps 4, 9
- **Depends on**: M1
- **Tier**: full

### M4 — Pick revealed lines

- **Tasks**: T032–T035
- **Deliverable**: Revealed lines are pickable exactly where the diff's lines are, with the same file and line identity; nowhere else.
- **Satisfies**: US4 acceptance scenarios 1–2 (scenario 3 is pinned in M5, T038); FR-006, FR-015 (pick part); SC-005
- **Verify**: `cargo test -p micold-client --test features_changes pickable`; quickstart Part B step 8b
- **Depends on**: M1
- **Tier**: light

### M5 — Browse repository files

- **Tasks**: T036–T046
- **Deliverable**: A file tree and path picker open any file of the worktree with numbers and colouring; Back returns to the diff with file, scroll and expansion; "Open diff" for changed files.
- **Satisfies**: US5 acceptance scenarios 1, 3, 5 and 4 (worktree); FR-011, FR-013, FR-014, FR-015; SC-006
- **Verify**: `cargo test -p micold-core --test review_browse`; `cargo test -p micold-client --test features_changes browse`; quickstart Part B steps 6 (tree and picker, Back), 8
- **Depends on**: M1, M3, M4
- **Tier**: full

### M6 — Browse at the pull request head

- **Tasks**: T047–T053
- **Deliverable**: Under a selected pull request, the tree, picker and browsed files are the PR head's, including files the PR does not change.
- **Satisfies**: US5 acceptance scenario 4 (PR); FR-011, FR-014; SC-007
- **Verify**: `cargo test -p micold-core --test review_browse head`; `cargo test -p micold-client --test features_pr_changes browse`; quickstart Part B steps 6, 8, 9
- **Depends on**: M3, M5
- **Tier**: full

### M7 — Follow references from the diff

- **Tasks**: T054–T062
- **Deliverable**: A path or import string in a diff line that names an existing repository file is clickable and opens that file at the referenced line.
- **Satisfies**: US5 acceptance scenario 2; FR-012
- **Verify**: `cargo test -p micold-core reference`; `cargo test -p micold-client --test features_changes reference`; quickstart Part B step 6
- **Depends on**: M5
- **Tier**: full
