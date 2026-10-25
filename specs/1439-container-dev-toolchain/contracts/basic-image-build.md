# Contract: what the basic image provides for building (#1439)

Extends feature 806's basic-image contract
(`specs/806-container-daemon-ssh-provisioning/contracts/basic-image.md`) and assumes #1435's
`curl`, `gh` and `jq`. Requirements: FR-001 to FR-007, FR-011.

## B1. Commands on the default `PATH` of every session, before any setup

| Command | From | Requirement |
|---|---|---|
| `mise` | `/usr/local/bin/mise`, the release pinned in `packaging/basic/mise.sha256` | FR-001 |
| `cc`, `gcc`, `c++`, `g++`, `ld` | `gcc`, `g++` (binutils) | FR-003 |
| `make` | `make` | FR-003 |
| `pkg-config` | `pkgconf` | FR-003 |
| `python3` | `python3` | FR-004 |

`mise --version` prints the version named in `mise.sha256` (its first field after `mise-v`).
Headers: `libc6-dev`. Every row works as uid 1000, as an arbitrary host uid, and as uid 10001 with a
read-only root file system and a writable `/data/home` (Kubernetes, FR-006).

## B2. Libraries `pkg-config` finds (FR-007, provisional option A)

`pkg-config --exists` succeeds for `xkbcommon`, `wayland-client`, `x11`, `xcursor`, `xrandr`, `xi`
(packages `libxkbcommon-dev libwayland-dev libx11-dev libxcursor-dev libxrandr-dev libxi-dev`).
`pkg-config --libs x11` prints `-lX11` (AS4). These come from one `RUN` layer of their own; under
option B that layer and this section are removed.

## B3. The pinned mise release (FR-002, FR-013)

`packaging/basic/mise.sha256` is a `sha256sum` check file with exactly two lines:

```text
<64 hex>  mise-v<version>-linux-x64
<64 hex>  mise-v<version>-linux-arm64
```

Both lines name the same `<version>`. The `Containerfile`:

1. copies `mise.sha256` into the build;
2. downloads `https://github.com/jdx/mise/releases/download/v<version>/mise-v<version>-linux-<arch>`
   to a file, `<arch>` from `TARGETARCH` (`amd64` → `x64`, `arm64` → `arm64`; any other value fails
   the build);
3. runs `sha256sum -c --ignore-missing` on it, then checks the file exists (so a missing download
   cannot pass `--ignore-missing`); a mismatch, a missing file or a failed download fails the build;
4. installs it as `/usr/local/bin/mise`, mode 0755, and removes the download.

The version is read from `mise.sha256` in the `RUN` step; it is not a build `ARG`, so nothing at
build time can swap the binary without changing the hashed directory.

## B4. The guard (`scripts/tests/basic-image-hash.test.sh`)

Keeps every existing refusal (a `COPY`/`ADD` of a key, credential, `.ssh`, token or the daemon; any
`npm`/`npx`/`pnpm`/`yarn` install; any `curl|wget … | sh`). Adds:

- the real `Containerfile` downloads `mise` only together with a `sha256sum -c` of `mise.sha256`
  (a copy without the check fails the case);
- `mise.sha256` has the two lines of B3, the same version on both, 64 lowercase hex each;
- `packaging/basic/README.md` names that version (drift between pin and README fails).

## B5. Home folders (FR-011)

Before sshd starts, `entrypoint.sh` runs `mkdir -p "$HOME/.local/bin" "<shims>"`, where `<shims>` is
`$MISE_DATA_DIR/shims`, else `$XDG_DATA_HOME/mise/shims`, else `$HOME/.local/share/mise/shims`
(1289 contract toolchain-path P2; empty counts as unset). Existing folders and their contents are
untouched. When the home is not writable, one line on stderr and the entrypoint carries on.

## B6. Unchanged

No key, token, credential, daemon or AI CLI (806 FR-013); no `USER`; sshd on 2222; the
`micold` passwd entry for uid 10001; `MICOLD_DATA_DIR`; no `XDG_DATA_HOME`.
