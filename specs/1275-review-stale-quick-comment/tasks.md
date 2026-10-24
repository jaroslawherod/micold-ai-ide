---

description: "Task list for #1275 Review UI — stale-awareness and quick comment"
---

# Tasks: Review UI — Stale-Awareness and Quick Comment

**Input**: Design documents from `specs/1275-review-stale-quick-comment/` (plan.md, spec.md,
research.md R1–R11, data-model.md §1–§14, contracts/core-api.md, contracts/changes-view-hold.md,
contracts/quick-comment.md, quickstart.md)

**Tests**: Mandatory (Constitution I). Every test task comes before the implementation it covers
and must fail first. Rule ids (I1–I8, S1–S10, D1–D8, K1–K5, H1–H16, Q1–Q12) are the contracts'.

**Documentation**: each client milestone updates the user guide, the layout snapshot and runs a
visual pass in the same PR (FR-029, Constitution VII). The core milestones change nothing a user
sees: their PRs carry the `docs-not-needed` label.

**Gating**: M1 and M2 (micold-core) build now. **M3, M4 and M5 must not start until #1269 (Review
tab) and #1271 (reviewed marks) are both merged** (research R11): #1269 reshapes the
Changes/Review view they change. **M6 must not start until #1271 is merged.** A unit that picks up a
gated milestone first checks both issues' PRs are `MERGED` and otherwise escalates.

**FR-014** is planned as the whole-view hold (provisional A, ledger *Open escalation*). If the user
picks B or C, only M3's tasks change (plan *Delivery order*); M1's `changed_since` serves all three.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: US1 stale-awareness, US2 drag to comment, US3 suggestion chips

## Phase 1: Setup

No setup tasks: existing workspace, no new crate and no new dependency (plan *Technical Context*).

---

## Phase 2: Foundational — change identity in the list read (FR-001a)

**Purpose**: every change-list row carries a `ChangeIdentity`; US1's stale set compares it.

- [ ] T001 [P] Write failing unit tests I1 and I2 in the `mod tests` of
  `crates/micold-core/src/review/changes.rs`: `parse_name_status` reads the old and new ids from a
  `--raw` record, an id of all zeros (40 `0`s, as `--no-abbrev` prints it) is `None`; `assemble`
  sets `ChangeIdentity::Sides { old, new }` with `VersionId::Object(id)` for a present id and
  `VersionId::Absent` for an added file's old side or a deleted file's new side; a rename keeps both
  ids.
- [ ] T002 Implement in `crates/micold-core/src/review/changes.rs` (data-model §1–§3):
  `VersionId { Unknown (default), Absent, Object(String), Stat { bytes: u64, modified_ns: u128 } }`
  and `ChangeIdentity { Unknown (default), Sides { old: VersionId, new: VersionId }, Patch(u64) }`,
  structural `PartialEq`, serde; `ChangedFile.identity: ChangeIdentity` with `#[serde(default)]`;
  `NameStatus.old_id`/`new_id: Option<String>` ("`None` for an id of all zeros"); `assemble` sets
  `Sides`, leaving a working-tree new side with no id for `git.rs` to fill. Add
  `identity: ChangeIdentity::Unknown` to every other `ChangedFile` literal:
  `crates/micold-core/src/context_header.rs`, `crates/micold-core/src/runs/summary.rs`,
  `crates/micold-core/src/review/pr_diff.rs`, the test helpers in
  `crates/micold-core/src/review/changes.rs` and `crates/micold-client/tests/features_changes.rs`
  (grep `ChangedFile {` first; LSP `findReferences` on the struct).
- [ ] T003 [P] Write failing real-git tests I1a, I3, I4, I5, I6 and I7 in
  `crates/micold-core/tests/review_git.rs`: full-length ids (length of `git rev-parse HEAD`);
  `BaseToHead` both sides `Object` with no `hash-object`; an equal-counts re-edit changes the
  identity (US1 AS12) and editing back restores it (AS4), a stat-clean file equals the same content
  hashed; an untracked file is hashed; a file over `limits::MAX_VERSION_BYTES` is `Stat` and a touch
  changes it; at most one `hash-object` process per `change_list` read (count it through the
  existing git-runner seam, or by a `GIT_TRACE` log); `change_list_of(.., &[p])` returns exactly
  `change_list`'s row for `p` (for a rename given `[new, from]`, the renamed row) or no row.
- [ ] T004 Implement in `crates/micold-core/src/review/git.rs` (data-model §4, research R1):
  `range_rows` and `range_paths` add `--no-abbrev` to the `--raw` read (name-only reads unchanged);
  private `identify(dir, files: &mut [ChangedFile])` runs one `git hash-object --stdin-paths` (no
  `-w`) over the working-tree new sides with no id that are hashable ("regular file, ≤
  `limits::MAX_VERSION_BYTES`, no newline in the path"), gives the others `Stat` from
  `symlink_metadata`, a missing file `Absent`; `change_list` calls it for `BaseToWorktree` and the
  uncommitted range after `merge_untracked`; add `GitCli::change_list_of(dir, scope, toggles,
  paths: &[RelPath]) -> io::Result<ChangeList>` (git pathspec, `ls-files` limited the same way) and
  make `change_list` call it with no paths.
- [ ] T005 [P] Write failing test I8 in `crates/micold-core/tests/pr_diff_split.rs`: two file
  sections differing in any byte get different `ChangeIdentity::Patch` values; equal sections, equal
  values.
- [ ] T006 Implement in `crates/micold-core/src/review/pr_diff.rs`: `split_pr_diff` sets
  `PrFile.file.identity = ChangeIdentity::Patch(hash)` of the file's section bytes with
  `std::collections::hash_map::DefaultHasher` (in memory only, as `core::store`).

**Checkpoint**: `mise run test-core` green; no visible change.

---

## Phase 3: User Story 1 core — the stale set and the held snapshot (FR-001–FR-004, FR-028) 🎯 MVP

**Goal**: `review::stale` decides which files changed since the shown list and holds the latest
read. **Independent test**: `mise run test-core` runs S1–S10.

- [ ] T007 [P] [US1] Create `crates/micold-core/src/review/stale.rs` (declared by T008, which
  owns the `review/mod.rs` edit) with failing unit tests S1–S10 from
  contracts/core-api.md in its `mod tests`: equal lists → empty, `observe` → `Same`; kind, `added`,
  `removed`, identity differences count, `content`/`large`/`origin` do not; added and removed paths
  count; rename counted once under its new path; count = set size and stable over repeats;
  `Held::observe` drops a file whose row is equal again and returns `Same` at empty; `apply`;
  `adopt` with and without the path in `fresh` (AS10, AS11); `PrDiff`'s `put` copies the patch and
  keeps GitHub's order, `ChangeList`'s keeps path order.
- [ ] T008 [US1] Add `pub mod stale;` to `crates/micold-core/src/review/mod.rs` and implement
  `review::stale` (data-model §6): `changed_since(shown, latest) ->
  BTreeSet<RelPath>`; `trait Listing: Clone { rows, put }` with impls for `ChangeList` and `PrDiff`;
  `Held<L: Listing> { latest, changed }` with private fields, existing only with `changed`
  non-empty; `enum Observed<L> { Same(L), Held(Held<L>) }`; `observe(shown, latest)`;
  `Held::{count, contains, latest_lists, latest, observe, adopt, apply}` with the signatures of §6.

**Checkpoint**: M1 done. `mise run test-core` green.

---

## Phase 4: User Story 2 core — the drag pick (FR-015–FR-018 decisions, FR-028)

**Goal**: `review::drag` turns press, rows passed and release into a click or a range.
**Independent test**: `mise run test-core` runs D1–D8.

- [ ] T009 [P] [US2] Create `crates/micold-core/src/review/drag.rs` (declared by T010) with failing unit tests D1–D8 from contracts/core-api.md in its `mod tests`.
- [ ] T010 [US2] Add `pub mod drag;` to `crates/micold-core/src/review/mod.rs` and implement
  `review::drag` (data-model §7) over the existing `review::Side` and
  `review::LineRange` (`crates/micold-core/src/review/mod.rs:67,77`): `gutter_line(old, new) ->
  Option<(Side, u32)>` (C1: new number first, else old, `None` for neither; the same rule as the
  client's `Msg::GutterPressed` arm, `crates/micold-client/src/features/changes.rs:2243`, which M4
  switches to it); `Drag { side, pressed, last, moved }` with private fields, `Drag::press`,
  `over(old, new)`, `span() -> (Side, LineRange)`, `release() -> Release { Click { side, line },
  Range { side, anchor, head } }`.

---

## Phase 5: User Story 3 core — chips and the setting (FR-022, FR-024, FR-028)

**Goal**: the three chips, their insertion rule and the off-by-default setting live in the core.
**Independent test**: `mise run test-core` runs K1–K5.

- [ ] T011 [P] [US3] Create `crates/micold-core/src/review/chips.rs` (declared by T012) with failing unit tests K1–K4 in its `mod tests`.
- [ ] T012 [US3] Add `pub mod chips;` to `crates/micold-core/src/review/mod.rs` and implement
  `review::chips` (data-model §8): `SuggestionChip { label: &'static str,
  text: &'static str }`, `CHIPS: [SuggestionChip; 3]` = *Rename* → "Rename this to ", *Add test* →
  "Add a test that covers this.", *Why?* → "Why is this needed?"; `insert(text, chip) -> String`
  (empty or whitespace-only → the chip's text; otherwise appended on a new line, no blank line when
  the text already ends with a newline).
- [ ] T013 [P] [US3] Write failing test K5 in the tests of `crates/micold-core/src/settings.rs`,
  beside `reduce_motion_round_trips`: default false, a `settings.json` without the key reads false,
  true round-trips through `StoredSettings` as `suggestion_chips`. Add `"suggestion_chips"` to the
  pinned key list in `crates/micold-core/tests/settings_issue_mapping.rs` (`:185-210`, sorted) in
  the same commit, so that test fails until T014 adds the key.
- [ ] T014 [US3] Implement `Settings::suggestion_chips: bool` with `#[serde(default)]` in
  `crates/micold-core/src/settings.rs`, on `StoredSettings` (`settings.rs:742`) too, carried by
  `StoredSettings::from_settings` and the conversion back (`StoredSettings::into_settings`,
  `settings.rs:935`), as `reduce_motion` is. Fix every exhaustive `Settings` literal or destructure
  the field breaks (a grep of the workspace for `Settings {` without `..` finds only these):
  - `ValidSettings::into_settings` (`crates/micold-client/src/features/settings.rs:795`): set
    `suggestion_chips: false`; `ValidSettings` gets no field until M5, and the save path below
    never writes this value;
  - `window_settings` (`crates/micold-client/src/shell/persist.rs:332`): take
    `suggestion_chips: stored.suggestion_chips` until M5, so a Settings save or a window write never
    overwrites the stored value;
  - `set_changed` (`persist.rs:409`, the deliberate exhaustive destructure): add
    `suggestion_chips: _` with a line in its doc comment that a save does not set it until M5 (the
    form does not hold it), and do not `set!` it.
  The `Settings` literals in `persist.rs`'s tests (`:1171`, `:1238`) and in
  `features/settings.rs:2727` end in `..`, so they need no change. The client shows nothing new.

**Checkpoint**: M2 done. `mise run test-core` and `mise run gate` green; no visible change.

---

## Phase 6: User Story 1 client — hold, notice and dot (FR-005–FR-012, FR-014) ⛔ after #1269 and #1271

**Goal**: the Review tab keeps its list and diff on a background read, shows *n files changed since
you looked* and a dot per changed row, and applies on a press. **Independent test**:
`cargo test -p micold-client --test features_changes` runs H1–H15; quickstart §A.

- [ ] T015 [P] [US1] Write failing tests H1–H6 and H15 in
  `crates/micold-client/tests/features_changes.rs`: a differing watch read leaves `list`,
  `selected`, `diff`, `diff_offset`, `pick`, `composer`, `anchors` unchanged and issues no
  `ReadDiff`; a `Same` read replaces silently; a user read (toggle, source, reopen, and one queued
  behind a watch read) applies; a watch read while held re-observes; #1269's show-time re-read is
  held like a watch read (research R9); `stale_notice` singular/plural/`None`;
  `changed_since_you_looked`; no list on screen applies at once; a failed watch read keeps list and
  hold and shows no error.
- [ ] T016 [P] [US1] Write failing tests H7–H10a in `crates/micold-client/tests/features_changes.rs`:
  selecting outside the set reads under the shown scope; inside the set and listed reads under the
  latest scope (`from` of a rename) and adopts `DiffRow::Row`; inside the set and no longer listed
  issues no read and `no_longer_changed` is true with the dot kept; with nothing held a
  working-tree `Row` writes into the shown list; `NotRead` adopts nothing; `Unlisted` under a hold
  acts as H9, with nothing held leaves the list. Also a unit test of `DiffRow::from_read` (T019):
  `Err` → `NotRead`, a list without the path → `Unlisted`, a list with it (or its rename) → `Row`.
- [ ] T017 [P] [US1] Write failing tests H11–H14 in `crates/micold-client/tests/features_changes.rs`:
  a differing pull request head refresh sets `pr.held` and leaves `pr.diff`, `pr.selected`,
  `pr.file`; selecting a held PR file shows the latest patch and leaves the set; `StaleApplied`
  applies, keeps the selected file while listed and re-reads its diff keeping the old one meanwhile,
  re-judges anchors, re-parses `pr.file`; composer text and picks are kept by today's refresh rules
  (`composer_placed`), an unsaved new comment on a removed file closes; while held,
  `ComposerEdited`/`ComposerSaved` behave as without a hold and quote the lines shown.
- [ ] T018 [US1] Implement the working-tree hold in `crates/micold-client/src/features/changes.rs`
  (data-model §10, §12, §13; research R3, R4): `OpenView.held: Option<Held<ChangeList>>`,
  `OpenView.apply_next: bool` set by every user-caused read and kept by a queued `again`;
  `list_read` goes through `stale::observe` unless `apply_next` is set or no list is on screen, and
  a held read neither re-reads the diff nor touches `selected`, `pick`, composer or anchors
  (`refresh_browsed` still runs); `Msg::StaleApplied` → `Held::apply` then the refresh path
  (`request_diff(state, true)`); helpers `stale_notice`, `changed_since_you_looked`,
  `no_longer_changed`. The show-time re-read #1269 added must not set `apply_next`.
- [ ] T019 [US1] Implement selection under a hold (FR-009, research R5): `enum DiffRow { NotRead,
  Unlisted, Row(ChangedFile) }` on `Msg::DiffRead.row` and `FileSelected`/`DiffRead` adoption via
  `Held::adopt` in `crates/micold-client/src/features/changes.rs`; in
  `crates/micold-client/src/shell/changes.rs` the working-tree `ReadDiff` job also runs
  `GitCli::change_list_of(dir, scope, toggles, &[path, from])` and maps the result with the pure
  `DiffRow::from_read(io::Result<ChangeList>, path)` defined in `features/changes.rs` (a failure →
  `NotRead`, error not shown), so the shell job holds no logic of its own.
- [ ] T020 [US1] Implement the pull request hold (research R6, data-model §11) in
  `crates/micold-client/src/features/changes.rs`: `PullRequestView.held: Option<Held<PrDiff>>` set
  when a head refresh answers with differences; selection of a held file adopts row and patch from
  `held.latest`; `StaleApplied` makes `pr.diff` the latest and re-parses `pr.file`. Choose whether
  `PrDiffLoad::Ready` holds a `PrDiff` or converts at the hold boundary, and say which in the PR.
- [ ] T021 [P] [US1] Add the `ChangedDot` primitive in `crates/micold-material/src/changed_dot.rs`
  (exported from `crates/micold-material/src/lib.rs`), a builder ending in `.into()`, drawn with
  `unread_mark`'s `filled_circle` and `DIAMETER` (`crates/micold-material/src/unread_mark.rs`) rather
  than a new circle style, with an in-file geometry test; show it in
  `crates/micold-client/src/showcase/sections/review.rs`.
- [ ] T022 [US1] Draw it in `crates/micold-client/src/ui/changes.rs` (glue, after #1269 the Review
  tab's view): the notice as the shared `Button` text variant labelled `stale_notice(view)` in the
  view header, pressing it sends `Msg::StaleApplied`; `ChangedDot` on each file row where
  `changed_since_you_looked` is true (the tree row of #1273, if merged, places the same primitive);
  the diff area says the file has no changes any more when `no_longer_changed`.
- [ ] T023 [US1] Update *Keeping up with changes* in `docs/user-guide/reviewing-changes.md`: the
  list and diff stay put, the notice and its count, the dots, what selecting a dotted file does,
  what applies at once (toggles, source, no list on screen).
- [ ] T024 [US1] Add the notice and a dotted row to the scene in
  `crates/micold-client/tests/layout_snapshot.rs` and regenerate
  `crates/micold-client/tests/fixtures/layout_snapshot.txt` with `UPDATE_LAYOUT_SNAPSHOT=1`; the
  fixture diff shows only those additions.
- [ ] T025 [US1] Visual pass at 1280×800 in light and dark (`visual-pass` skill through an
  `autopilot-worker`) of quickstart §A step 2's state (header notice, row dots, unchanged diff);
  evidence in `specs/1275-review-stale-quick-comment/visual-pass/`.

**Checkpoint**: M3 done. Quickstart §A passes; `mise run gate` green.

---

## Phase 7: User Story 2 client — drag to comment (FR-015–FR-021) ⛔ after #1269 and #1271

**Goal**: a drag over the line numbers picks the range and opens the composer under it.
**Independent test**: Q1–Q9 in `features_changes.rs`; the gutter's message test; quickstart §B
steps 2–4.

- [ ] T026 [P] [US2] Write a failing in-crate test in
  `crates/micold-client/src/ui/material/diff_view.rs`: the `Gutter` publishes a press as today
  (Shift extends) and publishes `over` on cursor entry only while `.dragging(true)`.
- [ ] T027 [P] [US2] Write failing tests Q1–Q9 in `crates/micold-client/tests/features_changes.rs`
  (contracts/quick-comment.md).
- [ ] T028 [US2] Extend `DiffView` in `crates/micold-client/src/ui/material/diff_view.rs` with
  `.on_gutter_drag(press, over)` and `.dragging(bool)`; the `Gutter` widget (`diff_view.rs:800`)
  publishes `over(old, new)` when the cursor enters it during a drag (research R7).
- [ ] T029 [US2] Implement the drag in `crates/micold-client/src/features/changes.rs`:
  `OpenView.drag: Option<DragState { drag: Drag, before_pick: Option<Pick>, before_composer:
  Option<Composer> }>`; `GutterPressed { extend: false }` uses core `review::drag::gutter_line`
  (replacing the client's own C1 code) and starts a drag where picking is allowed;
  `Msg::GutterDragOver`, `GutterDragReleased` (click → pick only; range → open or move a `New`
  composer, keep an `Edit`/`Answer` one), `GutterDragCancelled` (restore `before_pick` and
  `before_composer`).
- [ ] T030 [US2] Add the window-wide release and **Esc** listener, active only while `OpenView.drag`
  is `Some`, in `crates/micold-client/src/shell/subscriptions.rs`, and focus the composer's text
  area after a range release in `crates/micold-client/src/shell/changes.rs`.
- [ ] T031 [US2] Wire `.on_gutter_drag(..)` and `.dragging(..)` in
  `crates/micold-client/src/ui/changes.rs` (glue).
- [ ] T032 [US2] Update *Picking lines* and *Writing a comment* in
  `docs/user-guide/reviewing-changes.md`: drag over line numbers, Esc cancels, click and Shift-click
  as before.
- [ ] T033 [US2] Run `crates/micold-client/tests/layout_snapshot.rs`; the drag adds no static
  element, so `crates/micold-client/tests/fixtures/layout_snapshot.txt` should stay unchanged:
  regenerate it only for a picked-range change, and say why in the PR.
- [ ] T034 [US2] Visual pass at 1280×800 in light and dark of quickstart §B steps 2–4 (mouse drag,
  touchpad drag, Esc mid-drag); evidence and any step a container could not run in
  `specs/1275-review-stale-quick-comment/visual-pass/`.

**Checkpoint**: M4 done.

---

## Phase 8: User Story 3 client — suggestion chips (FR-022–FR-027) ⛔ after #1269 and #1271

**Goal**: a Settings switch, off by default, shows *Rename*, *Add test*, *Why?* under every
composer. **Independent test**: Q10–Q12, the settings tests; quickstart §B step 5.

- [ ] T035 [P] [US3] Write failing tests in the tests of
  `crates/micold-client/src/features/settings.rs`: the switch on the draft, Save applies it, Cancel
  discards it (FR-022, FR-026).
- [ ] T036 [P] [US3] Write failing tests Q10–Q12 and H14's chip case in
  `crates/micold-client/tests/features_changes.rs`.
- [ ] T037 [P] [US3] Make `ToggleChip` (`crates/micold-material/src/toggle_chip.rs`) join keyboard
  traversal through `TakesTheKeyboard`, as `AttachmentChip` does
  (`crates/micold-material/src/attachment_chip.rs`), with an in-file focus/geometry test; show the
  focusable chip in `crates/micold-client/src/showcase/sections/controls.rs`.
- [ ] T038 [US3] Put `suggestion_chips` on the Appearance draft and the applied state with
  `SuggestionChipsToggled(bool)` in `crates/micold-client/src/features/settings.rs`, following
  `reduce_motion` (including the field table at `features/settings.rs:34`), and replace T014's
  stopgaps: `ValidSettings` carries it into `into_settings`, `window_settings` takes it from the
  window's applied state instead of `stored`, and `set_changed` sets it (`set!(suggestion_chips)`),
  with a test in `persist.rs` that saving it keeps every other setting, as
  `saving_reduce_motion_keeps_every_other_setting` does; add the **Suggestion chips** `Checkbox` under **Reduce motion** in
  `crates/micold-client/src/ui/settings/appearance.rs` with a new `FieldId`.
- [ ] T039 [US3] Implement in `crates/micold-client/src/features/changes.rs`:
  `State.suggestion_chips`, `Msg::SuggestionChipsInForce(bool)`, `Msg::ChipPressed(usize)` →
  `composer.text = chips::insert(..)` (nothing with no composer or the setting off),
  `chips_shown(state)`; in `crates/micold-client/src/shell/changes.rs` send `SuggestionChipsInForce`
  at start and after a settings save, and on `ChipPressed` run `sync_composer` (`shell/changes.rs:107`)
  then focus the text area.
- [ ] T040 [US3] Draw the chips under the composer when `chips_shown` in
  `crates/micold-client/src/ui/changes.rs`, after the text area in keyboard order (glue).
- [ ] T041 [US3] Update `docs/user-guide/settings.md` (the switch, off by default) and *Writing a
  comment* in `docs/user-guide/reviewing-changes.md` (the chips and what they insert).
- [ ] T042 [US3] Add the chips row and the Settings switch to the scene in
  `crates/micold-client/tests/layout_snapshot.rs` and regenerate
  `crates/micold-client/tests/fixtures/layout_snapshot.txt`.
- [ ] T043 [US3] Visual pass at 1280×800 in light and dark of quickstart §B step 5 (switch off: no
  chips; on: three chips, Tab reaches each with its ring, *Add test* fills an empty box), plus one
  frame showing notice, dot and chips together; evidence in
  `specs/1275-review-stale-quick-comment/visual-pass/`.

**Checkpoint**: M5 done. SC-003, SC-004, SC-006 checkable.

---

## Phase 9: Reviewed files while held (FR-013) ⛔ after #1271

- [ ] T044 [P] [US1] Write failing test H16 in `crates/micold-client/tests/features_changes.rs`: a
  held file #1271 marks reviewed is shown unseen with the dot, *n of m reviewed* counts
  `reviewed ∖ changed`, and the hold writes nothing to the service.
- [ ] T045 [US1] Implement it in `crates/micold-client/src/features/changes.rs` (the reviewed count)
  and in the row glue of `crates/micold-client/src/ui/changes.rs` (no reviewed mark on a dotted row)
  (research R10).
- [ ] T046 [US1] Update `docs/user-guide/reviewing-changes.md` (reviewed marks and held changes),
  regenerate `crates/micold-client/tests/fixtures/layout_snapshot.txt` if the row changed, and run a
  1280×800 light and dark visual pass of a reviewed file with the dot; evidence in
  `specs/1275-review-stale-quick-comment/visual-pass/`.

**Checkpoint**: M6 done.

---

## Phase 10: Polish (close unit, no code)

- [ ] T047 Run quickstart §A in full and §C on macOS and Windows (or record what stood in for them)
  in `specs/1275-review-stale-quick-comment/quickstart.md`'s results; tick checklists.

---

## Dependencies & Execution Order

- Phase 2 → Phase 3 (M1). Phases 4–5 (M2) need nothing from M1; M1 and M2 may run in parallel.
- Phase 6 (M3) needs M1, #1269 and #1271 merged. Phase 7 (M4) needs M2 and the same merges.
  Phase 8 (M5) needs M2 and the same merges. Phase 9 (M6) needs #1271 merged and M3 (the hold it
  reads).
- Within a phase: test tasks first (they must fail), then implementation, then glue, docs, snapshot,
  visual pass.

### Parallel opportunities

- M1: T001, T003, T005 together; then T002 before T004 (T004 fills the identity T002 adds).
- M2: T009, T011, T013 together; T010, T012, T014 then in any order.
- M3: T015, T016, T017, T021 together. M4: T026, T027 together. M5: T035, T036, T037 together.

## Implementation Strategy

Core first: M1 and M2 land now with unit tests (FR-028) and no visible change. The client
milestones are written once, in the Review tab #1269 builds, after #1269 and #1271 merge; each ships
its own docs, snapshot and visual pass. M6 follows #1271.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Core stale tracking 🎯 MVP

- **Tasks**: T001–T008
- **Deliverable**: every change-list row on `main` carries a change identity, and `review::stale`
  reports, counts and holds the files changed since a shown list, proven by core unit and real-git
  tests; nothing a user sees changes (label `docs-not-needed`).
- **Satisfies**: FR-001, FR-001a, FR-002, FR-003, FR-004, FR-028 (stale set); US1 AS3, AS4, AS5,
  AS10, AS11, AS12 at the core level
- **Verify**: `mise run test-core` (I1–I8, S1–S10)
- **Depends on**: —
- **Gating**: none; builds now
- **Tier**: full

### M2 — Core drag pick, chips and setting

- **Tasks**: T009–T014
- **Deliverable**: `review::drag`, `review::chips` and `Settings::suggestion_chips` (off by default,
  saved) are on `main` with unit tests; nothing a user sees changes (label `docs-not-needed`).
- **Satisfies**: FR-015–FR-018 (decisions), FR-022 (default and persistence), FR-024, FR-028
- **Verify**: `mise run test-core` (D1–D8, K1–K5)
- **Depends on**: —
- **Gating**: none; builds now
- **Tier**: full (adds a persisted settings key and a public core API)

### M3 — Hold and notice in the Review tab

- **Tasks**: T015–T025
- **Deliverable**: in the Review tab, background changes no longer move the list or the open diff;
  the header shows *n files changed since you looked*, changed rows carry a dot, and pressing the
  notice applies (working tree and pull request).
- **Satisfies**: US1 AS1–AS12; FR-005–FR-012, FR-014, FR-029 (*Keeping up with changes*, notice and
  dot in the snapshot and visual pass); SC-001, SC-002, SC-005
- **Verify**: `cargo test -p micold-client --test features_changes` (H1–H15); quickstart §A;
  `visual-pass/` frames of §B step 1
- **Depends on**: M1
- **Gating**: ⛔ do not start until #1269 and #1271 are both merged
- **Tier**: full

### M4 — Drag to comment

- **Tasks**: T026–T034
- **Deliverable**: dragging over line numbers picks the range as the pointer moves and, on release,
  opens the comment box under the last line with the caret in it; Esc restores the earlier pick.
- **Satisfies**: US2 AS1–AS7; FR-015–FR-021, FR-029 (*Picking lines*, *Writing a comment*, visual
  pass); SC-003
- **Verify**: `cargo test -p micold-client --test features_changes` (Q1–Q9); the `diff_view`
  gutter test; quickstart §B steps 2–4
- **Depends on**: M2
- **Gating**: ⛔ do not start until #1269 and #1271 are both merged
- **Tier**: full

### M5 — Suggestion chips

- **Tasks**: T035–T043
- **Deliverable**: Settings → Appearance has **Suggestion chips** (off by default); when on, every
  comment box shows *Rename*, *Add test*, *Why?*, reachable by Tab, inserting their text.
- **Satisfies**: US3 AS1–AS5; FR-022–FR-027, FR-029 (`settings.md`, *Writing a comment*, chips in
  the snapshot, visual pass); SC-004, SC-006
- **Verify**: `cargo test -p micold-client --test features_changes` (Q10–Q12); the settings tests;
  quickstart §B step 5
- **Depends on**: M2
- **Gating**: ⛔ do not start until #1269 and #1271 are both merged
- **Tier**: full

### M6 — Reviewed files while held

- **Tasks**: T044–T046
- **Deliverable**: a reviewed file with a held change shows as unseen with the dot, and *n of m
  reviewed* counts it as not reviewed.
- **Satisfies**: FR-013, FR-029 (docs, visual pass)
- **Verify**: `cargo test -p micold-client --test features_changes` (H16); `visual-pass/` frame
- **Depends on**: M3
- **Gating**: ⛔ do not start until #1271 is merged
- **Tier**: full

T047 (Polish, no code) is left to the close unit.
