# Implementation Plan: Support multiple session daemons: host and container

**Branch**: `feat/491_support-multiple-session-daemons-host-container` | **Date**: 2026-10-10 | **Spec**: [spec.md](./spec.md)

## Summary

Replace the single app-wide daemon (`Settings.daemon: DaemonConfig`, one `Placement`, one
`connection()` subscription, one fixed container `micold-sandbox` on port 7727) with a **registry of
named daemons** the client connects to concurrently. The client owns the registry and the
worktree-to-daemon **binding**; each daemon stays an unchanged, independent process with its own
catalog. Runtime is data (`DaemonRuntime`), so SSH (#687) and Kubernetes (#688) add a variant, not a
surface. No wire change: `PROTOCOL_VERSION` stays 39 (research R6). Design decisions and rejected
alternatives are in [research.md](./research.md); types in [data-model.md](./data-model.md);
interfaces in [contracts/](./contracts/); how to prove it in [quickstart.md](./quickstart.md).

## Technical Context

**Language**: Rust (stable, `rust-toolchain.toml`); iced for the Settings section.
**Dependencies**: no new crates (`serde`, `serde_json`, `tokio`, `iced` exist).
**Storage**: local only. Daemon list in `settings.json` (`Settings.daemons`, `Settings.next_daemon_id` and `Settings.legacy_default_daemon` (a `DaemonId`), additive defaulted fields; `settings_version` does not move, per the convention in `settings.rs`);
bindings in each project's own state file (`projects/<id>.json`, new optional field), both written
through the existing atomic `write_then_rename` path. Nothing remote.
**Testing**: `mise run test-core` (registry, migration, binding, naming rules, state machine),
`mise run test` (client reducers, daemon unchanged), `mise run image` + `mise run test-sandbox`
(real runtime, new `sandbox_real_multi_daemon*`), geometry gates for the Daemons section,
quickstart §B visual pass.
**Platforms**: Linux, macOS, Windows. Platform differences stay behind `pathmap`, `GitRouting` and
`Transport`; the registry and state machine are platform-free.
**Constraints**: one daemon failing must not touch another (per-daemon actor, per-daemon state, no
shared mutable connection); the app window never blocks on a daemon (each actor is its own
subscription); no concrete placement named outside the seams checked by
`micold-client/tests/no_concrete_implementations.rs`.
**Scale**: handfuls of daemons (2 to ~5); linear scans are fine.

## Constitution Check

| Principle | Result |
|---|---|
| I Test-First | PASS: each rule (name uniqueness, one-container-one-daemon, migration, removal -> "no daemon", state machine, routing of ops by binding) lands as a failing `micold-core`/render-free-client test first. Settings view glue uses the documented glue exception; its decisions live in `features/daemons.rs`. |
| II Multi-Session | PASS: sessions stay addressable by (daemon, session id); per-daemon actors, outboxes, catalogs; no shared state between daemons (FR-017, SC-006 test). |
| III Worktree Integration | PASS: worktrees stay app-managed; binding is app-owned; "no daemon" worktrees start nothing; Default (project root) location binds like a worktree (R4). |
| IV Local-First (NON-NEGOTIABLE) | PASS: registry and bindings are local files; endpoints are loopback/UDS only; no remote runtime shipped. |
| V Rust + iced | PASS: `DaemonRuntime` enum and `Binding` make "worktree on two daemons" and "session on an unknown daemon" unrepresentable (`Binding::{Bound(DaemonId), NoDaemon}`, newtypes `DaemonId` and `DaemonName`). |
| VI Cross-Platform | PASS: same code path on all three; Windows path-mapping difference surfaced in daemon state (R7); CI matrix unchanged plus the new sandbox job step. |
| VII Documentation | PASS: `docs/user-guide/settings.md`, `sandboxed-daemon.md`, `worktrees-and-sessions.md`, `agent-tools.md` updated in the same change (tasks). |
| VIII UI Components | PASS: Daemons rows built from existing settings-section and list-row components; new pieces (state badge, confirm dialog) reuse `ui/confirm_placement.rs` shape and the shared badge builder; any new widget goes to the shared library with a builder `.into()` API. |

No constitution violations; one deviation from the spec (host singleton) is recorded in Complexity Tracking.

## Requirement -> design map

| FR | Where |
|---|---|
| FR-001 | `micold-core/src/daemons.rs` (new): `DaemonRegistry`, `DaemonEntry`, `DaemonId`, `DaemonName`, `DaemonRuntime`; uniqueness in `DaemonRegistry::add/edit` |
| FR-002, FR-007 (merge) | `catalog_sync.rs::reconcile_catalog` per daemon, R11; core tests: two snapshots for one project, A's sessions survive B's snapshot |
| FR-002, FR-007 (connection) | `micold-client/src/daemon.rs`: `connection(placement)` becomes one subscription per `DaemonEntry` (`Subscription::batch`), keyed by `DaemonId`; `RECONNECT_BACKOFF` per actor |
| FR-003, FR-006, FR-014 | `micold-core/src/store.rs` project state gains `bindings: BTreeMap<location key, DaemonId>`; absent key resolves to `legacy_default_daemon` while that daemon is registered, else `NoDaemon`; `Binding` in `daemons.rs` |
| FR-004 | `shell/daemon_sync.rs::send_op` resolves target outbox from the op's binding; `Outbox` per daemon in `features/connection.rs`; terminals exec through `ui/terminal.rs` using the bound daemon's container name (test: terminal target resolved by binding); catalog merge per R11 |
| FR-005 | sidebar worktree row label (`features/sidebar.rs`, `ui/`) shows daemon name; no-daemon and unavailable states |
| FR-008, FR-009 | `micold-core/src/daemons.rs` `DaemonState` (starting, connected, unreachable, version mismatch, stopped) folded from the per-daemon `ConnectionStatus`; mismatch from the existing handshake refusal (`connect.rs`) |
| FR-010, FR-011 | `ui/settings/daemons.rs` (new), `features/daemons.rs` (new reducer), live via `Message::Daemon(DaemonId, ..)`; logs/errors ring buffer per daemon |
| FR-012, FR-014 | `ui/confirm_daemon_removal.rs` (new, modelled on `confirm_placement.rs`); counts from bindings + running sessions |
| FR-013 | `settings.rs`: when `daemons` is absent, synthesise a registry of one daemon from `DaemonConfig` and set `legacy_default_daemon` to it; unbound legacy worktrees resolve to it even after more daemons are added (R5, test: legacy file, add second daemon, legacy worktrees still bound) |
| FR-015 | `DaemonView` projection has only name/runtime/endpoint/version/state/bound; runtime-specific fields live in `DaemonRuntime` payload, never read by list/binding code |
| FR-016 | worktree create/attach in `features/worktree_form.rs` + `core/worktree.rs` branch-in-use check already project-wide; refusal text gains the holder's daemon from bindings |
| FR-017 | per-daemon container name, port and whole state dir (catalog, home, history, token) (R3); core test: two daemons yield disjoint `MountSet`s; registry persisted atomically |
| FR-018 | `crates/micold-daemon/tests/sandbox_real_multi_daemon.rs` + fake AI CLI stand-in (R8); `mise run test-sandbox`; CI step in `ci.yml` sandbox job |
| FR-019 | platform-free core; `pathmap` Windows mapping reported in daemon state (R7) |

## Project Structure

### Documentation (this feature)

```text
specs/491-multi-session-daemons/
├── plan.md  research.md  data-model.md  quickstart.md
├── contracts/
│   ├── daemon-registry.md      # settings.json + project-state schema and migration
│   └── client-daemon-routing.md  # per-daemon actor, state, op routing, removal
└── tasks.md                    # /speckit-tasks (not created here)
```

### Source Code

```text
crates/micold-core/src/
├── daemons.rs            # NEW registry, DaemonId, DaemonName, DaemonRuntime, DaemonState, Binding
├── settings.rs           # Settings.daemons + legacy_default_daemon, migration from DaemonConfig
├── store.rs              # per-project bindings in StoredProjectState
├── sandbox/mod.rs        # container name / port / state dir become per-daemon inputs (MountSet, host_token_path, with_history callers)
├── sandbox/placement.rs  # Placement::resolve takes a DaemonEntry's runtime
└── endpoint.rs           # DialAddress::Loopback{port} already per-address; host stays UDS
crates/micold-client/src/
├── daemon.rs             # one actor subscription per daemon, per-daemon Outbox; removes the DEFAULT_SANDBOX_PORT fallback (:249) in favour of the entry's port
├── catalog_sync.rs       # reconcile_catalog becomes per-daemon (R11)
├── ui/terminal.rs        # container exec target (CONTAINER_NAME, :398) taken from the bound daemon's entry
├── features/daemons.rs   # NEW reducer: list, edit draft, state, logs, removal flow
├── features/connection.rs# ConnectionStatus per daemon
├── shell/{daemon_sync,sandbox,persist,connection}.rs  # route by daemon; per-daemon bring-up
└── ui/settings/daemons.rs, ui/confirm_daemon_removal.rs   # NEW
crates/micold-daemon/     # unchanged logic; tests only
docs/user-guide/          # settings.md, sandboxed-daemon.md, worktrees-and-sessions.md, agent-tools.md
```

**Structure Decision**: all rules in `micold-core` (iced-free) and render-free client reducers;
`ui/` and `main.rs` stay glue (Principle I exception).

## Test strategy

| Requirement | Layer |
|---|---|
| FR-001, 003, 006, 013, 014, 015, 016 | core unit (`daemons.rs`, `settings.rs` round-trip with a pre-feature file, and a test that a file with `daemons` present is never re-migrated, `store.rs`) |
| FR-002, 004, 007, 008, 009 | client reducer tests + per-daemon actor with stand-in transports (`tests/`), no real runtime |
| FR-010, 011, 012, 005 | client feature reducers; geometry gates for the section and dialog; quickstart §B visual pass |
| FR-017, 019 | core unit (persistence), cross-platform CI matrix |
| FR-018, SC-001, SC-002, SC-006 | `sandbox_real_multi_daemon*` real runtime + fake AI CLI |

## Complexity Tracking

| Deviation | Why | Alternative rejected because |
|---|---|---|
| At most one host daemon | The host daemon is a per-user singleton: `endpoint::resolve` yields one UDS path and one lock file per user | A second host daemon needs a second endpoint and lock, a change to host spawn and `spawn.rs` that serves no stated use case; container daemons (the real multi-instance case) are uncapped. Narrows the clarified "several of one runtime" for host only; recorded in R3 |
