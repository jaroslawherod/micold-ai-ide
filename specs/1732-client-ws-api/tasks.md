# Tasks: JSON-over-WebSocket client API and a working web client (#1732)

**Input**: spec.md, plan.md, research.md (R1-R12), data-model.md, contracts/ws-protocol.md, quickstart.md
**Tests**: mandatory, written first and seen failing (Constitution I). Test tasks precede the code they cover.
Format: `- [ ] T### [P?] [US?] description with file path`. [P] = different files, no dependency on an open task.

## Phase 1: Setup (shared)

- [x] T001 Add `schemars` (derive) to `crates/micold-core/Cargo.toml` and `tokio-tungstenite` (already in `Cargo.lock` via `kube`, R1) to `crates/micold-daemon/Cargo.toml`; create the empty `crates/micold-core/src/client_api/{mod,messages,classify,diff,log,origin}.rs` and export `client_api` from `crates/micold-core/src/lib.rs`.
- [x] T002 [P] Add every `mod` line to `crates/micold-daemon/tests/orchestrator/main.rs` up front (`ws_support`, `ws_read_session`, `ws_origin`, `ws_protocol`, `ws_heartbeat`, `ws_resume`, `ws_listener`) with empty files, so the later [P] test tasks touch no shared file; create `crates/micold-daemon/src/orchestrator/{ws,directory,feed}.rs` stubs and `crates/micold-daemon/tests/orchestrator/ws_support.rs` (start a real listener on port 0, connect with a given `Origin`, send and read JSON frames; reuse the existing orchestrator test harness, grep `tests/orchestrator/` first).

## Phase 2: Foundational (daemon read request, wire types, pure logic)

**Order matters: T003/T004 come first (plan R9: a refusal or an unexpected cost must surface before the API types freeze).**

- [x] T003 Failing daemon integration test `crates/micold-daemon/tests/orchestrator/read_structured_events.rs` (add its `mod` line to `main.rs` itself; M1 lands before T002) for `ClientMsg::ReadStructuredEvents { project, session }`: returns the non-live history (`DaemonMsg::StructuredEvents`, `live: false`); takes no attachment, lock or view; works while a desktop holds the project (no `ProjectBusy`); a terminal session answers `TerminalSession`, an unknown one `NotFound` (FR-002, FR-014b).
- [x] T004 Implement `ClientMsg::ReadStructuredEvents` in `crates/micold-core` (protocol enum) and `crates/micold-daemon/src/state.rs` handler; bump `PROTOCOL_VERSION` (SCHEMA_HASH changes) and update the existing version/handshake tests; note the wire change in `docs/` protocol notes (R9).
- [x] T005 [P] Failing core unit tests then `client_api/messages.rs`: `ClientRequest` (`ping`, `list_daemons`, `list_sessions`, `read_session`, `subscribe { since: Option<{epoch,seq}> }`), `ServerMessage`, `ErrorCode` (`malformed`, `unknown_type`, `terminal_not_supported`, `not_found`, `daemon_unreachable`, `timeout`, `internal`, `project_busy`, `refused`), Snapshot/Daemon/Project/Location/Session/Change/ContentEntry per data-model.md, all `serde` + `schemars::JsonSchema`, snake_case; `ContentEntry.text` truncated to 64 KiB with a `truncated` flag; round-trip tests against the contract examples.
- [x] T006 [P] Failing test then `client_api/classify.rs` (FR-011, SC-005): `classify(&ClientRequest) -> Access { Read, Change }`, exhaustive `match` with no wildcard; a `KINDS` const checked against the schema `oneOf`; every kind asserted classified (all `Read` today).
- [x] T007 [P] Failing tests then `client_api/log.rs` (FR-003): `ChangeLog { epoch, next_seq, ring (cap 1024) }`, `append`, `since(epoch, n) -> Changes | ResyncNeeded`; cases: other epoch, `n+1 < oldest` resync, `n+1 == oldest` servable, `n == last` empty batch, `n > last` resync, `since(epoch,0)` on empty log empty only when `next_seq == 1`; seq strictly increasing from 1.
- [x] T008 [P] Failing tests then `client_api/diff.rs`: `diff(old, new) -> Vec<ChangeKind>` (removed first, upserts in new order with `after_key`, small `session_state`/`daemon_status` kinds); property test: applying the output to `old` with the reducer yields `new`; poller stop/restart diffs against the retained snapshot (R5).
- [x] T009 [P] Failing tests then `client_api/origin.rs` (FR-004): `OriginAllowList` matches scheme+host+port case-insensitively for scheme and host, default ports 80/443 normalised, no wildcard, `Origin: null` and malformed refused; exact origin only (`http://127.0.0.1:8731` does not allow `:9999`); absent `Origin` accepted for a loopback peer only.
- [x] T010 Failing test (`ws_read_session.rs`) then `crates/micold-daemon/src/orchestrator/directory.rs` (FR-007): `LinkDirectory` trait (`list(kind)`, `read_session(daemon, project, session)`) and the `InProcess` impl over `Arc<Shared>` reusing `list::gather` and the conversation helper of `list.rs`; opens a conversation as `list::ask` does, `Hello`, `ReadStructuredEvents`, projects events to `ContentEntry`; terminal -> `terminal_not_supported`, daemon refusal -> `not_found`/`refused`.
- [x] T011 [P] `crates/micold-daemon/src/orchestrator/config.rs`: `--api-addr` (env `MICOLD_ORCHESTRATOR_API_ADDR`), `--allow-origin` repeated (env `MICOLD_ORCHESTRATOR_ALLOW_ORIGINS`, comma separated); listener off unless `--api-addr` given; a non-loopback `--api-addr` refused until TLS (#1733); failing config tests first.

**Checkpoint**: core logic and the daemon read request exist; the listener is next.

## Phase 3: User Story 2 (server) and User Story 3 - the `/api/ws` listener (Priority P1/P2)

**Goal**: a real listener that serves list, read, subscribe with resume, heartbeat, typed errors and origin refusal.
**Independent test**: `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator ws_` (daemon integration tests, real listener).

### Tests first

- [x] T012 [P] [US3] Failing integration test `crates/micold-daemon/tests/orchestrator/ws_origin.rs`: foreign origin and right host with wrong port get 403 before any message, logged `refused origin <origin>` at WARN; allowed origin accepted; no `Origin` accepted on loopback; `Origin: null` refused; other paths 404 (SC-003, US3.1-2).
- [x] T013 [P] Failing integration test `ws_protocol.rs`: `list_daemons`, `list_sessions`, `read_session` (content, terminal -> `terminal_not_supported`, unknown -> `not_found`), unknown type and malformed JSON -> typed error with the connection still open, binary frame -> `malformed`, never a binary frame sent (FR-001, 002, 006, 008).
- [x] T014 [P] [US2] Failing integration test `ws_heartbeat.rs`: `ping` answered `pong` with the same id; a client silent for 40 s is released (paused tokio time); the server closes 1008 after 5 consecutive malformed frames and 1011 on an internal failure (FR-005, FR-008).
- [x] T015 [P] [US2] Failing integration test `ws_resume.rs`: subscribe without `since` returns a `snapshot`; changes arrive in order with strictly increasing `seq`; cut, 3 changes, reconnect with `since` receives exactly those 3, none already seen; stale epoch or too-old `since` -> `resync_needed`; 100 changes over 10 forced reconnects exactly once each (SC-002); change visible within 2 s (SC-006); poller stop/restart keeps the log (R5, R8).

### Implementation

- [x] T016 [US2] `orchestrator/feed.rs`: one poller task while at least one subscriber exists; every 1 s gather (existing `list::gather`, 2 s per-daemon deadline), diff against the last snapshot, append to the `ChangeLog`, broadcast; a link register or loss pokes it; snapshot and log retained across stop/restart; epoch random per process (R5, R8).
- [x] T017 [US3] `orchestrator/ws.rs`: `tokio-tungstenite` `accept_hdr_async` with an upgrade callback (path `/api/ws` only, `OriginAllowList` check, 403 + WARN log before any frame, R1), per-socket `ClientConnection { last_message_at, subscribed, cursor }`, request dispatch through `LinkDirectory`, `classify` called per request, typed errors, binary refusal, application-level pong and 40 s dead-client release.
- [x] T018 Failing test first in `ws_listener.rs`: `--api-addr` enables the listener, its absence binds nothing. Then wire the listener into the orchestrator startup (`crates/micold-daemon/src/orchestrator/mod.rs` or the entry point, grep for the existing http hook startup) behind `--api-addr`; keep the hand-rolled `http.rs` for the two loopback hooks.
- [x] T019 [P] `docs/` config reference for `--api-addr`, `--allow-origin`, `MICOLD_ORCHESTRATOR_*` (Constitution VII; the user-guide gate fires on `feat`).

**Checkpoint (M2)**: `ws_*` integration tests pass against a real listener.

## Phase 4: User Story 4 - generated schema and types, drift gate (Priority P2)

**Goal**: client and server cannot silently disagree.
**Independent test**: edit a type without regenerating; `scripts/api-types.sh --check` fails and names the command.

- [x] T020 [P] [US4] Failing core test `crates/micold-core/tests/client_api_schema.rs`: the checked-in `packages/protocol/schema/client-api.schema.json` equals the freshly generated schema (FR-009, FR-010, SC-004).
- [x] T021 [US4] `crates/micold-core/src/bin/micold-api-schema.rs` (pattern: `micold-tokens-ts.rs`, ~40 lines) writes the schema; generate and check in `packages/protocol/schema/client-api.schema.json`.
- [x] T022 [US4] `scripts/api-types.sh`: runs the schema bin then `json-schema-to-typescript` (add as a dev dependency of `packages/protocol`) into `packages/protocol/src/generated/client-api.ts`; `--check` regenerates to a temp dir, diffs, and prints the regenerate command; a `scripts/tests/api-types.test.sh` covers pass and fail (SC-004). Check in the generated TS.
- [x] T023 [US4] Call `scripts/api-types.sh --check` from `scripts/gate.sh`, `mise.toml` (task `api-types`, gate) and the CI workflow (`.github/workflows/`); grep how `micold-tokens-ts` drift is gated and mirror it.
- [x] T024 [P] [US4] FR-012 vectors: Rust test writes `packages/protocol/tests/vectors/sidebar-order.json` (snapshot, changes, expected key order; check mode fails on drift) covering grouping and nesting (daemon, project, worktrees, sessions under each), a daemon going offline (rows keep position, sessions `reachable: false`), remove-then-upsert of one key returning to its `after_key` place (R10).
- [x] T025 [US4] (after T024, reads its vectors file) Jest `packages/protocol/tests/order-vectors.test.ts`: apply the vectors with `applyChanges` in `packages/protocol/src/order.ts` and assert the same order; extend `order.ts` (`applyChanges`, `resync` replace) as needed (write the test first, FR-012).
- [x] T026 [P] [US4] `docs/` developer note on `scripts/api-types.sh` and the drift check.

**Checkpoint (M3)**: drift check green in gate and CI; vectors agree.

## Phase 5: User Story 1 (client) and User Story 2 (client) - web client on the real API (Priority P1)

**Goal**: the web client lists real daemons and sessions and survives drops. Needs M2 (server) and M3 (generated types).
**Independent test**: `mise run local-stack`, open `http://127.0.0.1:8731`; `mise run client-test` for the jest layer.

### Tests first

- [x] T027 [P] [US2] Failing jest `packages/protocol/tests/real/BrowserTransport.test.ts` with a fake WebSocket and `Clock`: connect, close-code policy (1001/1006/1011 reconnect and resume, 1008 straight to `offline` with no retry loop), visibility `visible` with stale pong reconnects at once (R6).
- [x] T028 [P] [US2] Failing jest (also: a `changes` message updates the open store with no reload, US1.3) `packages/protocol/tests/real/RealOrchestratorApi.test.ts`: ping every 20 s, dead at 40 s of silence; backoff base 500 ms, ceiling 15 s, full jitter, `offline` after 8 attempts with retry; resume sends `since {epoch, seq}`; `resync_needed` then `subscribe` with no `since` and the store replaces its snapshot; exactly-once across reconnects; `connecting` state on first open; unreachable sessions stay listed (FR-013, 014, 014a).
- [x] T029 [P] [US1] Failing jest/RNTL tests for the content view and connection banner states in `apps/client/src/features/{session,connection}/__tests__/` (translated connecting, reconnecting with attempt, unreachable with Retry, unreachable-session label, content empty/failed/truncated/refresh).
- [x] T030 [P] i18n completeness: extend the existing catalog completeness tests for the new keys (FR-015, SC-007); see T033 for the key list.

### Implementation

- [x] T031 [US2] `packages/protocol/src/real/BrowserTransport.ts`: the single browser-specific seam (close codes, `visibilitychange`, throttled timers); `ConnectionState = connecting | connected | reconnecting{attempt} | offline` added to the existing `ConnectionManager` (reuse its backoff, R6).
- [x] T032 [US1] `packages/protocol/src/real/{RealOrchestratorApi,wire}.ts`: implements the existing orchestrator API interface over `/api/ws` using the generated types; `wire.ts` brands plain string ids into `DaemonId` etc. at one seam; `lastSeq`/`epoch` in memory only; address from `EXPO_PUBLIC_ORCHESTRATOR_URL`, default `ws://127.0.0.1:8730/api/ws` (FR-013, FR-016).
- [x] T033 [P] [US1] `apps/client/src/i18n/{en,pl}.json`: groups `connection.*` (connecting, connected, reconnecting with attempt, unreachable title and body, retry), `session.unreachable`, `session.refresh`, `session.content.{empty,failed,truncated}`, `error.*` one per ErrorCode; the client maps `code` to a key and never shows the server `message`.
- [x] T034 [US1] `apps/client/src/features/{session,connection}/`: content view (snapshot read on open, translated refresh, reuse shared primitives and `ConnectionBanner`, anything new in the shared component folder with typed props, Constitution VIII), unreachable marker on session rows.
- [x] T035 [US1] Select `RealOrchestratorApi` in normal builds; keep the mock for tests and a new explicit mock-only `mise run client-web-mock` dev task (no `web` task exists on main; `client-web` is the static export), selected by `EXPO_PUBLIC_API_MODE=mock` in `apps/client` and `mise.toml` (FR-013).
- [x] T036 [US1] `scripts/local-stack.sh` and `mise.toml` task `local-stack` (minimal, so M4 is usable): private data dir, web export with the URL, orchestrator (`--api-addr 127.0.0.1:8730 --allow-origin http://127.0.0.1:8731 --allow-origin http://localhost:8731`), a daemon dialling it (`[orchestrator_link]`, #1725) on a seeded catalog (a project, a worktree, two sessions), `e2e/serve.mjs` for the export, `wait` with a `trap` stopping all three (FR-017, R11, R12); `scripts/tests/local-stack.test.sh`.
- [x] T037 [P] [US1] User guide page for the web client and local stack in `docs/` (Constitution VII; the user-guide gate needs it in this `feat` PR).

**Checkpoint (M4)**: `mise run local-stack` shows real daemons and sessions; jest green.

## Phase 5b: User Story 5 - local kind stack (Priority P2, M5)

Added 2026-11-17. Tests first; nothing here needs a cluster except T052.

### Tests first

- [x] T043 [US5] (done: kind v0.33.0, kindest/node v1.37.0, kubectl v1.37.2; helm stays v3.19.0 for want of a checksum; research.md R13) Re-check and bump pins in `scripts/install-kube-tools.sh` (kind, helm, kubectl) and pin the matching `kindest/node` digest in new `deploy/kind/local.yaml`, `postgres:18.7-alpine3.24` in values; record the versions and sources in research.md; extend `scripts/tests/install-kube-tools.test.sh` if its pin table changes.
- [x] T044 [P] [US5] Failing `scripts/tests/local-stack-kind.test.sh` with stub `docker`, `kind`, `kubectl`, `helm` on `PATH` logging argv: default mode is kind; `MICOLD_LOCAL_MODE=process` runs the M4 path; a missing tool is named with the process hint, exit non-zero, nothing created; cluster `micold-local` created from `deploy/kind/local.yaml` or reused; both images built and `kind load`ed; `helm upgrade --install` with `ci/local.yaml`; rollout wait; port-forwards on 8730/8731; on TERM forwards killed and `kind delete cluster` run, skipped with `MICOLD_LOCAL_KEEP=1`; a rerun with the stub `kind get clusters` listing `micold-local` issues no `kind create cluster` but still `docker build` (both images), `kind load docker-image` (both), `helm upgrade --install` and `kubectl rollout restart` (FR-019, 023, 025).
- [x] T045 [P] [US5] Failing chart checks in `scripts/tests/chart-local.test.sh` (chart-check.test.sh drives fixture charts, so a new file; kubeconform of ci/local.yaml runs in chart-check.sh) for `ci/local.yaml`: Postgres StatefulSet, headless Service, PVC and Secret rendered when `postgres.enabled`; orchestrator env takes the URL from the Secret with `state.mode=postgres`; a `daemons.count: 1` sidecar `daemon-0` in the orchestrator pod with the `seed-0` init container and an `[orchestrator_link]` to the orchestrator Service; `orchestrator.allowOrigins` rendered as one `--allow-origin` arg per entry (both 8731 origins); the web Deployment uses `micold-web:local` on 8731; kubeconform clean; `state.mode` `file` and `postgres` render, `sqlite` and `bogus` fail with exactly `state.mode=<value> is not supported by this chart: use file or postgres` (FR-020, 021, 022).
- [x] T046 [P] [US5] Failing unit test `a_given_data_dir_skips_the_users_daemon_toml` in the `#[cfg(test)]` module of `crates/micold-daemon/src/orchestrator/mod.rs`, against a new pure `implicit_config(flag, env, user_data_dir)`: with `--data-dir` (or its env) set, the implicit `default` lookup at mod.rs:207 (`micold_core::paths::data_dir()/daemon.toml`, used by `configured`) is not consulted (FR-024). `config_load.rs` already derives the host path from the given data dir and is not the bug.
- [x] T053 [P] [US5] Failing `scripts/tests/dockerfile-web.test.sh`: `Dockerfile.web` declares `ARG EXPO_PUBLIC_ORCHESTRATOR_URL` with the default `ws://127.0.0.1:8730/api/ws`, passes it to the export step, `EXPOSE 8731`, runs as non-root; with docker present (skipped otherwise) the built image serves `index.html` on 8731 containing the URL (FR-022).
- [x] T054 [P] [US5] Failing `scripts/tests/local-stack-probe.test.sh` against stub servers (a tiny node ws server answering `list` with the seed, a static file on a port): `scripts/local-stack-probe.sh` passes when both answer, fails naming the missing session or a non-200 web root, and honours its timeout (SC-008, US5.1).

### Implementation

- [x] T047 [US5] Chart: `postgres.*` values, schema and templates; `state.mode` accepts `file`/`postgres` with the new fail message in `_helpers.tpl` (store from #1948, drop the #1740 reference); `orchestrator.allowOrigins` value, `values.schema.json` entry and deployment args; update `specs/1981-kube-e2e/contracts/chart-values.md` (postgres.*, allowOrigins, daemons.*, state.mode) and the chart README (makes T045's Postgres half pass).
- [x] T048 [US5] Chart: `daemons.*` sidecars and config; `ci/local.yaml` sets `orchestrator.allowOrigins` and `web.image` `micold-web:local`; extract the M4 seed to POSIX `deploy/helm/micold/files/seed-catalog.sh` used by both process mode and the init container; `ci/local.yaml` (makes T045 pass).
- [x] T049 [US5] Fix the daemon.toml lookup in `orchestrator/mod.rs`: `configured()` has no data-dir input (mod.rs:273 passes only `args.config`), so `main` computes the default path with `implicit_config(args.data_dir, $MICOLD_ORCHESTRATOR_DATA_DIR, user data dir)` and passes it into `configured(file, default)` (makes T046 pass).
- [x] T055 [US5] `Dockerfile.web` (makes T053 pass) and `scripts/local-stack-probe.sh` (makes T054 pass).
- [x] T050 [US5] `scripts/local-stack-kind.sh` (no shared lib: e2e-kind.sh has only inline kind one-liners, R13); `scripts/local-stack.sh` mode dispatch, M4 body unchanged as process mode; web image built with the forwarded URL; `mise.toml` `local-stack` description updated (makes T044 pass).
- [x] T051 [P] [US5] Docs: user guide local-stack page (kind default, process fallback, `MICOLD_LOCAL_KEEP`, tool install via `install-kube-tools.sh`) and `docs/development/kube-e2e.md` cross-link.
- [ ] T052 [US5] (job and script written; not yet live: kind cannot run in a cloud session, so the first green run is this PR's CI) CI job `local-stack-kind` beside `kube-e2e` (same gating and tool install, 15 min timeout): runs kind mode with `MICOLD_LOCAL_KEEP=1`, in `scripts/local-stack-kind-live.sh`: each run of `scripts/local-stack.sh` is in the background and is TERMed after its probe; run 1 with KEEP probes (session over `/api/ws` and web root on 8731, 120 s after helm returns), deletes the orchestrator pod and probes the new pod; run 2, a rerun still with KEEP, must log the reuse and probe; run 3, the final one, is without KEEP, probes, and after its TERM the cluster must be gone (SC-008, US5.1, 2, 5); browser coverage of kind mode is out of scope (M6 runs Playwright on process mode); record the first live run's duration in the ledger.

## Phase 6: End to end, docs, polish (Priority P1/P2)

- [x] T038 [P] Failing Playwright `apps/client/e2e/stack.spec.ts` against the real stack in Chromium, Firefox and WebKit: starts `scripts/local-stack.sh` via the same script and a fixture seed (no AI CLI), sees the seeded session listed within 60 s (SC-001), opens it, content shown, a session created, renamed or ended on the daemon appears in the open page within 2 s with no reload (US1.3, SC-006), forced reconnect (kill and restart the orchestrator) then list returns, connects with no hand-edited allow-list (US3.3, FR-018).
  - Deviation (M6, diff review 1 F2): the browser test does not push a daemon-side session create/rename/end. Nothing outside the daemon's own protocol (schema-hash handshake) can make one, and the daemon's config watch adds projects only, within 5 s, not 2 s. The 2 s push of those changes stays covered by `tests/orchestrator/ws_*.rs`; the browser test covers the stack stopping and returning in the open page with no reload. SC-006 in the browser is open.
- [x] T039 `mise.toml` task `client-e2e-stack` and a CI job running it in three browsers (grep the existing Playwright CI job and mirror it).
- [x] T040 `scripts/local-stack.sh` polish: both ports checked free first via bash `/dev/tcp`, reports the holder and exits (no second copy), `MICOLD_LOCAL_API_PORT`/`MICOLD_LOCAL_WEB_PORT` overrides, trap stops all processes (R12); extend `scripts/tests/local-stack.test.sh`.
- [x] T041 Quickstart Part B visual pass via the `visual-pass` skill; save evidence under `specs/1732-client-ws-api/visual-pass/` (connection states, unreachable marker, content view, Polish).
- [x] T042 [P] Update the user guide and config reference for ports, overrides and the port-conflict behaviour in `docs/`.

## Dependencies and order

Phase 1 -> Phase 2 (T003/T004 first) -> Phase 3 (tests T012-T015 before T016-T018) -> Phase 4 (independent of Phase 3 except the types it generates) -> Phase 5 (needs Phases 3 and 4) -> Phase 5b (M5) -> Phase 6.
Phase 5b: T043 first; T044-T046, T053, T054 together; T047 -> T048; T049 after T046; T055 after T053/T054; T050 after T044, T048 and T055; T052 last.
Parallel: T005-T009 and T011 after T001; T012-T015 together; T020, T024, T026 (T025 after T024); T027-T030; T038 with T040.

## Implementation Strategy

MVP is the working web client (M4) on top of the server (M2) and generated types (M3); M5 moves the stack into a local kind cluster; M6 proves it across three browsers and polishes the stack. Each milestone ends gate-green and merges alone. TDD: every code task above is preceded by its failing test.

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

### M5 — Local kind stack

- **Tasks**: T043–T055
- **Deliverable**: `mise run local-stack` starts a kind cluster `micold-local` running orchestrator on Postgres, a seeded daemon and the web client, port-forwarded to the M4 addresses; `MICOLD_LOCAL_MODE=process` keeps the M4 stack; all checks but the live run pass without a cluster.
- **Satisfies**: US5 scenarios 1–5; FR-019–025; SC-008
- **Verify**: `scripts/tests/local-stack-kind.test.sh`, `scripts/tests/chart-check.test.sh`, the daemon test of T046; live: CI job `local-stack-kind`
- **Depends on**: M4
- **Tier**: full

### M6 — Three-browser end-to-end run, docs and stack polish

- **Tasks**: T038–T042
- **Deliverable**: a Playwright run on the real stack in Chromium, Firefox and WebKit (live change within 2 s, forced reconnect, local origin allowed) runs in CI; the stack reports port conflicts; user docs and the quickstart visual pass are recorded.
- **Satisfies**: US1–3 end to end; FR-016–018; SC-001, SC-002, SC-006
- **Verify**: `mise run client-e2e-stack`; quickstart Part B
- **Depends on**: M5
- **Tier**: full
