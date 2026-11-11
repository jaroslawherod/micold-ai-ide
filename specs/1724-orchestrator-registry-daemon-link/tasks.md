---

description: "Task list for #1724 orchestrator mode: registry, daemon link, heartbeat, offline"
---

# Tasks: Orchestrator mode — registry, daemon link, heartbeat, offline

**Input**: `specs/1724-orchestrator-registry-daemon-link/` (spec.md, plan.md, research.md, data-model.md, contracts/link.md, quickstart.md)

**Tests**: mandatory (Constitution I). In every phase the test tasks come first, are run red, and only then the code that makes them pass.

**Documentation**: each story's milestone updates `docs/daemon.md` and the user-guide page (Constitution VII); M1 creates them.

**Cross-platform**: all new code and tests run on Linux, macOS and Windows; no `cfg(unix)` outside `platform/`.

**Design points the tasks carry** (plan.md): (1) the announcement/verdict preamble is stable JSON outside `SCHEMA_HASH`, read before `Hello`/`Welcome`, so an older-protocol daemon can be refused and listed `needs-update`; (2) the orchestrator-initiated heartbeat adds `DaemonMsg::LinkPing` / `ClientMsg::LinkPong`, `PROTOCOL_VERSION` 80 to 81, ignoring arms in `server.rs` and every exhaustive client match; (3) two-phase claim (`claim` / `confirm` / `claim_failed`, a reservation ends at `LIVENESS_DEADLINE`) over a pure clock-injected `Registry` in `micold-core`. A reserved link refuses rivals for at most 9 s (extends FR-007).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: different files, no dependency on an unfinished task in the same phase.
- Core paths are under `crates/micold-core/src/orchestrator/`, daemon paths under `crates/micold-daemon/src/orchestrator/`.

## Phase 1: Setup

- [ ] T001 Create `crates/micold-core/src/orchestrator/mod.rs` (empty re-exports, `DEFAULT_TENANT = "default"`) and add `pub mod orchestrator;` to `crates/micold-core/src/lib.rs`
- [ ] T002 Create `crates/micold-daemon/src/orchestrator/mod.rs` (empty `run` stub), add `pub mod orchestrator;` to `crates/micold-daemon/src/lib.rs`, and wire the new test binary: `[[test]] name = "orchestrator"`, `path = "tests/orchestrator/main.rs"` in `crates/micold-daemon/Cargo.toml` (so `every_test_file_is_wired.rs` stays green)
- [ ] T003 [P] Derive `Serialize` on `Kind` in `crates/micold-core/src/daemon_config/mod.rs` (not hashed; add a round-trip test beside the existing `Kind` tests)

## Phase 2: Foundational (blocks every story)

**Tests first (red):**

- [ ] T004 [P] Unit tests for `DaemonRecord`, `Health`, `TenantId` in `crates/micold-core/src/orchestrator/record.rs`: name trimmed and falls back to the id, tenant defaults to `default` and is never empty, `Health` serializes `online`/`offline`/`needs-update`, a missing `health` reads `offline`, unknown fields ignored (data-model.md)
- [ ] T005 [P] Unit tests for `announce.rs` in `crates/micold-core/src/orchestrator/announce.rs`: a golden JSON fixture of an old announcement (`format: 1`) still decodes; unknown extra fields ignored; invalid id (empty, too long, bad characters) refused at decode; malformed JSON refused; `LinkQuery::List` encodes and decodes the stable JSON; a preamble that does not finish within `LIVENESS_DEADLINE` is closed (injected clock); `LinkVerdict::Accepted` and `Refused { reason, orchestrator_version, daemon_version, message }` encode to the stable JSON of contracts/link.md; preamble frame cap 64 KiB rejected above it; `AlreadyLinked` and `VersionMismatch` are distinct reasons
- [ ] T006 [P] Unit tests for the basic `Registry` in `crates/micold-core/src/orchestrator/registry.rs` (injected `now: Instant`, `wall_secs: u64`): `claim` on an unknown id returns `Reserved{link}` and changes no record; `confirm` upserts the record `online` with `last_seen = wall`; a relink with a new name or capabilities updates the record in place, never duplicates it; `list()` is sorted by id and empty for an empty registry; every mutating call returns a snapshot with a strictly increasing sequence number; `confirm` with an unknown `LinkToken` returns `ConfirmRefused::Stale` and changes nothing

**Implementation (green):**

- [ ] T007 [P] Implement `DaemonRecord`, `Health`, `TenantId` in `crates/micold-core/src/orchestrator/record.rs` per data-model.md
- [ ] T008 [P] Implement `LinkAnnouncement`, `LinkVerdict`, `RefuseReason` and their encode/decode in `crates/micold-core/src/orchestrator/announce.rs` (`#[serde(default)]` additive fields, `WireDaemonId` validated at decode, no dependency on `SCHEMA_HASH`)
- [ ] T009 Implement `Registry` (`claim` accept path, `confirm` (returns `Result<Confirmed, ConfirmRefused>`, `ConfirmRefused::{Stale, AlreadyLinked}`), `list`, `LinkToken`, `Claim`, `LinkAction::Stale`; every mutating call returns the sequence-stamped snapshot to save) in `crates/micold-core/src/orchestrator/registry.rs`; refusal rows, `claim_failed`, heartbeat and restore come in later milestones. Needs T007, T008

## Phase 3: User Story 1 — The orchestrator starts and lists a joined daemon (P1) 🎯 MVP

**Goal**: `micold-daemon orchestrator` starts, a daemon links and is listed with every field, no session is hosted.

**Independent test**: start on a temp state dir, dial a fake daemon, read the listing.

**Tests first (red):**

- [ ] T010 [P] [US1] Create the test harness `crates/micold-daemon/tests/orchestrator/main.rs` plus `support.rs`: temp state dir, `run_with` start on a real local socket in that dir, controllable test clock, fake daemon that sends the announcement, reads the verdict, sends `Hello`, reads `Welcome`, and a listing client; it also declares every `mod` of the test binary (`link_and_list`, `not_hosted`, `dispatch`, `heartbeat`, `version_mismatch`, `one_link_per_id`, `restart`, `no_outbound`, `netns`) so later test tasks touch only their own file. Then in `link_and_list.rs` the US1 scenarios 1 and 3 (SC-001): every field listed `online`; no tenant announced lists `default`
- [ ] T011 [P] [US1] `crates/micold-daemon/tests/orchestrator/not_hosted.rs`: session start and session list as post-`Hello` messages on an accepted link get the refusal `not hosted by the orchestrator` and an empty list; no child process or PTY is spawned (US1 scenario 2, FR-001)
- [ ] T012 [P] [US1] Test in `crates/micold-daemon/tests/orchestrator/dispatch.rs` of the pure `parse_mode(args)` (in `crates/micold-daemon/src/orchestrator/mod.rs`): `Mode::Orchestrator` only when `args[0] == "orchestrator"`; no argument and any other argument give `Mode::Daemon` (FR-010)

**Implementation (green):**

- [ ] T013 [US1] `crates/micold-daemon/src/orchestrator/link.rs`: per-link task: read the preamble (cap 64 KiB, `LIVENESS_DEADLINE` timeout), `Registry::claim`, write the `LinkVerdict`, switch to `WireCodec<ClientMsg, DaemonMsg>`, read `Hello`, `protocol::handshake::evaluate`, `confirm`, send `Welcome` with an empty catalog (contracts/link.md steps 1 to 3). Invalid id or malformed JSON closes the link. Needs T009
- [ ] T014 [US1] `crates/micold-daemon/src/orchestrator/query.rs` and `mod.rs`: `run` / `run_with(socket path, clock)` binds the local socket via `interprocess` and the platform path helpers, builds no `DaemonState`, accepts links into a shared `Mutex<Registry>` (lock never held across `.await`), serves the listing and answers session start/list with the not-hosted refusal. The listing is the stable-JSON `LinkQuery::List` frame of contracts/link.md (no new wire variant, so no protocol change before M2). Add a pure `parse_mode(args) -> Mode` in `mod.rs`. Needs T013
- [ ] T015 [US1] `crates/micold-daemon/src/main.rs`: one-line `match parse_mode(..)` calling `orchestrator::run`; no decision logic here. Needs T014
- [ ] T016 [US1] Docs: create the "Orchestrator mode" section in `docs/daemon.md`, a page `docs/user-guide/orchestrator-mode.md` linked from `docs/SUMMARY.md` (what it is, how to start, that it hosts no session, the listing fields)

**Checkpoint**: M1 deliverable works; `T010` to `T012` green.

## Phase 4: User Story 2 — A silent link shows offline, its return shows online (P1)

**Goal**: orchestrator-initiated heartbeat, `offline` within `LIVENESS_DEADLINE` + `CHECK_INTERVAL`, `online` on the next frame.

**Independent test**: controllable clock, silence a fake daemon, read, resume, read.

**Tests first (red):**

- [ ] T017 [P] [US2] Unit tests in `crates/micold-core/src/orchestrator/liveness.rs` and `registry.rs`: `Heartbeat` wraps `protocol::keepalive::Keepalive`; `tick` returns `SendPing` after `PING_INTERVAL`, `Idle` between, `Expired` after `LIVENESS_DEADLINE` and sets `offline` once with `last_seen` unchanged (SC-002, US2.1); `answered` sets `online` and advances `last_seen` to the injected wall time (US2.2); `closed` keeps health until the deadline; a never-issued token changes nothing; another daemon's record is unchanged (US2.3)
- [ ] T018 [P] [US2] Codec round-trip tests for `DaemonMsg::LinkPing { nonce }` and `ClientMsg::LinkPong { nonce }` in `crates/micold-core/src/protocol/messages.rs`, and a version test in `version.rs` that `PROTOCOL_VERSION` is 81
- [ ] T019 [P] [US2] Tests that the ignoring arms exist: a stray `ClientMsg::LinkPong` sent to a normal daemon is ignored and the connection stays up (in the existing `crates/micold-daemon/tests/connection/` suite, new file `link_pong_ignored.rs` wired in its `main.rs`)
- [ ] T020 [P] [US2] `crates/micold-daemon/tests/orchestrator/heartbeat.rs`: the fake daemon silenced past the deadline lists `offline` with the last answer's time; its reply brings `online` and a later `last_seen`; a second daemon's record is unchanged throughout; the orchestrator sends a `LinkPing` every `PING_INTERVAL` when idle

**Implementation (green):**

- [ ] T021 [US2] Add `DaemonMsg::LinkPing` and `ClientMsg::LinkPong` to `crates/micold-core/src/protocol/messages.rs`, bump `PROTOCOL_VERSION` 80 to 81 with its changelog line in `crates/micold-core/src/protocol/version.rs`, refresh pinned hashes
- [ ] T022 [US2] Use `LSP findReferences` on `DaemonMsg` and `ClientMsg` to find every exhaustive match: add an ignoring `ClientMsg::LinkPong` arm in `crates/micold-daemon/src/server.rs` and an ignoring `DaemonMsg::LinkPing` arm in each exhaustive match under `crates/micold-client/src/`. Red first: before the client arm, write a client-side test beside that match feeding a `DaemonMsg::LinkPing` and expecting no error, and see it fail for the missing arm. Needs T021
- [ ] T023 [P] [US2] Implement `Heartbeat` in `crates/micold-core/src/orchestrator/liveness.rs` and `Registry::answered`, `tick`, `closed` in `registry.rs`
- [ ] T024 [US2] `crates/micold-daemon/src/orchestrator/link.rs`: steady-state loop `select!` over inbound frames (any frame is `answered`, `LinkPong` included) and a `CHECK_INTERVAL` timer (`tick`; `SendPing` sends `DaemonMsg::LinkPing`); on `Expired` keep reading so a later frame brings the daemon back `online` without a new link. Needs T021, T023
- [ ] T025 [US2] Docs: `docs/daemon.md` and `docs/user-guide/orchestrator-mode.md` explain `online`/`offline`, the 3 s ping and the 9 s deadline

**Checkpoint**: M2 deliverable works.

## Phase 5: User Story 3 — An older-protocol daemon is refused, others unaffected (P1)

**Goal**: version mismatch is refused naming both versions and listed `needs-update`, unless the id is live.

**Independent test**: link a fake daemon announcing `PROTOCOL_VERSION - 1`.

**Tests first (red):**

- [ ] T026 [P] [US3] Unit tests in `crates/micold-core/src/orchestrator/registry.rs` for the version rows of the claim table: mismatch and no live link refuses `VersionMismatch` and records `needs-update` (older and newer); mismatch with a live link refuses and leaves record and link untouched (US3.4); a silent-link mismatched claim closes the silent link and records `needs-update`; `needs-update` then a current-version claim gives `online` (US3.5); `claim_failed(link, VersionMismatch)` after a reservation records `needs-update` the same way and never over a live record; `claim_failed` with `BuildMismatch` or `AuthRejected` changes no record and releases the reservation. Refusal message names both numbers (`announce.rs` test)
- [ ] T027 [P] [US3] `crates/micold-daemon/tests/orchestrator/version_mismatch.rs`: a daemon announcing `PROTOCOL_VERSION - 1` gets `Refused{VersionMismatch}` with both numbers (SC-003), is listed `needs-update` with both versions, other daemons stay `online`, a current-version relink turns it `online`; a daemon announcing the current number but a different schema hash is refused at `Hello` and recorded `needs-update`; a `BuildMismatch` or `AuthRejected` `Hello` refusal leaves the registry unchanged; an old-format preamble is still readable (golden fixture)

**Implementation (green):**

- [ ] T028 [US3] `Registry::claim` link-state model (`Reserved`/`Live`/`Silent`/`Closed`: which state an id's link is in) and its version rows, and `Registry::claim_failed` (reasons `VersionMismatch`, `BuildMismatch`, `AuthRejected`, `Dropped`) in `crates/micold-core/src/orchestrator/registry.rs`; listing carries the orchestrator's version for `needs-update` records
- [ ] T029 [US3] `crates/micold-daemon/src/orchestrator/link.rs`: write `LinkVerdict::Refused` and close on a refused claim; map a `handshake::evaluate` refusal after `Accepted` to `claim_failed` and send `DaemonMsg::Refused`. Needs T028
- [ ] T030 [US3] Docs: `needs-update` meaning and the refusal text in `docs/daemon.md` and `docs/user-guide/orchestrator-mode.md`

**Checkpoint**: M3 deliverable works.

## Phase 6: User Story 4 — One daemon id, one link (P2)

**Goal**: a rival claim on a live or reserved id is refused `AlreadyLinked`; closed, silent and restored ids accept.

**Independent test**: two links with one id.

**Tests first (red):**

- [ ] T031 [P] [US4] Unit tests in `crates/micold-core/src/orchestrator/registry.rs` for the rest of the decision table: live link refuses `AlreadyLinked` with the record untouched (US4.1); closed transport accepts and replaces (US4.2); silent link accepts, the old token becomes `Stale` and its late frames cannot move the new record (US4.3, US3 edge); restored-without-link accepts (US4.4); a silent old link that answers during a rival's reservation makes `confirm` refuse and leaves the old link live; a reserved link refuses a rival `AlreadyLinked` for at most `LIVENESS_DEADLINE`, then `claim_failed(Dropped)` or expiry releases it with no record change; two simultaneous claims through one `Mutex<Registry>` give exactly one `Reserved` (SC-004)
- [ ] T032 [P] [US4] `crates/micold-daemon/tests/orchestrator/one_link_per_id.rs`: second link refused `AlreadyLinked` while the first stays `online`; replacement after a closed transport; a silent link is closed when a new link takes over; a link that never sends `Hello` is released at the deadline (controllable clock) and a rival is then accepted; two fake daemons dial one id concurrently, repeated 50 times, exactly one `Accepted` each time (SC-004 on the wire)

**Implementation (green):**

- [ ] T033 [US4] Complete `Registry::claim` for current-version claims (live and reserved `AlreadyLinked` refusals, closed/silent replacement, reservation expiry at `LIVENESS_DEADLINE`, `confirm` refused when the silent old link answered during the reservation); the link-state model is from T028 in `crates/micold-core/src/orchestrator/registry.rs`
- [ ] T034 [US4] `crates/micold-daemon/src/orchestrator/link.rs`: `Hello` read timeout `LIVENESS_DEADLINE` calling `claim_failed(Dropped)`, connection close calling `claim_failed(Dropped)`, and the loop ends when its `LinkToken` is replaced. Needs T033
- [ ] T035 [US4] Docs: the one-link-per-id rule and the up-to-9 s reservation window in `docs/daemon.md` and `docs/user-guide/orchestrator-mode.md`

**Checkpoint**: M4 deliverable works.

## Phase 7: User Story 5 — The registry survives an orchestrator restart (P2)

**Goal**: the registry is persisted atomically and reloaded `offline`; a corrupt file fails start; the orchestrator holds no session.

**Independent test**: join two daemons, restart on the same state dir.

**Tests first (red):**

- [ ] T036 [P] [US5] Unit tests in `crates/micold-core/src/orchestrator/store.rs` and `registry.rs`: `JsonRegistryStore` round-trips `{ "version": 1, "daemons": [...] }`; an absent file loads empty; an unparsable or unreadable file returns `StoreError::Corrupt { path }` and is not touched; a torn write leaves the old or the new file (through `owner_only::write_with`); a failed save through `MemoryRegistryStore` leaves memory correct and the stored copy intact; `Registry::restore` marks every record `offline` (a saved `needs-update` too) with no link; mutating calls return sequence-stamped snapshots; the saver skips a snapshot older than the last saved (two out-of-order saves leave the newer file)
- [ ] T037 [P] [US5] `crates/micold-daemon/tests/orchestrator/restart.rs`: two joined daemons, stop, start on the same dir: same records listed `offline` (SC-005); a relink turns one `online` and updates, not duplicates, its record; the orchestrator spawns nothing and holds no session state to lose (FR-009, by construction: asserted by there being no session code path, with the fake daemon's own session fixture untouched); a corrupt `orchestrator/registry.json` makes the start fail non-zero with `orchestrator registry <path> is unreadable: <reason>` and the file unchanged; a failed save is logged to stderr while links keep working

**Implementation (green):**

- [ ] T038 [US5] `RegistryStore` trait, `JsonRegistryStore` (`orchestrator/registry.json` under `micold_core::paths::data_dir()`, `owner_only::write_with`), `MemoryRegistryStore` with an injectable save failure in `crates/micold-core/src/orchestrator/store.rs`; `Registry::restore`
- [ ] T039 [US5] `crates/micold-daemon/src/orchestrator/mod.rs`: load the store at start (corrupt: error naming the file, non-zero exit via `main.rs`), save the snapshot returned by each mutating registry call after unlocking, log a failed save to stderr, through the single saver that skips older snapshots. Needs T038
- [ ] T040 [US5] Docs: registry file location, restart behaviour (all `offline` until relink) and the corrupt-file message in `docs/daemon.md` and `docs/user-guide/orchestrator-mode.md`

**Checkpoint**: M5 deliverable works.

## Phase 8: Polish and cross-cutting

- [ ] T041 [P] Source-level test `crates/micold-daemon/tests/orchestrator/no_outbound.rs`: fails on any whole-word `connect`, `Stream::connect` or `open_pipe` call (word-boundary regex, comments stripped) under `crates/micold-daemon/src/orchestrator/` and `crates/micold-core/src/orchestrator/` (FR-008)
- [ ] T042 [P] Linux-only test in `crates/micold-daemon/tests/orchestrator/netns.rs` that re-runs the `link_and_list` and `heartbeat` tests (test binary re-executed with `unshare -rn`) (outbound connections blocked) and skips with a stated reason where unprivileged user namespaces are unavailable (FR-008, scope stated in the test)
- [ ] T043 Run `quickstart.md` Part A and B against the finished build and record the results; confirm `docs/daemon.md` and the user guide describe the whole feature and `docs/SUMMARY.md` links the page

## Dependencies and order

- Phase 1 then Phase 2 then Phases 3 to 7 in order (each builds on the registry and link of the one before: US2 needs the link loop, US3 and US4 extend `claim`, US5 adds the store around `run`); Phase 8 last.
- Inside a phase: tests, run red, then code. `[P]` tasks touch disjoint files.
- T021 (wire bump) precedes T022 and T024. T028 precedes T029. T033 precedes T034. T038 precedes T039.

## Implementation Strategy

MVP is M1 (Setup, Foundational, US1). Each later story is one milestone that merges on its own; `main` is green after each. The wire bump lands with US2 so no earlier milestone changes the protocol.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Orchestrator starts and lists a daemon 🎯 MVP

- **Tasks**: T001–T016
- **Deliverable**: `micold-daemon orchestrator` starts, accepts a daemon link through the preamble and `Hello`/`Welcome`, lists it with every field and hosts no session; `micold-daemon` without the argument is unchanged.
- **Satisfies**: US1 acceptance scenarios 1–3; FR-001, FR-002, FR-003, FR-010; SC-001, SC-006
- **Verify**: `mise run test-core`; `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator`; quickstart Part B steps 1, 2, 7
- **Depends on**: —
- **Tier**: full

### M2 — Heartbeat, offline and online again

- **Tasks**: T017–T025
- **Deliverable**: the orchestrator pings each link, shows a silent daemon `offline` within 10 s of its last frame and `online` on its next frame; protocol version is 81 with `LinkPing`/`LinkPong`.
- **Satisfies**: US2 acceptance scenarios 1–3; FR-005; SC-002
- **Verify**: `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator heartbeat`; `mise run test-core`; quickstart Part B step 3
- **Depends on**: M1
- **Tier**: full

### M3 — Older-protocol daemons listed needs-update

- **Tasks**: T026–T030
- **Deliverable**: a daemon on another protocol version is refused naming both versions and listed `needs-update`; a live record is never overwritten; a current-version relink turns it `online`.
- **Satisfies**: US3 acceptance scenarios 1–5; FR-006; SC-003
- **Verify**: `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator version_mismatch`; quickstart Part B step 4
- **Depends on**: M2
- **Tier**: full

### M4 — One live link per daemon id

- **Tasks**: T031–T035
- **Deliverable**: a second link on a live or reserved id is refused `AlreadyLinked`; closed, silent and restored ids accept and replace; a reservation lasts at most 9 s; of two simultaneous claims exactly one wins.
- **Satisfies**: US4 acceptance scenarios 1–4; FR-007; SC-004
- **Verify**: `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator one_link_per_id`; `mise run test-core`
- **Depends on**: M3
- **Tier**: full

### M5 — Registry survives a restart

- **Tasks**: T036–T040
- **Deliverable**: the registry is saved atomically and reloaded with every daemon `offline`; a corrupt file fails start naming the file; the orchestrator holds no session state.
- **Satisfies**: US5 acceptance scenarios 1–3; FR-004, FR-009; SC-005
- **Verify**: `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator restart`; quickstart Part B steps 5, 6
- **Depends on**: M4
- **Tier**: full

### M6 — No outbound connections, proven

- **Tasks**: T041–T043
- **Deliverable**: a source-level test and a Linux network-namespace run prove the orchestrator opens no outbound connection; quickstart Parts A and B recorded.
- **Satisfies**: FR-008; edge case cross-platform
- **Verify**: `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator no_outbound`; `mise run gate`
- **Depends on**: M5
- **Tier**: full
