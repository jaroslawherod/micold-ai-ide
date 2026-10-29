---

description: "Task list for #1602: MCP self stop and own-worktree delete"
---

# Tasks: MCP: a session ends itself and removes its own worktree

**Input**: Design documents from `/specs/1602-mcp-self-delete/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Tests**: Mandatory (Constitution I). Every test task precedes the implementation task it covers and must fail first.

**Documentation**: Mandatory (Constitution VII). Each story's user-guide task sits in the story's milestone.

**Cross-platform**: Daemon tests run on Linux, macOS and Windows (Constitution VI, SC-006).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: different files, no dependency on an unfinished task.
- Paths are relative to the repository root. `core` = `crates/micold-core/src`, `daemon` = `crates/micold-daemon/src`, `client` = `crates/micold-client/src`.

## Phase 1: Setup

None. The change extends existing crates (plan: no new crate or module tree).

## Phase 2: Foundational (the one protocol bump, 73 to 74)

- [ ] T001 [P] Failing wire tests in `crates/micold-core/tests/protocol_roundtrip.rs` and `crates/micold-core/tests/schema_hash.rs`: `ConfirmationRequested` with `loss` and `self_targeted`, `DaemonMsg::DeferredActionFailed`, `LossSummary`/`LossCommit` round trip; version 74; new schema-hash pin.
- [ ] T002 Add `LossSummary`, `LossCommit`, the two `ConfirmationRequested` fields and `DaemonMsg::DeferredActionFailed` in `core/protocol/messages.rs`; set `PROTOCOL_VERSION = 74` with its changelog comment in `core/protocol/version.rs`; update the pin. Fix constructors in daemon and client (`loss: None`, `self_targeted: false`) so the workspace compiles.

## Phase 3: User Story 1 + User Story 3 - own-worktree delete, guards stay (Priority: P1) 🎯 MVP

**Goal**: A session deletes its own worktree after confirmation: result first, then stop and removal; Default and other guards unchanged.

**Independent Test**: Quickstart steps 3 to 5 and 8 (Default refused); `mise run test-core` and the new daemon tests.

### Tests (write first, see them fail)

- [ ] T003 [P] [US3] Policy table tests in `core/mcp/policy.rs`: own worktree delete is `Confirm`, not `Refuse`; Default caller still refused `delete_worktree` and `rename_worktree`; the `delete_worktree` description in `tools/list` no longer says a session cannot stop or delete itself and mentions the confirmation (covers T014); `interrupt`, `send`, `read` on self still refused; `is_self_targeted` true only for the caller's own worktree.
- [ ] T004 [P] [US1] [US3] Daemon integration tests in `crates/micold-daemon/tests/mcp/mcp_lifecycle_tools.rs`: accept self-delete (result `scheduled`, session stopped, directory and branch gone, SC-001); `delete_branch: false` keeps the branch; decline and timeout change nothing (SC-002); `stop_sessions` false gives the live-sessions conflict with no prompt; unknown worktree gives `not_found` with no prompt; no attached window gives `needs_confirmation`; the result also names the directory and `branch_will_be_deleted` (FR-008); the prompt names that other sessions in W will stop; `branch_delete_failed` still counts the removal as done; the prompt withdrawn when another session deletes or renames W, or the caller closes while pending, runs no deferred step; Default caller refused with no prompt (SC-004).
- [ ] T005 [P] [US1] Daemon integration tests in `crates/micold-daemon/tests/mcp/mcp_confirmations.rs`: reply written before the stop; peer-close grace of 2 s; hangup still runs the deferred step; failed deferred removal (fault injected through `FakeGit`, or a read-only parent dir on Unix) logs an error and broadcasts `DeferredActionFailed`.
- [ ] T006 [P] [US1] Audit tests in `crates/micold-daemon/tests/mcp_audit_log.rs` (FR-010): one line on accept, decline, and a failed deferred step (extra `error` line).
- [ ] T007 [US1] Open-handle test (R6, SC-006) in `crates/micold-daemon/tests/mcp/mcp_lifecycle_tools.rs`: hold an open file handle inside W, released 4 s after the accept (longer than the current retry window of about 3 s); assert W is gone afterwards on all platforms. It only bites on Windows; its CI job is the proof.
- [ ] T008 [P] [US1] Client tests in `crates/micold-client/tests/features_agent_confirm.rs`: the dialog shows a line that the asking session ends when `self_targeted`; `DeferredActionFailed` raises `notify_error` naming the worktree and the reason.

### Implementation

- [ ] T009 [US3] `decide` in `core/mcp/policy.rs`: drop the own-worktree refusal for `delete_worktree`, return `Confirm`; add pure `is_self_targeted`; keep the Default guard (FR-001, FR-007, FR-016).
- [ ] T010 [US1] `daemon/mcp/confirm.rs`: `ConfirmRequest` gains `loss` and `self_targeted`; broadcast and `replay` carry both.
- [ ] T011 [US1] `daemon/mcp/tools.rs`: add the `Deferred` enum with only the `DeleteWorktree` variant (T029 adds the other two, avoiding `dead_code`); `delete_worktree` on the caller's own worktree builds `Deferred::DeleteWorktree` only after `ask_user` returned `Ok` and returns `{ scheduled, removed, branch_will_be_deleted }` (FR-003, FR-008).
- [ ] T012 [US1] `daemon/mcp/server.rs`: make `until_answered` generic over the call output; write the reply, wait for peer close (bounded 2 s), then run the `Deferred` on its own task via `ops::delete_worktree` with `stop_sessions: true`; on failure `tracing::error!`, audit `error` line and broadcast `DeferredActionFailed` (FR-009, R5).
- [ ] T013 [US1] Client: `client/shell/daemon_sync.rs` maps `self_targeted` into the prompt and `DeferredActionFailed` into `notify_error`; `client/features/agent_confirm.rs` and `client/ui/confirm_agent_request.rs` render the self-targeted line.
- [ ] T014 [US1] Tool text in `core/mcp/tools.rs`: remove "cannot stop or delete itself" and "own worktree cannot be deleted" from the `delete_worktree` description; add the confirmation, the scheduled result and that the session ends after its answer (FR-012).
- [ ] T015 [US1] Ticked when the Windows CI job of this milestone's PR is green: if T007 fails there, extend the retry bound of the directory removal in `daemon/ops.rs::delete_worktree` (shared with the sidebar delete) until it passes; otherwise leave `ops.rs` unchanged and record the result in the PR body.
- [ ] T016 [US1] Docs: `docs/user-guide/agent-tools.md` (own-worktree delete, confirmation, scheduled result, failure notice); amend `specs/034-daemon-mcp-server/spec.md` (FR-014, FR-015) and `specs/034-daemon-mcp-server/contracts/mcp-tools.md` for the delete case (FR-017).

**Checkpoint**: A session deletes its own worktree after confirmation on all three platforms.

## Phase 4: User Story 2 - the prompt shows what would be lost (Priority: P1)

**Goal**: Every `delete_worktree` prompt lists uncommitted files and unpushed commits, bounded to 50 each plus a count.

**Independent Test**: Quickstart steps 1, 2 and 6; the new core and daemon tests.

### Tests (write first)

- [ ] T017 [P] [US2] Parser unit tests in `core/worktree/loss.rs` (new file, `mod loss;` in `core/worktree.rs`): `status --porcelain -z` and log parsing, 50-entry bound, `*_more` counts, `unreadable`.
- [ ] T018 [P] [US2] Integration tests for `GitCli::worktree_loss` on a real temp repo in `crates/micold-core/tests/worktree_loss.rs`: dirty, untracked, upstream ahead, no upstream, clean, non-repo.
- [ ] T019 [P] [US2] Daemon tests in `crates/micold-daemon/tests/mcp/mcp_confirmations.rs`: the broadcast prompt lists files and commits, nothing when clean, bounded list, another worktree's delete too (scenarios 1 to 5).
- [ ] T020 [P] [US2] Client tests: `crates/micold-client/tests/features_agent_confirm.rs` (dialog lists files and commits, nothing when empty, "unreadable" line) and new `crates/micold-client/tests/confirm_agent_loss_list_geometry.rs` (list layout).

### Implementation

- [ ] T021 [US2] `core/worktree/loss.rs`: `WorktreeLoss`, parsers and `LossSummary::bounded` (50 entries plus count).
- [ ] T022 [US2] `Git::worktree_loss` in `core/git.rs` for `GitCli` (two local git calls, no remote) and `FakeGit`.
- [ ] T023 [US2] `daemon/mcp/tools.rs::delete_worktree` reads the loss at prompt time and sets `ConfirmRequest.loss` for every delete (FR-006).
- [ ] T024 [US2] Dialog list in `client/ui/confirm_agent_request.rs` and its `Prompt` field; first `grep -n` for an existing bounded-list widget in `micold-material` and reuse it (Principle VIII).
- [ ] T025 [US2] Docs: `docs/user-guide/agent-tools.md` loss list in the prompt; amend `specs/034-daemon-mcp-server/contracts/mcp-tools.md` for the prompt content.

**Checkpoint**: Every `delete_worktree` prompt shows what would be lost.

## Phase 5: User Story 4 - a session stops or deletes only itself (Priority: P3)

**Goal**: `stop_session` and `delete_session` accept no id or the caller's id, confirm, return the result first, then act.

**Independent Test**: Quickstart step 8 (`stop_session` without id).

### Tests (write first)

- [ ] T026 [P] [US4] Core tests: `parse_call` accepts an omitted `session` for `stop_session` and `delete_session` (`core/mcp/tools.rs`); policy table cases for self stop and delete, including a Default caller allowed (`core/mcp/policy.rs`, FR-016).
- [ ] T027 [P] [US4] Daemon tests in `crates/micold-daemon/tests/mcp/mcp_lifecycle_tools.rs`: stop and delete of self (omitted id and own id) return `scheduled` first then act; decline and timeout keep the session running; a Default session stops itself with no worktree touched; audit line (extend `crates/micold-daemon/tests/mcp_audit_log.rs`).

### Implementation

- [ ] T028 [US4] `core/mcp/tools.rs` schema and descriptions (optional `session`, drop "cannot stop or delete itself"); `core/mcp/policy.rs` `decide` returns `Confirm` for self `stop_session` and `delete_session` (FR-011).
- [ ] T029 [US4] `daemon/mcp/tools.rs`: extract `perform_stop_session` and `perform_delete_session` (R7); an omitted id means the caller; self-targeted calls build `Deferred::StopSession` or `Deferred::DeleteSession` and return `{ scheduled: true }`.
- [ ] T030 [US4] Docs: `docs/user-guide/agent-tools.md` (stop or delete self); amend `specs/034-daemon-mcp-server/spec.md` (FR-015 tool table) and `specs/034-daemon-mcp-server/contracts/mcp-tools.md`.

## Phase 6: Polish

Quickstart §B visual pass, task ticks and 034 wording check change no code: the close unit does them (milestones.md rule 4).

## Dependencies and order

- M3 runs after M2 (both edit `daemon/mcp/tools.rs` and the user guide).
- Foundational (T001, T002) before every story; it ships in M1.
- US3 guards and US1 share `decide` and the `Deferred` path: one milestone. US2 needs the wire fields from T002 and the `ConfirmRequest.loss` slot from T010. US4 needs `Deferred` (T011, T012).
- Within a story: tests, then core, then daemon, then client, then docs.
- Parallel: all `[P]` test tasks of a phase touch different files or different test cases appended to distinct functions.

## Implementation Strategy

MVP first: M1 delivers the request itself plus the guards and the wire bump. M2 adds the loss list on every delete prompt. M3 adds stop and delete of self.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Own-worktree delete after confirmation 🎯 MVP

- **Tasks**: T001–T016
- **Deliverable**: A session can call `delete_worktree` on its own worktree with `stop_sessions: true`; after the user accepts it gets a `scheduled` result, then the session stops and the worktree and branch are removed; a failed removal logs and shows an app notice; a Default session is still refused.
- **Satisfies**: US1 acceptance scenarios 1–6; US3 acceptance scenarios 1–2; FR-001, FR-002, FR-003, FR-004, FR-005, FR-007, FR-008, FR-009, FR-010, FR-012, FR-017 (delete case), FR-016; SC-001, SC-002, SC-004, SC-006
- **Verify**: `mise run gate`; the Windows CI job runs the T007 open-handle test; quickstart Part B steps 3–5, 7 and 8 (Default part)
- **Depends on**: —
- **Tier**: full

### M2 — Delete prompt lists what would be lost

- **Tasks**: T017–T025
- **Deliverable**: The confirmation prompt of every `delete_worktree` lists changed files and unpushed commits (50 each plus a count), shows nothing for a clean pushed worktree, and says when git could not be read.
- **Satisfies**: US2 acceptance scenarios 1–5; FR-006, FR-013; SC-003
- **Verify**: `mise run test-core`, `cargo test -p micold-client --test confirm_agent_loss_list_geometry`, the T019 daemon test; quickstart Part B steps 1, 2 and 6
- **Depends on**: M1
- **Tier**: full

### M3 — A session stops or deletes only itself

- **Tasks**: T026–T030
- **Deliverable**: `stop_session` and `delete_session` accept an omitted or own id, ask for confirmation, return `scheduled` first, then stop or delete the calling session; interrupt, send and read on self stay refused.
- **Satisfies**: US4 acceptance scenarios 1–3; FR-011, FR-012, FR-017 (session tools), FR-016; SC-005
- **Verify**: `mise run test-core` and the T027 daemon tests; quickstart Part B step 8 (`stop_session` without id)
- **Depends on**: M2
- **Tier**: full
