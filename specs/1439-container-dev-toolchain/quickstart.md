# Quickstart: Container sessions can build a mise project (#1439)

Part A is automated and runs in `mise run gate` or `mise run test-sandbox`. Part B is run by hand
once per milestone that changes the image, and once at the end, with results recorded in the ledger.

## Prerequisites

- Docker or Podman; network access to Debian, GitHub releases, `static.rust-lang.org`, conda and
  crates.io.
- #1289 and #1435 merged into the branch.
- `mise run image` builds `micold-basic:dev` (and the daemon image) from the working tree.

## Part A — automated

| ID | Command | Proves |
|---|---|---|
| A1 | `scripts/tests/basic-image-hash.test.sh` | B3/B4: mise download checked against `mise.sha256`, pin file shape, README names the version, old refusals kept (FR-002, FR-013) |
| A2 | `scripts/tests/basic-entrypoint.test.sh` | B5: home folders created, existing contents untouched, unwritable home does not stop the entrypoint (FR-011) |
| A3 | `scripts/tests/mise-toolchain.test.sh` | `mise.toml` declares rust with `rustfmt`, `clippy`, `rust-analyzer`, and `uv` (FR-008) |
| A4 | `cargo test -p micold-daemon setup_turn` (in `mise run gate`) | S1–S3: one shell at a time on a sandbox daemon, order kept, stop/delete/daemon stop of a waiting run, stopping the holder lets the next waiting run spawn, host daemon not serialised |
| A5 | `mise run test-sandbox` → `sandbox_real_basic_image` | B1, B2 in the built image: `mise --version` = pinned version, `cc`/`c++`/`make`/`pkg-config`/`python3` present, a C file compiles and links, `pkg-config --libs x11` = `-lX11`, the six `.pc` files found; the same as uid 10001 with `--read-only` and a writable `/data/home` (FR-001, FR-003, FR-004, FR-006, FR-007, FR-010) |
| A6 | `mise run test-sandbox` → `sandbox_real_toolchain_path` | the two home folders exist in a new sandbox before any setup (FR-011, AS3.1); a `mise` in `~/.local/bin` wins over `/usr/local/bin/mise` on `PATH` (edge case) |
| A8 | `cargo test -p micold-core install_env` and `cargo test -p micold-daemon install_env` (in `mise run gate`) | contract sandbox-install-env I1–I2: the three defaults on a sandbox daemon, a value from environment-include or the daemon's own environment kept, lower-case `npm_config_prefix` counts, no `HOME` sets none, host daemon unchanged (FR-015) |
| A9 | `cargo test -p micold-core recreate_deletes_the_drifted_pod_and_creates_a_new_one_keeping_the_volume` (existing, `kube/start_tests.rs:388`; in `mise run gate`) | recreating a drifted pod deletes only the pod and adopts the claim (FR-016, Kubernetes half) |
| A10 | `mise run test-sandbox` → `sandbox_real_self_install` (new, needs network) | Story 5 with no setup command: in a session terminal `mise use -g uv@<pin>` and `mise use -g rust@<pin>` (minimal profile), `npm install -g` of a local tarball, `cargo install --path` of the A7 fixture crate, `uv tool install` of a small pinned package, `mise use -g go@<pin>` then `go install` of a local module the test writes (`go.mod`, `main.go`, no dependencies), `mise use -g python@<pin>` then `pip install --user` of a small pinned package; each as the non-root user, nothing written outside the sandbox home (FR-014, FR-017); a new session in a second worktree finds every binary under `~/.local/bin` or the shims (FR-015); after a container restart, and after the container is replaced by switching the daemon to a second tag of the same image (`DifferentImage`), every binary is found and runs; a last replacement onto the no-outbound network posture (a posture change also replaces the container) runs the same check offline, so nothing was downloaded again (FR-016, SC-006, AS5.3–AS5.4); `apt-get install` fails (AS5.6) |
| A7 | `mise run test-sandbox` → `sandbox_real_mise_setup` (new, needs network) | Story 1 on a fixture project (rust pinned, minimal profile; a `build.rs` that runs `cc` on a C file and links it, no crates.io dependency): setup `mise trust && mise install` succeeds; `cargo build`/`cargo test` succeed in a terminal; `command -v cargo` resolves under the sandbox home; after a container restart still found; a second worktree's setup ends in under 60 s and its output shows no download; a worktree with no `mise.toml` sets up; a setup stopped mid-download is followed by a rerun that succeeds (AS1.1–AS1.5, FR-005, SC-003; edge cases "Empty project", "Network failure during setup") |

## Part B — by hand

**B1. This repository's gate in a container (Story 2, FR-009, SC-001, SC-002).**

1. `mise run image`; in the app, add a container daemon with **My own image** set to
   `micold-basic:dev` (the image the basic image will be once published and pinned).
2. Set this repository's setup command to the one `CONTRIBUTING.md` documents
   (`mise trust && mise install`). Create a worktree on that daemon.
3. Expected: setup ends **Set up**. In a session terminal:
   `command -v cargo rust-analyzer uv python3 gh` prints five paths;
   `mise run gate` exits 0. Ask the session's `LSP` tool for `hover` on a Rust symbol: it answers
   (AS2.4). Record the gate's wall time and the setup's.

**B2. Concurrent setup (Principle II, contract S2).** On a fresh daemon, create two worktrees of
this repository within a few seconds. Expected: the second's output starts with
`Waiting for another setup on this daemon to finish…`, both end **Set up**, and
`rustup component list --installed` lists `rustfmt`, `clippy` and `rust-analyzer`. Run `cargo build` in a session of each worktree at the same time: both succeed (edge case "Several sessions on one daemon"; cargo's own locks guard the shared `~/.cargo` registry and each worktree has its own `target`). Press
**Stop setup** on a third, waiting run: it ends **Stopped** at once.

**B3. The user guide's own-image example (Story 4, FR-012, SC-005, AS4 second half).** First read
the "Building projects in the basic image" section of `docs/user-guide/sandboxed-daemon.md`:
expected, it lists what the image carries, what it does not carry (the limits), and a complete
own-image example. Then copy the example
from `docs/user-guide/sandboxed-daemon.md` into a folder and run `docker build` (and
`podman build`) exactly as written. Expected: both build; a container daemon on the result starts
sessions, and `pkg-config --libs libpq` prints `-lpq` there.

**B4. A failed checksum fails the build (FR-002).** Change one hex digit in a copy of
`packaging/basic/mise.sha256` and build that copy. Expected: the build stops at the mise step with
`FAILED` from `sha256sum`; no image is tagged.

**B5. Size (SC-004).** For each architecture, the compressed size of the image from the PR's
`basic-image.yml` run, or `docker save micold-basic:dev | gzip -c | wc -c` minus the same for the
previous pin. Expected: growth ≤ 250 MB; record both numbers in the ledger and the README.

**B6. Kubernetes (edge case, FR-006).** With the 686 setup (kind or a real cluster), a pod daemon on
the dev image: `mise --version`, `cc --version`, `python3 --version` work, and setup installs into
`/data/home`. Then (AS5.5) a session runs `mise use -g uv@<pin>` and `npm install -g` of a small
package; stop and start the daemon (the pod is recreated): both binaries are found and run.

**B7. A session's own installs on a VM-shared home (Story 5, Principle VI).** On a macOS host and on
a Windows host (Docker Desktop or Podman machine), a container daemon on `micold-basic:dev` with no
setup command: a session runs `mise use -g uv@<pin>` and `npm install -g` of a small package; both
are found in a new session; restart the container: both still run. Record the host and runtime in
the ledger.
