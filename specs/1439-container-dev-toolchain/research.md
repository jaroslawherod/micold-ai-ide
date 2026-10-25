# Research: Container sessions can build a mise project (#1439)

Each decision: what was chosen, why, and what was rejected. Probes ran in this cloud container on
2026-10-25; their scripts and logs are not committed (session scratchpad).

## R1. How `mise` gets into the image

- **Decision**: download the standalone glibc binary of a pinned mise release
  (`mise-v<version>-linux-x64` / `mise-v<version>-linux-arm64`) from the release's GitHub assets in a
  `RUN` step, and check it with `sha256sum -c` against `packaging/basic/mise.sha256` (copied into the
  build stage first) before it is moved to `/usr/local/bin/mise`. The architecture comes from
  BuildKit's `TARGETARCH` (`amd64` → `x64`, `arm64` → `arm64`). `curl` comes from #1435's package
  list.
- **Rationale**: one file, no installer script, and the checksum file is a plain `sha256sum` file in
  the hashed directory, so a version bump changes the image's content hash and the checksum in the
  same diff (FR-002, FR-013). The guard test already refuses `curl … | sh`; a download to a file
  followed by `sha256sum -c` is not a pipe into a shell.
- **Alternatives rejected**:
  - `curl https://mise.run | sh`: a piped installer, which the guard refuses (806 FR-013) and which
    pins nothing.
  - Debian's or mise's apt repository: needs a signing key added to the image (a key in the image is
    what the guard exists to stop) and floats with the repository.
  - `ADD --checksum=sha256:…`: one checksum per `ADD`, so two architectures need a stage per
    architecture; also needs a newer Dockerfile frontend than Podman/buildah users may have.
  - The `-musl` build: also fine; the glibc build is what mise's own docs install on Debian, and
    trixie's glibc 2.41 is guaranteed by the base image.
  - npm `@jdxcode/mise`: an `npm install` in the image, which the guard refuses by design.
- **Size**: the npm platform package of mise 2026.10.19 (the same binary) is 61 MB gzip-compressed
  (173 MB unpacked), measured here. Counted in R6.
- **Risk for the implementing unit**: this cloud container cannot reach `github.com` release
  downloads or `mise.jdx.dev` (proxy 403). The pin is therefore taken by plan D1a on a machine or
  `workflow_dispatch` job that reaches GitHub: the release's `SHASUMS256.txt` with its clearsigned
  GPG copy (`SHASUMS256.asc`) and minisign signature verified against the keys mise's docs publish, only the two raw `linux-x64` /
  `linux-arm64` lines kept, each hash cross-checked against the binary in the npm
  `@jdxcode/mise-linux-{x64,arm64}` package (reachable here), provenance recorded in the ledger.
  The CI image build only shows the download matches the committed hash; it is not the source of
  the hash.

## R2. Which compiler packages

- **Decision**: `gcc`, `g++`, `make`, `libc6-dev`, `pkgconf` (trixie's `pkg-config`
  implementation; it provides the `pkg-config` command). `gcc` provides `cc`, which rustc uses as its
  linker.
- **Rationale**: FR-003 asks for a C/C++ compiler, linker, libc headers, `make` and `pkg-config`.
  `build-essential` adds `dpkg-dev` and with it Perl's full library, which no build here needs.
- **Alternatives rejected**: `build-essential` (larger, for Debian packaging); `clang` (larger,
  and Rust's default linker driver on Linux is `cc`).

## R3. `python3`

- **Decision**: Debian's `python3` (3.13 in trixie), no `pip`, no `venv` package.
- **Rationale**: FR-004; the repository's hooks and `scripts/autopilot/*.py` use the standard
  library. Projects that need packages use `uv` through mise.
- **Alternatives rejected**: `uv python install` in setup (not present before setup, and the hooks
  need it before any setup, AS6); `python3-full` (size).

## R4. The development libraries of FR-007 (provisional option A)

- **Decision**: the six packages CI installs for its Linux test job (`ci.yml:270-272`):
  `libxkbcommon-dev libwayland-dev libx11-dev libxcursor-dev libxrandr-dev libxi-dev`, in a
  **separate `RUN` layer** after the toolchain layer, under its own comment naming FR-007.
- **Rationale**: the spec's provisional answer (ledger D3). Kept as one layer, one README paragraph,
  one test constant and one milestone (M2), so that if the user picks option B (own image), the
  change is: drop that layer, drop the constant, and point AS4's fixture at a test-built `.pc` file
  (R7). Nothing else depends on it.
- **Alternatives rejected** (by the spec's clarify round, recorded here for the plan): a published
  "dev" image variant (a second image to hash, publish and pin, and a selector in the app); an own
  image only (the issue's acceptance says "on the default image").

## R5. Concurrent `mise install` into one sandbox home (spec assumption, Principle II)

- **Probe**: rustup 1.28.2 with one fresh `RUSTUP_HOME`/`CARGO_HOME`, `--profile minimal`:
  1. the same toolchain (`1.86.0`) installed twice at once: one run **failed** (`component download
     failed for cargo … No such file or directory`, exit 1); the other succeeded;
  2. two different toolchains (`1.87.0`, `1.88.0`) at once: both succeeded;
  3. `rustfmt` and `clippy` added to the same toolchain at once: both exited 0, but
     `rustup component list --installed` afterwards listed `rustfmt` and **not `clippy`**, while
     clippy's binaries were on disk — a lost update of rustup's component manifest.
- **mise**: takes a lock per tool-version install folder, so two `mise install` of the same
  `rust@stable` serialise inside mise. That could not be run here: mise resolves versions through
  `mise-versions.jdx.dev`/the GitHub API, both blocked from this container. It would not cover a
  second project pinning `rust@1.97.1` that adds components to a toolchain the first is changing,
  a session's `cargo` auto-installing a `rust-toolchain.toml` toolchain while setup runs, nor other
  installers in a setup command (`npm ci`, `uv sync`) that share a cache in the home.
- **Decision**: the assumption does not hold for rustup, so **the daemon serialises setup runs on an
  app-owned sandbox daemon** (container or pod, the same `offered.json` predicate 1289 uses for the
  `PATH`): one setup shell at a time per daemon, the rest wait their turn in order. Host and remote
  daemons keep 1289's behaviour (no serialisation; out of this spec's scope). Contract:
  [contracts/setup-turn.md](contracts/setup-turn.md).
- **Alternatives rejected**:
  - Rely on mise's lock: not verifiable here, and rustup's own races (probe 1 and 3) are reachable
    through mise when two projects or components meet on one toolchain.
  - Ask users to write `flock ~/.cache/… sh -c 'mise trust && mise install'`: every user has to
    know it, and the repository's documented command would differ from the user guide's.
  - A wrapper `mise` in the image that takes a lock: changes mise's behaviour for sessions too, and a
    `mise` in `~/.local/bin` bypasses it.
  - A new `Queued` setup state: a protocol and UI change for a wait that is normally seconds to
    minutes; the run is `Running` with a first output line saying it waits (S2), which the output
    view already shows.
- **Collision**: 1289's spec says "the app does not serialise runs" (1289 spec.md:158, :338). This
  feature narrows that for sandbox daemons; the user-guide sentence 1289 adds is updated in the same
  change (M3).

## R6. Image size (SC-004, budget 250 MB compressed per architecture)

| Layer | Estimate (compressed) | Source |
|---|---|---|
| mise binary | ~61 MB | measured: npm package of the same release |
| gcc, g++, make, libc6-dev, pkgconf | ~75 MB | Debian trixie package sizes (`Installed-Size` ≈ 260 MB) |
| python3 | ~12 MB | trixie `python3.13-minimal` + `libpython3.13-stdlib` |
| six X11/Wayland dev packages (FR-007) | ~8 MB | trixie package sizes |
| **Total** | **~156 MB** | under budget with ~90 MB margin |

- **Decision**: measure the real number in M1 and M2 from the image the PR's `basic-image.yml` run
  builds (or `docker save … | gzip -c | wc -c` locally) and record it in the ledger and the README;
  over 250 MB is a review blocker.

## R7. AS4's fixture library (spec review round 2, MINOR)

- **Decision**: `x11` — `pkg-config --libs x11` prints `-lX11` in the basic image (from
  `libx11-dev`, FR-007). The "own image built `FROM` it" half of AS4 is the user guide's own-image
  example (FR-012), which adds `libpq-dev`; quickstart B3 builds it and runs
  `pkg-config --libs libpq`.
- **If FR-007 becomes option B**: the real-runtime test writes a `fixture.pc` into a temp folder and
  runs `PKG_CONFIG_PATH=<folder> pkg-config --libs fixture`, which needs nothing installed; the
  own-image example then also adds the six packages.
- **Alternatives rejected**: `zlib` (`zlib1g-dev` is not in the image under either option, so it
  would add a package only for the test).

## R8. The Rust components this repository needs (FR-008)

- **Decision**: `mise.toml`'s `rust` entry becomes
  `rust = { version = "stable", components = "rustfmt,clippy,rust-analyzer" }`. mise's core rust
  backend passes `components` to rustup when it installs the toolchain, and puts `CARGO_HOME/bin`
  (rustup's proxies, `rust-analyzer` among them) behind its shims. `rust-toolchain.toml` stays
  `channel = "stable"` only.
- **Probe**: mise 2026.10.19 parses that entry here without error (`mise ls --current`); the install
  itself could not run (R5).
- **Rationale**: FR-008 says "through `mise.toml`"; the gate runs `cargo fmt` and `cargo clippy`,
  and the `LSP` tool needs `rust-analyzer`.
- **Alternatives rejected**: `components` in `rust-toolchain.toml` — every bare `cargo` in CI and on
  every developer machine would then download `rust-analyzer` too, which CI never uses.

## R9. `ffmpeg = "8"` without root

- **Probe**: mise 2026.10.19 resolves `ffmpeg = "8"` to `8.1.2` through its registry's first
  backend, `conda:ffmpeg`, with no `experimental` setting (`mise ls --current` here). conda packages
  are prebuilt and install into the home, no root (spec assumption holds as far as resolution; the
  install is proved by quickstart B2).
- **Alternatives rejected**: `ffmpeg` in the image (spec: no project toolchain in the image).

## R10. Making the `PATH` folders exist (FR-011)

- **Decision**: `packaging/basic/entrypoint.sh` creates `$HOME/.local/bin` and the shims folder
  (`$MISE_DATA_DIR/shims`, else `$XDG_DATA_HOME/mise/shims`, else `$HOME/.local/share/mise/shims`,
  the same rule as 1289's contract toolchain-path P2) with `mkdir -p`, after it has set `HOME` and
  before sshd starts. A failure (unwritable home) is reported on stderr and does not stop the
  entrypoint.
- **Rationale**: the entrypoint runs before the first session on both container daemons and
  Kubernetes pods (the pod spec sets no `command`, `kube/manifest.rs:468`), as the session user,
  with the home already mounted. `mkdir -p` never touches what is inside an existing folder (AS3.2).
- **Alternatives rejected**: creating them in the image (`/home/…` is replaced by the mounted
  sandbox home); creating them in the client's provisioning over SSH (a second place, and a pod
  restarted by Kubernetes is not provisioned again).

## R11. What runs before #1289 and #1435 land

- **Decision**: implementation starts only after both have merged; the design PR may merge before.
  This plan assumes their changes are present: 1289's `toolchain_path`, `with_sandbox_path` and
  setup runner (`crates/micold-daemon/src/setup.rs`, M3 merged, M4 open as #1407), and #1435's
  `curl`, `gh` and `jq` in the basic image's package list.
- **Files they also change** (expect rebase conflicts; take both sides):

  | File | #1435 | #1289 | This feature |
  |---|---|---|---|
  | `packaging/basic/Containerfile` | apt list (`curl`, `jq`), `gh` install | — | apt list, mise step, FR-007 layer |
  | `packaging/basic/README.md` | carries list | — | carries list, mise pin, update steps |
  | `crates/micold-core/src/sandbox/basic_image.rs` | `BASIC_IMAGE_CONTENT_HASH` (+ pin PR) | — | `BASIC_IMAGE_CONTENT_HASH` (+ pin PR) |
  | `crates/micold-core/tests/sandbox_real_basic_image.rs` | `PRESENT` | — | `PRESENT`, new tests |
  | `scripts/tests/basic-image-hash.test.sh` | possibly (gh install allowance) | — | mise cases |
  | `docs/user-guide/sandboxed-daemon.md` | image contents | PATH, setup pointer, own image | build section |
  | `crates/micold-daemon/src/setup.rs`, `state.rs` | — | M4 (#1407) and later | setup turn |
  | `docs/user-guide/worktrees-and-sessions.md` | — | setup docs (M5–M7) | one sentence on serialisation |

  The hash is a function of the whole directory, so whichever lands second recomputes
  `BASIC_IMAGE_CONTENT_HASH`; the publish-and-pin follow-up (806) runs once after this feature's
  image PR merges.
