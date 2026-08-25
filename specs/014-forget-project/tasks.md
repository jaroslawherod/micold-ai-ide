---
description: "Task list for feature: Forget a Project"
---

# Tasks: Forget a Project

**Input**: Design documents from `/specs/014-forget-project/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/ (all present)

**Tests**: MANDATORY per Constitution Principle I (Test-First, NON-NEGOTIABLE). Every unit of
core/reducer production code is preceded by a failing, reviewed test (Red → Green → Refactor).
GUI/process-spawn glue in `src/main.rs` + `src/ui/` (no decision logic) is validated by
`quickstart.md` per the named Principle I exception.

**Documentation**: MANDATORY per Principle VII. Each user-facing story ships its
`docs/user-guide/project-selection.md` update in the same change.

**Cross-platform**: Per Principle VI, all logic is platform-agnostic (path handling via existing
`project::canonicalize_best_effort`); CI runs the suite on Linux, macOS, and Windows.

**Organization**: Tasks are grouped by user story. The shared forget mechanism (pure
`Workspace::forget` + scaffolding) is Foundational because all three stories depend on it.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies on incomplete tasks)
- **[Story]**: US1 / US2 / US3 (Setup, Foundational, Polish carry no story label)
- Exact file paths included in each description

---

## Phase 1: Setup

**Purpose**: Establish a clean, green baseline so later Red tests are meaningful.

- [X] T001 Confirm the baseline suite is green by running `mise run test` (`cargo test --no-default-features --all-targets`) from the repo root; record any pre-existing failures before adding new code.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The shared forget mechanism used by every user story — pure core operation, the
session-lookup helper, and the message/overlay/state scaffolding needed to compile the tests.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [X] T002 Add scaffolding to `src/app.rs`: the `Message` variants `ProjectForgetRequested(PathBuf)`, `ProjectForgetConfirmed`, `ProjectForgetCancelled`; the `Overlay::ConfirmForgetProject` variant; and the `State.forget_target: Option<PathBuf>` field (doc-commented as transient, never persisted). Declarations only — no reducer behavior yet (structural, no decision logic).
- [X] T003 [P] Write FAILING unit tests for `Workspace::forget` in `tests/workspace.rs` per contract obligations T1, T2, T3, T4, T6, T7 (remove a non-active record leaving others + `active` intact; forget the active project clears `workspace.active`; forget the only project empties `projects` and clears `active`; dropping the project also drops its `sessions[path]` and `worktree_names[path]`; unknown/already-forgotten path is a no-op; a non-canonical path spelling still matches). Observe them fail (Red).
- [X] T004 [P] Write a FAILING unit test for `State::session_ids_of_project(&Path) -> Vec<SessionId>` in `tests/app_state.rs` (returns all recorded session ids for a project path, canonicalized; empty for unknown path). Observe it fail (Red).
- [X] T005 Implement `Workspace::forget(&mut self, path: &Path)` in `src/workspace.rs` per `contracts/workspace-forget.md` (canonicalize; `projects.retain`; `sessions.remove`; `worktree_names.remove`; clear `active` iff it equalled the path). Make T003 pass (Green).
- [X] T006 Implement `State::session_ids_of_project` in `src/app.rs` (read `workspace.sessions[canonicalize_best_effort(path)]`, collect `SessionId`s), mirroring `sessions_in_worktree`. Make T004 pass (Green).

**Checkpoint**: The pure removal operation and its helper exist and are unit-tested; scaffolding
compiles. User stories can now begin.

---

## Phase 3: User Story 1 - Forget a known project and remove it from the list (Priority: P1) 🎯 MVP

**Goal**: A user can invoke Forget on any listed project, confirm a dialog that states nothing on
disk is deleted (and, when the project has running sessions, how many will be stopped), and see the
entry removed, its live processes stopped, and the removal persisted across restart.

**Independent Test**: Open ≥2 projects; forget a non-active one; confirm; verify it is removed while
others remain, the folder/worktrees are untouched on disk, and it does not reappear after restart.

### Tests for User Story 1 (write FIRST — MANDATORY, observe Red) ⚠️

- [X] T007 [P] [US1] Write FAILING integration tests in `tests/forget_project.rs`: `ProjectForgetRequested(path)` sets `forget_target = Some(path)` and opens `Overlay::ConfirmForgetProject`; `ProjectForgetCancelled` closes the overlay, clears `forget_target`, and leaves `workspace` unchanged (FR-004); `ProjectForgetConfirmed` removes a non-active target from `projects` while other projects remain (FR-003) and clears `forget_target`/overlay.
- [X] T008 [P] [US1] Write a FAILING persistence test in `tests/store_roundtrip.rs`: after forgetting a project and saving via the store, a reload's `Workspace` does not contain the forgotten project, and surviving projects + the `active` pointer are intact (FR-007). **AND** (post-rebase, per-project storage) a project with sessions has its per-project state file at `store.project_state_path(path)` before forget; after `store.remove_project_state(path)` (or the forget+save flow) the file no longer exists and a fresh `load` yields no sessions for that path (FR-005/FR-012 — no session resurrection). Observe Red.

### Implementation for User Story 1 (make the above Green)

- [X] T009 [US1] Implement the reducer arms in `src/app.rs`: `ProjectForgetRequested` (set target + open overlay), `ProjectForgetConfirmed` (call `workspace.forget(target)`, clear `forget_target`, close overlay), `ProjectForgetCancelled` (clear `forget_target`, close overlay). Make T007 pass. (Active-session clearing is added in US2.)
- [X] T010 [P] [US1] Create `src/ui/confirm_forget.rs`: a confirmation view built on the shared `ui::material::Modal` (mirroring `confirm_delete.rs`) with title `Forget "<display_name>"?`, body line 1 stating only the remembered entry is removed and nothing on disk (folder, files, worktrees) is deleted (FR-002), a conditional body line 2 `This will stop {n} running session(s).` shown only when the passed running-session count `n > 0` (FR-002a), and **Forget** (`ProjectForgetConfirmed`, danger/filled) + **Cancel** (`ProjectForgetCancelled`, outlined) actions.
- [X] T011 [US1] Add a **Forget** button (using `Icon::Delete`, outlined/danger style) to each known-projects entry row in `src/ui/shell.rs`, dispatching `Message::ProjectForgetRequested(project.path.clone())` (place it after the existing Rename button).
- [X] T012 [US1] Route `Overlay::ConfirmForgetProject` to `confirm_forget::modal` in `src/ui/mod.rs`, resolving the target project's `display_name` and computing its running-session count via `workspace.running_session_count(&target)`; and map the overlay's Escape/scrim dismissal to `Message::ProjectForgetCancelled` in `src/app.rs` (alongside the existing `ConfirmWorktreeDelete` dismiss mapping).
- [X] T012b [US1] Add `JsonFileStore::remove_project_state(&self, project_path: &Path) -> io::Result<()>` in `src/store.rs` that deletes the file at `project_state_path(project_path)`, treating a "not found" result as success (FR-005). Make T008's state-file assertions pass. (Post-rebase: per-project storage from `main`'s `fix/state-lost`.)
- [X] T013 [US1] Add the binary handler for `Message::ProjectForgetConfirmed` in `src/main.rs`: before delegating, for each id in `app.core.session_ids_of_project(&target)` do `if let Some(mut st) = app.terminals.remove(&id) { st.kill_all(); }` (stop AI CLI + shell processes, FR-010; no worktree/FS removal, FR-006); then `app.core.update(Message::ProjectForgetConfirmed)`; then `persist(&mut app.core)` (FR-007; note `persist` now takes `&mut State`); then delete the project's per-project state file via the store's `remove_project_state(&target)` (FR-005/FR-012). Wire `ProjectForgetRequested`/`ProjectForgetCancelled` through the default pure-update arm.
- [X] T014 [US1] Document the **Forget** action in `docs/user-guide/project-selection.md`: how to invoke it, the confirmation, the non-destructive-to-disk guarantee, and the running-session-count warning (FR-002/FR-002a/FR-006).

**Checkpoint**: The full basic forget flow works end to end for any project, including stopping its
running sessions and persisting the removal. MVP deliverable.

---

## Phase 4: User Story 2 - Forget the currently active project (Priority: P2)

**Goal**: Forgetting the active project additionally clears the active working space (and the
active session), and forgetting the last project returns the shell to the first-run empty state.

**Independent Test**: Make a project active with a running session; forget it; verify there is no
active working space afterward, the session is stopped, worktrees remain on disk, and if it was the
only project the empty state is shown.

### Tests for User Story 2 (write FIRST — MANDATORY, observe Red) ⚠️

- [X] T015 [P] [US2] Write FAILING integration tests in `tests/forget_project.rs`: `ProjectForgetConfirmed` on the **active** project leaves no active project (`workspace.active == None`) and clears `active_session` to `None` (FR-008); forgetting the only known project leaves `workspace.projects` empty (the empty-state precondition, FR-009); forgetting a non-active background project leaves `active`/`active_session` unchanged.

### Implementation for User Story 2 (make the above Green)

- [X] T016 [US2] Extend the `ProjectForgetConfirmed` reducer arm in `src/app.rs`: capture `was_active = workspace.active == canonicalize(target)` before `forget`, and after `forget` set `self.active_session = None` when `was_active` (leave it untouched otherwise). Make T015 pass.
- [X] T017 [US2] Update `docs/user-guide/project-selection.md` to document forgetting the active project: the active working space is cleared, its running sessions are stopped, and forgetting the last project returns to the empty state (FR-008/FR-009/FR-010).

**Checkpoint**: Forgetting the active or last project behaves correctly; US1 still works.

---

## Phase 5: User Story 3 - Forget an unavailable project (Priority: P3)

**Goal**: The Forget action is available for projects whose folder is gone (marked Unavailable) and
removes them identically to available ones.

**Independent Test**: Create a known project, remove its folder so it is Unavailable, forget it, and
verify it is removed and does not reappear after restart.

### Tests for User Story 3 (write FIRST — MANDATORY, observe Red) ⚠️

- [X] T018 [P] [US3] Write a FAILING unit test in `tests/workspace.rs` (contract obligation T5): `Workspace::forget` removes a project whose `availability == Unavailable` exactly as it removes an `Available` one.
- [X] T019 [P] [US3] Write a FAILING integration test in `tests/forget_project.rs`: `ProjectForgetRequested` followed by `ProjectForgetConfirmed` for an **Unavailable** project's path opens the confirmation and removes the entry (the forget affordance is not gated by availability, FR-011).

### Implementation for User Story 3 (make the above Green)

- [X] T020 [US3] Ensure the **Forget** button in `src/ui/shell.rs` is rendered and enabled for `Unavailable` entries (unlike **Open**, it must NOT be disabled by availability). Adjust the entry-row logic if the button was gated in T011. Make T019's UI intent hold and keep T018 green.
- [X] T021 [US3] Update `docs/user-guide/project-selection.md` to note that Forget is the way to remove a stale/unavailable project entry (FR-011).

**Checkpoint**: All three stories independently functional.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Final verification across stories and platforms.

- [X] T022 [P] Review `docs/user-guide/project-selection.md` for a single coherent Forget section (merge the incremental US1/US2/US3 additions if fragmented) and confirm any docs index/nav references are correct.
- [X] T023 Run the `quickstart.md` manual validation (Scenarios A–D) against `mise run run`, confirming the confirmation modal, session-stop count, empty state, and non-destructive-to-disk behavior. *(2026-08-20 and 2026-08-21: run headlessly on Xvfb + lavapipe against this branch's pinned client/daemon pair, recorded in [evidence/manual-scenarios.md](./evidence/manual-scenarios.md). **All four scenarios pass.** A–C on 2026-08-20: the modal's wording, the session-stop count in both its plural and singular forms, no orphaned `claude` processes, the first-run empty state, the unavailable-project row, and the repo untouched on disk. D completed 2026-08-21 — a project carrying a custom name, a worktree-name override and a live session came back from a forget/re-open round trip with the folder-name default, the default worktree name, no session and **no duplicate entry** (FR-012).)*

  > **Run 2026-08-20** — Xvfb + lavapipe, per the repo's `visual-pass` skill:
  > [evidence/manual-scenarios.md](./evidence/manual-scenarios.md). Scenarios **A, B and C pass**,
  > including the session-stop count ("This will stop 2 running sessions."), no orphaned processes,
  > the empty state, and the non-destructive-to-disk guarantee. **D is partial**: its step 4
  > (no session resurrection — the per-project state file is deleted) is confirmed; steps 1–3, which
  > need the folder browser driven to re-open the same folder, were not run. Stays open on those.
- [X] T024 Verify the full suite builds and passes on Linux, macOS, and Windows via CI (Principle VI), then run `mise run test` locally as a final gate. *(2026-08-20: CI half satisfied by the three-OS matrix added in `10a1fe7` — latest green run [32302430171](https://github.com/jaroslawherod/micold-ai-ide/actions/runs/32302430171), all three matrix jobs `success`; the full GUI suite and clippy stay Linux-only by design. Local gate run on this branch: `mise run test` — **202 test binaries, 1970 passed, 0 failed**, no panics.)*

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately.
- **Foundational (Phase 2)**: Depends on Setup — BLOCKS all user stories (all stories call `Workspace::forget` and reference the new messages/overlay/state).
- **User Stories (Phase 3–5)**: All depend on Foundational. US1 is the MVP; US2 and US3 layer thin specializations on top.
- **Polish (Phase 6)**: Depends on all targeted stories being complete.

### User Story Dependencies

- **US1 (P1)**: Depends only on Foundational. Delivers the complete basic mechanism (button, modal, reducer, binary handler, persistence, docs).
- **US2 (P2)**: Depends on Foundational; extends US1's `ProjectForgetConfirmed` reducer arm (T016 builds on T009). Independently testable via the reducer.
- **US3 (P3)**: Depends on Foundational; touches the US1 shell button (T020 may adjust T011). Independently testable via `forget` unit + reducer integration.

### Within Each Story

- Tests are written and observed FAILING before implementation (Principle I).
- Pure core / reducer before UI wiring before binary glue.
- User-guide docs ship in the same story (Principle VII).

### Parallel Opportunities

- **Foundational**: T003 and T004 are `[P]` (different test files); T005/T006 follow.
- **US1**: T007 and T008 are `[P]` (different test files). T010 (`confirm_forget.rs`, new file) is `[P]` with T009/T011 work on other files.
- **US2**: single test (T015) then single impl (T016).
- **US3**: T018 and T019 are `[P]` (different test files).
- Across stories: once Foundational is done, US1/US2/US3 test-writing can be drafted in parallel, but US2's T016 and US3's T020 both build on US1 code, so implementation is cleanest in priority order.

---

## Parallel Example: Foundational + User Story 1

```bash
# Foundational — write the two failing unit tests in parallel (different files):
Task: "Failing Workspace::forget unit tests in tests/workspace.rs"          # T003
Task: "Failing State::session_ids_of_project unit test in tests/app_state.rs" # T004

# User Story 1 — write the two failing tests in parallel (different files):
Task: "Failing reducer-flow integration tests in tests/forget_project.rs"   # T007
Task: "Failing persistence test in tests/store_roundtrip.rs"                 # T008

# User Story 1 — the new modal file can be built alongside shell/reducer edits:
Task: "Create src/ui/confirm_forget.rs (shared Modal confirmation)"          # T010
```

---

## Implementation Strategy

### MVP First (User Story 1)

1. Phase 1 Setup → 2. Phase 2 Foundational (CRITICAL — blocks all stories) → 3. Phase 3 US1.
4. **STOP and VALIDATE**: forget any project, confirm, sessions stop, removal persists.
5. Demo — this is a fully usable feature on its own.

### Incremental Delivery

1. Foundational → shared `forget` core ready.
2. US1 → complete basic forget flow (MVP) → validate → demo.
3. US2 → active-project/empty-state handling → validate → demo.
4. US3 → unavailable-entry removal → validate → demo.

Each story adds value without breaking the previous ones.

---

## Notes

- `[P]` = different files, no dependency on an incomplete task.
- `[Story]` labels (US1/US2/US3) give traceability back to spec.md.
- Verify every test fails before implementing it (Principle I is NON-NEGOTIABLE).
- The only code not under an automated test is the `src/main.rs` process-kill/persist glue and the
  `src/ui/` rendering — validated by `quickstart.md` under the Principle I GUI-wiring exception.
- Commit after each task or logical group; stop at any checkpoint to validate a story independently.
