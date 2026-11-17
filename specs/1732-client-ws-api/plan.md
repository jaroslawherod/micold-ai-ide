# Implementation Plan: JSON-over-WebSocket client API and a working web client

**Branch**: `claude/project-thread-mezck0` | **Date**: 2026-11-17 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/1732-client-ws-api/spec.md` (reviewed, clarified CLEAN).

## Summary

Add a browser-facing API to the orchestrator and make the web client use it. A new listener
(`--api-addr`, off by default) serves `/api/ws`: JSON text messages with client-chosen request ids, an
application-level ping/pong, an `Origin` allow-list checked in the upgrade callback, and a sequenced change
feed with resume. The message types are new, dedicated Rust types in `micold-core` (`client_api`), not the
daemon protocol's `ClientMsg` (binary-hash-bearing, byte arrays, many daemon-only variants). The orchestrator
answers them from what it already has: `list::gather` (#1726) for daemons, projects, worktrees and sessions,
and a one-shot conversation on the owning daemon's link for session content (a new read-only daemon request, R9), behind a `LinkDirectory` trait
implemented in-process (FR-007). A single poller diffs successive snapshots into sequenced changes held in a
bounded ring (resume); the ordering is the orchestrator's existing stable order, sent computed (FR-012).
`schemars` derives a checked-in JSON Schema from the API types; `json-schema-to-typescript` turns it into the
checked-in TypeScript types that replace the #1735 stand-in. In the web client a `RealOrchestratorApi`
implements the existing `OrchestratorApi` over a browser `WebSocket` Transport and the existing
`ConnectionManager`; the mock stays for tests and the mock-only dev command. `mise run local-stack` starts the
orchestrator, one daemon on a seeded data directory and the web client; Playwright in Chromium, Firefox and
WebKit drives it. No iced code changes. One daemon wire change: `ClientMsg::ReadStructuredEvents` (R9), with a `PROTOCOL_VERSION` bump.

## Technical Context

**Language/Version**: Rust (workspace edition, MSRV 1.97); TypeScript ~5.9 (Expo 58 / React Native Web).

**Primary Dependencies**: Rust: `tokio-tungstenite` 0.29 (new to `micold-daemon`'s manifest, already in
`Cargo.lock` through `kube`'s `ws` feature, features `handshake` only), `schemars` 1.x (new crate, `micold-core`,
derive only). Existing: `tokio`, `serde`/`serde_json`, `futures-util`, `micold_core::orchestrator::list_merge`.
TypeScript: `json-schema-to-typescript` (new dev dependency of `@micold/protocol`, generation only); existing
`zustand`, `@formatjs/intl`, Playwright.

**Storage**: none new. The change log is in memory; an orchestrator restart is a resync by design (FR-003).

**Testing**: `cargo test` core units (diff, ring, classification, origin match, schema current); `micold-daemon`
integration tests in `tests/orchestrator/ws_*.rs` (real listener, real `tokio-tungstenite` client); jest
(`packages/protocol`, `apps/client`); Playwright in three browsers; quickstart Part B visual pass.

**Target Platform**: Orchestrator and daemons Linux (also builds elsewhere); web client current Chromium,
Firefox, Safari.

**Project Type**: workspace (core + daemon) plus the pnpm web client.

**Performance Goals**: SC-006 change visible within 2 s (poll 1 s); SC-001 stack up and a session listed in 60 s;
SC-002 100 changes over 10 reconnects exactly once.

**Constraints**: API listener loopback-only (non-loopback without TLS is refused at startup); no auth (#1733/#1734);
no terminal bytes (FR-006); the poller runs only while a subscriber exists.

**Scale/Scope**: a few browser tabs per orchestrator; ring of 1024 changes.

## Constitution Check

- [x] **I. Test-First**: every task starts with a failing test (list in "Test strategy"); tdd-plan derives it.
- [x] **II. Multi-Session Support**: sessions stay addressed by daemon/project/location/session id; each socket has
  its own subscription and sequence position; content reads are one-shot and keep no per-session state.
- [x] **III. Worktree Integration**: read-only; locations are the worktrees the daemons already hold; no git operation.
- [x] **IV. Local-First Storage**: the listener is off unless `--api-addr` is given and loopback-only without TLS; the
  local stack uses the SQLite store and no outside service; nothing leaves the machine.
- [x] **V. TypeScript + React Native Client, Rust Core**: the client reaches the orchestrator only over `/api/ws`
  through types generated from Rust (research R3); the API, poller and diff are Rust.
- [x] **VI. Linux Daemons, Web Client Reach**: browser-specific code is the single `BrowserTransport` seam
  (close codes, visibility, throttled timers); Playwright runs Chromium, Firefox, WebKit (FR-018).
- [x] **VII. Documentation First-Class**: `docs/` user-guide page for the web client and local stack, config reference
  for `--api-addr` / `--allow-origin` / `MICOLD_ORCHESTRATOR_*`, in the same PRs.
- [x] **VIII. Reusable UI Component Foundation**: the content view and unreachable marker reuse the existing shared
  primitives and `ConnectionBanner`; anything new goes into the shared component folder with typed props.

Post-design re-check: no violation; Complexity Tracking empty.

## Requirement map

| Requirement | Where |
|---|---|
| FR-001, FR-008 | contracts/ws-protocol.md; `orchestrator/ws.rs`; research R1, R2 |
| FR-002 | contracts/ws-protocol.md (list_daemons, list_sessions, read_session, subscribe); R8 |
| FR-003 | data-model.md *ChangeLog*; R5 |
| FR-004 | `orchestrator/origin.rs`, R1 (upgrade callback), contracts *Origin* |
| FR-005 | contracts *Heartbeat*; R7 |
| FR-006 | contracts *Errors* (`terminal_not_supported`) |
| FR-007 | `LinkDirectory`, R9 |
| FR-009, FR-010 | R3; `scripts/api-types.sh` |
| FR-011 | `client_api::classify` single table, R4 |
| FR-012 | R10 |
| FR-013, FR-014, FR-014a, FR-014b | `RealOrchestratorApi`, R6, data-model *ClientConnection* |
| FR-015 | `apps/client/src/i18n/{en,pl}.json`; existing catalog completeness tests |
| FR-016, FR-017, FR-018 | R11, R12; quickstart |

## Project Structure

### Documentation (this feature)

```text
specs/1732-client-ws-api/
├── plan.md  research.md  data-model.md  quickstart.md
├── contracts/ws-protocol.md
└── tasks.md   # tasks unit
```

### Source Code

```text
crates/micold-core/src/
├── client_api/                  # NEW: wire types (JsonSchema), classify table, diff, ChangeLog, origin match
│   ├── mod.rs messages.rs classify.rs diff.rs log.rs origin.rs
└── bin/micold-api-schema.rs     # NEW: writes the schema (pattern of micold-tokens-ts.rs)
crates/micold-daemon/src/orchestrator/
├── ws.rs                        # NEW: listener, upgrade callback, per-socket session, heartbeat
├── directory.rs                 # NEW: LinkDirectory trait + InProcess impl (list, read_session)
├── feed.rs                      # NEW: poller task: gather, diff, ChangeLog, broadcast
├── config.rs                    # --api-addr, --allow-origin, env twins
crates/micold-daemon/tests/orchestrator/ws_*.rs   # NEW integration tests
packages/protocol/
├── schema/client-api.schema.json                 # NEW generated, checked in
├── src/generated/client-api.ts                   # NEW generated, checked in
├── src/real/{RealOrchestratorApi,BrowserTransport,wire}.ts   # NEW
├── tests/vectors/sidebar-order.json              # NEW shared vectors (R10)
apps/client/src/features/{session,connection}/    # NEW content view, connection copy
apps/client/e2e/stack.spec.ts                     # NEW three-browser run against the real stack
scripts/{api-types.sh,local-stack.sh}  mise.toml (tasks api-types, local-stack, client-e2e-stack)
docs/                                             # user guide + config reference
```

## Milestones

| ID | Deliverable | Stories / FRs |
|---|---|---|
| M1 | Daemon `ReadStructuredEvents` request (wire change, version bump) with its failing test | FR-002, 014b (server); R9 |
| M2 | Orchestrator `/api/ws`: types, origin, heartbeat, list, `read_session`, subscribe, feed + resume, LinkDirectory, classification; integration tests | US2 (server), US3; FR-001..008, 011 |
| M3 | Schema and TS generation, drift check in gate and CI, FR-012 vectors | US4; FR-009, 010, 012 |
| M4 | Web client on the real API: transport, connection states, backoff, unreachable rows, content view, i18n en+pl, minimal `mise run local-stack`; normal build uses the real API, mock stays for tests and a new mock-only `mise run client-web-mock` | US1, US2 (client); FR-013..017 |
| M5 | Three-browser Playwright run on the stack, CI job, user docs, quickstart Part B pass, stack polish | US1..3 end to end; FR-016..018, SC-001, 002, 006 |

Each milestone ends with a gate-green PR; M2 depends on M1; M3 on M2 (wire types); M4 on M2 and M3; M5 on M4.

## Test strategy

| Requirement | Layer |
|---|---|
| diff, ring retention and boundaries (`since(epoch,0)` empty log, `n+1==oldest`), epoch, poller stop/restart, classify, origin match incl. port and `null`, schema current (FR-003, 004, 009, 011) | core unit |
| upgrade refused for foreign origin and for right host with wrong port before any message (SC-003); accepted; no-Origin on loopback (FR-004, SC-003) | daemon integration, real listener |
| ping/pong, dead-client release, typed errors, binary refusal, resume exact-once (FR-001..003, 005, 006, 008) | daemon integration |
| generated files match (FR-010, SC-004) | `scripts/api-types.sh --check` in the gate script and CI; core test for the schema half |
| client reconnect, backoff, resume, close-code policy (FR-014) | jest with fake Transport and Clock |
| order parity incl. grouping, offline, remove-then-upsert (FR-012) | Rust writes vectors, jest applies them with `order.ts`; both fail on drift |
| `ReadStructuredEvents` no attach, refuses terminal, works while a desktop holds the project (FR-002, 014b) | daemon integration |
| i18n completeness (FR-015) | existing catalog tests plus new keys |
| stack end to end, three browsers, forced reconnect, local origin allowed (FR-018, US3.3) | Playwright |
| look of connection states, unreachable marker, content view | quickstart Part B visual pass |

## Complexity Tracking

None.
