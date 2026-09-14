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
- [X] T041 ⚠️ Reopened [US1] Add the client-side sandbox lifecycle state in `crates/micold-client/src/features/sandbox.rs` and the off-thread runtime calls, progress and failure-to-`Message` glue in `crates/micold-client/src/shell/sandbox.rs`
      *(reopened — BUG-004)* The failure-to-`Message` glue landed; the **progress** glue did not.
      `boot()` passes `&mut |_| {}` and says so: *"threading a channel through boot for the sake of
      the first release's progress bar would buy less than the settings view (US3) will"*. So
      `Acquiring` and `Starting` are computed by the core and reach nothing, and the app sits on the
      `Probing` that `Sandbox::for_placement` set for the whole bring-up. Completed by T170.
      *(closed 2026-09-13 by T170)*
- [X] T042 [US1] Add the enable/disable control and the restart confirmation to the existing settings surface in `crates/micold-client/src/ui/settings_form.rs`, as a temporary home until US3 replaces it
- [X] T043 ⚠️ Reopened [US1] Show `StageProgress` during image acquisition in `crates/micold-client/src/ui/`, driven by the T039 callbacks (SC-004)
      *(reopened — BUG-004)* `ui/sandbox_status.rs` is complete and needs **no edit**: it renders
      `Probing`, `Acquiring(progress)` and `Starting`, and tests its own wording. It is driven by
      nothing, because T041 dropped the callbacks this task names. Reopened for its second clause
      only — closing it is T170 landing, then seeing this view move under §B.1 (T172).
      *(closed 2026-09-13 by T172)* Seen moving at the application: "Checking the container runtime" →
      "Getting the sandbox image" with bar and per-layer lines → "Starting the sandbox".
- [X] T044 [P] [US1] Write `docs/user-guide/sandboxed-daemon.md` — enabling, what the sandbox can and cannot see, the credential opt-ins and their default-off posture, and offline image import (Principle VII, FR-024a)
- [X] T045 [US1] Ran quickstart.md **§B.2** (plus §B.3 and §B.4) against Docker 29.5.1 and recorded it in `specs/027-sandboxed-daemon-runtime/evidence/us1-isolation.md` — the boundary, file ownership, network posture, limits and token non-leakage all hold. **§B.1 (first enable, cold, through the GUI) is still outstanding**: it needs the application running at a display, and it depends on T043's progress indicator to be meaningful *(the outstanding half is T172 — BUG-004; T043 is reopened again, so that dependency is live)*

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
      — ⚠️ **Scope note (BUG-003)**: this built the control that sets the placement; nothing was
      ever written that made the value act. FR-032 and FR-033 are absent from this task's
      requirement list because `data-model.md` §7 had claimed them for `SandboxState`, so the
      section saves the choice and the daemon never moves. **Not reopened** — the control is
      correct against FR-027, and the missing work is work that was never scheduled, not work that
      drifted. It is Phase 17.
- [X] T071 [US3] Grow `SettingsDraft` to hold per-section drafts with validation beside the type in `crates/micold-client/src/features/settings.rs` (US3 scenarios 2 and 3)
- [X] T072 [US3] Render each active credential opt-in individually while it is active, in `crates/micold-client/src/ui/settings/daemon.rs` (FR-004c, N-2)
- [X] T073 [US3] Route the view into the shell and remove `crates/micold-client/src/ui/settings_form.rs`'s modal, updating `crates/micold-client/src/ui/mod.rs`'s `view` signature
- [X] T074 [P] [US3] Restructure `docs/user-guide/settings.md` for the sectioned view
- [X] T075a *(test)* [US3] Extend `crates/micold-client/src/ui/material/field_focus.rs` — a `Button` and a `Select` join the focus traversal, answer their keys, and are drawn as holding it. Raised by the T075 pass: on the Appearance section eight Tab presses changed zero pixels, because `Button` and `Select` are not `Focusable` at all — so the Theme picker, every rail row, Cancel and Save are unreachable by keyboard (FR-030)
- [X] T075b [US3] Give them one. Generalise the checkbox's focus-holding wrapper into `crates/micold-client/src/ui/material/keyboard_focus.rs` and wrap `Button` with it, drawing `state::FOCUS_RING_WIDTH`'s indicator — the token has existed since feature 018 with no user, its own doc recording "buttons, rows, menu items and chips cannot hold focus in this rendering stack" as accepted gap #2. `Select` holds its own focus in `SelectState`, which is where its open flag already lives (FR-030, FR-022)
- [X] T075c [US3] Scroll whatever holds the keyboard into view, in `crates/micold-client/src/ui/focus.rs`, and chain it onto `Message::FocusMoved`. FR-030's second clause is "with the focused element visible", and iced's focus operations never look at a scrollable — so with T075b in place Tab walks into the Session service page's controls below the fold and the ring is painted where nobody can see it (FR-030)
- [X] T075 [US3] Run quickstart.md §B.6 with the repo's `visual-pass` skill — both themes, keyboard-only navigation, narrowest supported width — and record the result

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
- [X] T079 *(test)* [P] [US4] Write `crates/micold-core/tests/sandbox_capabilities.rs` — the probe is cached against the runtime version and re-runs when it changes, and `reconcile` is pure, total, and never mutates the profile (RC-1, RC-2, RC-3)
- [X] T080 *(test)* [P] [US4] Add range cases to `crates/micold-core/tests/settings_roundtrip.rs` — a value below a documented workable minimum is refused on save with a message naming the accepted range (US4 scenario 5, FR-016)

### Implementation

- [X] T081 [US4] Implement `ResourceBudget`, `MilliCpus` and `Bytes` as newtypes with `Option` semantics distinguishing unset from maximum, in `crates/micold-core/src/sandbox/mod.rs` (RB-1, RB-2)
- [X] T082 [US4] Implement `probe` and `RuntimeCapabilities` with per-limit `LimitSupport` carrying its reason, in `crates/micold-core/src/sandbox/runtime.rs` and `dialect/docker.rs` (R10, RC-1)
- [X] T083 [US4] Implement `reconcile(profile, caps) -> Vec<UnsatisfiableLimit>` in `crates/micold-core/src/sandbox/runtime.rs` — one fact consumed by both the view and the argv builder, so they cannot drift (RC-2)
- [X] T084 [US4] Emit the budget flags in `crates/micold-core/src/sandbox/argv.rs`, omitting any limit reconciliation reports as unsupported (C-2)
- [X] T085 [US4] Implement `NetworkPosture` and the masquerade-disabled user-defined network in `crates/micold-core/src/sandbox/mod.rs` and `dialect/docker.rs` (R4, C-5)
- [X] T086 [US4] Render limits in `crates/micold-client/src/ui/settings/daemon.rs` — supported ones editable, unsupported ones **disabled with the reason**, never hidden and never silently accepted (FR-015, SC-009)
- [X] T087 [US4] Warn at the point of setting change that turning the network off stops the AI agent reaching its provider, in `crates/micold-client/src/ui/settings/daemon.rs` (US4 scenario 4)
- [X] T088 [US4] Report which limit was reached and which setting governs it when a session is stopped by one, in `crates/micold-client/src/features/sandbox.rs` (US4 scenario 3 — not an anonymous failure)
- [X] T089 [US4] Document the limits, their workable minimums, the storage-limit portability caveat and the DNS-still-resolves caveat in `docs/user-guide/sandboxed-daemon.md` (R4, R5)

**Checkpoint**: the second half of "sandbox" — containment as well as isolation.

---

## Phase 7: User Story 5 — Docker today, something else tomorrow (P3)

**Goal**: the seam is real, proven by a second implementation rather than asserted.

**Independent test**: run Stories 1, 2 and 4's acceptance scenarios against the shipped runtime, and
confirm the same scenario set is expressible against a second runtime with no change to session,
worktree or settings behaviour.

### Tests first

- [X] T090 *(test)* [P] [US5] Parameterised the conformance suite over both dialects so podman passes K-1 … K-12 (contracts/container-runtime.md §"Conformance suite"). **`crates/micold-core/tests/sandbox_argv.rs` does not exist**: K-1 … K-7 and K-11 have always lived inline in `crates/micold-core/src/sandbox/argv.rs`'s own test module, and that is where they were parameterised; K-8 … K-10 and K-12 in `tests/sandbox_runtime.rs`. Six `podman_err_*.txt` fixtures were added and their README says plainly that they were **transcribed from podman's message strings rather than captured** — podman is not installed on this machine — with T098 named as the task that confirms or corrects them
- [X] T091 *(test)* [P] [US5] Write `crates/micold-core/tests/sandbox_detect.rs` — a runtime that is not installed, not running, and not usable by this user each produce a **distinct** classified error (US5 scenario 2, C-6)
- [X] T092 *(test)* [P] [US5] Assert in `crates/micold-core/tests/sandbox_detect.rs` that the unselected runtime is never invoked — the fake runtime's argv log for the other runtime stays empty (US5 scenario 3)

### Implementation

- [X] T093 [US5] Implement podman's dialect in `crates/micold-core/src/sandbox/dialect/podman.rs` — rootless defaults and `--userns=keep-id` (R3, C-4)
- [X] T094 [US5] Implement `detect` for both dialects in `crates/micold-core/src/sandbox/runtime.rs`, distinguishing not-installed, not-running and not-permitted (US5 scenario 2)
- [X] T095 [US5] Add runtime selection to `crates/micold-client/src/ui/settings/daemon.rs` using the existing `Select` component, defaulting to Docker (SP-2, FR-021)
- [X] T096 [US5] Ensure a detect failure reports which of the three it is with a next step, and leaves the app with a working service path, in `crates/micold-client/src/features/sandbox.rs` (US5 scenario 2). Added `Sandbox::fallback_offer`, which is where "leaves a working service path" becomes a property something can assert rather than a hope. Also renamed four remedies from "Settings → Daemon" to "Settings → Session service": the section has never been called Daemon in the UI, so the next step named a place the user could not find
- [X] T097 [P] [US5] Document the supported runtimes, podman's rootless differences, and the "adding a runtime" procedure in `docs/user-guide/sandboxed-daemon.md` (contracts/container-runtime.md §"Adding a runtime")
- [X] T098 [US5] Run quickstart.md §B.2 and §B.4 against podman on Linux, recording the result in `specs/027-sandboxed-daemon-runtime/evidence/us5-podman.md` — the claim that the seam is real, not a shim around one runtime. **Green on podman 5.8.4** (rootless, `crun`, cgroup v2, systemd cgroup manager), reached one level in — this host cannot run podman rootless at all (`newuidmap`/`newgidmap` absent, unprivileged user namespaces restricted), so it ran in a privileged `quay.io/podman/stable` container booting systemd with linger enabled. The harness stopped spelling `docker` first: `MICOLD_TEST_RUNTIME` selects the runtime and `Dialect::for_kind` supplies the program name, the identity flags and the control-socket path, so the pass exercises the seam rather than a second hard-coded runtime. It found four defects nothing else could: the §B.4 memory probe could not fail (a `\$x` escape inside single quotes is a fatal perl compile error, masked under Docker because perl constant-folds the 512 MiB allocation before reporting it); `podman rm -f` returns before the name is released, so `purge` now waits; podman 5.8.4 reports a too-small subuid range in words the classifier could not read, landing it in `Unknown`; and the port in "address already in use" was read as `0` for **both** runtimes. All six transcribed `podman_err_*.txt` fixtures are now captured, a seventh added, and the fixtures README records where each transcription was wrong

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
- [X] T101 *(test)* [P] [US6] Add M-4 to `crates/micold-core/tests/sandbox_state.rs` — registering a project marks the sandbox `Stale` and nothing restarts on its own (R9)
- [X] T102 *(test)* [P] [US6] Extend `crates/micold-client/tests/banner_is_not_a_snackbar.rs` — the failed and unsandboxed states are persistently visible, not a toast that scrolls away (FR-035b, S-3)
- [X] T103 *(test)* [P] [US6] Write `crates/micold-core/tests/sandbox_unmountable.rs` — a project on a path the runtime cannot share fails with a message naming the path and the reason, not a generic mount error (Edge Cases)
      *Found by the test, as intended:* the mount-refusal phrases were a single hard-coded list written from Docker's wording, and podman does not use it — it names the syscall (`statfs <path>: no such file or directory`) rather than the mount configuration, so a refused bind on podman landed in `Unknown`. The phrases moved into `Dialect::mount_rejected_phrases` beside the two lists that were already there, and the dialect's own test now requires every runtime to declare them.

### Implementation

- [X] T104 [US6] Implement the `SandboxState` machine in `crates/micold-core/src/sandbox/mod.rs` per data-model.md §7 — pure, with the client holding only the current value
- [X] T105 [US6] Implement the stale-on-project-change transition and the explicit restart action in `crates/micold-core/src/sandbox/lifecycle.rs` and `crates/micold-client/src/features/sandbox.rs` (R9, M-4)
      *Deviation:* the core half lives in `sandbox/lifecycle.rs`, not `sandbox/mod.rs` — that is where the rest of the state machine already was. Reaching the restart from a button also needed the bring-up recipe to outlive boot, so `shell::sandbox::BootPlan` was added and `App` carries one; the M-4 transition is driven from the daemon's catalog in `shell/daemon_sync.rs`, which is the only place that learns a project was registered. Fixed while here: an accepted fallback survived a *second* failure, so the banner kept reporting the first reason.
- [X] T106 [US6] Detect a sandbox stopped or removed outside the app and recover to a defined state rather than hanging, in `crates/micold-client/src/shell/sandbox.rs` (US6 scenario 3)
      *Deviation:* needed a twelfth `RuntimeError` — `SandboxStopped { name }` — since `Failure` is the only carrier the application has for "the sandbox is unusable and this is why", and none of the eleven existing variants means "it went away". Contract C-6 updated to list it. The check is asked once on a dropped connection rather than polled: the answer only changes when the connection does.
- [X] T107 [US6] Implement the per-occurrence consented fallback in `crates/micold-client/src/features/sandbox.rs` — offered on failure, never taken automatically, and reset on next launch (FR-035a, US6 scenario 2)
- [X] T108 [US6] Surface the failed and unsandboxed states through `ConnectionBanner` in `crates/micold-client/src/ui/`, persistently for as long as they last (FR-035b)
- [X] T109 [US6] Expose the daemon's in-sandbox diagnostics through the app via `logs` in `crates/micold-client/src/shell/` (US6 scenario 6)
      *Deviation:* not in `ui/` — the existing "show diagnostics" action already had a route, and what was missing was the answer when there is no connection to ask. `shell/daemon_sync.rs`'s `on_diagnostics_requested` now falls back to the runtime's `logs` instead of reporting that there is nothing to show, which is the case the user is almost always in when they ask.
- [X] T110 [US6] Implement explicit stop that leaves no orphaned container, and leave the sandbox running on app close by design, in `crates/micold-client/src/shell/sandbox.rs` (US6 scenario 4)
      *Note:* the stop is `stop` **then** `remove`, both idempotent per C-7, and it is routed from the existing "restart service" action — which previously stopped the *process* over its endpoint and would have left the container up with nothing in it. Leaving the sandbox running on app close needed no code: nothing on the close path touches it, which is the design.
- [X] T111 [P] [US6] Write the failure catalogue — cause, message, remedy — into `docs/user-guide/sandboxed-daemon.md` (FR-034)
- [x] T112 [US6] Run quickstart.md §B.3 and §B.5's failure items, recording the result in `specs/027-sandboxed-daemon-runtime/evidence/us6-failures.md`

      Run as `crates/micold-core/tests/sandbox_real_lifecycle.rs` behind the `sandbox-real-runtime`
      feature rather than as a hand-typed `docker` transcript: the checks drive `CliRuntime` against
      a real Docker daemon, so what they exercise is the argv and the state machine the application
      actually uses, and the same tests are what CI's Linux `sandbox-runtime` job runs. All seven
      pass against Docker 29.5.1, under CI's own command. *Found by writing it this way:* the CI
      step filters on `sandbox_real_`, which cargo matches against **test names**, not file names —
      so these seven, first written with plain descriptive names, would have been silently skipped
      there while passing locally. They now carry the prefix, and the contract is recorded beside
      the filter in `.github/workflows/ci.yml`. Four §B.5 items are **not** ticked and say why in
      the evidence: the stale-on-registration item is pure state-machine behaviour already covered by
      `sandbox_state.rs`, and accepting the fallback, surviving a client restart, and surviving a
      reboot need a GUI or a reboot of this machine.

**Checkpoint**: the new failure surface is bounded, documented and recoverable.

---

## Phase 9: Polish & Cross-Cutting Concerns

- [x] T113 Implement the daemon-backed `micold_core::git::Git` for Windows and wire it into `crates/micold-client/src/shell/capabilities.rs`, replacing path translation as R2's resolution and unblocking the remote placement
      — **not as a second `Git` implementation.** R2's wording asks for one, and it cannot be
      built: `Git` is a synchronous 13-method trait, while the daemon connection is asynchronous
      and correlated. An impl satisfying that signature would have to block on a round trip into a
      container from inside iced's `update`, trading a wrong worktree list for a frozen window.
      So the seam moved one level out. `Capabilities::git` is now `Option<Arc<dyn Git>>`, and
      `boot()` narrows it away with `without_local_git()` when
      `Placement::git_routing() == GitRouting::ViaDaemon` — the capability is *absent*, not
      substituted, and the type system then forces both call sites to say what they do without it.
      The client uses only two of the thirteen methods, and each gets the answer its nature allows:
      the open-project gate (`is_repo_root`) becomes protocol v7's `ClientMsg::RepoRootQuery` /
      `OperationResult::RepoRoot`, answered by the side that will run git; the worktree *seed*
      (`worktree_list_porcelain`) becomes empty, because a seed built from host paths while the
      daemon reports container paths is not a faster truth, just a different list shown briefly.
      The routing predicate reads `pathmap::is_identity()` — the same source of truth the mount set
      is built from, with a test asserting the two cannot drift. The gate's answer carries the
      folder back, so a client that has moved on discards it rather than opening what the user
      cancelled.
- [x] T114 *(test)* [P] Add Windows path-mapping cases to `crates/micold-core/tests/sandbox_argv.rs` and a test that the daemon-backed `Git` and `GitCli` agree on worktree listings for the same repository
      — the mapping had unit tests; everything *downstream* of it did not. `ProjectMount::project`
      and `MountSet::build` called `pathmap::map`, which is `cfg!(windows)`-gated, so the assembly
      that produces a Windows mount set — and the `-v` flags `argv` renders from it — was compiled
      by no CI runner this project has. Both now take the platform as a value (`project_for`,
      `build_for`), and the new `sandbox_argv.rs` drives them **both ways** on whatever platform
      runs: every Windows container path is a Linux absolute path under `/mnt/host`, the host half
      keeps its backslashes and drive letter, rule M-1 holds under both mappings, and `argv` and
      `git_routing_for` are asserted to agree about whether the two halves differ — the drift check
      between the mount set and T113's routing decision. The volume parser splits from the *right*
      because a Windows host path contains a colon; splitting left yields `C` as the host, which is
      the exact bug the file exists to catch. The state and token mounts are excluded from the
      identity claim: their container paths are fixed by the image, so they are never identity
      mounts, not even on Linux.
      The second half could not be written as stated — T113 removed the daemon-backed `Git` rather
      than adding one — so the surviving claim is the one that matters:
      `mutation_semantics.rs::the_streamed_worktree_list_matches_local_git_discovery` asserts the
      daemon's streamed worktree list equals `worktree::discover(&GitCli::new(), …)` for the same
      repository. Where the client has no local git that stream is not a faster copy of something
      it could compute; it is the only list it will ever have.
- [X] T115 [P] Verify the full quickstart.md §A suite is green on Linux, macOS and Windows with **no** runtime installed, from the CI matrix in `.github/workflows/` (Principle VI, the fake runtime's whole purpose)
      — **Two of §A's own gates were not running on macOS or Windows.** The matrix covers `micold-core`
      wholesale (`cargo test -p micold-core --all-targets`), so every core row is included by
      construction, and none of them is platform-gated — no `#[cfg(unix)]`, `#[cfg(windows)]` or
      `target_os` in any of the seven targets or in `argv.rs`'s unit tests — so what runs on Linux is
      what runs everywhere. The `micold-client` rows are different: that suite needs the iced system
      dependencies and runs in full on Linux only, so the render-free exceptions are named one
      `--test` flag at a time. `features_settings` and `anatomy_call_sites` were in §A's table and
      not in the flags. They ran, they passed, nothing failed — and the *three-platform* claim, which
      is the only claim the table is making, was false.
      Both are render-free (a reducer and a source-text scanner; `anatomy_call_sites` already
      normalises `\` to `/` in its display keys, the Windows hazard the step's own comment names) and
      both pass standalone the way CI invokes them — 9 and 10 tests. Added to the cross-platform step.
      `crates/micold-core/tests/quickstart_a_runs_everywhere.rs` now holds `ci.yml` to §A's table, so
      an enumerated list cannot drift out of the claim in silence again; reverting the two flags fails
      it by name. That gate reads the quickstart, so the file is `-micold-docs` in `.gitattributes`
      for the same reason `CHANGELOG.md` is — otherwise editing the table would skip the pipeline
      that checks the table.
      **The run**: green on all three, run 33003036028, recorded in
      `evidence/t115-three-platform-matrix.md`. It took five attempts and four of them were red, none
      a flake. The Windows leg found that `pathmap::map_for` built the container path with
      `PathBuf::push`, which writes `\` on a Windows host — so `docker -v` was handed
      `/mnt/host\c\Users/u/p` and **sandboxed mode was broken on Windows outright**. T114's
      `windows_host: bool` parameter carries the mapping's logic to a Linux runner but not its
      `PathBuf`, and the mapping's own tests compare `PathBuf`s, which Windows considers equal
      either way; only `sandbox_argv.rs`, which asserts on rendered argv strings, could see it. Two
      more were the same class in the suite itself — the scans in `quickstart_a_runs_everywhere.rs`
      and `anatomy_call_sites.rs` mis-parse under CRLF and blame the documents, now settled by
      `* text=auto eol=lf` in `.gitattributes` rather than in thirty readers. The fourth was the
      `sandbox-runtime` job never building `micold-daemon:dev`.
- [x] T116 [P] Measure SC-003 — sandboxed session start no more than 2s slower than unsandboxed — recording the numbers in `specs/027-sandboxed-daemon-runtime/evidence/performance.md`
      — **0ms** against a 2000ms budget: both placements 2ms median over 7 timed rounds, both showing a real `$` prompt.
      Three earlier revisions of the measurement passed while measuring nothing (an unmounted catalogue, a snapshot
      mistaken for a prompt, and two different shells); each is written up in the evidence, because a green comparative
      benchmark is exactly the kind that hides its own vacuity.
- [X] T117 ⚠️ Reopened [P] Measure SC-004 — first-time enable under 5 minutes with continuous progress, from a cold image state — into `specs/027-sandboxed-daemon-runtime/evidence/performance.md`
      — **851ms** against a 300,000ms budget (acquire 419ms, create 258ms, start 123ms, daemon answering 50ms), plus
      **9s** for SC-004b's source-change loop. The *duration* half is measured; the **continuity** half is not, and the
      evidence says so: the only acquisition route runnable here is the file import, which finished too fast to have
      any silence in it, and the route that would (a registry pull) has nothing published to pull.
      *(reopened — BUG-004)* And it is measured in the wrong **place**, which is the more useful
      finding: the clock and the progress counter both sit in `micold-core`, around `acquire_image`.
      The evidence file says the clock covers *"the application's whole enable sequence"*; it covers
      the core's. That is below the level the defect lives at, so the measurement passed over a
      client that discards every report it is given. SC-004c now says where to measure. Re-measure
      through the app once T170 lands — the registry route it could not take then is publishable
      now (Phase 15 shipped it), so the long case is finally runnable.
- [x] T118 Audit that no code path logs, prints, or includes the authentication token in argv or an error message (P-3), adding the grep-the-argv-and-log test to `crates/micold-core/tests/protocol_auth.rs`
      — the audit found a live vector, not a clean bill. `auth::Token` redacts its own `Debug`, but
      the token stops being a `Token` the moment it goes on the wire: `ClientMsg::Hello`,
      `handshake::Introduction` and `connect::Credentials` each held it as a bare `String` inside a
      `derive(Debug)` type, so any `{:?}` of a handshake frame — in a crate with 57 log sites —
      would have printed the secret in full. Fixed by introducing `PresentedToken`, a
      `#[serde(transparent)]` newtype with a hand-written redacting `Debug`, and using it at all
      three sites; `transparent` keeps the encoding byte-identical, so `SCHEMA_HASH` and the wire
      format are unchanged. It is declared *in* `protocol/messages.rs` because the hash is generated
      over that file's text alone.
      Four tests in `tests/protocol_auth.rs` now hold the property: the three Debug renderings on
      the handshake path, the generated `docker create`/`network create` argv (a real spec with a
      written token file — `docker inspect` shows argv to anyone), and the refusal a wrong token
      earns, checked in both renderings that reach a person (the client's
      `format!("daemon refused the connection: {reason:?}")` and the serialised `DaemonMsg::Refused`).
      Each carries a counterweight assertion, so a redaction that also broke authentication or
      dropped the token mount would fail rather than pass.
- [x] T119 [P] Update `README.md` and `docs/daemon.md` cross-references for the new placement model and the restructured settings docs
      — `docs/daemon.md`'s placement section had **no outbound links at all**: it described the model
      and then left the reader with no route to the page that tells them how to switch it on. It now
      points at `user-guide/sandboxed-daemon.md` and at Settings → Session service, and two claims in
      it were stale — "the wire protocol moved to version 6" (it is 7) and nothing at all about who
      answers "is this a git repository?" once the service cannot see the folder the way the app
      does. Both corrected in place.
      `docs/README.md` never listed `sandboxed-daemon.md`, so the whole page was reachable only from
      inside `settings.md`; added, and the settings and daemon entries rewritten to name what they
      actually cover now. `README.md` gained the session service and the container placement in its
      feature list, and its **Build & run** block was wrong — `cargo run --features gui` names a
      feature this workspace does not have, from before the core/client/daemon split. Replaced with
      the `mise` tasks CLAUDE.md declares canonical. The same stale wording survives in several early
      features' spec artifacts; the constitution's 1.4.1 report explicitly leaves those to their own
      passes, so they were not touched here.
- [x] T120 Run the complete quickstart.md §B pass end to end and record the evidence in `specs/027-sandboxed-daemon-runtime/evidence/`
      Run 2026-08-26 against Docker 29.5.1 (cgroup v2, `systemd` driver, `overlayfs`) on a freshly
      built `micold-daemon:dev`, in release, in one sitting: 22 real-runtime tests across
      `micold-core` and `micold-daemon`, all green. `evidence/quickstart-b-closeout.md`.
      Two things the pass produced rather than confirmed. **The survival box could be closed after
      all** (§B.5, FR-014a/b/c): the opt-in's restart policy now has one home,
      `argv::restart_policy`, called by both `argv::create` and the harness — which had hardcoded
      `"no"` and so could never have caught it going wrong — and `sandbox_real_staleness.rs` gained
      a probe that kills the container's own process on the host, watches the runtime bring it back
      unasked, and finds the session still in the catalogue and still answering, plus a `--restart
      no` control that stays exited. No reboot was performed; the evidence says so where the tick
      is. The first spelling of that probe used `docker kill` and *always* failed: an API-issued
      kill is recorded as a manual stop, which `unless-stopped` is defined to respect, so the
      runtime obeying the policy and ignoring it look identical.
      **`--no-fail-fast` matters here.** Outside release, `sandbox_real_session_start` refuses to
      measure — correctly — and cargo then stops at that target, silently skipping every test
      after it, including the whole of `sandbox_real_staleness`.
      The idle-repaint box in §B.6 stays open: lavapipe cannot settle it, and this pass did not
      change that.

---

## Phase 10: Bugfix BUG-001 — Save reverts a theme chosen from the app bar

**Goal**: Stop the Settings view undoing a choice the user made outside it and can see applied. The
T075 visual pass observed this twice and recorded it as out of scope; it is not — the two writers on
the theme were harmless while Settings was a modal covering the app bar, and FR-026 is what put them
both on screen at once.

### Tests for BUG-001 (MANDATORY — Constitution Principle I) ⚠️

- [x] T121 [BUG-001] `crates/micold-client/tests/settings_draft_tracks_the_live_theme.rs`: four
      rules over the reducer — the cycle, the outright pick, that an Appearance edit is *still* only
      a draft, and that the menu works with no form open. The middle two are the ones that make this
      a gate rather than a patch: a fix applied to `ThemeModeCycled` alone leaves
      `ThemePreferenceChanged` reachable, and "fixing" it by applying the theme live would leave
      Cancel nothing to discard. The first two were confirmed red against the unfixed reducer.

### Implementation for BUG-001

- [x] T122 [BUG-001] `features/settings.rs::apply_theme`, called by both app-bar entry points: set
      `theme_pref`, and carry it into `settings_draft.appearance.theme` when a form is open. A
      change made outside the form is newer than the draft, so the draft takes it. Deliberately not
      via `edit()` — that clears `draft.error`, and a theme picked from the app bar is not the user
      acting on the form, so clearing it would empty the message FR-029 sends them to read.

**Bugfix**: 2026-08-27 — BUG-001. **No requirement added**: FR-020's save semantics and FR-026's
full-surface view are each correct; the defect is in their interaction, which neither had to state.
**No task reopened**: T065 renders the Appearance section as specified, and the staleness is on the
seeding side. See `bugs/BUG-001.md`.

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

## Phase 11: FR-023a and FR-004d — the image ships the AI CLIs, and `~` is writable

**Goal**: Make "a sandboxed session can run the AI CLI the user picked" true rather than assumed.
FR-023 has always said the image carries "the tooling a session needs"; the published image carried
none of the AI CLIs, and the 22 green real-runtime probes could not see it because every one of them
drives a shell. Shipping them then exposed a second defect underneath: `HOME` pointed at a path that
does not exist inside the container, so the first thing an AI CLI does — write to `~` — failed.

**Why the two are one phase.** Neither is shippable alone. The image without the home fix ships a
`copilot` that dies on `EACCES: mkdir '/home/<user>'` at startup; the home fix without the image
fixes a home nothing was using.

### Tests (MANDATORY — Constitution Principle I) ⚠️

- [x] T123 [US1] `crates/micold-daemon/tests/sandbox_real_ai_cli.rs`: for every `AiCli::ALL`
      variant, its command is on `PATH` inside a real sandbox **and runs** (`--version` exits 0).
      Driven from `AiCli::ALL` rather than a written-out list, so a third provider added to the
      application fails this until the image ships it. Confirmed red twice: first with both CLIs
      missing, then — after the image shipped them — with `[("copilot", "rc=1")]`, which is the
      defect T125 fixes. Locating a binary is not enough here: `claude` tolerated the broken home
      and `copilot` did not, so a `command -v` check would have passed while a session failed.
- [x] T124 [US1] `crates/micold-daemon/tests/sandbox_real_boundary.rs`: `~` is writable, what lands
      there appears in the application's own directory, and it does **not** appear in the user's
      home. The file's existing probes all pass with *no* home at all — a `HOME` pointing nowhere
      lists nothing either — so the write is what distinguishes a shadowed home from a missing one.

### Implementation

- [x] T125 [US1] `sandbox/mod.rs`: `HomeMount`, a fifth member of `MountSet`, mounting
      `<state>/sandbox-home` at the host home path; `argv::mount_args` emits it first, since a
      project under the user's home has to land on top of it. Declared in the set rather than
      emitted by `argv` because obligation C-3 is that `argv` invents no mount — an implicit home
      mount is exactly the convenience C-3 forbids, and an explicit one keeps the rule literally
      true. `shell/sandbox.rs` creates the directory before `create`: a bind source the runtime has
      to create, it creates as root.
- [x] T126 [US1] `packaging/sandbox/Containerfile`: base `node:22-trixie-slim` and pinned
      `@anthropic-ai/claude-code` and `@github/copilot`. Both constraints on that tag are load
      bearing and recorded in the file — trixie for glibc, 22 because claude-code declares
      `engines: node >=22.0.0` and trixie ships Node 20.
- [x] T127 [US1] `sandbox_argv.rs`, `sandbox_credentials.rs`, `argv.rs`'s K-4 check: the mount-count
      assertions move by one, and the home is excluded from `mapped_volumes` — it is a
      *substitution*, not a mapping, so its two halves are supposed to differ on every platform.
      The credentials test gains the assertion that keeps the two apart: the container half is the
      user's home path and the host half is never it.

**Not in this phase**: FR-023b and FR-023c. Reporting a missing CLI where the image is chosen, and
answering availability from inside the sandbox instead of from the client's own `PATH`
(`provider.rs::resolves_on_path`), are a settings-surface change with their own tests.

**Requirement added**: FR-004d, and FR-023 split into FR-023a/b/c with SC-012. FR-023 said the image
carries what a session needs without saying who owns that when the user substitutes an image, and
nothing said the sandbox has a home of its own — which is why an image that satisfied FR-023 on
paper still could not run the CLI it shipped.

## Phase 12: FR-026b–e and FR-014d — the rail becomes a navigation rail, and the menu stops duplicating it

**Goal**: every section identified by an icon as well as a name; the rail collapsible to those icons
and still fully navigable; and no setting offered by two controls, with nothing lost when the
duplicates go.

**Why now**: FR-026 made Settings a full surface that no longer covers the app bar, which turned two
long-harmless duplicates into two live writers of one value — that is BUG-001's whole mechanism. The
rail's fixed width is the other half of the same surface: it was introduced to make Settings roomy
and is the one part of it that cannot be given back.

### Tests first

- [x] T128 [US3] *(test)* every `SettingsSection` carries an icon, and no two share one (FR-026b).
      Written against `SettingsSection::icon` rather than against the rendered rail, because the rail
      is one presentation of the section and a second one must not be able to invent its own icons.
      **Landed in `tests/settings_rail.rs`, not `settings_sections.rs`**: the icons and the collapse
      are one claim — icons exist *so that* collapsing keeps the sections distinguishable — and
      `settings_sections.rs` is a source-scanning gate about which section owns which setting.
- [x] T129 [US3] *(test)* `ui/material/section_list.rs` unit tests: the collapsed rail's width is
      the Material navigation-rail width and is stable across which section is current and whether a
      badge is shown — the same claims the expanded gates already make, now made per state (FR-026c).
      "One pressable node per section and no label text" is asserted through `row_parts`, a pure
      description of what a row is built from, rather than by walking the widget tree: the tree can
      only be asked where things are, and what had to be shown is that collapsing costs no
      *information* — the glyph stays, the badge stays in the one form there is room for, and only
      the name goes.
- [x] T130 [US3] *(test)* `crates/micold-client/tests/settings_rail.rs`: reaching every section
      while collapsed; the toggle flips the flag; the flag survives closing and reopening Settings
      within the session; Cancel does not revert it, Save does not write it, and closing the rail is
      not an edit to the form (FR-026c, FR-026d). One file with T128 — see there.
- [x] T131 [US3] *(test)* `crates/micold-client/tests/settings_sections.rs`: no message the app
      bar's overflow menu emits is one a settings section owns (FR-026e, SC-014), plus a gate that
      `logout_survival::enable_for` still has a caller, so removing the menu item cannot quietly
      remove the host-process capability. A **source scan** of `overflow_items` rather than a call
      to it: `ui/mod.rs` declares `mod toolbar;` privately, and widening the crate's public API to
      let a test look at a menu would be a worse trade than reading the file the gate is about — the
      same trade `settings_sections.rs` already makes for the sections themselves.
- [x] T132 [US3] *(test)* saving resolves the survival opt-in through the *configured* placement —
      the service-manager flow under host-process, the restart policy under sandboxed — and says so
      where the placement cannot offer it (FR-014d, SC-014). **Not in `features_settings.rs`**: the
      save path is `shell/persist.rs`, which lives in the GUI binary and no integration test can
      reach, so the gates are inline `#[cfg(test)]` modules beside the code they are about —
      `survival_step` in `persist.rs` (act on a change, and only on a change, in both directions),
      `survival_support` in `ui/settings/daemon.rs` (every placement says what it will do), and
      `enable_for`/`disable_for` in `micold-core` (the dispatch itself).

### Implementation

- [x] T133 [US3] `features/settings.rs`: `SettingsSection::icon`, beside `label` and `index`;
      `ui/material/section_list.rs`'s `Section` gains an icon and renders it in the leading slot
      `Button` already has. Icons from the existing `Icon` set — Principle VIII, and FR-026b says so
      explicitly because a settings-only icon set is the obvious shortcut here.
- [x] T134 [US3] `ui/material/section_list.rs`: the collapsed rendering, plus the affordance that
      toggles it. In the shared component rather than in the view, because FR-026a forbids a private
      rail and a collapsed rail built in `settings_view.rs` would be exactly that. The control is
      drawn *below* the destinations: it is not one of the places the user navigates to, and putting
      it first would make the top-left glyph — where the eye starts — the one that goes nowhere.
      Landed with the showcase entry made live in both widths (`showcase/sections/surfaces.rs`,
      `showcase/state.rs`): Principle VIII wants the state on the page, and a second *posed* rail
      would show a picture of a collapsed rail without showing the one claim it makes — that every
      destination is still pressable once the labels are gone.
- [x] T135 [US3] `app.rs`/`ui/settings_view.rs`: `settings_rail_collapsed` on `State` beside
      `settings_section`, a message that toggles it, and the rail rendered in the state it names.
      Not in `SettingsDraft` and not in `settings.json`: FR-026d makes it view state, which is what
      keeps it out of the save-together rule and off the schema.
- [x] T136 [US3] `ui/toolbar.rs`: drop the theme cycle and "Keep sessions after logout"; keep open
      Settings, diagnostics and About. `Message::ThemeModeCycled` and `mode_icon` went with the
      first, `Message::LogoutSurvivalRequested` with the second, and `ui/settings/appearance.rs`'s
      note and module doc — which argued for keeping the toolbar shortcut — are rewritten to say why
      it went. The BUG-001 fix stays. `Message::ThemePreferenceChanged` is **kept without a
      producer**, deliberately: it is the reducer's contract for a live theme change, and it is that
      rule — apply it, and carry it into an open draft — that a second writer would have to obey the
      day one is added again. Deleting it would delete the rule with it. The BUG-001 gate now drives
      that message.
- [x] T137 [US3] `shell/persist.rs`/`shell/service_control.rs`: saving the form applies the
      survival opt-in through `logout_survival::enable_for` with the resolved placement — the caller
      that function was written for and had never had. This is what makes removing the menu item
      safe: without it, dropping the item drops the host-process capability with it.

      Two things the plan did not name, both required by FR-014d's "rather than being absent or
      silently ineffective". **`disable_for`**, new in `micold-core`: the menu command could only
      ever enable, so until now unticking the box wrote `false` to a file and left the socket unit
      enabled — the sessions went on surviving. It disables and stops the unit, and deliberately
      does **not** run `loginctl disable-linger`, which is a per-user switch other services may rely
      on and which this application did not create exclusively. And **`survival_support`** in the
      section: the checkbox now says what the configured placement will actually do with it, and
      warns where the host-process mechanism is unavailable instead of sitting there inert.

**Not in this phase**: FR-023b and FR-023c, still. They are a settings-surface change and this phase
is a settings-surface phase, but they are about *the image* and are gated on asking the container
what it has rather than asking the client's `PATH` — a protocol question, not a layout one. Phase 13
is that question.

**Requirements added**: FR-026b, FR-026c, FR-026d, FR-026e, FR-014d, SC-013, SC-014.

## Phase 13: FR-023b and FR-023c — the answer comes from where sessions run, and the missing one is named

**Goal**: stop the client answering "which AI CLIs exist" from its own `PATH`, and say — at the two
points of choice, and only there — which CLI the running sandbox does not provide.

**Why now**: Phase 11 made the published image ship every AI CLI (FR-023a). That is the whole of the
guarantee for a user on the default image, and it is worth nothing to a user who substituted one —
FR-025 says they may, FR-023b says the obligation goes with the image, and nothing in this
application can make a stranger's image keep it. What is left to do is the only honest thing: find
out, and say so.

**The defect underneath, which is not a UI defect.** `Capabilities::available_providers()` walked
*this process's* `PATH`. That was correct while the session service was always a child of this
process, and FR-021 ended that: the client is on the host, the sessions are in a container, and the
same four lines went on answering confidently about the wrong machine. It does not crash, and it
does not look wrong on any machine a developer would test it on — a workstation has both CLIs
installed, so the host's answer and the container's agree everywhere except on the user's machine.
That is why the fix is a protocol pair and a gate, not a better probe.

### Tests first

- [x] T138 [US3] *(test)* `crates/micold-core/tests/available_here.rs`: the probe over a scratch
      `PATH` — nothing installed offers nothing, one installed offers exactly that one, uninstalling
      shrinks the offer, and the order is `AiCli::ALL`'s rather than `PATH`'s. **Moved, not
      written**: this suite was `shell/capabilities.rs`'s inline module and its assertions are
      unchanged, because what they assert never depended on who was asking. FR-023c moved the
      question into `micold-core`, so its test came with it.
- [x] T139 [US3] *(test)* `crates/micold-daemon/tests/ai_cli_availability.rs`: over a real duplex
      connection, `AiCliAvailabilityRequest` is answered from the **service's own** environment —
      a stubbed `claude` on a scratch `PATH` comes back as exactly `[ClaudeCode]`, and an
      environment with no CLI at all comes back as an empty set rather than as a failure. The two
      together are what pin the answer to the process that ran it: either alone is satisfied by a
      constant.
- [x] T140 [US3] *(test)* `crates/micold-client/tests/cli_availability_comes_from_the_service.rs`:
      no client source calls `available_here` or `provider().is_available()`, **and** the shell is
      still seen issuing the request, handling the reply, and writing the field. Both halves,
      because a scan for an absence passes trivially once the feature is deleted rather than moved.
      Seen to fail: reintroducing a one-line probe into `features/session.rs` reports
      "``features/session.rs`` calls ``provider().is_available()…)``".
- [x] T141 [US3] *(test)*
      `crates/micold-client/tests/missing_cli_is_reported_where_it_is_chosen.rs`: the notice names
      the CLI, the image, and the obligation; says nothing before the service has answered; says
      nothing when everything is present; names every CLI when the image provides none; and under
      the host placement names no image at all. Asserted by parts rather than verbatim — the parts
      are exactly what FR-023b enumerates, so an assertion that loses one is a requirement that
      stopped being met.

### Implementation

- [x] T142 [US3] `micold-core`: `provider::available_here()`, and the `AiCliAvailabilityRequest` /
      `AiCliAvailability` pair in `protocol/messages.rs`. The reply carries the set and **only** the
      set — not the image, not whether it is containerised at all. The client started the service
      and holds both already, and a second copy of a fact one side owns is a second thing that can
      disagree.
- [x] T143 [US3] `micold-daemon/src/server.rs`: answer the request from `available_here()`. Four
      lines, and the whole of FR-023c: the process that will spawn the CLI is the process that says
      whether it is there.
- [X] ⚠️ **Was reopened (BUG-002), closed again** T144 [US3] `micold-client`: delete `Capabilities::available_providers()`; change
      `State::available_providers` to `Option<CliAvailability>`; ask on connect and on the two named
      events research R11 already required (Settings opening, the override menu opening); fill the
      field from the reply, **stamping it** with what it describes as it arrives. The stamp is read
      from the sandbox's own state rather than the configured placement, because those come apart in
      the one case that matters — after FR-035a's "run without it for now" the placement still says
      the user wanted a container and the thing answering is a host process.
      **Reopened 2026-08-28 (BUG-002)**: three of the four halves landed. The connect-time ask did
      not — `on_connected` calls `ask_cli_availability` above the line that installs the outbox, so
      the call takes its own disconnected early-return every time and no request goes out. Closed
      again 2026-08-28 by T157/T158.
- [x] T145 [US3] `features/settings.rs::missing_cli_notice`, rendered in `ui/settings/environment.rs`
      under the CLI picker and in `ui/settings/daemon.rs` under the image reference — FR-023b's two
      points of choice, and not session start. It names the image the service was **started from**,
      never the one in the draft field: the field may say something the running container has never
      heard of, and naming it would describe a machine that does not exist yet. Muted, not a
      caution: an image with one AI CLI may be exactly what its author intended, and a red warning
      on every visit is a nag rather than an answer.

- [x] T146 [US3] quickstart §B.6, last box: look at the notice in place. The wording is already
      gated on painted strings; what a test cannot settle is whether a muted line under a select
      reads as an answer about the image or as something gone wrong, in both schemes. Record it in
      `evidence/us3-settings-view.md` with the rest of the §B.6 pass. It found one: the notice sat
      in the column of the control *below* it. Fixed with a shared `field_note`, gated by
      `a_field_note_shares_its_fields_column.rs`, which fails on the old geometry.

**Requirements closed**: FR-023b, FR-023c. With them the feature has no unimplemented requirement
left. T146 closed the last §B box with it.

---

## Phase 14: the real-runtime job was testing half the feature

**Goal**: Make the claims the sandbox exists for re-check themselves, rather than resting on one
manual pass. T006 asked for a job "running the `sandbox_real_*` tests"; what it delivered runs
`-p micold-core`, and the other eleven live in `micold-daemon`.

The eleven are not the remainder — they are the headline. `micold-core`'s twelve cover the adapter:
argv a real runtime accepts, the isolation it produces, egress, storage, lifecycle. `micold-daemon`'s
cover what that isolation is *for*, and every one of them is a user story's own claim: the boundary
probed from **inside a session the daemon spawned** (US1, quickstart §B.2), a sandboxed terminal
answering exactly as an unsandboxed one (US2, §B.3), a limit stopping the session and not the
service (US4), the image carrying every AI CLI the application offers (FR-023a), the fingerprint
refusal (FR-024d), and sessions surviving a client restart with the survival opt-in bringing the
sandbox back unasked (FR-014a/b/c).

They ran on no machine at all. Not skipped, not disabled — never built, because cargo was never
asked for that crate's targets, and a package nobody names produces no output to notice. The one
run they have had is T120's, by hand, on 2026-08-26. Meanwhile §B.2 says in writing that its boxes
"re-check themselves on every `sandbox-runtime` CI run", which was false the day it was written.
This is the same shape as the assertion gate's own untested cases, recorded in `ci.yml`: *"They
existed and nothing ran them."*

- [x] T147 `.github/workflows/ci.yml`: a second step in `sandbox-runtime`, `cargo test --release -p
      micold-daemon --features sandbox-real-runtime sandbox_real_ --no-fail-fast --
      --test-threads=1`. `--release` is required rather than preferred — the daemon inside the image
      is release-built, `sandbox_real_session_start` compares it against `CARGO_BIN_EXE_micold-daemon`,
      and a debug host daemon against a release containerised one flatters the container, which is
      the wrong direction for a claim of the form "the sandbox is not much slower". The test refuses
      to report a number when the profiles disagree, refusing is a failure, and without
      `--no-fail-fast` cargo stops at that target and skips every one after it in silence —
      `sandbox_real_staleness` included. That is T120's lesson, which until now was written down in
      a task and enforced nowhere.
- [x] T148 `mise.toml`: `mise run test-sandbox`, both crates in the one invocation that reports the
      truth, refusing early when the runtime or `micold-daemon:dev` is missing. Every way of
      getting this invocation wrong is silent and reports success — naming one crate, dropping
      `--release`, dropping `--no-fail-fast`, dropping `--test-threads=1`, or expecting
      `sandbox_real_` to match file names rather than test names. A one-command form is what stops
      the next pass from rediscovering that list.
- [x] T149 Run all 23 against a real runtime on the branch that adds the job, so the job is landed
      green rather than hopefully: `evidence/real-runtime-ci-coverage.md`.

**Requirements closed**: none new. What closes here is a gap between what the feature claims to
verify continuously and what it verified continuously — which is the sort of gap that is only ever
found by counting.

## Phase 15: nothing published the image FR-024 requires

**Goal**: Make the reference the application ships with resolve to something. FR-024 says the
default image "MUST be published and versioned with the application release and acquired
automatically, so a first run requires no manual image preparation." Through 0.11.0 none of that
was true: `release.yml` built `.deb`s and nothing else, and `DEFAULT_IMAGE` named
`ghcr.io/micold/micold-daemon:<version>` — a namespace this repository does not own, in which
nothing had ever been pushed.

This is the one requirement in the feature with no implementation behind it, and every property
that would normally expose that was pointing the other way. The sandbox is opt-in, so no default
path touches it. The image's *own* tests build `micold-daemon:dev` locally, so the whole
real-runtime suite — all 23 of Phase 14's tests — passes without a registry existing. T117's
performance evidence even wrote the symptom down, that "the route that would (a registry pull) has
nothing published to pull", and read it as a measurement caveat rather than as a missing feature.
What a first-time user would actually have met is a `denied`, on the one path FR-024 exists to make
automatic.

- [x] T150 `crates/micold-core/src/sandbox/image.rs`: point `DEFAULT_IMAGE` at
      `ghcr.io/jaroslawherod/micold-daemon`, this repository's own GHCR namespace, and split the
      repository out into `DEFAULT_IMAGE_REPOSITORY` so the release workflow has one string to
      check itself against. A unit test binds the two together — the split is only worth anything
      while the reference the app resolves is built from the same value the workflow greps for.
- [x] T151 `.github/workflows/release.yml`: an `image` job per architecture (amd64 on
      `ubuntu-22.04`, arm64 on `ubuntu-22.04-arm`) that builds the daemon natively, builds the
      image, and pushes `:<version>-<arch>`; then `image-manifest` composing the multi-architecture
      `:<version>` the client actually asks for. Three guards run before anything is built, because
      each failure they catch produces a *successful* release that no user can pull from: the tag
      must carry the expected prefix, `[workspace.package] version` must equal the tag's version,
      and `image.rs` must name the namespace this job is about to push to.
- [x] T152 The same two runtime checks the local `mise run image` does, now on the release path and
      per architecture: the daemon must *execute* inside the image (a build only proves the file
      was copied — glibc is what decides whether it runs, and the failure surfaces at a user's
      first session, not here), and both AI CLIs must be on `PATH` (FR-023a). Per-architecture
      rather than once, because an npm package with native components is precisely the thing that
      is present on amd64 and absent on arm64.
- [x] T153 `publish` waits for `image-manifest`. A GitHub release is immutable once published, so a
      published release whose version names an unpullable image is permanent; a draft one is a
      re-run. This orders the failure the recoverable way round.
- [x] T154 `packaging/sandbox/Containerfile`: `org.opencontainers.image.source` and friends, so
      GHCR attaches the package to this repository instead of leaving it loose under the account.
      The URL is the real remote and deliberately not `[workspace.package] repository`, which still
      names an older home — following the manifest here would break the link silently.
- [x] T155 Docs: `packaging/sandbox/README.md` (publishing is the release's job),
      `docs/user-guide/sandboxed-daemon.md` and the settings-schema contract moved off the namespace
      that never existed. `evidence/image-publishing.md` records what was verified before the first
      release ran it, and then what that release did.
- [x] T156 Ran it. `micold-ai-ide-v0.12.0` (run `33081010155`) published
      `ghcr.io/jaroslawherod/micold-daemon:0.12.0` as an index over `linux/amd64` and `linux/arm64`,
      with the per-architecture tags beside it. The three guards passed against a real tag, and
      the AI-CLI check passed on the arm64 runner — the one thing the host measurements could not
      reach.
- [x] T157 The first run, end to end, against the published image — a real client on Xvfb resolving
      `DEFAULT_IMAGE` from a settings file that does not name an image, pulling `:0.12.0` from GHCR,
      starting the sandbox and running `claude` inside the container. This is the only check in the
      feature that exercises the default rather than a reference a test supplied, which is precisely
      the gap the wrong namespace slipped through. `evidence/first-run-end-to-end.md`.
- [x] T158 Repair a persisted reference into the retired `ghcr.io/micold` namespace on read
      (`ImageSource::repair_retired_namespace`, called from `StoredSettings::into_settings` beside
      the budget clamp). T155 corrected the constant, which reaches every user with no `sandbox`
      block on disk — and nobody who had ever opened the sandbox section, whose file holds the dead
      namespace as a *value* that no serde default can displace. Found on this developer's own
      machine while setting T157 up: `"reference": "ghcr.io/micold/micold-daemon:0.10.0"`. Scoped to
      registry sources, so an archive or local build named after that namespace keeps its name.
      Tests: four in `sandbox/image.rs`, three in `tests/settings_retired_image_namespace.rs`.

**One prediction here was wrong, and is recorded rather than quietly dropped**: T155 originally
carried a hand step, on the rule that a GHCR package is private when first created and would need
its visibility flipped before FR-024 became observable. It did not — the package was public on
creation, because the job pushes with `GITHUB_TOKEN` from a workflow in this repository, so the
package arrives linked to it and inherits its visibility. An anonymous pull of `:0.12.0` answers
200. That rule is about packages pushed with a personal access token, which arrive unlinked.
`evidence/image-publishing.md` keeps the check that distinguishes the two states, because a private
package's `denied` is indistinguishable from a package that was never pushed.

**T157 also found a defect outside this feature, and did not fix it here**: the session supervisor
restarted `claude` 24 times in 13 minutes without ever reaching `MAX_RESTART_ATTEMPTS`, because the
survivor reset in `state.rs` clears the counter for any process still alive one ~250 ms tick after
its respawn. Feature 010's FR-022a guard therefore only fires against a process that dies inside a
quarter of a second, not against the start-run-die shape every configuration failure actually takes.
Recorded in `evidence/first-run-end-to-end.md`; it belongs to 010.

**Requirements closed**: FR-024.

## Phase 16: Bugfix BUG-002 — the connect-time availability ask never went out

**Goal**: Make T144's first half real. The client asks the service which AI CLIs it can run at three
moments; two of them work. The third — the connection itself — is a call placed 37 lines above the
assignment it depends on, so it returns without sending and `State::available_providers` stays
`None` for the whole run unless the user happens to open Settings. `None` is read as the empty set
by everything that decides what to offer, so the sidebar's override chevron is absent (026 FR-004,
FR-006) and an unavailable default starts instead of offering (026 FR-002).

### Tests for BUG-002 (MANDATORY — Constitution Principle I) ⚠️

- [X] T157 [BUG-002] `crates/micold-client/src/main.rs::tests::connecting_asks_which_clis_the_service_can_run`:
      drive `Msg::Connected` with an outbox whose receiver the test holds, and assert
      `ClientMsg::AiCliAvailabilityRequest` arrives on **that** connection's channel. Behavioural,
      not textual, because that is the whole lesson of this bug:
      `cli_availability_comes_from_the_service.rs` asserts the three spellings appear under `shell/`
      and they do — a dead call site spells identically to a live one, which is why a source scan
      watched this ship. Second assertion in the same test: after the reply is folded in,
      `start_affordance_offers_a_choice()` is true for a two-CLI answer, so the gate covers the
      symptom and not only the send. Confirm both red against the unfixed `on_connected`.
      **Landed in `src/main.rs`'s test module, not `tests/`** as this task first said: `App` is a
      type of the *binary* crate — an integration test under `tests/` links `micold_client`, the
      lib, and cannot name it. The `connect(app, catalog) -> Vec<ClientMsg>` fixture the assertion
      needs was already there. A third assertion was added after the first red run: the connection
      must send `ClientMsg::Attach` too. Without it the fixture passed for a second reason — it sent
      *nothing at all*, so an `Outbox` that dropped every message would have failed the test
      identically to the bug. With an active project the red run reads
      `[Attach, SetViewedSession]` — the outbox demonstrably working, and the availability request
      demonstrably absent.

### Implementation for BUG-002

- [X] T158 [BUG-002] `shell/daemon_sync.rs::on_connected`: move `ask_cli_availability(app)` below
      `app.daemon = Some(outbox)`. The comment already standing at that assignment — "`app.daemon`
      is assigned before the sends, not after, because `view_and_start` below reads it" — states the
      rule this call was breaking; extend it to name the availability ask as the second reader, so
      the next send added above the line is recognised as the same mistake. Done: the ask now sits
      immediately below the assignment and the comment names both readers and says what a send
      placed above the line actually does — returns silently, rather than merely arriving late.

**Bugfix**: 2026-08-28 — BUG-002. **No requirement added**: FR-023c is correct and the plan states
the intended behaviour ("asks on connect and again whenever a surface that offers a CLI opens"); the
code is what drifted. **Clarified, not added**: FR-023c gains a note that a set which has not been
answered yet is not an empty set, and 026 FR-006 the same on its own side — the reading that let the
drift look correct. **One task reopened**: T144. See `bugs/BUG-002.md`.

## Phase 17: Bugfix BUG-003 — choosing "In a container" saved the choice and did nothing else

**Goal**: Make the placement select move the daemon. Set **Where sessions run** to *In a container*,
press Save, and today the view closes, the note reads "Currently in a container.", and the session
service is still the host process it was — every session unconfined, nothing said. The next launch
does come up sandboxed, which is why this survived: it is not broken forever, only for as long as
the user believes they are contained and are not.

FR-032 and FR-033 said to confirm and restart. Neither produced a task, because `data-model.md` §7
had folded them into `SandboxState`'s requirement range. FR-032a, FR-032b and FR-033a settle what
they left open.

### Tests for BUG-003 (MANDATORY — Constitution Principle I) ⚠️

- [X] T159 [BUG-003] *(test)* `crates/micold-client/tests/saving_a_placement_moves_the_daemon.rs`:
      a save whose draft placement differs from the store's asks for confirmation and applies
      **nothing** before it is answered — not the placement, and not the other fields either
      (FR-032, FR-032a). Confirmed red against the current reducer, where the save writes every
      field and asks nothing.
      **Written in two files, not one.** T159–T162 name a single integration test, and half of what
      they assert is unreachable from one: `App`, `shell::persist` and `app.placement` live in the
      **binary**, so `tests/*.rs` cannot see them. The pure half (what the reducer does with a
      pending change) is the file named above; the shell half (T162) is in `main.rs`'s
      `#[cfg(test)] mod tests`, beside the other `update_inner` tests. Splitting on the crate
      boundary rather than writing the lot as binary tests keeps the rules that *are* pure testable
      without a `Task` runtime, which is the same division M2 draws in the code.
- [X] T160 [BUG-003] *(test)* Same file — a save that leaves the placement where it was asks
      nothing and behaves exactly as it does today, and choosing a placement then pressing Cancel
      asks nothing (FR-032a). This is the rule that keeps the fix from becoming a modal in front of
      every Save; it mirrors `persist.rs::survival_step`'s "act on a change, and only on a change",
      which is why the two are asserted together.
- [X] T161 [BUG-003] *(test)* Same file — declining leaves the store byte-for-byte unchanged, the
      settings surface open, and the draft holding every edit the user had made, the placement
      included (FR-032b). The one that would catch a fix which saves the other four fields and
      silently drops the fifth — the failure class BUG-001 was.
- [X] T162 [BUG-003] *(test)* `crates/micold-client/src/main.rs` (`mod tests`, not the file above —
      see T159) — confirming moves `app.placement`, and yields the bring-up
      task rather than deferring to the next launch (FR-033a). Asserted through the boot plan the
      save must construct, since a user switching *into* the sandbox has none from `startup.rs`;
      without that the fix would be an assignment that dials a port nothing is listening on.
- [X] T163 [BUG-003] *(test)* [P] `crates/micold-client/tests/settings_reports_the_live_placement.rs`:
      the note under the select renders the placement **in force**, not the draft's, so it cannot
      read "Currently in a container." over a host process (FR-035b). A separate file because it is
      a view property and needs none of the save path.

### Implementation for BUG-003

- [X] T164 [BUG-003] `crates/micold-client/src/ui/confirm_placement.rs` and its `SurfaceId` in
      `features/settings.rs` — the modal, following the `confirm_*.rs` family's shape
      (`FloatingSurface` + `Registered`, `material::dialog::fields`/`actions`). It names the
      placement being moved to, that the service restarts, and — when sessions are running — how
      many stop and that they become resumable (FR-032, FR-033).
- [X] T165 [BUG-003] `shell/persist.rs::on_settings_saved` — read the stored placement before the
      write, alongside the survival opt-in that already does this; on a difference, raise the
      confirmation and return without writing. Split the applying half into a
      `on_placement_change_confirmed` that performs the save it deferred, assigns `app.placement`,
      builds the `BootPlan` from the validated settings, and returns `shell::sandbox::boot` — the
      third caller of the mechanism `RestartRequested` and `FallbackAccepted` already use
      (FR-032a, FR-032b, FR-033a).
- [X] T166 [BUG-003] `crates/micold-client/src/ui/settings/daemon.rs` — the note reads the live
      placement, and the select's supporting text stops saying "Takes effect the next time the
      application starts". That string was an accurate description of the code and an undeclared
      narrowing of FR-032, recorded where nobody would read it as a gap; FR-033a no longer permits
      what it promises (FR-035b).

**Bugfix**: 2026-09-03 — BUG-003. **Requirements added**: FR-032a, FR-032b, FR-033a — see `spec.md`.
**Design corrected**: `data-model.md` §7's requirement range, which is what swallowed FR-032/FR-033;
`plan.md` gained the increment. **No task reopened**: T070 built the control it was asked for and is
annotated in place, because the defect is a task that was never written rather than one that
drifted. See `bugs/BUG-003.md`.

## Phase 18: Bugfix BUG-004 — switching to a container left the client dialling a sandbox nobody starts

**Goal**: Give the sandboxed placement the availability guarantee the host placement has always had.
Choose *In a container* and the client starts dialling 127.0.0.1:7727 once a second; if the container
is not up — no image pulled yet, the user stopped it, the bring-up failed once at launch — nothing
ever starts it. `connect_or_spawn` does exactly this for the host process on every attempt;
`connect_at` does not, and no caller makes up the difference. The sandboxed bring-up has three
triggers, all one-shot: launch (`shell/startup.rs:307`), a placement change
(`shell/persist.rs:391`), and a person pressing Restart (`Msg::RestartRequested`). Miss all three
and the only remedy is relaunching the application.

Underneath it a second defect makes the first one unreadable: `shell/sandbox.rs::boot` passes
`&mut |_| {}` as the progress callback, so a bring-up that is working sits on `Probing` for its whole
duration while `on_connect_failed` posts "Could not connect to the session daemon" once a second.
A working bring-up looks identical to a failed one, and louder. `ui/sandbox_status.rs` renders every
stage correctly and is driven by nothing.

FR-002a, FR-036a and FR-036b settle the first; SC-004c and `data-model.md` §7's S-1 clause settle
the second — T117's measurement passed over it because it ran in `micold-core`, around
`acquire_image`, below the level the dropped callback lives at.

### Tests for BUG-004 (MANDATORY — Constitution Principle I) ⚠️

- [X] T167 [BUG-004] *(test)* `crates/micold-core/src/sandbox/lifecycle.rs` (`mod tests`) — the
      `service_absent` edge: from `Failed` with attempts remaining it yields `Probing`; from
      `Probing`, `Acquiring` or `Starting` it yields `None`, so a bring-up already in flight is never
      started a second time; from `Running` and `Stale` it yields `None`, which is S-7's scope line —
      R9 protects a *running* sandbox and this edge never touches one; and once the bound is spent it
      yields `None` for good, leaving `Failed` standing for FR-034's manual remedy (S-6, S-2,
      FR-036a). The bound and the spacing are values the caller reads, not sleeps, so this stays a
      pure test — the same division S-5 draws.
      *Deviation:* written in `crates/micold-core/tests/sandbox_state.rs` rather than `mod tests`,
      beside the other lifecycle edges and the `every_state()` helper the scope assertion needs.
      Red against a stub returning `None`: three of the four failed on their own assertions; the
      fourth (`absence_only_brings_up_a_sandbox_that_has_failed`) was confirmed by a mutant that
      admitted `Probing`.
- [X] T168 [BUG-004] *(test)* `crates/micold-client/src/main.rs` (`mod tests` — `App`,
      `shell::daemon_sync` and `app.sandbox_boot` live in the binary and are not reachable from
      `tests/*.rs`, the same reason T162 sits there) — a connect failure under a `LocalSandbox`
      placement that holds a `BootPlan` yields the bring-up task **and** moves the state off
      `Probing` as its stages arrive, rather than notifying a connection failure (FR-002a, FR-036a,
      FR-036b, SC-004c); a second failure while that bring-up is in flight yields nothing; and with
      no plan, or with the attempts spent, it reports the failure exactly as it does today. Confirmed
      red against `on_connect_failed`, which returns `Task::none()` in every one of those cases, and
      against `boot`, whose `&mut |_| {}` makes the stage assertion unsatisfiable.
      *Deviation:* the stage-ordering assertion lives in `shell/sandbox.rs`'s `mod tests`
      (`every_stage_a_bring_up_enters_is_reported_before_how_it_ended`, over the extracted
      `reported` stream), since `update_inner` does not run the tasks it returns; `main.rs` asserts
      the `Msg::Progress` arm moves the state. Added: a sandbox that came up earns its attempts back,
      and a `HostProcess` placement never brings one up (FR-035). Five were red against the stubs;
      the three that already passed (no plan, spent, host placement) were each confirmed by a mutant.

### Implementation for BUG-004

- [X] T169 [BUG-004] `crates/micold-core/src/sandbox/lifecycle.rs` — the `service_absent` transition
      and the attempt budget guarding it, beside `bring_up` and `container_lost`. It is a distinct
      witness from `RestartRequested`, which exists to say a person asked; this edge exists because
      nobody has to (S-6). `container_lost` stays as it is — it answers a different question (the
      container we were *using* went away) and returns `None` for `Probing`, which is why the FR-036
      machinery is inert exactly when it is needed.
      *Landed as* `service_absent`, `UnattendedBringUps` and `UNATTENDED_BRING_UP_DELAYS` (0s, 5s,
      15s), plus `SandboxState::is_coming_up` for T171's FR-036b check; `Sandbox::service_absent`
      applies it client-side and `Sandbox::started` restores the budget.
- [X] T170 [BUG-004] `crates/micold-client/src/shell/sandbox.rs::boot` — thread `observe` through to
      the application instead of discarding it, so `StageProgress` reaches `Msg::Progress` and the
      view moves. The comment there deferred this on the grounds that a `Task::future` yields one
      message and the settings view would be worth more; `Task::stream` over a channel the `start`
      callback feeds is the shape that fits. Closes T041's progress half and T043's first clause.
      *Landed as* `reported(after, work)` (the stream) under `boot_after(plan, after)`; `boot` is
      `boot_after(plan, ZERO)`.
- [X] T171 [BUG-004] `crates/micold-client/src/shell/daemon_sync.rs::on_connect_failed` — consult
      `app.sandbox_boot`, which already holds the plan, and ask T169's edge whether to bring the
      sandbox up; on a yes, return `sandbox::boot(plan)` and report the stage rather than the dial
      failure (FR-036b). Bounded and spaced by the budget, never by the connection's 1 Hz retry, and
      never a silent fall back to the host process — FR-035a's consented fallback is the only way out
      and it stays a user decision (FR-036a, S-2).
      *Landed as* `boot_after(plan, delay)` — the budget's spacing is the wait — and a refused dial
      while `is_coming_up()` is silent. `LocalSandbox` placements only.
- [X] T172 [BUG-004] Re-run `quickstart.md` §B.1 with the sandbox stopped from outside while the
      application is open, and re-measure SC-004/SC-004c **at the application** — cold image state,
      the registry route Phase 15 published, stage changes no more than 10s apart — into
      `specs/027-sandboxed-daemon-runtime/evidence/performance.md`. Closes T043's second clause and
      T117, whose numbers were taken in `micold-core` and therefore could not see the dropped
      callback.
      *Result (2026-09-13, Xvfb + lavapipe)*: registry route ~23s from launch, stage changes ≤2.37s
      apart; stopped from outside → re-attached unattended ~7.7s after the container exited; zero
      "Could not connect" toasts. **SC-004c's FR-036b clause still fails**: the red "Not connected to
      the session service · Reconnecting..." banner outranks the stage for the whole bring-up, and a
      "The sandbox did not start … Restart the sandbox" card flashes before the automatic bring-up.
      Not fixed here — needs its own task. Registry route stops at the v10/v9 handshake refusal (no
      v10 image published); attach was shown on the local-build route. See `evidence/performance.md`.

**Bugfix**: 2026-09-12 — BUG-004. **Requirements added**: FR-002a, FR-036a, FR-036b, SC-004c and
US6 scenario 9 — see `spec.md`. **Design corrected**: `data-model.md` §7 gained rules S-6 and S-7 and
the two `Failed` edges; `contracts/container-runtime.md` C-8 gained the caller's half of the
guarantee; `plan.md` gained the increment. **Three tasks reopened**: T041 and T043, whose progress
glue never landed, and T117, which measured SC-004 a layer below where the defect lives. See
`bugs/BUG-004.md`.


## Phase 19: TDD remediation — BUG-004

**Goal**: Clear the findings of `tdd/verification.md` (2026-09-13, verdict **FAIL**). **BUG-004 is not
done until T173–T179 are cleared**: T171's "bounded and spaced", T168's "yields the bring-up task" and
T170's progress wiring each survive a mutant that removes them, and FR-036b and FR-036a's per-attempt
reason are untested. Each task names the mutant from the report's *Mutation results* table. Done means
a new test fails with that mutant applied, and the full suite (`mise run test`) is green without it.

### Blocking (HIGH)

- [X] T173 [BUG-004] *(test)* Finding 2 — `crates/micold-client/src/main.rs:2981` (`connection_failed`
      discards the task). Return the `Task` and assert that a refused dial on a failed sandbox yields a
      bring-up; a refused dial during one (`:3020`), with no plan (`:3068`), or on the host placement
      (`:3080`) yields none. Proven when M13 (`daemon_sync.rs:307` → `Task::none()`) fails it:
      `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide`.
- [X] T174 [BUG-004] *(test)* Finding 1 — `crates/micold-client/src/shell/sandbox.rs:221` and
      `crates/micold-client/src/shell/daemon_sync.rs:307`. Pin that an unattended bring-up waits the
      budget's delay before starting (paused tokio clock over `reported`/`boot_after`, or an equivalent
      observable), and that `on_connect_failed` passes the delay it was given. Proven when M10 (wait
      removed) and M11 (`boot_after(plan, ZERO)`) each fail it: `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide`.
- [X] T175 [BUG-004] *(test)* Finding 3 — `crates/micold-client/src/features/sandbox.rs:317`. Drive
      the budget through messages alone (`ConnectFailed`, then `SandboxMsg::Failed`, repeated) without
      writing `app.sandbox.unattended` in setup (`main.rs:3052`, `:3118`). Assert exactly
      `UNATTENDED_BRING_UP_DELAYS.len()` bring-ups, then the failure reported and `Failed` left standing.
      Proven when M14 (budget write dropped) fails it: `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide`.
- [X] T176 [BUG-004] *(test)* Finding 4 — `crates/micold-core/src/sandbox/lifecycle.rs:122`. An
      `every_state()` test of `is_coming_up` in `crates/micold-core/tests/sandbox_state.rs`, and B6
      (`main.rs:3020`) extended over `Probing`, `Acquiring` and `Starting`. Proven when M15 (`Probing`,
      `Starting` → false) fails both: `scripts/build-lock.sh cargo test -p micold-core --test
      sandbox_state` and `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide`.
- [X] T177 [BUG-004] *(test)* Finding 5 — `crates/micold-client/src/shell/sandbox.rs:200`, test at
      `:492-497`. Exercise the production bring-up closure, not one the test writes. Extract it (for
      example `boot_work(plan, runtime)`), feed it a `CliRuntime` over `RecordingRunner`, and assert that
      `Progress(Probing)` arrives first and the outcome last. Proven when M16 (`observe` → `&mut |_| {}`)
      fails it: `scripts/build-lock.sh cargo test -p micold-client`.
- [ ] T178 [BUG-004] Finding 6 — FR-036b / SC-004c. Test first, in `main.rs` `mod tests`: after a
      refused dial starts a bring-up, and for every `is_coming_up()` state, `connection_status(&app)` is
      not `Disconnected`, and no "The sandbox did not start" card offering FR-035a's fallback is shown.
      Both are red today (`daemon_sync.rs:296` sets `disconnected` unconditionally; `ui/mod.rs:118-122`,
      `:231`). Replace the toast-only FR-036b assertions at `main.rs:3013` and `:3037`, then fix until
      green. Proven by `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide` and by re-running T172's stopped-from-outside pass with the banner
      absent from the frames (`evidence/performance.md`).
- [X] T179 [BUG-004] Finding 7 — FR-036a / US6 scenario 9, "each attempt MUST report why it failed".
      Test first: after a failed attempt and the refused dial that starts the next one
      (`features/sandbox.rs:315-318` moves `Failed(reason)` to `Probing`), the previous attempt's reason
      is still reported — in the log line at `daemon_sync.rs:303-306` and where the user can see it.
      Red today; fix until green. Proven by `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide`.

### Non-blocking (MED, LOW)

- [X] T180 [BUG-004] Finding 10 — test `ConnectFailed` arriving while the state is still `Running`,
      before `check_alive`'s `Lost` (`daemon_sync.rs:286-289`, `:302-317`). Decide and pin whether it
      is silent or reported; today it notifies "Could not connect". `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide`.
- [X] T181 [BUG-004] Finding 9 — a `sandbox_real_*` test behind `sandbox-real-runtime`: stop the
      container from outside with a client attached and assert re-attachment without user action within
      the budget. `mise run image && mise run test-sandbox`.
- [X] T182 [BUG-004] Finding 11 — `main.rs:2990-2996`: assert on notification level and on both
      visible and pending, not on prose in `visible()`. Proven by rewording `daemon_sync.rs:317` in a
      scratch change and seeing the negative cases (`:3013`, `:3037`) still meaningful. `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide`.
- [X] T183 [BUG-004] Finding 12 — `crates/micold-core/tests/sandbox_state.rs:590`: check S-6 against
      the real `RECONNECT_BACKOFF` (`crates/micold-client/src/daemon.rs:127`), not a copy, and name the
      `10` ceiling (`:570`, `:597`). `mise run test`.
- [X] T184 [BUG-004] Finding 13 — `sandbox_state.rs:615`: move the length assertion out of the spacing
      test (the bound test owns it) and guard the `.skip(1)` loop at `:603` with `waits.len() >= 2`.
      `scripts/build-lock.sh cargo test -p micold-core --test sandbox_state`.
- [X] T185 [BUG-004] Findings 14–16 — rename `main.rs:3094` (it asserts state, not display) and
      `sandbox_state.rs:434` (`only` is no longer true for `Failed`); guard the setup loop at `main.rs:3049`;
      add rule messages to `main.rs:3078-3079`, `:3089-3090`. `mise run test`.
- [X] T186 [BUG-004] Finding 8 — record the test-first evidence with the work, not in a session
      scratchpad: write `tdd/test-list.md` for Phase 18/19, and commit T173–T179 with their red output,
      so a re-run of `/speckit.tdd.verify` can grade ordering from history.
- [X] T187 [BUG-004] T178's visual finding 1 (`evidence/performance.md`) — FR-036b / FR-035a. Test
      first, in `main.rs` `mod tests`: after the liveness check reports the container stopped
      (`SandboxMsg::Lost`) with unattended attempts left, a bring-up is scheduled in the same update, and
      neither "The sandbox did not start" nor the fallback nor the `Disconnected` banner is shown. Red
      today: `Failed(SandboxStopped)` stands until the next refused dial (~2s). Proven by
      `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide`.
- [X] T188 [BUG-004] T178's visual finding 2 — FR-036b / SC-004c. Test first: after `Started`, and
      until the service answers, `connection_status(&app)` is not `Disconnected`; bounded, so a service
      that never answers is still reported, and a service that answered and then went away is still a
      lost connection. Red today (~0.5s banner at first enable and on recovery). Proven by
      `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide`, then T178's visual re-run.

**Order**: T173 first; T174 and T175 build on the task it exposes. T176 and T177 are independent.
T178 and T179 change behavior and follow T173–T176, whose tests cover the code they touch. Then
re-run `/speckit.tdd.verify`.

---

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

**Phase 13**: T138, T139 and T141 are three independent test files. T140 is the only one that has to
follow its implementation, because what it asserts is an absence that does not exist yet. T146 is a
manual pass and follows everything.

**Phase 14**: T147 and T148 are independent files. T149 has to follow both, since what it records is
those two run.

**Phase 15**: T150 and T154 are independent files. T151 has to follow T150 — its guard greps for
what T150 writes — and T152/T153 are steps and edges within T151's job graph rather than separate
work. T155 follows everything, because a release that has not run yet is the one thing it cannot
record.

**Phase 16**: none. T158 is one moved line and T157 must fail against the code before it moves, so
the two are strictly ordered.

**Phase 17**: T159–T161 are one file and sequential within it. T162 is a *different* file — the
binary's own `mod tests`, because `App`, `shell::persist` and `app.placement` are not reachable from
`tests/*.rs` — so it is parallel with the three, as is T163. T164 and T166 touch disjoint modules
and are parallel; T165 needs T164's message to exist before it can raise anything.

**Phase 18**: T167 and T168 are different crates and are parallel. T169 and T170 are disjoint —
one is the core edge, the other the client's progress glue — and are parallel; T171 needs T169's
transition to exist before it can ask anything. T172 is a manual pass and a measurement, and follows
all four.

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
