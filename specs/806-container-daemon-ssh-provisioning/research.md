# Research: Container daemons provisioned over SSH from a basic image (806)

**Status: DRAFT by plan unit 1 (handed over at the context cap).** Decisions below are the first
unit's findings from the code; the next plan unit verifies, completes (rejected alternatives,
remaining items) and writes plan.md, data-model.md, contracts/ and quickstart.md from them.

## Code found (what exists today)

- **SSH provisioning (489)**: `crates/micold-core/src/ssh/` — `provision.rs` (`SshRunner`,
  `Artefacts {own, bundled(os, arch)}`, `SshHost`, `ProvisionOptions`, `provision()` probe → copy
  only when absent → re-probe, `connect()` = provision + `argv::bridge` + `connect::handshake_with`
  over `Transport::Ssh`, no token), `argv.rs` (pure command lines; `probe_script`, `copy`, `bridge`,
  `build_id`, `remote_dir` = `$HOME/.micold/remote/<build>`), `classify.rs` (`SshFailure`,
  `ReasonContext`, reason table), `system.rs` (`SystemSsh`, the one real runner; `bundled()` reads
  `<exe dir>/remote/micold-daemon-<os>-<arch>`). Contract:
  `specs/489-ssh-remote-daemon/contracts/ssh-transport.md`. The bridge is
  `micold-daemon remote bridge` (spawns the daemon detached, relays stdio; no listener).
- **Client SSH dial**: `crates/micold-client/src/daemon.rs` `Dial::{Host, Container{port,
  token_path}, Ssh{target}}`, `connect_and_pump`, `ssh_attempt`, `copy_progress`
  (`ConnectionMsg::CopyProgress`); `shell/daemons.rs::system_ssh()` builds `SystemSsh`.
- **Container bring-up (027)**: `crates/micold-core/src/sandbox/` — `lifecycle.rs` (`SandboxState`
  Probing/Acquiring/Starting/Running/Stale/Failed, `Stage`, `bring_up`, `adopt`), `runtime.rs`
  (`ContainerRuntime` trait, `RuntimeError`, `Progress`), `image.rs` (`ImageSource{kind: Registry |
  ImportedFile | LocalBuild, reference, path}`, `DEFAULT_IMAGE` =
  `ghcr.io/jaroslawherod/micold-daemon:<CARGO_PKG_VERSION>`, `RETIRED_IMAGE_REPOSITORY`,
  `repair_retired_namespace`, `DEV_IMAGE_TAG`), `argv.rs` (`create`), `mod.rs` (`SandboxProfile
  {runtime, image, budget, network, credentials, survive_logout}`, `ResourceBudget` + minimums,
  `SandboxSpec{control_port, home, mounts, uid, gid, ...}`, `STATE_CONTAINER_DIR`). Client:
  `crates/micold-client/src/shell/sandbox.rs::start` (writes token, creates sandbox home, mounts,
  calls `bring_up`), `BootPlan`. Daemon registry: `crates/micold-core/src/daemons.rs`
  (`ContainerSettings{profile, container_name, port}`, `DaemonRuntime`, `migrated_entry`,
  `DaemonEntry::state_dir`).
- **Image today**: `packaging/sandbox/Containerfile` (node:24-trixie-slim, git, openssh-client, AI
  CLIs pinned by ARG, daemon COPY'd in, `MICOLD_LISTEN_ADDR=0.0.0.0:7727`). Published only by
  `.github/workflows/release.yml` jobs `image` / `image-manifest` on a release; the job greps
  `sandbox/image.rs` for the repository name. `packaging/sandbox/README.md` "Visibility needs no hand
  step": a package pushed by `GITHUB_TOKEN` from this repo is linked and public.
- **CI real-runtime suite**: `.github/workflows/ci.yml` job `sandbox-runtime` builds
  `micold-daemon:dev` locally and runs `sandbox_real_*` tests (`sandbox-real-runtime` feature);
  `mise run image`, `mise run test-sandbox`. Job `ssh-remote` runs 489's SSH suite.
- **AI CLI availability**: `crates/micold-core/src/provider.rs::available_in(path)` /
  `available_here()`; the start-refusal rule of feature 026 (`cli_reason.rs`,
  `crates/micold-client/tests/unavailable_default_says_so.rs`). `AiCli::ALL` (5 CLIs).
- **Bundled Linux daemons for macOS/Windows clients do not exist yet**: 489 M8 (T082–T084) is
  still pending (`specs/489-ssh-remote-daemon/autopilot.md`). FR-009 needs them, for
  `linux-x86_64` and `linux-aarch64` (Apple silicon Docker runs arm64 Linux). `ring` is the only
  native-crypto dependency; it builds on musl.

## Decisions reached (to verify and complete)

- **R1 Publishing the basic image (escalation candidate, category 3).** A release-only publish
  cannot precede the commit that pins its digest (the client of release N would ship without it, so
  the default would work only from release N+1, and never on main between). Proposed: a new
  workflow `.github/workflows/basic-image.yml`, on push to `main` touching `packaging/basic/**` plus
  `workflow_dispatch`, builds amd64 + arm64, pushes `ghcr.io/jaroslawherod/micold-basic:<content
  hash of packaging/basic>` (skips when the tag exists) and the multi-arch index, prints the index
  digest. A later milestone pins `micold-basic@sha256:<digest>` in a new
  `crates/micold-core/src/sandbox/basic_image.rs`, with a test that the pinned content hash equals
  the hash of `packaging/basic/` (drift fails CI) and a CI/network step that the pinned digest is
  pullable anonymously. This changes D2's "release workflow's existing image jobs": the merge of the
  milestone that adds the workflow publishes to GHCR, an outward action — ask the owner (options:
  publish on merge via the new workflow (recommended); publish through a release; owner runs
  `workflow_dispatch` by hand). The real-runtime tests never need the published image: CI builds
  the basic image locally (`micold-basic:dev`).
- **R2 One port, daemon over the SSH bridge.** The container's sshd is published on the daemon's
  stored loopback port (spec 805 rules unchanged); the daemon's control channel runs over
  `micold-daemon remote bridge` exactly as 489, so no second port, no token mount for new
  containers, and the same `connect()` for container, remote host and 686's pod (pod supplies a
  port-forward to sshd). A running pre-upgrade container (US4 #3) keeps the legacy TCP+token
  `Dial::Container` path until restarted with no sessions.
- **R3 sshd as the host uid.** The container runs `--user uid:gid` with no passwd entry; sshd needs
  one. The basic image's entrypoint adds a passwd line for the current uid (image makes
  `/etc/passwd` writable) and runs a non-root `sshd -D -f <config>` on container port 2222, key
  auth only, `AuthorizedKeysFile` = a read-only bind mount. The entrypoint also restarts the last
  provisioned daemon build if present, so a restart-policy restart (survive logout) keeps
  unattended behaviour.
- **R4 Daemon key and host key.** Client generates per daemon, under the daemon's state dir
  `ssh/`: a client key pair and the container host key pair, with `ssh-keygen` (ships with every
  OpenSSH client the 489 runtime already requires; no new crate) — alternative `ssh-key` crate to
  weigh. Host private key and client public key are bind-mounted read-only; the client writes a
  per-daemon `known_hosts`, and runs ssh with `-F none`-equivalent options: `IdentitiesOnly=yes`,
  `IdentityAgent=none`, `-i <daemon key>`, `UserKnownHostsFile=<daemon known_hosts>`,
  `StrictHostKeyChecking=yes`, `-p <stored port> 127.0.0.1` (FR-006). Windows key ACLs via
  `crates/micold-core/src/owner_only.rs`.
- **R5 Shared provisioning contract.** Generalise `ssh::provision` into a target-agnostic step
  list (reach, probe, copy, start/bridge, version check, AI CLI install where app-owned) over
  `SshRunner` + a `ProvisionTarget` that supplies only how to reach sshd (`SshTarget` for 489;
  loopback port + daemon key for containers; 686 adds its port-forward). Same step names, progress
  and reasons; write it as `contracts/provisioning.md`.
- **R6 AI CLI install.** Versions move from Containerfile ARGs to a Rust table (npm package +
  pinned version per `AiCli`). Install over SSH: `npm install -g --prefix
  /var/lib/micold-ai-ide/ai-clis/<cli>@<version>` (state dir, survives recreation, FR-011); the
  daemon is started with those bin dirs on `PATH`, so `available_in(PATH)` offers only installed
  ones (FR-012). Tests install a scripted stand-in package from a local tarball (no network).
- **R7 Artefacts.** Container target uses the 489 rule (own build when the platform matches, else
  bundled). This feature delivers 489 M8 extended: release builds `x86_64-` and
  `aarch64-unknown-linux-musl` daemons and bundles them into the macOS and Windows packages.

## Still to research

Migration hook (where `repair_retired_namespace` runs; `settings_retired_image_namespace.rs`), the
form (spec 804 container section, `features/sandbox.rs`), per-step time bounds (FR-019) and
cancellation (FR-020), progress steps in `SandboxState`/`ConnectionMsg`, concurrent starts
(FR-023), the 489 `ssh-remote` CI job's move onto the shared step, what #686's plan expects
(`specs/686-kubernetes-daemon-runtime/plan.md`).
