# The basic image

The image container daemons start from (feature 806, FR-002): an SSH server and the tools a session
needs. The client starts a container from it, reaches its sshd over a loopback port, copies its own
`micold-daemon` build in and installs the AI CLIs the user selected for that daemon, the same way it
provisions a daemon on an SSH host (feature 489). So the image does not follow the app's version: one
published image serves every client, released or not.

## What it carries, and what it must not

Carries: `sshd` (config `/etc/micold/sshd_config`, port 2222, key auth only, run as the container's
uid), Node 24 and `npm` (the AI CLIs are npm packages), `git`, `openssh-client`, `flock`, `bash`,
`procps`, `less`, `tzdata`, glibc 2.41 (trixie), and `XDG_DATA_HOME=/var/lib` for the daemon's state.

Must not carry: `micold-daemon`, any AI CLI, or any key, token or credential. The SSH host key and
authorized key are mounted read-only at `/run/micold/ssh` at run time, and AI CLI sign-ins reach the
container only through the credential shares the user allowed (FR-013).
`scripts/tests/basic-image-hash.test.sh` refuses a `COPY`/`ADD` of a key, credential, `.ssh`, token
or the daemon and any `npm install` in the `Containerfile`; the real-runtime test
`sandbox_real_basic_image_has_no_daemon_or_cli` checks the built image.

The image already holds a passwd entry for uid 10001 (home `/data/home`), the uid the Kubernetes pod runs as with a read-only root filesystem (feature 686), where nothing can be appended at run time.
`entrypoint.sh` adds a passwd entry for any other run-time uid (home `$MICOLD_HOME`), starts the last
provisioned daemon build (`$HOME/.micold/remote/last`) in the background when there is one, and runs
sshd as its main process. When the runtime stops the container it first stops every daemon (so they
save their terminal history), then sshd.

## A user's own image

"My own image" must provide what the client relies on:

- an sshd on container port 2222 accepting the key at `/run/micold/ssh/authorized_keys`, with host key
  `/run/micold/ssh/host_ed25519_key`. The simplest way is `FROM ghcr.io/jaroslawherod/micold-basic:<tag>`;
- `sh`, `uname`, `mkdir`, `cat`, `mv`, `chmod`;
- for AI CLIs, `npm` and `flock`.

## Build locally

```sh
docker build -f packaging/basic/Containerfile -t micold-basic:dev packaging/basic
MICOLD_BASIC_IMAGE_TAG=micold-basic:dev \
  cargo test -p micold-core --features sandbox-real-runtime --test sandbox_real_basic_image
```

`podman build` works the same.

## Hash, publish, pin

1. **Hash.** Any change under `packaging/basic/` changes `scripts/basic-image-hash.sh`'s output. Put
   the new value in `BASIC_IMAGE_CONTENT_HASH` (`crates/micold-core/src/sandbox/basic_image.rs`) in the
   same PR: `scripts/tests/basic-image-hash.test.sh` fails while they differ, and
   `scripts/basic-image-hash.sh --check` prints `CURRENT` or `DRIFT <pinned> <tree>`.
2. **Publish.** `.github/workflows/basic-image.yml` builds both architectures on a pull request that
   touches this directory, without pushing. On merge to `main` (or a manual run) it pushes
   `ghcr.io/jaroslawherod/micold-basic:<hash>` for `linux/amd64` and `linux/arm64`, unless that tag
   already exists, and prints the pin in the run's summary:
   `BASIC_IMAGE = "ghcr.io/jaroslawherod/micold-basic:<hash>@sha256:<index digest>"`.
3. **Pin.** A follow-up PR sets `BASIC_IMAGE` in `basic_image.rs` to that line, so the client pulls
   by digest. CI's `sandbox-runtime` job checks the pin can be pulled anonymously. Until the pin PR
   merges, clients keep the previous pin, which still resolves: published tags are never moved or
   deleted.

The package is created by the workflow's `GITHUB_TOKEN`, so it is linked to this repository and
inherits its public visibility, as the daemon image does (`packaging/sandbox/README.md`).
