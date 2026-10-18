# Contract: the basic image and its publication

## Image (`packaging/basic/`, `NEW`) — FR-002

- `Containerfile`: `FROM docker.io/library/node:24-trixie-slim` (Node for npm-installed CLIs, glibc
  2.41), plus `openssh-server`, `git`, `openssh-client`, `ca-certificates`, `util-linux` (`flock`),
  and what `packaging/sandbox/Containerfile` installs for sessions today, minus the daemon `COPY`,
  the AI CLI `ARG`/`npm install` block, `MICOLD_LISTEN_ADDR`, `MICOLD_TOKEN_PATH` and `EXPOSE 7727`.
  Keeps `XDG_DATA_HOME=/var/lib` and the Pi/OpenCode offline env. `EXPOSE 2222`. No `USER`.
  `/etc/passwd`, `/etc/group` writable by gid 0; `/etc/micold/sshd_config` (below);
  `ENTRYPOINT ["/usr/local/bin/micold-entrypoint"]`.
- `entrypoint.sh` → `/usr/local/bin/micold-entrypoint`: add the passwd/group line for `$(id -u)` with
  home `$MICOLD_HOME` when absent; start `$HOME/.micold/remote/$(cat $HOME/.micold/remote/last)/micold-daemon`
  in the background when that file exists; run `/usr/sbin/sshd -D -e -f /etc/micold/sshd_config` as the main process, and on SIGTERM/SIGINT send
  SIGTERM to every daemon (not bridge) process, wait up to 10 s (the runtime's default grace) for them to be gone (their orderly stop
  saves terminal history, feature 041 FR-002), then stop sshd.
- `sshd_config`: `Port 2222`, `ListenAddress 0.0.0.0`, `HostKey /run/micold/ssh/host_ed25519_key`,
  `AuthorizedKeysFile /run/micold/ssh/authorized_keys`, `PubkeyAuthentication yes`,
  `PasswordAuthentication no`, `KbdInteractiveAuthentication no`, `PermitRootLogin no`,
  `AllowTcpForwarding remote`, `GatewayPorts no`, `X11Forwarding no`, `PermitTTY no`,
  `StrictModes no`, `PidFile none`, `UsePAM no`.
- Labels: `org.opencontainers.image.source` (this repository), `title=micold-basic`,
  `org.micold.basic.content=<content hash>`.
- Must not contain `micold-daemon` or any AI CLI (a `sandbox_real_basic_image_has_no_daemon_or_cli` real-runtime
  test runs `command -v` for each inside it).

A user image ("My own image") must provide: an sshd reachable on container port 2222 that accepts
the key at `/run/micold/ssh/authorized_keys` with host key `/run/micold/ssh/host_ed25519_key` — the
simplest way is `FROM ghcr.io/jaroslawherod/micold-basic:<tag>`; `sh`, `uname`, `mkdir`, `cat`,
`mv`, `chmod`; and for AI CLIs, `npm` and `flock`. The user guide says so (FR-004, FR-026).

## Content hash (`scripts/basic-image-hash.sh`, `NEW`)

`sha256` over `find packaging/basic -type f | LC_ALL=C sort` names and contents; first 16 hex.
`--check` compares with `BASIC_IMAGE_CONTENT_HASH` in `sandbox/basic_image.rs` and prints `CURRENT` or
`DRIFT <pinned> <tree>` (exit 0 both: drift is expected between a change's merge and its pin PR).

## Publication (`.github/workflows/basic-image.yml`, `NEW`) — R1

| Trigger | Does |
|---|---|
| `pull_request` with `packaging/basic/**` changed | build `linux/amd64` + `linux/arm64`, no push |
| `push` to `main` with `packaging/basic/**` changed, and `workflow_dispatch` | compute hash; if `ghcr.io/jaroslawherod/micold-basic:<hash>` exists, print its digest and stop; else build and push both arches by digest, push the index tagged `<hash>`, print `BASIC_IMAGE = "ghcr.io/jaroslawherod/micold-basic:<hash>@sha256:<index digest>"` in the job summary |

Permissions: `contents: read`, `packages: write`; login with `GITHUB_TOKEN`. Native runners per
arch (as `release.yml` `image`), merged by `docker buildx imagetools create`.

## Pin check (`ci.yml`, `NEW` step in the `sandbox-runtime` job)

`docker manifest inspect "$(grep -o 'ghcr.io/jaroslawherod/micold-basic:[^"]*' crates/micold-core/src/sandbox/basic_image.rs)"`
succeeds anonymously. Fails the job on a pin nobody can pull.

## Unchanged

`release.yml` jobs `image` / `image-manifest` keep publishing `micold-daemon` (out of scope); the
client no longer references it except to recognise it for migration (`is_released_image`), and the
job's grep of `sandbox/image.rs` for `DEFAULT_IMAGE_REPOSITORY` keeps matching.
