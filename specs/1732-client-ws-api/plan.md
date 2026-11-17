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
| M5 | Local kind stack: kind config, chart Postgres + daemon Deployment + local values, `local-stack.sh` kind mode (process fallback), daemon.toml fix, CI live job | US5; FR-019..025, SC-008 |
| M6 | Three-browser Playwright run on the stack, CI job, user docs, quickstart Part B pass, stack polish | US1..3 end to end; FR-016..018, SC-001, 002, 006 |

Each milestone ends with a gate-green PR; M2 depends on M1; M3 on M2 (wire types); M4 on M2 and M3; M5 on M4; M6 on M5 (its Playwright run targets whichever mode is available, process mode in CI).

## Local kind stack (M5)

Added 2026-11-17. Reuses #1981 (`deploy/helm/micold`, `scripts/e2e-kind.sh`,
`scripts/install-kube-tools.sh`, `Dockerfile.orchestrator`, the `kube-e2e` CI job).

- **Modes**: `scripts/local-stack.sh` dispatches on `MICOLD_LOCAL_MODE` (`kind` default, `process`
  = M4 body moved to a function unchanged). Kind mode lives in `scripts/local-stack-kind.sh`
  (the kind steps of `e2e-kind.sh` are inline one-liners, so nothing is extracted; R13).
- **Cluster**: `deploy/kind/local.yaml` (one control-plane node, image pinned by digest). Cluster
  name `micold-local`, distinct from e2e's, so both can coexist. Reuse when it exists.
- **Images**: `micold-orchestrator:local` from `Dockerfile.orchestrator` (the same binary is the
  daemon); `micold-web:local` from new `Dockerfile.web` (node stage runs the Expo static export
  with build arg `EXPO_PUBLIC_ORCHESTRATOR_URL`, default `ws://127.0.0.1:8730/api/ws`; runtime
  stage serves it on 8731 with an unprivileged static server); `kind load docker-image`. A rerun
  with the cluster present skips `kind create`, rebuilds, loads, `helm upgrade --install`s and
  `kubectl rollout restart`s (FR-023).
- **Chart**: new `postgres.*` values (official image, StatefulSet + headless Service + PVC, Secret
  with a generated password via `lookup`-or-`randAlphaNum`), `state.mode` accepts `file` and `postgres`
  (store from #1948; the stale #1740 message in `_helpers.tpl` is replaced, FR-020) and is wired to
  `database.secretName`; new `orchestrator.allowOrigins` list rendered as `--allow-origin` args; new `daemons.count` sidecars `daemon-<i>` in the orchestrator pod
  (the link is a local socket; R13), each with a `seed-<i>` init container (pinned `alpine/git`)
  running the seed shared with process mode, `deploy/helm/micold/files/seed-catalog.sh` (in the
  chart so `.Files.Get` can ship it), whose `[orchestrator_link] socket` is the shared
  `/run/micold/orchestrator.sock`; `--api-addr=127.0.0.1:8730` on the pod's loopback (the API
  refuses other addresses until TLS, #1733), reached by port-forward; `ci/local.yaml` values for kind mode, also used by chart-check.
- **Access**: `kubectl port-forward` for both services to loopback; `orchestrator.allowOrigins` in
  `ci/local.yaml` holds `http://127.0.0.1:8731` and `http://localhost:8731`.
- **Teardown**: `trap` kills port-forwards and runs `kind delete cluster` unless
  `MICOLD_LOCAL_KEEP=1`.
- **Fold-ins from #1981**: daemon.toml lookup skipped when a data dir is given (FR-024, a
  container would otherwise read `$HOME`); stale `state.mode` refusal (FR-020). The first live
  kind run (#1981 T022) is subsumed by this milestone's live check.
- **Pins** (checked 2026-11-17): `postgres:18.7-alpine3.24` (Docker Hub); Kubernetes stable
  v1.37.2 (dl.k8s.io). GitHub was unreachable from the design session, so the kind, kindest/node
  digest and helm pins (repo has kind v0.30.0, helm v3.19.0, kubectl v1.34.0) are re-checked and
  bumped in T043 against the kind release notes; the node image must match the kind release.
  Postgres: the official image, not the Bitnami chart (Bitnami images left the free catalogue in
  2025) and no operator, to keep the chart self-contained.
- **Live probe**: `scripts/local-stack-probe.sh` asserts the seeded session over `/api/ws` and that
  `http://127.0.0.1:8731/` serves the export's `index.html`; browser coverage of kind mode is out
  of scope (M6's Playwright runs on process mode). Its stub-mode test lands before T050.
- **No cluster in cloud**: all script behaviour is tested with stub binaries on `PATH` recording
  their argv (pattern of `scripts/tests/e2e-kind.test.sh`); manifests by `helm template` +
  kubeconform in `chart-check.sh`. The live run is `mise run local-stack` on a docker host and a
  CI job `local-stack-kind` beside `kube-e2e`.

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
| kind-mode script: mode select, tool check, argv issued, teardown, keep (FR-019, 023, 025) | `scripts/tests/local-stack-kind.test.sh` with stub binaries |
| chart Postgres, daemon Deployment, state.mode=postgres, local values (FR-020..022) | `scripts/tests/chart-check.test.sh` (helm template + kubeconform) |
| daemon.toml not read with a data dir (FR-024) | daemon unit test |
| live kind stack, restart survival (SC-008) | CI job `local-stack-kind`; local `mise run local-stack` |
| look of connection states, unreachable marker, content view | quickstart Part B visual pass |

## Complexity Tracking

None.
