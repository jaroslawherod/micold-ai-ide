# Tasks: Daemon-qualified refs and the routed envelope (protocol bump)

**Input**: `specs/1723-daemon-qualified-refs-routed-envelope/` (spec.md, plan.md, data-model.md, research.md, contracts/, quickstart.md)

Format: `- [ ] T### [P?] [US?] description with file path`. `[P]` = disjoint files. Tests lead the code they cover (Constitution I): each test task is red before its implementation task. Use `mise run check` while editing and targeted `scripts/build-lock.sh cargo test -p <crate> <name>`; `mise run gate` before a push. Crate paths are under `crates/`.

## Phase 1: Setup

- [ ] T001 Register an empty `routing` module in `micold-core/src/protocol/mod.rs` and create `micold-core/src/protocol/routing.rs` (module doc only), so later tasks compile.

## Phase 2: Foundational (blocks all stories)

- [ ] T002 [P] Test `WireDaemonId` in `micold-core/src/protocol/routing.rs` (unit): valid ids; invalid ids (empty, 64 chars, uppercase, leading `.`/`-`/`_`, space, non-ASCII); JSON and postcard round trip; `Deserialize` rejects an invalid id; `Display`, `Eq`, `Hash`, `Ord`.
- [ ] T003 Implement `WireDaemonId` in `micold-core/src/protocol/routing.rs`: newtype over `String`, "1 to 63 chars, `[a-z0-9][a-z0-9._-]*`"; `new`/`FromStr`/`Deserialize` validate; serialises as a string. Not `daemons::DaemonId`.
- [ ] T004 [P] Test the `[daemon] id` setting in `micold-core/src/daemon_config/` (unit): absent gives `local`; a valid id is read; an invalid id is a config error naming the key.
- [ ] T005 Add optional `id` to `[daemon]` in `micold-core/src/daemon_config/mod.rs`, validated through `WireDaemonId` in `micold-core/src/daemon_config/validate.rs`; absent means `local`.

## Phase 3: User Story 1 - A ref names its daemon and survives a round trip (P1) 🎯 MVP

**Goal**: session and worktree rows carry their daemon id; a ref built from a row equals the decoded ref.
**Independent test**: `cargo test -p micold-core --test protocol_roundtrip` and `-p micold-daemon --test unwrapped_client`.

- [ ] T006 [US1] Tests in `micold-core/tests/protocol_roundtrip.rs`: `SessionRef`, `WorktreeRef`, `ProjectRef`, `SessionSummary` and `WorktreeSnapshot` round trip on JSON and postcard; a ref from `qualified()` equals the decoded ref; the same local ref under two daemon ids is unequal (and hashes apart). Add fixture constructors that default `daemon` to `local`, extending the existing `sample_summary` builders.
- [ ] T007 [US1] Add `SessionRef { daemon, id: SessionId }`, `WorktreeRef { daemon, project: PathBuf, dir_name }`, `ProjectRef { daemon, path: PathBuf }` to `micold-core/src/protocol/routing.rs` (derived `Eq`/`Hash`/serde).
- [ ] T008 [US1] Add `daemon: WireDaemonId` (no `serde(default)`) to `SessionSummary` and `WorktreeSnapshot` in `micold-core/src/protocol/messages.rs`, with `SessionSummary::qualified() -> SessionRef` and `WorktreeSnapshot::qualified(project: &Path) -> WorktreeRef`.
- [ ] T009 [US1] Fix every constructor of `SessionSummary` and `WorktreeSnapshot` the compiler reports across `micold-core`, `micold-daemon` and `micold-client` (tests and fixtures use the T006 constructors).
- [ ] T010 [US1] Daemon test `unwrapped_client` in `micold-daemon/tests/`: a plain client against one daemon still works (FR-010) and its session and worktree rows carry the configured `daemon.id` (default `local`).
- [ ] T011 [US1] Fill `daemon` from `daemon.id` in `micold-daemon/src/catalog.rs` (`session_summary`, the `WorktreeSnapshot` builders) and `micold-daemon/src/state.rs`.
- [ ] T012 [US1] Document the `daemon.id` setting in `docs/user-guide/daemon-configuration.md`.

## Phase 4: User Story 2 - A routed envelope carries any client message to a named daemon (P1)

**Goal**: `Kind::Routed` frames carry `{target, conn, seq, msg}`; bad targets and routing mix-ups are rejected.
**Independent test**: `scripts/build-lock.sh cargo test -p micold-core --lib routing` and `--test routed_codec`.

- [ ] T013 [US2] Unit tests in `micold-core/src/protocol/routing.rs` for `RoutedEnvelope::from_json_slice`: missing, empty, malformed and double-wrapped target give `MissingTarget`, `InvalidTarget`, `NestedEnvelope`; `RoutedEnvelope::owns(&self, row_daemon)` returns `RoutingError::ForeignRow` for a row whose daemon differs from the target (the check for the later daemon-to-client wrap, #1725; helper only here); the payload is not interpreted when the target is bad; `ConnectionId` and `Sequence` round trip incl. `u64::MAX`; same `conn` with two targets are distinct.
- [ ] T014 [US2] Implement `ConnectionId(u64)`, `Sequence(u64)`, `RoutedEnvelope<M> { target, conn, seq, msg }` with `owns(&self, row_daemon)`, and `RoutingError` in `micold-core/src/protocol/routing.rs`; `from_json_slice` checks the target before the payload; Display names the target problem.
- [ ] T015 [US2] New `micold-core/tests/routed_codec.rs`: an envelope around each `ClientMsg` variant (reuse `sample_client_msgs`) round trips through `WireCodec`; a `Routed` frame on a `Plain` codec and a `Control` frame on a `Routed` codec give `CodecError::WrongRouting`; grid frames pass in both; the handshake completes on a plain codec, then `set_routing(Routing::Routed)` rejects `Control` and accepts `Routed`; a `ClientCodec` receiving `Routed` gets `WrongRouting`.
- [ ] T016 [US2] Add `Kind::Routed = 2` (body JSON, `encoding` must be `Json`) in `micold-core/src/protocol/envelope.rs`.
- [ ] T017 [US2] In `micold-core/src/protocol/codec.rs`: `Frame::Routed(RoutedEnvelope<C>)`, `Routing { Plain, Routed }` (default `Plain` in `WireCodec::new()`), `with_routing`, `set_routing`, `CodecError::WrongRouting { expected, got }` and `CodecError::Routing(RoutingError)`.
- [ ] T018 [US2] Give every `Frame` match the compiler reports an arm (rejecting, or already `if let`): `micold-daemon/src/server.rs` (~884, close the connection), `micold-client/src/daemon.rs` (~1154, protocol violation), `micold-core/src/connect.rs`, `micold-core/src/provision/mod.rs`, `micold-core/src/kube/fake_host.rs`, daemon test supports.
- [ ] T019 [US2] Daemon test in `micold-daemon/tests/`: a routed frame sent on a plain connection closes it and delivers nothing.

## Phase 5: User Story 3 - A mismatched peer is refused naming both versions (P2, ships in M1)

**Goal**: `PROTOCOL_VERSION` is 78, the schema hash covers `routing.rs`, the refusal names both versions.
**Independent test**: `cargo test -p micold-core --test handshake` and `--test schema_hash`.

- [ ] T020 [US3] Tests: in `micold-core/tests/handshake.rs` a handshake with `PROTOCOL_VERSION - 1` is refused and the text contains both numbers; in `micold-core/tests/schema_hash.rs` changing the `routing.rs` input changes the hash.
- [ ] T021 [US3] Hash `routing.rs` in `micold-core/build.rs` and `micold-core/src/protocol/hashing.rs`.
- [ ] T022 [US3] Bump `PROTOCOL_VERSION` 77 to 78 in `micold-core/src/protocol/version.rs` and update every test or doc that asserts 77.

## Phase 6: User Story 4 - The contract is written down (P2)

**Goal**: one rule for "same project" is derived by the daemon and written in the contract and `docs/daemon.md`.
**Independent test**: `cargo test -p micold-core --lib project_identity` and `--test contract_example`.

- [ ] T023 [P] [US4] Unit tests in `micold-core/src/project_identity.rs`: `git@host:o/r.git` and `https://host/o/r` both give `remote:host/o/r`; `ssh://` and port forms; host lowercased; `origin` wins, else first remote in config order; override gives `override:<value>` ("1 to 128 chars, no whitespace or control characters") and wins over a remote; no remote gives `local:<daemon>:<path>`; two no-remote repos differ; `C:/x` and `C:\x` count as no remote.
- [ ] T024 [P] [US4] Tests for `Git::config_local_get(repo, key)` in `micold-core/src/git.rs`: `GitCli` on a temp repo (set, unset) and `FakeGit`; local config only.
- [ ] T025 [US4] Add `Git::config_local_get` to the trait, `GitCli` (pattern `remote_list`) and `FakeGit` in `micold-core/src/git.rs`; make `github::split_remote` `pub(crate)` in `micold-core/src/github.rs`.
- [ ] T026 [US4] Implement `ProjectIdentity` and `ProjectIdentity::derive(remotes, override, daemon, project_path)` in `micold-core/src/project_identity.rs`, registered in `micold-core/src/lib.rs`; a single-letter host is rejected before `split_remote`; tagged `remote:`/`override:`/`local:` strings.
- [ ] T027 [US4] Tests: `ProjectSnapshot` with `daemon` and `identity` round trips on both wires and `qualified()` equals the decoded `ProjectRef` (`micold-core/tests/protocol_roundtrip.rs`); a daemon test in `micold-daemon/tests/` that its project row carries the derived identity and the `micold.projectId` override.
- [ ] T028 [US4] Add `daemon` and `identity: ProjectIdentity` to `ProjectSnapshot` and `ProjectSnapshot::qualified() -> ProjectRef` in `micold-core/src/protocol/messages.rs`; fix every constructor the compiler reports; fill both in `micold-daemon/src/catalog.rs` from `Git::remote_list` and `config_local_get("micold.projectId")`.
- [ ] T029 [US4] Document `micold.projectId` (how to set it on each clone, what happens without a remote) in `docs/user-guide/daemon-configuration.md`.
- [ ] T030 [US4] Test `contract_example` in `micold-core/tests/` reading `specs/orchestrator/contracts/routed-envelope.md`: its worked-example frame decodes to the stated envelope; every `RoutingError` variant and envelope field is listed.
- [ ] T031 [US4] Write `specs/orchestrator/contracts/routed-envelope.md` (new file only; do not edit other files in that directory): byte-level fields with type and range, error variants, one worked example frame, sequence-number note (#1730), remote normalisation steps, the project-identity rule and User Story 4 examples.
- [ ] T032 [P] [US4] Add a section to `docs/daemon.md`: daemon id, qualified refs, the envelope, its rejection rule, protocol number 78, the project-identity rule with its reason and failure case.

## Dependencies and order

Setup -> Foundational -> US1 -> US2 -> US3 -> US4. The protocol bump and hash input (T021, T022) ship in M1 with the first wire change, so `main` never carries a changed wire under 77; US3's tests (T020) follow them. US2 needs `WireDaemonId` (T003). US3's hash input needs `routing.rs` (T001). US4's `ProjectSnapshot` change needs US1's `WireDaemonId` and the daemon fill pattern; its contract documents US2's envelope.

Parallel: T002 with T004; T023 with T024; T032 with T031.

## Implementation Strategy

Protocol 78 is the number for the whole feature: intermediate `main` commits (M1 to M3) differ in wire layout under 78 and are told apart only by `SCHEMA_HASH`; 78 is not released before M4.

MVP is US1 (M1): daemon ids and qualified rows on `main`. Each later story ships alone. Polish has no code: quickstart results and task ticks belong to the close unit. Test list: derived inline from the Test strategy table in plan.md (no separate `tdd/test-list.md`).

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Daemon id, qualified rows, protocol 78 🎯 MVP

- **Tasks**: T001–T012, T020–T022
- **Deliverable**: session and worktree rows carry the daemon's configured id (`daemon.id`, default `local`), qualified refs built from them round trip and differ across daemons, the protocol number is 78 with `routing.rs` in the schema hash, and a previous-version peer is refused naming both versions.
- **Satisfies**: US1 acceptance scenarios 1–3 (session and worktree; the project row follows in M3); US3 acceptance scenario 1 (folded in: the bump must travel with the first wire change, and US3 alone is one test); FR-001, FR-002, FR-003 (session, worktree), FR-007, FR-010; SC-003
- **Verify**: `scripts/build-lock.sh cargo test -p micold-core --lib routing`, then `--test protocol_roundtrip`, `--test handshake`, `--test schema_hash`, and `-p micold-daemon --test unwrapped_client` (one `cargo test` per target)
- **Depends on**: —
- **Tier**: full

### M2 — Routed envelope frame

- **Tasks**: T013–T019
- **Deliverable**: a `Kind::Routed` frame carries any `ClientMsg` with target, connection id and sequence; frames without a valid target and plain/routed mix-ups are rejected.
- **Satisfies**: US2 acceptance scenarios 1–3; FR-004, FR-005, FR-006; SC-001, SC-002
- **Verify**: `scripts/build-lock.sh cargo test -p micold-core --lib routing`, then `--test routed_codec`, and the daemon routed-on-plain test
- **Depends on**: M1
- **Tier**: full

### M3 — Project identity

- **Tasks**: T023–T029
- **Deliverable**: project rows carry the owning daemon and a derived project identity (normalised remote, `micold.projectId` override, or per-daemon local), the same for `git@host:o/r.git` and `https://host/o/r`.
- **Satisfies**: US1 acceptance scenario 1 (project row), US4 acceptance scenario 2; FR-003 (project), FR-008
- **Verify**: `scripts/build-lock.sh cargo test -p micold-core --lib project_identity`, then `--test protocol_roundtrip`, and the daemon project-row test
- **Depends on**: M1
- **Tier**: full

### M4 — Contract and daemon docs

- **Tasks**: T030–T032
- **Deliverable**: `specs/orchestrator/contracts/routed-envelope.md` and `docs/daemon.md` describe the fields, errors, protocol 78 and the project-identity rule; the contract's worked example frame decodes in a test.
- **Satisfies**: US4 acceptance scenario 1; FR-009; SC-004
- **Verify**: `scripts/build-lock.sh cargo test -p micold-core --test contract_example`
- **Depends on**: M2, M3
- **Tier**: full
