# Implementation Plan: A session keeps its name when nothing is running it

**Branch**: `feat/the-name-of-session-should-be-always-visible-even-when-not-loaded` | **Date**: 2026-09-12 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/029-persistent-session-names/spec.md`

## Summary

A session's name is observed but never written down. The daemon reads the AI CLI's OSC-0 terminal
title on the supervisor tick, keeps it in the live registry as `LiveSession::last_title`, and
projects it onto the wire snapshot in `overlay_live_summaries` — a *projection*, discarded with the
process. The persisted field it should be feeding (`StoredSession::title`, already round-tripping
through `store.rs`) is only ever written for a session the discovery pass adopted, so every session
this application started itself is persisted with `title: None` and comes back as `SessionLabel::Pending`
— "New session" — until something runs it again.

The fix is two writes the daemon is already positioned to make, and no new storage:

1. **Record what is displayed.** When `drain_signals` observes a session's title change, persist it
   to the catalog (FR-001, FR-003, FR-005). The projection then agrees with the record instead of
   replacing it, and a restart shows the same name.
2. **Recover what was never recorded.** In the attach-time blocking hop that already refreshes
   worktrees and discovers external sessions, fill in the name of any *known* session still labelled
   `Pending` from that session's own AI CLI records, and persist it (FR-006, FR-007, FR-010). This
   is what gives the reporter's existing sessions their names back without opening them.

No wire-protocol change, no client change, no storage-schema change, and no `schema_version` bump:
`SessionSummary.title` already carries `SessionLabel` and the client already adopts it on every
snapshot reconcile.

## Technical Context

**Language/Version**: Rust, pinned to `stable` by `rust-toolchain.toml` (both `mise run` and bare `cargo`).

**Primary Dependencies**: `serde` / `serde_json` (persisted catalog), `tokio` (daemon supervisor + blocking hops), `uuid`. `iced` is untouched — this feature adds no UI.

**Storage**: Local JSON only. The per-project state file written by `micold_core::store::JsonFileStore`, of which the daemon is the single writer. `StoredSession::title: Option<String>` already exists and already round-trips.

**Testing**: `cargo test --workspace` via `mise run test`; `mise run test-core` for the render-free core while iterating. New coverage lands in `crates/micold-daemon/tests/` (integration, real `Catalog` over a temp data dir) and `crates/micold-core/tests/` (store round-trip).

**Target Platform**: Desktop — Linux, macOS, Windows.

**Project Type**: Desktop application; three-crate Cargo workspace (`micold-core`, `micold-client`, `micold-daemon`).

**Performance Goals**: The session list appears no slower than today for a project with 50 sessions (SC-007). The recovery pass adds at most one transcript read per *unnamed* known session per project open, and zero for a named one.

**Constraints**: Fully offline (Principle IV). Every durable write is best-effort — a failed write must not change what is displayed or interrupt a session (FR-009). The daemon remains the only writer of the catalog; the client must not gain a second write path. No `schema_version` bump.

**Scale/Scope**: Tens of sessions per project, a handful of projects; two AI CLIs (`claude`, `copilot`), both covered by the same rule.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-checked after Phase 1 design — see [Post-Design Re-check](#post-design-re-check).*

- [x] **I. Test-First (NON-NEGOTIABLE)**: PASS. Every changed path is render-free and reachable from `tests/` — `Catalog` methods, `DaemonState::drain_signals`/the recovery pass, and the `store.rs` round-trip. No GUI glue is touched, so the Principle I GUI exception is not invoked at all. Red-Green-Refactor per task, tests named in [quickstart.md](./quickstart.md).
- [x] **II. Multi-Session Support**: PASS. The name is per-session state keyed by `SessionId`, written through `Workspace::find_session_mut` so it can only ever reach the session it names. FR-012 (no session displays, inherits, or overwrites another's name) is covered by a dedicated test, not by inspection.
- [x] **III. Worktree Integration**: PASS. No worktree lifecycle change. The recovery pass enumerates locations exactly as `discover_external_sessions` already does — `SessionLocation::Default` plus each startable worktree — and resolves each session's directory through `SessionLocation::cwd`, the single authoritative implementation. No new or unmanaged location.
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: PASS. Reads and writes are local files only (the project state file, and the AI CLI's own on-disk records). Nothing leaves the device; the feature is fully functional offline.
- [x] **V. Rust + iced Stack**: PASS. Rust only, no framework change. "Named or not named" stays modelled as `SessionLabel::{Pending, Named}` rather than an `Option<String>` or an empty string, so "a session with a blank name" remains unrepresentable.
- [x] **VI. Cross-Platform Parity**: PASS. No platform-specific code — file I/O through existing abstractions, path derivation through the existing provider. CI runs all three platforms; the change touches source, so the documentation-only exemption does not apply.
- [x] **VII. Documentation First-Class**: PASS. User-visible behaviour changes, so `docs/user-guide/worktrees-and-sessions.md` gains a short subsection under *What the sidebar shows* explaining that a session's name is remembered, where it comes from, and when a row still reads "New session". Same change, verified by the docs build in CI.
- [x] **VIII. Reusable UI Component Foundation**: PASS (not engaged). No widget added, edited, or forked — the sidebar renders `SessionLabel::display()` exactly as it does today.

## Project Structure

### Documentation (this feature)

```text
specs/029-persistent-session-names/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/
│   └── session-name-persistence.md
├── checklists/
│   └── requirements.md  # /speckit-specify output
└── tasks.md             # /speckit-tasks output — NOT created here
```

### Source Code (repository root)

```text
crates/micold-core/
├── src/
│   ├── session.rs       # SessionLabel, Session::set_title — unchanged behaviour, re-used
│   ├── store.rs         # StoredSession::title — already round-trips; no schema change
│   ├── provider.rs      # AiCliProvider::read_title — re-used by the recovery pass
│   └── workspace.rs     # find_session_mut(id) -> (project, &mut Session) — the write path
└── tests/
    └── session_name_round_trip.rs      # NEW — a Named label survives save + load

crates/micold-daemon/
├── src/
│   ├── catalog.rs       # NEW Catalog::record_session_name / recover_session_names
│   ├── state.rs         # drain_signals returns observed name changes; recovery pass
│   └── server.rs        # supervisor tick persists them; attach hop runs recovery
└── tests/
    ├── session_name_persistence.rs     # NEW — observed name survives a catalog reload
    └── session_name_recovery.rs        # NEW — a Pending known session recovers its name

crates/micold-client/                   # UNCHANGED — already adopts SessionSummary.title

docs/user-guide/worktrees-and-sessions.md   # "What the sidebar shows" gains the naming rule
```

**Structure Decision**: The existing three-crate workspace, unchanged. All logic lands in
`micold-daemon` (the catalog's single writer, and the only process that observes a live title) on
top of primitives `micold-core` already exposes; `micold-client` is not touched, which is the
strongest available evidence that the fix is at the right seam.

## Post-Design Re-check

Re-evaluated after Phase 1 with the data model and contract in hand — all eight still PASS, with two
points worth recording:

- **Principle I** is stronger after design than before it: moving the durable write out of
  `drain_signals` into a caller-driven blocking hop (research R2) keeps the observation step
  lock-only and pure-ish, so the "what changed" decision stays directly unit-testable without a PTY.
- **Principle II** gained a concrete invariant during data modelling — *a name write is addressed by
  `SessionId`, never by index or by position in a project's session list* (data-model invariant 4) —
  which is what makes FR-012 testable rather than merely asserted.

No Complexity Tracking entries: the design adds no new crate, no new storage location, no new RPC,
and no new abstraction.
