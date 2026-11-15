# Implementation Plan: A session sees its parent, siblings and children, and messages its parent

**Branch**: `claude/project-thread-x5s2f0` | **Date**: 2026-11-15 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/1945-mcp-parent-sibling-view/spec.md`

## Summary

Four changes, all in the daemon and its MCP tool layer (the iced app is untouched):

1. **One relation, one place.** A pure function in `micold-core::worktree_parent` derives a
   worktree's *neighbourhood* (effective parent, siblings, children) from the live worktree names and
   the recorded parent views, treating a worktree with no recorded parent as a child of `default`
   (FR-001). `Scope::new` (visibility, FR-002/005/008) and the new tool (FR-003/004) both call it, so
   the two cannot drift.
2. **Two new tools**: `neighbourhood` (read-only, FR-003/004/005) and `send_parent_input`
   (FR-006/007/008). The second resolves the parent target and delegates to the existing
   `send_session_input` path, so policy, confirmation, trust and "session gone" behave unchanged.
3. **Record the parent before the session starts** (FR-009): the link is recorded inside
   `ops::create_worktree_with_session`, between the worktree create and the session create.
4. **Docs and skill** (FR-010): `docs/user-guide/agent-tools.md` and the autopilot-supervisor skill.

## Technical Context

**Language/Version**: Rust (workspace edition), no new crates.

**Primary Dependencies**: existing `serde_json`, `micold-core` (`worktree_parent`, `mcp::tools`,
`mcp::policy`), `micold-daemon` (`mcp::tools`, `ops`, `state`).

**Storage**: none new. No link is written for the implicit parent (FR-001); the existing
`worktree_parents`/`worktree_ids` catalog maps are read only.

**Testing**: core unit tests for the derivation (`cargo test -p micold-core`), daemon integration
tests under `crates/micold-daemon/tests/mcp/` (real MCP endpoint over a `state_over` fixture), core
catalog test `micold-core/tests/mcp_tools_catalog.rs`. No GUI, so no geometry gates or quickstart
§B visual pass.

**Target Platform**: the daemon, identical on Linux, macOS, Windows (no platform code).

**Project Type**: Rust workspace; daemon + MCP server.

**Performance Goals**: none beyond today; the tool is one catalog snapshot read like `list_sessions`.

**Constraints**: local only; a call must never reveal anything outside the amended scope (SC-005).

**Scale/Scope**: tens of worktrees per project; derivation is O(worktrees) per call.

## Constitution Check

*Re-checked after Phase 1: unchanged.*

- **I. Test-First**: PASS. Tasks order a failing test before each production change; the test list
  is in [Test strategy](#test-strategy).
- **II. Multi-Session**: PASS. Reads are per-caller; delivery reuses the per-session input path and
  its atomic submission; no shared mutable state added.
- **III. Worktree Integration**: PASS. Uses the existing worktree parent model; `default` is the
  sanctioned root location; no manual git steps.
- **IV. Local-First Storage**: PASS. No storage, no network.
- **V. Rust + iced Stack**: PASS (Rust; no iced code touched). The neighbourhood is a typed struct
  (`ParentSide`, `Neighbourhood`), so "implicit and removed" is not representable.
- **VI. Cross-Platform Parity**: PASS. No platform-specific code; paths in the tool result go
  through the existing `native()` helper only where rows already do.
- **VII. Documentation**: PASS. FR-010; `docs/user-guide/agent-tools.md` changes in the same PR.
- **VIII. Reusable UI Components**: N/A (no UI).

## Project Structure

### Documentation (this feature)

```text
specs/1945-mcp-parent-sibling-view/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/mcp-neighbourhood.md
└── tasks.md            # /speckit-tasks, not this step
```

### Source Code

```text
crates/micold-core/src/worktree_parent.rs    # + Neighbourhood, ParentSide, neighbourhood()
crates/micold-core/src/mcp/tools.rs          # + Operation::{Neighbourhood, SendParentInput}, TOOLS entries, parse
crates/micold-core/src/mcp/policy.rs         # + the two variants (Neighbourhood: allowed; SendParentInput: delegated)
crates/micold-core/tests/mcp_tools_catalog.rs# catalog lists and parse tests
crates/micold-daemon/src/mcp/tools.rs        # Scope::new via neighbourhood(); Context::neighbourhood();
                                             # send_parent_input(); create_with_optional_session
crates/micold-daemon/src/ops.rs              # StartDefinition.parent; record before session create
crates/micold-daemon/src/server.rs           # the app's StartDefinition passes parent: None
crates/micold-daemon/tests/mcp/              # mcp_worktree_parent_neighbourhood.rs (new); amend
                                             # mcp_worktree_parent_visibility.rs, mcp_worktree_parent.rs
docs/user-guide/agent-tools.md
.claude/skills/autopilot-supervisor/SKILL.md
specs/1857-worktree-parent/spec.md           # note pointing to 1945 (FR-003, FR-015 amended)
```

**Structure Decision**: derive in core (pure, unit-testable, no daemon state), adapt in the daemon.
This follows the 1857 split where `worktree_parent.rs` holds "the pure rules".

## Design

### D1. The derivation (FR-001, FR-002, FR-004, FR-005)

`worktree_parent::neighbourhood(live: &[String], views: &BTreeMap<String, ParentView>, own:
Option<&str>) -> Neighbourhood`. `own = None` is `default`. Each live worktree `w` has an *effective
parent* `eff(w)`:

- `views[w]` present, state `Known`: that name (`default` or a worktree);
- `views[w]` present, state `Removed`: none (links no one, as today);
- no entry: `default` (implicit).

For `own = Some(o)`: `parent` = `eff(o)` with its provenance (`Recorded` | `Implicit`) and, for a
removed link, `ParentSide::Removed(last name)`; `siblings` = live `w != o` with `eff(w) == eff(o)`
when `eff(o)` is a parent (none when removed); `children` = live `w` with `eff(w) == o`. For
`own = None` (`default`): no parent, children = all `w` with `eff(w) == default`, which includes the
unparented worktrees (implicit), each flagged `Implicit`. A live `own` not in `live` (unknown
worktree) yields an empty neighbourhood. Mutual siblings fall out of the single relation (spec
scenarios 1.3/1.4). A recorded parent that no longer lives is already `Removed` (1857 FR-007).

### D2. Scope (FR-001, FR-002, FR-005, FR-008)

`Scope::new` keeps its signature and the caller-in-`default` and unknown-caller-worktree cases
(`unrestricted`). Otherwise `visible = {own} ∪ parent(Known/implicit) ∪ siblings ∪ children`, taken
from the derivation; `hidden` is unchanged (worktrees not in the set). `Scope::new` needs the live
names, so it reads `project.worktrees`. Every tool already goes through `Scope`, so FR-001 reaches
all of them with no per-tool change.

### D3. `neighbourhood` tool (FR-003, FR-004, FR-005)

Read-only; dispatched in `read_call` like `whoami` through `Context::neighbourhood()`, which builds
rows with the existing `session_row` (so rows match `list_sessions`, including `is_caller`). Shape
in [contracts/mcp-neighbourhood.md](contracts/mcp-neighbourhood.md). The answer carries `parent.recorded`
(true/false) and a `note` string for the implicit and `default` cases (FR-004). Rows are filtered
through `Scope` as a defence in depth (SC-005); for a derived set they are already inside it.

### D4. `send_parent_input` (FR-006, FR-007, FR-008)

Input `{text}` (same `NonEmptyText` rule as `send_session_input`). Dispatched in `call` beside
`SendSessionInput`. Steps: resolve caller and project; `neighbourhood()`; refuse per the contract's table (`invalid_input` in `default`, `not_found` for
`removed`, `conflict` for no session), naming the worktree; else pick the target, then call the existing `send_session_input(state,
caller, SessionRef(target), text, hangup)` and add `session` and `parent_worktree` to its reply.
Because it re-enters the by-id path, the cross-session policy (`Off`, `ConfirmEachSend`), the trust
question, atomic submission and "session gone" apply unchanged (FR-008, Assumptions). Target rule:
among sessions of the parent worktree (for the implicit or `default` parent: sessions with no
worktree dir) whose lifecycle is `Running`, the first in the project's session list, which is
creation order (catalog append order; research R3); ties (none expected) by lowest id. Policy
mapping: `Operation::SendParentInput` is checked as `SendSessionInput` of the resolved target, so
`policy.rs` gets the variant only for exhaustiveness plus the `Off` gate.

### D5. Record before start (FR-009)

`StartDefinition` gains `parent: Option<&ParentChoice>`. `create_worktree_with_session` calls
`state.record_worktree_parent(&project, &dir_name, definition.parent)` right after
`create_worktree` returns `Ok` and before `create_session_with_prompt`. The post-call recording in
`create_with_optional_session` for the session branch is removed (kept for the no-session branch).
Failure semantics are unchanged: `CliUnavailable` and `Create` return before the record (no link);
`Session` failure keeps it. `server.rs` (the app path) passes `parent: None`.

### D6. Docs (FR-010)

`agent-tools.md`: tool table rows for both tools, rewrite "Which worktrees a session can see" (the
"worktree with no parent has no parent or siblings" sentence is replaced by the implicit parent),
the parent-choice rule for several sessions. SKILL.md: the worker asks its parent with
`send_parent_input`. Close-out: a note in `specs/1857-worktree-parent/spec.md` (ledger follow-up).

## Requirement map

| FR | Where |
|---|---|
| FR-001, FR-002 | D1, D2 |
| FR-003, FR-004, FR-005 | D1, D3, contract |
| FR-006, FR-007, FR-008 | D4 |
| FR-009 | D5 |
| FR-010 | D6 |

## Test strategy

Layers: **core unit** = `micold-core` unit tests; **daemon integration** = `micold-daemon/tests/mcp`.

| Behaviour | Layer |
|---|---|
| Derivation: implicit parent, mutual siblings, removed, default caller, unknown worktree (US1, US2.2-2.4, edge cases) | core unit in `worktree_parent.rs` |
| Catalog lists and parse for the two tools; extra args rejected | core `mcp_tools_catalog.rs` |
| `list_sessions` / `list_worktrees` from an unparented caller (US1.1-1.5, SC-001, SC-005) | daemon integration |
| `neighbourhood` for recorded, unparented, removed, default callers; caller's own several sessions (US2) | daemon integration |
| `send_parent_input`: one, several (stable twice), none, implicit, default, removed (US3) | daemon integration |
| Policy: `Off` and `ConfirmEachSend` apply; cannot reach outside scope (FR-008) | daemon integration |
| Parent link present at session start, incl. delayed start; none when create fails (US4, SC-004) | daemon integration, using the existing create fixture with a start hook delay |
| Existing 1857 visibility tests whose "no parent has no siblings" expectation changes | amend in place |
| Docs and skill text (US5) | docs check in CI plus a grep assertion in the tasks |

## Complexity Tracking

No violations.
