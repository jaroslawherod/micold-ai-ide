# Tasks: One Session per Worktree, Chosen at Start

**Input**: `/specs/776-one-session-per-worktree/` (spec.md, plan.md, research.md R1-R11, data-model.md, contracts/wire.md, contracts/mcp-tools.md, quickstart.md)

**Tests**: mandatory (Constitution I). Every test task precedes the implementation it covers and must be seen failing for the right reason first.

**Format**: `- [ ] T### [P?] [US?] Description with file path`. `[P]` = different files, no dependency on an unfinished task.

User stories: US1 refused second session (P1), US2 defined at start (P1), US3 resume/restart replace (P2), US4 legacy catalogs (P3).

## Phase 1: Setup

- [ ] T001 Add the empty `pub mod occupancy;` export in `crates/micold-core/src/lib.rs` and create `crates/micold-core/src/occupancy.rs` with the module doc comment only, so tests in T003 compile against a real module path.

## Phase 2: Foundational (blocks every story)

- [ ] T002 [P] Test the wire additions in `crates/micold-core/tests/protocol_roundtrip.rs`: `WorktreeCreate.session: Option<SessionDefinition>`, `OperationResult::WorktreeCreated { dir_name, session: Option<SessionId> }`, `AttachItem::Session { id, replace }`, `AttachResult.existing: Option<ExistingSession>`, `RefuseReason::LocationHasSession` round-trip; a frame without the new fields decodes with the defaults (`None` / `false`); `PROTOCOL_VERSION == 41`.
- [ ] T003 [P] Test the rule in `crates/micold-core/src/occupancy.rs` (`#[cfg(test)]`): `occupancy` is `Free` for no sessions and for all-archived (FR-010), `Taken` for every `SessionLifecycle` (starting, running, restarting, stopped, failed, exited) and names the first in list order; `Default` is a key like any worktree (FR-011); `start_block` is `Clear` for the location's only session and `Blocked { live }` when another un-archived one is starting/running/restarting (FR-013); `refusal_text` holds the id, the name and the `start_session` hint, with the project-root wording for Default (contracts/mcp-tools.md).
- [ ] T004 Implement `ExistingSession`, `Occupancy`, `StartBlock`, `occupancy`, `start_block`, `refusal_text`, `start_block_text` in `crates/micold-core/src/occupancy.rs` per data-model.md.
- [ ] T005 Add the wire types and fields of T002 in `crates/micold-core/src/protocol/messages.rs` (all serde-defaulted; `SessionDefinition.prompt` "non-empty after trim; empty is `None`"), `RefuseReason::LocationHasSession` with its `text()` arm in `crates/micold-core/src/attach.rs`, bump `PROTOCOL_VERSION` to 41 with its history entry and `SCHEMA_HASH` in `crates/micold-core/src/protocol/version.rs`; keep the daemon and client compiling by matching the new fields with `..` and ignoring them (behaviour lands in later milestones).

**Checkpoint**: `mise run test-core` green; no behaviour change for users yet.

## Phase 3: User Story 1 - A second session in a worktree is refused (P1) MVP

**Goal**: no path can add a second session to a location that has one; the refusal names the existing session.

**Independent test**: `cargo test -p micold-daemon --test one_session_per_worktree` (refusals, catalog unchanged, 100 concurrent creates, one winner).

- [ ] T006 [P] [US1] Create daemon integration test `crates/micold-daemon/tests/one_session_per_worktree.rs` (real daemon over its socket, pattern of `mcp_create_session.rs`): second `SessionCreate` RPC refused in each lifecycle state with `OperationError { kind: Refused, message: refusal_text, detail: Some(existing id) }` and the catalog unchanged (US1 s1, s4); Default root refused (FR-011); 100 concurrent creates on an empty worktree, exactly one `Ok` (SC-003, FR-005); a worktree whose only session is archived accepts one (FR-010).
- [ ] T007 [P] [US1] Add MCP tests in `crates/micold-daemon/tests/mcp_create_session.rs` and `crates/micold-core/tests/mcp_tools_catalog.rs`: `create_session` on an occupied worktree fails with category `Conflict` and the contract message, checked after policy and before the CLI availability check; the tool description gains the "at most one session" sentence.
- [ ] T008 [US1] Test in `crates/micold-daemon/tests/one_session_per_worktree.rs` (same file as T006, run after it): review send into a worktree whose own session is stopped starts that session and adds none (R10).
- [ ] T009 [US1] Make `Catalog::create_session` check `occupancy` and push under one `&mut self`, returning `CreateSessionError::Occupied(ExistingSession) | Io` in `crates/micold-daemon/src/catalog.rs`; adapt the passthrough in `crates/micold-daemon/src/state.rs`, `ops::create_session_with_prompt` in `crates/micold-daemon/src/ops.rs` and the `crates/micold-daemon/src/runs.rs` caller.
- [ ] T010 [US1] Map the typed refusal in `crates/micold-daemon/src/server.rs` (`SessionCreate` -> `OperationError` per contracts/wire.md) and `crates/micold-daemon/src/mcp/tools.rs` (`create_session` -> `Conflict`, early check before the CLI check, plus the in-lock one); update the `create_session` description in `crates/micold-core/src/mcp/tools.rs`.
- [ ] T011 [US1] Make `review_send_to_new_session` in `crates/micold-daemon/src/state.rs` reuse the location's own session (start it) instead of creating a second one (R10).
- [ ] T012 [P] [US1] Test in `crates/micold-client/tests/features_session.rs` that a refused create reply shows the refusal text (notice) and leaves the model unchanged; then show it in `crates/micold-client/src/shell/daemon_sync.rs`.
- [ ] T013 [P] [US1] Document the refusal in `docs/user-guide/agent-tools.md` (`create_session` error naming the existing session) and `docs/user-guide/worktrees-and-sessions.md` (one session per worktree) (FR-014, Constitution VII).

**Checkpoint**: `mise run gate` green; `create_session` and the RPC refuse a second session.

## Phase 4: User Story 2 - The session is defined when the worktree starts (P1)

**Goal**: worktree and session are created together with the chosen CLI and first prompt; the sidebar offers a start only on a location without a session.

**Independent test**: `create_worktree` with `ai_cli` + `prompt` yields one defined session; the dialog does the same; no add action on an occupied row.

- [ ] T014 [P] [US2] Daemon tests in `crates/micold-daemon/tests/mcp_create_worktree.rs` and `crates/micold-daemon/tests/one_session_per_worktree.rs`: `create_worktree` with `ai_cli`/`prompt` (and `prompt` only = default CLI) returns `session`, `lifecycle`, `prompt_delivered`; no definition = byte-identical old reply and `create_session` then works (US2 s5); missing CLI leaves no worktree and no session (US2 s4); worktree created but session failed names the worktree and leaves it without a session; `WorktreeCreate` RPC with `session: Some` does the same; a Default-session caller of `create_worktree` with and without a definition creates the worktree and, with one, runs the new session in the new worktree, never in Default (Constitution III exception, research R6).
- [ ] T015 [P] [US2] Tool-schema tests in `crates/micold-core/tests/mcp_tools_catalog.rs`: `create_worktree` gains optional `ai_cli` (enum of `AiCli::ALL` tool names) and `prompt`.
- [ ] T016 [US2] Implement `ops::create_worktree_with_session` (CLI check at the future cwd, create, then session via the same locked create path) in `crates/micold-daemon/src/ops.rs`; wire it from `WorktreeCreate` in `crates/micold-daemon/src/server.rs` and from `create_worktree` in `crates/micold-daemon/src/mcp/tools.rs` (policy as `CreateWorktree` and `CreateSession`); update `crates/micold-core/src/mcp/tools.rs` schema and descriptions.
- [ ] T017 [P] [US2] Client reducer tests in `crates/micold-client/tests/features_worktree_form.rs`: dialog has an AI CLI choice (default CLI preselected, "None" last) and a first-prompt field; confirm sends `session: Some(definition)` or `None`; a prompt with "None" is not representable.
- [ ] T018 [P] [US2] Client tests in `crates/micold-client/tests/features_session.rs`: `start_intent` offers a start only for a `Free` location (`occupancy`), none for an occupied one, Default included; `WorktreeCreated.session` is selected and viewed as `SessionCreated` is.
- [ ] T019 [US2] Add the fields to `crates/micold-client/src/features/worktree_form.rs` and render them in `crates/micold-client/src/ui/worktree_form.rs` (existing select and text-field primitives only, Principle VIII); send the definition and select the returned session in `crates/micold-client/src/shell/daemon_sync.rs`.
- [ ] T020 [US2] Make `start_intent` in `crates/micold-client/src/features/session.rs` ask `occupancy`, and have `crates/micold-client/src/ui/sidebar.rs` render exactly one start action for a free location and none for an occupied one (FR-004).
- [ ] T021 [P] [US2] Update `docs/user-guide/agent-tools.md` (`create_worktree` inputs `ai_cli` and `prompt`, reply fields).
- [ ] T036 [P] [US2] Update `docs/user-guide/worktrees-and-sessions.md` (dialog fields, definition at start, no add action on a started worktree).
- [ ] T022 [US2] Visual pass of quickstart Part B items 1-2 via the `visual-pass` skill; save evidence in `specs/776-one-session-per-worktree/tdd/`.

## Phase 5: User Story 3 - Resume and restart replace, never add (P2)

**Goal**: restart/start keep the same session; a confirmed provider resume replaces the occupant; "attach all" catalogues one session per location.

**Independent test**: `cargo test -p micold-daemon --test attach_apply --test one_session_per_worktree`.

- [ ] T023 [P] [US3] Daemon tests in `crates/micold-daemon/tests/one_session_per_worktree.rs`: stop, `start_session`, restart keep the same id and one session (FR-008a, FR-009 pinned); `start_session` on a running session is still a no-op success.
- [ ] T024 [P] [US3] Daemon tests in `crates/micold-daemon/tests/attach_apply.rs`: resume into an occupied location without `replace` is `Refused(LocationHasSession)` with `existing`, nothing stopped or written; with `replace: true` the old process is stopped first, the new record persists, then the old is archived (R7), a failed write leaves the old listed; a batch of several sessions for one location keeps the most recent (`cap_per_location`), the rest stay resumable; a single explicit resume is never capped (FR-012).
- [ ] T025 [P] [US3] Core tests in `crates/micold-core/tests/attach_discovery.rs`: `cap_per_location` keeps the greatest `last_activity` per `AttachTarget` and drops every candidate for an occupied one; `catalog_adoption.rs` in `crates/micold-daemon/tests/` covers `adopt_discovered_sessions` catalogue at most one per location.
- [ ] T026 [US3] Implement `cap_per_location` in `crates/micold-core/src/attach.rs`; `Catalog::replace_session` and the adoption cap in `crates/micold-daemon/src/catalog.rs`; `attach_apply` replace and cap in `crates/micold-daemon/src/state.rs`.
- [ ] T027 [P] [US3] Client tests in `crates/micold-client/tests/features_attach.rs` and `attach_dialog.rs`: a `Refused(LocationHasSession)` result opens a confirmation naming both sessions; confirm re-sends the item with `replace: true`; cancel changes nothing.
- [ ] T028 [US3] Add the confirm-replace step in `crates/micold-client/src/features/attach.rs` and render it in `crates/micold-client/src/ui/attach_dialog.rs`.
- [ ] T029 [P] [US3] Document resume-replaces and its confirmation in `docs/user-guide/worktrees-and-sessions.md`.
- [ ] T030 [US3] Visual pass of quickstart Part B item 3 (confirmation naming both sessions, cancel) via `visual-pass`; evidence in `specs/776-one-session-per-worktree/tdd/`.

## Phase 6: User Story 4 - Existing worktrees with several sessions (P3)

**Goal**: a legacy catalog opens intact; no extra session can be created; a start is refused while another of the location's sessions is live.

**Independent test**: `cargo test -p micold-daemon --test one_session_per_worktree legacy`.

- [ ] T031 [P] [US4] Daemon tests in `crates/micold-daemon/tests/one_session_per_worktree.rs`: a catalog fixture with two sessions in one worktree loads with both listed and byte-identical records (SC-005); `create_session` refused; start of a stopped session while the other runs is a no-op (nothing spawned, no record rewritten) and names the live one (FR-013); delete of an extra frees the path; sequence test over create/start/stop/restart/resume/delete from two connections on a fresh catalog never shows two un-archived sessions (SC-001).
- [ ] T032 [P] [US4] Client tests in `crates/micold-client/tests/features_session.rs`: selecting or starting the blocked row views it without starting and shows `start_block_text` as tooltip and notice; add to `crates/micold-core/tests/mcp_tools_catalog.rs` that the `start_session` description states the rule and its refusal (FR-014).
- [ ] T033 [US4] Implement `DaemonState::start_refusal` and the in-lock check in `start_session_gated` in `crates/micold-daemon/src/state.rs`, applied by `SessionStart`/`SessionRestart` in `crates/micold-daemon/src/server.rs` and by `start_session` in `crates/micold-daemon/src/mcp/tools.rs` (`Conflict`); update the `start_session` description in `crates/micold-core/src/mcp/tools.rs`.
- [ ] T034 [US4] Ask `occupancy::start_block` in `crates/micold-client/src/features/session.rs` and render the tooltip in `crates/micold-client/src/ui/sidebar.rs`.
- [ ] T035 [P] [US4] Document legacy worktrees (extras kept, refusal while one runs, delete extras) in `docs/user-guide/worktrees-and-sessions.md` and the `start_session` error in `docs/user-guide/agent-tools.md`.

## Dependencies

- Phase 2 blocks all stories. T002/T003 before T004/T005.
- US1 (T006-T013) needs Phase 2. US2 needs US1's locked create (T009). US3 needs T005 (wire) and US1. US4 needs `start_block` (T004) and US1.
- Within a story: tests, then implementation, then docs.
- Files shared across tasks (`one_session_per_worktree.rs`, `state.rs`, `session.rs`, the user-guide pages) are edited in order of task ID, not in parallel.

## Implementation Strategy

MVP: Phases 1-3 (rule, wire, refusal everywhere a session is created). Then US2, US3, US4, each its own PR. Quickstart Part A is the automated pass; Part B items are tasks T022 and T030.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — The rule and the refusal 🎯 MVP

- **Tasks**: T001–T013
- **Deliverable**: A second session in a worktree or the project root is refused by the daemon (RPC, `create_session`, review send), naming the existing session, and the wire carries the optional fields for the rest of the feature.
- **Satisfies**: US1 acceptance scenarios 1–4; FR-001, FR-002, FR-003, FR-005, FR-011, FR-014 (refusal part); SC-002, SC-003
- **Verify**: `cargo test -p micold-daemon --test one_session_per_worktree` and `mise run test-core`
- **Depends on**: —
- **Tier**: full

### M2 — Define the session at worktree start (daemon and tools)

- **Tasks**: T014–T016, T021
- **Deliverable**: `create_worktree` (MCP) and the `WorktreeCreate` RPC accept an AI CLI and first prompt and create the worktree and its one session together; a missing CLI creates nothing.
- **Satisfies**: US2 acceptance scenarios 2, 4, 5; FR-006, FR-010, FR-014 (`create_worktree`)
- **Verify**: `cargo test -p micold-daemon --test mcp_create_worktree --test one_session_per_worktree`
- **Depends on**: M1
- **Tier**: full

### M3 — Dialog fields and no add action

- **Tasks**: T017–T020, T022, T036
- **Deliverable**: The create-worktree dialog has an AI CLI choice and a first-prompt field that start the session with the worktree; the sidebar offers a start action only on a location without a session.
- **Satisfies**: US2 acceptance scenarios 1, 3; FR-004, FR-006, FR-014 (dialog)
- **Verify**: `cargo test -p micold-client --test features_worktree_form --test features_session` and quickstart §B items 1–2
- **Depends on**: M2
- **Tier**: full

### M4 — Resume replaces, never adds

- **Tasks**: T023–T030
- **Deliverable**: Resuming a provider session into an occupied worktree asks for confirmation and replaces the occupant; "attach all" catalogues one session per location; restart and start keep one session.
- **Satisfies**: US3 acceptance scenarios 1–4; FR-007, FR-008a, FR-008b, FR-009, FR-012, FR-014 (resume)
- **Verify**: `cargo test -p micold-daemon --test attach_apply --test one_session_per_worktree` and quickstart §B item 3
- **Depends on**: M1
- **Tier**: full

### M5 — Legacy catalogs stay intact

- **Tasks**: T031–T035
- **Deliverable**: A catalog with several sessions in a worktree opens untouched, refuses new sessions, and refuses a start while another of its sessions is live, naming it.
- **Satisfies**: US4 acceptance scenarios 1–3; FR-013, FR-014 (`start_session`); SC-001, SC-005
- **Verify**: `cargo test -p micold-daemon --test one_session_per_worktree` and `cargo test -p micold-client --test features_session`
- **Depends on**: M1, M3, M4
- **Tier**: full
