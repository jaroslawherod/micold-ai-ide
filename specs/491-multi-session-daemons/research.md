# Research: multiple session daemons

Facts below were read from the code at this commit (paths in `plan.md`).

## R1. Who owns the registry and the binding?
**Decision**: the client. `Settings.daemons` (settings.json) holds the list; each project's state
file holds its worktree bindings.
**Rationale**: a daemon cannot know of its siblings, and the client already owns `projects.json`
and `settings.json`; FR-007 requires that no daemon's state depends on another's.
**Rejected**: a "supervisor" daemon (single point of failure, violates FR-007); storing bindings in
each daemon's catalog (a removed daemon would take its bindings with it, FR-014).

## R2. One connection actor per daemon
**Decision**: `connection(placement)` (one `Subscription::run_with`) becomes a batch keyed by
`DaemonName`; each actor owns its backoff, handshake, `Outbox` and status.
**Rationale**: the actor is already self-contained (`daemon.rs` `actor`, `connect_and_pump`);
isolation falls out of independence.
**Rejected**: multiplexing several daemons through one connection (no such wire concept; breaks
isolation).

## R3. What must become per-daemon in the container runtime
Currently global: `CONTAINER_NAME = "micold-sandbox"`, `DEFAULT_SANDBOX_PORT = 7727`, token at
`host_token_path(state_dir)` (`sandbox.token`).
**Decision**: per container daemon: container name (default `micold-sandbox` for the migrated
daemon, `micold-sandbox-<slug>` for new ones), loopback port (default 7727, new ones chosen as next
free from 7728 and persisted), whole state dir `<state_dir>/daemons/<slug>/` (the container's catalog, HOME, history and token: `MountSet` mounts the state dir as a unit, so moving only the token would share one catalog between daemons).
Migration keeps the legacy name/port/token location for the migrated daemon so a running container
is adopted, not recreated (spec: reconnected, not restarted).
**Rejected**: ephemeral ports (the client must dial before the container exists, per `endpoint.rs`).
The host runtime is a singleton per user (UDS + lock via `endpoint::resolve`), so at most one host
daemon is addable; the spec's "several of one runtime" applies to containers. Recorded as a justified
deviation in plan.md Complexity Tracking.

## R4. Binding granularity and Default location
**Decision**: binding keys on the project's session location (`SessionLocation::Worktree(dir)` or
`Default`), stored per project. A project's first binding defaults to the daemon the user picks in
the add-project flow, else the migrated daemon.
**Rationale**: sessions carry `worktree_dir` / `None` already; Principle III keeps Default as the
only non-worktree location.

## R5. Migration
**Decision**: on load of a pre-feature `settings.json` (single `daemon` block), synthesise
`daemons = [DaemonEntry{ name: "Host" | "Container", runtime from placement kind + profile }]` and set
`legacy_default_daemon` to its name; a missing binding resolves to it, so no per-project rewrite is
needed and legacy worktrees stay bound when more daemons are added.
**Rejected**: inferring "the only daemon" (breaks on the second add); rewriting every project's
bindings at migration (a failed partial write leaves projects half migrated). The old `daemon` key is still written for one
release for rollback; the registry is authoritative when present.
No version bump (`settings_version` does not move for additive defaulted fields, `settings.rs`
convention). The gate is `daemons` present: a document that has it is never re-migrated.

## R6. Protocol
**Decision**: no wire change, `PROTOCOL_VERSION` 39 unchanged. Per-daemon version mismatch uses the
existing handshake refusal (`version_mismatch_resolves` tests) surfaced per actor.
**Rejected**: a daemon-name field in the handshake (a daemon need not know its name).

## R7. Windows/macOS path visibility
**Decision**: reuse `sandbox::pathmap` and `Placement::git_routing`; when a container daemon maps
paths (Windows), its `DaemonState` detail says "paths are mapped" (FR-019 edge case) and git for its
worktrees routes `ViaDaemon`. No new mechanism.

## R8. End-to-end tests
**Decision**: new `sandbox_real_multi_daemon*` in `micold-daemon/tests` using
`sandbox_real_support`, with a scripted `fake-ai-cli` on PATH (as `session_start.rs` stand-ins do)
baked into the test via a mounted directory, not the shipped image; behind
`sandbox-real-runtime`; run by `mise run test-sandbox` and the existing CI sandbox job.
**Rejected**: a real AI CLI (sign-in, cost).

## R9. Tool server (MCP)
Each daemon runs its own tool server over its own catalog; agents see only their daemon's
worktrees. Documented as a limitation in `agent-tools.md`; no cross-daemon tool is added.

## R10. Duplicate names across daemons (FR-016)
Branch/worktree uniqueness already holds project-wide in `core/worktree.rs` because all worktrees
share one repository; the only addition is naming the holder's daemon in the refusal.

## Risks to confirm during tasks
- Whether a project must be registered in each daemon's catalog before a session can start there
  (assumed: client issues the existing add-project op on first bind).
- Client-side git routing for a bound project when its daemon is down (read-only views degrade to
  the "unavailable" state, no fall back to another daemon, per rule P-2).

## R11. Merging N catalog snapshots
`catalog_sync.rs::reconcile_catalog` treats one daemon as the sole writer (full replace of a
project's sessions and worktree list). **Decision**: reconcile per daemon: sessions merge by
(daemon, id) and a snapshot only replaces sessions of its own daemon; the worktree list is the union
over daemons, each worktree's status/provenance/display name taken from its bound daemon's snapshot
(unbound ones from any daemon that lists them, shown "no daemon"). Provenance and display-name
removal is scoped to the snapshot's daemon.
**Rejected**: designating a "primary" daemon authoritative for worktrees (its outage would blank
other daemons' worktrees, violating FR-007).
