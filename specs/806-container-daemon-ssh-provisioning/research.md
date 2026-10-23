# Research: Container daemons provisioned over SSH from a basic image (806)

Decisions for [plan.md](./plan.md). Each records what was chosen, why, and what was rejected.
Code paths are the workspace's own; `NEW` marks what this feature adds.

## Code found (what exists today)

- **SSH provisioning (489)**: `crates/micold-core/src/ssh/` — `provision.rs` (`SshRunner`,
  `Artefacts {own, bundled(os, arch)}`, `SshHost`, `ProvisionOptions`, `provision()` = probe → copy
  only when absent → re-probe, `connect()` = provision + `argv::bridge` + `connect::handshake_with`
  over `Transport::Ssh`, no token), `argv.rs` (pure command lines: `command()` common prefix with
  `BatchMode=yes`, `StrictHostKeyChecking=yes`, `ClearAllForwardings=yes`; `probe_script`, `copy`,
  `bridge`, `build_id`, `remote_dir` = `$HOME/.micold/remote/<build>`), `classify.rs` (`SshFailure`,
  `ReasonContext`, reason table), `system.rs` (`SystemSsh`, the one real runner; `bundled()` reads
  `<bundled_dir>/micold-daemon-<os>-<arch>`), `target.rs` (`SshTarget {host, user, port,
  key_file}`). Contract: `specs/489-ssh-remote-daemon/contracts/ssh-transport.md`. The bridge is
  `micold-daemon remote bridge` (`crates/micold-daemon/src/remote.rs`): spawns the daemon detached
  on the pinned endpoint (Linux `/tmp/micold-<euid>/`, inside the container's own filesystem), relays
  stdio, opens no listener.
- **Client SSH dial**: `crates/micold-client/src/daemon.rs` `Dial::{Host, Container{port,
  token_path}, Ssh{target}}`, `connect_and_pump`, `copy_progress` → `ConnectionMsg::CopyProgress(u8)`
  (`features/connection.rs`) → `features::daemons::Msg::CopyProgress`; `shell/daemons.rs::system_ssh()`
  builds `SystemSsh`.
- **Container bring-up (027)**: `crates/micold-core/src/sandbox/` — `lifecycle.rs` (`SandboxState`
  Disabled/Probing/Acquiring/Starting/Running/Stale/Failed, `Stage`, pure `adopt()` that replaces a
  container whose image differs at once, `bring_up()`), `runtime.rs` (`ContainerRuntime`: `detect`,
  `probe`, `inspect_image`, `acquire_image`, `create`, `start`, `stop`, `remove`, `inspect`;
  `RuntimeError::Timeout`), `exec.rs` (streamed child output, `QUIET_TICK`), `image.rs`
  (`ImageSource{kind: Registry | ImportedFile | LocalBuild, reference, path}`, `DEFAULT_IMAGE` =
  `ghcr.io/jaroslawherod/micold-daemon:<CARGO_PKG_VERSION>`, `RETIRED_IMAGE_REPOSITORY`,
  `repair_retired_namespace()`, `DEV_IMAGE_TAG` = `micold-daemon:dev`), `argv.rs` (`create`,
  `network_create`: `NoOutbound` is a bridge with masquerade off), `mod.rs` (`SandboxProfile
  {runtime, image, budget, network, credentials, survive_logout}`, `ResourceBudget` + minimums,
  `NetworkPosture::{NoOutbound (default), Outbound}`, `MountSet`, `HomeMount` under the state dir,
  `STATE_CONTAINER_DIR` = `/var/lib/micold-ai-ide`). Client: `crates/micold-client/src/shell/sandbox.rs`
  (`start`, `BootPlan`, `InFlight {task, cancelled}` whose `cancel()` aborts the task but does not
  interrupt a runtime call in progress). Daemon registry: `crates/micold-core/src/daemons.rs`
  (`ContainerSettings{profile, container_name, port}`, `DaemonRuntime`, `migrated_entry`).
  Settings read-repair: `crates/micold-core/src/settings.rs` (~l.851) calls
  `repair_retired_namespace()`; test `crates/micold-core/tests/settings_retired_image_namespace.rs`.
- **Form**: `crates/micold-client/src/ui/settings/container_fields.rs` (`ContainerView`,
  `FieldErrors`, `limits()`, `sections()`), drawn by `ui/settings/daemons.rs`.
- **Image today**: `packaging/sandbox/Containerfile` (node:24-trixie-slim, git, openssh-client, AI
  CLIs pinned by `ARG`, daemon `COPY`'d in, `MICOLD_LISTEN_ADDR=0.0.0.0:7727`,
  `MICOLD_TOKEN_PATH`, `XDG_DATA_HOME=/var/lib`). Published only by `.github/workflows/release.yml`
  jobs `image` / `image-manifest` on a release; the job greps `sandbox/image.rs` for the repository
  name. `packaging/sandbox/README.md`: a package pushed by `GITHUB_TOKEN` from this repo is linked
  and public with no hand step.
- **CI real-runtime suites**: `.github/workflows/ci.yml` job `sandbox-runtime` builds
  `micold-daemon:dev` and runs `sandbox_real_*` (`sandbox-real-runtime` feature; `mise run image`,
  `mise run test-sandbox`). Job `ssh-remote` installs `openssh-server` and runs `ssh_real_*`
  (`ssh-real-runtime` feature) in `micold-daemon`.
- **AI CLI availability**: the daemon answers from `micold_core::provider::available_in(PATH)`
  (`crates/micold-daemon/src/state.rs` `ai_clis_available_in`, `spawn_path_for`); the start refusal
  of feature 026 (`cli_reason.rs`, `crates/micold-client/tests/unavailable_default_says_so.rs`).
  `AiCli::ALL` has 5 CLIs, all npm-distributed.
- **Bundled Linux daemons for macOS/Windows clients do not exist yet**: 489 M8 (T082–T084) is
  pending. `ring` is the only native-crypto dependency and builds on musl. No `ssh-key` /
  `ed25519` crate is in `Cargo.lock`.
- **686 (Kubernetes)** is designed, not implemented: its plan introduces a `daemon_image/` module and
  `DaemonImageSource` "ahead of #806" for a build-and-push model this feature's owner decision
  replaced. 686 has not merged; this feature lands first and its contract
  ([contracts/provisioning.md](./contracts/provisioning.md)) is what 686 re-plans onto.

## R1. Publishing the basic image

- **Decision**: New workflow `.github/workflows/basic-image.yml`. On `pull_request` touching
  `packaging/basic/**` it builds both architectures without pushing. On `push` to `main` touching
  `packaging/basic/**` (and `workflow_dispatch`) it builds `linux/amd64` and `linux/arm64`, pushes
  `ghcr.io/jaroslawherod/micold-basic:<content hash>` (skipped when that tag exists), pushes the
  multi-arch index, and prints the index digest and the pin line. The content hash is
  `scripts/basic-image-hash.sh` (sha256 over the sorted file list and contents of
  `packaging/basic/`, first 16 hex). The client pins `micold-basic:<hash>@sha256:<digest>` in `NEW
  crates/micold-core/src/sandbox/basic_image.rs`. Changing the image is two PRs: the change (its
  merge publishes), then the pin. A CI step in `ci.yml` checks the pinned reference is pullable
  anonymously (`docker manifest inspect`); drift between the pin and `packaging/basic/` is reported
  by `scripts/basic-image-hash.sh --check`, not a failure, because a change cannot be pinned before
  its merge publishes it. Tests never need the published image: CI and `mise run image` build
  `micold-basic:dev` locally. The orchestrator has the owner's answer on this publish route (asked
  in the run); a different answer changes only this workflow's trigger.
- **Rationale**: FR-002 needs a version-independent image a client of any version can pull, named by
  digest. A release-only publish cannot precede the commit that pins it: release N would ship
  unpinned and main would have no pullable default between releases. Pushing by `GITHUB_TOKEN` links
  and publishes the package with no hand step (packaging/sandbox/README.md).
- **Rejected**: D2's "release workflow's existing image jobs" (cannot precede the pin, above);
  publishing by hand (an outward step on someone's machine, not reproducible); a fatal drift test
  (would fail every PR that changes `packaging/basic/` until a merge it blocks); tag-only pin
  (a moved tag would change what every client runs).

## R2. One port: sshd published, daemon over the 489 bridge

- **Decision**: The container's sshd listens on container port 2222 and is published on
  `127.0.0.1:<stored port>` (spec 805 rules unchanged). The control channel runs over `micold-daemon
  remote bridge` exactly as 489: no second port, no token mount for new containers, the same
  `connect()` for container, remote host and 686's pod. A pre-upgrade container still running
  sessions keeps the legacy TCP+token `Dial::Container` path until restarted with no sessions
  (FR-017).
- **Rationale**: FR-005/FR-006 (loopback only, no extra port), FR-008 (one mechanism). The bridge
  already solves "start the daemon if absent, relay stdio, one daemon per endpoint".
- **Rejected**: publishing both sshd and the daemon's TCP listener (two loopback ports, keeps the
  token mount, two transports to test); `docker exec` instead of SSH (not usable for 686's pod or
  489's host, so not one mechanism); SSH local port-forward to a daemon TCP listener (needs the
  token again and a listener inside the container).

## R3. sshd as the host uid, and the entrypoint

- **Decision**: The container runs `--user uid:gid` with no passwd entry; sshd needs one. The basic
  image makes `/etc/passwd` and `/etc/group` group-writable by gid 0 and its entrypoint
  (`packaging/basic/entrypoint.sh`) appends a line for the current uid with `HOME` = the container
  home path the client passes in `MICOLD_HOME`, then `exec`s a non-root `sshd -D -e -f
  /etc/micold/sshd_config`: `Port 2222`, `PasswordAuthentication no`, `KbdInteractiveAuthentication
  no`, `PubkeyAuthentication yes`, `AuthorizedKeysFile /run/micold/ssh/authorized_keys`, `HostKey
  /run/micold/ssh/host_ed25519_key`, `AllowTcpForwarding remote`, `GatewayPorts no`,
  `PermitTTY no`, `StrictModes no` (the mounts are owned by the host uid, which is the container
  uid), `PidFile none`. Before `exec` it starts the daemon of the last provisioned build
  (`$HOME/.micold/remote/last`, written by provisioning) in the background when present, so a
  restart-policy restart (survive logout, 027) runs the daemon unattended as today.
- **Rationale**: the image must work for every uid (no `USER`, as 027); sshd refuses a user that
  `getpwuid` cannot resolve. Non-root sshd works for key auth of its own user only, which is all
  that is needed.
- **Rejected**: running the container as root and sshd dropping privileges (files written to the
  host-mounted state dir would be root-owned, 027 forbids); `nss_wrapper` (an extra library in
  the image for the same effect as one appended line).

## R4. Daemon key and host key

- **Decision**: Per daemon, under `<daemon state dir>/ssh/` (`NEW`), the client generates once with
  `ssh-keygen -t ed25519 -N "" -q` a client key pair (`client_ed25519`, `.pub`) and a container host
  key pair (`host_ed25519`, `.pub`), writes `authorized_keys` (the client public key with
  `restrict,port-forwarding` options) and `known_hosts` (`[127.0.0.1]:<port> <host public key>`,
  rewritten when the port changes). The host private key and `authorized_keys` are bind-mounted
  read-only at `/run/micold/ssh/`. ssh runs with `-F none` (no user configuration), `IdentitiesOnly=yes`,
  `IdentityAgent=none`, `-i <client key>`, `UserKnownHostsFile=<daemon known_hosts>`,
  `GlobalKnownHostsFile=none`, `StrictHostKeyChecking=yes`, `-p <stored port>`, `-l <container
  user name>`, host `127.0.0.1`. Private keys are owner-only: Unix mode 0600, Windows through
  `crates/micold-core/src/owner_only.rs` (Windows `ssh.exe` refuses a key others can read).
- **Rationale**: FR-006 (client-created per-daemon key, never the user's keys or agent), and a pinned
  host key means no trust-on-first-use prompt and no write to the user's `known_hosts` (489 FR-010).
  `ssh-keygen` ships with every OpenSSH client the 489 runtime already requires on all three OSes.
- **Rejected**: the `ssh-key` crate (a new dependency tree for what one subprocess does; would still
  need the Windows ACL step); `StrictHostKeyChecking=no` (accepts any key on the port; another local
  process holding the port could impersonate the container); the user's `~/.ssh` (FR-006).

## R5. One shared provisioning contract

- **Decision**: `NEW crates/micold-core/src/provision/` holds the target-agnostic sequence: `Reach`
  (wait until sshd answers), `Probe`, `Copy` (only when absent), `InstallAiClis` (only for app-owned
  targets with a non-empty selection), `Start` (bridge + handshake), `VersionCheck` (489 FR-011
  rule, unchanged). It runs over the existing `SshRunner` + `Artefacts` and a `ProvisionTarget` that
  supplies only how to reach sshd (`ssh::argv::Reach`), whether it is app-owned, its step bounds and
  its install egress. 489's `ssh::provision::{provision, connect}` become the sequence with the
  `UserHost` target, keeping their behaviour and reason texts. The container is the `Container`
  target; 686 adds a target whose reach is a port-forward to the pod's sshd. Step names, progress
  events and reasons: [contracts/provisioning.md](./contracts/provisioning.md).
- **Rationale**: FR-008 and US6: same steps, progress and reasons on every runtime; 686 supplies only
  reach.
- **Rejected**: a container-only copy of the 489 code (two mechanisms, US6 fails); a trait per step
  with per-target implementations (targets differ only in reach, bounds and ownership, so the steps
  need no polymorphism).

## R6. AI CLI install

- **Decision**: The pinned versions move from `Containerfile` `ARG`s to a Rust table (`NEW
  crates/micold-core/src/provision/ai_cli.rs`: per `AiCli`, npm package, pinned version, bin names).
  The install step runs one fixed script over SSH (`sh -s` on stdin, as the 489 probe), under
  `flock <state>/ai-clis/.lock`: for each selected CLI whose `<state>/ai-clis/<cli>@<version>/`
  is absent, `npm install -g --prefix <dir>.part <package>@<version>` then rename `.part` to the
  final directory; output streams as progress. `<state>` is `/var/lib/micold-ai-ide` (host-mounted
  daemon state), so installs survive restart and container recreation (FR-011). The step then writes
  `<state>/ai-clis/offered.json` (the selected CLIs whose directory exists, with their bin dirs).
  The daemon prepends those bin dirs to the `PATH` it resolves providers against
  (`state.rs::spawn_path_for`), so `available_in` offers only selected and installed CLIs (FR-012)
  and a deselected CLI stops being offered at once while its running sessions keep their own
  process. A failed CLI leaves no directory, is reported by name with the installer's output
  (`ProvisionWarning`), and does not stop the start. Tests install a scripted stand-in package
  from a local tarball (`npm install -g <tarball>`), no network.
- **Rationale**: FR-010–FR-012 and the Assumption that versions are pinned per release; the daemon
  already resolves providers from a `PATH`, so a `PATH` entry is the smallest change.
- **Rejected**: installing into the image (rebuild per selection, dropped by D1); `npm install` under
  `$HOME` (one global prefix: a failed CLI half-installed beside working ones, no atomic rename);
  telling the daemon its CLIs over the protocol (a protocol bump for what a file read answers).

## R7. Daemon builds for every client and container architecture

- **Decision**: Containers use the 489 artefact rule (own build when client OS/arch match the
  container's `uname`, else bundled). This feature delivers 489 M8 (T082–T084) extended to two
  architectures: `NEW scripts/remote-daemon-bundle.sh` builds `x86_64-unknown-linux-musl` and
  `aarch64-unknown-linux-musl` daemons (`cargo zigbuild`, statically linked), named
  `micold-daemon-linux-x86_64` / `-linux-aarch64`; `scripts/macos-bundle.sh` and
  `scripts/windows-installer.sh` put them in the bundled dir `SystemSsh` reads, and the `.deb`/`.rpm`
  packages carry the one for the other architecture. `release.yml` and the CI bundle jobs build them.
  A client with no matching build fails `NoMatchingBuild` naming the container's platform (FR-009).
- **Rationale**: Docker Desktop on Apple silicon runs arm64 Linux; on Intel macOS and Windows,
  x86_64 Linux. A static musl build runs on the basic image and on any user image regardless of its
  glibc. `ring` builds on musl.
- **Rejected**: copying only the client's own build (a macOS or Windows binary cannot run in a
  Linux container); downloading the daemon from a release at start (unreleased clients have none,
  FR-002/US1); glibc cross builds (would bind user images to a minimum glibc).

## R8. AI CLI downloads under the no-outbound posture

- **Decision**: When the daemon's posture is `NoOutbound` (the default) and the install step has
  something to download, provisioning opens the install command with an SSH remote forward
  `-R 127.0.0.1:17728:127.0.0.1:<relay port>` to a client-side **install relay** (`NEW
  crates/micold-core/src/provision/relay.rs`, tokio): an HTTP `CONNECT` proxy on client loopback
  that admits only the hosts the pinned table names (`registry.npmjs.org:443`) and refuses every
  other target, alive only for that one command. The script sets `HTTPS_PROXY` /
  `npm_config_https_proxy` to `http://127.0.0.1:17728`. With `Outbound` the install uses the
  container's own network and no relay. The install command alone is built without
  `ClearAllForwardings` (which also clears command-line forwards); `-F none` already excludes the
  user's configured forwards. A relay or forward failure fails that step naming the posture, within
  its bound (spec edge case "never hangs").
- **Rationale**: the default posture would otherwise make every first start of a default daemon
  fail its install (US1). Sessions keep no egress: the forward exists only on the install
  connection, only to the registry, only during the step. Reach goes through SSH, so the same
  relay serves 686's pods in restricted namespaces.
- **Rejected**: failing the install naming the posture (US1 would fail with default settings);
  a sibling helper container with egress writing into the state dir (a second container per start,
  not available to a pod or a remote host, so not one mechanism); `docker network connect` for the
  install window (gives every process in the container egress during it); SSH reverse dynamic
  forward (`-R <port>` SOCKS) (no host allowlist: any postinstall script reaches any host through
  the user's machine); downloading tarballs in the client (the client would resolve npm dependency
  trees).

## R9. Migration

- **Decision**: The read-repair in `settings.rs` (where `repair_retired_namespace()` runs today) and
  the daemon-registry read (`daemons.rs`) map each stored `ImageSource` to `ImageChoice`
  ([data-model.md](./data-model.md)): a `Registry` reference whose repository is the released image
  (current or retired namespace, any tag or digest) becomes `Basic` and sets
  `migration: MovedOffReleasedImage`; any other reference, an archive or a `LocalBuild` becomes
  `Own` with the reference or path unchanged; a profile with no stored image (serde default) becomes
  `Basic` with no notice. Every pre-upgrade container daemon gets `ai_clis = AiCli::ALL` (D3); a
  daemon created after the upgrade defaults to `{ClaudeCode}`. Settings are written back on the next
  save in the new shape (`image_choice`, `ai_clis`, `migration`); the old `image` key is read, never
  written. The notice clears on the first successful start on the basic image.
- **Rationale**: FR-016, SC-005, US4. The existing read-repair is the place the app already rewrites
  what the user did not choose.
- **Rejected**: keeping `ImageSource` and adding a `Basic` kind (keeps `LocalBuild` and the
  fingerprint rules, a third option the form must hide, FR-001 says exactly two); migrating on
  first start (the form would show the released image until then, FR-016 says "at once").

## R10. Adopting a container: the legacy case

- **Decision**: `adopt()` gains `Adoption::Legacy(id)`: an existing container on the released image
  (by repository) that is running is attached through the legacy `Dial::Container` TCP+token path,
  not replaced, and its row says it is still on the released image; the restart action that moves
  it is enabled only when the daemon reports no running session (489 FR-011 rule). A stopped legacy
  container is replaced at once (no sessions can run). For every other image difference the
  existing `Replace` stays. `StaleBuild` / the strict fingerprint check apply to no `ImageChoice`
  any more: the daemon's version is checked at the handshake instead (FR-007).
- **Rationale**: FR-017 and US4 #3–#4; today `adopt()` replaces any container whose image differs,
  which would end running sessions.
- **Correction (2026-10-23, BUG-1165)**: the restart gate was implemented on the client's cached
  session list, and provisioning does not replace a stale daemon in place: `remote bridge` reuses the
  daemon already listening. The 489 rule (explanation, offer over the pinned reach, Replace, the
  row's Stop as the way out) is added for containers by Phase 14 of tasks.md.
- **Rejected**: attaching every differing running container (a user's own image change would never
  apply, US2 #2 expects the restart confirmation).

## R11. Step bounds, cancellation and rollback

- **Decision**: Each step has a bound in `StepBounds` (contract): container target `Acquire` 15 min,
  `InstallAiClis` 10 min in total, every other step 120 s (FR-019); `Reach` polls sshd (TCP connect
  then an SSH no-op every 500 ms) inside its 120 s, covering "SSH server not ready yet". The 489
  user-host target keeps its connect deadline and its unbounded copy with progress (489 research R3:
  a slow link to a remote host is not a failure), recorded as a per-target bound, not a different
  sequence. Cancellation is a `CancelToken` (`NEW`, an `Arc<AtomicBool>` plus a `Notify`) checked
  between steps and passed to `exec.rs` and `SystemSsh`, which kill the child on cancel; the client's
  `InFlight::cancel()` sets it. On failure or cancel the attempt removes what it created
  (`AttemptLedger`: container created in this attempt → `remove`; daemon started in this attempt →
  `stop` the container, which ends it); an adopted container or a daemon found running is left
  alone, and host state is kept (FR-020).
- **Rationale**: FR-019/FR-020/SC-004; today `InFlight::cancel()` cannot interrupt a pull.
- **Rejected**: one overall start timeout (cannot name the step, FR-019); retrying a timed-out step
  (FR-019 forbids silent retry).

## R12. Concurrent starts

- **Decision**: Within a client, the existing per-daemon `InFlight` keeps one start at a time. Across
  clients: the container name is deterministic, so a second `create` that fails with a name
  conflict re-inspects and adopts the winner (`Attach`, one retry); the copy lands by atomic rename;
  the install step holds `flock` on `<state>/ai-clis/.lock`; the bridge connects to the one daemon
  the daemon's singleton allows (`crates/micold-daemon/src/singleton.rs`). Result: one container,
  one daemon (FR-023).
- **Rejected**: a host-side lock file (two clients on different hosts of one runtime socket share
  no filesystem lock, and the runtime already arbitrates the name).

## R13. Progress and failure display

- **Decision**: `ConnectionMsg::CopyProgress(u8)` generalises to `ConnectionMsg::Provisioning(
  ProvisionEvent)` (step, copy percent, install output line), mapped into
  `features::daemons::State` per daemon and drawn in the daemon's row (FR-021, US1 #3). A start
  failure becomes the daemon's `unreachable` state with the `ProvisionFailure` reason and output,
  and is appended to the daemon's recent errors shown by the existing diagnostics view
  (`features/daemons.rs` "recent errors"), US5 #3. Install warnings are listed the same way.
- **Rejected**: a separate progress dialog (FR-021 says in the row).

## R14. CI and the 489 suite

- **Decision**: `ci.yml` `sandbox-runtime` builds `micold-basic:dev` from `packaging/basic/` and runs
  the `sandbox_real_*` suite, now including `NEW sandbox_real_provision.rs` (fresh start, reuse
  without copying, image without sshd, install failure with the stand-in CLI, migrated daemon,
  limits). `ssh-remote` keeps running `ssh_real_*` unchanged in name, now over the shared sequence
  (FR-025). Both stay off by default locally (`sandbox-real-runtime`, `ssh-real-runtime` features)
  and run with `mise run test-sandbox` / the 489 task.
- **Rejected**: a new CI job (the two existing jobs already have the runtime and the sshd).
