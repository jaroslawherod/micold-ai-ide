# Tasks: Identify Agent Work From the AI CLI's Native Signals

**Input**: `specs/552-native-agent-session-signals/` (spec.md, plan.md, research.md, data-model.md, contracts/subagent-hooks.md, quickstart.md)
**Tests**: required (Constitution I): every code task is preceded by the failing test that covers it. Deletions start by flipping the existing visibility/attach tests to the new expectation (red), then remove the code.

## Format

`- [ ] T### [P?] [US?] description with file path`. `[P]` = disjoint files, no dependency on an incomplete task.

## Phase 1: Setup

- [x] T001 Capture live evidence before any registration code (research R1, R3, R7): (a) run a Claude session with a settings file registering `SubagentStart` and `SubagentStop` http hooks to a local listener, run an Explore sub-agent, save the two raw bodies and the `--settings` acceptance result; (b) run the installed Copilot CLI with a sub-agent and save the `events.jsonl` lines naming a sub-agent event, or record that none exists. Write the findings to `specs/552-native-agent-session-signals/research.md` (new section R8) and the bodies to `specs/552-native-agent-session-signals/contracts/captures/`. A result contradicting R1 (payload carries a distinct session id, or `--settings` rejects an event) stops the run and returns to the plan.

## Phase 2: Foundational (blocks the stories)

- [x] T002 Flip the worktree visibility expectations to "every worktree is listed": rewrite `crates/micold-client/tests/worktree_visibility.rs`, the hiding cases of `crates/micold-client/tests/sidebar_tree.rs` (444-685, 941-1052) and `crates/micold-client/tests/app_state.rs` (617-777), and the toggle uses in `crates/micold-client/tests/{features_worktree,sidebar_state,worktree_claim,directory_availability,feature_write_isolation,availability_is_asked_only_on_named_events}.rs` (compile fixes only for `worktree_claim`, deleted in T023); they must fail (red) before T005. `crates/micold-core/tests/reserved_convention.rs` stays until T024.

## Phase 3: User Story 1 - Every worktree is listed, with no name-based hiding (P1) 🎯 MVP

**Goal**: no code classifies a worktree by name; no toggle; all worktrees listed.
**Independent test**: a fixture with `agent-<17 hex>` / `worktree-agent-<17 hex>` and ordinary worktrees lists them all, with no control, and renaming changes nothing.

- [x] T003 [US1] Add the rename-invariance and no-control tests (SC-001, SC-002): a worktree list renamed to and from the old convention keeps its listing and order, and the sidebar view model exposes no reveal control, in `crates/micold-client/tests/worktree_visibility.rs`.
- [x] T004 [US1] Add a core test that a stored project file holding a pre-feature hide preference key still loads with every worktree visible (FR-010 hide clause) in `crates/micold-core/tests/store_roundtrip.rs`.
- [x] T005 [US1] Stop the client consulting the classification: remove every call of `classify_owner`, `WorktreeOwner` and `matches_reserved_convention` from `crates/micold-client/src/{features/worktree,features/sidebar,ui/sidebar,ui/mod,catalog_sync}.rs` so visibility, tags and ordering depend on no name. The core symbols (`worktree.rs` classification, `attach.rs` `attachable_worktrees`/`NeedsWorktreeAttach`, `plan_backfill`, `CheckedOutAt.owner`, daemon `mcp/tools.rs` `app_created`) and every wire shape in `protocol/messages.rs` stay untouched in M1 (only provenance-driven attach, removed in M3, still reads them), so the schema hash does not move; their deletion is T024/T025.
- [x] T006 [US1] Remove `show_agent_worktrees`, `Msg::ShowAgentWorktreesToggled`, the "Agent" tag in `worktree_tags` and the visibility filter in `crates/micold-client/src/features/{worktree,sidebar}.rs`, `crates/micold-client/src/ui/{sidebar,mod}.rs`, `crates/micold-client/src/shell/startup.rs`, `crates/micold-client/src/main.rs`, `crates/micold-client/src/main_tests.rs`.
- [x] T007 [P] [US1] Update `docs/user-guide/worktrees-and-sessions.md` and `docs/user-guide/icons.md`: all worktrees are listed, no "Show agent worktrees" chip, no agent tag.
- [x] T008 [P] [US1] Add the superseded note to `specs/014-hide-agent-worktrees/spec.md` (and its `plan.md`/`tasks.md` heads if they carry rules): names 552 and what replaced the hiding.
- [x] T009 [US1] Run the `visual-pass` skill for quickstart §B (sidebar shows every worktree, no chip) and save evidence in `specs/552-native-agent-session-signals/visual-pass/`.

## Phase 4: User Story 2 - Agent runs recognised from the provider's own signal (P1)

**Goal**: `SubagentStart`/`SubagentStop` are registered, parsed, recorded per origin session, persisted and carried on `SessionSummary`, never touching the activity FSM.
**Independent test**: post both payloads to the hook receiver for a known session; assert the summary's `agents`; restart; assert it persists.

- [x] T010 [US2] Write failing core tests for `AgentRun` rules (data-model.md): idempotent start, stop sets `Stopped` and fills `transcript`, stop-before-start inserts `Stopped`, cap 256 evicting oldest `Stopped` first, exit/restart marks `Running` runs `Stopped`, in `crates/micold-core/tests/agent_run.rs`.
- [x] T011 [US2] Implement `AgentRun`, `AgentRunState`, `AgentSignal` and the upsert rules in `crates/micold-core/src/agent_run.rs`; export from `crates/micold-core/src/lib.rs`.
- [x] T012 [US2] Write failing store tests in `crates/micold-core/tests/store_roundtrip.rs`: `StoredSession.agents` round-trips, is skipped when empty, an older file without it loads, and `SCHEMA_VERSION` stays 1.
- [x] T013 [US2] Add `StoredSession.agents: Vec<AgentRun>` (`#[serde(default, skip_serializing_if = "Vec::is_empty")]`) in `crates/micold-core/src/store.rs` and `Session.agents` in `crates/micold-core/src/session.rs`.
- [x] T014 [US2] Write failing wire tests: `SessionSummary.agents` serialises and defaults empty; `PROTOCOL_VERSION` 40 -> 41 and the history entry in `crates/micold-core/tests/schema_hash.rs` and `crates/micold-core/src/protocol/version.rs` tests.
- [x] T015 [US2] Add `SessionSummary.agents: Vec<AgentRunSummary>` in `crates/micold-core/src/protocol/messages.rs`, bump `PROTOCOL_VERSION` to 41 in `crates/micold-core/src/protocol/version.rs`, update the pin in `crates/micold-core/tests/schema_hash.rs` and the version history in `docs/daemon.md`.
- [x] T016 [US2] Write failing hook tests in `crates/micold-daemon/src/hooks.rs` (unit: `classify_hook` per contract C2 table incl. missing/empty/non-string `agent_id` -> `Ignored`; `settings_json` registers both events with `matcher: ""`, replacing the "SubagentStop must not be configured" assertion) and in `crates/micold-daemon/tests/hooks_receiver.rs` (post Start then Stop, assert `SessionSummary.agents`, `200` for ignored, `400` for invalid JSON, body never logged).
- [x] T017 [US2] Implement `HookClass::Agent(AgentSignal)`, the payload struct, registration of `SubagentStart`/`SubagentStop` in `settings_json` and the receiver dispatch in `crates/micold-daemon/src/hooks.rs`, using the payload shapes captured at T001.
- [x] T018 [US2] Write failing daemon tests: `SubagentStop` still leaves the activity state unchanged with the event now registered (extend `crates/micold-daemon/tests/attention_claims.rs`, FR-007); a signal for session A leaves session B untouched (FR-014); a session with no signal has empty `agents` (FR-006); persisted runs survive a daemon restart and `Running` runs come back `Stopped` (FR-005); origin deletion removes its runs, in `crates/micold-daemon/tests/agent_runs.rs`.
- [x] T019 [US2] Implement `note_agent_signal(session, signal) -> bool`, persistence through the catalog write path, `session_summary` population, restart/exit reconciliation and `broadcast_catalog()` on change in `crates/micold-daemon/src/state.rs` and `crates/micold-daemon/src/catalog.rs`; `note_activity` is never called for `Agent`.
- [x] T020 [P] [US2] Add a client reducer test that a summary carrying `agents` lists every worktree and session unchanged and hides nothing (FR-011) in `crates/micold-client/tests/sidebar_tree.rs`; make the client compile against the new field in `crates/micold-client/src/catalog_sync.rs`.
- [x] T021 [P] [US2] Document the Claude signal in `docs/user-guide/agent-tools.md` and `docs/user-guide/worktrees-and-sessions.md`: sub-agent runs are identified from `SubagentStart`/`SubagentStop`, recorded on the session and kept across restarts; background `claude --bg` sessions are a documented gap (R2).

## Phase 5: Provenance and worktree attaching removed (FR-009, FR-010) [US1]

**Goal**: no read or write of provenance decides anything; feature 582's worktree attaching is gone; discovery and resume stay.
**Independent test**: a pre-feature state file with provenance keys loads and hides nothing; the MCP tool list has no `attach_worktree`; the attach offer fires for stored provider sessions the catalog lacks.

- [x] T022 [US1] Write failing tests: `PROTOCOL_VERSION` is 46 and `Operation::AttachWorktree`/`user_created` are absent from the wire (`crates/micold-core/tests/schema_hash.rs`); a stored project file with `worktree_provenance` and `provenance_migrated` loads and is rewritten without them (`crates/micold-core/tests/store_roundtrip.rs`, `crates/micold-core/tests/store_fault_isolation.rs`); `attachable_worktrees`/worktree `AttachTarget` are gone and a session whose worktree exists is `resumable` (`crates/micold-core/tests/attach_discovery.rs`); the MCP catalog and policy have no `attach_worktree` (`crates/micold-core/tests/mcp_tools_catalog.rs`, `crates/micold-core/tests/mcp_policy.rs`).
- [x] T023 [US1] Write failing daemon and client tests: `crates/micold-daemon/tests/attach_apply.rs` and `attach_mcp.rs` have no worktree target; the start-up offer fires on stored provider sessions the catalog lacks, dismissal stays in memory (`crates/micold-client/tests/attach_offer.rs`); the attach dialog has no worktree rows (`crates/micold-client/tests/attach_dialog.rs`); delete `crates/micold-core/tests/{worktree_provenance,provenance_migration}.rs`, `crates/micold-daemon/tests/worktree_provenance_rpc.rs` and the provenance cases in `crates/micold-daemon/tests/{worktree_claim,ops_extraction,mcp_read_tools,mcp_audit_log,mcp_binding_spawn}.rs`, `crates/micold-client/tests/worktree_claim.rs` and the other client tests listed by `grep -l worktree_provenance crates/micold-client/tests`.
- [x] T024 [US1] Delete the classification in `crates/micold-core/src/worktree.rs` (`AGENT_*_PREFIX`, `is_agent_id`, `matches_reserved_convention`, `WorktreeOwner`, `classify_owner`, `CheckedOutAt.owner`) with `crates/micold-core/tests/reserved_convention.rs` and the tests naming them (`branch_conflict`, `branch_candidates`, `daemons_bindings`, `attach_discovery`, daemon `worktree_claim`, client `worktree_form_daemon_choice`), and drop the `WorktreeSummary.user_created` wire field and `app_created` in `mcp/tools.rs`, and the daemon `list_worktrees` `include_hidden` filter (`worktree_rows`, which in M1 still skips Agent rows the sidebar now lists). Then remove provenance in core: `worktree_provenance`/`provenance_migrated`/`record_user_created` in `crates/micold-core/src/workspace.rs`; `ProvenanceView`, `durably_known_worktrees`, `plan_backfill` in `crates/micold-core/src/worktree.rs`; keep the two keys as read-and-ignored `serde` fields in `crates/micold-core/src/store.rs`; delete `crates/micold-core/src/bin/micold-demo-provenance.rs` and its step in `site/capture/demo-project.sh`.
- [x] T025 [US1] Remove worktree attaching in core: `attachable_worktrees`, the `AttachTarget` worktree arm and `NeedsWorktreeAttach` in `crates/micold-core/src/attach.rs`; `attach_worktree` in `crates/micold-core/src/mcp/{tools,policy}.rs`; `Operation::AttachWorktree` and the worktree attach shapes in `crates/micold-core/src/protocol/messages.rs`; bump `PROTOCOL_VERSION` 45 -> 46 in `crates/micold-core/src/protocol/version.rs`, update the pin in `crates/micold-core/tests/schema_hash.rs` and `docs/daemon.md`.
- [x] T026 [US1] Remove provenance claim and attach handling in the daemon: `crates/micold-daemon/src/{catalog,state,ops,server,runs}.rs`, `crates/micold-daemon/src/mcp/tools.rs`.
- [x] T027 [US1] Remove the client's worktree attach rows, banner wording and provenance sync: `crates/micold-client/src/features/{attach,worktree,window}.rs`, `crates/micold-client/src/ui/attach_dialog.rs`, `crates/micold-client/src/catalog_sync.rs`, `crates/micold-client/src/overlay/registry.rs`, `crates/micold-client/src/shell/{startup,daemon_sync}.rs`, `crates/micold-client/src/app.rs`.
- [x] T028 [P] [US1] Update `docs/user-guide/agent-tools.md` (no `attach_worktree` tool) and `docs/user-guide/worktrees-and-sessions.md` (attach offers stored provider sessions only); drop the `attach_worktree` entry in `docs/development/duplication-triage.md`.
- [x] T029 [P] [US1] Add superseded notes to `specs/029-worktree-provenance/spec.md` and `specs/582-attach-provider-worktrees-sessions/spec.md`, each naming 552 and what replaced the rules.

## Phase 6: User Story 3 - Providers without a native signal (P2)

**Goal**: Copilot mapped from its own event log if T001 confirmed an event; Pi, Codex, OpenCode are stated gaps; no guessing.
**Independent test**: a Copilot `events.jsonl` fixture with a sub-agent event yields a run; a fixture session of a provider without a signal has empty `agents`.

- [x] T030 [US3] From T001's capture, write failing tests for `copilot_agent_event` in `crates/micold-daemon/src/activity.rs` (unit, with the captured lines as fixture under `crates/micold-daemon/tests/fixtures/`) and an integration case in `crates/micold-daemon/tests/agent_runs.rs`; if T001 found no Copilot event, instead add a test that a Copilot session never gains runs and skip T031.
- [x] T031 [US3] Implement `copilot_agent_event` and route it through `note_agent_signal` in `crates/micold-daemon/src/{activity,event_log}.rs`, never through `note_activity`.
- [x] T032 [US3] Reconcile with the earlier edits of these pages and add the per-provider table (Claude, Copilot per T001, Pi/Codex/OpenCode = no native signal, `claude --bg` gap) to `docs/user-guide/agent-tools.md`, `docs/user-guide/settings.md` and the entry in `docs/README.md` (FR-012).

## Phase 7: User Story 4 - Superseded specs and docs say so (P3)

- [x] T033 [US4] First `grep -rln "Show agent worktrees\|show_agent_worktrees\|worktree-agent-" specs` to list every other spec whose rules this removes and mark each superseded (FR-013). Then add a docs check `scripts/tests/superseded-552.test.sh` asserting: no `Show agent worktrees`, `worktree-agent-` or `provenance` hit in `docs/user-guide` outside a superseded note; every spec found above starts with a note naming 552 (quickstart §A.2, §A.3). Register it where `mise run test-scripts` finds it.

## Dependencies

- T001 first; T017 and T030 consume its captures.
- T002 (red) -> T005, T006. Phase 5 depends on Phase 3 (no classification reads provenance first).
- US2 (Phase 4) is independent of Phase 3 code but follows it in milestone order; Phase 5 follows Phase 4 so the protocol bumps are 41 then 42.
- US3 depends on US2's `note_agent_signal` (T019). T033 depends on T008, T029 and the doc edits.

## Parallel opportunities

T007/T008 (docs, specs); T020/T021 after T015; T028/T029 after T027.

## Implementation Strategy

MVP is US1 + capture (M1); it removes only the client's use of the classification so main stays green. Then US2 (the replacement), then the provenance/attach removal, then providers and the docs check. Each milestone leaves `main` green: M1 removes only the client's reads of the classification, so core helpers, provenance and `attach_worktree` still compile until M3.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Every worktree listed, name-based hiding removed 🎯 MVP

- **Tasks**: T001–T009
- **Deliverable**: the sidebar lists every worktree, including `agent-<hex>` ones, with no "Show agent worktrees" control and no client code deciding visibility by name, and the live Claude/Copilot hook captures are recorded. The core classification helpers remain, unused by the client, until M3; `attach_worktree` behaves as before.
- **Satisfies**: US1 acceptance scenarios 1–3; FR-001 (visibility; remaining helpers go in M3), FR-002, FR-010 (hide clause), FR-013 (014)
- **Verify**: `mise run test-core` and `mise run gate` (includes the client's `worktree_visibility` test); quickstart §B
- **Depends on**: —
- **Tier**: full

### M2 — Claude sub-agent runs recorded from SubagentStart/SubagentStop

- **Tasks**: T010–T021
- **Deliverable**: a Claude session's sub-agent runs appear on its `SessionSummary.agents`, survive a daemon restart, and never change the origin's activity state; protocol is 41.
- **Satisfies**: US2 acceptance scenarios 1–5; FR-003 (Claude), FR-004, FR-005, FR-006, FR-007, FR-008, FR-011, FR-014
- **Verify**: `cargo test -p micold-daemon --test hooks_receiver --test agent_runs --test attention_claims`; quickstart §C
- **Depends on**: M1
- **Tier**: full

### M3 — Provenance and worktree attaching removed

- **Tasks**: T022–T029
- **Deliverable**: provenance records and `attach_worktree` are gone, old state files load, the attach offer is driven by stored provider sessions the catalog lacks; protocol is 46.
- **Satisfies**: FR-009, FR-010, FR-013 (029, 582); US1 acceptance scenario 2
- **Verify**: `cargo test -p micold-core --test store_roundtrip --test attach_discovery --test mcp_tools_catalog` and `cargo test -p micold-client --test attach_offer`
- **Depends on**: M1, M2
- **Tier**: full

### M4 — Other providers mapped or stated as gaps, docs check

- **Tasks**: T030–T033
- **Deliverable**: Copilot sub-agent events recorded (or the gap stated), the per-provider table is in the user guide, and a CI script keeps the superseded notes and guide wording honest.
- **Satisfies**: US3 acceptance scenarios 1–2; US4 acceptance scenarios 1–2; FR-003 (other providers), FR-012, FR-013
- **Verify**: `scripts/tests/superseded-552.test.sh` and `cargo test -p micold-daemon --test agent_runs`
- **Depends on**: M2, M3
- **Tier**: full
