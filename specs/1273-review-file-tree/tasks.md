---

description: "Task list for feature 1273: changed-file tree, filter and sort in the Review UI"
---

# Tasks: Review UI — changed-file tree, filter and sort

**Input**: Design documents from `specs/1273-review-file-tree/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/review-tree.md](./contracts/review-tree.md),
[quickstart.md](./quickstart.md)

**Tests**: Per Constitution Principle I (Test-First, NON-NEGOTIABLE), every behaviour task has a
test task before it, written Red first. The GUI exception covers only the row drawing in
`ui/changes.rs` and `ChangeBar`'s painting, backed by the `ChangeBar` geometry test, the layout
snapshot and quickstart Part B.

**Documentation**: Per Constitution Principle VII, `docs/user-guide/reviewing-changes.md` (*What the
list shows*) is updated in the milestone that makes each part visible: the tree and the bar in M4,
the filter box and the sort control in M5 (FR-018).

**Cross-platform**: No OS branch. Paths are `/`-separated `RelPath`s; ordering is byte order of path
components; matching is `micold_core::typeahead` (Principle VI).

**Delivery order** (research R10): Phases 1–3 are core-only (`micold-core`), change no existing file
except `crates/micold-core/src/review/mod.rs`, and merge while #1272 is open. Phases 4–7 change the
client and **start only after #1272 merges**, following wherever #1272 put the list pane (spec
Assumptions). #1271 is not waited on: until it lands the reviewed predicate is always-false and
`sort_options(false)` hides **Unreviewed first** (Clarifications, FR-012).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an incomplete task)
- **[Story]**: US1, US2, US3, US4 from spec.md

---

## Phase 1: Setup

**Purpose**: the new core module exists, so every later test file compiles against it.

- [ ] T001 Create `crates/micold-core/src/review/tree.rs` with its module header (feature 1273, links to `specs/1273-review-file-tree/contracts/review-tree.md` and `data-model.md`; render-free, iced-free), declare `pub mod tree;` in `crates/micold-core/src/review/mod.rs`, and create the empty test file `crates/micold-core/tests/review_tree.rs`

---

## Phase 2: Foundational — the shared tree layout (blocks every story)

**Purpose**: one tree layout in core for both the changed-file tree and the **Files** browser
(research R1, FR-017): grouping, chaining, both openness modes, Path order, and the collapse state
transitions. Nothing here is reachable from the UI yet.

- [ ] T002 Write failing tests in `crates/micold-core/tests/review_tree.rs` for `layout` under `TreeSort::Path`, no filter: contract rows L1 (folders before files, depths `src` 0, `ui` 1, `b.rs` 2, `a.rs` 1, `README.md` 0), L2 (`chain: true` gives one row labelled `crates/core/src/review` with `paths` = the four real folders, outermost first), L3 (`src` in `collapsed` gives `src` with `open: false` and `README.md` only; removing it restores `ui` with its own state), L4 (chained `a/b/c` with `a/b` collapsed shows `open: false`), one file only and all files at the top level (no folder rows), a rename (the file sits under its new path's folder), sibling order by name with a chained folder ordered by its first folder's name (`a/b/c.rs` chained and `a-x/d.rs` list `a/b` before `a-x`; spec 4.1), and SC-002 (a 30-file, 5-folder fixture: no file row `label` contains `/`); every file row's `RowKind::File { leaf }` indexes the input slice
- [ ] T003 Write failing **Files**-parity tests in `crates/micold-core/tests/review_tree.rs` for contract row L12: `chain: false`, `Openness::DefaultClosed { open }` reproduces the rows that `the_tree_shows_a_folders_files_while_it_is_open` and `a_change_re_reads_the_browsed_file_and_the_tree` in `crates/micold-client/tests/features_changes.rs` expect today (top-level folders closed, an open folder's contents one level deeper, no chaining), and an unsorted input lists each level's files in byte order of the name
- [ ] T004 Implement in `crates/micold-core/src/review/tree.rs` the types of data-model §1–§3 — `Leaf { path: &str, added: u32, removed: u32 }`, `TreeSort { Path, Size, UnreviewedFirst }` (`Default` = `Path`), `Openness::{DefaultOpen { collapsed }, DefaultClosed { open }}`, `LayoutOptions { chain, openness, sort, query: Option<&Query>, reviewed: &dyn Fn(&str) -> bool }`, `Row { depth: u16, label, spans: Vec<Range<usize>>, kind }`, `RowKind::{Folder(FolderRow), File { leaf: usize }}`, `FolderRow { paths, open, added: u64, removed: u64 }` — and `layout(leaves, options) -> Vec<Row>` with one `BTreeMap` build, folders before files, Path order by byte order of the row's name (a chained folder's first folder name; data-model §3), chaining and both openness modes (a chained row is closed when any of its `paths` is in the collapsed set), until T002 and T003 pass; until Phase 3 every sort orders as `Path`, `query` is ignored and totals are 0 (no `todo!`; Phase 3 fills them)
- [ ] T005 Write failing tests in `crates/micold-core/tests/review_tree.rs` for `TreeChoices` (data-model §2): `Default` is empty filter, `Path`, empty sets; `toggle` on an open chained row inserts only its deepest path, `toggle` on a collapsed chained row removes all its `paths`; `reveal(path)` removes every ancestor folder of `path` from `collapsed` (FR-008, edge case "Selection inside a collapsed folder"); no method prunes a folder no longer listed (a folder that leaves and returns keeps its state); `options(&query, &reviewed)` gives `DefaultOpen` over `collapsed`, `chain: true` and the stored sort
- [ ] T006 Implement `TreeChoices { filter: String, sort: TreeSort, collapsed: BTreeSet<String>, filter_collapsed: BTreeSet<String> }` with `toggle`, `reveal` and `options` in `crates/micold-core/src/review/tree.rs` until T005 passes (filter-time behaviour of the active set comes in T013–T015)

**Checkpoint**: `mise run test-core` passes; no client file changed.

---

## Phase 3: Core rules for size, filter and sort (US1–US4, core only)

**Purpose**: every remaining rule of the tree in core, so the client phases only wire and draw. Each
task carries the story it serves.

### Totals, bar, indentation, hover (US1, US2)

- [ ] T007 [US2] Write failing tests in `crates/micold-core/tests/review_tree.rs` for folder totals: contract L5 (`x.rs` +40 −10 and `y.rs` +1 −1 under one folder give `added: 41`, `removed: 11`); a chained row's totals cover every file below it; a collapsed folder keeps its totals (scenario 2.5)
- [ ] T008 [P] [US2] Write failing unit tests in `crates/micold-core/src/review/tree.rs` for `bar_scale` and `bar` (data-model §5, research R8): `bar_scale` is the largest `added + removed` over the whole slice; `bar(40, 10, scale, w)` splits 4:1 (scenario 2.1); a file with total 100 is longer than one with 10 and no total exceeds `max_width` (2.2); `None` when `added + removed == 0` or `scale == 0` (2.3, 2.4, pure rename); a one-line change is at least `MIN_BAR` = 2.0 px
- [ ] T009 [US1] Write failing unit tests in `crates/micold-core/src/review/tree.rs` for `max_indent_levels(list_width, step, min_name_width)` (the name keeps `min_name_width` before indentation grows) and `indent_levels(depth, max) == min(depth, max)` (edge case "Very deep paths"), and for `hover_text(path, renamed_from)` = `path`, or `path` + `\nRenamed from <old>` for a rename (FR-005)
- [ ] T010 [US2] Implement folder totals in `layout`, `bar_scale`, `bar`, `BarParts { added: f32, removed: f32 }`, `MIN_BAR`, `max_indent_levels`, `indent_levels` and `hover_text` in `crates/micold-core/src/review/tree.rs` until T007–T009 pass

### Filter (US3)

- [ ] T011 [US3] Write failing tests in `crates/micold-core/tests/review_tree.rs` for the filter (research R4): L8 (`tests` over `src/daemon/tests.rs`, `src/lib.rs`, `docs/tests.md` lists `tests.rs` under `src/daemon` and `tests.md` under `docs`, no `lib.rs`, `spans` cover `tests` in each name, rebased to the label), a folder row's `spans` are the union of its listed files' spans inside its label, L9 (`src` in `collapsed` still shows open while the filter holds text), L10 (no match gives an empty `Vec`), whitespace-only text is no filter, the filter never reorders (Path order kept), and SC-001 (30 files in 5 folders, `tests.r` lists exactly `daemon/tests.rs`; the fixture holds no path the matcher reaches by one edit or a subsequence)
- [ ] T012 [US3] Implement filtering in `layout` with `typeahead::match_one(path, query)` per file, folders kept only on a matching file's path, totals over listed files, and span rebasing for file and folder labels in `crates/micold-core/src/review/tree.rs` until T011 passes
- [ ] T013 [US3] Write failing tests in `crates/micold-core/tests/review_tree.rs` for `TreeChoices` while filtering (research R2): `set_filter` from empty to non-empty clears `filter_collapsed`; while the filter holds text `toggle` writes `filter_collapsed`, never `collapsed`, and `options` uses `filter_collapsed` as the active set; `clear_filter` empties `filter` and `filter_collapsed` and leaves `collapsed` as before filtering (FR-010, scenario 3.2); `reveal` clears both sets; the filter text is stored as typed
- [ ] T014 [US3] Implement `set_filter`, `clear_filter` and the active-set switch in `toggle` and `options` in `crates/micold-core/src/review/tree.rs` until T013 passes

### Sort (US4)

- [ ] T015 [US4] Write failing tests in `crates/micold-core/tests/review_tree.rs` for the sorts: L6 (`Size`: folders with sums 50 and 3 in that order, files 100, 10, 5, equal totals keep Path order) and SC-004 (largest file first at each level, largest folder first at the top level), L7 (`UnreviewedFirst` with reviewed = {`src/a.rs`}: unreviewed files before `a.rs`, a folder holding any unreviewed file before one holding none, ties in Path order), folders before files under every sort, the tree kept under every sort (depths unchanged), L11 (`Size` + filter: filtered rows in Size order, totals over listed files)
- [ ] T016 [US4] Write failing unit tests in `crates/micold-core/src/review/tree.rs` for `Offer::{Shown, Hidden, Disabled(&'static str)}`, `UNREVIEWED_UNTIL_MARKS == Offer::Hidden` and `sort_options(marks)` (`false`: Path Shown, Size Shown, UnreviewedFirst `UNREVIEWED_UNTIL_MARKS`; `true`: all Shown), and for `TreeChoices::set_sort(sort, offers)` storing a sort only when `offers` lists it `Shown` (UnreviewedFirst refused while hidden, FR-012)
- [ ] T017 [US4] Implement per-level ordering for `Size` and `UnreviewedFirst` (research R3), `Offer`, `UNREVIEWED_UNTIL_MARKS`, `sort_options` and `set_sort` in `crates/micold-core/src/review/tree.rs` until T015 and T016 pass

### Responsiveness (FR-015, SC-003)

- [ ] T018 Write the SC-003 timing test in `crates/micold-core/tests/review_tree.rs` in the style of `crates/micold-core/tests/command_palette.rs:255` (slowest of repeated runs): 5,000 synthetic files in 200 folders; building the `Leaf`s from the 5,000 paths plus `layout` after a collapse, one filter keystroke and a sort change each take under 100 ms in release (the threshold is 500 ms under `cfg!(debug_assertions)`, so `mise run test-core` and CI's debug runs stay green without hiding a regression); if it fails, make `layout` meet it (one `BTreeMap` build, one sort per level, one `match_one` per file; research R9)

**Checkpoint**: `mise run test-core` passes; every contract row L1–L12 and every function of data-model
§2, §4, §5 has a passing test; still no client file changed.

---

## Phase 4: User Stories 1 and 2 in the client — view state (after #1272 merges)

**Goal**: the changed-file list's rows come from `review::tree`, collapsed state lives in each open
view, and the **Files** browser runs on the same layout. Nothing new is drawn yet.

**Independent Test**: `mise run gate`; the reducer tests below, and the existing **Files** tree tests
pass with their expectations unchanged.

- [ ] T019 [US1] Rebase onto `main` once #1272 has merged; record in the ledger the list pane's default width and the row constants T027 uses; update `specs/1273-review-file-tree/plan.md` (Project Structure, contract §2's placement) if #1272 moved the list pane or renamed `OpenView`, `ListBody` or the changes messages, and record that mapping in the ledger too
- [ ] T020 [US1] Write failing reducer tests in `crates/micold-client/tests/features_changes.rs` for: `OpenView.tree` default at open (every folder open, Path); `Msg::TreeFolderToggled(paths)` collapses and expands and recomputes `OpenView::change_rows`; a file inside a collapsed folder is revealed when selected by `FileSelected`, by `OpenFileDiff` from a browsed file, and by `ReferenceActivated` when it lands on a changed file (FR-008, one test each), and selecting its row keeps the diff; collapsing the folder holding the selected file keeps `selected` and the diff; a `list_read` keeps the collapsed folders, the selection when the file is still listed, and `list_offset` (FR-014, SC-005 for folds, edge case "The list changes while shown"); folds made while the list is reading or failed apply when it returns (edge case "Reading or failure"); two `OpenView`s on the same entry keep separate choices (FR-013); `tree_pane(view)` is `TreePane::Hidden` and `change_rows` empty for every non-`ListBody::Files` state, `Rows` otherwise (FR-016)
- [ ] T021 [P] [US1] Write failing tests in `crates/micold-client/tests/features_pr_changes.rs`: switching source working tree ↔ a pull request keeps `tree` and lays out the new source's files, a folder of the same path stays collapsed (edge case "Switching source")
- [ ] T022 [US1] Add `tree: TreeChoices` and `change_rows: Vec<Row>` to `OpenView`, `Msg::TreeFolderToggled(Vec<String>)`, the reveal call in the `FileSelected` arm (and in `ReferenceActivated` if it selects without `FileSelected`), the render-free `tree_pane(view) -> TreePane::{Hidden, Rows, NoMatch}` (data-model §6), and one `refresh_change_rows` step that every arm changing the list, the source or the choices calls (never `view`; research R9) in `crates/micold-client/src/features/changes.rs`, building `Leaf`s from `ChangedFile` (0 and 0 for binary, not text, Mode-only and pure rename) and an always-false reviewed predicate, and the bar scale (`bar_scale`) from the whole list, never the filtered rows (spec Assumptions), until T020 and T021 pass
- [ ] T023 [US1] Make `features::changes::tree_rows` an adapter over `review::tree::layout` with `chain: false` and `Openness::DefaultClosed { open: &FilesPane::open_folders }`, deleting the client's `Folder` walk, in `crates/micold-client/src/features/changes.rs`; the existing **Files** tree tests in `crates/micold-client/tests/features_changes.rs` pass unchanged (FR-017)

**Checkpoint**: `mise run gate` passes; the Changes list still draws its flat rows.

---

## Phase 5: User Stories 1 and 2 in the client — drawing the tree and the bar

**Goal**: the list pane draws the tree: folder rows with chevrons and totals, name-first file rows
with kind tags, the change bar and counts, hover text, indentation capped.

**Independent Test**: quickstart Part B steps 1, 2, 4, 6 at 1280×800 in both themes; the layout
snapshot and showcase completeness tests.

- [ ] T024 [P] [US2] Write the failing geometry test in `crates/micold-client/src/ui/material/change_bar.rs`: two adjacent segments at the `BarParts` lengths, total width never over `.max_width(px)`, nothing drawn for a zero part
- [ ] T025 [US2] Implement the shared `ChangeBar` component (builder `ChangeBar::new(parts, roles).max_width(px)`, `From<ChangeBar> for Element`, added and removed colours from design tokens per research R7) in `crates/micold-client/src/ui/material/change_bar.rs`, export it from `crates/micold-client/src/ui/material/mod.rs`, until T024 passes
- [ ] T026 [US2] Add `ChangeBar` samples (4:1, one large and one small, minimum one-line) to `crates/micold-client/src/showcase/sections/review.rs` so `crates/micold-client/tests/showcase_completeness.rs` passes
- [ ] T027 [US1] Write a failing unit test in `crates/micold-client/src/ui/changes.rs` that, with the row constants and the default list width recorded in T019, `max_indent_levels` leaves room for a 24-character name (SC-006); then draw `OpenView::change_rows` on `VirtualRows` in `crates/micold-client/src/ui/changes.rs` per contract §2.3: folder row (chevron, label, `+a −r` caption, press → `Msg::TreeFolderToggled(paths)`), file row (name emphasised and ellipsised, kind `Tag` for Added / Deleted / Renamed / Mode, Binary / Not text tag, `ChangeBar` and `+a −r` when `bar` is `Some`, unsent-comment tag when non-zero, `Tooltip` with `hover_text`, press → `Msg::FileSelected`, selected outline as today), indentation `indent_levels(depth, max_indent_levels(..))` steps; the empty, reading, failed and pull-request states unchanged, nothing drawn for the tree unless `tree_pane` is `Rows` (FR-016)
- [ ] T028 [US1] Re-record the layout-snapshot fixture for the changed Changes-list states and add a covered state `changes-tree-collapsed` (nested folders, one chained, one collapsed, a binary and a Mode row) in `crates/micold-client/tests/support/covered_states.rs`; `crates/micold-client/tests/layout_snapshot.rs` passes (if it fails on `main` itself for another thread's layout, note it and do not re-record that thread's states)
- [ ] T029 [US1] Rewrite *What the list shows* in `docs/user-guide/reviewing-changes.md` for the tree (folders, chained folders, collapsing, name-first rows, hover), the bar and folder totals (FR-018)
- [ ] T030 [US1] Run the visual pass (quickstart Part B steps 1, 2, 4, 6; 1280×800, light and dark) and save its evidence in `specs/1273-review-file-tree/visual-pass/`

**Checkpoint**: `mise run gate` passes; a reviewer sees the tree and bars in the Review view.

---

## Phase 6: User Stories 3 and 4 in the client — filter and sort

**Goal**: the filter box and the sort chips above the tree, wired to the core rules of Phase 3.

**Independent Test**: quickstart Part B steps 3, 5, 7; the reducer and harness tests below.

- [ ] T031 [P] [US3] Write the failing harness test in `crates/micold-client/src/ui/material/text_field.rs` with `crates/micold-client/src/ui/material/test_support.rs`: an Escape press on a focused field with `.on_escape(M)` publishes `M` once and is captured; on an unfocused field nothing (research R5)
- [ ] T032 [US3] Add the builder method `TextField::on_escape(M)` in `crates/micold-client/src/ui/material/text_field.rs` until T031 passes
- [ ] T033 [US3] Write failing reducer tests in `crates/micold-client/tests/features_changes.rs`: `Msg::TreeFilterChanged` narrows `change_rows` and keeps `selected` and the diff when the selected file is filtered out (scenario 3.4); `Msg::TreeFilterCleared` empties the filter, restores the pre-filter folds (scenario 3.2) and is a no-op when already empty; a fold made while filtering is gone after clear; a `list_read` keeps filter, sort and folds (SC-005); a source switch keeps filter and sort (FR-013); filter and sort chosen while the list is reading or failed apply when it returns; a filter matching nothing makes `tree_pane` `NoMatch` and keeps `selected` and the diff (scenario 3.3); typing a filter leaves the bar scale unchanged
- [ ] T034 [US4] Write failing reducer tests in `crates/micold-client/tests/features_changes.rs`: `Msg::TreeSortChosen(Size)` reorders `change_rows`; `TreeSortChosen(UnreviewedFirst)` is refused while `sort_options(false)` hides it; the sort survives a `list_read` (scenario 4.5)
- [ ] T035 [US3] Add `Msg::TreeFilterChanged(String)`, `Msg::TreeFilterCleared` and `Msg::TreeSortChosen(TreeSort)` to `crates/micold-client/src/features/changes.rs`, each calling the `TreeChoices` method and `refresh_change_rows`, until T033 and T034 pass
- [ ] T036 [US3] Draw above the tree in `crates/micold-client/src/ui/changes.rs` per contract §2.1, §2.2, §2.4: the filter `TextField` (placeholder `Filter files…`, leading search icon, trailing clear action when non-empty, `.on_escape(Msg::TreeFilterCleared)`), one `ToggleChip` per `sort_options(false)` entry (`Shown` active when chosen, `Disabled(reason)` disabled inside a `Tooltip`, `Hidden` not drawn; research R11), matched spans highlighted in row labels, and when `tree_pane` is `NoMatch` the line `No changed file matches the filter` with a **Clear filter** text button; filter box and chips drawn only when `tree_pane` is not `Hidden` (FR-016)
- [ ] T037 [US3] Add covered states `changes-tree-filter` (matches highlighted, Size chosen) and `changes-tree-no-match` in `crates/micold-client/tests/support/covered_states.rs` and record them; `crates/micold-client/tests/layout_snapshot.rs` passes
- [ ] T038 [US3] Add the filter box (matching as **Open file by path**, highlights, `Esc` and the clear control, the no-match line) and the sort control (Path, Size; **Unreviewed first** once reviewed marks exist) to *What the list shows* in `docs/user-guide/reviewing-changes.md`
- [ ] T039 [US3] Run the visual pass (quickstart Part B steps 3, 5, 7; 1280×800, light and dark) and save its evidence in `specs/1273-review-file-tree/visual-pass/`

**Checkpoint**: `mise run gate` passes; all four stories work in the Review view.

---

## Phase 7: Polish

- [ ] T040 Record quickstart Part A and Part B results under *Part B results* in `specs/1273-review-file-tree/quickstart.md` (close unit)

---

## Dependencies & Execution Order

- **Phase 1 → Phase 2 → Phase 3**: core only; Phase 3's groups (totals/bar, filter, sort) depend on
  Phase 2 and each other only through `layout` in one file, so their test tasks [P] can be written
  together, their implementations run in order T010, T012, T014, T017, then T018.
- **Phase 4** depends on Phase 3 **and on #1272 being merged** (T019 first).
- **Phase 5** depends on Phase 4. **Phase 6** depends on Phase 5 (it draws above the tree Phase 5
  draws).
- **Phase 7** is the close unit's.

## Parallel Opportunities

- T008 (unit tests in `tree.rs`) beside T007 (`tests/review_tree.rs`); the other Phase 3 test tasks share a file with one of them.
- T021 beside T020; T024 beside T027's preparation; T031 beside T033.

## Implementation Strategy

MVP is the core layout (Phases 1–2): it carries FR-017's one shared tree and every US1 rule, proven
against the **Files** browser's current rows. Phase 3 completes the core so the client phases only
wire and draw. The client work waits for #1272 and then lands as view state (Phase 4), drawing of the
tree and bar (Phase 5, stories 1 and 2 visible), and filter and sort (Phase 6, stories 3 and 4
visible).

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Shared tree layout in core 🎯 MVP

- **Tasks**: T001–T006
- **Deliverable**: `micold_core::review::tree::layout` lays out a path list as a tree — folders before
  files, chained single-child folders, default-open with a collapsed set or default-closed with an
  open set — and `TreeChoices` toggles and reveals folders; the **Files** browser's current rows are
  reproduced by core tests. No client change: the tree is drawn in M4, filter and sort in M5.
- **Satisfies**: US1 acceptance scenarios 1–3 (core); FR-001, FR-002, FR-003, FR-004 (state), FR-017
  (layout); SC-002; contract L1–L4, L12
- **Verify**: `mise run test-core` (`cargo test -p micold-core --test review_tree`)
- **Depends on**: — (mergeable before #1272)
- **Tier**: full

### M2 — Totals, bar maths, filter and sort in core

- **Tasks**: T007–T018
- **Deliverable**: the core tree computes folder totals, bar lengths, the indentation cap and hover
  text, filters with highlight spans and filter-time folds, orders by Path, Size and Unreviewed first
  against a reviewed predicate, decides the sort offer (`UNREVIEWED_UNTIL_MARKS = Hidden`), and lays
  out 5,000 files in under 100 ms. No client change: the tree is drawn in M4, filter and sort in M5.
- **Satisfies**: US2 scenarios 1–5, US3 scenarios 1, 2, 5, US4 scenarios 1–4 (core); FR-006, FR-007,
  FR-009, FR-010, FR-012, FR-017; SC-001, SC-003, SC-004; contract L5–L11
- **Verify**: `mise run test-core`; the `review_tree` timing test passes in release
  (`cargo test -p micold-core --release --test review_tree`)
- **Depends on**: M1 (mergeable before #1272). Over 10 tasks, kept whole: its tasks are one file's
  rules with small tests, about 600 lines, and a split by story leaves halves that change the same
  `layout` function.
- **Tier**: full

### M3 — Tree view state in the Changes reducer; Files on the core layout

- **Tasks**: T019–T023
- **Deliverable**: each open Changes view keeps its collapsed folders across re-reads and source
  switches, a file selected from elsewhere opens its folders, and the **Files** browser draws from
  `review::tree` with its rows unchanged; the list still looks as today (no half-wired UI): the tree is drawn in M4, filter and sort in M5.
- **Satisfies**: US1 scenarios 3, 4 (state); FR-003, FR-004, FR-008, FR-013, FR-014 (folds), FR-016,
  FR-017; SC-005 (folds)
- **Verify**: `mise run gate`; `cargo test -p micold-client --test features_changes --test
  features_pr_changes`
- **Depends on**: M2, **#1272 merged** (do not start before)
- **Tier**: full

### M4 — Draw the changed files as a tree with change bars

- **Tasks**: T024–T030
- **Deliverable**: the Review view's file list is a tree with collapsible (chained) folders and their
  totals, name-first file rows with kind tags, a green/red change bar with `+a −r`, the full path on
  hover and capped indentation; `ChangeBar` is in the showcase; the user guide describes it.
- **Satisfies**: US1 scenarios 1–5, US2 scenarios 1–5; FR-001–FR-008, FR-015, FR-016, FR-018 (tree,
  bar); SC-002, SC-006 (tree, bar)
- **Verify**: `mise run gate` (layout snapshot `changes-tree-collapsed`, showcase completeness, the
  `ChangeBar` geometry test); quickstart Part B steps 1, 2, 4, 6
- **Depends on**: M3 (after #1272)
- **Tier**: full

### M5 — Filter box and sort control

- **Tasks**: T031–T039
- **Deliverable**: above the tree a filter box narrows the files with highlighted matches, clears with
  `Esc` or its clear control and says when nothing matches; sort chips order each level by Path or
  Size; filter, sort and folds survive re-reads; the user guide describes both.
- **Satisfies**: US3 scenarios 1–6, US4 scenarios 1–5; FR-009–FR-014, FR-018 (filter, sort);
  SC-001, SC-005, SC-006
- **Verify**: `mise run gate` (layout snapshot `changes-tree-filter`, `changes-tree-no-match`; the
  `TextField` Escape harness test); quickstart Part B steps 3, 5, 7
- **Depends on**: M4 (after #1272)
- **Tier**: full

T040 (quickstart results) changes no code and is left to the close unit.
