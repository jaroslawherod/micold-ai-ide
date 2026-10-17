# Quickstart: validate container daemons provisioned over SSH (806)

Contracts: [provisioning](./contracts/provisioning.md), [basic image](./contracts/basic-image.md),
[settings](./contracts/daemon-settings.md). Types: [data-model.md](./data-model.md).

## Prerequisites

- Linux with Docker or Podman, the OpenSSH client (`ssh`, `ssh-keygen`), `mise trust` done.
- No `micold-daemon:*` image needed; remove old ones to prove it: `docker image rm
  micold-daemon:dev` (ignore "no such image").

## Part A — automated

| Run | Expect |
|---|---|
| `mise run gate` (cloud: `scripts/gate.sh`) | fmt, clippy, tests and script tests green, incl. core migration, argv, sequence, reason-text and relay tests |
| `mise run image` | builds `micold-basic:dev` from `packaging/basic/`; `docker run --rm --entrypoint sh micold-basic:dev -c 'command -v micold-daemon claude codex'` prints nothing |
| `mise run test-sandbox` | `sandbox_real_*` incl. `sandbox_real_provision` pass: fresh start, warm start with nothing copied, image without sshd, stand-in CLI install failure, migrated daemon, limits, stop during start, concurrent starts |
| `cargo test -p micold-daemon --features ssh-real-runtime ssh_real_ -- --test-threads=1` (needs a local sshd, as CI `ssh-remote`) | 489's suite green on the shared sequence |
| `scripts/basic-image-hash.sh --check` | `CURRENT`, or `DRIFT` only between an image change's merge and its pin PR |
| `docker manifest inspect <BASIC_IMAGE from sandbox/basic_image.rs>` | succeeds without login |

## Part B — by hand at a display (visual pass)

1. **New daemon (US1)**: Settings → Daemons → add a container daemon. The Image select shows exactly
   "Basic image (recommended)" and "My own image"; AI CLIs shows five boxes with only Claude Code
   ticked. Save, start. The row shows, live: getting the image → starting the container →
   connecting to the SSH server → checking the daemon build → copying the daemon (percent) →
   installing the AI CLIs (output lines) → starting the daemon; then `connected` with the client's
   version. Start a Claude Code session; the provider picker does not offer Codex.
2. **Warm start (US1 #4, SC-003)**: quit and reopen the client. The daemon is found running, no copy
   or install step appears, connected in under 15 s.
3. **Resources (US2)**: set CPU 2, memory 4 GiB, save; confirm the restart. `docker inspect
   <container> --format '{{.HostConfig.NanoCpus}} {{.HostConfig.Memory}}'` →
   `2000000000 4294967296`.
4. **Own image (US3)**: `mise run image`, choose "My own image" `micold-basic:dev`; the form says the
   image must run an SSH server. Start → connected. Then choose `docker.io/library/alpine:3` → start
   fails in "connecting to the SSH server" with the "no reachable SSH server" reason, row
   `unreachable`, entry in recent errors.
5. **Install failure (US5 #1)**: tick Codex, set the daemon's network to no outbound and block
   `registry.npmjs.org` on the host (e.g. `/etc/hosts` → `0.0.0.0`). Start: the row names Codex as
   not installed with npm's output; the daemon still connects; Claude Code (already installed) is
   offered, Codex is not. Undo the block.
6. **Migration (US4)**: with the client stopped, edit a container daemon in `settings.json` to
   `"image": {"kind":"registry","reference":"ghcr.io/jaroslawherod/micold-daemon:0.9.0"}` and remove
   `image_choice`, `ai_clis`. Open the client: the form shows the basic image and all five CLIs
   ticked; the row says it was moved off the released image. Start → connected; the notice is gone.
7. **Legacy container with sessions (US4 #3–#4)**: run a pre-upgrade build's container daemon with a
   session, upgrade the client, open it. The session keeps running; the row says the container is
   still on the released image; restart is disabled until the session ends, then moves it to the
   basic image.
8. **Remote SSH daemon (US6)**: start a 489 remote daemon. The same step labels appear except
   installing; its form has no AI CLI section.
