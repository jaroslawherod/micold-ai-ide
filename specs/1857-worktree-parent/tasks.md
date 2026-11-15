---

description: "Task list for worktree parent registration (MCP + sidebar) and supervisor support"
---

# Tasks: Worktree parent registration (MCP + sidebar) and supervisor support

**Input**: `specs/1857-worktree-parent/` (spec.md, plan.md, research.md, data-model.md, contracts/, quickstart.md)
**Prerequisites**: plan.md, spec.md
**Tests**: mandatory and first (Constitution I): each implementation task is preceded by a failing test task.
**Documentation**: each user-facing story carries its user-guide task (Constitution VII).
**Milestones** (one GitHub issue each): **M1** MCP and persistence (Phases 1-4, 6; US1, US2, US3, US5), **M2** sidebar UI (Phase 5; US4; blocked by M1), **M3** supervisor integration (Phase 7; blocked by M1; consumed by #1813, #1814, #1816).

## Milestones

Each milestone merges to `main` on its own (speckit-autopilot). The ledger
(`specs/1857-worktree-parent/autopilot.md`) records their PRs.

### M1 — MCP and persistence (#1858)

- **Tasks**: T001–T014, T020–T022
- **Satisfies**: US1, US2, US3, US5

### M1b — Visibility by parent (#1858)

- **Tasks**: T030–T032
- **Satisfies**: FR-015, FR-016, FR-017

### M2 — Sidebar parent label and filter (#1859)

- **Tasks**: T015–T019
- **Satisfies**: US4

### M3 — Supervisor integration

- **Tasks**: T023–T026, T033
- **Satisfies**: FR-013

## Format: `- [ ] [TaskID] [P?] [Story?] Description with file path`

## Phase 1: Setup (M1)

- [x] T001 Add the types `WorktreeId` (uuid newtype), `ParentRef` (`Default` | `Worktree(WorktreeId)`) and `ParentState` (`Known` | `Removed`) in `crates/micold-core/src/worktree.rs` with serde forms from `contracts/storage-parent.md`

## Phase 2: Foundational (M1; blocks every story)

- [x] T002 [P] Add failing store tests in `crates/micold-core/src/store.rs` (test module): `worktree_ids` and `worktree_parents` round-trip; both omitted when empty; a file without them loads empty; the legacy-catalog fallback leaves them empty
- [x] T003 Add `worktree_ids: BTreeMap<String, WorktreeId>` and `worktree_parents: BTreeMap<WorktreeId, ParentLink {parent, removed, last_name}>` to the per-project state in `crates/micold-core/src/store.rs`, `#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]`, no schema bump (`data-model.md`); T002 goes green
- [x] T004 [P] Add failing catalog tests in `crates/micold-daemon/tests/` (new `worktree_parent_catalog.rs`): first catalog refresh mints an id for every live worktree and keeps ids across refreshes and renames; reconcile sets `removed` on links whose target id has no live worktree; `Default` links are never removed
- [x] T005 Implement id minting and the reconcile pass in `crates/micold-daemon/src/catalog.rs` next to the `origin` resolution (`catalog.rs:2336`); persist through the store; T004 goes green

**Checkpoint**: ids and tombstones exist; nothing is exposed yet.

## Phase 3: User Story 1 - A supervisor finds its own runs (Priority: P1) 🎯 MVP (M1)

**Goal**: `create_worktree` records the parent; `list_worktrees` returns it and filters by it.
**Independent test**: `quickstart.md` Part A steps 1-3.

- [x] T006 [P] [US1] Add failing schema tests in `crates/micold-core/tests/mcp_tools_catalog.rs`: `create_worktree` and `list_worktrees` accept an optional `parent` string; unknown extra args still refused
- [x] T007 [P] [US1] Add failing daemon tests in `crates/micold-daemon/tests/mcp/mcp_worktree_parent.rs` (new): root session creates -> `parent: "default"`; session in `W` creates -> `"W"`; explicit `parent` overrides; explicit unknown or non-live parent -> `invalid_input` and no worktree or id created; unresolvable caller -> created unparented; app-created worktree has no parent; `list_worktrees` `parent` filter returns direct children only, `default` valid, unknown name -> empty list
- [x] T008 [US1] Extend the tool schemas and `create_worktree_request` in `crates/micold-core/src/mcp/tools.rs` (`parent` input on both tools; result and row fields `parent`, `parent_state`); T006 green
- [x] T009 [US1] In `crates/micold-daemon/src/mcp/tools.rs` resolve the caller's worktree via `resolve_caller` (R4), validate an explicit parent (R5) before any git work, and record the link and a fresh id (R3) in `create_literal_worktree` and `create_derived_worktree`; `list_worktrees` returns and filters by parent; T007 green
- [x] T010 [US1] Document `parent`, its default, the override and the filter in `docs/user-guide/agent-tools.md` (new section after "Creating a worktree from a type, ticket and name")

## Phase 3b: Visibility by parent (M1; FR-015, FR-016)

- [x] T030 [P] [US1] Add failing tests in `crates/micold-daemon/tests/mcp/mcp_worktree_parent_visibility.rs` (new module of the `mcp` binary, D1): from a worktree `A` with parent `P` and sibling `B`, child `C` of `A`, and an unrelated `X`: `list_worktrees`/`list_sessions` return exactly `A`, `P`, `B`, `C`; `get_session`, `read_session_output`, `send_session_input`, `interrupt_session`, `stop_session`, `create_session`, `rename_worktree`, `delete_worktree` on `X` or on `default` -> `not_found`; an unparented caller sees itself and its children; a message to an out-of-scope session is refused while the same message sent to the parent succeeds (relay, FR-017); a `default` caller sees all; explicit `parent` outside the set (including `default` from a non-root caller) -> `invalid_input`
- [x] T031 [US1] Implement one visibility filter in `crates/micold-daemon/src/mcp/tools.rs` (beside `resolve_caller`) computed from the parent links in the catalog (`crates/micold-daemon/src/catalog.rs`), applied by every tool of `contracts/mcp-parent.md`; T030 green
- [x] T032 [US1] Document the visibility rule and the relay through the parent (FR-017) in `docs/user-guide/agent-tools.md` (extend "What it can see" and "Reading and typing into other sessions")

## Phase 4: User Story 2 and User Story 3 - Survives renames and restarts; removed parent (Priority: P1/P2) (M1)

**Independent test**: `quickstart.md` Part A steps 4-5.

- [x] T011 [P] [US2] Add failing tests in `crates/micold-daemon/tests/mcp/mcp_worktree_parent.rs`: rename the parent -> children still resolve, not removed; daemon restart (reload store) -> parent unchanged; project folder relocated and reopened: no relocation fixture exists for per-project state (it is keyed by project path), so this is covered by the store round-trip tests and the daemon restart test (drift-M1.md)
- [x] T012 [P] [US3] Add failing tests in the same file: delete the parent through the tool and through the app op -> children `parent_state: "removed"`, `parent` keeps the last name, no error; recreate a worktree with the deleted name -> fresh id, no children adopted; delete a child -> its link and id entries dropped; parent removed outside the app -> removed at the next reconcile; deleting an unrelated worktree changes nothing for a `default` child
- [x] T013 [US3] In `crates/micold-daemon/src/ops.rs` (`delete_worktree`, `rename_worktree` untouched) tombstone children and drop the deleted worktree's own entries; keep `last_name` current on each reconcile; T011 and T012 green
- [x] T014 [US3] Document the removed state and the same-name residual (research R3) in `docs/user-guide/agent-tools.md`

## Phase 5: User Story 4 - Sidebar hierarchy (Priority: P2) (M2, blocked by M1)

**Independent test**: `quickstart.md` Part B.

- [x] T015 [P] [US4] Add failing client tests in `crates/micold-client/src/features/sidebar.rs` (test module): worktrees with a parent get a parent tag, unparented none; removed parent tag marked removed; `TagFilter::Parent` narrows to that parent's direct children and composes with `HasIssue`/`Type`; `available_tag_filters` lists only parents that have children; renaming the parent updates the label; state grouping and stable order unchanged (specs 1125, 1718)
- [x] T016 [US4] Surface `parent` and `parent_state` to the client snapshot through the existing worktree view model (`crates/micold-core/src/worktree.rs` `Worktree`, filled from the daemon catalog in `crates/micold-daemon/src/catalog.rs`)
- [x] T017 [US4] Add the parent tag in `worktree_tags` (`crates/micold-client/src/features/worktree.rs`) using the shared tag chip, `TagFilter::Parent` and its entry in `available_tag_filters` (`crates/micold-client/src/features/sidebar.rs`); T015 green; no feature-local widget (Constitution VIII)
- [x] T018 [US4] Run the `visual-pass` skill for `quickstart.md` Part B and record the result in `specs/1857-worktree-parent/quickstart.md`
- [x] T019 [US4] Document the label and filter in `docs/user-guide/worktrees-and-sessions.md`

## Phase 6: User Story 5 - Sessions report the parent (Priority: P3) (M1)

- [x] T020 [P] [US5] Add failing tests in `crates/micold-daemon/tests/mcp/mcp_worktree_parent.rs`: `list_sessions` and `get_session` rows carry the worktree's parent fields; none for `default` or an unparented worktree; `origin`/`origin_state` unchanged and independent (FR-011)
- [x] T021 [US5] Add the fields in `session_row` (`crates/micold-daemon/src/mcp/tools.rs:2161`) and `get_session`; T020 green
- [x] T022 [US5] Document the session fields and their independence from `origin` in `docs/user-guide/agent-tools.md` ("Sub-agent runs" neighbourhood)

## Phase 7: Supervisor integration (M3, blocked by M1; FR-013)

**Goal**: supervisor skills find their runs by parent and never touch another parent's runs.

- [x] T023 [P] Add failing fixture cases in `scripts/tests/` (new `supervisor-ownership.test.sh`): from a stubbed `list_worktrees` JSON with runs of two parents, the ownership helper selects only the supervisor's own runs and refuses a run with another parent or none, including a run whose parent is `removed`
- [x] T024 Add `scripts/autopilot/runs-of-parent.sh` (reads worktree rows as JSON on stdin and a parent ref; prints own runs); T023 green
- [x] T033 Write the relay rule into the supervisor skill's rules and the worker prompt it sends (`.claude/skills/autopilot-supervisor/` and the worker prompt it sends): a worker contacts only its parent and siblings; questions and cross-scope messages go to the parent, which forwards them up or down (FR-017); the supervisor never expects a worker to reach another supervisor's run; fixture case in `scripts/tests/supervisor-ownership.test.sh`
- [x] T025 Write the ownership rule (a supervisor in `default` is not limited by FR-015, so the rule still applies to it; one in its own worktree is limited by the daemon) in `.claude/skills/autopilot-supervisor/` (landed with #1813): create worktrees with the default parent, list runs by `parent`, record `parent` per run in the supervisor ledger, never act on another parent's run, treat a `removed` parent as an orphan to report, not to adopt
- [x] T026 Note the parent scope on #1813, #1814, #1816 and the epic #1817 (delivered as comments, not body edits; see Phase 8 and drift-M3.md)

## Phase 8: Polish and tracking

- [x] T027 Run `mise run gate`; resolve fmt, clippy and test failures (cloud: `scripts/gate.sh` fmt and clippy green; the test step needed `cargo-nextest`, installed, then the disk filled; the full test run is left to CI, the close diff touches only specs/)
- [x] T028 Run `speckit-analyze` for spec, plan and tasks consistency and fix findings
- [x] T029 Create the three milestone issues (existing: M1/M1b #1858, M2 #1859, M3 #1860 with Blocked by in their bodies; notes already on #1813, #1814, #1816, #1817, and #1814's body carries the recovery case) with Blocked by / Blocks relations, and update #1817 (checklist, dependencies), #1813 (sets the parent on every worktree it creates), #1814 and #1816 (find runs by parent, never touch another parent), and #1814 (recovery case for a supervised session over the context cap: `/clear` then `/speckit-autopilot resume`)

## Dependencies

- Phase 1 -> Phase 2 -> Phases 3, 4, 6 (M1; Phase 4 and 6 after Phase 3's create path exists).
- M1b (Phase 3b, T030-T032) after M1's create and list paths (T007-T009); T033 after T031.
- M2 (Phase 5) after M1's row shape exists (T008, T009, T013).
- M3 (Phase 7) after M1 (T009 for the row shape); #1813 starts after M1 (needs the default parent on creation), #1814 and #1816 after M3.
- Parallel: T002 with T004; T006 with T007; T011 with T012; T015 with M1 docs; T023 with M2.

## Implementation strategy

MVP: Phases 1-3 (record, list, filter). Then Phase 4 (durability, removed), Phase 6 (sessions), M2 and M3 in parallel once M1 merges.

## Phase 9: TDD remediation

**Verdict FAIL** (`tdd/verification.md`, audited at `c53be1dc9`): T034 is done (test-after debt accepted and recorded in `tdd/cycle-log.md`); T035-T041 are open follow-ups. Findings 2-9 below close coverage gaps; none may weaken an existing test.

- [x] T034 [F1, F6] Write `specs/1857-worktree-parent/tdd/test-list.md` (behaviors B1-B13 of `tdd/verification.md`, each with its tests and the criterion it traces), and record in `tdd/cycle-log.md` that M1, M1b, M2 and M3 were test-after and which behaviors were re-proven by mutant; re-run `/speckit-tdd-verify specs/1857-worktree-parent` and expect no `NO_TEST` and a stated, accepted `TEST_AFTER` debt
- [ ] T035 [F2] Make `a_message_to_an_out_of_scope_session_is_refused_while_the_parent_is_reachable` (`crates/micold-daemon/tests/mcp/mcp_worktree_parent_visibility.rs:188`) assert delivery to the parent (a sandbox-backed fixture as in `mcp_cross_session`), not only "not `not_found`"; prove with `scripts/build-lock.sh cargo test -p micold-daemon --test mcp a_message_to_an_out_of_scope_session`
- [ ] T036 [F3] Add a daemon test that a root caller passing `parent: "default"` to `create_worktree` succeeds with `("default","known")`, beside `mcp_worktree_parent.rs:223`; prove with `scripts/build-lock.sh cargo test -p micold-daemon --test mcp worktree_parent`
- [ ] T037 [F4] Add a test that deleting a parent through the MCP `delete_worktree` tool leaves its children `parent_state: "removed"`; prove with the same command as T036
- [ ] T038 [F5] Add tests for FR-005 child rename, FR-003 (a worktree created through the app op has no parent) and FR-012 (a parent is not readable or writable across projects), and either a relocation test for US2 AC3 / SC-002 or amend the spec to drop the relocation promise; prove with `scripts/build-lock.sh cargo test -p micold-daemon --test mcp worktree_parent`
- [ ] T039 [F7] In `crates/micold-core/src/worktree_parent.rs` add unit tests that fail for mutants M1, M3, M4 and M8 of `tdd/verification.md` (self-parent guard, orphan-link sweep, `ParentView::label` for `default`, parent id reuse in `record`), and remove or test the dead `last_name` refresh (`:83-85`); prove with `scripts/build-lock.sh cargo test -p micold-core --lib worktree_parent` and re-run the eight mutants
- [ ] T040 [F8] Replace `renaming_the_parent_updates_the_label_and_keeps_order` (`crates/micold-client/tests/sidebar_parent.rs:147`) with one that drives the rename through the catalog snapshot to the row chip; prove with `scripts/build-lock.sh cargo test -p micold-client --test main sidebar_parent`
- [ ] T041 [F9] Replace the word-presence checks in `scripts/tests/supervisor-ownership.test.sh:72-81` with checks of behavior, or record in the test why wording is the contract; prove with `bash scripts/tests/supervisor-ownership.test.sh`
