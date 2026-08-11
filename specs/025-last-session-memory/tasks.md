# Tasks: Reopen on the session I was last using

**Input**: Design documents from `/specs/025-last-session-memory/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/](./contracts/), [quickstart.md](./quickstart.md)

**Tests**: Per Constitution Principle I (Test-First, NON-NEGOTIABLE), test tasks are MANDATORY and
written to fail first. The one claim no test in this repository can make is that any of it survives
an actual restart — every test runs in one process. That is quickstart §B, and it is the feature.

**Documentation**: Per Principle VII, each user-facing story carries its user-guide task in the same
change, in `docs/user-guide/worktrees-and-sessions.md` under
`## Starting, switching, and closing sessions` — the section that already describes switching and
the current-session mark.

**Cross-platform**: Per Principle VI, nothing here branches on platform — a serialised field and a
lookup, over paths the store already canonicalises.

**Build commands**: `mise run test` (whole workspace, matches CI), `mise run test-core` (store and
workspace only, much faster while iterating on persistence). Never raw `cargo` — see CLAUDE.md.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2, US3)

---

## Phase 1: Setup

- [X] T001 Confirm a green baseline with `mise run test` before editing anything under `crates/`, so a later red test is this feature's and not inherited

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Give the memory a home that can be persisted. Every story reads or writes it, so
nothing else can start until it has moved.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [X] T002 Write failing tests in `crates/micold-core/tests/store_roundtrip.rs`: `last_session` survives save → load; a project with no memory round-trips as `None`; and a per-project file written **without** the field loads as `None` rather than failing — the claim that lets this ship with no `schema_version` bump (research R7, §1.3)
- [X] T003 Write failing tests in `crates/micold-core/tests/workspace.rs`: `foreground_by_project` is keyed by canonicalised path, exactly as `Workspace::sessions` is, so the two can never be looked up differently (data-model, research R2)
- [X] T004 Add `foreground_by_project: BTreeMap<PathBuf, SessionId>` to `Workspace` in `crates/micold-core/src/workspace.rs`, with a doc comment saying what it is and that the daemon owns writing it (invariant I2)
- [X] T005 Add `last_session: Option<SessionId>` to `StoredProjectState` in `crates/micold-core/src/store.rs` (`#[serde(default)]`), and carry it both ways: `save()` writes each project's entry, `load()` populates `Workspace::foreground_by_project`. It is per-project data about that project's sessions, so it belongs in the per-project file beside them — and in the file `remove_project_state` already deletes
- [X] T006 Remove `foreground_by_project` from `State` in `crates/micold-client/src/app.rs` and point `record_foreground` / `explain_foreground` in `crates/micold-client/src/features/session.rs` at the new home. Behaviour is unchanged — only where they read from is
- [X] T007 Confirm `crates/micold-client/tests/features_session.rs` and `crates/micold-client/tests/switch_active.rs` still pass unedited. They are the proof the move changed no behaviour; needing to edit them means it did

**Checkpoint**: The memory has a home on disk and is loaded from it. Nothing yet writes it, and
launch still starts on the overview.

---

## Phase 3: User Story 1 - Pick up where I left off (Priority: P1) 🎯 MVP

**Goal**: Quitting with a session in front of you and reopening puts you back on it.

**Independent Test**: Open a project, select a session, quit, start again. The same session is in
front of you, with no clicks.

### Tests for User Story 1 (MANDATORY — Constitution Principle I) ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [X] T008 [P] [US1] Failing test in `crates/micold-daemon/` covering `SetViewedSession`: a report naming a session records it against that project and persists; a second report naming the **same** session writes nothing (§2.3, FR-001a) — attach re-sends the current id and a session start may name the session already in front of the user
- [X] T009 [P] [US1] Failing test in `crates/micold-daemon/`: a report of **no session** leaves the memory untouched (§2.6, FR-005a). This is the clause that stops closing a session from silently costing the user the place they would have returned to
- [X] T010 [P] [US1] Failing test in `crates/micold-client/tests/app_state.rs`: applying a project's memory makes that session current — and does so through the path that arms feature 024's reveal, so the row is listed open (§4.1, FR-012)
- [X] T011 [P] [US1] Failing test in `crates/micold-client/tests/app_state.rs`: applying a memory **starts nothing** — every session's lifecycle after is exactly what it was before (§3.3, FR-004, SC-005)
- [X] T012 [P] [US1] Failing test in `crates/micold-client/tests/terminal_focus.rs`: applying a memory leaves `terminal_focused` false, while a project *switch* still focuses. The two paths differ here deliberately and only a test keeps them apart (§3.4, FR-013, research R5)

### Implementation for User Story 1

- [X] T013 [US1] Record and persist the memory in `crates/micold-daemon/src/catalog.rs`: a method mutating `workspace.foreground_by_project` then calling `self.persist()`, modelled on `set_worktree_display_name` (`catalog.rs:257`) which is the same shape for the same reason
- [X] T014 [US1] Call it from `State::set_viewed` in `crates/micold-daemon/src/state.rs:485`, guarded by the two conditions from T008/T009: only when the reported session is `Some`, and only when it differs from what is remembered
- [X] T015 [US1] Apply the memory at launch in `boot()` in `crates/micold-client/src/main.rs` (after `prune_empty_sessions`, which is what makes a memory naming a pruned session stop resolving): resolve with `explain_foreground` and apply with `set_current_session`
- [X] T016 [US1] ~~Do **not** call `restore_after_activation` from `boot()` in `crates/micold-client/src/main.rs`, and say why in a comment there: feature 023 added `focus_terminal()` to it, and FR-013 says a launch must not put keystrokes into a terminal. Reusing the function would import the focus with the restore (research R5)
- [X] T017 [US1] Document reopening in `docs/user-guide/worktrees-and-sessions.md` under `## Starting, switching, and closing sessions`: the app reopens on the session you were last using in that project, whether or not it is still running, and it does not start or focus anything

**Checkpoint**: US1 is the feature. Shippable alone — the remaining stories are about behaving
sensibly when it cannot be honoured.

---

## Phase 4: User Story 2 - The memory is per project, and survives switching (Priority: P2)

**Goal**: Each project reopens on its own last session, and switching after launch uses that
project's memory too.

**Independent Test**: Two projects left on different sessions. Quit, restart, switch between them —
each lands on its own.

### Tests for User Story 2 (MANDATORY — Constitution Principle I) ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [X] T018 [P] [US2] Failing test in `crates/micold-core/tests/store_roundtrip.rs`: two projects with different memories round-trip independently, and writing one does not disturb the other (§1.2, FR-008)
- [X] T019 [P] [US2] Failing test in `crates/micold-client/tests/switch_active.rs`: switching to a project **not yet visited in this run** uses the memory loaded from disk, not just one recorded since launch (FR-008) — the case that distinguishes a persisted memory from the in-memory one that already existed
- [X] T020 [P] [US2] Failing test in `crates/micold-client/tests/switch_active.rs`: switching several times then reading each project's memory gives the session last current *in that project*, not the one current at the end (US2 scenario 2)

### Implementation for User Story 2

- [X] T021 [US2] Confirm no code change is needed in `crates/micold-client/src/features/session.rs` beyond Phase 2 and US1 — the map is already per project and `record_foreground` already keys by the active project. If a test above fails, the defect is in the move (T004–T006), not in new behaviour; fix it there rather than adding a second mechanism
- [X] T022 [US2] Document the per-project behaviour in `docs/user-guide/worktrees-and-sessions.md`: each project remembers its own session, and switching takes you to that project's

**Checkpoint**: US1 + US2 — every project reopens where you left it.

---

## Phase 5: User Story 3 - It behaves sensibly when the session is gone (Priority: P2)

**Goal**: A memory that cannot be honoured falls back quietly, damages nothing, and never blocks a
launch.

**Independent Test**: Close the remembered session, or delete its worktree, then restart. The app
starts normally on the overview or another session, with the rest of the project untouched.

### Tests for User Story 3 (MANDATORY — Constitution Principle I) ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [X] T023 [P] [US3] Failing test in `crates/micold-client/tests/features_session.rs`: a memory naming a **closed** (archived) session is not restored, and one naming an **absent** session falls back to the existing behaviour — first running session, else none (§3.2, FR-005, FR-007)
- [X] T024 [P] [US3] Failing test in `crates/micold-client/tests/app_state.rs`: a memory that cannot be honoured leaves every other location's open/closed state alone (§3.6, FR-006)
- [X] T025 [P] [US3] Failing test in `crates/micold-core/tests/store_fault_isolation.rs`: an unreadable or malformed per-project file yields a usable workspace with no memory and no error — a launch must not fail, warn or block on it (§3.7, FR-010)
- [X] T026 [P] [US3] Failing test in `crates/micold-client/tests/switcher_forget_menu.rs`: forgetting a project discards its memory, and re-opening the same folder later starts without one (§2.5, FR-009)
- [X] T027 [P] [US3] Failing test in `crates/micold-core/tests/store_roundtrip.rs`: closing the session a memory names does **not** erase the memory — only another session becoming current replaces it (FR-005a, invariant I0). The stored id may name a session that can never be restored, and that is the intended state

### Implementation for User Story 3

- [X] T028 [US3] Confirm the fallbacks need no new code in `crates/micold-client/src/features/session.rs`: `explain_foreground` already declines an archived session and already returns `NoSessionsForKey` / `NoneActive`, and `load_project_state` already recovers from an unreadable file. Any test above that fails names a real gap — fix it where the behaviour lives, not at the launch site
- [X] T029 [US3] Document the fallbacks in `docs/user-guide/worktrees-and-sessions.md`: if the session you were on has been closed or its worktree removed, the app opens on the project as it would otherwise, and nothing else about it changes

**Checkpoint**: All three stories independently functional.

---

## Phase 6: Polish & Cross-Cutting Concerns

- [X] T030 Run the whole automated gate: `mise run test`, and record which of quickstart §A's rows each new test satisfies
- [X] T031 Confirm `crates/micold-core/tests/schema_hash.rs` is **unchanged**. This feature adds no protocol message and edits none; if the hash moved, something reached for the wire that did not need to (research R3)
- [X] T032 Run quickstart §B (B1–B7) by hand and fill in the recording table in [quickstart.md](./quickstart.md). B1, B2 and B3 need the process actually stopped and started, so none of them can be automated — if §B was not run, say so there rather than leaving the table blank
- [X] T033 [P] Cross-cutting docs review in `docs/`: confirm the three added passages read as one narrative and that nothing elsewhere still says the app forgets your session at exit
- [X] T034 Confirm CI is green on Linux, macOS and Windows for `.github/workflows/ci.yml` (Principle VI) — this feature has no platform branch, so a platform-specific failure means a path or serialisation assumption leaked

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: no dependencies
- **Foundational (Phase 2)**: depends on Setup — **blocks every user story**, because it is what makes the memory persistable at all
- **US1 (Phase 3)**: depends on Phase 2
- **US2 (Phase 4)**: depends on Phase 2 and on US1's write path (T013/T014) — without it nothing is ever recorded to be per-project about
- **US3 (Phase 5)**: depends on Phase 2 and on US1's launch path (T015)
- **Polish (Phase 6)**: depends on the stories being delivered

### Within Each User Story

- Tests are written and FAIL before implementation (Principle I)
- The store and workspace change before the daemon writes it, which comes before the launch reads it
- The user-guide task ships in the same change as its story (Principle VII)

### Parallel Opportunities

- T008–T012 (US1 tests) span four files and are genuinely parallel; T010 and T011 share `tests/app_state.rs` and should be sequenced or committed together
- T018–T020 (US2 tests) — T018 is in core, T019/T020 share `tests/switch_active.rs`
- T023–T027 (US3 tests) span four files; only T024 and T027's homes are shared with earlier work
- T002 and T003 (foundational tests) are different files and can be written together
- The four user-guide tasks (T017, T022, T029, T033) touch the same file and must not run in parallel

**Parallel example — User Story 1 tests:**

```bash
Task: "Failing daemon test for recording and de-duplicating SetViewedSession"
Task: "Failing daemon test that a no-session report does not clear the memory"
Task: "Failing test that applying a memory starts nothing, in tests/app_state.rs"
Task: "Failing test that a launch does not focus the terminal, in tests/terminal_focus.rs"
```

---

## Implementation Strategy

### MVP (User Story 1 only)

1. Phase 1 → Phase 2 → Phase 3
2. **STOP and VALIDATE**: quickstart §B1, §B2 and §B3 — you land on the session, nothing started,
   nothing focused
3. Shippable. US1 is the feature; US2 and US3 make it behave well when the world has changed
   underneath it

### Incremental Delivery

1. Setup + Foundational → the memory has a home and is loaded; nothing visible changes
2. **+ US1** → reopening lands where you left (MVP)
3. **+ US2** → every project, and switching after launch
4. **+ US3** → closed sessions, deleted worktrees, unreadable files

### Where the risk is (research R8)

1. **T004–T006, the move.** `foreground_by_project` becomes state the daemon also writes. The risk
   is ownership rather than the move itself: the client must keep reading and never persist, exactly
   as it already does for sessions. T007 is the check that the move changed no behaviour
2. **T016, not focusing.** Easy to get wrong by reusing the switch path, and invisible to a test
   that only asserts which session is current — which is why T012 exists separately
3. **T014's two conditions.** Getting `None`-ignores-the-memory wrong means closing a session
   silently costs the user their place; getting the de-duplication wrong means rewriting a file that
   holds every session record, on every attach
4. Everything else — a defaulted field and a lookup that already exists

---

## Notes

- 34 tasks: 1 setup, 6 foundational, 10 US1, 5 US2, 7 US3, 5 polish
- Two tasks (T021, T028) expect to find **no code needed**. They are deliberately not omitted: if a
  test in their phase fails, the finding is that the move or the launch path is wrong, and the fix
  belongs there rather than in a second mechanism bolted on beside it
- No protocol message is added or changed, so no schema hash moves and no version handshake shifts
- Commit after each task or logical group; stop at any checkpoint to validate a story on its own
