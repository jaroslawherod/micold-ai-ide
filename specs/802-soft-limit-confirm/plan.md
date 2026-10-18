# Implementation Plan: Soft Session Limit - Confirm Before a Parallel Session

**Branch**: `feat/802_make-one-session-per-worktree-a-soft-limit-confirm` | **Date**: 2026-10-18 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/802-soft-limit-confirm/spec.md`

## Summary

776 already made the limit soft: `allow_additional_session: true` skips the daemon's check, and the
row entry "Start another session..." and the blocked start already open the confirmation. One
person-initiated route is left: the start action (default or override-list AI CLI) sent for a
location the window believes free. The daemon refuses it (`OperationError`, `ErrorKind::Refused`,
`detail` = the existing session's id) and the window shows the refusal as an error. This feature
closes that route on the client only, in two layers:

1. **Pre-check** in `on_session_start_requested`: if `State::start_offered(location)` is false, open
   the confirmation instead of sending (the window already knows the location is occupied).
2. **Refusal fallback** for the stale case (the catalog push has not arrived): the
   `PendingOp::CreateSession` error arm recognises the occupied refusal (`Refused` with a session id
   `detail`), and, for a create that carried `allow_additional_session: false` and the person's
   chosen CLI, opens the same confirmation instead of `notify_error`. Only a person-initiated start
   does: `SessionMsg::StartRequested` gains `automatic: bool`; the startup auto-start in `main.rs`
   (~355) sets it, and its refusal keeps the error notice (R7). When the window's catalog
   lacks the occupant the dialog draws generic text ("already has a session") instead of nothing, so
   a set `additional_target` always has a visible dialog.

The confirmation keeps the AI CLI the person chose (override list), via a new `additional_provider`
beside `additional_target`. Agents are unchanged (FR-005): decision documented in the user guide and
tool descriptions. No wire, daemon or catalog change.

## Technical Context

**Language/Version**: Rust, stable toolchain via `mise`

**Primary Dependencies**: existing only (`iced`, `micold-core`). No new crate.

**Storage**: none; no persisted change.

**Testing**: `mise run test-core` (tool catalogue text), client tests in `crates/micold-client/tests/`
and `main_tests.rs` (reducer and shell arms), daemon test unchanged (regression:
`mcp_create_session.rs`), `mise run gate`.

**Target Platform**: Linux, macOS, Windows; window dialog, no platform code.

**Project Type**: desktop app (iced client + daemon + core).

**Performance Goals**: none.

**Constraints**: wire protocol and daemon default-refuse unchanged (spec Assumptions).

**Scale/Scope**: about 4 client functions, 1 field, 2 doc pages, 2 tool descriptions.

## Constitution Check

*GATE: passed before Phase 0; re-checked after Phase 1: unchanged.*

- [x] **I. Test-First**: PASS. Each route and the fallback gets a failing client test first (tasks
  order tests before code).
- [x] **II. Multi-Session Support**: PASS. Strengthens it: a second session in a worktree is
  reachable from every person route; sessions stay independent.
- [x] **III. Worktree Integration**: PASS. Default root behaves as a worktree; no worktree change.
- [x] **IV. Local-First Storage**: PASS. Nothing remote, nothing stored.
- [x] **V. Rust + iced Stack**: PASS. The refusal is recognised from typed fields (`ErrorKind`,
  `detail` id), not by matching message text; `PendingOp::CreateSession` carries typed data.
- [x] **VI. Cross-Platform Parity**: PASS. Same dialog on all platforms.
- [x] **VII. Documentation First-Class**: PASS. `docs/user-guide/agent-tools.md` and
  `worktrees-and-sessions.md` change in the same PR (FR-006).
- [x] **VIII. Reusable UI Component Foundation**: PASS. Reuses `ConfirmAdditionalSessionDialog`
  as is; no new widget.

## Project Structure

```text
specs/802-soft-limit-confirm/   spec, plan, research, data-model, contracts/, quickstart
crates/micold-client/src/
  features/session.rs           additional_provider field; additional_requested keeps provider;
                                additional_request() uses it; message for the refusal fallback
  shell/daemon_sync.rs          on_session_start_requested thin dispatcher over a reducer decision in
                                features/session.rs (open or send, so tests can link it); matches at
                                ~232, ~1237, ~1735 follow the new PendingOp shape; PendingOp::CreateSession
                                carries (location, provider); error arm opens the confirmation
  ui/confirm_additional_session.rs  generic text when the catalog lacks the occupant
  main.rs                       startup auto-start sets `automatic`; PendingOp match (~351)
crates/micold-core/src/mcp/tools.rs   create_session / start_session descriptions (FR-006)
docs/user-guide/{agent-tools,worktrees-and-sessions}.md
```

## Requirement map

| Requirement | Where |
|---|---|
| FR-001, FR-004 | pre-check and refusal fallback open the existing confirmation; blocked start and row entry already do (regression tests) |
| FR-002 | `on_additional_session_confirmed` sends `allow_additional_session: true` (776), with the chosen CLI |
| FR-003 | `additional_cancelled` (776); fallback creates nothing before confirm |
| FR-005 | daemon and MCP untouched; proven by the daemon refusal tests (`mcp_create_session.rs`) and the `mcp_tools_catalog.rs` pin |
| FR-006 | tool descriptions in `micold-core/src/mcp/tools.rs` and the two user-guide pages; `mcp_tools_catalog.rs` pins the text |
| FR-007 | the pre-check and fallback send nothing flagged until `AdditionalConfirmed`; daemon refusal unchanged |
| SC-001..004 | client tests per route; catalog-unchanged assertion on decline; existing daemon tests for SC-004 |

## Test strategy (which layer proves what)

- **Core unit**: tool descriptions mention confirmation and the flag (`mcp_tools_catalog.rs`).
- **Client reducer** (`crates/micold-client/tests/`): `additional_requested` keeps the provider;
  decline clears target and provider; the dialog draws generic text when the occupant is not in the catalog; an automatic start's refusal keeps the notice.
- **Client shell** (`main_tests.rs`, as `a_refused_session_create_shows_the_refusal_text_and_changes_nothing`,
  which is changed to the new behaviour): an occupied location opens the confirmation and sends no
  `SessionCreate`; a stale `Refused` reply opens it; confirm sends one flagged create with the
  chosen CLI; other `Refused` replies (no id detail) still `notify_error`.
- **Daemon**: no change; existing refusal and flagged-create tests are the regression for FR-005/007.
- **Visual pass (quickstart §B)**: none new; the dialog is unchanged.

## Complexity Tracking

No violations.
