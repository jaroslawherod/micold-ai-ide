# Implementation Plan: One Session per Worktree, Chosen at Start

**Branch**: `feat/776_one-session-per-worktree-chosen-at-start` | **Date**: 2026-10-12 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/776-one-session-per-worktree/spec.md`

## Summary

A worktree (or the Default root) hosts at most one un-archived session. The invariant is enforced
where the record is written, in one place, under the daemon's catalog lock, and every other
surface follows from it:

1. **One rule, in `micold-core`**: a pure `occupancy` module decides, from a location's sessions,
   whether it is `Free` or `Taken(existing)`, and whether starting a given session is `Blocked` by
   another live one (legacy catalogs, FR-013). Daemon and client both call it; neither re-derives.
2. **Refuse at the write**: `Catalog::create_session` checks and pushes under one `&mut self`, so
   FR-005 holds for any mix of windows and agents. Its refusal is a typed error that carries the
   existing session; every caller maps it (`SessionCreate` RPC, MCP `create_session`, review send,
   runs).
3. **Define at start**: `WorktreeCreate` and MCP `create_worktree` take an optional session
   definition (AI CLI, first prompt); the daemon creates the worktree and its session in one
   operation, checking the CLI first so a missing CLI leaves nothing.
4. **No add affordance**: the sidebar offers the start action only on a location with no session;
   the create-worktree dialog gains the CLI and prompt fields.
5. **Resume replaces**: provider-session resume (`AttachApply` of a session) into an occupied
   location is refused until the person confirms; with confirmation the daemon swaps the records
   in one catalog write after stopping the old process. Project-open adoption and "attach all"
   catalogue at most one session per location.
6. **Legacy**: nothing is migrated or deleted; extra sessions stay; a start is refused while
   another of the location's sessions is live.

No new storage, no new crate, no new lifecycle state.

## Technical Context

**Language/Version**: Rust, stable toolchain via `mise`

**Primary Dependencies**: existing only: `iced` (client), `serde`/`serde_json`, `uuid`, `tokio`
(daemon). No new crate.

**Storage**: none new. The catalog (`micold-daemon/src/catalog.rs`, project store via
`micold-core/src/store.rs`) already persists sessions with `location` and `archived`; the rule is
derived from them. No schema change, so an old catalog loads unchanged (SC-005).

**Testing**: `cargo test` via `mise run test-core` (rule, wire, tool catalogue) and `mise run
gate`; daemon integration tests in `crates/micold-daemon/tests/` (real daemon over its socket, as
`mcp_create_session.rs`); client reducer and layout tests in `crates/micold-client/tests/`;
quickstart §B visual pass for the dialog and sidebar.

**Target Platform**: Linux, macOS, Windows (Principle VI). The rule is path-free and
process-free; nothing platform-specific is added.

**Project Type**: desktop app: `micold-core` (render-free), `micold-daemon`, `micold-client`.

**Performance Goals**: none new. The check is a scan of one project's session list under a lock
already held for the write.

**Constraints**: offline; check-and-create atomic under the catalog lock (FR-005); wire change is
additive (optional fields) with a `PROTOCOL_VERSION` bump (40 to 41).

**Scale/Scope**: tens of sessions per project; one scan per create/start.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-checked after Phase 1 design: unchanged.*

- [x] **I. Test-First (NON-NEGOTIABLE)**: PASS. Every decision (rule, refusal mapping, atomic
  create, definition at start, replace, legacy refusal, adoption cap) lands first in core or
  daemon logic with a failing test; the tasks unit orders tests before code. Client and sidebar
  glue is decision-free (it asks `occupancy`), so the quickstart-§B exception covers only
  rendering.
- [x] **II. Multi-Session Support**: PASS. Sessions stay independently addressable and persisted;
  the rule is per worktree and changes nothing across worktrees (spec out of scope). Replace
  stops the old session's own processes only.
- [x] **III. Worktree Integration**: PASS. The app owns create-with-session; the Default root
  follows the same rule (FR-011). The Default-session exception is untouched: a Default session
  calling `create_worktree` with a definition still only creates, and the new session runs in the
  new worktree (research R6).
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: PASS. Catalog on the local filesystem only.
- [x] **V. Rust + iced Stack**: PASS. The refusal is a typed result (`Occupancy`, a typed
  `CreateSessionError`), not a string check; the start-with-definition payload is one type
  (`SessionDefinition`), so "a worktree created with half a definition" is unrepresentable.
- [x] **VI. Cross-Platform Parity**: PASS. No path or process behaviour in the rule; CI runs all
  three platforms for these paths.
- [x] **VII. Documentation First-Class**: PASS. User guide pages `worktrees-and-sessions.md` and
  `agent-tools.md` change in the same PR (FR-014).
- [x] **VIII. Reusable UI Component Foundation**: PASS. The dialog reuses the existing select,
  text-field and form primitives; no new widget. Any new composition goes through the existing
  builders.

## Project Structure

### Documentation (this feature)

```text
specs/776-one-session-per-worktree/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── wire.md           # ClientMsg/DaemonMsg/ErrorKind changes, PROTOCOL_VERSION
│   └── mcp-tools.md      # create_session, start_session, create_worktree
└── tasks.md              # next unit
```

### Source Code (repository root)

```text
crates/micold-core/src/
├── occupancy.rs          # NEW: Occupancy, StartBlock, refusal wording (pure)
├── lib.rs                # export occupancy
├── attach.rs             # RefuseReason::LocationHasSession; per-location cap helper
├── session.rs            # (read) Session, SessionLocation, SessionLifecycle::is_live
├── mcp/tools.rs          # create_worktree + create_session/start_session descriptions and
│                         #   optional ai_cli/prompt on create_worktree
└── protocol/
    ├── messages.rs       # WorktreeCreate.session, WorktreeCreated.session,
    │                     #   AttachApply.replace
    └── version.rs        # PROTOCOL_VERSION 41

crates/micold-daemon/src/
├── catalog.rs            # create_session -> CreateSessionError; attach_session/adopt cap;
│                         #   replace_session (swap under one write)
├── state.rs              # create_session passthrough; start_refusal; in-lock check+claim in start_session_gated; review_send_to_new_session
│                         #   reuses the entry's own session; attach_apply replace + cap
├── ops.rs                # create_worktree_with_session (CLI check, create, session)
├── server.rs             # SessionCreate, WorktreeCreate, SessionStart/Restart mapping
├── runs.rs               # unchanged call, new typed error handled
└── mcp/tools.rs          # create_session refusal, start_session blocked, create_worktree
                          #   definition

crates/micold-client/src/
├── features/session.rs   # start affordance offered only for a Free location
├── features/worktree_form.rs   # AI CLI + prompt fields, "None" choice
├── features/attach.rs    # confirm-replace step
├── ui/sidebar.rs         # hide start action when occupied (renders the reducer's answer)
├── ui/worktree_form.rs   # render the two fields
├── ui/attach_dialog.rs   # render the confirmation
└── shell/daemon_sync.rs  # send definition, show the refusal text, select and view the
                          #   session named by WorktreeCreated.session (as SessionCreated does)

docs/user-guide/worktrees-and-sessions.md, agent-tools.md      # FR-014
crates/micold-core/tests/, crates/micold-daemon/tests/, crates/micold-client/tests/
```

**Structure Decision**: the existing three-crate layout. The rule lives in `micold-core` so the
daemon enforces it and the client's affordance reads the same function; no logic in `main.rs`
or `ui/`.

## Requirement map

| Requirement | Where |
|---|---|
| FR-001, FR-005 | `occupancy::Occupancy` + atomic check in `Catalog::create_session` (R1, R2) |
| FR-002, FR-003 | `CreateSessionError::Occupied` mapped by `SessionCreate`, MCP `create_session`, review send (R3); wording in `occupancy::refusal_text` (contracts) |
| FR-004 | `SessionState::start_intent` returns no start for an occupied location; sidebar renders it (R5) |
| FR-006 | `WorktreeCreate.session`, MCP `create_worktree` `ai_cli`/`prompt`, dialog fields (R4, R6); `ops::create_worktree_with_session` |
| FR-007, FR-008b | `AttachApply.replace`, `Catalog::replace_session`, confirm step (R7) |
| FR-008a, FR-009 | existing `SessionRestart`/`SessionStart` paths, unchanged; pinned by tests |
| FR-010 | `Occupancy::Free` includes a worktree whose sessions are all archived |
| FR-011 | `SessionLocation::Default` is just another key (R1) |
| FR-012 | `attach::cap_per_location`, `adopt_discovered_sessions` cap (R8) |
| FR-013 | `occupancy::start_block`; in-lock check in `start_session_gated` plus `start_refusal` at the entries (R9) |
| FR-014 | user-guide pages, tool catalogue strings |
| SC-001..SC-005 | tests listed in quickstart.md |

## Test strategy (which layer proves what)

| Layer | Proves |
|---|---|
| core unit (`occupancy`) | free/taken for every lifecycle, archived ignored, Default key, legacy block naming the live one |
| core unit (wire, tools) | additive fields round-trip; old frames decode; tool schemas and descriptions; refusal text holds id, name, `start_session` hint |
| daemon integration | second `create_session` refused, catalog unchanged; 100 concurrent creates, one wins (SC-003); create_worktree with definition, missing CLI leaves nothing; restart/stop/start keep one; replace stops the old process; legacy catalog opens intact and refuses; adoption cap; review send reuses own stopped session |
| client reducer | start affordance absent when occupied; dialog fields and "None"; refusal text shown; confirm-replace step |
| geometry gates / quickstart §B | dialog layout, sidebar row without the action (rendering only) |

## Complexity Tracking

No violations.
