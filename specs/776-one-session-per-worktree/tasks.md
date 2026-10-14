# Tasks: One Session per Worktree, Chosen at Start

**Input**: `/specs/776-one-session-per-worktree/` (spec.md, plan.md, research.md R1-R14, data-model.md, contracts/wire.md, contracts/mcp-tools.md, quickstart.md)

**Tests**: mandatory (Constitution I). Every test task precedes the implementation it covers and must be seen failing for the right reason first.

**Format**: `- [ ] T### [P?] [US?] Description with file path`. `[P]` = different files, no dependency on an unfinished task.

User stories: US1 refused second session unless really requested (P1), US2 defined at start (P1), US3 resume/restart replace (P2), US4 worktrees with several sessions (P3), US5 worktree rows and main session (P2), US6 helper session on purpose (P2), US7 one main session, changeable (P2).

**Amended 2026-10-13** (soft limit, default session; research R12, R13). T001-T013 are merged (M1, #794) and stay ticked: the default path still refuses. T037-T049 are new; amended wording of open tasks is marked *(amended)*. Milestones are renumbered in *Milestones* below.

**Amended 2026-10-14** (main session and helpers; research R14, spec Session 2026-10-14). T014-T016 and T021 stay M3's; T050-T054 (core, wire v43, daemon) are new and go first in M3; T055-T057 are M5's. T038's `default_session` tests and T044's `last_viewed` are replaced (*amended*). T001-T013 and T037-T042 are merged and stay ticked; what they built that changes is listed in research R14.

## Phase 1: Setup

- [x] T001 Add the empty `pub mod occupancy;` export in `crates/micold-core/src/lib.rs` and create `crates/micold-core/src/occupancy.rs` with the module doc comment only, so tests in T003 compile against a real module path.

## Phase 2: Foundational (blocks every story)

- [x] T002 [P] Test the wire additions in `crates/micold-core/tests/protocol_roundtrip.rs`: `WorktreeCreate.session: Option<SessionDefinition>`, `OperationResult::WorktreeCreated { dir_name, session: Option<SessionId> }`, `AttachItem::Session { id, replace }`, `AttachResult.existing: Option<ExistingSession>`, `RefuseReason::LocationHasSession` round-trip; a frame without the new fields decodes with the defaults (`None` / `false`); `PROTOCOL_VERSION == 41`.
- [x] T003 [P] Test the rule in `crates/micold-core/src/occupancy.rs` (`#[cfg(test)]`): `occupancy` is `Free` for no sessions and for all-archived (FR-010), `Taken` for every `SessionLifecycle` (starting, running, restarting, stopped, failed, exited) and names the first in list order; `Default` is a key like any worktree (FR-011); `start_block` is `Clear` for the location's only session and `Blocked { live }` when another un-archived one is starting/running/restarting (FR-013); `refusal_text` holds the id, the name and the `start_session` hint, with the project-root wording for Default (contracts/mcp-tools.md).
- [x] T004 Implement `ExistingSession`, `Occupancy`, `StartBlock`, `occupancy`, `start_block`, `refusal_text`, `start_block_text` in `crates/micold-core/src/occupancy.rs` per data-model.md.
- [x] T005 Add the wire types and fields of T002 in `crates/micold-core/src/protocol/messages.rs` (all serde-defaulted; `SessionDefinition.prompt` "non-empty after trim; empty is `None`"), `RefuseReason::LocationHasSession` with its `text()` arm in `crates/micold-core/src/attach.rs`, bump `PROTOCOL_VERSION` to 41 with its history entry and `SCHEMA_HASH` in `crates/micold-core/src/protocol/version.rs`; keep the daemon and client compiling by matching the new fields with `..` and ignoring them (behaviour lands in later milestones).

**Checkpoint**: `mise run test-core` green; no behaviour change for users yet.

## Phase 3: User Story 1 - A second session in a worktree is refused (P1) MVP

**Goal**: no path can add a second session to a location that has one; the refusal names the existing session.

**Independent test**: `cargo test -p micold-daemon --test one_session_per_worktree` (refusals, catalog unchanged, 100 concurrent creates, one winner).

- [x] T006 [P] [US1] Create daemon integration test `crates/micold-daemon/tests/one_session_per_worktree.rs` (real daemon over its socket, pattern of `mcp_create_session.rs`): second `SessionCreate` RPC refused in each lifecycle state with `OperationError { kind: Refused, message: refusal_text, detail: Some(existing id) }` and the catalog unchanged (US1 s1, s4); Default root refused (FR-011); 100 concurrent creates on an empty worktree, exactly one `Ok` (SC-003, FR-005); a worktree whose only session is archived accepts one (FR-010).
- [x] T007 [P] [US1] Add MCP tests in `crates/micold-daemon/tests/mcp_create_session.rs` and `crates/micold-core/tests/mcp_tools_catalog.rs`: `create_session` on an occupied worktree fails with category `Conflict` and the contract message, checked after policy and before the CLI availability check; the tool description gains the "at most one session" sentence.
- [x] T008 [US1] Test in `crates/micold-daemon/tests/review_send.rs` (`r10_an_ended_session_is_started_again_and_no_second_one_is_created`; the stand-in `claude` and window helpers live there): review send into a worktree whose own session is stopped starts that session and adds none (R10).
- [x] T009 [US1] Make `Catalog::create_session` check `occupancy` and push under one `&mut self`, returning `CreateSessionError::Occupied(ExistingSession) | Io` in `crates/micold-daemon/src/catalog.rs`; adapt the passthrough in `crates/micold-daemon/src/state.rs`, `ops::create_session_with_prompt` in `crates/micold-daemon/src/ops.rs` and the `crates/micold-daemon/src/runs.rs` caller.
- [x] T010 [US1] Map the typed refusal in `crates/micold-daemon/src/server.rs` (`SessionCreate` -> `OperationError` per contracts/wire.md) and `crates/micold-daemon/src/mcp/tools.rs` (`create_session` -> `Conflict`, early check before the CLI check, plus the in-lock one); update the `create_session` description in `crates/micold-core/src/mcp/tools.rs`.
- [x] T011 [US1] Make `review_send_to_new_session` in `crates/micold-daemon/src/state.rs` reuse the location's own session (start it) instead of creating a second one (R10).
- [x] T012 [P] [US1] Test in `crates/micold-client/tests/features_session.rs` that a refused create reply shows the refusal text (notice) and leaves the model unchanged; then show it in `crates/micold-client/src/shell/daemon_sync.rs`.
- [x] T013 [P] [US1] Document the refusal in `docs/user-guide/agent-tools.md` (`create_session` error naming the existing session) and `docs/user-guide/worktrees-and-sessions.md` (one session per worktree) (FR-014, Constitution VII).

**Checkpoint**: `mise run gate` green; `create_session` and the RPC refuse a second session.

## Phase 3b: Soft limit (US1 amendment; M2)

**Goal**: an explicit request adds a second session; every default path still refuses. Daemon, wire and tools only; no UI.

**Independent test**: `cargo test -p micold-daemon --test one_session_per_worktree soft_limit` and `mise run test-core`.

- [x] T037 [P] [US1] Wire tests in `crates/micold-core/tests/protocol_roundtrip.rs`: `allow_additional_session` on `SessionCreate`, `SessionStart`, `SessionRestart` and `AttachItem::Session` round-trips; a frame without it decodes `false`; `PROTOCOL_VERSION == 42`.
- [x] T038 [P] [US1] Core tests in `crates/micold-core/src/occupancy.rs` (`#[cfg(test)]`): `refusal_text` and `start_block_text` end with the way forward (tool wording, contracts/mcp-tools.md); `default_session` returns `last_viewed` when it names an un-archived session of the location, else the first un-archived in list order, else `None`, ignores other locations and archived sessions (R13).
- [x] T039 [P] [US1] Daemon tests `soft_limit_*` in `crates/micold-daemon/tests/one_session_per_worktree.rs`: `SessionCreate` with the flag on an occupied worktree and on the Default root adds a second session and leaves the first byte-identical; without it still refused (M1 tests unchanged); the flag on a free worktree creates its first; 100 concurrent creates without the flag, one winner (SC-003); the `SessionStart`/`SessionRestart`/`AttachItem::Session` fields are only carried (their behaviour is M7's T031/T033 and M6's T024).
- [x] T040 [P] [US1] Tool tests in `crates/micold-daemon/tests/mcp_create_session.rs` and `crates/micold-core/tests/mcp_tools_catalog.rs`: `create_session` schema holds optional boolean `allow_additional_session`; the flag creates beside the existing one (the `start_session` flag is M7's T032-T033); a non-boolean value is an input error; the refusal message names the flag; policy for `CreateSession` still applies with the flag set; descriptions state the soft limit.
- [x] T041 [US1] Implement: `Additional` and `default_session` and the amended texts in `crates/micold-core/src/occupancy.rs`; the four wire fields and `PROTOCOL_VERSION` 42 with history entry and `SCHEMA_HASH` in `crates/micold-core/src/protocol/messages.rs` and `version.rs`; `Catalog::create_session(.., Additional)` in `crates/micold-daemon/src/catalog.rs` (all existing callers `No`), the mapping in `crates/micold-daemon/src/server.rs` and `crates/micold-daemon/src/mcp/tools.rs`, schemas and descriptions in `crates/micold-core/src/mcp/tools.rs` (`create_session` only). The start block and the `start_session` flag are M7's (nothing in main blocks a start yet).
- [x] T042 [P] [US1] Amend `docs/user-guide/agent-tools.md` and `docs/user-guide/worktrees-and-sessions.md`: the limit is soft, the refusal's wording, `allow_additional_session` on `create_session` (FR-014, Constitution VII).

**Checkpoint**: `mise run gate` green; the default path unchanged, the flag adds.

## Phase 3c: Main session (US7 amendment; first in M3)

**Goal**: the daemon designates, persists, moves and promotes the main session; the wire carries it. No UI.

**Independent test**: `cargo test -p micold-daemon --test one_session_per_worktree main_session` and `mise run test-core`.

- [ ] T050 [P] [US7] Core and wire tests: in `crates/micold-core/src/occupancy.rs` (`#[cfg(test)]`) `main_session` returns the un-archived session with `main`, else the first un-archived in list order, else `None`, ignores other locations and archived sessions; `occupancy` names it; `refusal_text`/`start_block_text` say "main session"; remove the `default_session`/`last_viewed` tests of T038. In `crates/micold-core/tests/protocol_roundtrip.rs`: `Session.main` round-trips and a record without it decodes `false`; `ClientMsg::SessionSetMain { id }` round-trips; `PROTOCOL_VERSION == 43`.
- [ ] T051 [P] [US7] Daemon tests `main_session_*` in `crates/micold-daemon/tests/one_session_per_worktree.rs` (US7 s1-s5, FR-020-FR-022, SC-008): create on a free worktree (with and without `allow_additional_session`) is main; `allow_additional_session` on an occupied one is a helper and the main is unchanged; `SessionSetMain` on a helper moves the role in one write (exactly one main), persists across a daemon restart, and is a no-op for the main, an archived or an unknown id; archive of the main promotes the first remaining un-archived in list order in the same write, archive of a helper changes nothing, archive of the only session frees the location and the next create is main; a catalog fixture without the marker derives its main as the first un-archived and no record is rewritten on load; the refusal of M1/M2 names the main.
- [ ] T052 [US7] Implement in `crates/micold-core`: `Session.main` (serde-default) in `src/session.rs`; `occupancy::main_session` replacing `default_session` and the naming/wording in `src/occupancy.rs`; `ClientMsg::SessionSetMain`, `PROTOCOL_VERSION` 43 with history entry and `SCHEMA_HASH` in `src/protocol/messages.rs` and `version.rs`.
- [ ] T053 [US7] Implement in `crates/micold-daemon`: `main` set by `Catalog::create_session` and inherited by `Catalog::replace_session` (`src/catalog.rs`), promotion in the archive path in the same persisted write, `Catalog::set_main`; adoption sets it for the one catalogued session per location (`src/state.rs`); map `SessionSetMain` in `src/server.rs` (fire-and-forget; the catalog update is the answer). Keep the daemon and client compiling: the client ignores the field until M4.
- [ ] T054 [P] [US7] Amend `docs/user-guide/agent-tools.md` (the refusal names the main session; `list_sessions` rows do not mark it and listing order does not indicate it) and `docs/user-guide/worktrees-and-sessions.md` (main and helper sessions, set at start, how it is moved and promoted on delete) (FR-014, Constitution VII).

**Checkpoint**: `mise run gate` green; the daemon designates and moves the main; no UI change.

## Phase 4: User Story 2 - The session is defined when the worktree starts (P1)

**Goal**: worktree and session are created together with the chosen CLI and first prompt; the sidebar offers a start only on a location without a session.

**Independent test**: `create_worktree` with `ai_cli` + `prompt` yields one defined session; the dialog does the same; no add action on an occupied row.

- [ ] T014 [P] [US2] Daemon tests in `crates/micold-daemon/tests/mcp_create_worktree.rs` and `crates/micold-daemon/tests/one_session_per_worktree.rs`: `create_worktree` with `ai_cli`/`prompt` (and `prompt` only = default CLI) returns `session`, `lifecycle`, `prompt_delivered`; no definition = byte-identical old reply and `create_session` then works (US2 s5); missing CLI leaves no worktree and no session (US2 s4); worktree created but session failed names the worktree and leaves it without a session; `WorktreeCreate` RPC with `session: Some` does the same; a Default-session caller of `create_worktree` with and without a definition creates the worktree and, with one, runs the new session in the new worktree, never in Default (Constitution III exception, research R6).
- [ ] T015 [P] [US2] Tool-schema tests in `crates/micold-core/tests/mcp_tools_catalog.rs`: `create_worktree` gains optional `ai_cli` (enum of `AiCli::ALL` tool names) and `prompt`.
- [ ] T016 [US2] Implement `ops::create_worktree_with_session` (CLI check at the future cwd, create, then session via the same locked create path) in `crates/micold-daemon/src/ops.rs`; wire it from `WorktreeCreate` in `crates/micold-daemon/src/server.rs` and from `create_worktree` in `crates/micold-daemon/src/mcp/tools.rs` (policy as `CreateWorktree` and `CreateSession`); update `crates/micold-core/src/mcp/tools.rs` schema and descriptions.
- [ ] T017 [P] [US2] Client reducer tests in `crates/micold-client/tests/features_worktree_form.rs`: dialog has an AI CLI choice (default CLI preselected, "None" last) and a first-prompt field; confirm sends `session: Some(definition)` or `None`; a prompt with "None" is not representable.
- [ ] T018 [P] [US2] Client tests in `crates/micold-client/tests/features_session.rs`: `start_intent` offers a start only for a `Free` location (`occupancy`), none for an occupied one, Default included (additional sessions are US6's menu entry, not this action); `WorktreeCreated.session` is selected and viewed as `SessionCreated` is.
- [ ] T019 [US2] Add the fields to `crates/micold-client/src/features/worktree_form.rs` and render them in `crates/micold-client/src/ui/worktree_form.rs` (existing select and text-field primitives only, Principle VIII); send the definition and select the returned session in `crates/micold-client/src/shell/daemon_sync.rs`.
- [ ] T020 [US2] Make `start_intent` in `crates/micold-client/src/features/session.rs` ask `occupancy`, and have `crates/micold-client/src/ui/sidebar.rs` render exactly one start action for a free location and none for an occupied one (FR-004).
- [ ] T021 [P] [US2] Update `docs/user-guide/agent-tools.md` (`create_worktree` inputs `ai_cli` and `prompt`, reply fields).
- [ ] T036 [P] [US2] Update `docs/user-guide/worktrees-and-sessions.md` (dialog fields, definition at start, no add action on a started worktree).
- [ ] T022 [US2] Visual pass of quickstart Part B items 1, 2 and 4 via the `visual-pass` skill; save evidence in `specs/776-one-session-per-worktree/tdd/`.

## Phase 4b: User Story 5 - Worktree rows and the main session (P2)

**Goal**: one row per worktree, no session rows by default, selecting a row opens its main session.

**Independent test**: `cargo test -p micold-client --test features_sidebar --test features_session`.

- [ ] T043 [P] [US5] Client reducer tests in `crates/micold-client/tests/features_sidebar.rs` and `features_session.rs`: a worktree with one session builds no session rows and has no expander; with two it has an expander, collapsed by default, listing the main first and the helper marked *(amended)*; a row press selects `main_session` whatever was last viewed *(amended)*; a worktree with no session selects nothing and keeps its start action; the Default root follows the same; unread and activity of a collapsed row aggregate as before (feature 575).
- [ ] T044 [US5] Implement in `crates/micold-client/src/features/sidebar.rs` (expander only for several, default collapsed), `crates/micold-client/src/features/session.rs` (row-press selection via `occupancy::main_session`; no `last_viewed`) *(amended)* and `crates/micold-client/src/ui/sidebar.rs` (no sub-rows for one session, helpers marked, press emits the selection; rename and close of the single session in the worktree row's existing context menu), existing primitives only (Principle VIII).
- [ ] T045 [P] [US5] Update `docs/user-guide/worktrees-and-sessions.md`: the sidebar lists worktrees, selecting one opens its main session, several sessions expand with the helpers marked.

## Phase 4c: User Stories 6 and 7 - A helper on purpose; changing the main (P2)

**Goal**: the app's explicit request (row menu entry and confirmation) adds a helper; a helper can be made main; deleting a main names its successor.

**Independent test**: `cargo test -p micold-client --test features_session additional`.

- [ ] T046 [P] [US6] Client tests in `crates/micold-client/tests/features_session.rs`: the worktree row menu has "Start another session..." only for an occupied location (Default included); it opens a confirmation naming the worktree and the existing session; confirm sends `SessionCreate { allow_additional_session: true }` and the created session is selected; cancel changes nothing; the confirmation survives a refusal arriving first (US6 s4).
- [ ] T047 [US6] Add the menu entry and the confirmation state in `crates/micold-client/src/features/session.rs` and `crates/micold-client/src/features/worktree.rs`, and `crates/micold-client/src/ui/confirm_additional_session.rs` (pattern of `ui/confirm_session_remove.rs`) rendered from `crates/micold-client/src/ui/mod.rs` / the sidebar menu; send in `crates/micold-client/src/shell/daemon_sync.rs`.
- [ ] T048 [P] [US6] Update `docs/user-guide/worktrees-and-sessions.md`: "Start another session..." and its confirmation.
- [ ] T055 [P] [US7] Client tests in `crates/micold-client/tests/features_session.rs` and `features_sidebar.rs`: a helper sub-row menu has "Make main session" and the main's does not; choosing it sends `SessionSetMain` and, on the catalog update, the row press opens the new main and the old is listed as a helper; the delete confirmation of a main that has helpers names `main_session` of the remaining sessions, and of a helper or a lone main does not (FR-021, FR-022).
- [ ] T056 [US7] Implement the menu entry and the delete-confirmation wording in `crates/micold-client/src/features/session.rs`, `crates/micold-client/src/ui/sidebar.rs` and the existing confirmation view; send in `crates/micold-client/src/shell/daemon_sync.rs`.
- [ ] T057 [P] [US7] Update `docs/user-guide/worktrees-and-sessions.md`: "Make main session" and the promotion on delete.
- [ ] T049 [US6] Visual pass of quickstart Part B items 5 and 6 via `visual-pass`; evidence in `specs/776-one-session-per-worktree/tdd/`.

## Phase 5: User Story 3 - Resume and restart replace, never add (P2)

**Goal**: restart/start keep the same session; a confirmed provider resume replaces the occupant; "attach all" catalogues one session per location.

**Independent test**: `cargo test -p micold-daemon --test attach_apply --test one_session_per_worktree`.

- [ ] T023 [P] [US3] Daemon tests in `crates/micold-daemon/tests/one_session_per_worktree.rs`: stop, `start_session`, restart keep the same id and one session (FR-008a, FR-009 pinned); `start_session` on a running session is still a no-op success.
- [ ] T024 [P] [US3] Daemon tests in `crates/micold-daemon/tests/attach_apply.rs` *(amended)*: `AttachItem::Session` with both `replace` and `allow_additional_session` is `Refused` (malformed); resume with `allow_additional_session: true` into an occupied location adds the resumed session beside the existing one, stops nothing and archives nothing; a request into a free location ignores the flag; resume into an occupied location without `replace` is `Refused(LocationHasSession)` with `existing`, nothing stopped or written; with `replace: true` the old process is stopped first, the new record persists, then the old is archived (R7), a failed write leaves the old listed; the replacing record inherits `main` from the replaced one and a kept-beside record is a helper *(amended 2026-10-14)*; a batch of several sessions for one location keeps the most recent (`cap_per_location`), the rest stay resumable; a single explicit resume is never capped (FR-012).
- [ ] T025 [P] [US3] Core tests in `crates/micold-core/tests/attach_discovery.rs`: `cap_per_location` keeps the greatest `last_activity` per `AttachTarget` and drops every candidate for an occupied one; `catalog_adoption.rs` in `crates/micold-daemon/tests/` covers `adopt_discovered_sessions` catalogue at most one per location.
- [ ] T026 [US3] Implement `cap_per_location` in `crates/micold-core/src/attach.rs`; `Catalog::replace_session` and the adoption cap in `crates/micold-daemon/src/catalog.rs`; `attach_apply` replace, keep-both (`allow_additional_session`) and cap in `crates/micold-daemon/src/state.rs` *(amended)*.
- [ ] T027 [P] [US3] Client tests in `crates/micold-client/tests/features_attach.rs` and `attach_dialog.rs`: a `Refused(LocationHasSession)` result opens a confirmation naming both sessions with Replace and Keep both; Replace re-sends the item with `replace: true`, Keep both with `allow_additional_session: true`; cancel changes nothing *(amended)*.
- [ ] T028 [US3] Add the confirm step (replace or keep both, *amended*) in `crates/micold-client/src/features/attach.rs` and render it in `crates/micold-client/src/ui/attach_dialog.rs`.
- [ ] T029 [P] [US3] Document resume-replaces and its confirmation in `docs/user-guide/worktrees-and-sessions.md`.
- [ ] T030 [US3] Visual pass of quickstart Part B item 3 (confirmation naming both sessions, cancel) via `visual-pass`; evidence in `specs/776-one-session-per-worktree/tdd/`.

## Phase 6: User Story 4 - Worktrees that hold several sessions (P3)

**Goal**: a catalog with several sessions in a worktree opens intact; no extra session is created and no second is started beside a live one without an explicit request (*amended*).

**Independent test**: `cargo test -p micold-daemon --test one_session_per_worktree legacy`.

- [ ] T031 [P] [US4] Daemon tests in `crates/micold-daemon/tests/one_session_per_worktree.rs`: a catalog fixture with two sessions in one worktree loads with both listed and byte-identical records (SC-005); `create_session` refused; start of a stopped session while the other runs is a no-op (nothing spawned, no record rewritten) and names the live one (FR-013), and runs with `allow_additional_session` through `SessionStart` and `start_session` (*amended*); delete of an extra frees the path; sequence test over create/start/stop/restart/resume/delete from two connections on a fresh catalog never shows two un-archived sessions (SC-001).
- [ ] T032 [P] [US4] Client tests in `crates/micold-client/tests/features_session.rs`: starting the blocked row views it without starting, shows `start_block_text` as tooltip and offers the T047 confirmation naming the live session; confirm re-sends the start with `allow_additional_session: true` (*amended*); add to `crates/micold-core/tests/mcp_tools_catalog.rs` that the `start_session` description states the rule and its refusal (FR-014).
- [ ] T033 [US4] Implement (*amended*: the check is general and skipped on an explicit request; `start_session` and `SessionStart`/`SessionRestart` take `allow_additional_session`; `start_session` schema and description in `crates/micold-core/src/mcp/tools.rs`; tool tests in `crates/micold-daemon/tests/mcp_create_session.rs`) `DaemonState::start_refusal` and the in-lock check in `start_session_gated` in `crates/micold-daemon/src/state.rs` (including the skip), applied by `SessionStart`/`SessionRestart` in `crates/micold-daemon/src/server.rs` and by `start_session` in `crates/micold-daemon/src/mcp/tools.rs` (`Conflict`).
- [ ] T034 [US4] Ask `occupancy::start_block` in `crates/micold-client/src/features/session.rs` and render the tooltip in `crates/micold-client/src/ui/sidebar.rs`.
- [ ] T035 [P] [US4] Document legacy worktrees (extras kept, refusal while one runs, delete extras) in `docs/user-guide/worktrees-and-sessions.md` and the `start_session` error in `docs/user-guide/agent-tools.md`.

## Dependencies

- US7's daemon half (T050-T054) needs T041 and is needed by US5 (T044 reads `main_session`), US6 (the new session is a helper) and US3 (replace inherits `main`). T055-T056 need T044 and T047.
- Phase 2 blocks all stories. T002/T003 before T004/T005.
- US1 (T006-T013) needs Phase 2. The soft limit (T037-T042) needs US1 and is needed by every later story: it changes `Catalog::create_session`'s signature and the wire. US2 needs T041. US5 needs T052 (`main_session`). US6 needs T041, T053 and US5's menu. US3 needs T041 and US1. US4 needs T041, `start_block` (T004) and US6's confirmation (T047).
- Within a story: tests, then implementation, then docs.
- Files shared across tasks (`one_session_per_worktree.rs`, `state.rs`, `session.rs`, the user-guide pages) are edited in order of task ID, not in parallel.

## Implementation Strategy

MVP: Phases 1-3 (rule, wire, refusal everywhere a session is created), merged. Then the soft limit (3b), the main session's daemon half with US2 (3c, M3), the client stories US5, US6 and US7, US3, US4, each its own PR. Quickstart Part A is the automated pass; Part B items are tasks T022, T030 and T049.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot). M1 is merged (#794). M2-M7 are the amended cut (2026-10-13); the old M2-M5 were renumbered M3, M4, M6, M7 and M5 is new. Amended 2026-10-14: M3 gains the main-session daemon half (T050-T054) and M5 gains the client half (T055-T057); the cut and order are unchanged.

### M1 — The rule and the refusal 🎯 MVP (merged, #794)

- **Tasks**: T001–T013
- **Deliverable**: A second session in a worktree or the project root is refused by the daemon (RPC, `create_session`, review send), naming the existing session, and the wire carries the optional fields for the rest of the feature.
- **Satisfies**: US1 acceptance scenarios 1–4 (as then written); FR-001, FR-002, FR-003, FR-005, FR-011, FR-014 (refusal part); SC-002, SC-003
- **Verify**: `cargo test -p micold-daemon --test one_session_per_worktree` and `mise run test-core`
- **Depends on**: —
- **Tier**: full

### M2 — Soft limit: an explicit request adds a session (daemon, wire, tools)

- **Tasks**: T037–T042
- **Deliverable**: `create_session` accepts `allow_additional_session: true` and then creates a session beside the existing one; the refusal names the flag; without it nothing changes; wire v42 carries the field on the start and attach messages for M6 and M7.
- **Satisfies**: US1 acceptance scenarios 4–6; FR-001, FR-002, FR-003, FR-005, FR-015 (`create_session`, wire), FR-014 (tools); SC-001–SC-003
- **Verify**: `cargo test -p micold-daemon --test one_session_per_worktree --test mcp_create_session` and `mise run test-core`
- **Depends on**: M1
- **Tier**: full

### M3 — Define the session at worktree start; the main session (daemon, wire, tools)

- **Tasks**: T014–T016, T021, T050–T054
- **Deliverable**: `create_worktree` (MCP) and the `WorktreeCreate` RPC accept an AI CLI and first prompt and create the worktree and its one session together, which is its main session; a missing CLI creates nothing. The daemon persists which session is main, moves it on `SessionSetMain`, promotes the first remaining session when the main is deleted, and names it in refusals (wire v43). (WIP for T014–T016 kept at local tag `wip-776-m3`, cherry-picked after M2 merges and adapted to M2's `Additional::No` signature; T050–T054 are written on top.)
- **Satisfies**: US2 acceptance scenarios 2, 4, 5; US7 acceptance scenarios 1–5 (daemon half); FR-006, FR-010, FR-014 (`create_worktree`, refusal), FR-020, FR-022; SC-008
- **Verify**: `cargo test -p micold-daemon --test mcp_create_worktree --test one_session_per_worktree` and `mise run test-core`
- **Depends on**: M2
- **Tier**: full

### M4 — Dialog fields, worktree rows and the main session

- **Tasks**: T017–T020, T022, T036, T043–T045
- **Deliverable**: The create-worktree dialog starts the session with the worktree; the sidebar lists worktrees only (an expander only for several sessions, collapsed, the main first and helpers marked), a row press opens the main session, and a row with a session has no start action.
- **Satisfies**: US2 acceptance scenarios 1, 3; US5 acceptance scenarios 1–5; FR-004, FR-006, FR-017, FR-018, FR-019, FR-014 (dialog, sidebar); SC-004, SC-006
- **Verify**: `cargo test -p micold-client --test features_worktree_form --test features_session --test features_sidebar` and quickstart §B items 1, 2, 4
- **Depends on**: M3
- **Tier**: full

### M5 — Start a helper on purpose; make a session main (app)

- **Tasks**: T046–T049, T055–T057
- **Deliverable**: The worktree row menu offers "Start another session..." on an occupied worktree; confirming adds a helper session and shows it. A helper's menu offers "Make main session"; deleting a main names the session that takes over.
- **Satisfies**: US6 acceptance scenarios 1–4; US7 acceptance scenarios 2–3 (app half); FR-016 (menu entry), FR-021, FR-022 (confirmation); SC-007, SC-008
- **Verify**: `cargo test -p micold-client --test features_session --test features_sidebar additional main` and quickstart §B items 5, 6
- **Depends on**: M2, M4
- **Tier**: full

### M6 — Resume replaces, or keeps both

- **Tasks**: T023–T030
- **Deliverable**: Resuming a provider session into an occupied worktree asks for confirmation and replaces the occupant (the resumed session becomes main) or keeps both (the resumed session is a helper); "attach all" catalogues one session per location; restart and start keep one session.
- **Satisfies**: US3 acceptance scenarios 1–4; FR-007, FR-008a, FR-008b, FR-008c, FR-009, FR-012, FR-015 (`AttachItem::Session`), FR-014 (resume)
- **Verify**: `cargo test -p micold-daemon --test attach_apply --test one_session_per_worktree` and quickstart §B item 3
- **Depends on**: M2, M3 (main role)
- **Tier**: full

### M7 — Worktrees with several sessions stay intact

- **Tasks**: T031–T035
- **Deliverable**: A catalog with several sessions in a worktree opens untouched, refuses new sessions and a start beside a live session unless the user confirms or `start_session` passes `allow_additional_session`, naming the live one. Builds the start block (it does not exist on main) with the flag's skip.
- **Satisfies**: US4 acceptance scenarios 1–3; FR-013, FR-015 (`start_session`), FR-016 (blocked start), FR-014 (`start_session`); SC-001, SC-005
- **Verify**: `cargo test -p micold-daemon --test one_session_per_worktree` and `cargo test -p micold-client --test features_session`
- **Depends on**: M2, M5
- **Tier**: full
