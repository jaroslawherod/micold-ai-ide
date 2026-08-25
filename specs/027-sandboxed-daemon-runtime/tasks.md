---
description: "Task list for feature 027 — The Session Daemon in a Sandbox"
---

# Tasks: The Session Daemon in a Sandbox

**Input**: Design documents from `/specs/027-sandboxed-daemon-runtime/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/](./contracts/), [quickstart.md](./quickstart.md)

**Tests**: Per Constitution Principle I (NON-NEGOTIABLE), test tasks are mandatory and come **before**
the implementation they cover. Every `T…` marked *(test)* must be written, run, and seen to **fail**
before its implementing task begins.

**Documentation**: Per Principle VII, each user-facing story carries its user-guide task in the same
change. A story is not done until its docs exist.

**Cross-platform**: Per Principle VI, everything in `micold-core` is platform-agnostic and tested on
all three platforms via the **fake runtime binary** (T004). Real-runtime coverage is Linux CI plus
`quickstart.md` §B.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: parallelisable — different file, no dependency on an incomplete task
- **[Story]**: US1 … US6, matching spec.md's user stories

## Path Conventions

Existing three-crate workspace. Paths are repo-relative:
`crates/micold-core/src/…`, `crates/micold-client/src/…`, `crates/micold-daemon/src/…`,
tests in each crate's `tests/` directory.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: the scaffolding every later phase compiles against, plus the test harness that makes
Principle I affordable for a feature whose subject is a subprocess.

- [X] T001 Create the module skeleton in `crates/micold-core/src/sandbox/` — `mod.rs`, `placement.rs`, `runtime.rs`, `argv.rs`, `parse.rs`, `exec.rs`, `image.rs`, `pathmap.rs`, `dialect/mod.rs` — each with its doc comment and `todo!()` bodies, wired into `crates/micold-core/src/lib.rs`
- [X] T002 [P] Create `packaging/sandbox/Containerfile` building the daemon plus shell, git and the AI CLI, and `packaging/sandbox/README.md` documenting build, publish, and `docker save`/`load` export
- [X] T003 [P] Add `[tasks.image]` to `mise.toml` building a `:dev` image from the working tree (FR-024c), routed through `scripts/build-lock.sh` like the other build tasks
- [X] T004 Build the fake runtime harness in `crates/micold-core/src/sandbox/exec.rs` — `CommandRunner` injected, `RecordingRunner` recording argv and replaying canned output in-process. **Deviates from contracts/container-runtime.md §"The fake runtime"**, which specified a binary first on `PATH`: `PATH` is process-global and cargo runs tests as parallel threads, so that harness races by construction (and edition 2024 marks `set_var` `unsafe`). Everything the conformance suite asserts sits above the seam and is unchanged; `SystemRunner` keeps one real-spawn test
- [X] T005 [P] Add canned runtime fixtures in `crates/micold-core/tests/fixtures/runtime/` — `docker_version.json`, `docker_inspect_container.json`, `docker_inspect_image.json`, `podman_version.json`, plus failure fixtures for daemon-down, image-not-found, permission-denied and truncated JSON
- [X] T006 [P] Add the Linux-only real-runtime CI job to the workflow under `.github/workflows/`, running the `sandbox_real_*` tests behind a feature flag so the default matrix stays runtime-free

**Checkpoint**: the workspace compiles, and a test can assert on argv without Docker installed.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: the seam itself — placement, transport, persistence and the runtime trait. Every user
story depends on all of it.

**⚠️ CRITICAL**: no user story work begins until this phase is complete.

### Placement and connection

- [X] T007 *(test)* Write `crates/micold-core/tests/placement.rs` — resolution is pure (P-1), never substitutes a placement (P-2), and a fallback is not representable as a resolution outcome (P-3), per data-model.md §1
- [X] T008 Implement `Placement`, `RemotePlacement` and resolution in `crates/micold-core/src/sandbox/placement.rs`, including the non-constructible `Remote` variant FR-003a requires
- [X] T009 *(test)* Extend `crates/micold-core/tests/connect.rs` for `connect_or_start(placement)` — the host-process path is byte-for-byte unchanged, and a sandbox placement takes the new path
- [X] T010 Rename and generalise `connect_or_spawn` to `connect_or_start(placement)` in `crates/micold-core/src/connect.rs`, updating every call site
- [X] T011 Add the loopback-TCP endpoint alongside socket and named pipe in `crates/micold-core/src/endpoint.rs`, keeping the existing `0700`-directory contract intact for the host placement (R1)

### Protocol v5 → v6

- [X] T012 *(test)* Write `crates/micold-core/tests/protocol_auth.rs` — P-1 (right/wrong/absent token), P-3 (token in no log, argv or inspect output), P-6 (`PROTOCOL_VERSION` is 6, a v5 handshake is rejected before authenticating), per contracts/protocol-delta.md
- [X] T013 Implement the shared-secret token — generate, write `0600`, mount read-only, present, constant-time verify — in `crates/micold-core/src/protocol/auth.rs`
- [X] T014 Bump `PROTOCOL_VERSION` to 6 and emit `BUILD_FINGERPRINT` from `crates/micold-core/build.rs` into the generated file beside `SCHEMA_HASH`, declared in `crates/micold-core/src/protocol/version.rs`
- [X] T015 *(test)* Add P-4 to `crates/micold-core/tests/protocol_auth.rs` — a fingerprint mismatch refuses a `LocalBuild` image as `StaleDevImage` and accepts a `Registry` one (the asymmetry R8 requires)
- [X] T016 Implement the closed refusal enumeration and its remedies in `crates/micold-core/src/protocol/` — `ProtocolMismatch`, `SchemaMismatch`, `VersionMismatch`, `StaleDevImage`, `AuthRejected` (P-5)
- [X] T017 Teach `crates/micold-daemon/src/server.rs` to accept the loopback listener and verify the token, and `crates/micold-daemon/src/main.rs` to bind per placement and read its mounted secret when containerised
- [X] T018 Update `crates/micold-core/tests/schema_hash.rs` for the one deliberate hash move, with a comment naming this feature as the reason

### Settings v3 → v4

- [X] T019 *(test)* Write the v4 cases in `crates/micold-core/tests/settings_roundtrip.rs` — T-1 … T-8 from contracts/sandbox-settings-schema.md, with T-3 (credentials absent → **empty**) called out as a security property
- [X] T020 Add the nested `daemon` block to `Settings` in `crates/micold-core/src/settings.rs`, bump `SETTINGS_VERSION` to 4, and give every added field a serde default (S-1, S-2, S-4)
- [X] T021 Implement unknown-field preservation across load/save in `crates/micold-core/src/settings.rs` (S-5) — new in v4, since the flat schema never needed it
- [X] T022 Implement budget clamping in `crates/micold-core/src/sandbox/mod.rs` following the existing `clamp_scrollback` / `clamp_env_include_timeout` idiom (S-7, RB-1)

### The runtime seam

- [X] T023 *(test)* Write `crates/micold-core/tests/sandbox_runtime.rs` against the fake runtime — K-8 (each canned failure maps to its `RuntimeError` variant), K-9 (stop/remove/start idempotent), K-12 (malformed JSON classifies, never panics)
- [X] T024 Define the `ContainerRuntime` trait, `RuntimeKind`, `RuntimeVersion`, `RuntimeCapabilities`, `LimitSupport`, `IdentityMapping` and the closed `RuntimeError` in `crates/micold-core/src/sandbox/runtime.rs` (contracts/container-runtime.md §"The trait", C-6)
- [X] T025 Implement the single process-spawn shim in `crates/micold-core/src/sandbox/exec.rs` — the only impure code in the layer
- [X] T026 Implement `--format '{{json .}}'` parsing into typed facts in `crates/micold-core/src/sandbox/parse.rs`, with truncated and unexpected input classified rather than unwrapped

**Checkpoint**: placement resolves, the handshake authenticates, settings persist, and the trait exists with a testable fake behind it. User stories can now proceed.

---

## Phase 3: User Story 1 — The agent can only touch the project (P1) 🎯 MVP

**Goal**: sandboxed mode on, sessions run, and the sandbox genuinely cannot reach the host.

**Independent test**: enable sandboxed mode with defaults, start a session, and from its terminal
attempt to list the home directory, read a file outside the project, and inspect the host process
table — all three fail while the same commands against the project succeed.

### Tests first

- [X] T027 *(test)* [P] [US1] Write `crates/micold-core/tests/sandbox_argv.rs` — K-1 (argv is a pure function of the spec, identical across repeated builds) and K-4 (argv mounts equal the `MountSet` as sets)
- [X] T028 *(test)* [P] [US1] Add K-5 to `crates/micold-core/tests/sandbox_argv.rs` — on Linux/macOS specs every `ProjectMount` has `container == host` (M-2, the claim git's worktree metadata depends on)
- [X] T029 *(test)* [P] [US1] Add K-11 to `crates/micold-core/tests/sandbox_argv.rs` — the escalation denylist: no `--privileged`, `--cap-add`, `--pid=host`, `--network=host`, `seccomp=unconfined`, and no host path outside the `MountSet` (C-9)
- [X] T030 *(test)* [P] [US1] Add K-6 to `crates/micold-core/tests/sandbox_argv.rs` — the identity flag matches the dialect's `IdentityMapping` (C-4, R3)
- [X] T031 *(test)* [P] [US1] Write `crates/micold-core/tests/sandbox_credentials.rs` — the empty default shares nothing, each opt-in adds exactly its own mount and no other, and no free-text path can enter through the credentials field (N-1)
- [X] T032 *(test)* [P] [US1] Add K-10 to `crates/micold-core/tests/sandbox_runtime.rs` — `acquire_image` emits more than one progress callback for multi-layer canned output (C-8)

### Implementation

- [X] T033 [US1] Implement `SandboxProfile`, `CredentialShare` and the empty-by-default credential set in `crates/micold-core/src/sandbox/mod.rs` (SP-1, FR-004a/b)
- [X] T034 [US1] Implement `MountSet`, `ProjectMount`, `NamedVolume` and `SecretMount` in `crates/micold-core/src/sandbox/mod.rs` — only registered projects, no implicit home, no runtime socket (M-1, C-3)
- [X] T035 [US1] Implement host↔sandbox path identity in `crates/micold-core/src/sandbox/pathmap.rs` — identity on Linux/macOS, with the Windows boundary declared but unused until T099 (R2)
- [X] T036 [US1] Implement the mount and identity portions of argv construction in `crates/micold-core/src/sandbox/argv.rs`, pure and argument-driven (C-1)
- [X] T037 [US1] Implement Docker's dialect — flag names, defaults, `--user <uid>:<gid>` — in `crates/micold-core/src/sandbox/dialect/docker.rs` (FR-021, C-4)
- [X] T038 [US1] Implement `ImageRef` parsing, moving-tag detection and the pull/import/build decision in `crates/micold-core/src/sandbox/image.rs` (FR-024, FR-024a–c)
- [X] T039 [US1] Implement `acquire_image` with progress reporting, and `create`/`start`/`stop`/`remove`/`inspect` over the exec shim in `crates/micold-core/src/sandbox/runtime.rs` (C-7, C-8)
- [X] T040 [US1] Implement the sandbox lifecycle side of `connect_or_start` in `crates/micold-core/src/connect.rs` — probe, acquire, start, handshake — returning classified failures rather than falling back (P-2)
- [X] T041 [US1] Add the client-side sandbox lifecycle state in `crates/micold-client/src/features/sandbox.rs` and the off-thread runtime calls, progress and failure-to-`Message` glue in `crates/micold-client/src/shell/sandbox.rs`
- [X] T042 [US1] Add the enable/disable control and the restart confirmation to the existing settings surface in `crates/micold-client/src/ui/settings_form.rs`, as a temporary home until US3 replaces it
- [ ] T043 [US1] Show `StageProgress` during image acquisition in `crates/micold-client/src/ui/`, driven by the T039 callbacks (SC-004)
- [X] T044 [P] [US1] Write `docs/user-guide/sandboxed-daemon.md` — enabling, what the sandbox can and cannot see, the credential opt-ins and their default-off posture, and offline image import (Principle VII, FR-024a)
- [X] T045 [US1] Ran quickstart.md **§B.2** (plus §B.3 and §B.4) against Docker 29.5.1 and recorded it in `specs/027-sandboxed-daemon-runtime/evidence/us1-isolation.md` — the boundary, file ownership, network posture, limits and token non-leakage all hold. **§B.1 (first enable, cold, through the GUI) is still outstanding**: it needs the application running at a display, and it depends on T043's progress indicator to be meaningful

**Checkpoint**: the feature's core claim is demonstrable. This is the MVP.

---

## Phase 4: User Story 2 — The service keeps its promises inside the box (P1)

**Goal**: nothing the daemon already guaranteed is lost by moving it into a container.

**Independent test**: with sandboxed mode on, create a worktree-backed session, produce scrollback,
close the app, confirm the session still runs, reopen and re-attach; recreate the sandbox and confirm
the catalogue survives; reboot with survival opted out and opted in and confirm each.

### Tests first

- [X] T046 *(test)* [P] [US2] Written as `crates/micold-core/tests/sandbox_parity.rs` — daemon state is mounted from somewhere the container does not own, and two independently built argv for one profile mount the same state, so create/remove/create cannot land elsewhere (FR-011). **Restated**: the mount is a host bind rather than a named volume, per T050's deviation; the property the task was protecting is unchanged
- [X] T047 *(test)* [P] [US2] Add restart-policy cases to `crates/micold-core/tests/sandbox_argv.rs` — survival enabled yields `--restart unless-stopped`, disabled yields `--restart no`, on all three platforms' specs (R6, FR-014a/b)
- [X] T048 *(test)* [P] [US2] Add port-publishing cases to `crates/micold-core/tests/sandbox_argv.rs` — a user-exposed port appears as a published port, and the daemon's own control port is always published to loopback
- [X] T049 *(test)* [P] [US2] Extend `crates/micold-daemon/tests/` for reconnect across a client restart while sandboxed, asserting the session catalogue and scrollback are intact (FR-014)

### Implementation

- [X] T050 [US2] Implement the daemon state mount in `crates/micold-core/src/sandbox/mod.rs` and `argv.rs` so `projects.json`, per-project state and logs survive container recreation (FR-011). **Deviates from data-model.md rule M-3**, which specified a runtime-managed named volume: the client has to read the registered project list *before* the sandbox exists to know what to mount, and inside a volume that file is unreachable from the host — so the second start would mount a stale list. A bind mount of the host state directory satisfies FR-011 just as well and keeps one source of truth
- [X] T051 [US2] Map the existing session-survival opt-in onto the runtime's restart policy in `crates/micold-core/src/sandbox/argv.rs`, and route `logout_survival.rs`'s outcome through the placement so the setting keeps one name and one meaning (R6)
- [X] T052 [US2] Report `SurvivalOutcome::Enabled` for the sandboxed placement on macOS and Windows in `crates/micold-core/src/logout_survival.rs`, where the host-process path reports `Unsupported` (FR-014b — the bar the spec raises deliberately)
- [X] T053 [US2] Implement user-exposed port publishing in `crates/micold-core/src/sandbox/argv.rs` and its setting in `crates/micold-core/src/sandbox/mod.rs` (US2 scenario 8)
- [X] T054 [US2] Verify worktree creation inside the sandbox lands on the host under `<project>/.claude/worktrees/` and add the assertion to `crates/micold-daemon/tests/` (US2 scenario 2, Principle III)
- [X] T055 [US2] Ensure git author identity resolves inside the sandbox — via the `GitConfig` credential opt-in when enabled, and with a named, actionable failure when a commit is attempted without it (US2 scenario 7, US1 scenario 6)
- [X] T056 [US2] Confirm terminal behaviour parity — rendering, resize, title, bell, clipboard — across the sandboxed transport, extending `crates/micold-client/tests/` where the transport is observable (US2 scenario 6, SC-001)
- [X] T057 [US2] Implement stale-sandbox detection at startup in `crates/micold-core/src/sandbox/runtime.rs` — a container from a previous or mismatched version is replaced, not attached to and not accumulated beside (US6 scenario 5, FR-024d)
- [X] T058 [P] [US2] Document the placement model and the sandboxed lifecycle in `docs/daemon.md`
- [X] T059 [US2] Ran the state-persistence and end-to-end items and recorded them in `specs/027-sandboxed-daemon-runtime/evidence/us2-parity.md` — including the real handshake against a container. **The reboot items are outstanding**: whether the host brings the container back is not something a test can establish, and the mechanism is asserted instead in `sandbox_parity.rs` and `logout_survival.rs`
- [X] T060 [US2] Run quickstart.md §B.7 — `mise run image`, the `StaleDevImage` refusal, and the `docker save`/`load` offline path (FR-024a/c/d, Principle IV)

**Checkpoint**: sandboxed mode costs the user nothing they had before.

---

## Phase 5: User Story 3 — Settings becomes a view with sections (P2)

**Goal**: a full-surface Settings view with a navigation rail, every existing setting preserved in
exactly one section, and every daemon setting together in one.

**Independent test**: open Settings, visit each section, confirm every pre-existing setting is
present, editable and saved, reachable in at most one section change; navigate by keyboard alone;
check both themes at the supported window sizes.

### Tests first

- [X] T061 *(test)* [P] [US3] Write `crates/micold-client/tests/settings_sections.rs` — every setting that existed before this feature is present in exactly **one** section (US3 scenario 5, FR-028 — the migration's real risk)
- [X] T062 *(test)* [P] [US3] Add cross-section draft cases to `crates/micold-client/tests/features_settings.rs` — unsaved edits survive a section change, a save applies every visited section together, and a validation failure reports against its field with that section shown (US3 scenarios 2 and 3)
- [X] T063 *(test)* [P] [US3] Extend `crates/micold-client/tests/anatomy_call_sites.rs` — `section_list` is built in `ui/material/` with the chainable-builder-into-`Element` API, not privately in the feature (Principle VIII). **Restated**: the assertions landed in `tests/settings_sections.rs` instead. `anatomy_call_sites.rs` guards *token* bindings at call sites — that a spacing or colour is named rather than spelled — and knows nothing about composition; the rail's home and its builder shape are a claim about the settings view, which is what `settings_sections.rs` is about. The property the task was protecting is unchanged and asserted
- [X] T064 *(test)* [P] [US3] Confirm `crates/micold-client/tests/idle_requests_no_frames.rs` covers the new view — no repainting at rest with Settings open (the regression a view rewrite is most likely to cause)

### Implementation

- [X] T065 [US3] Build the `section_list` primitive in `crates/micold-client/src/ui/material/section_list.rs` with the mandated builder API and its own unit tests (Principle VIII, FR-026a)
- [X] T066 [US3] Create the full-surface view in `crates/micold-client/src/ui/settings_view.rs`, composing `NavigationDrawer` for the rail and `section_list` for the sections (FR-026)
- [X] T067 [P] [US3] Move the appearance settings into `crates/micold-client/src/ui/settings/appearance.rs`
- [X] T068 [P] [US3] Move the terminal settings into `crates/micold-client/src/ui/settings/terminal.rs`
- [X] T069 [P] [US3] Move the environment-include settings into `crates/micold-client/src/ui/settings/environment.rs`
- [X] T070 [US3] Build the daemon section in `crates/micold-client/src/ui/settings/daemon.rs` — placement, runtime, image, and the sandbox controls promoted out of T042's temporary home (FR-027)
- [X] T071 [US3] Grow `SettingsDraft` to hold per-section drafts with validation beside the type in `crates/micold-client/src/features/settings.rs` (US3 scenarios 2 and 3)
- [X] T072 [US3] Render each active credential opt-in individually while it is active, in `crates/micold-client/src/ui/settings/daemon.rs` (FR-004c, N-2)
- [X] T073 [US3] Route the view into the shell and remove `crates/micold-client/src/ui/settings_form.rs`'s modal, updating `crates/micold-client/src/ui/mod.rs`'s `view` signature
- [X] T074 [P] [US3] Restructure `docs/user-guide/settings.md` for the sectioned view
- [ ] T075 [US3] Run quickstart.md §B.6 with the repo's `visual-pass` skill — both themes, keyboard-only navigation, narrowest supported width — and record the result

**Checkpoint**: Settings is a view, and it is where the rest of this feature is configured from.

---

## Phase 6: User Story 4 — Limits the developer sets (P2)

**Goal**: processor, memory, process-count, storage and network bounds the user chooses, enforced
where the runtime can and shown as unavailable with a reason where it cannot.

**Independent test**: set a low memory and processor budget, run a workload that would otherwise
exhaust the host, and observe the host stays responsive, the limit holds, and the app explains what
happened rather than showing an unexplained dead session.

### Tests first

- [X] T076 *(test)* [P] [US4] Add K-2 to `crates/micold-core/tests/sandbox_argv.rs` — each supported limit produces exactly its flag with the expected unit conversion
- [X] T077 *(test)* [P] [US4] Add K-3 to `crates/micold-core/tests/sandbox_argv.rs` — an unsupported limit produces **no** flag and reconciliation reports it with a reason (C-2, R5 — checked as behaviour, not documented as a caveat)
- [X] T078 *(test)* [P] [US4] Add K-7 to `crates/micold-core/tests/sandbox_argv.rs` — `NoOutbound` emits the masquerade-disabled network **and** the published port, and asserts the measured failure mode (an `--internal` network making the port inert) is never generated (C-5, R4)
- [ ] T079 *(test)* [P] [US4] Write `crates/micold-core/tests/sandbox_capabilities.rs` — the probe is cached against the runtime version and re-runs when it changes, and `reconcile` is pure, total, and never mutates the profile (RC-1, RC-2, RC-3)
- [ ] T080 *(test)* [P] [US4] Add range cases to `crates/micold-core/tests/settings_roundtrip.rs` — a value below a documented workable minimum is refused on save with a message naming the accepted range (US4 scenario 5, FR-016)

### Implementation

- [X] T081 [US4] Implement `ResourceBudget`, `MilliCpus` and `Bytes` as newtypes with `Option` semantics distinguishing unset from maximum, in `crates/micold-core/src/sandbox/mod.rs` (RB-1, RB-2)
- [ ] T082 [US4] Implement `probe` and `RuntimeCapabilities` with per-limit `LimitSupport` carrying its reason, in `crates/micold-core/src/sandbox/runtime.rs` and `dialect/docker.rs` (R10, RC-1)
- [X] T083 [US4] Implement `reconcile(profile, caps) -> Vec<UnsatisfiableLimit>` in `crates/micold-core/src/sandbox/runtime.rs` — one fact consumed by both the view and the argv builder, so they cannot drift (RC-2)
- [ ] T084 [US4] Emit the budget flags in `crates/micold-core/src/sandbox/argv.rs`, omitting any limit reconciliation reports as unsupported (C-2)
- [X] T085 [US4] Implement `NetworkPosture` and the masquerade-disabled user-defined network in `crates/micold-core/src/sandbox/mod.rs` and `dialect/docker.rs` (R4, C-5)
- [ ] T086 [US4] Render limits in `crates/micold-client/src/ui/settings/daemon.rs` — supported ones editable, unsupported ones **disabled with the reason**, never hidden and never silently accepted (FR-015, SC-009)
- [ ] T087 [US4] Warn at the point of setting change that turning the network off stops the AI agent reaching its provider, in `crates/micold-client/src/ui/settings/daemon.rs` (US4 scenario 4)
- [ ] T088 [US4] Report which limit was reached and which setting governs it when a session is stopped by one, in `crates/micold-client/src/features/sandbox.rs` (US4 scenario 3 — not an anonymous failure)
- [ ] T089 [US4] Document the limits, their workable minimums, the storage-limit portability caveat and the DNS-still-resolves caveat in `docs/user-guide/sandboxed-daemon.md` (R4, R5)

**Checkpoint**: the second half of "sandbox" — containment as well as isolation.

---

## Phase 7: User Story 5 — Docker today, something else tomorrow (P3)

**Goal**: the seam is real, proven by a second implementation rather than asserted.

**Independent test**: run Stories 1, 2 and 4's acceptance scenarios against the shipped runtime, and
confirm the same scenario set is expressible against a second runtime with no change to session,
worktree or settings behaviour.

### Tests first

- [ ] T090 *(test)* [P] [US5] Parameterise `crates/micold-core/tests/sandbox_argv.rs` and `sandbox_runtime.rs` over both dialects so podman passes K-1 … K-12 (contracts/container-runtime.md §"Conformance suite")
- [ ] T091 *(test)* [P] [US5] Write `crates/micold-core/tests/sandbox_detect.rs` — a runtime that is not installed, not running, and not usable by this user each produce a **distinct** classified error (US5 scenario 2, C-6)
- [ ] T092 *(test)* [P] [US5] Assert in `crates/micold-core/tests/sandbox_detect.rs` that the unselected runtime is never invoked — the fake runtime's argv log for the other runtime stays empty (US5 scenario 3)

### Implementation

- [X] T093 [US5] Implement podman's dialect in `crates/micold-core/src/sandbox/dialect/podman.rs` — rootless defaults and `--userns=keep-id` (R3, C-4)
- [ ] T094 [US5] Implement `detect` for both dialects in `crates/micold-core/src/sandbox/runtime.rs`, distinguishing not-installed, not-running and not-permitted (US5 scenario 2)
- [ ] T095 [US5] Add runtime selection to `crates/micold-client/src/ui/settings/daemon.rs` using the existing `Select` component, defaulting to Docker (SP-2, FR-021)
- [ ] T096 [US5] Ensure a detect failure reports which of the three it is with a next step, and leaves the app with a working service path, in `crates/micold-client/src/features/sandbox.rs` (US5 scenario 2)
- [ ] T097 [P] [US5] Document the supported runtimes, podman's rootless differences, and the "adding a runtime" procedure in `docs/user-guide/sandboxed-daemon.md` (contracts/container-runtime.md §"Adding a runtime")
- [ ] T098 [US5] Run quickstart.md §B.2 and §B.4 against podman on Linux, recording the result in `specs/027-sandboxed-daemon-runtime/evidence/us5-podman.md` — the claim that the seam is real, not a shim around one runtime

**Checkpoint**: FR-020's abstraction is demonstrated rather than asserted, and SC-009 is measurable.

---

## Phase 8: User Story 6 — Nothing fails silently (P3)

**Goal**: every new failure class produces a distinct, actionable message and a defined recovery, and
no session ever runs unsandboxed without the user choosing it for that occasion.

**Independent test**: provoke runtime absent, image unavailable, project path unmountable, and
sandbox removed externally; confirm each gives a distinct message and recovery, and that in no case
does a session start unsandboxed without an explicit choice.

### Tests first

- [X] T099 *(test)* [P] [US6] Write `crates/micold-core/tests/sandbox_state.rs` — S-2 as a **graph property**: no edge leaves `Failed` for a working unsandboxed daemon without an explicit action (FR-035)
- [X] T100 *(test)* [P] [US6] Add S-4 to `crates/micold-core/tests/sandbox_state.rs` — every terminal failure carries a reason **and** a remedy drawn from the closed enumeration (FR-034)
- [ ] T101 *(test)* [P] [US6] Add M-4 to `crates/micold-core/tests/sandbox_state.rs` — registering a project marks the sandbox `Stale` and nothing restarts on its own (R9)
- [ ] T102 *(test)* [P] [US6] Extend `crates/micold-client/tests/banner_is_not_a_snackbar.rs` — the failed and unsandboxed states are persistently visible, not a toast that scrolls away (FR-035b, S-3)
- [ ] T103 *(test)* [P] [US6] Write `crates/micold-core/tests/sandbox_unmountable.rs` — a project on a path the runtime cannot share fails with a message naming the path and the reason, not a generic mount error (Edge Cases)

### Implementation

- [X] T104 [US6] Implement the `SandboxState` machine in `crates/micold-core/src/sandbox/mod.rs` per data-model.md §7 — pure, with the client holding only the current value
- [ ] T105 [US6] Implement the stale-on-project-change transition and the explicit restart action in `crates/micold-core/src/sandbox/mod.rs` and `crates/micold-client/src/features/sandbox.rs` (R9, M-4)
- [ ] T106 [US6] Detect a sandbox stopped or removed outside the app and recover to a defined state rather than hanging, in `crates/micold-client/src/shell/sandbox.rs` (US6 scenario 3)
- [X] T107 [US6] Implement the per-occurrence consented fallback in `crates/micold-client/src/features/sandbox.rs` — offered on failure, never taken automatically, and reset on next launch (FR-035a, US6 scenario 2)
- [ ] T108 [US6] Surface the failed and unsandboxed states through `ConnectionBanner` in `crates/micold-client/src/ui/`, persistently for as long as they last (FR-035b)
- [ ] T109 [US6] Expose the daemon's in-sandbox diagnostics through the app via `logs` in `crates/micold-client/src/ui/` (US6 scenario 6)
- [ ] T110 [US6] Implement explicit stop that leaves no orphaned container, and leave the sandbox running on app close by design, in `crates/micold-client/src/shell/sandbox.rs` (US6 scenario 4)
- [ ] T111 [P] [US6] Write the failure catalogue — cause, message, remedy — into `docs/user-guide/sandboxed-daemon.md` (FR-034)
- [ ] T112 [US6] Run quickstart.md §B.3 and §B.5's failure items, recording the result in `specs/027-sandboxed-daemon-runtime/evidence/us6-failures.md`

**Checkpoint**: the new failure surface is bounded, documented and recoverable.

---

## Phase 9: Polish & Cross-Cutting Concerns

- [ ] T113 Implement the daemon-backed `micold_core::git::Git` for Windows and wire it into `crates/micold-client/src/shell/capabilities.rs`, replacing path translation as R2's resolution and unblocking the remote placement
- [ ] T114 *(test)* [P] Add Windows path-mapping cases to `crates/micold-core/tests/sandbox_argv.rs` and a test that the daemon-backed `Git` and `GitCli` agree on worktree listings for the same repository
- [ ] T115 [P] Verify the full quickstart.md §A suite is green on Linux, macOS and Windows with **no** runtime installed, from the CI matrix in `.github/workflows/` (Principle VI, the fake runtime's whole purpose)
- [ ] T116 [P] Measure SC-003 — sandboxed session start no more than 2s slower than unsandboxed — recording the numbers in `specs/027-sandboxed-daemon-runtime/evidence/performance.md`
- [ ] T117 [P] Measure SC-004 — first-time enable under 5 minutes with continuous progress, from a cold image state — into `specs/027-sandboxed-daemon-runtime/evidence/performance.md`
- [ ] T118 Audit that no code path logs, prints, or includes the authentication token in argv or an error message (P-3), adding the grep-the-argv-and-log test to `crates/micold-core/tests/protocol_auth.rs`
- [ ] T119 [P] Update `README.md` and `docs/daemon.md` cross-references for the new placement model and the restructured settings docs
- [ ] T120 Run the complete quickstart.md §B pass end to end and record the evidence in `specs/027-sandboxed-daemon-runtime/evidence/`

---

## Dependencies

```text
Phase 1 Setup
   └─> Phase 2 Foundational  (BLOCKS everything below)
          ├─> Phase 3 US1 (P1)  ── MVP
          │      ├─> Phase 4 US2 (P1)      needs US1's container lifecycle
          │      ├─> Phase 6 US4 (P2)      needs US1's argv builder
          │      └─> Phase 7 US5 (P3)      needs US1's docker dialect to mirror
          ├─> Phase 5 US3 (P2)             independent of US1 — needs only Phase 2's settings v4
          └─> Phase 8 US6 (P3)             needs US1's lifecycle; US3 improves its surface
                 └─> Phase 9 Polish
```

**Story independence.** US3 is genuinely independent: a sectioned Settings view holding today's
settings is deliverable without any sandbox code, which is why the spec calls it independently
valuable. US1 is the only hard prerequisite for US2, US4, US5 and US6. T042 exists so US1 is
shippable before US3 lands, and T070 retires it.

**Cross-story ordering that is not a dependency.** US5's podman dialect is written against the same
conformance suite as Docker's, so it can begin as soon as T037 exists, not when Phase 3 ends.

## Parallel Opportunities

**Phase 1**: T002, T003, T005, T006 in parallel after T001.

**Phase 2**: the three groups — placement (T007–T011), protocol (T012–T018), settings (T019–T022) —
touch disjoint files and can run concurrently; the runtime seam (T023–T026) needs none of them.

**Phase 3**: T027–T032 are six independent test files, all parallel. Then T033–T035 are parallel
(different modules), T037 and T038 are parallel, and T044 runs alongside any of it.

**Phase 4**: T046–T049 parallel; T058 parallel with everything.

**Phase 5**: T061–T064 parallel; T067, T068, T069 are three disjoint section modules and are the
clearest parallel block in the feature; T074 parallel throughout.

**Phase 6**: T076–T080 parallel; T081 and T082 parallel.

**Phase 7**: T090–T092 parallel; T093 and T094 parallel; T097 alongside.

**Phase 8**: T099–T103 parallel; T111 alongside.

**Phase 9**: T114–T117 and T119 all parallel.

## Implementation Strategy

**MVP = Phase 1 + Phase 2 + Phase 3 (US1).** That is the feature's entire claim — a daemon that
cannot reach the host — and it is demonstrable by quickstart §B.2 without any of the configuration
surface existing. Ship or review there before continuing.

**Increment 2 = Phase 4 (US2).** Isolation is only adoptable if it costs nothing, so parity is the
next thing that matters, not configuration.

**Increment 3 = Phases 5 and 6 (US3, US4).** The view and the limits land together because the limits
need somewhere to live, and US3 is the surface US4 is configured from.

**Increment 4 = Phases 7 and 8 (US5, US6).** The second runtime proves the seam; the failure
catalogue bounds the support burden the feature introduces.

**Phase 9 is not optional.** T113 is the resolution of the one Constitution deviation this feature
carries, and T118 checks a security property that no other task covers.
