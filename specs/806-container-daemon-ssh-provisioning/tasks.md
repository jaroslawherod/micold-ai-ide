---

description: "Tasks for container daemons provisioned over SSH from a basic image (806)"
---

# Tasks: Container daemons provisioned over SSH from a basic image

**Input**: Design documents from `specs/806-container-daemon-ssh-provisioning/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/](./contracts/), [quickstart.md](./quickstart.md)

**Tests**: Constitution Principle I (Test-First, non-negotiable). Every phase lists its failing tests
first; implement only after they fail for the right reason. Layers are the plan's *Test strategy*.

**Documentation**: Constitution Principle VII. Each milestone that changes behaviour updates
`docs/user-guide/sandboxed-daemon.md` and/or `docs/user-guide/remote-daemons.md` in the same PR
(`scripts/check-user-guide-updated.sh`).

**Cross-platform**: Constitution Principle VI. Core and client tests run on the Linux, macOS and
Windows CI matrix; path, key-permission and `ssh` argv tests are platform-parametrised (FR-024).
Real-runtime suites run on Linux CI only (as for 027 and 489).

**Order**: the plan's *Delivery order*, with these changes: the static daemon builds (plan step 2)
follow the shared module (step 3), because their `NoMatchingBuild` lives in its probe step; the AI
CLI install step of the shared module (step 5, core and daemon side) comes before the container
target (step 4), so that when containers move to the basic image on `main` they still carry AI CLIs;
and plan step 6 is split: row progress ships with the shared module (M2), the image form with the
container target (M5), the AI CLI checkboxes with the selection (M6). The shared provisioning
(`micold_core::provision`, [contracts/provisioning.md](./contracts/provisioning.md)) that feature
686 reuses ships in M2 (reach, probe, copy, start, version, keys, failures, bounds, cancel) and M4
(AI CLI install, relay) so 686 can build on it before the container work lands.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: the user story from spec.md (US1–US6); Setup and Foundational tasks carry none

---

## Phase 1: Setup — the basic image and its publication (FR-002)

**Purpose**: an image with sshd, Node and session tools, no daemon and no AI CLI, published by this
project on merge so a later milestone can pin its digest ([contracts/basic-image.md](./contracts/basic-image.md), R1).

### Tests (write first, must fail)

- [X] T001 [P] Write `scripts/tests/basic-image-hash.test.sh`: `scripts/basic-image-hash.sh` prints the same 16 lowercase hex twice in a row and after re-creating the files in another order; the value changes when one byte of a file under a temporary copy of `packaging/basic/` changes; `--check` prints `CURRENT` when the tree matches `BASIC_IMAGE_CONTENT_HASH` in `crates/micold-core/src/sandbox/basic_image.rs` and `DRIFT <pinned> <tree>` otherwise, exit 0 in both cases; and a Containerfile check that `packaging/basic/Containerfile` has no `COPY`/`ADD` of a key, credential, `.ssh`, `micold-daemon` or token file and no `npm install` (FR-002, FR-013)
- [X] T002 [P] Write unit tests in `crates/micold-core/src/sandbox/basic_image.rs` (`#[cfg(test)]`): `is_released_image` is true for `ghcr.io/jaroslawherod/micold-daemon` and for the retired-namespace repository (`RETIRED_IMAGE_REPOSITORY` in `sandbox/image.rs`) with no tag, any tag and any `@sha256:` digest; false for `ghcr.io/jaroslawherod/micold-basic:<tag>`, `micold-daemon:dev`, `micold-basic:dev` and `docker.io/library/alpine:3`; `BASIC_IMAGE_CONTENT_HASH` is exactly 16 lowercase hex characters
- [X] T003 [P] Write the real-runtime test `sandbox_real_basic_image_has_no_daemon_or_cli` in `crates/micold-core/tests/sandbox_real_basic_image.rs` (behind the `sandbox-real-runtime` feature, image tag from `MICOLD_BASIC_IMAGE_TAG`, default `micold-basic:dev`): `command -v` for `micold-daemon`, `claude`, `codex`, `gemini`, `opencode`, `pi` inside the image finds none; `command -v sshd node npm git flock` finds all; `glibc` version (`ldd --version`) ≥ 2.41

### Implementation

- [X] T004 Create `packaging/basic/Containerfile`, `packaging/basic/entrypoint.sh` and `packaging/basic/sshd_config` exactly as [contracts/basic-image.md](./contracts/basic-image.md) *Image* says: `FROM docker.io/library/node:24-trixie-slim`; the session packages of `packaging/sandbox/Containerfile` plus `openssh-server`, `openssh-client`, `git`, `ca-certificates`, `util-linux`; minus the daemon `COPY`, the AI CLI `ARG`/`npm install` block, `MICOLD_LISTEN_ADDR`, `MICOLD_TOKEN_PATH`, `EXPOSE 7727`; keep `XDG_DATA_HOME=/var/lib` and the Pi/OpenCode offline env; `EXPOSE 2222`; no `USER`; `/etc/passwd` and `/etc/group` writable by gid 0; the three labels; entrypoint adds the passwd/group line for `$(id -u)` with home `$MICOLD_HOME` when absent, starts `$HOME/.micold/remote/$(cat $HOME/.micold/remote/last)/micold-daemon` in the background when that file exists, then `exec /usr/sbin/sshd -D -e -f /etc/micold/sshd_config`; `sshd_config` with every directive the contract lists (`Port 2222` … `UsePAM no`)
- [X] T005 [P] Write `packaging/basic/README.md` for maintainers: what the image carries and must not carry, what a user image must provide, and the hash → publish → pin flow (T006, T008, M5's pin task)
- [X] T006 Write `scripts/basic-image-hash.sh` per the contract's *Content hash*: sha256 over `find packaging/basic -type f | LC_ALL=C sort` names and contents, first 16 hex; `--check` compares with `BASIC_IMAGE_CONTENT_HASH` grepped from `crates/micold-core/src/sandbox/basic_image.rs`
- [X] T007 Create `crates/micold-core/src/sandbox/basic_image.rs` (declared in `sandbox/mod.rs`) with `BASIC_IMAGE_REPOSITORY = "ghcr.io/jaroslawherod/micold-basic"`, `BASIC_IMAGE_CONTENT_HASH` (the value T006 prints for the T004 tree), `DEV_BASIC_IMAGE = "micold-basic:dev"`, `RELEASED_IMAGE_REPOSITORIES = [DEFAULT_IMAGE_REPOSITORY, RETIRED_IMAGE_REPOSITORY]` and `is_released_image(reference)` (repository match through the existing `ImageRef::parse`, any tag or digest). `BASIC_IMAGE` itself is added in T049, once the image is published
- [X] T008 Write `.github/workflows/basic-image.yml` per the contract's *Publication* table: `pull_request` touching `packaging/basic/**` builds `linux/amd64` and `linux/arm64` without pushing; `push` to `main` touching `packaging/basic/**` and `workflow_dispatch` compute the hash, stop with the existing digest when `ghcr.io/jaroslawherod/micold-basic:<hash>` exists, else build and push both arches by digest on native runners (as `release.yml` `image`), merge with `docker buildx imagetools create`, tag `<hash>`, and print `BASIC_IMAGE = "ghcr.io/jaroslawherod/micold-basic:<hash>@sha256:<index digest>"` in the job summary; permissions `contents: read`, `packages: write`; actions pinned by SHA like the other workflows
- [X] T009 In `.github/workflows/ci.yml` job `sandbox-runtime`, also build `micold-basic:dev` from `packaging/basic/` and run `sandbox_real_basic_image` (T003) beside the existing suites (`mise run image` also builds `micold-basic:dev`, so `mise run test-sandbox` runs it locally)

**Checkpoint (M1)**: on merge, `basic-image.yml` publishes `ghcr.io/jaroslawherod/micold-basic:<hash>` for both architectures and prints the `BASIC_IMAGE` line.

---

## Phase 2: Foundational — shared provisioning over SSH (FR-005–FR-008, FR-018, FR-019, FR-021; US6 for the 489 runtime)

**Purpose**: `micold_core::provision` with the sequence Reach → Probe → (Copy) → (InstallAiClis) →
Start → VersionCheck over a `ProvisionTarget`, the 489 remote runtime moved onto it unchanged, and
the row showing the shared step names ([contracts/provisioning.md](./contracts/provisioning.md), R5,
R11, R13). Feature 686 can adopt this phase by supplying `Reach::Pinned` and `Ownership`.

### Tests (write first, must fail)

- [x] T010 [P] Write argv tests in `crates/micold-core/src/ssh/argv.rs`: every existing builder given `&Reach::UserConfig(target)` produces today's argv byte for byte (the existing 489 assertions pass unchanged apart from the argument type); `Reach::Pinned(PinnedReach { host: "127.0.0.1", port, user, key, known_hosts })` emits `-F none`, `-o IdentitiesOnly=yes`, `-o IdentityAgent=none`, `-o UserKnownHostsFile=<known_hosts>`, `-o GlobalKnownHostsFile=none`, `-i <key>`, `-l <user>`, `-p <port>` and host `127.0.0.1`; `reach_check()` runs `true`; `install()` is the only builder without `ClearAllForwardings` and carries `-R 127.0.0.1:17728:127.0.0.1:<relay port>` only when given a relay port; platform-parametrised (`ssh.exe` on Windows)
- [x] T011 [P] Write tests in `crates/micold-core/src/provision/keys.rs`: `keygen_argv` is `ssh-keygen -t ed25519 -N "" -q -f <path>`; `DaemonKeys::ensure(dir, runner)` with a fake runner creates `client_ed25519`, `host_ed25519` and their `.pub` once and never overwrites existing ones on a second call; private keys are owner-only (mode `0600` on Unix, the `owner_only.rs` ACL on Windows); `authorized_keys` holds exactly the client public key; `known_hosts` is rewritten as `[127.0.0.1]:<port> <host public key>` when the port changes; two daemons' key dirs share no file (Principle II)
- [x] T012 [P] Write tests in `crates/micold-core/src/provision/failure.rs`: each `Step` label is the contract's text ("getting the image", "starting the container", "connecting to the SSH server", "checking the daemon build", "copying the daemon", "installing the AI CLIs", "starting the daemon", "checking the daemon version"); every `FailureCause` row of the contract's reason table produces exactly its cause text; `ProvisionFailure::reason()` is `"<step label> failed: <cause text>"` followed by the last 40 lines of `output` where the cause has output; `FailureCause::Ssh(f)` reuses the 489 `SshFailure` text unchanged; `ProvisionWarning` renders `"<CLI name> was not installed:"` plus the installer output
- [x] T013 [P] Write sequence tests in `crates/micold-core/src/provision/mod.rs` with a fake `SshHost` and a recording event sink: `Entered(step)` precedes each step, in contract order; `Copy` is skipped when the probe reports `have` and a second run with the build present copies nothing (invariant 1, US6 #2); a `UserHost` target never enters `InstallAiClis` and runs no install command (invariant 2, US6 #3); `Reach` retries every 500 ms only on connection refused / connection reset / `kex_exchange_identification`, any other failure is final (`AuthRefused` for a refused key), and only "not up yet" failures until the bound give `NoSshServer`; a step that exceeds its `StepBounds` entry fails with `TimedOut { bound }` naming that step; no other step is retried (invariant 4); a set `CancelToken` before or during a step gives `Cancelled`; a handshake refused for a version mismatch follows the 489 FR-011 rule (`VersionCheck`, `VersionMismatch { found, expected }`); `Start` writes `$HOME/.micold/remote/last` for `AppOwned` targets only; `CopyPercent` events reach the sink; every FR-018 cause the sequence can produce (`NoSshServer`, `AuthRefused`, `CopyFailed`, `NoMatchingBuild`, `DaemonStart`, `VersionMismatch`, `TimedOut`, `Cancelled`) is reported by its step through the fake host within that step's bound (SC-004 at the fake-host layer); `StepBounds::container()` is acquire 15 min, create-container / reach / probe / copy / start / version 120 s, install 600 s total; `AiCliSelection::new_daemon()` is `{ClaudeCode}` and `AiCliSelection::all()` is `AiCli::ALL`
- [x] T014 [P] Write tests in `crates/micold-core/src/provision/attempt.rs` (`CancelToken`: `cancel()` wakes every waiter, is idempotent, `is_cancelled()` reflects it) and in `crates/micold-core/src/ssh/system.rs` (a child started through `SystemSsh` with a token is killed when the token is cancelled; Unix-only `sleep` child, `cfg`-gated, with a Windows equivalent using `timeout`)
- [x] T015 [P] Write a test in `crates/micold-core/src/ssh/provision.rs` that `provision()` and `connect()` run the shared sequence with a `UserHost` target and the 489 bounds (reach, probe, start = the connect deadline; copy unbounded), so the existing 489 unit tests there and in `crates/micold-client` stay green unchanged
- [x] T016 [P] Write client tests in `crates/micold-client/src/features/daemons.rs` (`#[cfg(test)]`): `ConnectionMsg::Provisioning(ProvisionEvent::Entered(step))` sets the daemon's current step, `CopyPercent(n)` its percent, for that daemon only; a start failure sets `unreachable` with `ProvisionFailure::reason()` and appends the reason and output to that daemon's recent errors; a second daemon's state is untouched (FR-022)

### Implementation

- [x] T017 Add `Reach`, `PinnedReach`, `command(&Reach)`, `install()` and `reach_check()` in `crates/micold-core/src/ssh/argv.rs`; move every caller onto `&Reach::UserConfig` (find them with LSP `findReferences` on `command`)
- [x] T018 [P] Create `crates/micold-core/src/provision/keys.rs`: `DaemonKeys { dir }` with the paths of [data-model.md](./data-model.md) *DaemonKeys*, `keygen_argv`, `ensure` (creates once, never overwrites; owner-only through `owner_only.rs`; writes `authorized_keys`), `write_known_hosts(port)`
- [x] T019 [P] Create `crates/micold-core/src/provision/failure.rs`: `Step` with `label()`, `FailureCause` (every variant of data-model), `ProvisionFailure { step, cause, output }` with `reason()`, `ProvisionWarning { cli, output }`
- [x] T020 [P] Create `crates/micold-core/src/provision/attempt.rs` with `CancelToken(Arc<(AtomicBool, Notify)>)`; in `crates/micold-core/src/ssh/system.rs` take an optional token and kill the child on cancel (`AttemptLedger` comes in T079)
- [x] T021 Create `crates/micold-core/src/provision/mod.rs` (declared in `crates/micold-core/src/lib.rs`): `ProvisionTarget { reach, owned, bounds }`, `Ownership::{AppOwned { ai_clis: AiCliSelection, table: AiCliTable, state_dir, egress }, UserHost}` (`AiCliSelection(BTreeSet<AiCli>)` with `new_daemon()` and `all()` is created here in `crates/micold-core/src/sandbox/mod.rs`; its profile field and serde default come in T077; `AiCliTable` is a placeholder type filled in T038), `Egress::{Direct, Relay { allow }}`, `StepBounds` with `StepBounds::container()` (acquire 15 min; create-container, reach, probe, copy, start, version 120 s; install 600 s total) and `StepBounds::user_host(connect_deadline)` (copy `None`), `ProvisionEvent`, `Provisioned { connected, warnings, copied }` and `run(host, target, options, cancel, sink)` implementing the contract's sequence; the `InstallAiClis` step is a no-op for `AppOwned` until T035 (it is never entered for `UserHost`)
- [x] T022 Make `provision()` and `connect()` in `crates/micold-core/src/ssh/provision.rs` thin wrappers over `provision::run` with the `UserHost` target; keep `SshRunner`, `Artefacts`, `SshHost` and `Probe` there
- [x] T023 Replace `ConnectionMsg::CopyProgress(u8)` with `ConnectionMsg::Provisioning(ProvisionEvent)` in `crates/micold-client/src/features/connection.rs`, `crates/micold-client/src/shell/connection.rs`, `crates/micold-client/src/daemon.rs` (the copy-progress sender) and `crates/micold-client/src/features/daemons.rs` (`Msg::CopyProgress` → per-daemon step and percent; failure → `unreachable` reason and recent errors)
- [x] T024 Draw the current step label and copy percentage in the daemon's row in `crates/micold-client/src/ui/settings/daemons.rs` with the existing `progress.rs` indicator (no new widget)
- [x] T025 Update `docs/user-guide/remote-daemons.md`: the start steps a remote daemon's row now shows and the reason texts it reports (FR-026)

**Checkpoint (M2)**: a 489 remote daemon starts through `micold_core::provision`; its row shows the shared step labels and copy percentage; the CI `ssh-remote` job (`ssh_real_*`) is green unchanged.

---

## Phase 3: Foundational — Linux daemon builds for both container architectures (FR-009; 489 T084–T086)

**Purpose**: every client can supply a daemon that runs in an `x86_64` or `aarch64` Linux container (R7).

### Tests (write first, must fail)

- [x] T026 [P] Write `scripts/tests/remote-daemon-bundle.test.sh` (489 T084, two architectures): `scripts/remote-daemon-bundle.sh --dry-run` names the targets `x86_64-unknown-linux-musl` and `aarch64-unknown-linux-musl` and the outputs `micold-daemon-linux-x86_64` and `micold-daemon-linux-aarch64`; the staging of `scripts/macos-bundle.sh` and `scripts/windows-installer.sh` places both in the bundled dir `SystemSsh::bundled_dir` reads; the `[package.metadata.deb]` assets in `crates/micold-client/Cargo.toml` and the `rpm` job of `.github/workflows/release.yml` carry them; the script fails with a clear message when `cargo-zigbuild` or `zig` is missing
- [x] T027 [P] Write unit tests in `crates/micold-core/src/ssh/system.rs`: for a probed `linux`/`x86_64` or `linux`/`aarch64` target, the client's own build is chosen when client OS and arch match, else `micold-daemon-linux-<arch>` from `bundled_dir`; with neither, the probe step fails with `NoMatchingBuild { os, arch }` whose text is `no matching daemon build for linux-aarch64` (FR-009)

### Implementation

- [x] T028 Write `scripts/remote-daemon-bundle.sh` (489 T085): `cargo zigbuild --release -p micold-daemon` for both musl targets, statically linked, output `micold-daemon-linux-x86_64` / `micold-daemon-linux-aarch64` into a given directory; `--dry-run` prints the plan
- [x] T029 Include the two builds from `scripts/macos-bundle.sh`, `scripts/windows-installer.sh`, the `[package.metadata.deb]` assets of `crates/micold-client/Cargo.toml` (the build for the other architecture), and build them once in a `linux-daemons` job of `.github/workflows/ci.yml` and of `release.yml` (install `cargo-zigbuild` and zig there); the CI macOS/Windows jobs and the release macOS/Windows jobs download that artifact, the `deb`/`rpm` jobs build in place (zig crashes on the Windows ARM64 runner, so no packaging host cross-links)
- [x] T030 Map the probed `uname -m` to the bundled file names and return `NoMatchingBuild` in `crates/micold-core/src/ssh/system.rs` (`bundled`) and the probe step of `crates/micold-core/src/provision/mod.rs`
- [x] T031 Document per-platform requirements (`ssh.exe`, agent service, supported remote OS) and the bundled Linux builds for both architectures in `docs/user-guide/remote-daemons.md` (489 T086)

**Checkpoint (M3)**: `scripts/tests/remote-daemon-bundle.test.sh` passes; a macOS or Windows client starts a remote daemon on a Linux host of either architecture.

---

## Phase 4: Foundational — AI CLI install on app-owned targets (FR-011, FR-012, FR-013; R6, R8)

**Purpose**: the `InstallAiClis` step of the shared sequence, its no-outbound relay and the daemon
offering only installed CLIs. Used by containers in M5 and by 686 pods; nothing reaches it from the
UI yet (no `AppOwned` target exists on `main` until M5).

### Tests (write first, must fail)

- [x] T032 [P] Write tests in `crates/micold-core/src/provision/ai_cli.rs`: the pinned table has one row per `AiCli::ALL` with the package and version moved from the `ARG`s of `packaging/sandbox/Containerfile`, and `allow` = `registry.npmjs.org:443`; the install script for a set of missing CLIs contains, per CLI, `npm install -g --prefix <state>/ai-clis/<cli>@<version>.part <package>@<version>`, the `mv` to `<state>/ai-clis/<cli>@<version>`, the `rm -rf` of `.part` and `MICOLD_INSTALL <cli> failed` on failure, holds `flock` on `<state>/ai-clis/.lock`, writes `<state>/ai-clis/offered.json` atomically (selected ∩ installed, shape of data-model *Daemon-side file*), prints "the image has no npm (Node.js), which this AI CLI's installer needs" per CLI when `npm` is missing, exports `HTTPS_PROXY` and `npm_config_https_proxy` = `http://127.0.0.1:17728` only for `Egress::Relay`, and contains no text from user input; the script never names a credential path (FR-013)
- [x] T033 Write tests in `crates/micold-core/src/provision/ai_cli.rs` that `AiCliTable::pinned()` is the only table production code builds (the client passes it; no setting or user text can change a row), and that a table whose row points `<package>` at a local tarball path yields `npm install -g --prefix … <tarball>` with no version suffix and an empty `allow`, so tests install a stand-in with no network (FR-025)
- [x] T034 [P] Write tests in `crates/micold-core/src/provision/relay.rs`: `CONNECT registry.npmjs.org:443` → `200` and bytes relayed both ways to a local fake upstream; `CONNECT other.example:443` → `403`; a `GET` → refused and closed; the relay stops when its install command ends; a relay that cannot bind gives `PostureBlocksDownload` on `InstallAiClis`
- [x] T035 [P] Extend the sequence tests in `crates/micold-core/src/provision/mod.rs`: an `AppOwned` target with missing selected CLIs enters `InstallAiClis` after Copy and before Start, emits `InstallLine` per output line and `InstallDone(cli, result)` per CLI; a failed CLI becomes a `ProvisionWarning` and the start still succeeds (FR-012); nothing missing → step skipped and nothing downloaded (invariant 1); the install exceeding 600 s → `TimedOut` on `InstallAiClis`; `Egress::Relay` builds the command with `install()` and the relay port
- [x] T036 [P] Write tests in `crates/micold-daemon/src/state.rs`: with `<state>/ai-clis/offered.json` present, `spawn_path_for` prepends its bin dirs and `ai_clis_available_in` offers exactly the listed CLIs; file missing → today's behaviour; malformed → ignored and logged; a CLI missing from the file is not offered (FR-012)

### Implementation

- [x] T037 [P] Create the stand-in AI CLI package for real-runtime tests in `crates/micold-daemon/tests/sandbox_real_support/standin-cli/` (`package.json` with a `bin` running the same script `fake_ai_cli` in `crates/micold-daemon/tests/sandbox_real_support/mod.rs` writes, plus a variant whose `preinstall` exits 1) and a harness helper that `npm pack`s it into the daemon's state dir and returns an `AiCliTable` mapping every `AiCli` to the tarball (or the failing one for named CLIs)
- [x] T038 Create `crates/micold-core/src/provision/ai_cli.rs`: `AiCliTable` with `pinned()` (the pinned table) and the tarball row form of T033, the install script builder and the `offered.json` writer per [contracts/provisioning.md](./contracts/provisioning.md) *Install script*
- [x] T039 Create `crates/micold-core/src/provision/relay.rs`: tokio HTTP `CONNECT` relay on client loopback, allowlist from the table, one install command's lifetime
- [x] T040 Implement the `InstallAiClis` step in `crates/micold-core/src/provision/mod.rs`: list missing CLIs from the probe, run the script with `install()` (relay when `Egress::Relay`), parse `MICOLD_INSTALL` lines into warnings, bound 600 s
- [x] T041 Read `offered.json` in `spawn_path_for` / `ai_clis_available_in` in `crates/micold-daemon/src/state.rs` (no protocol change)

**Checkpoint (M4)**: `provision::run` on an `AppOwned` target installs the selected CLIs (through the relay under `Egress::Relay`) and the daemon offers only those installed; feature 686 can now adopt the whole contract.

---

## Phase 5: User Story 1 — A new container daemon runs the client's own version (Priority: P1) 🎯 MVP

**Goal**: a container daemon starts from the basic image, is provisioned over SSH and connects with
the client's exact version; it is reused on reopen.

**Independent Test**: with no `micold-daemon:*` image present, add a container daemon with default
settings, start it, and check it connects with the client's version (quickstart §B 1–2).

### Tests (write first, must fail)

- [x] T042 [P] [US1] Write tests in `crates/micold-core/src/sandbox/image.rs`: `ImageChoice::default()` is `Basic`; `ImageChoice::Basic.reference()` is `BASIC_IMAGE`; `ImageRef::parse(BASIC_IMAGE).digest.is_some()`; `OwnImageDraft { reference: "", archive: None }` fails validation naming the image field; an unparsable reference fails with `ImageRef::parse`'s message; a reference for which `is_released_image` holds fails with "The released daemon image is no longer used; choose the basic image." (FR-001, FR-003, FR-004)
- [x] T043 [P] [US1] Write tests in `crates/micold-core/src/sandbox/mod.rs` and `crates/micold-core/src/sandbox/argv.rs`: for `Basic` and `Own`, `SandboxSpec` publishes `127.0.0.1:<port>` → container `2222` only; mounts the key dir's `host_ed25519`, `host_ed25519.pub` and `authorized_keys` read-only at `/run/micold/ssh`; sets `MICOLD_HOME`; has no token mount and no `MICOLD_LISTEN_ADDR`; the credential mounts are exactly the 027 shares the user allowed and nothing else (FR-013); the create argv carries the matching flags
- [x] T044 [P] [US1] Write `adopt` tests in `crates/micold-core/src/sandbox/lifecycle.rs`: a running container whose image repository is released → `Adoption::Legacy(id)`; a stopped one → `Replace`; another image difference → `Replace`; matching image → `Attach` / `Start`; `StaleReason::StaleBuild` is never produced; `Stage` maps to `Step::Acquire` and `Step::CreateContainer`
- [x] T045 [P] [US1] Write client tests in `crates/micold-client/src/shell/sandbox.rs` (`#[cfg(test)]`, fake runtime and fake `SshHost`): `start` ensures `DaemonKeys` in `<state>/ssh` before bring-up, provisions with `Reach::Pinned { 127.0.0.1, stored port, … }`, `Ownership::AppOwned { state_dir: "/var/lib/micold-ai-ide", egress }` with `egress = Relay` for `NoOutbound` and `Direct` otherwise, `StepBounds::container()`; emits `Entered(Acquire)` and `Entered(CreateContainer)` into the same sink; `Adoption::Legacy` dials `Dial::Container` (TCP+token), every other adoption `Dial::ContainerSsh`
- [x] T046 [P] [US1] Write the geometry gate `crates/micold-client/tests/gates/container_section_fits.rs`: the Container section with the two-entry image select and the own-image reference field, archive picker and note keeps its fields aligned and unclipped at the narrowest Settings width (follow an existing gate in `crates/micold-client/tests/gates/`)
- [x] T047 [US1] Move the real-runtime harness onto the basic image: `crates/micold-daemon/tests/sandbox_real_support/` and the `crates/micold-core/tests/sandbox_real_*.rs` suites build on `micold-basic:dev` (`MICOLD_BASIC_IMAGE_TAG`) and start through provisioning; rewrite `crates/micold-daemon/tests/sandbox_real_fingerprint.rs` and `sandbox_real_staleness.rs` to the handshake version rule (R10); `sandbox_real_ai_cli.rs` and `sandbox_real_ai_cli_sessions.rs`, which relied on CLIs baked into the old image, get them through the install step with the stand-in table (T037); `sandbox_real_limits.rs` is T065's
- [x] T048 [US1] Write `crates/micold-daemon/tests/sandbox_real_provision.rs` (feature `sandbox-real-runtime`): fresh start reports the client's exact version (SC-001); a second start with the container running copies and downloads nothing and connects in < 15 s (SC-003, US1 #4); a session starts with the stand-in AI CLI installed by the install step from the stand-in table (T037), no network (FR-025); after `docker rm -f` of the container the next start recreates it, copies nothing and installs nothing, because the build and the CLIs live in the host-mounted state dir (FR-011, edge case "Container recreated")

### Implementation

- [x] T049 [US1] Add `BASIC_IMAGE` to `crates/micold-core/src/sandbox/basic_image.rs`: the exact line printed in the job summary of the `basic-image.yml` run on `main` after M1 merged (read it with the GitHub MCP actions tools); `scripts/basic-image-hash.sh --check` prints `CURRENT`
- [x] T050 [US1] Add `ImageChoice`, `OwnImage` and `OwnImageDraft` (with validation) in `crates/micold-core/src/sandbox/image.rs`; replace `SandboxProfile.image: ImageSource` with `ImageChoice` in `crates/micold-core/src/sandbox/mod.rs` (serde reads old `image` through `ImageSource::into_choice`, added here in `sandbox/image.rs`, and writes `image_choice`); remove `DEFAULT_IMAGE` and `DEV_IMAGE_TAG` from every default (keep `DEFAULT_IMAGE_REPOSITORY` for `is_released_image` and the `release.yml` grep) (FR-003)
- [x] T051 [US1] Change `SandboxSpec` in `crates/micold-core/src/sandbox/mod.rs` and the create arguments in `crates/micold-core/src/sandbox/argv.rs` per data-model *SandboxSpec* (publish 2222, read-only `/run/micold/ssh` mount, `MICOLD_HOME`, no token mount for basic/own)
- [x] T052 [US1] Add `Adoption::Legacy`, drop the fingerprint replace and map `Stage` onto `Step` in `crates/micold-core/src/sandbox/lifecycle.rs`
- [x] T053 [US1] Add `Dial::ContainerSsh { reach, target }` calling `provision::run` in `crates/micold-client/src/daemon.rs`; keep `Dial::Container` for `Adoption::Legacy` only
- [x] T054 [US1] In `crates/micold-client/src/shell/sandbox.rs` `start`: `DaemonKeys::ensure`, write `known_hosts` for the stored port, bring the container up, provision; until M6 the target's `ai_clis` is `AiCliSelection::all()`, so container daemons keep every CLI they had; the table is `AiCliTable::pinned()`
- [x] T055 [US1] `mise.toml`: `image` builds `micold-basic:dev` from `packaging/basic/` (no daemon build), `test-sandbox` checks for `micold-basic:dev`; `.github/workflows/ci.yml` `sandbox-runtime` runs the suites against it and adds the pin check `docker manifest inspect "$(grep -o 'ghcr.io/jaroslawherod/micold-basic:[^"]*' crates/micold-core/src/sandbox/basic_image.rs)"`

**Checkpoint**: US1 #1, #3, #4 hold; container daemons connect on the basic image.

---

## Phase 6: User Story 4 — Existing container daemons keep working after the upgrade (Priority: P1)

**Goal**: stored daemons on the released image become basic-image daemons at load; a running legacy
container with sessions is attached, not replaced (R9, R10).

**Independent Test**: load pre-upgrade settings with a daemon on `ghcr.io/jaroslawherod/micold-daemon:0.9.0`, start it, check it runs the basic image with every other setting kept (quickstart §B 6–7).

### Tests (write first, must fail)

- [x] T056 [P] [US4] Write `crates/micold-core/tests/settings_released_image_migration.rs`: one test per *image* row of the read-rules table in [contracts/daemon-settings.md](./contracts/daemon-settings.md) (no image → `basic`, no notice; released repository any tag/digest → `basic` + `moved_off_released_image`; other registry ref → `own` same ref; `imported_file` → `own` same path; `local_build` → `own` same ref; `image_choice` present → as stored, `image` ignored); name, port, budget, network, credentials, survive_logout and runtime unchanged (FR-016, SC-005); `migrated_entry` follows the same rules; the written file has `image_choice` and `migration` (omitted when `None`) and never `image`
- [x] T057 [P] [US4] Extend `crates/micold-core/tests/settings_retired_image_namespace.rs` to assert the final `ImageChoice::Basic` and `MigrationNotice::MovedOffReleasedImage` after `repair_retired_namespace` then `into_choice`
- [x] T058 [P] [US4] Write client tests in `crates/micold-client/src/features/daemons.rs` and `crates/micold-client/src/ui/settings/daemons.rs`: a stored `MovedOffReleasedImage` shows "Moved off the released daemon image; it moves to the basic image on its next start."; it is cleared and saved after the first `Started` on `Basic`; a `Legacy` adoption shows "Still running on the released daemon image; restart it with no sessions running to move it to the basic image." with restart enabled only when the daemon reports zero running sessions (FR-017)
- [x] T059 [US4] Add to `crates/micold-daemon/tests/sandbox_real_provision.rs`: settings with a released-image reference start on the basic image with the client's version and the notice cleared (US4 #1); a running container of a released `ghcr.io/jaroslawherod/micold-daemon` tag with a session is attached (`Adoption::Legacy`), not recreated, its session keeps running; after the session ends, a restart replaces it with a basic-image container reporting the client's version (US4 #3, #4, FR-017)

### Implementation

- [x] T060 [US4] Add `MigrationNotice` and `ContainerSettings.migration` (`#[serde(default)]`, omitted when `None`) in `crates/micold-core/src/daemons.rs`; apply `ImageSource::into_choice` in `migrated_entry` and the registry read
- [x] T061 [US4] Call `ImageSource::into_choice` (from T050) in the read-repair of `crates/micold-core/src/settings.rs` right after `repair_retired_namespace()`
- [x] T062 [US4] Show the migration and legacy notices and gate the legacy restart in `crates/micold-client/src/ui/settings/daemons.rs` and `crates/micold-client/src/features/daemons.rs`; clear `migration` and save on success in `crates/micold-client/src/shell/sandbox.rs`

**Checkpoint**: US4 #1–#5 hold.

---

## Phase 7: User Story 2 — Choose the daemon's resources (Priority: P1)

**Goal**: resources stay configurable and are applied on the basic and own images (FR-014, FR-015).
No new code beyond keeping them wired; the tests prove nothing regressed.

### Regression tests (characterisation: written before T051 changes `SandboxSpec`, so they guard it; they are expected green once it compiles, not red)

- [x] T063 [US2] In `crates/micold-core/src/sandbox/mod.rs` tests: `ResourceBudget` flags are applied to `SandboxSpec` for both `ImageChoice::Basic` and `ImageChoice::Own` (FR-015)
- [x] T064 [US2] In `crates/micold-client/src/ui/settings/container_fields.rs` tests: the four limit fields keep their minimums, errors beside the field, "not enforced by this runtime" notices and the restart confirmation for a running daemon, for both image choices (US2 #2–#4, FR-014)
- [x] T065 [US2] Move `crates/micold-daemon/tests/sandbox_real_limits.rs` onto the basic image (CPU 2, memory 4 GiB reported by `inspect`; changed limit applies on restart) (US2 #1, SC-006)

**Checkpoint**: US2 #1–#4 hold on the basic image.

---

## Phase 8: User Story 3 — Use my own image (Priority: P2)

**Goal**: "My own image" by reference or archive, provisioned like the basic image (FR-001, FR-004).

### Tests (write first, must fail)

- [x] T066 [P] [US3] Write form tests in `crates/micold-client/src/ui/settings/container_fields.rs`: the image select has exactly "Basic image (recommended)" and "My own image" (US4 #5: no entry names the released image); "My own image" shows the reference field, the archive picker and "Your image must run an SSH server; see the user guide."; saving it empty is refused naming the field (US3 #4); a released-image reference is refused with the FR-003 message
- [x] T067 [US3] Add own-image cases to `crates/micold-daemon/tests/sandbox_real_provision.rs`: `micold-basic:dev` as own image → connected with the client's version (US3 #1); `docker.io/library/alpine:3` → fails in `Reach` with `NoSshServer`, row `unreachable` (US3 #2); a nonexistent reference → `PullOrLoad` naming the image with the runtime's output (US3 #3)

### Implementation

- [x] T068 [US3] Replace the three `IMAGE_SOURCES` with the two-entry `Select` in `crates/micold-client/src/ui/settings/container_fields.rs`, reusing the existing reference `TextField` and archive picker for "My own image", plus the SSH-server note
- [x] T069 [US3] Write `docs/user-guide/sandboxed-daemon.md`: the image choice, what an own image must provide (sshd on port 2222 accepting `/run/micold/ssh/authorized_keys` with host key `/run/micold/ssh/host_ed25519_key`, `sh uname mkdir cat mv chmod`, `npm` and `flock` for AI CLIs; simplest `FROM ghcr.io/jaroslawherod/micold-basic:<tag>`), the resources, the migration and legacy notices; no mention of the released image as an option (FR-026)
- [x] T070 [US3] Visual pass (`visual-pass` skill) for quickstart §B steps 1 (without the AI CLI boxes), 3, 4, 6 and 7; save the evidence under `specs/806-container-daemon-ssh-provisioning/visual/`; time step 1 from "add" to a session running, excluding the pull, and record it against SC-002 (< 5 min)

**Checkpoint (M5)**: US1 #1/#3/#4, US2, US3 and US4 hold; no form offers the released image.

---

## Phase 9: User Story 1 #2 and User Story 5 #1 — per-daemon AI CLI selection (FR-010, FR-012)

**Goal**: each container daemon has its own AI CLI selection; only selected and installed CLIs are
offered; a failed install is a warning in the row (R6).

**Independent Test**: a daemon with Claude Code selected and Codex not selected offers Claude Code
only; a blocked Codex install shows npm's output and the daemon still connects (quickstart §B 1, 5).

### Tests (write first, must fail)

- [X] T071 [P] [US1] Write tests in `crates/micold-core/src/sandbox/mod.rs`: a `SandboxProfile` deserialised without `ai_clis` has `all()` (D3); an empty selection round-trips; extend `crates/micold-core/tests/settings_released_image_migration.rs` with the `ai_clis` rows of the read-rules table
- [X] T072 [P] [US1] Write form tests in `crates/micold-client/src/ui/settings/container_fields.rs`: one checkbox per `AiCli::ALL`; a newly added daemon has only Claude Code ticked; the "changes apply at the next start" note; an empty selection saves; a change on a running daemon goes through the restart confirmation; the section is not shown for a remote SSH daemon (US6 #3)
- [X] T073 [P] [US5] Write client tests in `crates/micold-client/src/features/daemons.rs`: `InstallLine` events show in the row; `ProvisionWarning`s from `Provisioned.warnings` are listed in the row and in recent errors naming the CLI with the installer output (US5 #1, FR-012)
- [X] T074 [P] [US1] Extend `crates/micold-client/tests/unavailable_default_says_so.rs`: a stored default AI CLI that is deselected, or whose install failed, starts nothing, substitutes nothing and says it is not available on that daemon (D4)
- [ ] T075 [US5] Add to `crates/micold-daemon/tests/sandbox_real_provision.rs`: a stand-in CLI whose install fails → warning with its output, daemon connected, that CLI not offered, the other offered (US5 #1); deselecting a CLI stops it being offered after restart (US1 #2); under the `NoOutbound` posture the install goes through the relay and completes, and with the relay forced to fail it fails naming the posture (`PostureBlocksDownload`) within its bound (edge case "No-outbound network posture")
- [X] T076 [P] [US1] Extend `crates/micold-client/tests/gates/container_section_fits.rs` with the five AI CLI checkboxes

### Implementation

- [X] T077 [US1] Add `SandboxProfile.ai_clis: AiCliSelection` (serde default `all()`) in `crates/micold-core/src/sandbox/mod.rs`; the add-daemon path sets `new_daemon()`; `crates/micold-client/src/shell/sandbox.rs` passes the stored selection into `Ownership::AppOwned`
- [ ] T078 [US1] Add the AI CLI `Checkbox` list and note in `crates/micold-client/src/ui/settings/container_fields.rs`; show install lines and warnings in `crates/micold-client/src/ui/settings/daemons.rs` and recent errors in `crates/micold-client/src/features/daemons.rs`; document the AI CLI selection, the empty selection and the install under no-outbound in `docs/user-guide/sandboxed-daemon.md` (FR-026); visual pass for quickstart §B 1 and 5, evidence under `specs/806-container-daemon-ssh-provisioning/visual/`

**Checkpoint (M6)**: US1 #2 and US5 #1 hold.

---

## Phase 10: User Story 5 — Failures, bounds, cancellation, rollback, concurrency (FR-018–FR-020, FR-022, FR-023, FR-025)

**Goal**: every container start step is bounded and cancellable, a failed or stopped start leaves
nothing behind, concurrent starts give one container, and the end-to-end suite covers it in CI (R11,
R12, R14).

### Tests (write first, must fail)

- [ ] T079 [P] [US5] Write `AttemptLedger` tests in `crates/micold-core/src/provision/attempt.rs`: rollback removes a container created in this attempt and stops one whose daemon this attempt started; never touches an adopted container or a daemon found running; host state dir untouched (FR-020)
- [ ] T080 [P] [US5] Write tests in `crates/micold-core/src/sandbox/exec.rs` and `crates/micold-core/src/sandbox/lifecycle.rs`: `acquire_image` fails with `TimedOut { 15 min }` on `Acquire` (fake clock / injected bound); a cancel kills the runtime child; `bring_up` records into the ledger; a `create` that fails with a name conflict re-inspects and adopts the winner once (`Attach`), never retries further (FR-023)
- [ ] T081 [P] [US5] Write client tests in `crates/micold-client/src/shell/sandbox.rs`: `InFlight::cancel()` sets the `CancelToken`, the start ends `stopped` with rollback; a failed start rolls back and shows `unreachable` with the reason in recent errors (US5 #3)
- [ ] T082 [US5] Add to `crates/micold-daemon/tests/sandbox_real_provision.rs`: stop during a long install leaves no container or daemon process (FR-020); two concurrent starts of one daemon give one container and one daemon (FR-023); add to `crates/micold-daemon/tests/sandbox_real_multi_daemon_container.rs` a case where one daemon is provisioning or fails provisioning while another stays connected and responsive (FR-022)

### Implementation

- [ ] T083 [US5] Add `AttemptLedger` and `rollback` in `crates/micold-core/src/provision/attempt.rs`
- [ ] T084 [US5] Bound `acquire_image` at 15 min and kill on cancel in `crates/micold-core/src/sandbox/exec.rs`; `bring_up` takes the `CancelToken`, records into the ledger and re-adopts on a create conflict in `crates/micold-core/src/sandbox/lifecycle.rs`
- [ ] T085 [US5] Carry the `CancelToken` in `InFlight` and roll back on failure or cancel in `crates/micold-client/src/shell/sandbox.rs`
- [ ] T086 [US5] Make sure `.github/workflows/ci.yml` `sandbox-runtime` runs `sandbox_real_provision` and `ssh-remote` runs `ssh_real_*`; document the real-runtime task in `docs/development/` where the sandbox suite is documented (FR-025); note in `docs/user-guide/sandboxed-daemon.md` the step bounds and that stopping a start leaves nothing behind

**Checkpoint (M7)**: US5 #2–#3 hold; SC-004 in CI.

---

## Phase 11: User Story 6 — One provisioning for every runtime (Priority: P2)

US6 is delivered by Phase 2 (489 on the shared sequence, US6 #2, #3) and Phase 5 (containers on the
same sequence). Its cross-runtime check:

- [x] T087 [US6] Write a core test in `crates/micold-core/src/provision/mod.rs` that runs the sequence for a `UserHost` and an `AppOwned` target over the same fake host failing the copy, and asserts identical step labels, `CopyPercent` events and `CopyFailed` reason text (US6 #1, invariant 3); it needs only Phase 2 types and ships in M2's PR

---

## Phase 12: Polish

- [ ] T088 Run quickstart Part A in full and §B step 8 (remote daemon: same labels, no install, no AI CLI section); record results in `specs/806-container-daemon-ssh-provisioning/quickstart.md`
- [ ] T089 [P] Describe `micold_core::provision` and the basic image in the architecture doc under `docs/` that describes the sandbox and the remote runtime

---

## Dependencies & Execution Order

- Phase 1 (M1) has no dependency. Its merge publishes the image whose digest T049 pins.
- Phase 2 (M2) depends on nothing in Phase 1; it is ordered after M1 only so one PR merges at a time.
- Phase 3 (M3) depends on Phase 2 (`NoMatchingBuild`, probe step).
- Phase 4 (M4) depends on Phase 2.
- Phases 5–8 (M5) depend on M1 (published image), M2, M3 (builds for container architectures) and M4 (CLIs installed).
- Phase 9 (M6) depends on M5. Phase 10 (M7) depends on M5 (and M6 for the install-cancel case); T087 (Phase 11) ships in M2.
- Phase 12 is the close unit's.

Within each phase: tests → types → behaviour → client → docs. Tests marked [P] in a phase touch
different files and can be written together.

### Parallel examples

- M2: T010, T011, T012, T013, T014, T015, T016 together; then T018, T019, T020 together.
- M4: T032, T034–T037 together, T033 after T032 (same file); T038 and T039 together.
- M5: T042–T046 and T056–T058 together; T063 after T043, T064 and T066 one after the other (same files).

## Implementation Strategy

Ship the shared pieces first (M1 image, M2 provisioning, M3 builds, M4 install) so feature 686 can
plan onto them, then switch containers in one step (M5) with migration and legacy handling, so `main`
never has container daemons that lose their AI CLIs or have running sessions ended. M6 adds the
per-daemon selection, M7 the bounds, rollback and the end-to-end suite.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Basic image published 🎯 MVP

- **Tasks**: T001–T009
- **Deliverable**: merging publishes `ghcr.io/jaroslawherod/micold-basic:<hash>` for `linux/amd64` and `linux/arm64`, carrying sshd, Node and session tools and no daemon or AI CLI; the run's summary prints the `BASIC_IMAGE` pin.
- **Satisfies**: FR-002; FR-013 (image carries no credential)
- **Verify**: `bash scripts/tests/basic-image-hash.test.sh`; `scripts/basic-image-hash.sh --check` → `CURRENT`; CI `sandbox-runtime` runs `sandbox_real_basic_image`; after merge, the `basic-image.yml` run summary on `main`
- **Depends on**: —
- **Tier**: full

### M2 — Shared provisioning over SSH, 489 moved onto it

- **Tasks**: T010–T025, T087
- **Deliverable**: `micold_core::provision` implements [contracts/provisioning.md](./contracts/provisioning.md) (reach, probe, copy, start, version check, keys, reason texts, bounds, cancel); a remote SSH daemon starts through it and its row shows the shared step labels and copy percentage.
- **Satisfies**: US6 #1 (fake host), #2, #3; FR-005, FR-006 (keys, pinned reach), FR-007, FR-008, FR-018 (reason texts), FR-019 (sequence bounds), FR-021 (row progress), FR-022
- **Verify**: `cargo test -p micold-core provision:: ssh::`; `cargo test -p micold-client daemons`; CI `ssh-remote` (`ssh_real_*`) green
- **Depends on**: —
- **Tier**: full

### M3 — Linux daemon builds for both container architectures

- **Tasks**: T026–T031
- **Deliverable**: macOS, Windows and Linux packages carry static `micold-daemon-linux-x86_64` and `-linux-aarch64`; a client with no matching build reports `no matching daemon build for linux-<arch>`.
- **Satisfies**: FR-009; 489 T084–T086
- **Verify**: `bash scripts/tests/remote-daemon-bundle.test.sh`; `cargo test -p micold-core ssh::system`
- **Depends on**: M2
- **Tier**: full

### M4 — AI CLI install step in the shared provisioning

- **Tasks**: T032–T041
- **Deliverable**: `provision::run` on an app-owned target installs the selected AI CLIs into the state dir (through the registry-only relay under no-outbound) and the daemon offers only the installed ones; feature 686 can adopt the full contract.
- **Satisfies**: FR-011, FR-012 (daemon side, warnings), FR-013 (install script)
- **Verify**: `cargo test -p micold-core provision::`; `cargo test -p micold-daemon state::`
- **Depends on**: M2
- **Tier**: full

### M5 — Container daemons on the basic image over SSH, with migration

- **Tasks**: T042–T070
- **Deliverable**: a new or migrated container daemon starts from the pinned basic image, is provisioned over SSH and connects with the client's exact version; the form offers exactly "Basic image" and "My own image"; a running legacy container with sessions is kept until a restart with none.
- **Satisfies**: US1 #1, #3, #4; US2 #1–#4; US3 #1–#4; US4 #1–#5; FR-001, FR-003, FR-004, FR-005, FR-006, FR-007, FR-011 (recreated container), FR-014–FR-017, FR-026; SC-001, SC-002 (timed in the visual pass), SC-003, SC-005, SC-006. Until M7, a failed container start reports its step but does not yet roll back or bound the pull (FR-019 acquire bound, FR-020).
- **Verify**: `cargo test -p micold-core --test settings_released_image_migration`; `cargo test -p micold-client container_fields daemons`; `mise run image && mise run test-sandbox` (CI `sandbox-runtime`: `sandbox_real_provision`, `sandbox_real_limits`); quickstart §B 1–4, 6, 7
- **Depends on**: M1, M2, M3, M4
- **Tier**: full

### M6 — Per-daemon AI CLI selection

- **Tasks**: T071–T078
- **Deliverable**: each container daemon has its own AI CLI checkboxes (new daemon: Claude Code only; migrated: all); only selected and installed CLIs are offered; a failed install shows in the row with the installer output while the daemon connects.
- **Satisfies**: US1 #2; US5 #1; FR-010, FR-012; D3, D4
- **Verify**: `cargo test -p micold-client container_fields unavailable_default_says_so`; CI `sandbox-runtime` (`sandbox_real_provision` install-failure case); quickstart §B 1, 5
- **Depends on**: M5
- **Tier**: full

### M7 — Bounded, cancellable starts and the end-to-end suite

- **Tasks**: T079–T086
- **Deliverable**: every container start step fails naming its step within its bound, a stopped or failed start leaves no container or daemon it created, concurrent starts give one container, and CI runs the end-to-end provisioning suite.
- **Satisfies**: US5 #2, #3; FR-018, FR-019, FR-020, FR-022, FR-023, FR-025; SC-004
- **Verify**: `cargo test -p micold-core provision::attempt sandbox::`; CI `sandbox-runtime` (`sandbox_real_provision`, `sandbox_real_multi_daemon_container`) and `ssh-remote`
- **Depends on**: M5, M6
- **Tier**: full

Phase 12 (T088–T089) changes no code and is left to the close unit.
