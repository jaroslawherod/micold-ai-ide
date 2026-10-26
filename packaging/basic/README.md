# The basic image

The image container daemons start from (feature 806, FR-002): an SSH server and the tools a session
needs. The client starts a container from it, reaches its sshd over a loopback port, copies its own
`micold-daemon` build in and installs the AI CLIs the user selected for that daemon, the same way it
provisions a daemon on an SSH host (feature 489). So the image does not follow the app's version: one
published image serves every client, released or not.

## What it carries, and what it must not

Carries: `sshd` (config `/etc/micold/sshd_config`, port 2222, key auth only, run as the container's
uid), Node 24 and `npm` (the AI CLIs are npm packages), `git`, `git-lfs`, `gh`, `curl`, `wget`,
`jq`, `ripgrep`, `openssh-client`, `flock`, `bash`, `procps`, `less`, `tzdata`, glibc 2.41
(trixie), and `MICOLD_DATA_DIR=/var/lib/micold-ai-ide` for the daemon's state.

For building (#1439), before any setup has run:

- `mise` 2026.10.19 at `/usr/local/bin/mise`, the release binary checked at build time against
  `mise.sha256` (two lines, `linux-x64` and `linux-arm64`, from the release's signed
  `SHASUMS256.txt`); a mismatch or a failed download fails the build. See "Update mise" below;
- a C/C++ toolchain (`gcc`, `g++`, `cc`, `c++`, `ld`, the C library headers), `make` and
  `pkg-config` (`pkgconf`), so a Rust toolchain installed by mise links crates with C code;
- `python3`, and `xz` and `unzip` for what installers download.
- the X11/Wayland development libraries this repository's tests link (`libxkbcommon-dev`,
  `libwayland-dev`, `libx11-dev`, `libxcursor-dev`, `libxrandr-dev`, `libxi-dev`, FR-007), in a layer
  of their own after the mise step, so `pkg-config` finds them and the default image builds and tests
  this repository. Any other system library a project needs comes from an own image built `FROM` this
  one.

The entrypoint also creates `~/.local/bin` and mise's shims folder in the home before sshd starts,
the two folders every session's `PATH` holds (#1289), leaving anything already in them alone. It
picks the shims folder from the container's environment: a `MISE_DATA_DIR` or `XDG_DATA_HOME` set
only by a project's environment-include puts another folder on `PATH`, which mise creates itself.

Must not carry: `micold-daemon`, any AI CLI, or any key, token or credential. The SSH host key and
authorized key are mounted read-only at `/run/micold/ssh` at run time, and AI CLI sign-ins reach the
container only through the credential shares the user allowed (FR-013).
`scripts/tests/basic-image-hash.test.sh` refuses a `COPY`/`ADD` of a key, credential, `.ssh`, token
or the daemon and any `npm install` in the `Containerfile`; the real-runtime test
`sandbox_real_basic_image_has_no_daemon_or_cli` checks the built image.

The image already holds a passwd entry for uid 10001 (home `/data/home`), the uid the Kubernetes pod runs as with a read-only root filesystem (feature 686), where nothing can be appended at run time.
`entrypoint.sh` gives the login name `micold` to any other run-time uid (home `$MICOLD_HOME`), replacing that entry, starts the last
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

## Update mise

The pin is taken by hand, on a machine that reaches `github.com`, from a release's signed checksums,
never from the build that checks them:

1. **Download** the release's checksums into an empty folder:
   `gh release download v<ver> -R jdx/mise -p SHASUMS256.txt -p SHASUMS256.asc -p SHASUMS256.txt.minisig`
   (or `curl -fLO https://github.com/jdx/mise/releases/download/v<ver>/<file>` for each).
2. **Verify both signatures** with the keys mise's own documentation names (`docs/installing-mise.md`
   and `SECURITY.md`, "Release gpg key", in jdx/mise), never keys from the release itself; re-read
   them each time and stop if they changed:
   - GPG: import the release key `24853EC9F655CE80B48E6C3A8B81C9D17413A06D` (from
     `keys.openpgp.org`, or the block in `SECURITY.md`), check `gpg --fingerprint` shows exactly
     that, then `gpg --output signed.txt --decrypt SHASUMS256.asc` (the file is clearsigned) must
     report a good signature by that fingerprint;
   - minisign: `minisign -Vm SHASUMS256.txt -P RWTC3g8W3z4RZK3V3qv7fa1QY4JEWyBtqIHW+85QlJpZc5yG+uNYNBSZ`
     (`minisign.pub` in jdx/mise);
   - `cmp signed.txt SHASUMS256.txt` finds no difference.
3. **Keep two lines**: `grep -E '  (\./)?mise-v<ver>-linux-(x64|arm64)$' signed.txt`, with the `./`
   dropped, so each reads `<64 lowercase hex>  mise-v<ver>-linux-<arch>`. Archives, `-musl`
   builds and other platforms stay out.
4. **Cross-check** each hash against a second origin: `npm pack @jdxcode/mise-linux-x64@<ver>` and
   `@jdxcode/mise-linux-arm64@<ver>`, unpack each into an empty folder and `sha256sum` the `mise`
   binary inside. Both must equal step 3's lines.
5. **Record** the version, both hashes, the GPG fingerprint and minisign key used, the npm result,
   and who ran it where, in the PR.

Then write the two lines to `mise.sha256`, put the version in this README, and follow "Hash,
publish, pin" below. `scripts/tests/basic-image-hash.test.sh` fails while `mise.sha256` is not two
such lines of one version, while this README does not name that version, or while the
`Containerfile` downloads mise without checking it against `mise.sha256`.

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
