# Tasks: JSON-over-WebSocket client API and a working web client (#1732)

**Input**: spec.md, plan.md, research.md (R1-R12), data-model.md, contracts/ws-protocol.md, quickstart.md
**Tests**: mandatory, written first and seen failing (Constitution I). Test tasks precede the code they cover.
Format: `- [ ] T### [P?] [US?] description with file path`. [P] = different files, no dependency on an open task.

## Phase 1: Setup (shared)

- [ ] T001 Add `schemars` (derive) to `crates/micold-core/Cargo.toml` and `tokio-tungstenite` (already in `Cargo.lock` via `kube`, R1) to `crates/micold-daemon/Cargo.toml`; create the empty `crates/micold-core/src/client_api/{mod,messages,classify,diff,log,origin}.rs` and export `client_api` from `crates/micold-core/src/lib.rs`.
- [ ] T002 [P] Add every `mod` line to `crates/micold-daemon/tests/orchestrator/main.rs` up front (`ws_support`, `ws_read_session`, `ws_origin`, `ws_protocol`, `ws_heartbeat`, `ws_resume`, `ws_listener`) with empty files, so the later [P] test tasks touch no shared file; create `crates/micold-daemon/src/orchestrator/{ws,directory,feed}.rs` stubs and `crates/micold-daemon/tests/orchestrator/ws_support.rs` (start a real listener on port 0, connect with a given `Origin`, send and read JSON frames; reuse the existing orchestrator test harness, grep `tests/orchestrator/` first).

## Phase 2: Foundational (daemon read request, wire types, pure logic)

**Order matters: T003/T004 come first (plan R9: a refusal or an unexpected cost must surface before the API types freeze).**

- [ ] T003 Failing daemon integration test `crates/micold-daemon/tests/orchestrator/read_structured_events.rs` (add its `mod` line to `main.rs` itself; M1 lands before T002) for `ClientMsg::ReadStructuredEvents { project, session }`: returns the non-live history (`DaemonMsg::StructuredEvents`, `live: false`); takes no attachment, lock or view; works while a desktop holds the project (no `ProjectBusy`); a terminal session answers `TerminalSession`, an unknown one `NotFound` (FR-002, FR-014b).
- [ ] T004 Implement `ClientMsg::ReadStructuredEvents` in `crates/micold-core` (protocol enum) and `crates/micold-daemon/src/state.rs` handler; bump `PROTOCOL_VERSION` (SCHEMA_HASH changes) and update the existing version/handshake tests; note the wire change in `docs/` protocol notes (R9).
- [ ] T005 [P] Failing core unit tests then `client_api/messages.rs`: `ClientRequest` (`ping`, `list_daemons`, `list_sessions`, `read_session`, `subscribe { since: Option<{epoch,seq}> }`), `ServerMessage`, `ErrorCode` (`malformed`, `unknown_type`, `terminal_not_supported`, `not_found`, `daemon_unreachable`, `timeout`, `internal`, `project_busy`, `refused`), Snapshot/Daemon/Project/Location/Session/Change/ContentEntry per data-model.md, all `serde` + `schemars::JsonSchema`, snake_case; `ContentEntry.text` truncated to 64 KiB with a `truncated` flag; round-trip tests against the contract examples.
- [ ] T006 [P] Failing test then `client_api/classify.rs` (FR-011, SC-005): `classify(&ClientRequest) -> Access { Read, Change }`, exhaustive `match` with no wildcard; a `KINDS` const checked against the schema `oneOf`; every kind asserted classified (all `Read` today).
- [ ] T007 [P] Failing tests then `client_api/log.rs` (FR-003): `ChangeLog { epoch, next_seq, ring (cap 1024) }`, `append`, `since(epoch, n) -> Changes | ResyncNeeded`; cases: other epoch, `n+1 < oldest` resync, `n+1 == oldest` servable, `n == last` empty batch, `n > last` resync, `since(epoch,0)` on empty log empty only when `next_seq == 1`; seq strictly increasing from 1.
- [ ] T008 [P] Failing tests then `client_api/diff.rs`: `diff(old, new) -> Vec<ChangeKind>` (removed first, upserts in new order with `after_key`, small `session_state`/`daemon_status` kinds); property test: applying the output to `old` with the reducer yields `new`; poller stop/restart diffs against the retained snapshot (R5).
- [ ] T009 [P] Failing tests then `client_api/origin.rs` (FR-004): `OriginAllowList` matches scheme+host+port case-insensitively for scheme and host, default ports 80/443 normalised, no wildcard, `Origin: null` and malformed refused; exact origin only (`http://127.0.0.1:8731` does not allow `:9999`); absent `Origin` accepted for a loopback peer only.
- [ ] T010 Failing test (`ws_read_session.rs`) then `crates/micold-daemon/src/orchestrator/directory.rs` (FR-007): `LinkDirectory` trait (`list(kind)`, `read_session(daemon, project, session)`) and the `InProcess` impl over `Arc<Shared>` reusing `list::gather` and the conversation helper of `list.rs`; opens a conversation as `list::ask` does, `Hello`, `ReadStructuredEvents`, projects events to `ContentEntry`; terminal -> `terminal_not_supported`, daemon refusal -> `not_found`/`refused`.
- [ ] T011 [P] `crates/micold-daemon/src/orchestrator/config.rs`: `--api-addr` (env `MICOLD_ORCHESTRATOR_API_ADDR`), `--allow-origin` repeated (env `MICOLD_ORCHESTRATOR_ALLOW_ORIGINS`, comma separated); listener off unless `--api-addr` given; a non-loopback `--api-addr` refused until TLS (#1733); failing config tests first.

**Checkpoint**: core logic and the daemon read request exist; the listener is next.

## Phase 3: User Story 2 (server) and User Story 3 - the `/api/ws` listener (Priority P1/P2)

**Goal**: a real listener that serves list, read, subscribe with resume, heartbeat, typed errors and origin refusal.
**Independent test**: `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator ws_` (daemon integration tests, real listener).

### Tests first

- [ ] T012 [P] [US3] Failing integration test `crates/micold-daemon/tests/orchestrator/ws_origin.rs`: foreign origin and right host with wrong port get 403 before any message, logged `refused origin <origin>` at WARN; allowed origin accepted; no `Origin` accepted on loopback; `Origin: null` refused; other paths 404 (SC-003, US3.1-2).
- [ ] T013 [P] Failing integration test `ws_protocol.rs`: `list_daemons`, `list_sessions`, `read_session` (content, terminal -> `terminal_not_supported`, unknown -> `not_found`), unknown type and malformed JSON -> typed error with the connection still open, binary frame -> `malformed`, never a binary frame sent (FR-001, 002, 006, 008).
- [ ] T014 [P] [US2] Failing integration test `ws_heartbeat.rs`: `ping` answered `pong` with the same id; a client silent for 40 s is released (paused tokio time); the server closes 1008 after 5 consecutive malformed frames and 1011 on an internal failure (FR-005, FR-008).
- [ ] T015 [P] [US2] Failing integration test `ws_resume.rs`: subscribe without `since` returns a `snapshot`; changes arrive in order with strictly increasing `seq`; cut, 3 changes, reconnect with `since` receives exactly those 3, none already seen; stale epoch or too-old `since` -> `resync_needed`; 100 changes over 10 forced reconnects exactly once each (SC-002); change visible within 2 s (SC-006); poller stop/restart keeps the log (R5, R8).

### Implementation

- [ ] T016 [US2] `orchestrator/feed.rs`: one poller task while at least one subscriber exists; every 1 s gather (existing `list::gather`, 2 s per-daemon deadline), diff against the last snapshot, append to the `ChangeLog`, broadcast; a link register or loss pokes it; snapshot and log retained across stop/restart; epoch random per process (R5, R8).
- [ ] T017 [US3] `orchestrator/ws.rs`: `tokio-tungstenite` `accept_hdr_async` with an upgrade callback (path `/api/ws` only, `OriginAllowList` check, 403 + WARN log before any frame, R1), per-socket `ClientConnection { last_message_at, subscribed, cursor }`, request dispatch through `LinkDirectory`, `classify` called per request, typed errors, binary refusal, application-level pong and 40 s dead-client release.
- [ ] T018 Failing test first in `ws_listener.rs`: `--api-addr` enables the listener, its absence binds nothing. Then wire the listener into the orchestrator startup (`crates/micold-daemon/src/orchestrator/mod.rs` or the entry point, grep for the existing http hook startup) behind `--api-addr`; keep the hand-rolled `http.rs` for the two loopback hooks.
- [ ] T019 [P] `docs/` config reference for `--api-addr`, `--allow-origin`, `MICOLD_ORCHESTRATOR_*` (Constitution VII; the user-guide gate fires on `feat`).

**Checkpoint (M2)**: `ws_*` integration tests pass against a real listener.

## Phase 4: User Story 4 - generated schema and types, drift gate (Priority P2)

**Goal**: client and server cannot silently disagree.
**Independent test**: edit a type without regenerating; `scripts/api-types.sh --check` fails and names the command.

- [ ] T020 [P] [US4] Failing core test `crates/micold-core/tests/client_api_schema.rs`: the checked-in `packages/protocol/schema/client-api.schema.json` equals the freshly generated schema (FR-009, FR-010, SC-004).
- [ ] T021 [US4] `crates/micold-core/src/bin/micold-api-schema.rs` (pattern: `micold-tokens-ts.rs`, ~40 lines) writes the schema; generate and check in `packages/protocol/schema/client-api.schema.json`.
- [ ] T022 [US4] `scripts/api-types.sh`: runs the schema bin then `json-schema-to-typescript` (add as a dev dependency of `packages/protocol`) into `packages/protocol/src/generated/client-api.ts`; `--check` regenerates to a temp dir, diffs, and prints the regenerate command; a `scripts/tests/api-types.test.sh` covers pass and fail (SC-004). Check in the generated TS.
- [ ] T023 [US4] Call `scripts/api-types.sh --check` from `scripts/gate.sh`, `mise.toml` (task `api-types`, gate) and the CI workflow (`.github/workflows/`); grep how `micold-tokens-ts` drift is gated and mirror it.
- [ ] T024 [P] [US4] FR-012 vectors: Rust test writes `packages/protocol/tests/vectors/sidebar-order.json` (snapshot, changes, expected key order; check mode fails on drift) covering grouping and nesting (daemon, project, worktrees, sessions under each), a daemon going offline (rows keep position, sessions `reachable: false`), remove-then-upsert of one key returning to its `after_key` place (R10).
- [ ] T025 [US4] (after T024, reads its vectors file) Jest `packages/protocol/tests/order-vectors.test.ts`: apply the vectors with `applyChanges` in `packages/protocol/src/order.ts` and assert the same order; extend `order.ts` (`applyChanges`, `resync` replace) as needed (write the test first, FR-012).
- [ ] T026 [P] [US4] `docs/` developer note on `scripts/api-types.sh` and the drift check.

**Checkpoint (M3)**: drift check green in gate and CI; vectors agree.

## Phase 5: User Story 1 (client) and User Story 2 (client) - web client on the real API (Priority P1)

**Goal**: the web client lists real daemons and sessions and survives drops. Needs M2 (server) and M3 (generated types).
**Independent test**: `mise run local-stack`, open `http://127.0.0.1:8731`; `mise run client-test` for the jest layer.

### Tests first

- [ ] T027 [P] [US2] Failing jest `packages/protocol/tests/real/BrowserTransport.test.ts` with a fake WebSocket and `Clock`: connect, close-code policy (1001/1006/1011 reconnect and resume, 1008 straight to `offline` with no retry loop), visibility `visible` with stale pong reconnects at once (R6).
- [ ] T028 [P] [US2] Failing jest (also: a `changes` message updates the open store with no reload, US1.3) `packages/protocol/tests/real/RealOrchestratorApi.test.ts`: ping every 20 s, dead at 40 s of silence; backoff base 500 ms, ceiling 15 s, full jitter, `offline` after 8 attempts with retry; resume sends `since {epoch, seq}`; `resync_needed` then `subscribe` with no `since` and the store replaces its snapshot; exactly-once across reconnects; `connecting` state on first open; unreachable sessions stay listed (FR-013, 014, 014a).
- [ ] T029 [P] [US1] Failing jest/RNTL tests for the content view and connection banner states in `apps/client/src/features/{session,connection}/__tests__/` (translated connecting, reconnecting with attempt, unreachable with Retry, unreachable-session label, content empty/failed/truncated/refresh).
- [ ] T030 [P] i18n completeness: extend the existing catalog completeness tests for the new keys (FR-015, SC-007); see T033 for the key list.

### Implementation

- [ ] T031 [US2] `packages/protocol/src/real/BrowserTransport.ts`: the single browser-specific seam (close codes, `visibilitychange`, throttled timers); `ConnectionState = connecting | connected | reconnecting{attempt} | offline` added to the existing `ConnectionManager` (reuse its backoff, R6).
- [ ] T032 [US1] `packages/protocol/src/real/{RealOrchestratorApi,wire}.ts`: implements the existing orchestrator API interface over `/api/ws` using the generated types; `wire.ts` brands plain string ids into `DaemonId` etc. at one seam; `lastSeq`/`epoch` in memory only; address from `EXPO_PUBLIC_ORCHESTRATOR_URL`, default `ws://127.0.0.1:8730/api/ws` (FR-013, FR-016).
- [ ] T033 [P] [US1] `apps/client/src/i18n/{en,pl}.json`: groups `connection.*` (connecting, connected, reconnecting with attempt, unreachable title and body, retry), `session.unreachable`, `session.refresh`, `session.content.{empty,failed,truncated}`, `error.*` one per ErrorCode; the client maps `code` to a key and never shows the server `message`.
- [ ] T034 [US1] `apps/client/src/features/{session,connection}/`: content view (snapshot read on open, translated refresh, reuse shared primitives and `ConnectionBanner`, anything new in the shared component folder with typed props, Constitution VIII), unreachable marker on session rows.
- [ ] T035 [US1] Select `RealOrchestratorApi` in normal builds; keep the mock for tests and a new explicit mock-only `mise run client-web-mock` dev task (no `web` task exists on main; `client-web` is the static export), selected by `EXPO_PUBLIC_API_MODE=mock` in `apps/client` and `mise.toml` (FR-013).
- [ ] T036 [US1] `scripts/local-stack.sh` and `mise.toml` task `local-stack` (minimal, so M4 is usable): private data dir, web export with the URL, orchestrator (`--api-addr 127.0.0.1:8730 --allow-origin http://127.0.0.1:8731 --allow-origin http://localhost:8731`), a daemon dialling it (`[orchestrator_link]`, #1725) on a seeded catalog (a project, a worktree, two sessions), `e2e/serve.mjs` for the export, `wait` with a `trap` stopping all three (FR-017, R11, R12); `scripts/tests/local-stack.test.sh`.
- [ ] T037 [P] [US1] User guide page for the web client and local stack in `docs/` (Constitution VII; the user-guide gate needs it in this `feat` PR).

**Checkpoint (M4)**: `mise run local-stack` shows real daemons and sessions; jest green.

## Phase 6: End to end, docs, polish (Priority P1/P2)

- [ ] T038 [P] Failing Playwright `apps/client/e2e/stack.spec.ts` against the real stack in Chromium, Firefox and WebKit: starts `scripts/local-stack.sh` via the same script and a fixture seed (no AI CLI), sees the seeded session listed within 60 s (SC-001), opens it, content shown, a session created, renamed or ended on the daemon appears in the open page within 2 s with no reload (US1.3, SC-006), forced reconnect (kill and restart the orchestrator) then list returns, connects with no hand-edited allow-list (US3.3, FR-018).
- [ ] T039 `mise.toml` task `client-e2e-stack` and a CI job running it in three browsers (grep the existing Playwright CI job and mirror it).
- [ ] T040 `scripts/local-stack.sh` polish: both ports checked free first via bash `/dev/tcp`, reports the holder and exits (no second copy), `MICOLD_LOCAL_API_PORT`/`MICOLD_LOCAL_WEB_PORT` overrides, trap stops all processes (R12); extend `scripts/tests/local-stack.test.sh`.
- [ ] T041 Quickstart Part B visual pass via the `visual-pass` skill; save evidence under `specs/1732-client-ws-api/visual-pass/` (connection states, unreachable marker, content view, Polish).
- [ ] T042 [P] Update the user guide and config reference for ports, overrides and the port-conflict behaviour in `docs/`.

## Dependencies and order

Phase 1 -> Phase 2 (T003/T004 first) -> Phase 3 (tests T012-T015 before T016-T018) -> Phase 4 (independent of Phase 3 except the types it generates) -> Phase 5 (needs Phases 3 and 4) -> Phase 6.
Parallel: T005-T009 and T011 after T001; T012-T015 together; T020, T024, T026 (T025 after T024); T027-T030; T038 with T040.

## Implementation Strategy

MVP is the working web client (M4) on top of the server (M2) and generated types (M3); M5 proves it across three browsers and polishes the stack. Each milestone ends gate-green and merges alone. TDD: every code task above is preceded by its failing test.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Daemon read-only structured-events request

- **Tasks**: T003–T004
- **Deliverable**: a daemon answers `ClientMsg::ReadStructuredEvents` with a session's non-live history without attaching or locking, even while a desktop holds the project; `PROTOCOL_VERSION` is bumped and documented.
- **Satisfies**: FR-002 (content read), FR-014b (server); research R9
- **Verify**: `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator read_structured_events`
- **Depends on**: —
- **Tier**: full

### M2 — `/api/ws` listener with resume and origin checks

- **Tasks**: T001–T002, T005–T019
- **Deliverable**: with `--api-addr` and `--allow-origin` set, the orchestrator serves `/api/ws`: list, read a session's content, subscribe with exact-once resume, app-level heartbeat, typed errors, foreign origins refused with 403.
- **Satisfies**: US2 server-side scenarios 1–2; US3 scenarios 1–2; FR-001–008, FR-011; SC-002, SC-003, SC-005, SC-006
- **Verify**: `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator ws_` and `mise run test-core`
- **Depends on**: M1 (`read_session` uses the new daemon request)
- **Tier**: full

### M3 — Generated schema and TypeScript types with a drift gate

- **Tasks**: T020–T026
- **Deliverable**: `scripts/api-types.sh` regenerates the checked-in JSON Schema and TS types from the Rust types; the gate and CI fail when stale; shared sidebar-order vectors pass in Rust and jest.
- **Satisfies**: US4 scenarios 1–3; FR-009, FR-010, FR-012; SC-004
- **Verify**: `scripts/api-types.sh --check` (fails after editing a type without regenerating) and `pnpm --filter @micold/protocol test`
- **Depends on**: M2 (the wire types of T005 live in M2's `client_api`)
- **Tier**: full

### M4 — Web client on the real API 🎯 MVP

- **Tasks**: T027–T037
- **Deliverable**: `mise run local-stack` starts orchestrator, a seeded daemon and the web client; the client lists real daemons and sessions, shows connection states, reconnects with backoff and resume, marks unreachable sessions, opens session content, in English and Polish.
- **Satisfies**: US1 scenarios 1–4; US2 scenario 3; US3 scenario 3; FR-013–017, FR-014a, FR-014b; SC-007
- **Verify**: `mise run client-test`, then quickstart Part B steps 1–4
- **Depends on**: M2, M3
- **Tier**: full

### M5 — Three-browser end-to-end run, docs and stack polish

- **Tasks**: T038–T042
- **Deliverable**: a Playwright run on the real stack in Chromium, Firefox and WebKit (live change within 2 s, forced reconnect, local origin allowed) runs in CI; the stack reports port conflicts; user docs and the quickstart visual pass are recorded.
- **Satisfies**: US1–3 end to end; FR-016–018; SC-001, SC-002, SC-006
- **Verify**: `mise run client-e2e-stack`; quickstart Part B
- **Depends on**: M4
- **Tier**: full
