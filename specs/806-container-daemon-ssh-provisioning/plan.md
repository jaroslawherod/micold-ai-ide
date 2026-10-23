# Implementation Plan: Container daemons provisioned over SSH from a basic image

**Branch**: `claude/project-thread-uwrpqv` | **Date**: 2026-10-17 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/806-container-daemon-ssh-provisioning/spec.md`

**Bugfix**: 2026-10-23 — [BUG-1165](./bugs/BUG-1165.md) Updated from bugfix patch (FR-007 row, Phase 14)

## Summary

A container daemon stops running the released `micold-daemon` image. It runs a **basic image**
(`ghcr.io/jaroslawherod/micold-basic`, published by a new on-merge workflow and pinned by digest in
the client) that carries sshd, Node and session tools but no daemon and no AI CLI. On start the
client publishes the container's sshd on the daemon's stored loopback port, reaches it with a
per-daemon key and pinned host key, copies its own daemon build (or a bundled static Linux build
for the container's architecture) only when absent, installs the selected AI CLIs into the
host-mounted state directory (through a registry-only relay when the posture blocks egress), and
connects over the 489 `remote bridge` — no TCP listener, no token. That sequence is one shared
module, `micold_core::provision`, which the 489 remote-host runtime moves onto unchanged and
feature 686's pod adopts by supplying only how to reach its sshd. Stored daemons on the released
image migrate to the basic image at load; a legacy container still running sessions is attached
as before until restarted with none.

Decisions and rejected alternatives: [research.md](./research.md). Types:
[data-model.md](./data-model.md). Interfaces: [contracts/](./contracts/). Proof:
[quickstart.md](./quickstart.md).

## Technical Context

**Language/Version**: Rust (workspace toolchain, `rust-toolchain.toml`); POSIX `sh` for the image's
entrypoint and the remote scripts.

**Primary Dependencies**: existing only — tokio (relay, cancellation), serde, iced (form, row).
System tools already required: the OpenSSH client (`ssh`, `ssh-keygen`; 489), Docker/Podman (027).
Build only: `cargo-zigbuild` + zig for the two `*-unknown-linux-musl` daemon builds, run once on a Linux runner (`linux-daemons` job in `ci.yml`/`release.yml`); macOS and Windows jobs download the artifact because zig crashes on the Windows ARM64 runner.
No new crate.

**Storage**: `settings.json` (daemon entries, new keys `image_choice`, `ai_clis`, `migration`;
[contracts/daemon-settings.md](./contracts/daemon-settings.md)); per daemon state dir: `ssh/` keys
(`NEW`), `ai-clis/` installs and `offered.json` (`NEW`, mounted at `/var/lib/micold-ai-ide`),
`sandbox-home/.micold/remote/<build>/` daemon builds (the 489 layout inside the mounted home).

**Testing**: `cargo test` (core unit and integration, client, daemon), `scripts/tests/*.test.sh`,
real-runtime suites behind `sandbox-real-runtime` and `ssh-real-runtime` features (CI jobs
`sandbox-runtime`, `ssh-remote`), quickstart §B visual pass for the form and the row.

**Target Platform**: clients on Linux, macOS (Intel, Apple silicon), Windows; containers are Linux
`x86_64` or `aarch64` under Docker or Podman.

**Project Type**: desktop app (iced client + session daemon + render-free core).

**Performance Goals**: SC-002 first start < 5 min excl. image download; SC-003 warm start
< 15 s once the container runs (probe + bridge, nothing copied or downloaded).

**Constraints**: loopback only (FR-006); step bounds 15 min / 10 min / 120 s (FR-019); no
silent retry; no change to another daemon (FR-022); offline start when everything is in place.

**Scale/Scope**: a handful of container daemons per client; 5 AI CLIs; 2 container architectures.

## Constitution Check

*GATE: passed before Phase 0; re-checked after Phase 1 (below, unchanged).*

| Principle | Verdict |
|---|---|
| **I. Test-First** | PASS. Every unit below starts from a failing test at its layer (Test strategy). Pure pieces (argv, migration, adopt, install script, relay allowlist, reason texts, bounds) are core unit tests with a fake `SshRunner`/`ContainerRuntime`; end-to-end behaviour is the real-runtime suite. |
| **II. Multi-Session Support** | PASS. Keys, installs, `offered.json`, container and build dirs are per daemon; provisioning holds no cross-daemon lock (contract invariant 6); a running legacy container's sessions are never ended (R10). |
| **III. Worktree Integration** | PASS. Project and worktree mounts are unchanged (027 `MountSet`); nothing here creates or moves a worktree. |
| **IV. Local-First Storage** | PASS. All state stays in the local state dir and `settings.json`. The basic image and AI CLI packages are fetched only when a start needs them and the user chose a container daemon (as today's image pull and CLI installs); a start with everything in place is fully offline (edge case "Offline start"). The install relay forwards only to the pinned npm registry, only during the install step. Nothing is uploaded. |
| **V. Rust + iced** | PASS. Rust and iced only; `OwnImage` makes "own image with neither reference nor archive" unrepresentable, `Reach` separates user-config and pinned ssh, `Step`/`FailureCause` are closed enums. Shell appears only as the image's entrypoint and the fixed remote scripts, as in 489. |
| **VI. Cross-Platform Parity** | PASS. Same flow on all three clients; platform differences sit in `ssh::argv` (`ssh.exe`), `owner_only.rs` (key ACLs) and the bundled musl builds (R7). CI: unit tests on the three-OS matrix; real runtime on Linux (Docker on macOS/Windows runners is unavailable, as for 027/489 — the same limit, recorded there). |
| **VII. Documentation** | PASS. `docs/user-guide/sandboxed-daemon.md` (image choice, user image requirements, AI CLIs, resources, migration; no released image) and `docs/user-guide/remote-daemons.md` (shared steps, bundled builds) change in the milestone that changes the behaviour (FR-026); `packaging/basic/README.md` for maintainers. |
| **VIII. Reusable UI Components** | PASS. The form uses the existing shared `Select`, `TextField`, `Checkbox` (`ui/material/`) in `ui/settings/container_fields.rs`; the row's step and progress use the existing `progress.rs` indicator. No new widget. |

No violation: Complexity Tracking is empty.

## Requirement map

| FR | Where |
|---|---|
| FR-001 | `ImageChoice` (data-model); form `Select` ([daemon-settings](./contracts/daemon-settings.md)) |
| FR-002 | `packaging/basic/`, `basic-image.yml`, `BASIC_IMAGE` pin ([basic-image](./contracts/basic-image.md), R1) |
| FR-003 | `DEFAULT_IMAGE` removed from every default; save refuses the released image; migration (R9) |
| FR-004 | `OwnImage` pull/load through the existing `acquire_image`; form note; user guide |
| FR-005 | `provision` sequence Reach→Probe→Copy→Start ([provisioning](./contracts/provisioning.md)), R2 |
| FR-006 | `DaemonKeys`, `Reach::Pinned`, loopback publish (R2, R4) |
| FR-007 | `VersionCheck` step = 489 FR-011 rule; client side (BUG-1165): `links::remote_mismatch`, `ask_offer`, `ReplaceRequested` and `ssh::status` over `Reach::Pinned` for containers; banner restart targets the active daemon (`shell/service_control.rs`); legacy Stop gate off the session cache |
| FR-008 | `micold_core::provision` + `ProvisionTarget`; 489 moved onto it; contract written (R5) |
| FR-009 | `scripts/remote-daemon-bundle.sh`, bundles in macOS/Windows/deb/rpm packages, `NoMatchingBuild` (R7) |
| FR-010 | `AiCliSelection` (`new_daemon`/`all`), form checkboxes |
| FR-011 | `InstallAiClis` step into `<state>/ai-clis/<cli>@<version>` (R6), relay (R8) |
| FR-012 | `offered.json` → daemon `PATH`; `ProvisionWarning` (R6) |
| FR-013 | credential mounts unchanged (027); install script never touches them; image carries none |
| FR-014, FR-015 | `ResourceBudget` unchanged, applied in `SandboxSpec`; restart confirmation unchanged |
| FR-016, FR-017 | read-repair (R9), `MigrationNotice`, `Adoption::Legacy` (R10) |
| FR-018 | `ProvisionFailure` + reason table (contract), recent errors (R13) |
| FR-019, FR-020 | `StepBounds`, `CancelToken`, `AttemptLedger` rollback (R11) |
| FR-021 | `ConnectionMsg::Provisioning(ProvisionEvent)` → row (R13) |
| FR-022 | per-daemon state only; starts run on the blocking pool per daemon as today |
| FR-023 | create-conflict re-adopt, atomic copy, install `flock`, daemon singleton (R12) |
| FR-024 | Principle VI row above |
| FR-025 | `sandbox_real_provision.rs` + existing `ssh_real_*` in CI (R14) |
| FR-026 | user guide pages (Principle VII row) |

## Design

### Core (`crates/micold-core/src/`)

- **`provision/` (`NEW`)**: `mod.rs` (`ProvisionTarget`, `Ownership`, `Egress`, `StepBounds`,
  `Step`, `ProvisionEvent`, `Provisioned`, `run()` — the sequence over `SshHost`), `failure.rs`
  (`ProvisionFailure`, `FailureCause`, `ProvisionWarning`, reason texts), `ai_cli.rs` (pinned table
  moved from the Containerfile `ARG`s, install script builder, `offered.json` shape), `relay.rs`
  (allowlist parse + tokio CONNECT relay), `keys.rs` (`DaemonKeys`, `ssh-keygen` argv, owner-only),
  `attempt.rs` (`CancelToken`, `AttemptLedger`). The `SshRunner` / `Artefacts` / `SshHost` traits
  and `Probe` stay in `ssh/provision.rs`, whose `provision()` / `connect()` become thin wrappers
  calling `provision::run` with the `UserHost` target, so 489 callers and tests are unchanged.
- **`ssh/argv.rs`**: `Reach` enum; `command()` takes `&Reach`; `Pinned` emits `-F none`,
  `IdentitiesOnly=yes`, `IdentityAgent=none`, `UserKnownHostsFile`, `GlobalKnownHostsFile=none`,
  `-i`, `-l`, `-p`; `install()` builder (the one command without `ClearAllForwardings`, carrying
  `-R` when relayed); `reach_check()` (`true`). `ssh/system.rs`: kill the child on `CancelToken`.
- **`sandbox/image.rs`**: `ImageChoice`, `OwnImage`, `ImageSource::into_choice` (migration input).
  **`sandbox/basic_image.rs` (`NEW`)**: pin constants, `is_released_image`.
- **`sandbox/mod.rs`**: `SandboxProfile { image: ImageChoice, ai_clis: AiCliSelection, … }`;
  `SandboxSpec` publishes `2222`, adds the read-only key mount and `MICOLD_HOME`, drops the token
  mount for the new path. **`sandbox/argv.rs`**: the matching `create` arguments.
- **`sandbox/lifecycle.rs`**: `Adoption::Legacy`, no fingerprint replace; `bring_up` takes a
  `CancelToken` and records into `AttemptLedger`; create-conflict re-adopt (R12); `Stage` maps onto
  `Step::{Acquire, CreateContainer}`. **`sandbox/exec.rs`**: kill on cancel; `acquire_image` bound
  15 min.
- **`daemons.rs`**: `ContainerSettings.migration`; `migrated_entry` and the registry read apply the
  migration; **`settings.rs`**: the read-repair calls the migration where
  `repair_retired_namespace()` runs today. `ImageSource::repair_retired_namespace` stays and still
  runs first (retired namespace → current released reference); `into_choice` then maps the repaired
  `ImageSource` onto `ImageChoice`. Its unit tests in `image.rs` and
  `settings_retired_image_namespace.rs` stay; the latter is extended to assert the final `ImageChoice`.

### Daemon (`crates/micold-daemon/src/`)

- **`state.rs`**: `spawn_path_for` prepends the bin dirs of `<state>/ai-clis/offered.json` when the
  file exists (container daemons only in practice), so `ai_clis_available_in` offers selected and
  installed CLIs only. No protocol change.

### Client (`crates/micold-client/src/`)

- **`daemon.rs`**: `Dial::ContainerSsh { reach, target }` for basic/own images (calls
  `provision::run` with the `Container` target); `Dial::Container` kept for `Adoption::Legacy` only.
- **`shell/sandbox.rs`**: `start` ensures `DaemonKeys`, brings the container up, then provisions;
  `InFlight` carries the `CancelToken`; on success clears `migration` and saves; on failure rolls
  back.
- **`features/connection.rs` / `features/daemons.rs`**: `ConnectionMsg::Provisioning(ProvisionEvent)`
  replaces `CopyProgress`; per-daemon `step`, `percent`, install lines, warnings; recent errors gain
  the failure.
- **`ui/settings/container_fields.rs`**: image `Select` (two entries), own-image fields and note, AI
  CLI `Checkbox` list; **`ui/settings/daemons.rs`**: row shows step/progress, migration and legacy
  notices, restart enabled only with no sessions for legacy.

### Packaging, CI, release

- `packaging/basic/` (`Containerfile`, `entrypoint.sh`, `sshd_config`, `README.md`) `NEW`;
  `packaging/sandbox/Containerfile` stays for the release job only.
- `.github/workflows/basic-image.yml` `NEW`; `ci.yml` `sandbox-runtime` builds `micold-basic:dev`,
  adds the pin check; `release.yml` and the bundle scripts build and ship the musl daemons;
  `scripts/basic-image-hash.sh`, `scripts/remote-daemon-bundle.sh` `NEW`.
- `mise.toml`: `image` builds `micold-basic:dev`; `test-sandbox` unchanged in shape.

## Test strategy

| Layer | Covers |
|---|---|
| Core unit (`micold-core`, fakes) | `Reach` argv for both variants (489 argv byte-identical); sequence order, skip rules, bounds, cancel, `Reach` retry only on not-up-yet; every `FailureCause` reason text; install script text and `offered.json`; relay allowlist (allowed, refused, non-CONNECT); migration table row by row (`settings_retired_image_namespace.rs` extended + `NEW settings_released_image_migration.rs`); `adopt` incl. `Legacy`; `AiCliSelection` defaults; `ImageChoice` validation; `BASIC_IMAGE` has a digest; `AttemptLedger` rollback; the install script text and `SandboxSpec` mounts never read, write or mount a credential path beyond the existing 027 credential shares (FR-013); `ResourceBudget` still applied to `SandboxSpec` for basic and own images (FR-014, FR-015). FR-001–FR-020, FR-023. |
| Daemon (`micold-daemon`) | `offered.json` → `ai_clis_available_in` (present, missing, malformed, deselected). FR-012. |
| Client (`micold-client`) | progress events into the row state, warnings and recent errors; form validation (empty own image, released image refused, empty selection accepted); legacy restart enabled only with zero sessions; the resource fields keep their existing validation and restart confirmation for both image choices (FR-014); `unavailable_default_says_so.rs` extended for a deselected / failed CLI (D4). FR-001, FR-010, FR-012, FR-014, FR-017, FR-018, FR-021. |
| Geometry gates | `NEW crates/micold-client/tests/gates/container_section_fits.rs`: the Container section with the two-entry select, own-image fields and five checkboxes keeps its fields aligned and unclipped at the narrowest Settings width. FR-001, FR-010. |
| Real runtime (`sandbox_real_provision.rs` `NEW`, Linux CI) | fresh start reports the client's version; warm start copies and downloads nothing (< 15 s); image without sshd → `NoSshServer`; stand-in CLI install failure → warning, daemon connected; migrated daemon starts on basic; limits applied; stop during start leaves nothing; concurrent starts → one container. `ssh_real_*` unchanged and green on the shared sequence. The existing `sandbox_real_*` suites (both crates) move onto `micold-basic:dev` + provisioning through their `sandbox_real_support` harness; `sandbox_real_fingerprint.rs` / `sandbox_real_staleness.rs` are rewritten to the handshake version rule (R10), since the fingerprint replace is gone. `sandbox_real_multi_daemon*.rs` gains a case where one daemon is provisioning or fails provisioning while another stays connected and responsive (FR-022). FR-005–FR-007, FR-011, FR-012, FR-015, FR-017–FR-020, FR-022, FR-023, FR-025, SC-001, SC-003, SC-004, SC-006. |
| Script tests (`scripts/tests/`) | `basic-image-hash.test.sh` (stable, order-independent, `--check`); `remote-daemon-bundle.test.sh` (489 T082: packages carry both musl daemons); a Containerfile check in `basic-image-hash.test.sh` that `packaging/basic/` copies no credential or key into the image (FR-013). FR-002, FR-009, FR-013. |
| Three-OS CI matrix (existing `ci.yml` Linux/macOS/Windows unit jobs) | every core and client test above runs on all three; path, key-permission and `ssh` argv tests are platform-parametrised. FR-024. |
| User-guide check (`scripts/check-user-guide-updated.sh`, CI) | the milestones that change behaviour touch `docs/user-guide/sandboxed-daemon.md` / `remote-daemons.md`. FR-026. |
| Quickstart §B visual pass | form (image select, own-image note, checkboxes), row during start (step names, copy %, install output), migration and legacy notices. FR-001, FR-010, FR-017, FR-021. |

## Delivery order (for the tasks unit)

1. Basic image, hash script, publish workflow (its merge publishes the first image, R1).
2. Static Linux daemons for both architectures in every package (R7; 489 T082–T084).
3. Shared `provision` module; 489 moved onto it, `ssh_real_*` green (R5).
4. Container target: keys, sshd publish, `Reach::Pinned`, `Adoption::Legacy`, migration, pin of the
   digest published by step 1, existing `sandbox_real_*` suites moved onto the basic image (US1, US4).
5. AI CLI install, relay, daemon `offered.json` (US1 #2, US5 #1).
6. Form, row progress, user guide (US2, US3, FR-021, FR-026).
7. Bounds, cancellation, rollback, concurrent starts, `sandbox_real_provision.rs` in CI (US5, FR-025).

## Project Structure

### Documentation (this feature)

```text
specs/806-container-daemon-ssh-provisioning/
├── plan.md  research.md  data-model.md  quickstart.md
├── contracts/
│   ├── provisioning.md      # shared SSH provisioning (489, 806, 686)
│   ├── basic-image.md       # image contents, hash, publish workflow, pin check
│   └── daemon-settings.md   # settings shape, migration, form
└── tasks.md                 # tasks unit
```

### Source Code

```text
crates/micold-core/src/
├── provision/               # NEW: mod.rs failure.rs ai_cli.rs relay.rs keys.rs attempt.rs
├── ssh/{argv,provision,system}.rs
├── sandbox/{image,basic_image(NEW),mod,argv,lifecycle,exec}.rs
├── daemons.rs  settings.rs
crates/micold-core/tests/settings_released_image_migration.rs        # NEW
crates/micold-daemon/src/state.rs
crates/micold-daemon/tests/sandbox_real_provision.rs                 # NEW (sandbox-real-runtime)
crates/micold-client/src/{daemon.rs, shell/sandbox.rs, features/{connection,daemons}.rs,
                          ui/settings/{container_fields,daemons}.rs}
packaging/basic/{Containerfile,entrypoint.sh,sshd_config,README.md}  # NEW
scripts/{basic-image-hash.sh,remote-daemon-bundle.sh}                # NEW
scripts/tests/{basic-image-hash,remote-daemon-bundle}.test.sh        # NEW
.github/workflows/{basic-image.yml(NEW),ci.yml,release.yml}
docs/user-guide/{sandboxed-daemon,remote-daemons}.md
```

**Structure Decision**: the existing three crates; the shared mechanism is a core module beside
`ssh/` and `sandbox/`, so the client, the 489 runtime and 686 reach it through `micold_core`.

## Follow-ups outside this flow

- 686's spec, research K1/K2 and plan (`daemon_image/`, `DaemonImageSource`, build and push) are to be
  re-planned onto [contracts/provisioning.md](./contracts/provisioning.md) and `ImageChoice` by the
  686 flow.
- 489's tasks T082–T084 are delivered here (delivery step 2); the 489 flow marks them done.

## Complexity Tracking

None.
