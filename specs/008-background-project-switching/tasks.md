---
description: "Task list for Background Project Switching"
---

# Tasks: Background Project Switching

**Input**: Design documents from `specs/008-background-project-switching/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md

**Tests**: MANDATORY (Constitution Principle I). Every user story writes failing tests BEFORE
implementation (Red-Green-Refactor). Pure-core tests run under `cargo test --no-default-features`
against in-memory fixtures — no real git, no spawned processes, no GUI. Terminal/switcher behavior
that needs the runtime is covered by gui-gated tests. Session **isolation** is covered by an
integration test, not unit tests alone (Constitution "Isolation & lifecycle gate"). Prefer headless
VT/logic tests over launching the GUI.

**Documentation**: MANDATORY per story (Constitution Principle VII) — each user-facing story ships
its docs in the same change (`docs/user-guide/project-selection.md`, `docs/user-guide/worktrees-and-sessions.md`).

**Cross-platform**: Linux, macOS, Windows (Constitution Principle VI). No OS branching added; PTY
specifics stay inside existing crates.

**No new dependencies**: `Cargo.toml` is unchanged (plan.md Technical Context).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an incomplete task)
- **[Story]**: US1 / US2 / US3 (setup, foundational, polish have no story label)

## Path Conventions

Single Rust project: render-free core in `src/*.rs` (`app.rs`, `workspace.rs`, `session.rs`) + `tests/*.rs`;
gui-gated layer in `src/main.rs` and `src/ui/**`.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Test scaffolding; confirm scope needs no new deps.

- [x] T001 [P] Add core test scaffolding for multi-project state: a helper that builds a `State`/`Workspace` with two-or-more projects, each holding sessions in caller-chosen lifecycles (`Running`/`Idle`/`Failed`), in a new `tests/support.rs` (`mod support`), compiling under `cargo test --no-default-features`. Confirm no `Cargo.toml` changes are required.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Pure, story-agnostic query helpers on `Workspace` that later phases build on.

**⚠️ CRITICAL**: Complete before starting the user-story phases that consume these helpers.

- [x] T002 [P] Write failing tests for `Workspace::find_session` / `find_session_mut`: resolve a session id that lives in a **non-active** project; return `None` for an unknown id; owning project path is returned — in `tests/workspace_lookup.rs` (data-model.md).
- [x] T003 Implement `Workspace::find_session(&self, id) -> Option<(&Path, &Session)>` and `find_session_mut(&mut self, id) -> Option<(PathBuf, &mut Session)>` in `src/workspace.rs` to make T002 pass.
- [x] T004 [P] Write failing test for `Workspace::running_session_count(path)` — counts `is_active()` sessions for a project; `0` for a project with none/unknown — in `tests/workspace_lookup.rs` (FR-007, research R6).
- [x] T005 Implement `Workspace::running_session_count(&self, path) -> usize` in `src/workspace.rs` to make T004 pass.

**Checkpoint**: Cross-project session lookup + per-project running count available and unit-tested.

---

## Phase 3: User Story 1 - Keep sessions alive across a project switch (Priority: P1) 🎯 MVP

**Goal**: Switching the active project no longer stops the outgoing project's sessions; they keep running in the background and are restored (with the prior foreground) on return. Background crashes auto-restart under the existing guard and are surfaced on return. Concurrent background sessions across projects stay isolated.

**Independent Test**: Start a session in Project A (via the existing body "Known projects" list — no top-bar switcher needed), switch to Project B, wait, switch back to A: the session is still running with output produced while away; kill A's child while backgrounded → it auto-restarts and a notice appears on return.

### Tests for User Story 1 (MANDATORY — Constitution Principle I) ⚠️

> Write these FIRST and confirm they FAIL before implementing.

- [x] T006 [P] [US1] Failing core test: `State::switch_active(path)` leaves the outgoing project's sessions `Running` (no lifecycle mutation, none dropped) — BS-1 — in `tests/switch_active.rs`.
- [x] T007 [P] [US1] Failing core test: `switch_active` records the outgoing `active_session` into `foreground_by_project` **before** activating the new project, and on return restores it → else first running → else `None`, others stay background — BS-3, I1 — in `tests/switch_active.rs`.
- [x] T008 [P] [US1] Failing core test: `switch_active` to an unavailable project returns `false` and leaves active project + all sessions unchanged — BS-10 — in `tests/switch_active.rs`.
- [x] T009 [P] [US1] Failing core test: `note_background_restart(id)` marks the id only when its owning project ≠ active; the next `switch_active` into that project sets `notice` and clears those ids — BS-7 — in `tests/background_restart.rs`.
- [x] T010 [P] [US1] Failing gui-gated test: the `terminals` map is retained across a switch (not drained), an inactive project's session keeps being pumped, and a killed background PTY is respawned by the poll loop — BS-1, BS-2, BS-6 — in `tests/terminal_background.rs` (gui feature).
- [x] T011 [P] [US1] Failing **integration** test (Constitution "Isolation & lifecycle gate", Principle II): with **three** projects each holding a concurrently-running background session, assert isolation — each session is bound to its own worktree cwd and routed by its own `SessionId`, output produced by one never appears in another's VT grid, and all three run at once with no cap applied — BS-4, BS-5, FR-010, FR-013 — in `tests/session_isolation.rs` (gui feature).

### Implementation for User Story 1

- [x] T012 [US1] Add `State` fields `foreground_by_project: BTreeMap<PathBuf, SessionId>`, `restarted_while_inactive: BTreeSet<SessionId>`, `notice: Option<String>` (with defaults) in `src/app.rs` (data-model.md).
- [x] T013 [US1] Implement `State::switch_active(&mut self, path) -> bool` in `src/app.rs`: **first** record the current (outgoing) `active_session` into `foreground_by_project[outgoing]`, **then** `activate(path)` (reject-if-unavailable — leave state unchanged on `false`), then restore the incoming project's foreground, and arm `notice` from `restarted_while_inactive`; make T006–T008 pass. Ordering matters (I1): recording must precede activation so the outgoing project is captured, not the incoming one.
- [x] T014 [US1] Implement `State::note_background_restart(&mut self, id)` in `src/app.rs`; make T009 pass.
- [x] T015 [US1] Route switches through the core in `src/main.rs`: (a) `Message::KnownProjectReopened` — delete `app.stop_active_project_sessions()` and call `app.core.switch_active(&path)`; (b) `Message::FolderChosen` — because `open_or_activate` mutates `active`, either capture the previous active path **before** calling `open_or_activate` and pass the outgoing foreground into the core explicitly, or reorder so `switch_active` records the outgoing foreground first (I1); never drain `terminals` — BS-1.
- [x] T016 [US1] Make crash handling project-aware in `src/main.rs`: rewrite `handle_process_exits` (and `session_cwd`/`with_session`) to resolve the exited session via `workspace.find_session_mut` across all projects, derive cwd from owning project path + `worktree_dir`, apply `on_unexpected_exit`, respawn on `Resume`, and call `core.note_background_restart(id)` when the owner ≠ active — BS-6, BS-8, BS-9.
- [x] T017 [US1] Retire `App::stop_active_project_sessions` (`src/main.rs`): confirm no remaining callers; keep process kill only in `impl Drop for App` and `Message::SessionClosed`. Ensure `Session::stop_for_project_change` is no longer invoked on a mere switch.
- [x] T018 [US1] Render the return notice in `src/ui/shell.rs` (reuse the `worktree_error` banner pattern); add `Message::NoticeDismissed` in `src/app.rs` to clear `core.notice` — SC-007.
- [x] T019 [US1] User-guide docs: document switching without losing running work, background sessions, and the "restarted while you were away" notice in `docs/user-guide/worktrees-and-sessions.md` (Principle VII).

**Checkpoint**: US1 fully functional and independently testable via any existing switch entry point; isolation gate covered.

---

## Phase 4: User Story 2 - Quick project switcher in the top bar (Priority: P2)

**Goal**: A shared switcher control next to the top-bar menu button lists known projects and switches the active one in a single selection, without the folder-browser dialog; unavailable projects are indicated and not selectable; an "Add project…" row opens the existing folder browser.

**Independent Test**: Open the control immediately left of the menu button, confirm it lists known projects with the active one marked, select another available project → it becomes active; select "Add project…" → the folder browser opens.

### Tests for User Story 2 (MANDATORY — Constitution Principle I) ⚠️

> Write these FIRST and confirm they FAIL before implementing.

- [x] T020 [P] [US2] Failing gui-gated test: `ProjectSwitcher` renders one row per known project, marks the active row, disables unavailable rows, and includes a trailing "Add project…" row; selecting an available non-active row emits `Message::KnownProjectReopened(path)`, "Add project…" emits `Message::ProjectSelectorOpened` — contracts/project-switcher-ui.md — in `tests/project_switcher.rs` (gui feature).
- [x] T021 [P] [US2] Failing gui-gated test: the top bar places the switcher trigger immediately left of the `MenuTrigger`, and the switcher panel + overflow menu are mutually exclusive (opening one closes the other) in `tests/project_switcher.rs` (gui feature).

### Implementation for User Story 2

- [x] T022 [US2] Add `Message::ProjectSwitcherToggled` and switcher-open overlay state (mutually exclusive with the overflow menu) in `src/app.rs`; wire the pure toggle.
- [x] T023 [US2] Implement the shared builder primitive `ProjectSwitcher` (trigger + floating panel rows; chainable `.projects()/.on_select()/.on_add()`; terminates in `impl From<ProjectSwitcher> for Element`; theming via `Roles`) in `src/ui/material/project_switcher.rs` — Principle VIII. If a trailing badge/marker is needed, extend the shared `MenuItem`/`menu_overlay` (still builder-style), not a feature-local widget.
- [x] T024 [US2] Export `ProjectSwitcher` from `src/ui/material/mod.rs`.
- [x] T025 [US2] Place the switcher trigger left of the menu trigger in `src/ui/toolbar.rs` via `Toolbar::new(...).action(switcher).action(menu_trigger)`; extend `toolbar::view` to receive the data it renders (known projects + active path) — FR-004.
- [x] T026 [US2] Float the switcher panel as an overlay in `src/ui/mod.rs::view` (reuse the `menu_overlay` path); build rows from `workspace.projects` + `workspace.active`; select → `KnownProjectReopened`, add → `ProjectSelectorOpened` — FR-005/006/008/009.
- [x] T027 [US2] User-guide docs: document the top-bar switcher (location next to the menu button, switching, add-project) in `docs/user-guide/project-selection.md` (Principle VII).

**Checkpoint**: US1 and US2 both work independently.

---

## Phase 5: User Story 3 - See which projects have work running (Priority: P3)

**Goal**: Each switcher row shows whether that project has running background sessions (a count), so the user can tell at a glance where live work is.

**Independent Test**: Start a session in Project A, switch to B, open the switcher: A shows a running-session count badge, B (no sessions) shows none.

### Tests for User Story 3 (MANDATORY — Constitution Principle I) ⚠️

> Write these FIRST and confirm they FAIL before implementing.

- [x] T028 [P] [US3] Failing gui-gated test: a project with N active sessions renders a running-count badge of N; a project with none renders no badge; the badge value tracks `Workspace::running_session_count` — FR-007 — in `tests/project_switcher.rs` (gui feature).

### Implementation for User Story 3

- [x] T029 [US3] Feed `workspace.running_session_count(path)` into each `ProjectRow` and render the count badge in `ProjectSwitcher` rows (`src/ui/material/project_switcher.rs`); pass the counts through `toolbar::view`/`src/ui/mod.rs` — FR-007, research R6.
- [x] T030 [US3] User-guide docs: document the running-session indicator in the switcher in `docs/user-guide/project-selection.md` (Principle VII).

**Checkpoint**: All user stories independently functional.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Cross-cutting cleanup and full validation. (Per-story docs already shipped in their phases.)

- [x] T031 [P] Cross-cutting docs review: cross-reference the switcher (project-selection.md) and background-session behavior (worktrees-and-sessions.md), update any index/nav in `docs/user-guide/`.
- [x] T032 Reconcile stale comments/naming referring to stopping sessions on switch (e.g. FR-023-era "stop the outgoing project's sessions" notes) in `src/main.rs`, now that switching is non-destructive.
- [x] T033 Verify build + full test suite (`cargo test` and `cargo test --no-default-features`) pass on Linux, macOS, and Windows (Principle VI).
- [X] T034 Run `quickstart.md` validation: headless test commands + the 7-step manual walkthrough (SC-001…SC-007).

  > **Run 2026-08-21** — [evidence/T034-quickstart.md](./evidence/T034-quickstart.md). The automated
  > half is green (201 test binaries, 0 failures) and every assertion the quickstart names by
  > behaviour is present. Manual steps 1–5 and 7 pass; **step 6 half-fails** and produced
  > [BUG-003](./bugs/BUG-003.md) — a missing folder is never *indicated*, because availability is
  > only scanned at startup and on a project reopen. Nothing is silently activated, which is FR-008's
  > other clause. SC-005's ~1 s is left unmeasured: `import` alone costs ~300 ms and lavapipe is a
  > software rasteriser.
  >
  > A second defect turned up outside the numbered steps: selecting a project leaves the switcher
  > panel open, against the contract's "Panel closes." — [BUG-002](./bugs/BUG-002.md).

## Phase 7: Convergence

- [X] T035 (regression, introduced by feature 010's daemon migration) `State::note_background_restart`
  — which raises the FR-011/SC-007 "a background session was restarted while you were away"
  notice — had zero production call sites; it was only ever invoked by its own isolated test
  (`tests/background_restart.rs`). Originally the client itself supervised session
  restarts and could call it directly; once feature 010 moved all restart/crash-loop
  supervision into the daemon, the client only learns lifecycle changes by periodically
  reconciling a `CatalogSnapshot`, and nothing in that reconcile path ever detected "a
  session just transitioned into `Restarting`" to raise the marker. Fixed
  `reconcile_catalog` in `src/main.rs` to compare each session's incoming lifecycle against
  its previous value and call `note_background_restart` on a transition into `Restarting`
  (the method itself no-ops for the active project, so no additional active/inactive check
  is needed at the call site). Regression test:
  `reconcile_detects_a_background_restart_and_arms_the_return_notice` (inline
  `#[cfg(test)] mod tests` in `main.rs`, alongside the existing `reconcile_catalog` tests),
  confirmed red without the fix and green with it. Per FR-011 (missing).

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately.
- **Foundational (Phase 2)**: Depends on Setup. Provides the `Workspace` query helpers consumed by US1 (`find_session_mut`) and US3 (`running_session_count`).
- **User Stories (Phase 3–5)**: Depend on Foundational. US1 is the MVP. US2 and US3 depend on Foundational; US3 additionally builds on the `ProjectSwitcher` created in US2.
- **Polish (Phase 6)**: After all targeted stories.

### User Story Dependencies

- **US1 (P1)**: Independent — testable via the existing body "Known projects" switch entry; needs only Foundational (`find_session_mut`).
- **US2 (P2)**: Independent of US1 — the switcher switches projects via the existing `KnownProjectReopened` path (behaves per whatever switch semantics are in place). Needs Foundational.
- **US3 (P3)**: Depends on **US2** (extends `ProjectSwitcher` rows with a count) and Foundational (`running_session_count`).

### Within Each User Story

- Tests written and FAILING before implementation (Principle I).
- Core state/helpers before gui wiring; shared UI primitive before its placement/rows.
- User-guide docs ship in the same story (Principle VII).
- Story complete only when its tests pass, docs exist, and it builds on all three platforms.

### Parallel Opportunities

- Setup T001 runs alone.
- Foundational T002 and T004 (test-writing, different concerns) are [P]; their impls T003/T005 follow.
- US1 tests T006–T011 are all [P] (distinct files) — write them together, then implement T012–T019.
- US2 tests T020–T021 are [P]; US3 test T028 stands alone.
- Docs review T031 is [P] with T032.

---

## Parallel Example: User Story 1

```bash
# Write all US1 failing tests together (distinct files):
Task: "Core test: switch keeps outgoing sessions Running (BS-1) in tests/switch_active.rs"
Task: "Core test: foreground restore + record-before-activate (BS-3, I1) in tests/switch_active.rs"
Task: "Core test: switch to unavailable rejected (BS-10) in tests/switch_active.rs"
Task: "Core test: restart-while-inactive marker + notice (BS-7) in tests/background_restart.rs"
Task: "GUI test: terminals retained + background PTY respawn (BS-1/2/6) in tests/terminal_background.rs"
Task: "Integration test: 3-project concurrent isolation + no cap (BS-4/5, FR-010/013) in tests/session_isolation.rs"

# Then implement core before gui:
Task: "Add State fields in src/app.rs"
Task: "Implement State::switch_active (record-before-activate) in src/app.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 only)

1. Phase 1: Setup.
2. Phase 2: Foundational (`find_session_mut` is required by US1).
3. Phase 3: US1 — non-destructive switching + background crash/notify + isolation.
4. **STOP and VALIDATE**: switch away and back from any existing entry point; confirm sessions survive, background crashes are restarted + surfaced, and concurrent projects stay isolated.
5. Demo — this alone delivers the core user value (safe switching), even without the new switcher UI.

### Incremental Delivery

1. Setup + Foundational → helpers ready.
2. US1 → non-destructive switching (MVP) → demo.
3. US2 → top-bar switcher → demo.
4. US3 → running-session indicators → demo.

Each story adds value without breaking the previous ones.

---

## Notes

- [P] = different files, no dependency on an incomplete task.
- Verify each test fails before implementing it.
- Commit after each task or logical group.
- Behavior invariants referenced as BS-* live in `contracts/background-session-lifecycle.md`; UI contract in `contracts/project-switcher-ui.md`.
- No `Cargo.toml` / persisted-schema changes (plan.md, data-model.md).
