---

description: "Task list for the session tree of agent-spawned sessions (#551)"
---

# Tasks: Session Tree for Agent-Spawned Sessions

**Input**: Design documents from `/specs/551-session-spawn-tree/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md

**Tests**: Mandatory (Constitution I). Each test task precedes the implementation it covers and must be seen failing first. `[A#]`/`[U#]` markers name behaviors of `tdd/test-list.md`.

**Documentation**: The user-guide task of each story sits in that story's milestone (Constitution VII).

**Cross-platform**: No OS branch; ids are UUIDs and provider stores are read through existing path derivation.

## Format: `[ID] [P?] [Story] Description`

## Phase 1: Setup

- [ ] T001 Add the `origin` module to `crates/micold-core/src/lib.rs` and create `crates/micold-core/src/origin.rs` with `SessionOrigin { id: SessionId, status: OriginStatus }` and `OriginStatus = Known | Removed` (serde, `#[serde(rename_all = "snake_case")]`), no logic yet

- [ ] T025 [P] [U15] Spike (runs here so M2 and M3 know its outcome; its fixtures are consumed by T026), before any provider reader: confirm the signals of research.md R2 (Claude Code `claude --bg` from a session, OpenCode `parentID`, Codex `session_meta.source`); record each real output as a fixture under `crates/micold-core/tests/fixtures/<provider>/` and update the R2 table in `specs/551-session-spawn-tree/research.md`, `contracts/provider-signals.md` and spec Assumptions with confirmed or "gap"

## Phase 2: Foundational (blocks every story)

- [ ] T002 [P] Write failing unit tests for `validate_origin(child, candidate, lookup, reported_by_provider) -> Result<(), OriginRejected>` in `crates/micold-core/src/origin.rs`: self rejected; candidate in another project/worktree (`SessionLocation` unequal) rejected; chain from candidate reaching the child (cycle, bounded walk) rejected; an unknown candidate (not in the lookup) is accepted only when the caller says it was reported by a provider reader (`reported_by_provider: bool` argument) and rejected otherwise (R4) [U1] [U2] [U3]
- [ ] T003 Implement `origin::validate_origin` in `crates/micold-core/src/origin.rs` (one pure function used by every write path, R4; signature adds `reported_by_provider: bool`) [U1] [U2] [U3]
- [ ] T004 [P] Write failing tests in `crates/micold-core/tests/store_roundtrip.rs`: a catalog file without the key loads with every session top-level and `SCHEMA_VERSION` stays 1 (FR-015); a stored `origin` survives save and load (FR-006) [U5]
- [ ] T005 Add `StoredSession.origin: Option<SessionId>` (`#[serde(default, skip_serializing_if = "Option::is_none")]`) in `crates/micold-core/src/store.rs` and `Session.origin: Option<SessionOrigin>` in `crates/micold-core/src/session.rs` ("write-once; `None` for every session today"; only `id` is stored, `status` is runtime) and `StoredSession.origin_rejected: bool` (`#[serde(default, skip_serializing_if = "is_false")]`, set when a pending provider origin failed the full check) [U5]
- [ ] T006 [P] Write failing tests in `crates/micold-core/tests/protocol_roundtrip.rs`: `SessionSummary.origin: Option<SessionOrigin>` (`#[serde(default)]`) round-trips in `SessionChanged` and snapshots; an older payload without it decodes to `None`; `PROTOCOL_VERSION` is 41 [U6]
- [ ] T007 Add `SessionSummary.origin` in `crates/micold-core/src/protocol/messages.rs` and bump `PROTOCOL_VERSION` 40 to 41 in `crates/micold-core/src/protocol/version.rs` [U6]

## Phase 3: User Story 1 + 2, daemon half (Priority: P1) 🎯 MVP

**Goal**: A session started by an agent through the tool server is recorded as a child of the caller, the fact persists, and clients and assistants can read it.

**Independent Test**: `cargo test -p micold-daemon --test mcp_create_session` and the restart/removal tests below.

### Tests

- [ ] T008 [P] [US1] Write failing tests in `crates/micold-daemon/tests/mcp_create_session.rs`: `create_session` for the caller's own worktree returns a row with `origin` = caller `ref` and `origin_state: "known"` and the catalog stores it (FR-019); an archived caller records no origin; another worktree yields a top-level session with no origin (FR-009); no input parameter names an origin; `list_sessions` and `get_session` rows carry `origin`/`origin_state` and omit them for top-level sessions (FR-013); a caller that exited still records [U7] [U8] [U9] [A1]
- [ ] T009 [P] [US1] Write failing tests in `crates/micold-core/tests/mcp_tools_catalog.rs` that the `list_sessions`/`get_session`/`create_session` tool descriptions name the `origin` and `origin_state` row fields (contracts/mcp-origin.md) [U9]
- [ ] T010 [P] [US2] Write failing tests in `crates/micold-daemon/tests/catalog_adoption.rs`: `Catalog::load` after restart keeps every origin (SC-002); a live child becomes interrupted-resumable and stays nested under its origin (US2.2); the origin is persisted in the same `persist()` as the new session; `record_origin` on a session that already has an origin returns `Ok(false)` and keeps the first, and recording the same origin again is idempotent (FR-010) [U4] [U10] [A5] [A6]
- [ ] T011 [P] [US2] Write failing tests in `crates/micold-daemon/tests/session_origin_removal.rs`: single delete (`Catalog::archive_session`), `archive_session_ids` and `archive_worktree_sessions` each emit a `SessionChanged` with `status: "removed"` for every direct child in the same step; the child is neither deleted nor stopped (FR-018); stopping an origin changes nothing; a snapshot after the archive carries `removed`; two subscribers see the same summary (FR-012) [U11] [A7]

### Implementation

- [ ] T012 [US1] Add `NewSession.origin: Option<SessionId>` in `crates/micold-daemon/src/ops.rs` (passed by `create_session_with_prompt`), `origin: None` at the existing caller in `crates/micold-daemon/src/runs.rs`, and an `origin` argument on `DaemonState::create_session` and the `NewSession` built in `crates/micold-daemon/src/state.rs` [U7]
- [ ] T013 [US1] In `crates/micold-daemon/src/catalog.rs` make `Catalog::create_session` take `origin: Option<SessionId>`, validate with `validate_origin`, store it in the same `persist()`; add `record_origin` returning `Ok(false)` when an origin exists (FR-010) [U7] [U8] [U10] [U4]
- [ ] T014 [US2] In `crates/micold-daemon/src/catalog.rs` overlay `OriginStatus` in `session_summary` (`Removed` when the origin's record is archived) and add one helper shared by the three archive paths that emits a child `SessionChanged` with `status: removed` [U11]
- [ ] T015 [US1] In `crates/micold-daemon/src/mcp/tools.rs` pass `Some(caller)` to `Catalog::create_session` when the target worktree equals the caller's (`resolve_caller` location), else `None`; add `origin` and `origin_state` to `session_row`; update the row text in `crates/micold-core/src/mcp/tools.rs` [U7] [U8] [U9]
- [ ] T016 [US1] Document the new row fields and the `create_session` nesting in `docs/user-guide/agent-tools.md` [U9]

### Outer loop

- [ ] T038 [US2] Run the M1 acceptance behaviors through the real entry points (`mcp_create_session`, `catalog_adoption`, `session_origin_removal`) and confirm each is green: [A5] [A6] (daemon half of [A1], [A7])

## Phase 4: User Story 1 + 2, sidebar half (Priority: P1)

**Goal**: The sidebar shows children nested under their origin at any depth, in every location, with the child marker and the pending/removed markers.

**Independent Test**: `cargo test -p micold-client --test sidebar_tree --test layout_snapshot`.

### Tests

- [ ] T017 [P] [US1] Write failing unit tests for `SessionTree` in `crates/micold-core/src/origin.rs`: each session appears exactly once (FR-008); nested origin yields depth 2+ (US1.2); a cycle breaks at the first repeat with the repeated member as a root and the build terminates; children and roots keep catalog order; an origin not in the input is `Orphan::Pending` when status `known` and `Orphan::Removed` when status `removed` [U12] [A2]
- [ ] T018 [P] [US1] Write failing reducer tests in `crates/micold-client/tests/sidebar_tree.rs`: `worktree_tree` nests children (FR-003, FR-004); a session without origin is exactly as today (FR-005, SC-003); orphans are top-level at depth 1 with "origin pending"/"origin removed" (FR-016, US2.3); a pending child moves under its origin when the origin appears; the Default group nests the same way; a child's select, stop, resume and delete messages are the same as a top-level session's (FR-007) [U13] [A1] [A2] [A3] [A7]
- [ ] T019 [P] [US1] Write failing layout-gate rows in `crates/micold-client/tests/layout_snapshot.rs`: every child row, orphans included, carries the child marker and no user-started top-level row does (FR-007, SC-004) [U14] [A4]

### Implementation

- [ ] T020 [US1] Implement `SessionTree` (derived, never stored; Root / Child / Orphan{Pending|Removed}) in `crates/micold-core/src/origin.rs` [U12]
- [ ] T021 [US1] Build the nesting from `SessionTree` in `crates/micold-client/src/features/sidebar.rs` (`worktree_tree` and the Default group), depth-first in catalog order [U13]
- [ ] T022 [US1] Add the child-marker builder method to `TreeItem` in `crates/micold-client/src/ui/material/tree_view.rs` and an entry in the component showcase (Constitution VIII) [U14]
- [ ] T023 [US1] Recurse over children in `crates/micold-client/src/ui/sidebar.rs` (`group_items`, `build_default_item`, `session_tree_item(depth, ..)`), marker on children, muted annotation on orphans [U13] [U14]
- [ ] T024 [US1] Add the session-tree section to `docs/user-guide/worktrees-and-sessions.md` (what nests, markers, removed origin, no manual reparenting) [U13]

### Outer loop

- [ ] T039 [US1] Run the sidebar acceptance behaviors (`sidebar_tree`, `layout_snapshot`) and confirm each is green: [A1] [A2] [A3] [A4] [A7]

## Phase 5: User Story 3 (Priority: P2)

**Goal**: Providers that natively report a spawning session are nested; providers that cannot stay as today, and the guide says which.

**Independent Test**: `cargo test -p micold-core --test provider_origin` and the adoption tests.

### Tests

- [ ] T026 [P] [US3] Write failing tests in `crates/micold-core/tests/provider_origin.rs`: the default `reported_origin` is `None`; each confirmed provider's fixture yields `Some(parent)` and the same fixture without the field yields `None`; a name, path or timestamp never yields an origin (FR-002) [U15] [A8]
- [ ] T027 [P] [US3] Write failing tests in `crates/micold-daemon/tests/catalog_adoption.rs`: discovery records a reported origin once (FR-001, FR-010); a child adopted before its origin stays top-level pending and nests when the origin is adopted (FR-016); a pending origin that fails the full check (same-worktree, cycle) when it appears is cleared once, `origin_rejected` is persisted and no later pass re-records it (R4); an adopted session of a gap provider stays top-level [U16] [A8]
- [ ] T028 [P] [US3] Write a failing test in `crates/micold-core/tests/provider_origin_docs.rs` that the provider table in `docs/user-guide/worktrees-and-sessions.md` lists every `AiCli` variant (SC-005) [U17] [A9]

### Implementation

- [ ] T029 [US3] Add `AiCliProvider::reported_origin(&self, config_dir, cwd, session_id) -> Option<Uuid>` (default `None`; pure read, no write) and the readers for the confirmed providers in `crates/micold-core/src/provider.rs` [U15]
- [ ] T030 [US3] In `crates/micold-daemon/src/state.rs` `discover_external_sessions`, call `reported_origin` for newly adopted sessions and for adopted sessions without an origin on each later pass; in `crates/micold-daemon/src/catalog.rs` keep a not-yet-known origin pending, run the full check when it appears, and clear it once if rejected, setting `origin_rejected` so discovery skips the session afterwards [U16]
- [ ] T031 [US3] Add the provider table (reports or not, by which signal, what a gap does instead) to `docs/user-guide/worktrees-and-sessions.md` (FR-014) [U17] [A9]

### Outer loop

- [ ] T040 [US3] Run the provider acceptance behaviors (`provider_origin`, `catalog_adoption`, `provider_origin_docs`) and confirm each is green: [A8] [A9]

## Phase 6: User Story 4 (Priority: P3)

**Goal**: A busy origin folds, shows its child count and attention, and keeps its fold across restarts.

**Independent Test**: `cargo test -p micold-client --test sidebar_tree` fold cases and the settings round trip.

### Tests

- [ ] T032 [P] [US4] Write a failing settings test in `crates/micold-core/tests/settings_roundtrip.rs`: `folded_origins: BTreeSet<SessionId>` (`#[serde(default)]`) round-trips, a file without it loads empty, `settings_version` does not move [U18] [A12]
- [ ] T033 [P] [US4] Write failing reducer tests in `crates/micold-client/tests/sidebar_tree.rs`: `Msg::ToggleOrigin` folds and expands; a folded origin reports its direct-child count and an attention rollup over descendants (FR-011); fold then restart then snapshot arrives still folded, with no pruning (FR-017); a never-folded origin is expanded [U19] [A10] [A11] [A12]

### Implementation

- [ ] T034 [US4] Add `Settings.folded_origins` in `crates/micold-core/src/settings.rs` (stored as folded, not expanded) [U18]
- [ ] T035 [US4] Add `Msg::ToggleOrigin(SessionId)` and the folded set in `crates/micold-client/src/features/sidebar.rs`, persisted through the existing settings path, with the count and attention rollup reusing the existing attention derivation [U19]
- [ ] T036 [US4] Add the folded-summary builder method (count, attention dot) to `TreeItem` in `crates/micold-client/src/ui/material/tree_view.rs` with a showcase entry, and wire fold toggling and the summary in `crates/micold-client/src/ui/sidebar.rs` [U19]
- [ ] T037 [US4] Document folding and its persistence in `docs/user-guide/worktrees-and-sessions.md` [U19]

### Outer loop

- [ ] T041 [US4] Run the fold acceptance behaviors (`sidebar_tree`, `settings_roundtrip`) and confirm each is green: [A10] [A11] [A12]

## Dependencies & Order

Phase 1 then 2 block all. Phase 3 (daemon) before Phase 4 (client reads `SessionSummary.origin`). Phase 5 needs Phase 3 (`record_origin`); its nesting becomes visible with Phase 4. The spike T025 runs in M1 so its outcome is known before M2 and M3. Phase 6 needs Phase 4.

## Implementation Strategy

MVP is M1: origin recorded, persisted and visible to assistants and clients. M2 makes it visible in the sidebar. Quickstart Part B (visual pass), recording its results and ticking tasks is left to the close unit (no code).

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Origin recorded, persisted and exposed 🎯 MVP (data and tool API; M2 makes it visible in the sidebar)

- **Tasks**: T001–T016, T025, T038
- **Deliverable**: A session created through the tool server from a session in the same worktree is recorded as its child, survives a daemon restart, and `list_sessions`/`get_session` and the wire show its origin; deleting the origin leaves the child with status removed; the spike's result on provider signals is recorded in research.md R2.
- **Satisfies**: US1 acceptance scenario 3 (unchanged top-level); US2 acceptance scenarios 1–2; FR-006, FR-009, FR-010, FR-012, FR-013, FR-015, FR-018, FR-019
- **Verify**: `cargo test -p micold-daemon --test mcp_create_session --test catalog_adoption --test session_origin_removal` and `mise run test-core`; `ls crates/micold-core/tests/fixtures/` shows the spike's fixtures and research.md R2 marks each provider confirmed or gap
- **Depends on**: —
- **Tier**: full

### M2 — Sidebar shows the tree

- **Tasks**: T017–T024, T039
- **Deliverable**: The sidebar nests child sessions under their origin at any depth, in worktrees and the Default entry, with the child marker and the origin pending/removed markers.
- **Satisfies**: US1 acceptance scenarios 1, 2, 4; US2 acceptance scenario 3; FR-003, FR-004, FR-005, FR-007, FR-008, FR-016; SC-001, SC-004
- **Verify**: `cargo test -p micold-client --test sidebar_tree --test layout_snapshot`; quickstart Part B steps 1, 2, 5, 6, 7
- **Depends on**: M1
- **Tier**: full

### M3 — Providers report their origin

- **Tasks**: T026–T031, T040
- **Deliverable**: Sessions adopted from a provider whose native signal is confirmed nest under their origin; every other provider is unchanged, and the user guide lists each provider's status.
- **Satisfies**: US3 acceptance scenarios 1–2; FR-001, FR-002, FR-014; SC-003, SC-005
- **Verify**: `cargo test -p micold-core --test provider_origin --test provider_origin_docs` and `cargo test -p micold-daemon --test catalog_adoption`; quickstart Part B step 8
- **Depends on**: M1
- **Tier**: full

### M4 — Fold a busy tree

- **Tasks**: T032–T037, T041
- **Deliverable**: The user collapses an origin to hide its children, sees the child count and an attention marker, and the fold survives an app restart.
- **Satisfies**: US4 acceptance scenarios 1–3; FR-011, FR-017
- **Verify**: `cargo test -p micold-client --test sidebar_tree`, `cargo test -p micold-core --test settings_roundtrip`; quickstart Part B steps 3, 4
- **Depends on**: M2
- **Tier**: full
