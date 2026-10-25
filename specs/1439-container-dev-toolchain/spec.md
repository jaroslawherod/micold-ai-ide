# Feature Specification: Container sessions can build a mise project

**Feature Branch**: `claude/project-thread-xudko5`

**Created**: 2026-10-25

**Status**: Draft

**Input**: User description: "Container sessions cannot build the project: basic image has no mise /
compiler toolchain / python3 and no way to install them (issue #1439). An AI session in a container
daemon on the basic image cannot build or verify this repository, and it cannot install what it
needs. `mise.toml` declares `rust = "stable"`, `uv`, `node = "24"` and `ffmpeg = "8"`; in the
container only `git`, `git-lfs`, `ssh`, `bash` and `node` v24 exist. Absent: `mise`, `cargo`,
`rustc`, `rustup`, `rust-analyzer`, `uv`, `python3`, `ffmpeg`, a C toolchain/linker (and `gh`,
`curl`, `wget`, `jq`, which #1435 covers). The session runs as `uid=1000(node)` with no `sudo`, an
unwritable `/usr/local/bin` and no root for `apt`. So `mise run gate` cannot run, the `LSP` tool has
no `rust-analyzer`, and `scripts/autopilot/*.py` and the repo's hooks have no `python3`. The
`~/.local/bin` and mise shims folders are on the session `PATH` (feature 1289) but do not exist, so
`mise` looks unconfigured rather than absent. Preferred: ship `mise` in the basic image (one static
binary from a pinned release, checksum-verified, allowed by the hash test) plus
`build-essential`/`pkg-config`, so a project's setup command (#1289), e.g. `mise trust && mise
install`, installs the toolchain into the persistent sandbox home. Alternatives: a 'dev' image
variant selectable per daemon or project; or document the gap with a worked own-image example. Also
make the shims and `~/.local/bin` folders exist, so `command -v` reflects reality. Acceptance: in a
new container daemon on the default image, with this repo's setup command, a session can run `mise
run gate` to completion, and `command -v cargo rust-analyzer uv python3 gh` all resolve. Related:
#1289, #1435, #1438."

## Context and dependencies

This feature builds on two others and does not respecify them:

- **#1289** (per-project setup command and toolchain `PATH`): a project's setup command runs in
  each new worktree on its daemon, and sessions on container and Kubernetes daemons get the sandbox
  home's `~/.local/bin` and mise shims folders on `PATH`.
- **#1435**: the basic image carries `gh`, `curl` and `jq`.

Both are assumed to have landed before this feature is built.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A mise project's toolchain installs itself in a new container daemon (Priority: P1)

A user adds a container daemon on the default (basic) image and sets their project's setup command
to `mise trust && mise install`. When they create a worktree on that daemon, setup installs every
tool the project's `mise.toml` declares into the sandbox home, and a session in that worktree can
build and test the project with no hand-installation, no root and no change of image.

**Why this priority**: This is the issue. Today a session on the basic image cannot build or
verify any Rust project, and it has no way to fix that itself, so an AI session stops instead of
working.

**Independent Test**: Start a fresh container daemon on the basic image built from the branch. Set
the setup command of a small fixture project, whose `mise.toml` declares a Rust toolchain and whose
crate links a C dependency through `pkg-config`, to `mise trust && mise install`. Create a worktree;
setup succeeds; in a session there, `cargo build` and `cargo test` succeed.

**Acceptance Scenarios**:

1. **Given** a new container daemon on the basic image, **When** a session runs `command -v mise`,
   **Then** it resolves, and `mise --version` prints the version the image pins.
2. **Given** a project whose setup command is `mise trust && mise install`, **When** a worktree is
   created on a new container daemon on the basic image, **Then** setup succeeds and installs every
   tool the project's `mise.toml` declares into the sandbox home.
3. **Given** setup has succeeded, **When** a session builds a Rust crate that compiles C code or
   links a system library found through `pkg-config`, **Then** compiling and linking succeed.
4. **Given** setup has succeeded once on a daemon, **When** the container restarts or a second
   worktree of the same project is created on that daemon, **Then** the installed tools are still
   found and the second setup does not download them again.
5. **Given** a new container daemon on the basic image, before any setup has run, **When** a
   session or one of the repository's hooks runs a `python3` script, **Then** `python3` is found
   and runs it.

---

### User Story 2 - This repository's gate runs in a container session (Priority: P1)

A user (or an AI session running `/speckit-autopilot`) works on this repository in a container
daemon on the default image, with the repository's documented setup command. The session can run
the project's full merge gate (`mise run gate`) to completion, and the tools the repository's
instructions rely on — `cargo`, `rust-analyzer`, `uv`, `python3`, `gh` — are all found.

**Why this priority**: It is the issue's acceptance, and the case that stopped the #1403 run. It
proves the general route of Story 1 works for the most demanding project the app is used on.

**Independent Test**: In a new container daemon on the basic image built from the branch, create a
worktree of this repository with its documented setup command. After setup succeeds, a session
runs `command -v cargo rust-analyzer uv python3 gh` (all five resolve) and then `mise run gate`,
which finishes with exit status 0.

**Acceptance Scenarios**:

1. **Given** a worktree of this repository on a new container daemon on the basic image, with the
   setup command the repository documents, **When** setup finishes, **Then** it has succeeded.
2. **Given** setup has succeeded, **When** a session runs `command -v cargo rust-analyzer uv
   python3 gh`, **Then** each of the five resolves.
3. **Given** setup has succeeded, **When** a session runs `mise run gate`, **Then** every step (fmt
   check, clippy, the workspace tests, the shell tests) runs and the gate exits 0 on a clean
   checkout of `main`.
4. **Given** setup has succeeded, **When** a session's `LSP` tool asks rust-analyzer about a Rust
   symbol in the worktree, **Then** rust-analyzer starts and answers.

---

### User Story 3 - What a session sees on PATH is what exists (Priority: P2)

An AI session that probes its environment (`command -v`, `ls` of the `PATH` folders) gets a true
picture: the home toolchain folders that #1289 puts on `PATH` exist, so an absent tool reads as
absent, and a present-but-unconfigured mise reads as such.

**Why this priority**: The misleading `PATH` sent the #1403 agent down the wrong diagnosis. Smaller
than Stories 1 and 2, and independent of them.

**Independent Test**: Start a new container daemon on the basic image; before any setup, a session
lists `~/.local/bin` and the mise shims folder of the sandbox home: both exist (empty).

**Acceptance Scenarios**:

1. **Given** a new container daemon, **When** the first session starts, **Then** `~/.local/bin` and
   the mise shims folder that #1289 puts on `PATH` both exist in the sandbox home.
2. **Given** a sandbox home that already holds files in either folder, **When** the container
   restarts, **Then** their contents are unchanged.

---

### User Story 4 - The user guide says what the basic image can build (Priority: P2)

A user deciding how to run a project in a container daemon reads the sandbox page and learns what
the basic image provides for building projects, how a mise project gets its toolchain (the setup
command), what still needs an own image, and a worked example of an own image `FROM` the basic
image for a project that needs extra system packages.

**Why this priority**: Without it the route of Story 1 is undiscoverable, and the projects it does
not cover have no documented way forward.

**Independent Test**: Read the sandbox page: it names the build tools the basic image carries, gives
`mise trust && mise install` as the setup command for a mise project, and has a complete own-image
example that builds with `docker build` or `podman build` as given.

**Acceptance Scenarios**:

1. **Given** the sandbox page, **When** a user looks up how to build a mise project in a container
   daemon, **Then** it names the setup command and what the basic image provides.
2. **Given** a project that needs a system package the basic image does not carry, **When** the
   user follows the page's own-image example, **Then** the example image builds as written and the
   resulting daemon runs sessions.

---

### Edge Cases

- **Empty project / no `mise.toml`**: `mise install` with nothing declared succeeds and installs
  nothing; the image's own tools (`python3`, the C toolchain, `mise`) still work.
- **Network failure during setup**: setup fails as #1289 reports it; a partly installed toolchain
  does not break the next run, which resumes or reinstalls the missing tools.
- **Download of `mise` at image build fails or its checksum does not match**: the image build
  fails; no image is published with an unverified `mise`.
- **Concurrent setup** (Principle II): two worktrees of the same or different projects on one daemon
  run `mise install` at the same time into the shared sandbox home; both succeed, and neither
  leaves the other's tools broken (mise serialises writes to its own install folders).
- **Several sessions on one daemon** use the same toolchain from the shared home at once; one
  session's build does not disturb another's.
- **Kubernetes daemons** (feature 686) start from the same basic image with a read-only root file
  system: `mise`, the C toolchain and `python3` work there, and the toolchain installs into the
  pod's home volume.
- **Architectures and host platforms** (Principle VI): the basic image is published for
  `linux/amd64` and `linux/arm64`; `mise` and the build tools are present and work on both, so a
  daemon on an Apple-silicon Mac, on Windows (Docker/Podman) and on Linux behaves the same.
- **A project that needs system libraries the basic image does not carry** (for example the
  X11/Wayland development libraries this repository's GUI crates link against): see FR-007.
- **An own image** (**My own image**) is not changed by this feature; an own image built `FROM` the
  basic image inherits `mise` and the build tools.
- **A sandbox home created before this feature**, holding tools a user installed by hand: they keep
  working, and the folders of Story 3 are created around them without touching existing files.
- **A `mise` already in `~/.local/bin`** (installed by hand earlier): it comes first on `PATH`
  (#1289's order) and is used; the image's copy is the fallback.
- **Image size**: the first start downloads a larger image; later starts are unaffected (cached).

## Requirements *(mandatory)*

### Functional Requirements

**Toolchain manager and build tools in the basic image (Story 1)**

- **FR-001**: The basic image MUST provide `mise` on the default `PATH` of every session, at a
  version the repository pins, for both published architectures.
- **FR-002**: The `mise` the image ships MUST be verified against a checksum pinned in the
  repository when the image is built; a mismatch or failed download MUST fail the build. The
  image's guard test (`scripts/tests/basic-image-hash.test.sh`) MUST keep refusing keys, tokens,
  the daemon, AI CLIs and piped-script installs, and MUST accept this way of adding `mise`.
- **FR-003**: The basic image MUST provide a C/C++ compiler, linker, `make` and `pkg-config`, so a
  Rust toolchain installed by mise can compile and link crates with C code or system libraries
  found through `pkg-config`.
- **FR-004**: The basic image MUST provide `python3` on the default `PATH`, usable before any setup
  has run.
- **FR-005**: Tools that a setup command installs with `mise` MUST land in the sandbox home, so they
  persist across container restarts and are shared by every worktree on that daemon, and MUST be
  found by sessions through #1289's `PATH` without further configuration.
- **FR-006**: The basic image MUST still carry no key, token, credential, daemon or AI CLI
  (feature 806, FR-013), and MUST keep working as the base for **My own image** and for Kubernetes
  daemons.
- **FR-007**: For system libraries a project needs beyond FR-003 (this repository's GUI crates link
  X11/Wayland development libraries): [NEEDS CLARIFICATION: should the basic image also carry the
  development libraries this repository's gate needs (making the default image build this repo, as
  the issue's acceptance asks), or should project-specific system libraries come from an own image
  or a separate "dev" image variant, with this repository's own container setup documented to use
  it?]

**This repository builds in a container (Story 2)**

- **FR-008**: This repository MUST declare every tool its gate and its instructions need through
  `mise.toml`, including the Rust components the gate and the `LSP` tool use (formatter, linter,
  rust-analyzer), so `mise install` alone provides them.
- **FR-009**: The repository MUST document its container setup command, and with it, on a new
  container daemon on the image FR-007 settles on, setup MUST succeed, `command -v cargo
  rust-analyzer uv python3 gh` MUST resolve all five, and `mise run gate` MUST exit 0 on a clean
  checkout of `main`.
- **FR-010**: Installing this repository's toolchain in a container session MUST NOT require root,
  `sudo` or writing outside the sandbox home and the worktree.

**PATH reflects reality (Story 3)**

- **FR-011**: Before the first session on a container or Kubernetes daemon starts, the sandbox home
  MUST contain `~/.local/bin` and the mise shims folder that #1289 puts on `PATH`; creating them
  MUST NOT change any file already in them.

**Documentation (Story 4)**

- **FR-012**: The sandbox page of the user guide MUST say what the basic image carries for building
  projects (`mise`, the C toolchain, `pkg-config`, `python3`), that a mise project gets its
  toolchain through the setup command `mise trust && mise install`, what it does not carry, and
  give a complete own-image example `FROM` the basic image that adds system packages.
- **FR-013**: `packaging/basic/README.md` MUST list the new contents, the pinned `mise` version and
  checksum, and how to update them.

### Key Entities

- **Pinned `mise` release**: a version and one checksum per architecture, kept in the repository and
  checked when the basic image is built.
- **Sandbox home toolchain**: the tools mise installs under the sandbox home; persistent per daemon
  and shared by its worktrees.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: On a new container daemon on the default image, a session in a worktree of this
  repository, after its documented setup command succeeds, completes the full merge gate with zero
  manual install steps.
- **SC-002**: The five tools `cargo`, `rust-analyzer`, `uv`, `python3` and `gh` resolve in 100% of
  sessions on such a daemon after setup has succeeded.
- **SC-003**: A second worktree of the same project on the same daemon finishes setup without
  downloading the toolchain again, in under 1 minute on the same machine.
- **SC-004**: The basic image's compressed download grows by no more than 250 MB per architecture.
- **SC-005**: A user who reads the sandbox page can tell, without trying it, whether the basic image
  can build their project and, if not, has a working own-image example to start from.

## Out of scope

- The per-project setup command, its UI and the toolchain `PATH` (#1289).
- `gh`, `curl`, `wget` and `jq` in the basic image (#1435).
- The session environment issue #1438.
- Installing any project's toolchain (Rust, uv, node, ffmpeg) into the image itself: it installs
  per daemon through setup.
- A per-project image choice in the app (deferred by #1289; a second container daemon covers it).
- Host and remote (SSH) daemons, which use the user's own environment.
- Caching toolchain downloads across different daemons.

## Assumptions

- `mise` is distributed as one self-contained binary per architecture with published checksums, so
  it can be added to the image without a piped installer.
- mise installs Rust through rustup into the home, and the Rust components (formatter, linter,
  rust-analyzer) can be declared in `mise.toml`.
- `uv` installed by mise does not provide `python3` on `PATH` by default, so the image ships
  `python3` (also needed by the repository's hooks before any setup runs).
- `ffmpeg` installs through mise's conda backend without root, as `mise.toml` already says.
- The sandbox home is persistent per daemon and writable by the session user, including on
  Kubernetes (the pod's home volume).
- A new basic image reaches users through the existing hash, publish and pin steps (feature 806);
  existing containers move to it on their next restart.
- The 250 MB size budget is a starting estimate for a Debian compiler toolchain plus `python3`; the
  plan measures it.
