---

description: "Task list for worktree parent registration (MCP + sidebar) and supervisor support"
---

# Tasks: Worktree parent registration (MCP + sidebar) and supervisor support

**Input**: `specs/1857-worktree-parent/` (spec.md, plan.md, research.md, data-model.md, contracts/, quickstart.md)
**Prerequisites**: plan.md, spec.md
**Tests**: mandatory and first (Constitution I): each implementation task is preceded by a failing test task.
**Documentation**: each user-facing story carries its user-guide task (Constitution VII).
**Milestones** (one GitHub issue each): **M1** MCP and persistence (Phases 1-4, 6; US1, US2, US3, US5), **M2** sidebar UI (Phase 5; US4; blocked by M1), **M3** supervisor integration (Phase 7; blocked by M1; consumed by #1813, #1814, #1816).

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

## Phase 4: User Story 2 and User Story 3 - Survives renames and restarts; removed parent (Priority: P1/P2) (M1)

**Independent test**: `quickstart.md` Part A steps 4-5.

- [x] T011 [P] [US2] Add failing tests in `crates/micold-daemon/tests/mcp/mcp_worktree_parent.rs`: rename the parent -> children still resolve, not removed; daemon restart (reload store) -> parent unchanged; project folder relocated and reopened: no relocation fixture exists for per-project state (it is keyed by project path), so this is covered by the store round-trip tests and the daemon restart test (drift-M1.md)
- [x] T012 [P] [US3] Add failing tests in the same file: delete the parent through the tool and through the app op -> children `parent_state: "removed"`, `parent` keeps the last name, no error; recreate a worktree with the deleted name -> fresh id, no children adopted; delete a child -> its link and id entries dropped; parent removed outside the app -> removed at the next reconcile; deleting an unrelated worktree changes nothing for a `default` child
- [x] T013 [US3] In `crates/micold-daemon/src/ops.rs` (`delete_worktree`, `rename_worktree` untouched) tombstone children and drop the deleted worktree's own entries; keep `last_name` current on each reconcile; T011 and T012 green
- [x] T014 [US3] Document the removed state and the same-name residual (research R3) in `docs/user-guide/agent-tools.md`

## Phase 5: User Story 4 - Sidebar hierarchy (Priority: P2) (M2, blocked by M1)

**Independent test**: `quickstart.md` Part B.

- [ ] T015 [P] [US4] Add failing client tests in `crates/micold-client/src/features/sidebar.rs` (test module): worktrees with a parent get a parent tag, unparented none; removed parent tag marked removed; `TagFilter::Parent` narrows to that parent's direct children and composes with `HasIssue`/`Type`; `available_tag_filters` lists only parents that have children; renaming the parent updates the label; state grouping and stable order unchanged (specs 1125, 1718)
- [ ] T016 [US4] Surface `parent` and `parent_state` to the client snapshot through the existing worktree view model (`crates/micold-core/src/worktree.rs` `Worktree`, filled from the daemon catalog in `crates/micold-daemon/src/catalog.rs`)
- [ ] T017 [US4] Add the parent tag in `worktree_tags` (`crates/micold-client/src/features/worktree.rs`) using the shared tag chip, `TagFilter::Parent` and its entry in `available_tag_filters` (`crates/micold-client/src/features/sidebar.rs`); T015 green; no feature-local widget (Constitution VIII)
- [ ] T018 [US4] Run the `visual-pass` skill for `quickstart.md` Part B and record the result in `specs/1857-worktree-parent/quickstart.md`
- [ ] T019 [US4] Document the label and filter in `docs/user-guide/worktrees-and-sessions.md`

## Phase 6: User Story 5 - Sessions report the parent (Priority: P3) (M1)

- [x] T020 [P] [US5] Add failing tests in `crates/micold-daemon/tests/mcp/mcp_worktree_parent.rs`: `list_sessions` and `get_session` rows carry the worktree's parent fields; none for `default` or an unparented worktree; `origin`/`origin_state` unchanged and independent (FR-011)
- [x] T021 [US5] Add the fields in `session_row` (`crates/micold-daemon/src/mcp/tools.rs:2161`) and `get_session`; T020 green
- [x] T022 [US5] Document the session fields and their independence from `origin` in `docs/user-guide/agent-tools.md` ("Sub-agent runs" neighbourhood)

## Phase 7: Supervisor integration (M3, blocked by M1; FR-013)

**Goal**: supervisor skills find their runs by parent and never touch another parent's runs.

- [ ] T023 [P] Add failing fixture cases in `scripts/tests/` (new `supervisor-ownership.test.sh`): from a stubbed `list_worktrees` JSON with runs of two parents, the ownership helper selects only the supervisor's own runs and refuses a run with another parent or none, including a run whose parent is `removed`
- [ ] T024 Add `scripts/autopilot/runs-of-parent.sh` (reads worktree rows as JSON on stdin and a parent ref; prints own runs); T023 green
- [ ] T025 Write the ownership rule in `.claude/skills/speckit-autopilot/rules/ownership.md` (or the supervisor skill's rules once #1813 lands): create worktrees with the default parent, list runs by `parent`, record `parent` per run in the supervisor ledger, never act on another parent's run, treat a `removed` parent as an orphan to report, not to adopt
- [ ] T026 Update #1813, #1814 and #1816 issue bodies and the epic #1817 (see Phase 8)

## Phase 8: Polish and tracking

- [ ] T027 Run `mise run gate`; resolve fmt, clippy and test failures
- [ ] T028 Run `speckit-analyze` for spec, plan and tasks consistency and fix findings
- [ ] T029 Create the three milestone issues with Blocked by / Blocks relations, and update #1817 (checklist, dependencies), #1813 (sets the parent on every worktree it creates), #1814 and #1816 (find runs by parent, never touch another parent), and #1814 (recovery case for a supervised session over the context cap: `/clear` then `/speckit-autopilot resume`)

## Dependencies

- Phase 1 -> Phase 2 -> Phases 3, 4, 6 (M1; Phase 4 and 6 after Phase 3's create path exists).
- M2 (Phase 5) after M1's T016 inputs exist (T008, T009, T013).
- M3 (Phase 7) after M1 (T009 for the row shape); #1813 starts after M1 (needs the default parent on creation), #1814 and #1816 after M3.
- Parallel: T002 with T004; T006 with T007; T011 with T012; T015 with M1 docs; T023 with M2.

## Implementation strategy

MVP: Phases 1-3 (record, list, filter). Then Phase 4 (durability, removed), Phase 6 (sessions), M2 and M3 in parallel once M1 merges.
