---

description: "Task list for MCP parent/sibling view, neighbourhood tool and parent messaging"
---

# Tasks: A session sees its parent, siblings and children, and messages its parent

**Input**: `specs/1945-mcp-parent-sibling-view/` (spec.md, plan.md, research.md, data-model.md, contracts/, quickstart.md)
**Prerequisites**: plan.md, spec.md
**Tests**: mandatory and first (Constitution I): each implementation task is preceded by a failing test task.
**Documentation**: each story that changes behaviour carries its user-guide task (Constitution VII).
**Scope**: daemon and MCP only; no GUI, so no visual pass.

## Format: `- [ ] [TaskID] [P?] [Story?] Description with file path`

## Phase 1: Foundational + US1 (P1) 🎯 MVP: an unparented session sees where it stands

**Goal**: one derivation of parent/siblings/children, used by `Scope`, so every tool shows an unparented session `default` and its siblings.

- [ ] T001 [US1] Core unit tests (failing) for `neighbourhood()`: implicit parent, mutual siblings (recorded vs implicit `default` parent), removed parent, `default` caller with implicit children, unknown worktree, in `crates/micold-core/src/worktree_parent.rs`
- [ ] T002 [US1] Add `Neighbourhood`, `ParentSide`, `Child` and `neighbourhood(live, views, own)` to `crates/micold-core/src/worktree_parent.rs` (data-model.md)
- [ ] T003 [US1] Daemon integration tests (failing) in `crates/micold-daemon/tests/mcp/mcp_worktree_parent_visibility.rs`: unparented caller lists sessions and worktrees (US1.1-1.5, SC-001), a caller whose worktree was removed sees only itself (SC-005); amend the 1857 expectations that "no parent has no siblings" in place (also `mcp_worktree_parent.rs`)
- [ ] T004 [US1] Rebuild `Scope::new` on `neighbourhood()` in `crates/micold-daemon/src/mcp/tools.rs` (D2): read `project.worktrees`, unknown own dir gives `visible = {own}`, `default` caller stays unrestricted
- [ ] T005 [US1] User guide: rewrite "Which worktrees a session can see" in `docs/user-guide/agent-tools.md` for the implicit `default` parent

**Checkpoint**: `list_sessions`/`list_worktrees` from an unparented caller show `default` and siblings.

---

## Phase 2: US2 + US3 (P1): neighbourhood tool and parent messaging

**Goal**: one call for the whole neighbourhood; send text to the parent without a session id.

- [ ] T006 [US2] [US3] Catalog tests (failing) in `crates/micold-core/tests/mcp_tools_catalog.rs`: both tools listed, parse, extra arguments rejected
- [ ] T007 [US2] [US3] Add `Operation::{Neighbourhood, SendParentInput}`, TOOLS entries and parsing in `crates/micold-core/src/mcp/tools.rs`; the variants in `crates/micold-core/src/mcp/policy.rs` (Neighbourhood allowed, SendParentInput gated as `SendSessionInput`)
- [ ] T008 [US2] Daemon integration tests (failing) in new `crates/micold-daemon/tests/mcp/mcp_worktree_parent_neighbourhood.rs` (register `mod mcp_worktree_parent_neighbourhood;` in `crates/micold-daemon/tests/mcp/main.rs`): `neighbourhood` for recorded, unparented, removed and `default` callers, caller with several sessions, an unknown-worktree caller (empty lists and the note "this session's worktree is not known to the daemon"), nothing outside scope (US2, FR-003..005, SC-002, SC-005)
- [ ] T009 [US2] Implement `Context::neighbourhood()` and its `read_call` dispatch in `crates/micold-daemon/src/mcp/tools.rs` per `contracts/mcp-neighbourhood.md` (rows via `session_row`, `recorded`, `note`)
- [ ] T010 [US3] Unit test (failing) for `parent_target` (several sessions, catalog order, non-`Running` skipped, none) beside the function in `crates/micold-daemon/src/mcp/tools.rs`
- [ ] T011 [US3] Refusal tests (failing) in `crates/micold-daemon/tests/mcp/mcp_worktree_parent_neighbourhood.rs`: `default` caller, removed parent, no running session, unknown worktree; nothing typed
- [ ] T012 [US3] Delivery tests (failing) in `crates/micold-daemon/tests/mcp_cross_session.rs` (`#![cfg(unix)]`, stub `claude`): delivery to one and to several parent sessions (same session twice), implicit `default` parent, policy `Off` and `ConfirmEachSend`, scope (FR-006..008, SC-003)
- [ ] T013 [US3] Add `parent_target` and `send_parent_input` (refusals per the contract table, then delegate to `send_session_input`, add `session` and `parent_worktree`) in `crates/micold-daemon/src/mcp/tools.rs` (D4)
- [ ] T014 [US2] [US3] User guide: tool table rows for both tools and the several-parent-sessions rule in `docs/user-guide/agent-tools.md`

**Checkpoint**: `quickstart.md` steps 1-3 pass.

---

## Phase 3: US4 (P2) + US5 (P3): record before start, agents told

**Goal**: the parent link exists before the child's session starts; the supervisor skill points workers at the parent route.

- [ ] T015 [US4] Test (failing) in `crates/micold-daemon/tests/mcp_cross_session.rs`: the new session's stub `claude` snapshots the parent link at start (catalog/app-state file or daemon query) and writes it into its marker; assert the marker holds the link (SC-004, US4.1)
- [ ] T016 [US4] Test in `crates/micold-daemon/tests/mcp/mcp_worktree_parent.rs`: a create that fails before the worktree exists (missing CLI) records no link (US4.2); a session create that fails after the worktree exists keeps the link (FR-009); an app-path create records none
- [ ] T017 [US4] Add `parent: Option<&ParentChoice>` to `StartDefinition` in `crates/micold-daemon/src/ops.rs`, record only `if let Some(parent)` between worktree create and session create; remove the post-call record for the session branch in `create_with_optional_session`; `crates/micold-daemon/src/server.rs` passes `parent: None` (D5)
- [ ] T018 [US4] User guide: say the link is recorded before the session starts in `docs/user-guide/agent-tools.md`
- [ ] T019 [US5] Tell workers to ask their parent with `send_parent_input` in `.claude/skills/autopilot-supervisor/SKILL.md`; add a grep check for the three items (neighbourhood tool, parent route, implicit parent) across guide and skill
- [ ] T020 [US5] Add a note in `specs/1857-worktree-parent/spec.md` that 1945 amends FR-003 and FR-015

**Checkpoint**: `quickstart.md` step 2 passes; guide and skill carry the three items.

---

## Dependencies & Execution Order

Phase 1 before 2 (the tools use the derivation); Phase 3 independent of 2 except the stub-session harness shared in `mcp_cross_session.rs`. Within a phase, test tasks precede their implementation.

## Implementation Strategy

MVP is Phase 1 (the reported failure). Then the tools, then ordering and docs.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Unparented sessions see `default` and siblings 🎯 MVP

- **Tasks**: T001–T005
- **Deliverable**: a session in a hand-made worktree lists `default`'s and its siblings' sessions and worktrees, and a removed-worktree session sees only itself.
- **Satisfies**: US1 acceptance scenarios 1–5; FR-001, FR-002; SC-001, SC-005
- **Verify**: `scripts/build-lock.sh cargo test -p micold-core worktree_parent` and `scripts/build-lock.sh cargo test -p micold-daemon --test mcp worktree_parent`
- **Depends on**: —
- **Tier**: full

### M2 — `neighbourhood` and `send_parent_input` tools

- **Tasks**: T006–T014
- **Deliverable**: a session calls `neighbourhood` for parent, siblings and children in one call, and `send_parent_input` types text into its parent's running session.
- **Satisfies**: US2 acceptance scenarios 1–5; US3 acceptance scenarios; FR-003–FR-008; SC-002, SC-003, SC-005
- **Verify**: `scripts/build-lock.sh cargo test -p micold-daemon --test mcp worktree_parent` and `scripts/build-lock.sh cargo test -p micold-daemon --test mcp_cross_session`; quickstart step 3
- **Depends on**: M1
- **Tier**: full

### M3 — Parent recorded before the child starts, agents told

- **Tasks**: T015–T020
- **Deliverable**: a created worktree's parent link exists before its session starts, and the guide and supervisor skill describe the new tools.
- **Satisfies**: US4 acceptance scenarios 1–2; US5 acceptance scenarios 1–2; FR-009, FR-010; SC-004
- **Verify**: `scripts/build-lock.sh cargo test -p micold-daemon --test mcp worktree_parent`; `scripts/build-lock.sh cargo test -p micold-daemon --test mcp_cross_session`; `grep -n "send_parent_input" docs/user-guide/agent-tools.md .claude/skills/autopilot-supervisor/SKILL.md`
- **Depends on**: M2
- **Tier**: full
