# Implementation Plan: MCP: a session ends itself and removes its own worktree

**Branch**: `feat/1602_mcp-let-a-session-stop-itself-and-delete-its-own` | **Date**: 2026-10-29 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/1602-mcp-self-delete/spec.md`

## Summary

Three changes to the daemon's MCP tool server (feature 034):

1. **Policy** (`micold-core`): `decide` stops refusing `delete_worktree` on the caller's own worktree and
   `stop_session` / `delete_session` on the caller's own id. Each now returns `Confirm`, flagged
   self-targeted. `stop_session` and `delete_session` take an optional id (omitted means the caller).
2. **Deferred action** (`micold-daemon`): for a self-targeted, accepted request the tool returns its
   result, the connection handler writes the reply and waits for the peer to read it (bounded), and only
   then runs the stop or removal on its own task. A failure there is logged and broadcast as a new
   `DaemonMsg::DeferredActionFailed`, which each window shows with `notify_error` (FR-009, ledger C2).
3. **Loss summary**: `delete_worktree` reads the worktree's uncommitted files and unpushed commits with a
   new `Git::worktree_loss`, and `ConfirmationRequested` carries them. The dialog lists them (FR-006, all
   deletes, own or not).

Wire change: one protocol bump, 73 to 74 (two fields on `ConfirmationRequested`, one new `DaemonMsg`).

## Technical Context

**Language/Version**: Rust (workspace edition), iced for the client.

**Primary Dependencies**: existing only: `tokio`, `serde`, `tracing`, the in-repo `Git` trait (`micold-core/src/git.rs`, impls `GitCli` and `FakeGit`). No new crate.

**Storage**: none new. Loss summary is read from git at prompt time and never stored (Principle IV).

**Testing**: `cargo test` via `mise run test-core` and `mise run gate`: core unit (policy table, status and log parsers, wire round trip, schema-hash pin), daemon integration (`crates/micold-daemon/tests/mcp/`), client (`crates/micold-client/tests/features_agent_confirm.rs`, geometry gates), quickstart §B visual pass.

**Target Platform**: Linux, macOS, Windows (Principle VI).

**Project Type**: desktop app plus session service (daemon), workspace of `micold-core`, `micold-daemon`, `micold-client`, `micold-material`.

**Performance Goals**: SC-001: session stopped and directory gone within 10 s of the result. Loss read under 1 s on a repository with 50 changed files (two local git calls).

**Constraints**: local git only, no remote contact (Principle IV). The prompt keeps the 60 s timeout. A list shows at most 50 entries plus a count (SC-003).

**Scale/Scope**: about 15 files including tests, 1 protocol bump, 1 user-guide page, amendments to 034 spec and contracts.

## Constitution Check

*GATE: passed before Phase 0; re-checked after Phase 1 design (still passes).*

| Principle | Status | Evidence |
|---|---|---|
| I. Test-First | PASS | Every code unit has a named failing test first in the Test strategy table; tasks order test tasks before code. |
| II. Multi-Session | PASS | Deferred action addresses one session id and one `(project, dir_name)`. Concurrent delete or rename of W withdraws the pending prompt through the existing `worktree_gone` and `session_gone` paths (spec Edge Cases). No shared state added. |
| III. Worktree Integration | PASS | The app, not a manual git step, removes W. Default session refusal (FR-007) is unchanged. A Default session stopping or deleting itself touches no worktree (FR-016). |
| IV. Local-First | PASS | `git status` and `git log` read local refs only. No network. |
| V. Rust + iced | PASS | Self-targeting is a typed flag on the decision (`PolicyDecision::Confirm`), the deferred step an enum `Deferred`, so "result before action" is the only constructible order. |
| VI. Cross-Platform | PASS, one risk | Removal reuses `ops::delete_worktree`, whose 3 s retry handles Windows handles held by killed shells. See research R6 for the residual risk and its test. |
| VII. Documentation | PASS | `docs/user-guide/agent-tools.md`, tool descriptions in `tools/list`, 034 amendments (FR-012, FR-017) are tasks in the same PR. |
| VIII. Reusable UI | PASS | The loss list is added to the existing `ui/confirm_agent_request.rs` dialog using shared material primitives; no new dialog type (spec Assumptions). Task checks for an existing bounded-list widget before adding one. |

## Requirement map

| FR | Where |
|---|---|
| FR-001, FR-002 | `micold-core/src/mcp/policy.rs` (`decide`): drop own-worktree refusal; the live-sessions conflict in `daemon/src/mcp/tools.rs::delete_worktree` already covers `stop_sessions: false` because the caller is live in W. |
| FR-003, FR-004, FR-008 | `Deferred` + connection handler: `daemon/src/mcp/tools.rs`, `daemon/src/mcp/server.rs`; result text in `tools.rs::delete_worktree`. [data-model.md](./data-model.md), R2. |
| FR-005 | Unchanged confirmation path: nothing runs before `ask_user` returns `Ok`; `Deferred` is only built after it. |
| FR-006 | `Git::worktree_loss`, `LossSummary` on the wire, dialog. R3, R4. |
| FR-007 | Unchanged branch in `decide` (Default caller); test kept. |
| FR-009 | `Deferred::run` failure path: `tracing::error!` plus `DaemonMsg::DeferredActionFailed`; client `notify_error`. R5. |
| FR-010 | Existing audit line in `tools::call` (written when the result is produced, before the deferred step); `target` for stop and delete of self names the session. |
| FR-011 | `decide` self branches; `parse_call` optional id; `interrupt`, `send`, `read` self refusals untouched. [contracts/mcp-tools.md](./contracts/mcp-tools.md). |
| FR-012, FR-017 | `micold-core/src/mcp/tools.rs` descriptions and schema; `docs/user-guide/agent-tools.md`; `specs/034-daemon-mcp-server/{spec.md,contracts/mcp-tools.md}` amendment. |
| FR-013 | Non-self paths keep their code; regression tests in 034's suites stay green (SC-005). |
| FR-016 | `decide`: the Default guard stays on `RenameWorktree` and `DeleteWorktree` only. |

## Test strategy

| Behavior | Layer | Test location |
|---|---|---|
| Own worktree, own session: `Confirm`, not `Refuse`; Default and interrupt/send/read self still refused | core unit | `micold-core/src/mcp/policy.rs` tests (table) |
| `parse_call`: `session` optional for `stop_session` and `delete_session` | core unit | `micold-core/src/mcp/tools.rs` tests |
| `porcelain -z` and log parsers, 50-entry bound, counts | core unit | new `micold-core/src/worktree/loss.rs` tests |
| `GitCli::worktree_loss` on a real temp repo: dirty, untracked, upstream ahead, no upstream, clean, non-repo | core integration | `micold-core/tests/` beside the existing `GitCli` tests |
| `ConfirmationRequested` and `DeferredActionFailed` round trip; schema-hash pin; version 74 | core | `protocol_roundtrip.rs`, `schema_hash.rs` |
| Result written before stop (order), peer-close grace, hangup still runs the deferred step | daemon integration | `daemon/tests/mcp/mcp_confirmations.rs` (new cases) |
| Accept self-delete: session stopped, directory and branch gone; `delete_branch: false`; decline and timeout change nothing (SC-002) | daemon integration | `daemon/tests/mcp/mcp_lifecycle_tools.rs` |
| Failed deferred removal logs and broadcasts the notice | daemon integration | `daemon/tests/mcp/mcp_confirmations.rs` (fault injected through `FakeGit` or a read-only parent dir on Unix) |
| Prompt lists files and commits, none when clean, bounded list, other worktree too | daemon integration plus client | daemon test reads the broadcast prompt; `features_agent_confirm.rs` checks the dialog |
| Dialog layout of the list and the self-targeted line | geometry gates, quickstart §B | new `crates/micold-client/tests/confirm_agent_loss_list_geometry.rs`, then visual pass |
| Audit line (FR-010) on accept, decline and a failed deferred step | daemon integration | `daemon/tests/mcp_audit_log.rs` |
| Default session stops or deletes itself after confirmation, no worktree touched (FR-016, story 4); policy-table case for it | daemon integration, core unit | `daemon/tests/mcp/mcp_lifecycle_tools.rs`, `crates/micold-core/tests/mcp_policy.rs` |
| Windows open-handle removal | CI Windows job | the daemon integration test above, run on all three platforms (SC-006) |

## Project Structure

### Documentation (this feature)

```text
specs/1602-mcp-self-delete/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── mcp-tools.md        # tool-surface delta (034 amendment)
│   └── protocol-delta.md   # wire delta, 73 to 74
└── tasks.md                # /speckit-tasks
```

### Source Code (existing paths; new files marked)

```text
crates/micold-core/src/
├── mcp/policy.rs            # decide: self-targeted Confirm
├── mcp/tools.rs             # descriptions, optional session id
├── git.rs                   # Git::worktree_loss (GitCli, FakeGit)
├── worktree/loss.rs         # NEW (declared with `mod loss;` in worktree.rs): LossSummary building, parsers, bound
└── protocol/{messages.rs,version.rs}   # fields, DeferredActionFailed, 74
crates/micold-daemon/src/mcp/
├── tools.rs                 # delete_worktree, stop_session, delete_session: Deferred, loss
└── server.rs                # until_answered made generic over the call output; write reply, await peer close, run Deferred
crates/micold-daemon/src/mcp/confirm.rs   # ConfirmRequest gains loss, self_targeted
crates/micold-client/src/
├── features/agent_confirm.rs, ui/confirm_agent_request.rs   # Prompt fields, dialog list
└── shell/daemon_sync.rs     # map new fields; DeferredActionFailed -> notify_error
docs/user-guide/agent-tools.md
specs/034-daemon-mcp-server/{spec.md,contracts/mcp-tools.md}
```

**Structure Decision**: no new crate or module tree. One new core file (`worktree/loss.rs`) holds the pure,
testable part; everything else extends the existing 034 path.

## Complexity Tracking

No constitution violations. One accepted risk (R6, Windows retry window) is tracked in research.
