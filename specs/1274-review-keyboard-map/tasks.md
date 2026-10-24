# Tasks: Keyboard Map for Reviewing Changes

**Input**: Design documents from `specs/1274-review-keyboard-map/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/keys.md](./contracts/keys.md),
[contracts/jump-to.md](./contracts/jump-to.md), [quickstart.md](./quickstart.md)

**Tests**: mandatory (Constitution I). Every test task comes before the implementation task it
covers and must fail first. Core tests are `#[cfg(test)] mod tests` in the module they cover;
client tests are integration tests under `crates/micold-client/tests/`.

**Documentation**: each milestone that ships a key or entry updates the user guide in its own PR
(Constitution VII).

**Cross-platform**: core has no `cfg(target_os)`; the one platform choice (primary modifier) is
`keymap::primary_held` in the client (Constitution VI).

**Blockers** (spec.md#dependencies): every task in a phase part marked **(M2)**, **(M3)** or
**(M4)** needs **#1269, #1270, #1271 and #1273 merged** on `main`; **(M4)** also needs **#1127
merged**. Tasks marked **(M1)** need no blocker and must not name a type from those issues
(research R1). Client file paths in M2–M4 tasks name today's modules (`features/changes.rs`,
`ui/changes.rs`); T012 and T052 re-point them at the blockers' modules once merged.

**D4 (provisional, ledger)**: keys are on by default with no Settings switch. If the user picks a
switch, `resolve` gains one argument and the sheet one filter (research R4); T004/T005 and T015/T016
change, nothing else.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: the user story of spec.md the task serves

---

## Phase 1: Setup (M1)

- [ ] T001 Declare `pub mod keys; pub mod navigate; pub mod jump;` in `crates/micold-core/src/review/mod.rs` and create `crates/micold-core/src/review/keys.rs`, `navigate.rs`, `jump.rs` with a module doc comment each (what rules the module holds, link to `specs/1274-review-keyboard-map/contracts/`)

---

## Phase 2: Foundational — key table and resolution (M1)

The table and `resolve` serve US1–US5; no story label.

- [ ] T002 Write failing tests in `crates/micold-core/src/review/keys.rs` for contract K1 and K4: `REVIEW_KEYS` has exactly 12 rows in K1's order; no two rows share `(key, mods)`; `action` is `None` only for the save-comment row (`Enter`, primary, no shift); `repeats` is true exactly for `j k n p ] [`; each `sheet` text equals K1's verbatim; `labels()` returns the char itself in both forms for single keys, (`Ctrl+Shift+Enter`, `Cmd+Shift+Enter`) for `Send` and (`Ctrl+Enter`, `Cmd+Enter`) for save-comment
- [ ] T003 Implement `ReviewAction` (the 11 variants of data-model.md), `KeyName { Char(char), Enter }`, `KeyMods { primary: bool, shift: bool }`, `ReviewKey { key, mods, action: Option<ReviewAction>, repeats: bool, sheet: &'static str }`, `pub const REVIEW_KEYS: [ReviewKey; 12]` and `ReviewKey::labels() -> (String, String)` in `crates/micold-core/src/review/keys.rs` so T002 passes; document `REVIEW_KEYS` as public and stable for #1276's guide test
- [ ] T004 Write failing tests in `crates/micold-core/src/review/keys.rs` for contract K2 and K3: every row with an action resolves to it under `Holder::Review`; `Holder::Other` gives none for every row; `Holder::Sheet` gives `ToggleSheet` only for unmodified non-repeat `?`; `Holder::Composer` gives `Send` only for the send chord, not repeat, and none for `j`; Ctrl, Alt or logo held alone refuses a single key; Ctrl+Alt without logo (AltGr) accepts `[`; Shift is not compared for chars (`?` with Shift acts) and `J` is unbound; `PressKey::Other` gives none; a repeat acts only for `repeats` rows; the send chord needs `primary` and Shift, and refuses Alt or the non-primary of Ctrl/logo; primary+Enter without Shift resolves to none (the composer keeps its save)
- [ ] T005 Implement `PressKey { Char(char), Enter, Other }`, `Press { key, primary, ctrl, alt, logo, shift, repeat }`, `Holder { Review, Composer, Sheet, Other }` and `resolve(press: &Press, holder: Holder) -> Option<ReviewAction>` in `crates/micold-core/src/review/keys.rs` matching only from `REVIEW_KEYS`, so T004 passes

**Checkpoint**: `mise run test-core` runs `review::keys` green.

---

## Phase 3: User Story 1 — Move through files and hunks (P1) 🎯

**Goal**: `j`/`k` step files, `n`/`p` step hunks across files, the current hunk is marked.

**Independent test**: spec.md US1 *Independent Test*; quickstart §B B1–B4.

### Core (M1)

- [ ] T006 [US1] Write failing tests in `crates/micold-core/src/review/navigate.rs` for `has_hunks` and `step_file`: `has_hunks` is true for `Content::Text` with `added + removed > 0`, false for binary, a pure rename (0 lines changed) and a large file not shown, true for a large file shown; `step_file` follows US1 s1 (`a`→`b`), s2 (wraps both ends), s5 (visits only the order passed, as a filtered tree gives), no selection (`j` first, `k` last), a selection no longer in the order (as no selection), empty order → `None`
- [ ] T007 [US1] Implement `NavFile { path: RelPath, has_hunks: bool }`, `has_hunks(&ChangedFile, large_shown: bool) -> bool` and `step_file(order, selected, dir) -> Option<RelPath>` reusing `crate::navigation::step` in `crates/micold-core/src/review/navigate.rs`, so T006 passes
- [ ] T008 [US1] Write failing tests in `crates/micold-core/src/review/navigate.rs` for `step_hunk` and `resolve_edge` (research R5): no current hunk → `n` `Current(0)`, `p` `Current(count − 1)`; inside a file → next/previous index; past the last hunk → `File { next file with hunks, First }`, before the first → `File { previous file with hunks, Last }`; files with `has_hunks: false` are passed over; wrapping at the ends of the order; wrapping back to the selected file itself gives `Current`; `SelectedDiff::Loading` and `Failed` → `Nothing`; no selection → `File { first with hunks, First }` / `File { last with hunks, Last }`; no file has hunks → `Nothing`; `resolve_edge(First, n) = Some(0)`, `resolve_edge(Last, n) = Some(n − 1)`, `resolve_edge(_, 0) = None`
- [ ] T009 [US1] Implement `SelectedDiff { Loading, Failed, Hunks(usize) }`, `Edge { First, Last }`, `HunkMove { Current(usize), File { path: RelPath, edge: Edge }, Nothing }`, `step_hunk(order, selected, diff, current, dir) -> HunkMove` and `resolve_edge(edge, count) -> Option<usize>` in `crates/micold-core/src/review/navigate.rs`, so T008 passes
- [ ] T010 [P] [US1] Write failing tests in `crates/micold-core/src/review/diff.rs` (existing `mod tests`, its `shaped` helper) for `header_rows`: a diff with two hunks separated by a gap gives the row index of each hunk header in hunk order, for the unified layout and for the side-by-side layout; a diff with no hunks gives an empty list
- [ ] T011 [US1] Implement `header_rows(shaped, layout: settings::DiffLayout) -> Vec<usize>` in `crates/micold-core/src/review/diff.rs` over the existing `unified_rows` / `side_by_side_rows` (research R9), so T010 passes

### Client (M2 — needs #1269, #1270, #1271, #1273 merged)

- [ ] T012 [US1] Re-read the merged #1269 (Review tab view module), #1270 (diff header, overflow menu), #1271 (**Mark reviewed** message, reviewed store) and #1273 (file tree, collapsed directories); update the file paths and message names of T015–T047 in `specs/1274-review-keyboard-map/tasks.md` and the D4–D7 names in `plan.md` to the merged code, and record what changed in the ledger
- [ ] T013 [P] [US1] Write failing tests in `crates/micold-client/tests/keymap.rs` for contract K5: `primary_held` is `logo` on macOS and `ctrl` elsewhere (`cfg!` arms); `review_press` maps produced text `"j"` to `PressKey::Char('j')`, Enter to `PressKey::Enter`, a dead key or no text to `PressKey::Other`, and copies `ctrl`, `alt`, `logo`, `shift`, `repeat` and `primary`
- [ ] T014 [US1] Implement `primary_held(Mods) -> bool` and `review_press(&KeyInput, repeat: bool) -> Press` in `crates/micold-client/src/keymap.rs` (no key matched there), so T013 passes
- [ ] T015 [US1] Write failing tests in `crates/micold-client/tests/review_keys.rs` (new) for the holder (research R7, FR-002, SC-002): `Review` with the Review tab shown and nothing else holding the keyboard (also with Tab focus on a Review tab button); `Composer` with the comment composer focused; `Sheet` with the shortcut sheet topmost; `Other` with the tree filter or any text field focused, a focused terminal, Settings, Overview, the quick-switcher, or another registered overlay topmost (US1 s6)
- [ ] T016 [US1] Implement the render-free holder function beside the Review tab state in `crates/micold-client/src/features/changes.rs` (or #1269's module per T012), reading `window.focused_field`, `overlay::registry::topmost`, quick-switcher, Settings, Overview and terminal focus, so T015 passes
- [ ] T017 [US1] Write failing tests in `crates/micold-client/tests/review_keys.rs` for the US1 effects of contract K7: `NextFile`/`PrevFile` select through the one `select_file` path (diff from the top, row brought into view, collapsed directory expanded, `current_hunk` and `pending_edge` cleared); a tree click also clears `current_hunk`; `NextHunk`/`PrevHunk` with `Current(i)` set `current_hunk` and scroll the header to the top; with `File` select the file and keep `pending_edge`, resolved by `resolve_edge` on a `DiffRead` of that path only (a `DiffRead` of another path leaves it); `Nothing` changes nothing; picked lines, composer text, whole-file and unified/side-by-side choices are unchanged (FR-004); a hidden Review tab of another entry is untouched (FR-014); on a 200-file, 500-comment review each of these arms completes under 100 ms (SC-003; T033 adds `]` `[` `c`, T041 `?`)
- [ ] T018 [US1] Add `current_hunk: Option<usize>` and `pending_edge: Option<(RelPath, Edge)>` to the Review tab view state, one `select_file` path used by tree click and keys, and the `NextFile`/`PrevFile`/`NextHunk`/`PrevHunk` reducer arms reusing `jump_offset`'s arithmetic and the pending-jump flag, in `crates/micold-client/src/features/changes.rs` (or #1269's module), so T017 passes; `ToggleReviewed`/`NextUnreviewed` follow in T025, and comment, send and sheet actions are ignored until M3
- [ ] T019 [US1] Add `review_key_listener` in `crates/micold-client/src/ui/mod.rs`: a `keyboard::listen()` subscription held only while the holder is `Review` or `Sheet`, mapping `KeyPressed { text, modifiers, repeat, .. }` → `keymap::review_press` → `review::keys::resolve` → one Review-tab message (contract K6); view and subscription glue, its decisions tested by T013/T015/T017 and the glue by quickstart §B B1–B4 and B13
- [ ] T020 [US1] Draw the current hunk's header row (row from `review::diff::header_rows`) with a theme-role accent in `crates/micold-client/src/ui/changes.rs` (or #1270's diff header), visible in both themes (FR-005; drawing covered by T021's snapshot and quickstart B4); add previous/next hunk icon buttons (shared icon button) sending the `PrevHunk`/`NextHunk` messages if #1270/#1273 did not add them; their tooltips name `p`/`n` from `ReviewKey::labels` (FR-013)
- [ ] T021 [US1] Add layout snapshot state `review-current-hunk` in `crates/micold-client/tests/support/covered_states.rs` and list it in `docs/development/layout-snapshot.md` (FR-021)

**Checkpoint**: quickstart §B B1–B4 and B13 pass by hand.

---

## Phase 4: User Story 2 — Mark progress and jump to what is left (P1)

**Goal**: `v` toggles the reviewed mark, `g` selects the next unreviewed file.

**Independent test**: spec.md US2 *Independent Test*; quickstart §B B5–B6.

### Core (M1)

- [ ] T022 [US2] Write failing tests in `crates/micold-core/src/review/navigate.rs` for `next_unreviewed`: US2 s3 (files 1–5, 2 and 4 unreviewed, from 1: 2, 4, 2), s4 (all reviewed → `AllReviewed`), s5 (the predicate is read at the call, so a file marked elsewhere is skipped), no selection → first unreviewed, the selected file the only unreviewed → `Select(selected)`, empty order → `Nothing`; `ALL_FILES_REVIEWED == "All files reviewed"`
- [ ] T023 [US2] Implement `Unreviewed { Select(RelPath), AllReviewed, Nothing }`, `next_unreviewed(order, selected, is_reviewed: impl Fn(&RelPath) -> bool) -> Unreviewed` and `pub const ALL_FILES_REVIEWED` in `crates/micold-core/src/review/navigate.rs`, so T022 passes

### Client (M2 — needs #1269, #1270, #1271, #1273 merged)

- [ ] T024 [US2] Write failing tests in `crates/micold-client/tests/review_keys.rs`: `ToggleReviewed` sends #1271's **Mark reviewed** message for the selected file only and nothing without a selection (US2 s1–s2); `NextUnreviewed` with `Select` goes through `select_file`, with `AllReviewed` sets the note to `ALL_FILES_REVIEWED` and keeps the selection (US2 s3–s4); a mark from another window is respected (US2 s5); on a 30-file review, `j` then `v` per file (2 presses each) leaves every file marked reviewed (SC-001; T038 adds the send chord)
- [ ] T025 [US2] Implement the `ToggleReviewed` and `NextUnreviewed` reducer arms in `crates/micold-client/src/features/changes.rs` (or #1269's module) and the **Mark reviewed** tooltip naming `v` from `ReviewKey::labels` (FR-013), so T024 passes
- [ ] T026 [US2] Add the section *Reviewing from the keyboard* to `docs/user-guide/reviewing-changes.md`: `j` `k` `n` `p` `v` `g`, the current-hunk mark, when the keys act (not in a text field or terminal), wrapping, files without hunks (FR-020)
- [ ] T027 [US2] Run the `visual-pass` skill (through an `autopilot-worker`) for quickstart §B B1–B6, B12 (hunk and **Mark reviewed** tooltips) and B13 at 1280×800, light and dark; save the evidence under `specs/1274-review-keyboard-map/visual-pass/` (FR-021)

---

## Phase 5: User Story 3 — Step through comments and write one (P2)

**Goal**: `]`/`[` visit comments in comment order; `c` opens the composer.

**Independent test**: spec.md US3 *Independent Test*; quickstart §B B7–B8.

### Core (M1)

- [ ] T028 [US3] Write failing tests in `crates/micold-core/src/review/navigate.rs` for `comment_order` and `step_comment`: order is file index, then line, then `created`, then id, with comments of unlisted files last by path; US3 s1 (`a.rs:3`, `a.rs:20`, `c.rs:7`; `select` is `Some(c.rs)` for the third, `None` within the selected file; fourth `]` wraps), s2 (`[` goes back), s3 (`in_diff: false` → `Show::Drawer`); a comment of an unlisted file → `Drawer` with `select: None`; no comments → `None`; `last` no longer present → as no last (first / last)
- [ ] T029 [US3] Implement `CommentFact { id, path, line, created, in_diff }`, `comment_order(facts, order) -> Vec<CommentFact>`, `Show { Card, Drawer }`, `CommentMove { id, select: Option<RelPath>, show }` and `step_comment(ordered, order, selected, last, dir) -> Option<CommentMove>` reusing `navigation::step` in `crates/micold-core/src/review/navigate.rs`, so T028 passes
- [ ] T030 [US3] Write failing tests in `crates/micold-core/src/review/navigate.rs` for `hunk_change_range` and `comment_target`: a hunk with added lines → new side, first..last added line; a remove-only hunk → old side, first..last removed line; a hunk with neither → `None`; `comment_target` precedence `composer_open` → `FocusComposer`, then `picked` → `Picked`, then a current hunk → `Hunk { side, range }`, else `Refused` (US3 s4–s6, FR-009); `PICK_OR_HUNK == "Pick lines or move to a hunk (n) to comment"`
- [ ] T031 [US3] Implement `hunk_change_range(&Hunk) -> Option<(Side, LineRange)>`, `CommentOpen { FocusComposer, Picked, Hunk { side, range }, Refused }`, `comment_target(composer_open, picked, current: Option<&Hunk>) -> CommentOpen` and `pub const PICK_OR_HUNK` in `crates/micold-core/src/review/navigate.rs`, so T030 passes
- [ ] T032 [US3] Add a timing test in `crates/micold-core/src/review/navigate.rs`: on 200 files and 500 comments, one `step_file`, `step_hunk`, `comment_order` + `step_comment` and `next_unreviewed` together take under 50 ms (SC-003, plan Performance Goals)

### Client (M3 — needs M2 merged)

- [ ] T033 [US3] Write failing tests in `crates/micold-client/tests/review_keys.rs`: `NextComment`/`PrevComment` select the file when `select` is set, `Card` sets the existing pending jump to that card, `Drawer` opens the drawer on that comment, `last_comment` holds the comment reached; `Comment` with `FocusComposer` focuses the open composer and keeps its text, `Picked` sends **Add comment**'s message, `Hunk` picks that range then sends it, `Refused` sets the note to `PICK_OR_HUNK`; the SC-003 timing of T017 holds for these arms
- [ ] T034 [US3] Add `last_comment: Option<CommentId>` and the `NextComment`/`PrevComment`/`Comment` reducer arms in `crates/micold-client/src/features/changes.rs` (or #1269's module), building `CommentFact`s with the view's existing jump check for `in_diff`, so T033 passes

---

## Phase 6: User Story 4 — Send the review from the keyboard (P2)

**Goal**: Ctrl+Shift+Enter (Cmd+Shift+Enter on macOS) sends as **Send review (n)** does.

**Independent test**: spec.md US4 *Independent Test*; quickstart §B B9–B10.

### Client (M3 — needs M2 merged)

- [ ] T035 [P] [US4] Write failing tests in `crates/micold-client/tests/features_composer.rs`: Ctrl/Cmd+Enter saves the comment; with Shift also held it does not; a `TextArea` built with `.on_press(..)` emits the message for each press
- [ ] T036 [US4] Add the `.on_press(..)` builder option to `crates/micold-client/src/ui/material/text_area.rs` and exclude Shift from its Ctrl/Cmd+Enter submit, so T035 passes
- [ ] T037 [P] [US4] Write a failing test in `crates/micold-client/tests/keymap.rs` (FR-011): the `Send` row's chord, in both platform forms, equals no reserved chord of `specs/803-keyboard-navigation/spec.md#FR-006` (macOS, Windows, GNOME lists) and no existing app chord in `keymap.rs`
- [ ] T038 [US4] Write failing tests in `crates/micold-client/tests/review_keys.rs`: `Send` under `Review` has the effect of `Msg::SendPressed` (US4 s1); no drafts or a send under way → nothing (s2–s3); the chord in the composer (`Holder::Composer` via `.on_press`) sends only saved drafts and leaves the composer open, focused, with its text (s4, SC-006); after T024's 30-file walk, one chord sends (SC-001)
- [ ] T039 [US4] Wire the composer in `crates/micold-client/src/ui/changes.rs` with `.on_press` → `resolve(_, Holder::Composer)` → `Msg::SendPressed`, add the `Send` reducer arm, and the **Send review (n)** tooltip naming the chord from `ReviewKey::labels` (FR-013), so T037–T038 pass
- [ ] T040 [US4] Document the send chord, both platform forms: the row in `docs/user-guide/terminal-panes.md#keyboard-shortcuts`, the bullet in `docs/user-guide/worktrees-and-sessions.md#interacting-with-the-terminal`, and the chord in *Reviewing from the keyboard* of `docs/user-guide/reviewing-changes.md` (FR-020)

---

## Phase 7: User Story 5 — Find the keys (P2)

**Goal**: `?` and the overflow item open a shortcut sheet listing `REVIEW_KEYS`.

**Independent test**: spec.md US5 *Independent Test*; quickstart §B B11–B12.

### Client (M3 — needs M2 merged)

- [ ] T041 [US5] Write failing tests in `crates/micold-client/tests/overlay_registry.rs` (`ShortcutSheet` identity, `Layer::Dialog`, Escape and outside click close it) and in `crates/micold-client/tests/review_keys.rs` (`ToggleSheet` opens and closes it; while open the holder is `Sheet` and `j` changes nothing; the overflow item's message opens the same sheet; `?` meets the SC-003 timing of T017) (FR-012)
- [ ] T042 [US5] Add `ShortcutSheet` (`floating_surface!` on `Layer::Dialog`, `Registered`) with its open/close messages in the Review tab feature module, its `REGISTERED` line in `crates/micold-client/src/overlay/registry.rs`, the `ToggleSheet` reducer arm and the **Keyboard shortcuts** item in #1270's overflow menu, so T041 passes
- [ ] T043 [US5] Add the view `crates/micold-client/src/ui/shortcut_sheet.rs`: every `REVIEW_KEYS` row in table order, the running platform's label from `ReviewKey::labels` and the `sheet` text, built from `ui/material` parts; add the sheet to `crates/micold-client/src/showcase/sections/review.rs`
- [ ] T044 [US5] Write a failing docs test in `crates/micold-client/tests/keymap.rs` (SC-004): `docs/user-guide/reviewing-changes.md` names every `REVIEW_KEYS` row's labels (both forms) and `sheet` text
- [ ] T045 [US5] Complete *Reviewing from the keyboard* in `docs/user-guide/reviewing-changes.md`: the full key table from `REVIEW_KEYS` (`]` `[` `c` `?`, send chord, save-comment chord), the shortcut sheet and the overflow item, so T044 passes (FR-020)
- [ ] T046 [US5] Add layout snapshot state `review-shortcut-sheet` in `crates/micold-client/tests/support/covered_states.rs` and list it in `docs/development/layout-snapshot.md` (FR-021)
- [ ] T047 [US5] Run the `visual-pass` skill (through an `autopilot-worker`) for quickstart §B B7–B12 and B15 at 1280×800, light and dark; save the evidence under `specs/1274-review-keyboard-map/visual-pass/` (FR-021)

---

## Phase 8: User Story 6 — Jump to a file or comment by name (P3)

**Goal**: the jump-to field lists the review's files and comments.

**Independent test**: spec.md US6 *Independent Test*; quickstart §B B14.

### Core (M1)

- [ ] T048 [US6] Write failing tests in `crates/micold-core/src/quick_switch.rs` (existing `mod tests`; its existing tests untouched) for contract J2–J3: a candidate with `detail: None` ranks exactly as before (FR-019); a `name` match sorts before any `detail`-only match whatever their `Rank` (`Via::Name < Via::Detail`, FR-017); `detail` is matched only when `name` misses; spans index `name` for `Via::Name` and `detail` for `Via::Detail`
- [ ] T049 [US6] Add `EntryKey::ReviewFile(RelPath)`, `EntryKey::ReviewComment(CommentId)`, `Candidate.detail: Option<String>`, `Via { Name, Detail }`, `Ranked.via` and the `(via, rank, index)` sort to `filter` in `crates/micold-core/src/quick_switch.rs`; add `detail: None` to the existing `Candidate` literals in `crates/micold-client/src/features/quick_switch.rs::candidates` (no behaviour change), so T048 passes
- [ ] T050 [US6] Write failing tests in `crates/micold-core/src/review/jump.rs` for contract J1: a file candidate has `name` = file name, `detail` = full path, key `ReviewFile`; a comment candidate has `name` = its text with lines joined by spaces, `detail` = its file path, key `ReviewComment`; files come in file order, then comments in `navigate::comment_order`; each `ReviewEntry` carries the reviewed state, or status, path, line and first text line; on 200 files with distinct names, `filter` with a 4-character distinguishing part of a file's name ranks that file within the first 3 rows (SC-005: at most 2 Up/Down presses)
- [ ] T051 [US6] Implement `ReviewEntry` and `review_candidates(files: &[RelPath], is_reviewed, comments: &[ReviewComment]) -> Vec<(Candidate, ReviewEntry)>` in `crates/micold-core/src/review/jump.rs`, so T050 passes

### Client (M4 — needs #1127 and M3 merged)

- [ ] T052 [US6] Re-read the merged #1127 (title bar jump-to field) and update T053–T057's paths and message names in `specs/1274-review-keyboard-map/tasks.md` and D8 in `plan.md`; record what changed in the ledger
- [ ] T053 [US6] Write failing tests in `crates/micold-client/tests/quick_switch.rs` for contract J4–J5: review entries listed only when the current entry has a Review tab (shown or hidden), after the active project's locations and sessions and before other projects (US6 s1, s5); choosing `ReviewFile` shows the Review tab and selects through `select_file` (s3); choosing `ReviewComment` applies `step_comment`'s effect for that id (s4); neither changes the location, session or project; a key no longer listed does nothing (FR-018)
- [ ] T054 [US6] Insert `review_candidates` for the current entry's Review tab in `crates/micold-client/src/features/quick_switch.rs::candidates` and resolve the two keys in the choice to a Review-tab step, so T053 passes
- [ ] T055 [US6] Show the reviewed state on file rows and status, `path:line` and first text line on comment rows in the quick-switcher's row view (`crates/micold-client/src/ui/quick_switch.rs` or #1127's), with `Via` spans emphasising the matched text (FR-016)
- [ ] T056 [US6] Add the jump-to entries to *Reviewing from the keyboard* in `docs/user-guide/reviewing-changes.md` (FR-020)
- [ ] T057 [US6] Run the `visual-pass` skill (through an `autopilot-worker`) for quickstart §B B14 at 1280×800, light and dark; save the evidence under `specs/1274-review-keyboard-map/visual-pass/`

---

## Phase 9: Polish (close unit)

- [ ] T058 Record quickstart §C (macOS Cmd+Shift+Enter, Windows German layout `[` `]` `?`, focused terminal unchanged) in `specs/1274-review-keyboard-map/quickstart.md`, from CI and a manual run where available

---

## Dependencies & Execution Order

- **Phase 1 → Phase 2 → core parts of Phases 3–8** (all M1). Core parts of US1, US2, US3 share
  `navigate.rs` and run in order T006–T009, T022–T023, T028–T032; T010–T011 (`diff.rs`) and
  T048–T051 (`quick_switch.rs`, `jump.rs`; T050 needs T029's `comment_order`) are independent of
  them.
- **Blockers**: client parts of US1–US5 (M2, M3) wait for **#1269, #1270, #1271, #1273** merged;
  the client part of US6 (M4) also for **#1127**. M1 waits for nothing.
- Within the client: T012 first; T013–T019 (press, holder, state, listener) before every other
  client task; US3–US5 client (M3) after M2; US6 client (M4) after M3 (choosing a comment reuses
  T034's effect).

### Parallel opportunities

- T010 with T006–T009 (different files); T048 with T006–T009.
- T013 with T015 (different test files); T035 with T037.

## Implementation Strategy

Core first (M1) merges while the blockers are open and gives #1276 `REVIEW_KEYS`. Each later
milestone ships a user-visible slice once its blockers are on `main`: moving and marking (M2),
comments, send and the sheet (M3), jump-to (M4).

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Core review keys and navigation rules 🎯 MVP

- **Tasks**: T001–T011, T022–T023, T028–T032, T048–T051
- **Deliverable**: `micold_core::review::keys::REVIEW_KEYS` and `resolve`, the navigation rules of `review::navigate`, `review::diff::header_rows`, `review::jump::review_candidates` and the `quick_switch` detail ranking are on `main`, public and covered by unit tests; no client behaviour changes
- **Satisfies**: contract K1–K4, J1–J3; core of FR-001, FR-002, FR-003, FR-004, FR-006, FR-008, FR-009, FR-010, FR-012, FR-015, FR-016, FR-017, FR-019; SC-003, SC-005 (core bounds); US1 s1–s5, US2 s3–s5, US3 s1–s6 at the rule level
- **Verify**: `mise run test-core` — `review::keys`, `review::navigate`, `review::jump`, `review::diff` and `quick_switch` tests green, 968's existing `quick_switch` tests unchanged (quickstart §A, M1 row)
- **Depends on**: — (no blocker)
- **Tier**: full

### M2 — Move and mark from the keyboard

- **Tasks**: T012–T021, T024–T027
- **Deliverable**: in the Review tab, `j`/`k` step files, `n`/`p` step and mark hunks across files, `v` toggles the reviewed mark and `g` selects the next unreviewed file; keys never act in a text field or terminal; documented in the user guide
- **Satisfies**: US1 s1–s6, US2 s1–s5; FR-001 (those keys), FR-002, FR-003, FR-004, FR-005, FR-007, FR-008, FR-013 (hunk controls, **Mark reviewed**), FR-014, FR-015, FR-020 (those keys), FR-021 (current hunk); SC-002
- **Verify**: `mise run gate` (`tests/keymap.rs`, `tests/review_keys.rs`, layout snapshot); quickstart §B B1–B6, B12 (hunk, **Mark reviewed**), B13 via visual-pass
- **Depends on**: M1; **blocked by #1269, #1270, #1271, #1273 merged**
- **Tier**: full

### M3 — Comments, send chord and shortcut sheet

- **Tasks**: T033–T047
- **Deliverable**: `]`/`[` visit comments, `c` opens the composer, Ctrl/Cmd+Shift+Enter sends the review (also from the composer), `?` and the overflow item open the shortcut sheet listing every key; the user guide lists every key and the chord
- **Satisfies**: US3 s1–s6, US4 s1–s4, US5 s1–s5; FR-001 (`]` `[` `c` `?`), FR-006, FR-009, FR-010, FR-011, FR-012, FR-013 (**Send review**), FR-020, FR-021 (sheet); SC-001, SC-004, SC-006
- **Verify**: `mise run gate` (`tests/features_composer.rs`, `tests/keymap.rs` incl. the docs test, `tests/review_keys.rs`, `tests/overlay_registry.rs`, layout snapshot); quickstart §B B7–B12, B15 via visual-pass
- **Depends on**: M2; **blocked by #1269, #1270, #1271, #1273 merged**
- **Tier**: full

### M4 — Jump to review files and comments

- **Tasks**: T052–T057
- **Deliverable**: the jump-to field lists the current entry's review files and comments after the active project's locations and sessions, and choosing one shows it in the Review tab
- **Satisfies**: US6 s1–s5; FR-016, FR-017, FR-018, FR-019, FR-020 (jump-to); SC-005
- **Verify**: `mise run gate` (`tests/quick_switch.rs`); quickstart §B B14 via visual-pass
- **Depends on**: M3; **blocked by #1127 merged**
- **Tier**: full

T058 (quickstart §C record, no code) is left to the close unit.
