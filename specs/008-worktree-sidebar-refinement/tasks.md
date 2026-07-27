---

description: "Task list for Worktree Sidebar Refinement"
---

# Tasks: Worktree Sidebar Refinement

**Input**: Design documents from `/specs/008-worktree-sidebar-refinement/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Tests**: MANDATORY (Constitution Principle I — Test-First). Each story writes failing headless
tests BEFORE implementation (Red-Green-Refactor). Purely visual acceptance (padding, 80% font,
tag colors) is validated via quickstart.md; every underlying value/logic that CAN be tested
headless (name/tag derivation, filter predicate, reducers, persistence, tag color AA, size
constants, delete orchestration via fakes) MUST have a failing test first.

**Documentation**: MANDATORY per user-facing story (Constitution Principle VII).

**Cross-platform**: All logic platform-agnostic; git behind the `Git` abstraction; CI on
Linux/macOS/Windows (Principle VI).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an incomplete task)
- **[Story]**: US1–US5 (maps to spec.md user stories)

## Path Conventions

Single-project Rust + iced: pure logic in `src/*.rs`, GUI in `src/ui/`, headless tests in
`tests/`, side effects at `src/main.rs`, user guide in `docs/`.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Confirm baseline and register new module/test skeletons so the crate keeps
compiling as tasks land.

- [x] T001 Establish green baseline: run `cargo build` and `cargo test` (incl. `cargo test --no-default-features --test tokens`) on branch `008-worktree-sidebar-refinement` and record the passing state.
- [x] T002 Register the new shared component module `tag` in `src/ui/material/mod.rs` (empty compiling stub `src/ui/material/tag.rs`) and create empty test file `tests/worktree_delete.rs` so later tasks compile incrementally.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Tag semantics, tag colors/typography tokens, and the shared Tag chip — required by
US1 (tags) and US4 (filter), and by US5 (status tag + sizes). Delivers no standalone user value.

**⚠️ CRITICAL**: US1, US4, and US5 cannot begin until this phase completes. (US2/US3 depend only
on their own context-menu tasks and may start in parallel with this phase.)

### Tests (write first, must FAIL)

- [x] T003 [P] Tests for `naming::display_name` and `naming::parse_tags` in `tests/naming.rs`, covering the full example table in `contracts/naming-tags.md` (typed, untyped, issue-key, empty-remainder fallback).
- [x] T004 [P] Extend `tests/tokens.rs`: add all 11 tag `(on_fill, fill)` pairs to the fixed `pairs()` array so both `light_scheme_meets_aa_contrast` and `dark_scheme_meets_aa_contrast` enforce AA, and assert `tokens::sidebar::{NAME,TAG,SESSION}` equal `round(0.8 × base)`.

### Implementation

- [x] T005 [P] Add `Tag` enum (`Type(ConventionalType)`, `Issue(String)`, `Status(WorktreeStatus)`) in `src/naming.rs`.
- [x] T006 Implement `naming::display_name(&str) -> String` and `naming::parse_tags(&str) -> Vec<Tag>` in `src/naming.rs` to pass T003 (reuse `ConventionalType`; Jira regex `\b[A-Z][A-Z0-9]+-\d+\b`).
- [x] T007 [P] Add per-type + `issue` tag `Rgb` role pairs to `Roles` in both `LIGHT` and `DARK`, plus a `ConventionalType → (fill, on_fill)` lookup, in `src/tokens.rs` (pass T004 AA).
- [x] T008 [P] Add `pub mod sidebar { NAME=11; TAG=10; SESSION=11; }` (80% of BODY/LABEL) in `src/tokens.rs` (pass T004 size assertion).
- [x] T009 Add shared builder-style `Tag` chip (`Tag::new(label, fill, on_fill).size(px)` → `impl From<Tag> for Element`) in `src/ui/material/tag.rs`, and map the new `Rgb` roles → `iced::Color` in `src/ui/style.rs`.

**Checkpoint**: Tag semantics, AA-checked colors, sidebar sizes, and the Tag chip exist.

---

## Phase 3: User Story 1 - Recognize a worktree at a glance (Priority: P1) 🎯 MVP

**Goal**: Each worktree shows a friendly name with color-coded type + Jira issue tags.

**Independent Test**: Create worktrees with/without a Jira key and one non-conforming; verify
friendly name, correct color-coded type tag, issue tag only when a key is present, no misleading
tag on the non-conforming one — with on-disk branch/dir unchanged.

### Tests (write first, must FAIL)

- [x] T010 [P] [US1] Extend `tests/sidebar_tree.rs`: `State::worktree_tree()` exposes each worktree's derived `display_name` and `tags` (type + issue) for typed, untyped, and issue-bearing names.
- [x] T011 [P] [US1] Add a case in `tests/sidebar_tree.rs` asserting the derived name uses `naming::display_name` when no override is present (e.g. `feat-abc-123-login-page` → "Login page").

### Implementation

- [x] T012 [US1] Compute `display_name` (via `naming::display_name`; override integration added in US3) and `tags` (via `naming::parse_tags`) per worktree in `State::worktree_tree()` / `WorktreeNode` in `src/app.rs`.
- [x] T013 [US1] Extend `TreeItem`/`TreeView` with a two-line row (name line + optional tag row via `.tags(Vec<Tag>)`) and drop the leading git-status icon slot for worktree rows in `src/ui/material/tree_view.rs`.
- [x] T014 [US1] Render worktree rows in `src/ui/sidebar.rs` using the derived `display_name` (at `sidebar::NAME`) and `Tag` chips for type + issue (at `sidebar::TAG`); truncate an over-long name with an ellipsis while keeping its tags visible (spec Edge Cases: long names).
- [x] T015 [P] [US1] Document friendly names + color-coded type/Jira tags (with the per-type color legend) in the user guide under `docs/`.

**Checkpoint**: Worktrees render as name + color-coded tags; MVP is demoable.

---

## Phase 4: User Story 2 - Delete a worktree and everything it owns (Priority: P2)

**Goal**: Right-click → Delete removes the worktree dir, its sessions (terminating running
processes first), and its git branch, behind a confirmation.

**Independent Test**: Right-click a worktree with a running session, Delete, confirm → worktree,
sessions, and branch gone; cancel → nothing removed.

**Introduces** the shared context-menu infrastructure reused by US3.

### Tests (write first, must FAIL)

- [x] T016 [P] [US2] `tests/sidebar_state.rs`: `worktree_menu_open: Option<String>` single-open invariant — `WorktreeMenuToggled` opens/closes; opening one closes another; `WorktreeMenuDismissed` clears.
- [x] T017 [P] [US2] `tests/app_state.rs`: `WorktreeDeleteRequested` opens `Overlay::ConfirmWorktreeDelete`; `WorktreeDeleteConfirmed` drops the target worktree's session records and clears `active_session` if it matched; `WorktreeDeleteCancelled` is a no-op; `on_escape` maps the confirm overlay → cancel.
- [X] T018 [P] [US2] `tests/worktree_delete.rs`: with `FakeGit` + `FakeTerminalBackend`, confirm ⇒ FakeGit records worktree removed + branch deleted and the matching sessions' `FakeHandle` recorded `killed` while non-matching sessions are untouched; cancel ⇒ no `Git`/kill calls occur. (reopened then resolved — BUG-001: coverage asserts on `Git` call records only, so the post-removal filesystem step is untested; the existing locked-worktree error assertion holds for `FakeGit` but is unreachable for `GitCli`, which swallows git failures. Extended per T051/T053; closed 2026-07-20.)

### Implementation

- [x] T019 [US2] Add `worktree_menu_open: Option<String>` state + `WorktreeMenuToggled(String)` / `WorktreeMenuDismissed` messages + reducers in `src/app.rs`.
- [x] T020 [P] [US2] Extend `MenuOverlay` with a chainable `.anchor(...)` (row-anchored) in `src/ui/material/menu.rs`, preserving the existing toolbar top-right default.
- [x] T021 [US2] Wire right-click in `src/ui/sidebar.rs`: wrap each worktree row in `mouse_area(...).on_right_press(WorktreeMenuToggled(dir))` and render the anchored `MenuOverlay` (items: Rename → `WorktreeRenameStarted`, Delete → `WorktreeDeleteRequested`).
- [x] T022 [US2] Add `Overlay::ConfirmWorktreeDelete { dir_name }` + `WorktreeDeleteRequested/Confirmed/Cancelled` messages + reducers + `on_escape` arm in `src/app.rs`.
- [x] T023 [US2] Render the delete confirmation modal (naming the directory, its sessions, and the branch) in `src/ui/mod.rs`, with the Esc subscription and `ClosingOverlay` handling.
- [X] T024 [US2] Implement delete orchestration in `src/main.rs` on `WorktreeDeleteConfirmed`: kill sessions whose `worktree_dir == dir_name` (via `terminals`), then `worktree_remove(force=true)` → `worktree_prune` → `branch_delete` → `fs::remove_dir_all`, then `update` reducer → `discover_worktrees` → `persist` (order per `CleanupStep`, idempotent). (reopened then resolved — BUG-001: the `fs::remove_dir_all` step is **not** idempotent as this task required — it reports `ErrorKind::NotFound` as a user-facing failure on every successful delete, since `git worktree remove` already deleted the directory. Fixed per T054; closed 2026-07-20.)
- [x] T025 [P] [US2] Document the right-click Delete action and its confirmation semantics (dir + sessions + branch) in `docs/`.

**Checkpoint**: Worktrees are deletable with confirmation; running sessions terminate first.

---

## Phase 5: User Story 3 - Rename a worktree's displayed name (Priority: P3)

**Goal**: Right-click → Rename changes only the displayed name; persists across restart; tags
unchanged; folder/branch untouched.

**Independent Test**: Rename a worktree, restart the app → custom name persists; `git worktree
list` and the branch are unchanged; tags still derived from the branch.

**Depends on**: US2 context-menu infrastructure (T019–T021).

### Tests (write first, must FAIL)

- [x] T026 [P] [US3] `tests/store_roundtrip.rs`: `StoredProject.worktree_display_names` round-trips via `JsonFileStore::at(temp)`; a `projects.json` written WITHOUT the field loads to an empty map (no schema bump); the override survives a reload.
- [x] T027 [P] [US3] `tests/sidebar_state.rs`: rename draft lifecycle — `WorktreeRenameStarted` seeds the draft from the current display name; `…TextChanged` updates; `…Cancelled`/empty keeps the prior name; the confirmed path sets `error` on invalid input; and renaming two worktrees to the same display name is accepted (identity stays distinct — spec Edge Cases: duplicate display names).
- [x] T028 [P] [US3] `tests/app_state.rs`: `WorktreeRenameConfirmed` calls `Workspace::set_worktree_name` (mutating only the display name) and `on_escape` maps `Overlay::RenameWorktree` → cancel; assert the target worktree's `path` and `branch` are unchanged after the override is set (FR-007/FR-014).

### Implementation

- [x] T029 [P] [US3] Add `Project.worktree_names: BTreeMap<String,String>` and `Workspace::set_worktree_name` / `clear_worktree_name` (reusing `validate_rename`) in `src/project.rs` and `src/workspace.rs`.
- [x] T030 [P] [US3] Add `#[serde(default)] worktree_display_names` to `StoredProject`, map it in `from_workspace`/`into_workspace`, and reconcile it against live `dir_name`s on save, in `src/store.rs`.
- [x] T031 [US3] Add `Overlay::RenameWorktree` + `WorktreeRenameDraft` + `WorktreeRenameStarted/TextChanged/Confirmed/Cancelled` messages + reducers + `on_escape` arm in `src/app.rs`.
- [x] T032 [US3] Render the worktree-rename overlay in `src/ui/mod.rs` (reuse the `src/ui/rename.rs` pattern), with the Esc subscription and `ClosingOverlay` arm.
- [x] T033 [US3] Update the display-name derivation in `src/app.rs` (`worktree_tree`) to prefer the `worktree_names` override when present, else `naming::display_name` (completes T012).
- [x] T034 [US3] Call `persist(&app.core)` after `WorktreeRenameConfirmed` at the `src/main.rs` boundary (mirror the project-rename persistence site).
- [x] T035 [P] [US3] Document the right-click Rename action (display-name only, persists, does not touch folder/branch) in `docs/`.

**Checkpoint**: Worktrees are renamable (display-only) and the override persists.

---

## Phase 6: User Story 4 - Filter the worktree list by tag (Priority: P4)

**Goal**: Activate tag filters (type / has-issue / untyped, OR-combined); clear restores all.

**Independent Test**: With worktrees of several types, activate a type filter → only matches
list; add another → union; activate "untyped" → non-conforming ones; clear → full list.

**Depends on**: Foundational tags (T005–T006) and US1 per-row tags (T012).

### Tests (write first, must FAIL)

- [x] T036 [P] [US4] `tests/sidebar_tree.rs`: the filter predicate selects the right worktrees for each `TagFilter` (`Type`, `HasIssue`, `Untyped`) and for OR-combined sets; an empty filter set shows all; and the filtered set recomputes correctly after a worktree is renamed or removed (spec US4 AC#4 / FR-028).
- [x] T037 [P] [US4] `tests/sidebar_state.rs`: `SidebarFilterToggled` inserts/removes a filter; `SidebarFiltersCleared` empties the set.

### Implementation

- [x] T038 [US4] Add `enum TagFilter { Type(ConventionalType), HasIssue, Untyped }`, `sidebar_filters: BTreeSet<TagFilter>` state, and `SidebarFilterToggled(TagFilter)` / `SidebarFiltersCleared` messages + reducers in `src/app.rs`.
- [x] T039 [US4] Apply the `matches_filters` OR-predicate in `State::worktree_tree()` in `src/app.rs` (empty ⇒ all).
- [x] T040 [US4] Render the filter chip row (Tag-style toggles bound to `SidebarFilterToggled`), a clear control (`SidebarFiltersCleared`), and the no-match empty state at the top of the sidebar body in `src/ui/sidebar.rs`.
- [x] T041 [P] [US4] Document tag filtering (including the "untyped" bucket and OR behavior) in `docs/`.

**Checkpoint**: The worktree list is filterable by tag and clearable in one action.

---

## Phase 7: User Story 5 - A compact, space-efficient sidebar (Priority: P5)

**Goal**: Minimal left/right padding, no git icon, 80% sidebar font, and a lightweight
missing/invalid cue.

**Independent Test**: Compare before/after — no git icon, minimal padding, visibly smaller text
(80%), legible in light and dark; missing/invalid worktrees still distinguishable.

**Depends on**: US1 row redesign (T013–T014) and Foundational tokens (T007–T008).

### Tests (write first, must FAIL)

- [x] T042 [P] [US5] `tests/sidebar_tree.rs`: `worktree_tree()` includes a `Tag::Status` (missing/invalid) for non-`Valid` worktrees and none for `Valid` ones.

### Implementation

- [x] T043 [US5] Inject the `Status` tag from `WorktreeStatus` into a worktree row's tags and render the name in the `error` role color for non-`Valid` worktrees (missing/invalid cue, FR-011) in `src/app.rs` + `src/ui/sidebar.rs`.
- [x] T044 [US5] Confirm the leading git icon is fully removed and apply `sidebar::{NAME,TAG,SESSION}` sizes to the worktree/session rows in `src/ui/material/tree_view.rs` + `src/ui/sidebar.rs` (FR-010, FR-012). **Cross-feature note 2026-07-27 (BUG-005)**: correct as written and not reopened — FR-010 names *worktree* rows, and the git icon was indeed removed from them. Recording the boundary because it was later mistaken for a whole-sidebar cleanup: **session** rows kept a leading `Icon::ActiveMarker` (`check_circle`) that feature 005 had installed, so the sidebar stayed half-migrated for four features. FR-010's rationale (compact rows, no leading icon that does not vary) applies equally to session rows; feature 010 FR-016f now states it for them. See `specs/010-daemon-session-persistence/bugs/BUG-005.md`.
- [x] T045 [US5] Reduce the sidebar outer horizontal padding to `spacing::XS` and shrink the `tree_view` per-depth indent step in `src/ui/sidebar.rs` + `src/ui/material/tree_view.rs` (FR-009).
- [x] T046 [P] [US5] Document the compact sidebar and the missing/invalid indicator in `docs/`.

**Checkpoint**: The sidebar is compact and legible in both themes; state cues survive icon removal.

---

## Phase 8: Polish & Cross-Cutting Concerns

- [x] T047 [P] Cross-cutting user-guide review: index/navigation and screenshot updates across the new sidebar sections in `docs/`.
- [x] T048 Run the full suite green — `cargo test` and `cargo test --no-default-features --test tokens` — and confirm CI passes on Linux, macOS, and Windows (Principle VI).
- [x] T049 Run the `quickstart.md` manual GUI validation checklist in both light and dark themes.
- [x] T050 [P] Cleanup/refactor pass across `src/app.rs`, `src/ui/sidebar.rs`, and `src/ui/material/tree_view.rs` (remove dead code, reconcile duplication introduced across stories).

---

## Phase 9: BUG-001 — Delete reports a folder-removal error despite succeeding

**Goal**: A fully-successful delete is silent (FR-023, FR-023a); a genuine removal failure still
reaches the user (FR-023b). Satisfies SC-004a and closes reopened T018/T024.

### Tests (write first, must FAIL)

- [X] T051 [US2] `tests/worktree_delete.rs`: after a confirmed delete whose working directory is already absent by the time the filesystem cleanup runs (the normal case — git removed it), assert **no** notification is pushed. Drive it through the real orchestration path with a temp dir so the fs step actually executes; assert on notification output, not on `Git` call records (`FakeGit` never touches disk).
- [X] T052 [US2] `tests/worktree_delete.rs`: assert the converse — a working directory that genuinely survives removal still produces exactly one error notification naming the leftover path (guards against fixing T053 by muting the branch entirely).
- [X] T053 [P] [US2] `tests/worktree_delete.rs`: assert a genuine `git worktree remove` failure surfaces an error rather than being swallowed, and reconcile the existing locked-worktree assertion (currently passes only under `FakeGit`; `GitCli` returns `Ok(())` unconditionally).

### Implementation

- [X] T054 [US2] In `src/main.rs` (`WorktreeDeleteConfirmed`), treat `io::ErrorKind::NotFound` from the post-removal `fs::remove_dir_all` as success and suppress the notification; report only other error kinds. Mirror the existing create-rollback treatment at `CleanupStep::RemoveDir`. Makes T051/T052 green and closes T024's "idempotent" requirement.
- [X] T055 [US2] In `src/git.rs`, stop `GitCli::worktree_remove` from discarding git failures — propagate the error so FR-023/FR-023b's path is reachable in the shipped app, keeping "already-absent worktree" idempotent (the rollback case the current swallow was protecting). Makes T053 green.
- [ ] T056 [P] [US2] Re-run the delete section of `quickstart.md` manually: delete a worktree with a running session and confirm the sidebar updates with no error banner. (Attempted 2026-07-20 — inconclusive: the delete was reported as showing no error banner, but the only instance confirmed running at the time was the installed `/usr/bin` build of 2026-07-18, which predates this fix. Re-run against a build verified to come from this branch.)

**Checkpoint**: Deleting a worktree is silent on success and still loud on genuine failure.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: none — start immediately.
- **Foundational (Phase 2)**: after Setup. Blocks US1, US4, US5.
- **US1 (Phase 3)**: after Foundational.
- **US2 (Phase 4)**: after Setup; independent of Foundational (introduces its own menu infra). Can run alongside Phase 2/3.
- **US3 (Phase 5)**: after US2 (reuses context-menu infra T019–T021).
- **US4 (Phase 6)**: after Foundational + US1 (needs per-row tags).
- **US5 (Phase 7)**: after Foundational + US1 (row redesign).
- **Polish (Phase 8)**: after all targeted stories.
- **BUG-001 (Phase 9)**: after US2 (Phase 4). Independent of Phases 5–8.

### User Story Dependencies

- **US1 (P1)**: Foundational only → MVP.
- **US2 (P2)**: self-contained (menu + delete); reuses existing `Git`/session primitives.
- **US3 (P3)**: depends on US2 menu infra; also finalizes the display-name derivation started in US1 (T033 completes T012).
- **US4 (P4)**: depends on US1 tags.
- **US5 (P5)**: depends on US1 row redesign.

### Within Each Story

- Tests first and failing (Principle I) → implementation → user-guide doc (Principle VII) → checkpoint.
- Same-file tasks are sequential; `[P]` marks different-file, no-incomplete-dependency tasks.

### Parallel Opportunities

- Setup: T001 then T002.
- Foundational tests T003, T004 in parallel; then T005/T007/T008 in parallel (different files), T006 after T005, T009 after T007.
- US1 tests T010, T011 in parallel.
- US2 tests T016, T017, T018 in parallel; T020 (`menu.rs`) parallel with T019 (`app.rs`).
- US3 tests T026, T027, T028 in parallel; impl T029 (`project/workspace`) and T030 (`store`) in parallel.
- US4 tests T036, T037 in parallel.
- Doc tasks (T015, T025, T035, T041, T046) are `[P]` — different doc files.

---

## Parallel Example: Foundational Phase

```bash
# Tests first (different files):
Task: "Tests for naming::display_name + parse_tags in tests/naming.rs"          # T003
Task: "Extend tests/tokens.rs with 11 tag AA pairs + sidebar size asserts"      # T004

# Then implementation across different files:
Task: "Add Tag enum in src/naming.rs"                                            # T005
Task: "Add tag color role pairs to src/tokens.rs"                                # T007
Task: "Add sidebar size constants in src/tokens.rs"                              # T008
```

---

## Implementation Strategy

### MVP First (User Story 1)

1. Phase 1 Setup → 2. Phase 2 Foundational → 3. Phase 3 US1 → **STOP & VALIDATE** (friendly
   names + color-coded tags, no git icon). Demoable MVP.

### Incremental Delivery

Foundational → US1 (MVP) → US2 (delete) → US3 (rename) → US4 (filter) → US5 (density). Each
story is a shippable increment; run its tests + `quickstart.md` slice before moving on.

### Notes

- `[P]` = different files, no dependency on an incomplete task.
- Verify each story's tests FAIL before implementing (Red-Green-Refactor).
- Commit after each task or logical group.
- US2 introduces the context menu; US3 reuses it — implement US2 before US3.
- The display-name derivation is introduced in US1 (T012, derived-only) and completed in US3
  (T033, override-preferred) — expect T033 to edit the T012 code path.
- Phase 9 (BUG-001) depends only on US2 being implemented; T051–T053 must fail before T054/T055.
  T055 changes shared `Git` behaviour — re-run the full suite, not just `worktree_delete.rs`.

**Bugfix**: 2026-07-20 — BUG-001 Updated from bugfix patch

**Bugfix**: 2026-07-27 — BUG-005 (feature 010) Cross-feature note added to T044 recording that
FR-010's icon removal covered worktree rows only; session rows kept theirs. No task reopened. See
`specs/010-daemon-session-persistence/bugs/BUG-005.md`.
