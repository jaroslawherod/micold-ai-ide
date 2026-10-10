# Tasks: Support multiple session daemons: host and container

**Input**: `specs/491-multi-session-daemons/` (spec.md, plan.md, research.md, data-model.md, contracts/, quickstart.md)
**Tests**: mandatory (Constitution I). Every test task precedes the implementation it covers and must be seen failing first.
**Format**: `- [ ] T### [P?] [Story?] description with path`. `[P]` = different files, no dependency on an unfinished task.

Paths: core = `crates/micold-core/src`, client = `crates/micold-client/src`, daemon tests = `crates/micold-daemon/tests`.
Core tests live in `crates/micold-core/tests/`, client tests in `crates/micold-client/tests/`.

## Phase 1: Setup

- [ ] T001 Add module `daemons` (empty `DaemonId`, `DaemonName`, `DaemonRuntime`, `DaemonEntry`, `DaemonRegistry`, `Binding` stubs, `#[non_exhaustive]` runtime enum) in `crates/micold-core/src/daemons.rs` and export it from `crates/micold-core/src/lib.rs`
- [ ] T002 [P] Add fixtures: a pre-feature `settings.json` with `placement: host_process` and one with `placement: local_sandbox` plus a project state file without `bindings`, in `crates/micold-core/tests/fixtures/daemons/`

## Phase 2: Foundational (blocks every story)

Tests first.

- [ ] T003 [U1][U2][U3][U4][U5][U6][U7][U8][U9] [P] Registry rules test in `crates/micold-core/tests/daemons_registry.rs`: name trimmed, non-empty, unique case-insensitively (`DuplicateName`, `BlankName`); ids unique and never reused after removal (remove "X", add "X": new id); rename keeps the id; container name and port unique (`ContainerInUse{other}`); image present and valid, port in range (`InvalidField{field}`); at most one `host` entry (`HostExists`); `DaemonRuntime::label()` plain value (FR-001, FR-015)
- [ ] T004 [U10][U11][U12][U13][U14] [P] Migration test in `crates/micold-core/tests/daemons_migration.rs`: host fixture yields one `Host` entry and `legacy_default_daemon = "Host"`; container fixture yields one container entry carrying the legacy name `micold-sandbox`, port 7727 and the profile; a document with `daemons` present is never re-migrated; `settings_version` unchanged; the `daemon` key is still written; the migrated daemon has id 1 and `next_daemon_id` is 2; unknown `kind` loads as `DaemonRuntime::Unsupported` (state Stopped, reason "unsupported runtime", cannot start, removable) and survives save verbatim (FR-013, R5)
- [ ] T005 [U15][U16][U17][U18][U19][U20] [P] Binding test in `crates/micold-core/tests/daemons_bindings.rs`: `bindings: BTreeMap<dir_name, DaemonId>` with reserved key `""` for Default round-trips through project state; absent key resolves to `legacy_default_daemon` while it is registered, else `Binding::NoDaemon`; a key naming an unknown id reads `NoDaemon`; removing daemon "X" then adding a new "X" does not rebind its worktrees; renaming keeps bindings and the legacy default; adding a second daemon does not unbind legacy worktrees; removing an entry writes nothing to disk (FR-003, FR-006, FR-014)
- [ ] T006 [U21][U22] [P] Per-daemon isolation test in `crates/micold-core/tests/daemons_isolation.rs`: two container daemons yield disjoint `MountSet`s, distinct state dirs `<state_dir>/daemons/<slug>/`, distinct tokens and container names; the migrated daemon keeps the legacy name, port and token path (FR-017, R3)
- [ ] T007 [U1][U2][U3][U4][U5][U6][U7][U8][U9] Implement `DaemonId` (`u32`, from `Settings.next_daemon_id`, never reused), `DaemonName`, `DaemonRuntime` (`Host`, `Container(ContainerSettings{profile, container_name, port})`, `Unsupported{kind}`), `DaemonEntry{id, name, runtime, auto_start}`, `DaemonRegistry::{add, edit, remove}` with `RegistryError`, `Binding`, and `DaemonRuntime::label()` in `crates/micold-core/src/daemons.rs`, serde `snake_case`, every new field `#[serde(default)]` (T003)
- [ ] T008 [U10][U11][U12][U13][U14] Add `Settings.daemons`, `Settings.next_daemon_id` and `Settings.legacy_default_daemon` (a `DaemonId`; additive, defaulted; no `settings_version` bump) and the one-time migration from `DaemonConfig` in `crates/micold-core/src/settings.rs`; keep writing `daemon` mirroring the first entry (T004)
- [ ] T009 [U15][U16][U17][U18][U19][U20] Add `bindings` (omitted when empty) to the per-project stored state and `Binding` resolution in `crates/micold-core/src/store.rs`, written through the existing atomic `write_then_rename` (T005)
- [ ] T010 [U21][U22] Make container name, port and whole state dir per-daemon inputs in `crates/micold-core/src/sandbox/mod.rs` (`MountSet`, `host_token_path`, `with_history` callers) and let `Placement::resolve` take a `DaemonEntry`'s runtime in `crates/micold-core/src/sandbox/placement.rs` (T006)

**Checkpoint**: core has a registry that migrates, binds and isolates; nothing in the client uses it yet.

## Phase 3: User Story 4 (P2) and User Story 1 scenarios 2 and 4 (P1): the registry drives the app, bindings chosen and shown

**Goal**: an upgraded install opens with one daemon holding the previous placement, every worktree bound to it; each worktree row names its daemon and a new worktree's daemon is picked and kept across restarts.
**Independent test**: load a pre-feature settings file in the client state; assert one daemon, all worktrees bound, labels shown, choice persisted.

- [ ] T011 [A1] [P] [US4] Client upgrade test in `crates/micold-client/tests/upgrade_single_daemon.rs`: host and container fixtures give a client state with one daemon, every project/worktree/Default location bound to it, no prompt, no new credential sharing, close and reopen keeps daemons, bindings and settings (US4 scenarios 1 to 4)
- [ ] T012 [A2] [P] [US1] Sidebar label test in `crates/micold-client/tests/worktree_daemon_label.rs`: each worktree row projects its daemon name; a no-daemon worktree shows "no daemon"; two worktrees of one project on different daemons are both listed (US1 scenario 2, FR-005)
- [ ] T013 [A3] [P] [US1] Worktree form test in `crates/micold-client/tests/worktree_form_daemon_choice.rs`: create and attach offer every registry daemon, the choice is stored as a binding and survives reload; with an empty registry no runtime is offered; refusal for a branch or name already in use names the holder's worktree and daemon (US1 scenario 4, FR-006, FR-016)
- [ ] T014 [A1] [US4] Load the registry in client state and resolve a location's daemon through `Binding` in `crates/micold-client/src/shell/settings.rs`, `crates/micold-client/src/shell/persist.rs` and `crates/micold-client/src/features/settings.rs`; keep the single actor/placement path reading the resolved entry for now (T011)
- [ ] T015 [A2] [P] [US1] Show the daemon name on each worktree row and the "no daemon" state in `crates/micold-client/src/features/sidebar.rs` (T012)
- [ ] T016 [A3] [US1] Daemon picker on create/attach and binding write for new worktrees and Default in `crates/micold-client/src/features/worktree_form.rs` and `crates/micold-client/src/shell/persist.rs`; holder's daemon in the refusal text in `crates/micold-core/src/worktree.rs` (T013)
- [ ] T017 [P] [US4] User guide: document that the existing setup migrates to one daemon, worktree labels and the daemon choice, in `docs/user-guide/worktrees-and-sessions.md` and `docs/user-guide/settings.md`

**Checkpoint (M1)**: upgrading keeps today's setup; the app resolves worktrees through the registry and shows their daemon.

## Phase 4: User Story 1 scenarios 1 and 3 (P1) and User Story 2 (P1): concurrent daemons, isolated failures

**Goal**: host and container daemons connected at once; sessions and terminals run on the bound daemon; one daemon failing, mismatching or restarting leaves others untouched.
**Independent test**: two registry entries (host, container) with stand-in transports; stop one; the other keeps its sessions and input.

- [ ] T018 [P] [US2] State machine test in `crates/micold-core/tests/daemon_state.rs`: the transition table of `contracts/client-daemon-routing.md` (Stopped, Starting, Connected, Unreachable after the 3-failure debounce, VersionMismatch with client and daemon versions, any to Stopped; an established connection lost goes to Unreachable at once, a failing dial after at most 3 x `RECONNECT_BACKOFF`, inside SC-002's 5 s; a Stopped daemon (user-stopped or `auto_start` false) is neither spawned nor reconnected); a transition on one daemon yields no event for another (FR-009, FR-007)
- [ ] T019 [P] [US2] Per-daemon actor test in `crates/micold-client/tests/per_daemon_actors.rs`: one subscription per `DaemonEntry` keyed by `DaemonId`; editing one entry's runtime settings restarts only its actor and a rename restarts nothing; one transport failing does not change another's status, outbox or backoff; the window never waits on a daemon (FR-002, FR-007)
- [ ] T020 [P] [US1] Routing test in `crates/micold-client/tests/daemon_routing.rs`: `send_op` resolves the op's binding; `Bound(d)` and Connected goes to `d`'s outbox; Bound but not Connected refused "daemon <d> unavailable"; `NoDaemon` refused "no daemon"; never falls back to another daemon; the terminal exec target comes from the bound daemon's container name; first bind to daemon d, and each (re)connect of d, sends `ProjectAdd` for projects absent from d's catalog, and a bind while d is down is kept and registers on connect (FR-004, US1 scenario 3)
- [ ] T021 [P] [US2] Catalog merge test in `crates/micold-client/tests/catalog_two_daemons.rs`: two snapshots for one project; A's sessions survive B's snapshot; worktree list is the union with status taken from the bound daemon; provenance and display-name removal scoped to the snapshot's daemon; a lost daemon's sessions show "lost to its daemon", not finished (R11, FR-007)
- [ ] T022 [P] [US2] Version mismatch test in `crates/micold-client/tests/daemon_version_mismatch.rs`: client newer and daemon newer both mark only that daemon `VersionMismatch` with both versions and the way out; others stay Connected (US2 scenario 3, FR-008)
- [ ] T048 [P] [US2] Unavailable-label test in `crates/micold-client/tests/worktree_unavailable_label.rs`: rows bound to a non-Connected daemon show the strings of `contracts/client-daemon-routing.md` ("daemon <name> unavailable", version-mismatch text, "lost to daemon <name>", "container runtime not found", "paths are mapped" for a path-mapping container daemon), rows on other daemons are unchanged, a state change reaches the row within one reducer step (SC-002, R7, FR-019)
- [ ] T049 [P] [US2] Adopt test in `crates/micold-client/tests/daemon_adopt_on_restart.rs`: at launch a daemon that is already running is reconnected, not respawned or recreated, and `auto_start` false never spawns (spec: restart edge case)
- [ ] T023 [US2] Implement `DaemonState` (`Starting | Connected | Unreachable{reason} | VersionMismatch{client, daemon} | Stopped`, version, endpoint) and the transition fold in `crates/micold-core/src/daemons.rs` (T018)
- [ ] T024 [US1] Turn `connection(placement)` into a `Subscription::batch` with one actor per `DaemonEntry` keyed by `DaemonId`, per-daemon `Outbox` and `RECONNECT_BACKOFF`; replace the `DEFAULT_SANDBOX_PORT` fallback with the entry's port, in `crates/micold-client/src/daemon.rs` (T019)
- [ ] T025 [US2] Per-daemon `ConnectionStatus` and `Message::Daemon(DaemonId, DaemonEvent)` folded by `crates/micold-client/src/features/connection.rs` and `crates/micold-client/src/shell/connection.rs`; per-daemon bring-up (adopting an already-running daemon, never respawning a Stopped one) and container name in `crates/micold-client/src/shell/sandbox.rs` (T019, T022, T049)
- [ ] T026 [US1] Route ops by binding in `crates/micold-client/src/shell/daemon_sync.rs` (`send_op`), send `ProjectAdd` to the bound daemon on first bind and on each connect, and use the bound daemon's container name for terminals in `crates/micold-client/src/ui/terminal.rs` (T020)
- [ ] T027 [US2] Make `reconcile_catalog` per daemon in `crates/micold-client/src/catalog_sync.rs` (T021)
- [ ] T028 [US2] Show "daemon unavailable" and version-mismatch text on worktrees bound to a non-Connected daemon in `crates/micold-client/src/features/sidebar.rs`, others unchanged (T048); explain Windows mapped paths in daemon state per R7 (tests T048)
- [ ] T029 [P] [US1] User guide: running host and container daemons together, unavailable states, per-daemon tool server limitation (R9), and how to add a second daemon by editing the `daemons` array in `settings.json` until Settings gains the section (M3), in `docs/user-guide/sandboxed-daemon.md`, `docs/user-guide/settings.md` and `docs/user-guide/agent-tools.md`

**Checkpoint (M2)**: two daemons from the registry run concurrently and fail independently.

## Phase 5: User Story 3 (P2): manage daemons in Settings

**Goal**: Settings, Daemons lists every daemon live and supports add, edit, start, stop, reconnect, remove and rebind.
**Independent test**: add a container daemon, watch Starting then Connected without reopening Settings, stop it, remove it.

- [ ] T030 [P] [US3] Daemons reducer test in `crates/micold-client/tests/daemons_feature.rs`: add/edit validation names the field (blank name, duplicate, invalid image or port, container in use naming the other daemon, second host daemon); start, stop, reconnect apply to one daemon only; `Message::Daemon` state changes update the row live; row fields are name, runtime label, endpoint, version, state, bound worktrees for both runtimes; a daemon's recent connection errors and log tail are exposed for its details view and runtime-missing carries its reason (FR-010, FR-011, FR-015, US3 scenario 4)
- [ ] T031 [P] [US3] Removal flow test in `crates/micold-client/tests/daemon_removal.rs`: confirmation states worktree and running-session counts and "nothing on disk is deleted"; cancel changes nothing; confirm stops its sessions and actor, removes the entry, bindings resolve `NoDaemon`, no worktree directory touched; a `NoDaemon` worktree starts no session or terminal until bound by an explicit action; removing an unreachable or stopped daemon shows its sessions as ended by removal and never touches the container; the "Bind to daemon..." action binds an id and enables sessions; editing a running daemon's runtime settings asks the same counted confirmation, stops its sessions and restarts only its actor, a rename restarts nothing; with no daemons the empty state and add action are shown and the form offers no runtime (FR-011, FR-012, FR-014, US3 scenarios 5 to 7)
- [ ] T032 [P] [US3] Geometry gates for the Daemons section and the removal dialog in `crates/micold-client/tests/daemons_section_geometry.rs`, modelled on the existing settings geometry tests
- [ ] T033 [US3] Reducer `crates/micold-client/src/features/daemons.rs`: list, edit draft, state, per-daemon connection-error ring buffer and log tail, the `DaemonView` projection (`{id, name, runtime_label, endpoint, version, state, bound}` in `crates/micold-core/src/daemons.rs`), removal flow, rebind action (T030, T031)
- [ ] T034 [US3] Settings section `crates/micold-client/src/ui/settings/daemons.rs` registered in `crates/micold-client/src/ui/settings/mod.rs`, replacing `crates/micold-client/src/ui/settings/daemon.rs`, built from existing settings-section and list-row components (T032)
- [ ] T035 [P] [US3] Confirmation dialog `crates/micold-client/src/ui/confirm_daemon_removal.rs`, modelled on `crates/micold-client/src/ui/confirm_placement.rs`, wired in `crates/micold-client/src/app.rs` (T031)
- [ ] T036 [US3] "Bind to daemon..." action for a "no daemon" worktree in `crates/micold-client/src/features/sidebar.rs` and `crates/micold-client/src/features/worktree.rs` (T031)
- [ ] T037 [US3] Empty-state view (no daemons: explanation and add action) in `crates/micold-client/src/ui/settings/daemons.rs` and the worktree form in `crates/micold-client/src/features/worktree_form.rs`, after T034 and T016 (T031)
- [ ] T038 [P] [US3] User guide: the Daemons section, add/edit/remove and what removal does, in `docs/user-guide/settings.md` and `docs/user-guide/sandboxed-daemon.md`
- [ ] T039 [US3] Run the `visual-pass` skill on quickstart Part B items 1 to 4 and save the evidence under `specs/491-multi-session-daemons/` (needs eyes; geometry gates cannot judge it)

**Checkpoint (M3)**: the daemon list is managed entirely in the app.

## Phase 6: User Story 5 (P2): end-to-end CI against a real runtime

**Goal**: container path, host path and multi-daemon isolation proven with a scripted stand-in for the AI CLI.
**Independent test**: `mise run image && mise run test-sandbox` passes with Docker or Podman.

- [ ] T040 [US5] Scripted fake AI CLI stand-in (mounted directory on PATH, not the shipped image) and a helper to start N container daemons with distinct names, ports and state dirs, in `crates/micold-daemon/tests/sandbox_real_support/mod.rs` (R8)
- [ ] T041 [P] [US5] Container scenario in `crates/micold-daemon/tests/sandbox_real_multi_daemon_container.rs`: session starts, produces output, accepts input, behind `sandbox-real-runtime`
- [ ] T042 [P] [US5] Host scenario in `crates/micold-daemon/tests/sandbox_real_multi_daemon_host.rs`: same scenario against the host daemon with the same stand-in
- [ ] T043 [US5] Multi-daemon scenario in `crates/micold-daemon/tests/sandbox_real_multi_daemon.rs`: host and container bound worktrees, stop the container daemon, host session still accepts input, container worktrees report unreachable within 5 s, and the test fails if the failure leaks (SC-001, SC-002, SC-006)
- [ ] T044 [P] [US5] Parity test in `crates/micold-client/tests/daemon_rows_uniform.rs`: a host and a container daemon project to the same `DaemonView` fields and bindings carry no runtime-specific fields (US5 scenario 5, FR-015)
- [ ] T045 [US5] Confirm `.github/workflows/ci.yml` sandbox job's `sandbox_real_` filter runs the new files, add a step if it does not, and document the on-demand `mise run test-sandbox` in `docs/development/` where the sandbox suite is described, stating that the e2e runs on the Linux runner only and other platforms rest on the render-free tests (FR-018)

**Checkpoint (M4)**: the sandbox suite proves host, container and both together.

## Phase 7: Polish and cross-cutting (docs only, done in the close unit)

- [ ] T046 [P] Update any `docs/` architecture text that describes a single app-wide daemon to the registry model; grep `docs/` for "the daemon" placement descriptions
- [ ] T047 Record quickstart Part A and Part B results in `specs/491-multi-session-daemons/quickstart.md` and tick the tasks

## Dependencies and order

- Phase 1 then 2 (T007 to T010 depend on their tests T003 to T006) block all stories.
- Phase 3 (M1) needs Phase 2. Phase 4 (M2) needs Phase 3 (T014 resolved bindings). Phase 5 (M3) needs Phase 4 (`Message::Daemon`, states). Phase 6 (M4) needs Phase 4 and runs against the real runtime; T044 needs Phase 5's `DaemonView`.
- Within a phase: test tasks, then the implementation they name in parentheses.

## Parallel examples

- Phase 2: T003, T004, T005, T006 together (disjoint files); then T007, T008 (different files), T009, T010.
- Phase 4: T018 to T022, T048, T049 together; T029 beside any implementation task.
- Phase 5: T030, T031, T032 together; T035 and T038 beside T033; T037 after T034.

## Implementation strategy

MVP is M1: the registry, migration and bindings with the app running on one migrated daemon, so upgrading is a no-op for users and the model exists for everything after. Ship M2 next (the feature's reason to exist), then Settings (M3), then CI proof (M4). Each milestone keeps `main` green; until M3 a second daemon is added by editing `settings.json` (documented in M2's guide), so no half-wired UI ships.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Registry, migration and bindings drive the app 🎯 MVP

- **Tasks**: T001–T017
- **Deliverable**: an upgraded install (host or container) opens with one daemon in the registry holding the previous settings, every worktree bound to it and labelled with its name, and a new worktree's daemon choice persists across restarts.
- **Satisfies**: US4 acceptance scenarios 1–5; US1 acceptance scenarios 2, 4; FR-001, FR-003, FR-005, FR-006, FR-013, FR-016, FR-017 (persistence, disjoint mounts), FR-019 (migration on all three platforms)
- **Verify**: `mise run test-core` (daemons_registry, daemons_migration, daemons_bindings, daemons_isolation) and `cargo test -p micold-client --test upgrade_single_daemon --test worktree_daemon_label --test worktree_form_daemon_choice`
- **Depends on**: —
- **Tier**: full

### M2 — Host and container daemons run concurrently and fail independently

- **Tasks**: T018–T029, T048–T049
- **Deliverable**: with a host and a container entry in `settings.json`, both daemons connect at once, sessions and terminals run on each worktree's bound daemon, and stopping or mismatching one leaves the other's sessions untouched, with unavailable and version-mismatch states shown on its worktrees.
- **Satisfies**: US1 acceptance scenarios 1, 3; US2 acceptance scenarios 1–4; FR-002, FR-004, FR-007, FR-008, FR-009, FR-019 (state detail)
- **Verify**: `cargo test -p micold-core --test daemon_state` and `cargo test -p micold-client --test per_daemon_actors --test daemon_routing --test catalog_two_daemons --test daemon_version_mismatch --test worktree_unavailable_label --test daemon_adopt_on_restart`
- **Depends on**: M1
- **Tier**: full

### M3 — Settings Daemons section

- **Tasks**: T030–T039
- **Deliverable**: Settings → Daemons lists every daemon with live state and bound worktrees and lets the user add, edit, start, stop, reconnect and remove one, with a counted confirmation and an explicit rebind for "no daemon" worktrees.
- **Satisfies**: US3 acceptance scenarios 1–7; FR-010, FR-011, FR-012, FR-014, FR-015
- **Verify**: `cargo test -p micold-client --test daemons_feature --test daemon_removal --test daemons_section_geometry`; quickstart Part B items 1–4 (visual-pass evidence from T039)
- **Depends on**: M2
- **Tier**: full

### M4 — End-to-end proof against a real runtime

- **Tasks**: T040–T045
- **Deliverable**: `mise run test-sandbox` runs container, host and multi-daemon scenarios with a scripted AI CLI stand-in, and CI runs them.
- **Satisfies**: US5 acceptance scenarios 1–5; FR-015, FR-018; SC-001, SC-002, SC-006
- **Verify**: `mise run image && mise run test-sandbox` (the `sandbox_real_multi_daemon*` tests pass)
- **Depends on**: M2, M3
- **Tier**: full
