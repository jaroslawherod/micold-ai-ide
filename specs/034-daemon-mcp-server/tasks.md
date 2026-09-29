---
description: "Task list for feature 034 — the session service exposes an MCP server to the AI sessions it runs"
---

# Tasks: The session service exposes an MCP server to the AI sessions it runs

**Input**: Design documents from `/specs/034-daemon-mcp-server/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/binding.md](./contracts/binding.md),
[contracts/mcp-tools.md](./contracts/mcp-tools.md),
[contracts/protocol-delta.md](./contracts/protocol-delta.md), [quickstart.md](./quickstart.md)

**Tests**: Per Constitution Principle I (NON-NEGOTIABLE), test tasks are MANDATORY and come before
the implementation they cover; each must be observed failing first. The only glue under the
GUI/process-spawn exception is the rendering in `crates/micold-client/src/ui/` (dialog, settings
rows), validated by the geometry gates and quickstart §B4/§B6 with the `visual-pass` skill.

**Documentation**: Per Constitution Principle VII, each milestone updates the user guide for what it
ships (FR-019); the page is `docs/user-guide/agent-tools.md`.

**Cross-platform**: Per Constitution Principle VI, the only platform difference is the owner-only
binding file (`crates/micold-daemon/src/platform/{unix,windows}.rs`); any milestone that touches a
`cfg` arm runs `scripts/build-lock.sh cargo check --workspace --target aarch64-apple-darwin` before
pushing.

**Protocol**: each milestone that changes the wire bumps `PROTOCOL_VERSION` once, documents it in
`crates/micold-core/src/protocol/version.rs` and updates the pin in
`crates/micold-core/tests/schema_hash.rs` ([protocol-delta.md](./contracts/protocol-delta.md)).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies on incomplete tasks)
- **[Story]**: Which user story the task belongs to (US1–US4)
- Every description carries an exact file path

## Path Conventions

Three-crate Rust workspace: `crates/micold-core/`, `crates/micold-daemon/`,
`crates/micold-client/`, plus `docs/`. Build and test through `mise run <task>`, never bare `cargo`.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: module skeletons and test support only. No behaviour.

- [ ] T001 Create the empty module trees `crates/micold-core/src/mcp/mod.rs` (declared in `crates/micold-core/src/lib.rs`) and `crates/micold-daemon/src/mcp/mod.rs` (declared in `crates/micold-daemon/src/lib.rs`), each with a module doc naming feature 034 and its contracts
- [ ] T002 [P] Add a daemon test helper `crates/micold-daemon/tests/support/mcp.rs` (re-exported from `tests/support/mod.rs`): build a `DaemonState` over a temp git repository with named worktrees and sessions (pattern of `hooks_receiver.rs` `state_with_session`), and `post_mcp(addr, bearer, json) -> (status, body)` over a raw `TcpStream`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: the HTTP code shared with the hook receiver, the JSON-RPC layer, the listener and the
credential registry. Everything here ships in M1.

### Tests (write first, confirm they fail)

- [ ] T003 [P] Move the head-parsing, bounds and drain unit tests of `crates/micold-daemon/src/hooks.rs` into `crates/micold-daemon/src/http.rs`'s test module against the new API (`parse_head`, `find_head_end`, bounded read with a caller-chosen body limit), and confirm `crates/micold-daemon/tests/hooks_receiver.rs` is the unchanged regression for the hook receiver
- [ ] T004 [P] Write `crates/micold-core/tests/mcp_jsonrpc.rs`: parse request / notification / malformed JSON (`-32700`, id null) / unknown method (`-32601`); `initialize` answers the client's `protocolVersion` when it is one of `2025-03-26`, `2025-06-18`, `2025-11-25`, `2026-07-28`, else `2026-07-28`, with `capabilities.tools.listChanged = false` and `serverInfo.name = "micold"`; `ping` → `{}`; tool results serialise as mcp-tools.md §Results (success and `isError` shapes, category snake_case)
- [ ] T005 [P] Write `crates/micold-daemon/tests/mcp_endpoint.rs`: listener binds `127.0.0.1` only; missing / malformed / unknown bearer each answer `401` with an identical empty body (SC-006); `GET` and `DELETE /mcp` → `405`; other paths → `404`; head over 8 KiB → `431`, body over 1 MiB → `413`; `notifications/initialized` → `202`; a credential stops working after `delete_session` and a fresh `DaemonState` (service restart) accepts none of the old ones (FR-006)
- [ ] T006 [P] Write `crates/micold-daemon/tests/mcp_binding_file_mode.rs`: `platform::write_owner_only(dir, file, bytes)` creates the directory `0700` and the file `0600` on Unix (mode bits read back), and on Windows a protected DACL whose only ACE grants the current user's SID (read back with `GetNamedSecurityInfoW`) (FR-007, SC-009)

### Implementation

- [ ] T007 Extract bounded HTTP/1.1 handling from `crates/micold-daemon/src/hooks.rs` into `crates/micold-daemon/src/http.rs` (`Head`, `parse_head`, `find_head_end`, `respond`, `drain`, read-with-limit); `hooks.rs` calls it with its existing `MAX_BODY` (4 MiB) and behaviour is unchanged
- [ ] T008 Implement `crates/micold-core/src/mcp/jsonrpc.rs` and `crates/micold-core/src/mcp/errors.rs` (`ErrorCategory { NotFound, InvalidInput, Conflict, RefusedByPolicy, NeedsConfirmation, ServiceError }`, `OpError { category, message }`, result builders) to pass T004
- [ ] T009 Implement `platform::write_owner_only` in `crates/micold-daemon/src/platform/{mod,unix,windows}.rs` (Unix: `DirBuilder` `mode(0o700)`, `OpenOptions` `mode(0o600)`; Windows: SDDL `D:P(A;;GA;;;<sid>)` as `crates/micold-daemon/src/singleton.rs` builds it) to pass T006
- [ ] T010 Implement `crates/micold-daemon/src/mcp/credentials.rs` (TM2: `Uuid::new_v4().simple()` credential per session, idempotent while the session exists, `revoke(session)`, lookup by whole-string match) and `crates/micold-daemon/src/mcp/server.rs` (bind `127.0.0.1:0`, accept loop, 8 KiB head / 1 MiB body via `http.rs`, bearer check → `401`, JSON-RPC dispatch, body and credential never logged); hold it on `DaemonState` in a `OnceLock` like the hook receiver, bind it in `crates/micold-daemon/src/server.rs` `run` beside `HookReceiver::bind`, and revoke on `DaemonState::delete_session` in `crates/micold-daemon/src/state.rs` — to pass T005

---

## Phase 3: User Story 1 — The agent sees the project's sessions and worktrees without any setup (Priority: P1) 🎯 MVP

**Goal**: every Claude Code (and Copilot) session is bound at spawn and its agent can call `whoami`,
`list_worktrees`, `list_branches`, `list_sessions`, `get_session`; a toggle turns binding off.

**Independent Test**: quickstart §B1–§B2; `mcp_read_tools.rs` and `mcp_binding_spawn.rs`.

### Tests for User Story 1 (write first, confirm they fail)

- [ ] T011 [P] [US1] Write `crates/micold-core/tests/mcp_binding_plan.rs`: for `ToolServerSupport::McpConfigArg` the plan appends `--mcp-config <file> --allowedTools mcp__micold` **last** (no positional argument after them) and the file JSON is exactly contracts/binding.md §4's Claude entry (`type` `http`, url, `Authorization: Bearer <cred>`, `timeout` `120000`); `AdditionalMcpConfig` gives `--additional-mcp-config @<file> --allow-tool micold` with `"tools":["*"]`; `Unsupported` gives no args; `--strict-mcp-config` never appears
- [ ] T012 [P] [US1] Write `crates/micold-core/tests/mcp_name_collision.rs` over fixture files: a `mcpServers.micold` entry at top level or under the project path in `~/.claude.json`, in `$CLAUDE_CONFIG_DIR/.claude.json` when that variable is set, in `<cwd>/.mcp.json`, or in the Copilot `mcp-config.json` is detected; an absent, unreadable or malformed file is "not taken" (research R14)
- [ ] T013 [P] [US1] Extend `crates/micold-core/tests/ai_cli_provider_seam.rs`: every `AiCli::ALL` member answers `tool_server_support()` — Claude `McpConfigArg`, Copilot `AdditionalMcpConfig`, Pi `Unsupported { reason: "Pi has no MCP support" }` — with no `match` on the CLI outside `crates/micold-core/src/provider.rs` (`crates/micold-client/tests/no_concrete_implementations.rs` stays green)
- [ ] T014 [P] [US1] Write `crates/micold-core/tests/mcp_tools_catalog.rs`: `tools/list` names exactly the tools shipped so far with JSON-Schema `inputSchema`s and `readOnlyHint` set on read-only ones; argument validation for `list_worktrees {include_hidden?: bool}`, `list_sessions {worktree?}`, `get_session {session}` (bad UUID / wrong type → `invalid_input`); `WorktreeRef` parses `default` and directory names
- [ ] T015 [P] [US1] Write `crates/micold-daemon/tests/mcp_read_tools.rs` (US1 s2, s3; FR-010; edge cases *Empty project*, *Unknown target*, *Hidden assistant-owned worktrees*): `whoami` names the caller, project and hosting worktree; `list_worktrees` returns `default` then the sidebar's set with branch, status (`clean|missing|locked|prunable`), `app_created`, `assistant_owned`, `session_count`, and hidden rows only with `include_hidden`; `list_sessions` marks `is_caller`, filters by worktree; `get_session` adds `failure_reason` for `Failed`; `list_branches` reports `checked_out_in`; a session or worktree of another project, or an unknown one, is `not_found` with the same message
- [ ] T016 [P] [US1] Write `crates/micold-daemon/tests/mcp_binding_spawn.rs` (US1 s1, s4, s5; FR-002, FR-003, FR-005; SC-002): spawning a Claude session with a stand-in CLI (pattern of `session_identity_env.rs`) passes the binding args last and writes `<data_dir>/mcp/<uuid>.json` owner-only; 20 spawns leave hashed fixture `~/.claude.json`, `~/.claude/settings.json`, `.mcp.json` and `~/.copilot/mcp-config.json` byte-identical; a Pi session spawns with its exact pre-feature argv and one `info` line "no tool server: Pi has no MCP support"; a name collision spawns unbound with the collision logged; a respawn reuses the credential; a Regular-terminal session gets no binding and no log line

### Implementation for User Story 1

- [ ] T017 [US1] Add `ToolServerSupport` and `AiCliProvider::tool_server_support()` to `crates/micold-core/src/provider.rs` (Claude, Copilot, Pi impls) to pass T013
- [ ] T018 [US1] Implement `crates/micold-core/src/mcp/binding.rs` (`BindingPlan`, `BindingOutcome::{Bound, Skipped(SkipReason)}` with `SkipReason ∈ { Disabled, Unsupported(reason), NameTaken(path), ServerUnavailable, WriteFailed(text) }`, config JSON per CLI, collision check) to pass T011, T012
- [ ] T019 [US1] Implement `crates/micold-core/src/mcp/tools.rs` read-only part (tool catalog entries, `Operation` variants and validation for `whoami`, `list_worktrees`, `list_branches`, `list_sessions`, `get_session`; `WorktreeRef`, `SessionRef`) to pass T014
- [ ] T020 [US1] Wire the binding into the spawn path in `crates/micold-daemon/src/state.rs` (`tool_server_launch_for(id, &spec)` beside `activity_launch_for`, appended after every other argument in `start_session`; writes the file through `platform::write_owner_only` under `<data_dir>/mcp/`; logs exactly one `SkipReason` line per unbound AI-CLI spawn) to pass T016
- [ ] T021 [US1] Implement the read-only tool handlers in `crates/micold-daemon/src/mcp/tools.rs` (resolve `Caller` from the credential; scope every target to the caller's project; answer from `catalog_snapshot()` and the worktree cache, no git subprocess except `list_branches` via `worktree::branch_candidates` off the lock) to pass T015
- [ ] T022 [US1] Create `docs/user-guide/agent-tools.md` (what the tool server is, the read tools, which CLIs are bound and why Pi is not, scope = own project, that no configuration file is changed, the collision rule) and add it to `docs/SUMMARY.md`; add a paragraph to `docs/user-guide/sandboxed-daemon.md` that sandboxed sessions are bound the same way (FR-019)

### FR-004 toggle for User Story 1 (US1 s6)

- [ ] T023 [P] [US1] Extend `crates/micold-core/tests/settings_roundtrip.rs` and `crates/micold-core/tests/protocol_roundtrip.rs`: `tool_server_enabled` defaults to `true`, an older settings file without it loads `true`, and `DaemonSettings` / `SettingsSet` carry it
- [ ] T024 [P] [US1] Extend `crates/micold-daemon/tests/mcp_binding_spawn.rs`: with `tool_server_enabled = false` a new session spawns unbound with "disabled in settings" logged, and a session already running keeps answering (FR-004, US1 s6)
- [ ] T025 [P] [US1] Extend `crates/micold-client/tests/features_settings.rs` and `crates/micold-client/tests/support/covered_states.rs`: the Environment settings draft carries `tool_server_enabled`, toggling it sends `SettingsSet { tool_server_enabled: Some(_) }`, and the Environment page with the new row is a registered covered state for `crates/micold-client/tests/layout_snapshot.rs`
- [ ] T026 [US1] Add `tool_server_enabled` (default `true`, `#[serde(default)]`) to `crates/micold-core/src/settings.rs`, `crates/micold-daemon/src/catalog.rs`, `DaemonSettings` and `ClientMsg::SettingsSet` in `crates/micold-core/src/protocol/messages.rs`; bump `PROTOCOL_VERSION` 15 → 16 (or the next free number) in `crates/micold-core/src/protocol/version.rs` with its changelog line and the `schema_hash.rs` pin; add `DaemonState::set_tool_server_enabled` and the `SettingsSet` arm in `crates/micold-daemon/src/server.rs`; gate T020's binding on it — to pass T023, T024
- [ ] T027 [US1] Add the toggle row "Let AI sessions manage worktrees and sessions" to `crates/micold-client/src/ui/settings/environment.rs` with its draft/message/handler in `crates/micold-client/src/features/settings.rs`, reusing the row components `pi_activity_component` uses — to pass T025
- [ ] T028 [US1] Document the toggle in `docs/user-guide/settings.md` (Environment section) and link it from `docs/user-guide/agent-tools.md`

**Checkpoint**: a new Claude Code session calls `whoami`, `list_worktrees`, `list_sessions` with no setup (quickstart §B1–§B2).

---

## Phase 4: User Story 2 — The agent creates a worktree and starts a session in it (Priority: P1)

**Goal**: `create_worktree` and `create_session` (with an optional first prompt) behave exactly as
the create-worktree dialog and the session menu.

**Independent Test**: quickstart §B3; `mcp_create_worktree.rs`, `mcp_create_session.rs`.

### Tests for User Story 2 (write first, confirm they fail)

- [ ] T029 [P] [US2] Write `crates/micold-daemon/tests/ops_extraction.rs`: `ops::create_worktree`, `ops::delete_worktree` and `ops::rename_worktree` called without a client produce the same catalog, provenance record and on-disk result as the `WorktreeCreate` / `WorktreeDelete` / `WorktreeRename` messages do today; the existing `mutation_semantics.rs`, `mutation_atomicity.rs` and `worktree_provenance_rpc.rs` stay green unchanged
- [ ] T030 [P] [US2] Extend `crates/micold-core/tests/mcp_policy.rs` (new file): `policy::decide` refuses `create_worktree` from a Default caller with `refused_by_policy` naming Principle III (FR-015a), and proceeds for a worktree caller
- [ ] T031 [P] [US2] Extend `crates/micold-core/tests/mcp_tools_catalog.rs`: `create_worktree {branch, name?, mode?: new_branch|existing_local|track_remote, remote?}` (`track_remote` without `remote` → `invalid_input`; `overwrite` is not accepted) and `create_session {worktree, ai_cli?: claude_code|copilot|pi, prompt?}` validate as mcp-tools.md says
- [ ] T032 [P] [US2] Write `crates/micold-daemon/tests/mcp_create_worktree.rs` (US2 s1, s2; FR-009, FR-011; SC-003, SC-005; edge cases *Invalid names*, *Concurrency*): creates under `.claude/worktrees/<name>` with `name` defaulting to `naming::dir_name_from_branch(branch)`, records provenance (`app_created: true`), a registered fake client receives `CatalogChanged` within 2 s, and the result is the new `WorktreeRow`; an existing or checked-out branch fails `conflict` naming the `BranchSituation`, nothing on disk changes; an invalid name fails `invalid_input` with the dialog's `NamingError`/git message; 10 concurrent creates of one branch from 10 sessions → exactly 1 success, 9 `conflict`, one worktree on disk
- [ ] T033 [P] [US2] Write `crates/micold-core/tests/input_readiness.rs`: `AiCliProvider::input_readiness()` is Claude `HookSessionStart`, Pi `ExtensionEvent("session_start")`, Copilot `OutputSettled`; the pure output-settled rule in `crates/micold-core/src/mcp/submission.rs` reports ready only after first output followed by 1.5 s of none (fed with a fake clock); `encode_submission(text, bracketed)` wraps in `ESC[200~ … ESC[201~` when bracketed and always ends with `\r`
- [ ] T034 [P] [US2] Write `crates/micold-daemon/tests/mcp_create_session.rs` (US2 s3–s5; FR-017): `create_session` creates and starts a session in a worktree or `default`, visible to a fake client; omitting `ai_cli` uses `default_ai_cli`; a CLI not on the session environment's `PATH` fails `service_error` naming it and **no catalog record exists afterwards**; with `prompt`, a stand-in CLI receives the bracketed submission after the readiness signal (a `SessionStart` POST to the hook receiver for Claude) and the result says `prompt_delivered: true`; with no readiness within 60 s of the request, or a failed start, `prompt_delivered: false` and a later signal delivers nothing; the `SessionStart` hook leaves the activity FSM at `Unknown`

### Implementation for User Story 2

- [ ] T035 [US2] Create `crates/micold-daemon/src/ops.rs` by moving the bodies of the `WorktreeCreate`, `WorktreeDelete` and `WorktreeRename` arms of `route()` in `crates/micold-daemon/src/server.rs` into `ops::{create_worktree, delete_worktree, rename_worktree}` returning typed outcomes (create takes an `Option` progress sink for `OperationProgress`); `route()` wraps them and keeps its replies — to pass T029
- [ ] T036 [US2] Implement `crates/micold-core/src/mcp/policy.rs` `decide(&Caller, &Operation, &TargetFacts, CrossSessionAccess) -> PolicyDecision` with the FR-015a row for `create_worktree` to pass T030 (later milestones add rows)
- [ ] T037 [US2] Add `create_worktree` and `create_session` to `crates/micold-core/src/mcp/tools.rs` to pass T031
- [ ] T038 [US2] Implement the `create_worktree` handler in `crates/micold-daemon/src/mcp/tools.rs` (pre-flight via `worktree::preflight`, `CreateMode::is_compatible_with`, then `ops::create_worktree` under `worktree_gate`, `broadcast_catalog`) to pass T032
- [ ] T039 [US2] Add `InputReadiness` and `AiCliProvider::input_readiness()` to `crates/micold-core/src/provider.rs`, and the output-settled rule and `encode_submission` to `crates/micold-core/src/mcp/submission.rs`, to pass T033
- [ ] T040 [US2] Mark sessions ready for input in `crates/micold-daemon/src/state.rs` (`LiveSession.ready`, a `watch` per session): from `SessionStart` in `crates/micold-daemon/src/hooks.rs` (FSM unchanged), from `session_start` in the Pi tail (`crates/micold-daemon/src/activity.rs` `pi_event`, and add `session_start` to `EVENTS` in `crates/micold-daemon/assets/pi-activity.ts`), and from the output-settled rule on the primary PTY's output for `OutputSettled` providers, a declined Pi component, or Claude when the hook receiver is not running
- [ ] T041 [US2] Move `spawn_session_start` from `crates/micold-daemon/src/server.rs` into `crates/micold-daemon/src/ops.rs` (route() calls it there) and implement the `create_session` handler in `crates/micold-daemon/src/mcp/tools.rs` (availability check with `provider.is_available` against the env-include `PATH` before `DaemonState::create_session`; `begin_start` + `spawn_session_start` with no reply target; optional prompt wait bounded by 60 s from the request; write `encode_submission` to the primary PTY via `PtySession::write_input`, reading bracketed-paste mode from its `Term`) to pass T034
- [ ] T042 [US2] Document `create_worktree`, `create_session` and the first-prompt rule (ready signal per CLI, 60 s, Pi with the component declined) in `docs/user-guide/agent-tools.md`

**Checkpoint**: from a worktree session, an agent creates `feat-x` and a session in it that receives its first prompt (quickstart §B3).

---

## Phase 5: User Story 3 — The agent controls session lifecycle (Priority: P2)

**Goal**: start, stop, interrupt, rename and delete with the sidebar's safeguards, the FR-014
confirmation dialog, and the FR-015/FR-015a refusals.

**Independent Test**: quickstart §B4; `mcp_lifecycle_tools.rs`, `mcp_confirmations.rs`.

### Tests for User Story 3 — start and rename (write first, confirm they fail)

- [ ] T043 [P] [US3] Extend `crates/micold-core/tests/mcp_policy.rs`: `rename_worktree` and `delete_worktree` from a Default caller → `refused_by_policy` (FR-015a); `stop_session` / `delete_session` on self, and `delete_worktree` of the caller's hosting worktree → `refused_by_policy` (FR-015); `interrupt_session` on self → `invalid_input`; `stop_session`, `interrupt_session` on another session, `delete_session`, `delete_worktree` → `Confirm(op)`; `start_session`, `rename_worktree` from a worktree caller → `Proceed`; a refusal is decided before any confirmation
- [ ] T044 [P] [US3] Extend `crates/micold-core/tests/mcp_tools_catalog.rs` with `start_session`, `stop_session`, `interrupt_session`, `rename_worktree`, `delete_worktree {worktree, stop_sessions?=false, delete_branch?=true}`, `delete_session` (`default` as a rename/delete target → `invalid_input`; `destructiveHint` on the destructive ones)
- [ ] T045 [P] [US3] Write `crates/micold-daemon/tests/mcp_lifecycle_tools.rs` part 1 (US3 s1, s6; FR-012a): `start_session` on `Idle`, `Failed`, `InterruptedResumable` → `Starting` then `Running`; on `Starting`/`Running`/`Restarting` → success, unchanged, current lifecycle reported; `rename_worktree` updates the display name and broadcasts; Default-caller rename is refused

### Implementation for User Story 3 — start and rename

- [ ] T046 [US3] Add the T043 rows to `crates/micold-core/src/mcp/policy.rs` and the T044 tools to `crates/micold-core/src/mcp/tools.rs`, exposing in `tools/list` only the tools whose handlers exist in this milestone (`start_session`, `rename_worktree`)
- [ ] T047 [US3] Implement the `start_session` and `rename_worktree` handlers in `crates/micold-daemon/src/mcp/tools.rs` (`begin_start` + `spawn_session_start`; `ops::rename_worktree`) to pass T045
- [ ] T048 [US3] Document `start_session`, `rename_worktree` and the Default-session rule in `docs/user-guide/agent-tools.md`

### Tests for User Story 3 — confirmations and destructive operations (write first, confirm they fail)

- [ ] T049 [P] [US3] Extend `crates/micold-core/tests/protocol_roundtrip.rs`: `DaemonMsg::ConfirmationRequested { id, project, caller, caller_label, operation, target_label, expires_in_ms }`, `DaemonMsg::ConfirmationWithdrawn { id }` and `ClientMsg::ConfirmationAnswer { id, allow }` round-trip; `ConfirmOperation::SendInput` carries no text
- [ ] T050 [P] [US3] Write `crates/micold-daemon/tests/mcp_confirmations.rs` (FR-014; edge cases *Several windows*, *No window*, *Target changes while pending*): two fake clients both receive `ConfirmationRequested`; the first `ConfirmationAnswer` decides and both receive `ConfirmationWithdrawn`; a second answer is ignored; decline → `refused_by_policy` "declined by the user"; no answer in 60 s (paused tokio clock) → `needs_confirmation`; no client connected → `needs_confirmation` at once with no broadcast; deleting the target, or deleting/stopping the caller, while pending → `not_found` and withdrawn; closing the agent's HTTP connection while pending → abandoned, withdrawn, nothing changes; a client that connects while a prompt is pending receives it with the remaining time
- [ ] T051 [P] [US3] Write `crates/micold-daemon/tests/mcp_lifecycle_tools.rs` part 2 (US3 s2–s5; FR-009, FR-011): allowed `stop_session` ends the target's processes, marks it `Idle` in a `CatalogChanged` broadcast and leaves it resumable; `stop_session` on `Idle` → success unchanged; allowed `interrupt_session` writes `0x03` to the primary PTY and leaves it running; `delete_worktree` with live sessions and `stop_sessions: false` → `conflict` naming them, nothing changes; allowed `delete_worktree {stop_sessions: true}` stops them and removes it; allowed `delete_session` archives the record and revokes its credential; self-targets are refused before any prompt
- [ ] T052 [P] [US3] Write `crates/micold-client/tests/features_agent_confirm.rs`: `ConfirmationRequested` adds a pending prompt and opens the dialog; `ConfirmationWithdrawn` removes it; Allow/Deny send exactly one `ConfirmationAnswer` and close it; several pending prompts queue in arrival order; and register the open dialog as a covered state in `crates/micold-client/tests/support/covered_states.rs`

### Implementation for User Story 3 — confirmations and destructive operations

- [ ] T053 [US3] Add the three messages and `ConfirmOperation` to `crates/micold-core/src/protocol/messages.rs`; bump `PROTOCOL_VERSION` to the next free number with its changelog line in `crates/micold-core/src/protocol/version.rs` and the `schema_hash.rs` pin — to pass T049
- [ ] T054 [US3] Add `DaemonState::stop_session(id)` in `crates/micold-daemon/src/state.rs` (take the live entry, kill its process tree off the lock, set the record `Idle`, keep the credential, `broadcast_catalog`) and point the `SessionStop` arm of `route()` in `crates/micold-daemon/src/server.rs` at it
- [ ] T055 [US3] Implement the registry in `crates/micold-daemon/src/mcp/confirm.rs` (TM5: `ConfirmationId`, oneshot per request, 60 s deadline, `NoWindow` when no client, first answer wins, `TargetGone` on target/caller deletion or caller stop, `Abandoned` on HTTP connection close, `ConfirmationWithdrawn` on every exit, replay to a newly handshaken client) and the `ConfirmationAnswer` arm in `crates/micold-daemon/src/server.rs` — to pass T050
- [ ] T056 [US3] Implement the `stop_session`, `interrupt_session`, `delete_worktree` and `delete_session` handlers in `crates/micold-daemon/src/mcp/tools.rs` (policy → state conflicts → confirm → `stop_session` / `write_input(&[0x03])` / `ops::delete_worktree` / `delete_session`), and list them in `tools/list` — to pass T051
- [ ] T057 [US3] Implement the dialog state and reducer in `crates/micold-client/src/features/agent_confirm.rs` (`FloatingSurface` + `Registered` like `ConfirmSessionRemoveDialog` in `crates/micold-client/src/features/session.rs`), the arms in `crates/micold-client/src/shell/daemon_sync.rs`, and its one line in `crates/micold-client/src/overlay/registry.rs` — to pass T052
- [ ] T058 [US3] Render the dialog in `crates/micold-client/src/ui/confirm_agent_request.rs` from the shared Material dialog (`material::dialog`, `Button::filled/outlined`, `Surface::Dialog`) as `crates/micold-client/src/ui/confirm_session_remove.rs` does: calling session, operation and target named; Allow / Deny (GUI-glue exception; validated by T052's covered state and quickstart §B4)
- [ ] T059 [US3] Document the destructive operations, the confirmation dialog (every window, first answer, 60 s, no window) and the self-refusals in `docs/user-guide/agent-tools.md`

**Checkpoint**: an agent's `delete_worktree` shows a dialog in every window; Allow removes it, Deny refuses (quickstart §B4).

---

## Phase 6: User Story 4 — The agent coordinates with a sibling session (Priority: P3)

**Goal**: `read_session_output` and `send_session_input` under the three-value FR-016 option.

**Independent Test**: quickstart §B5; `mcp_cross_session.rs`.

### Tests for User Story 4 (write first, confirm they fail)

- [ ] T060 [P] [US4] Extend `crates/micold-core/tests/mcp_policy.rs`: `read_session_output` — Auto and ConfirmEachSend → `Proceed`, Off → `refused_by_policy`; `send_session_input` — Auto → `Proceed`, ConfirmEachSend → `Confirm(SendInput)`, Off → `refused_by_policy`; both on self → `invalid_input` (FR-015, FR-016)
- [ ] T061 [P] [US4] Extend `crates/micold-core/tests/mcp_tools_catalog.rs`: `read_session_output {session, lines?=200}` — `lines < 1` → `invalid_input`, `lines > 2000` clamped to 2000 (`LineCount`); `send_session_input {session, text}` — empty `text` → `invalid_input` (`NonEmptyText`, FR-012a)
- [ ] T062 [P] [US4] Extend `crates/micold-core/tests/settings_roundtrip.rs` and `protocol_roundtrip.rs`: `cross_session_access: CrossSessionAccess { Auto, ConfirmEachSend, Off }` defaults to `Auto`, an older file loads `Auto`, `DaemonSettings` / `SettingsSet` carry it
- [ ] T063 [P] [US4] Write a `plain_tail` unit test in `crates/micold-daemon/tests/scrollback_range.rs` (with `tests/support` terminal feeding): the last N lines of scrollback plus screen as right-trimmed text with no escape sequences, `truncated` true only when older lines existed, never more than N
- [ ] T064 [P] [US4] Write `crates/micold-daemon/tests/mcp_cross_session.rs` (US4 s1–s5; FR-010, FR-016; the isolation & lifecycle gate): at Auto, S1 reads S2's primary terminal (even with another shell instance attached) and a two-line `send_session_input` reaches S2's PTY as one bracketed submission ending `\r`; at ConfirmEachSend the read proceeds and the send waits for a fake client's answer (allow delivers, decline refuses, timeout → `needs_confirmation`, S2 untouched otherwise); at Off both are refused and S2 is untouched; a session of another project is `not_found`; changing the option through `SettingsSet` applies to the very next request of an already-running session
- [ ] T065 [P] [US4] Extend `crates/micold-client/tests/features_settings.rs` and `tests/support/covered_states.rs`: the Environment draft carries `cross_session_access`; choosing a value sends `SettingsSet { cross_session_access: Some(_) }`; the page with the select row is a covered state

### Implementation for User Story 4

- [ ] T066 [US4] Add the T060 rows to `crates/micold-core/src/mcp/policy.rs` and the T061 tools, `LineCount` and `NonEmptyText` to `crates/micold-core/src/mcp/tools.rs`
- [ ] T067 [US4] Add `cross_session_access` to `crates/micold-core/src/settings.rs`, `crates/micold-daemon/src/catalog.rs`, `DaemonSettings` / `SettingsSet` in `crates/micold-core/src/protocol/messages.rs`; bump `PROTOCOL_VERSION` to the next free number (changelog line, `schema_hash.rs` pin); `DaemonState::set_cross_session_access` and its `SettingsSet` arm in `crates/micold-daemon/src/server.rs` — to pass T062
- [ ] T068 [US4] Add `Framer::plain_tail(term, n) -> (Vec<String>, bool)` to `crates/micold-daemon/src/framer.rs` and `DaemonState::primary_framer(session)` in `crates/micold-daemon/src/state.rs` — to pass T063
- [ ] T069 [US4] Implement the `read_session_output` and `send_session_input` handlers in `crates/micold-daemon/src/mcp/tools.rs` (option read per request; confirm via `mcp/confirm.rs` for ConfirmEachSend; `encode_submission` written to the primary PTY) to pass T064
- [ ] T070 [US4] Add the select row "Let agents read and type into other sessions" (Auto / Confirm each send / Off) to `crates/micold-client/src/ui/settings/environment.rs` and its draft/message/handler in `crates/micold-client/src/features/settings.rs`, reusing the shared select component — to pass T065
- [ ] T071 [US4] Document both cross-session tools and the option's three values (default Auto, applies to the next request) in `docs/user-guide/agent-tools.md` and `docs/user-guide/settings.md`

**Checkpoint**: S1 reads S2's output and sends it a follow-up; the option tightens it without a restart (quickstart §B5).

---

## Phase 7: Polish & Cross-Cutting Concerns

- [ ] T072 [P] Write `crates/micold-daemon/tests/mcp_audit_log.rs` (FR-018, SC-010): every mutating tool writes exactly one `info` line with caller, op, target and outcome, and no `prompt` or `text` value appears in the log at any level (capture with the `log_redaction.rs` pattern); every failure in the acceptance tests carries one of the six categories
- [ ] T073 Implement the audit line once in `crates/micold-daemon/src/mcp/tools.rs` dispatch (fixed fields, target `micold::mcp`) to pass T072, removing any per-handler logging
- [ ] T074 [P] Write `crates/micold-daemon/tests/mcp_read_latency.rs` (SC-004): with 50 worktrees and 50 sessions, each read-only tool answers in under 1 s
- [ ] T075 [P] Write `crates/micold-daemon/tests/sandbox_real_mcp.rs` (feature `sandbox-real-runtime`, off by default; SC-008 for 027): a sandboxed session's binding file exists in the container's data dir and `whoami` answers from inside the container; nothing on the host's loopback answers on that port
- [ ] T076 Run quickstart §B1–§B6 with the real CLIs and the `visual-pass` skill; save evidence under `specs/034-daemon-mcp-server/evidence/`; if a Copilot probe fails, switch Copilot to `Unsupported` with the observed reason in `crates/micold-core/src/provider.rs` and the user guide (FR-005)
- [ ] T077 Final documentation pass: `docs/user-guide/agent-tools.md` covers every FR-019 item (tools, bound CLIs, toggle, cross-session option, confirmation policy, scope), and `docs/daemon.md` names the second loopback listener beside the hook receiver

---

## Dependencies & Execution Order

### Phase Dependencies

- Setup (T001–T002) → Foundational (T003–T010) → US1 (T011–T028).
- US2 depends on US1 (listener, credentials, tool dispatch).
- US3's start/rename part depends on US2 (`ops.rs`, `policy.rs`); its confirmation part depends on
  its start/rename part.
- US4 depends on US3's confirmation registry (Confirm each send) and on US2's `encode_submission`.
- Polish depends on all stories.

### Within Each User Story

Tests first, observed failing; core (`micold-core`) before daemon; daemon before client; the
user-guide task last in its group.

### Parallel Opportunities

- T003–T006 are independent test files.
- Within US1: T011–T016 in parallel; T023–T025 in parallel.
- Within US2: T029–T034 in parallel.
- Within US3: T043–T045, then T049–T052 in parallel.
- Within US4: T060–T065 in parallel.
- Polish: T072, T074, T075 in parallel.

## Parallel Example: User Story 1

```text
T011 mcp_binding_plan.rs   T012 mcp_name_collision.rs   T013 ai_cli_provider_seam.rs
T014 mcp_tools_catalog.rs  T015 mcp_read_tools.rs       T016 mcp_binding_spawn.rs
```

## Implementation Strategy

### MVP First

M1 (Setup + Foundational + the binding and `whoami`) is the smallest thing that proves the whole
chain — CLI flag, file, listener, credential, tool — against a real Claude Code.

### Incremental Delivery

Each milestone below merges on its own and leaves `main` with a working, documented subset of the
tools. No tool is listed in `tools/list` before its handler exists, and the confirmation dialog
arrives with the first tool that needs it.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — A Claude Code session is bound and answers `whoami` 🎯 MVP

- **Tasks**: T001–T022
- **Deliverable**: On `main`, every new Claude Code and Copilot session is spawned with a binding to
  the service's tool server, with no configuration file changed; its agent lists the server's tools
  and `whoami`, `list_worktrees`, `list_sessions`, `list_branches`, `get_session` answer for its own
  project; Pi sessions start unbound with the reason logged.
- **Satisfies**: US1 acceptance scenarios 1–5; FR-001, FR-002, FR-003, FR-005, FR-006, FR-007,
  FR-008 (read-only rows), FR-010, FR-013; SC-001, SC-002, SC-006, SC-009
- **Verify**: `mise run test-core` (`mcp_jsonrpc`, `mcp_binding_plan`, `mcp_name_collision`,
  `ai_cli_provider_seam`, `mcp_tools_catalog`) and `scripts/build-lock.sh cargo test -p
  micold-daemon --test mcp_endpoint --test mcp_binding_file_mode --test mcp_read_tools --test
  mcp_binding_spawn --test hooks_receiver`; quickstart §B1 steps 1–6 and §B2
- **Depends on**: —

### M2 — The user can turn the binding off

- **Tasks**: T023–T028
- **Deliverable**: On `main`, Settings → Environment has "Let AI sessions manage worktrees and
  sessions"; turned off, new sessions start without a binding and the log says why, while running
  sessions keep theirs.
- **Satisfies**: US1 acceptance scenario 6; FR-004, FR-005 (disabled reason)
- **Verify**: `scripts/build-lock.sh cargo test -p micold-daemon --test mcp_binding_spawn`,
  `mise run test-core` (`settings_roundtrip`, `protocol_roundtrip`, `schema_hash`),
  `scripts/build-lock.sh cargo test -p micold-client --test features_settings --test
  layout_snapshot`; quickstart §B6 (toggle row)
- **Depends on**: M1

### M3 — The agent creates a worktree and starts a session in it

- **Tasks**: T029–T042
- **Deliverable**: On `main`, an agent in a worktree session creates a worktree on a new branch and
  starts a session there with a first prompt; both appear in every window within 2 s, as if made
  from the dialog; a Default session's agent is refused.
- **Satisfies**: US2 acceptance scenarios 1–5; US3 acceptance scenario 6 (create); FR-009, FR-011,
  FR-015a (create), FR-017; SC-003, SC-005, SC-007
- **Verify**: `mise run test-core` (`mcp_policy`, `mcp_tools_catalog`, `input_readiness`) and
  `scripts/build-lock.sh cargo test -p micold-daemon --test ops_extraction --test
  mcp_create_worktree --test mcp_create_session --test mutation_semantics --test
  mutation_atomicity --test worktree_provenance_rpc`; quickstart §B3
- **Depends on**: M1

### M4 — The agent starts sessions and renames worktrees

- **Tasks**: T043–T048
- **Deliverable**: On `main`, an agent starts an idle, failed or resumable session and renames a
  worktree, with the result in every window; a Default session's agent cannot rename or delete a
  worktree.
- **Satisfies**: US3 acceptance scenarios 1, 6; FR-012a (start), FR-015a
- **Verify**: `mise run test-core` (`mcp_policy`, `mcp_tools_catalog`) and
  `scripts/build-lock.sh cargo test -p micold-daemon --test mcp_lifecycle_tools`
- **Depends on**: M3

### M5 — Destructive requests wait for the user's confirmation

- **Tasks**: T049–T059
- **Deliverable**: On `main`, an agent's `stop_session`, `interrupt_session`, `delete_session` or
  `delete_worktree` on another target opens a confirmation dialog in every window naming the caller,
  the operation and the target; Allow performs it, Deny refuses it, 60 s or no window answers "needs
  confirmation"; self-targets are refused.
- **Satisfies**: US3 acceptance scenarios 2–5; FR-009, FR-011, FR-012a (stop), FR-013, FR-014,
  FR-015
- **Verify**: `mise run test-core` (`protocol_roundtrip`, `schema_hash`) and
  `scripts/build-lock.sh cargo test -p micold-daemon --test mcp_confirmations --test
  mcp_lifecycle_tools`, `scripts/build-lock.sh cargo test -p micold-client --test
  features_agent_confirm --test layout_snapshot`; quickstart §B4 (visual pass) and §B1 step 7
- **Depends on**: M4

### M6 — Agents read and type into sibling sessions under the user's option

- **Tasks**: T060–T071
- **Deliverable**: On `main`, an agent reads a sibling session's recent output and sends it a
  prompt; Settings → Environment offers Auto (default) / Confirm each send / Off, and a change
  applies to the next request.
- **Satisfies**: US4 acceptance scenarios 1–5; FR-012, FR-012a (empty text), FR-015 (self), FR-016
- **Verify**: `mise run test-core` (`mcp_policy`, `mcp_tools_catalog`, `settings_roundtrip`,
  `protocol_roundtrip`) and `scripts/build-lock.sh cargo test -p micold-daemon --test
  mcp_cross_session --test scrollback_range`, `scripts/build-lock.sh cargo test -p micold-client
  --test features_settings --test layout_snapshot`; quickstart §B5, §B6
- **Depends on**: M5

### M7 — Audit, timing, sandbox and the full real-CLI pass

- **Tasks**: T072–T077
- **Deliverable**: On `main`, every mutating tool call leaves exactly one audit line with no prompt
  text, read tools meet SC-004, the sandboxed placement is covered by a real-runtime test, and the
  quickstart §B evidence is recorded.
- **Satisfies**: FR-018, FR-019; SC-004, SC-008, SC-010
- **Verify**: `scripts/build-lock.sh cargo test -p micold-daemon --test mcp_audit_log --test
  mcp_read_latency`; `mise run image && mise run test-sandbox` (`sandbox_real_mcp`);
  `specs/034-daemon-mcp-server/evidence/` holds §B1–§B6
- **Depends on**: M6

## Notes

- M1 is 22 tasks, above the ~15 guideline: the foundational listener, credentials and binding have
  no observable deliverable without at least one tool and one bound CLI, and the read-only tools are
  thin handlers over one snapshot. The toggle (US1 s6) is split into M2 along its acceptance
  scenario.
- US3 is split into M4 (start, rename) and M5 (confirmation chain) along its acceptance scenarios; M5
  keeps the dialog, the registry and the destructive tools together because scenario 4 cannot be
  observed without all three.
