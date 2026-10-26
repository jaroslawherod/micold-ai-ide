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

## Clarifications

### Session 2026-10-25

- Q: Where do the X11/Wayland development libraries this repository's gate links come from — the
  basic image, an own image, or a "dev" image variant? → A: The basic image carries them (FR-007);
  other projects' extra system libraries come from an own image. _(provisional: recommended option
  A, awaiting user decision)_

### Amendment 2026-10-26 (repository owner, on #1439)

- Requirement, verbatim: "as part of image push enough tools that agent can install them on it's
  own. It shoould have persistentn space so they will not disapear after container restart" — "I
  mean that it cann install tools needed to build project".
- Read as (confirmed back to the owner): an AI session in a container session, with **no** setup
  command configured, can install the user-level tools the project's build needs by itself (mise,
  and each language's own installer: `npm install -g`, `cargo install`, `uv tool install`, `go
  install`, `pip install --user` under a mise-installed Python), without root or `sudo`; the
  installs land in the persistent sandbox home, are on `PATH` for later sessions, and survive a
  container restart and the container's replacement on an image update. System packages (`apt`)
  stay out of reach: they come from the basic image or an own image built `FROM` it. → User Story 5,
  FR-005 widened, FR-014 to FR-018, SC-006.

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
crate compiles a C source file through a build script, to `mise trust && mise install`. Create a worktree;
setup succeeds; in a session there, `cargo build` and `cargo test` succeed.

**Acceptance Scenarios**:

1. **Given** a new container daemon on the basic image, **When** a session runs `command -v mise`,
   **Then** it resolves, and `mise --version` prints the version the image pins.
2. **Given** a project whose setup command is `mise trust && mise install`, **When** a worktree is
   created on a new container daemon on the basic image, **Then** setup succeeds and installs every
   tool the project's `mise.toml` declares into the sandbox home.
3. **Given** setup has succeeded, **When** a session builds a Rust crate whose build script
   compiles C code, **Then** compiling and linking succeed.
4. **Given** a library with a `pkg-config` file that the image (or an own image built `FROM` it)
   provides, **When** a session runs `pkg-config --libs` for it, **Then** `pkg-config` finds it.
5. **Given** setup has succeeded once on a daemon, **When** the container restarts or a second
   worktree of the same project is created on that daemon, **Then** the installed tools are still
   found and the second setup does not download them again.
6. **Given** a new container daemon on the basic image, before any setup has run, **When** a
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
   checkout of the branch that carries this feature (and of `main` after merge).
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

### User Story 5 - An AI session installs the build tools it needs itself, and they stay (Priority: P1)

A user adds a container daemon on the basic image and configures **no** setup command. An AI session
in a worktree finds that the project's build needs a tool the image does not carry (a Rust
toolchain, `uv`, a `cargo`- or `npm`-distributed helper). It installs the tool itself, as the
session user, with mise or with the language's own installer. The tool is found by every later
session on that daemon, in any worktree, and it is still there after the container restarts and
after the container is replaced because the basic image was updated. Only system packages are out
of reach, and the user guide says where they come from instead.

**Why this priority**: The repository owner's requirement. A setup command is the planned route,
but an agent working in a project nobody configured must not stop at "command not found", and what
it installs must not vanish at the next restart or image update and cost another download.

**Independent Test**: Start a container daemon on the basic image built from the branch, with no
setup command. In a session terminal, install a Rust toolchain and `uv` with `mise use -g`, a
package with `npm install -g` and a crate with `cargo install`. Each succeeds without root, and a new
session in a second worktree finds each binary. Restart the container, then replace it by switching
the daemon to another tag of the image: every binary is still found and runs, and nothing is
downloaded again.

**Acceptance Scenarios**:

1. **Given** a container daemon on the basic image with no setup command, **When** a session runs
   `mise use -g` for a tool (for example `rust` or `uv`), **Then** it succeeds as the session user,
   the tool is installed under the sandbox home, and a new session on any worktree of the daemon
   finds it with `command -v`.
2. **Given** such a daemon, **When** a session runs `npm install -g <package>`, `cargo install
   <crate>` (with a mise-installed Rust) or `uv tool install <package>` (with a mise-installed
   `uv`), **Then** each succeeds without root or `sudo`, writes only under the sandbox home, and
   the binary it creates is found by a new session with no further configuration.
3. **Given** tools a session installed itself, **When** the container restarts, **Then** each is
   still found and runs, and none is downloaded again.
4. **Given** tools a session installed itself, **When** the container is replaced (the daemon moves
   to an updated basic image, or another of its settings forces a new container), **Then** each is
   still found and runs, and none is downloaded again.
5. **Given** a Kubernetes daemon, **When** its pod is recreated, **Then** tools a session installed
   into `/data/home` are still found and run.
6. **Given** a session that needs a system package (for example `apt-get install libpq-dev`),
   **When** it tries to install it, **Then** it fails for lack of root, as today, and the sandbox
   page tells the user to add it through an own image built `FROM` the basic image.

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
  leaves the other's tools broken (whether mise and rustup already guarantee this is an assumption
  the plan verifies; if they do not, setup must serialise installs per daemon).
- **Several sessions on one daemon** use the same toolchain from the shared home at once; one
  session's build does not disturb another's.
- **Kubernetes daemons** (feature 686) start from the same basic image with a read-only root file
  system: `mise`, the C toolchain and `python3` work there, and the toolchain installs into the
  pod's home volume.
- **Architectures and host platforms** (Principle VI): the basic image is published for
  `linux/amd64` and `linux/arm64`; `mise` and the build tools are present and work on both, so a
  daemon on an Apple-silicon Mac, on Windows (Docker/Podman) and on Linux behaves the same.
- **A project that needs system libraries the basic image does not carry** (anything beyond
  FR-003 and FR-007's set, for example a database client library): the user builds an own image `FROM` the basic image that adds them (FR-012).
- **An own image** (**My own image**) is not changed by this feature; an own image built `FROM` the
  basic image inherits `mise` and the build tools.
- **A sandbox home created before this feature**, holding tools a user installed by hand: they keep
  working, and the folders of Story 3 are created around them without touching existing files.
- **A `mise` already in `~/.local/bin`** (installed by hand earlier): it comes first on `PATH`
  (#1289's order) and is used; the image's copy is the fallback.
- **Image size**: the first start downloads a larger image; later starts are unaffected (cached).
- **A session installs while setup or another session installs** (Story 5): the setup turn
  serialises setup runs only; installs sessions start themselves are not serialised. Two installs
  of the same Rust toolchain at once can fail one of them (research R5); rerunning the failed
  command succeeds. A lost component update is repaired by rerunning `rustup component add`.
- **The environment already names an install location** (an environment-include script or the
  daemon's own environment sets `NPM_CONFIG_PREFIX`, `CARGO_INSTALL_ROOT` or `GOBIN`): that value is
  kept; the defaults of FR-015 apply only where nothing is set.
- **An image update changes a system library** that a tool installed in the home links against (a
  `cargo install` of a crate linking OpenSSL, an `npm` package with a native module): that tool can
  stop working after the update and is reinstalled. Tools mise installs as prebuilt binaries, and
  Rust toolchains, do not depend on the image's libraries beyond glibc.
- **An AI CLI updates itself through npm**: with `NPM_CONFIG_PREFIX=$HOME/.local` (FR-015), an AI
  CLI's own `npm`-based self-update can now succeed and leave a second copy in `~/.local/bin`. This
  is accepted as harmless: the offered CLI's folder stays ahead of `~/.local/bin` on `PATH` (#1289),
  so the offered copy still runs, and the daemon reports only offered CLIs.
- **A daemon whose network posture blocks outbound connections**: a session there cannot download
  anything, so it cannot install tools itself; tools already in the home keep working.
- **Hosts whose container runtime shares folders through a VM** (macOS, Windows): the sandbox home
  is a host folder shared into the container; mise's shims and rustup's proxies (links and
  executables) work there as on Linux.

## Requirements *(mandatory)*

### Functional Requirements

**Toolchain manager and build tools in the basic image (Story 1)**

- **FR-001**: The basic image MUST provide `mise` on the default `PATH` of every session, at a
  version the repository pins, for both published architectures.
- **FR-002**: The `mise` the image ships MUST be verified against a checksum pinned in the
  repository when the image is built; a mismatch or failed download MUST fail the build. The
  image's guard test (`scripts/tests/basic-image-hash.test.sh`) MUST keep refusing keys, tokens,
  the daemon, AI CLIs and piped-script installs, and MUST accept this way of adding `mise`.
- **FR-003**: The basic image MUST provide a C/C++ compiler, linker, the C library headers, `make`
  and `pkg-config`, so a Rust toolchain installed by mise can compile and link crates with C code,
  and `pkg-config` finds the libraries that the image or an own image built `FROM` it provides.
  The development libraries of FR-007 are part of what the image provides.
- **FR-004**: The basic image MUST provide `python3` on the default `PATH`, usable before any setup
  has run.
- **FR-005**: Tools installed with `mise`, whether by a setup command or by a session itself (FR-014),
  MUST land in the sandbox home, so they persist (FR-016) and are shared by every worktree on that
  daemon, and MUST be found by sessions through #1289's `PATH` without further configuration.
- **FR-006**: The basic image MUST still carry no key, token, credential, daemon or AI CLI
  (feature 806, FR-013), and MUST keep working as the base for **My own image** and for Kubernetes
  daemons.
- **FR-007**: The basic image MUST also carry the X11/Wayland development libraries this
  repository's gate links (the set CI installs for its Linux test job: `libxkbcommon-dev`,
  `libwayland-dev`, `libx11-dev`, `libxcursor-dev`, `libxrandr-dev`, `libxi-dev`), so the default
  image builds and tests this repository. System libraries any other project needs beyond FR-003 and
  this set come from an own image built `FROM` the basic image (FR-012); no separate "dev" image
  variant is published.

**This repository builds in a container (Story 2)**

- **FR-008**: This repository MUST declare every tool its gate and its instructions need through
  `mise.toml`, including the Rust components the gate and the `LSP` tool use (formatter, linter,
  rust-analyzer), so `mise install` alone provides them.
- **FR-009**: The repository MUST document its container setup command, and with it, on a new
  container daemon on the default (basic) image, setup MUST succeed, `command -v cargo
  rust-analyzer uv python3 gh` MUST resolve all five, and `mise run gate` MUST exit 0 on a clean
  checkout of the branch that carries this feature (and of `main` after merge).
- **FR-010**: Installing this repository's toolchain in a container session MUST NOT require root,
  `sudo` or writing outside the sandbox home and the worktree.

**PATH reflects reality (Story 3)**

- **FR-011**: Before the first session on a container or Kubernetes daemon starts, the sandbox home
  MUST contain `~/.local/bin` and the mise shims folder that #1289 puts on `PATH`; creating them
  MUST NOT change any file already in them.

**A session installs its own tools (Story 5)**

- **FR-014**: With no setup command configured, a session on a container or Kubernetes daemon MUST
  be able to install user-level tools as the session user, with no root or `sudo`: with mise (`mise
  use -g`, `mise install`) and with each language's own installer (`npm install -g`, `cargo
  install`, `uv tool install`, `go install`, `pip install --user` under a mise-installed Python).
  Everything an install keeps MUST be under the sandbox home (and the worktree, for a project-local
  one); temporary files an installer writes and removes (for example under `/tmp`) are excepted.
- **FR-015**: The binaries those installs create MUST be found by later sessions on any worktree of
  the daemon through #1289's `PATH` (`~/.local/bin` or mise's shims), with no further
  configuration. Where an installer's own default target is outside the home or off that `PATH`,
  every spawn on a container or Kubernetes daemon MUST get a default under the home:
  `NPM_CONFIG_PREFIX=$HOME/.local`, `CARGO_INSTALL_ROOT=$HOME/.local`, `GOBIN=$HOME/.local/bin`,
  each only when the spawn environment (environment-include, else the daemon's own) does not set
  it. Installers whose default is already in the home (mise's data folder, `~/.cargo`,
  `~/.rustup`, uv's tool folder, pip's user base `~/.local`) MUST be left on their defaults. Host
  and remote daemons MUST keep their current environment (as #1289 FR-008).
- **FR-016**: The sandbox home MUST persist for the life of the daemon across a container restart,
  the container's replacement (a new basic image, or any other setting that forces a new
  container) and, on Kubernetes, the pod's recreation, so nothing installed in it has to be
  installed or downloaded again.
- **FR-017**: The basic image MUST carry what those installers need to download, unpack and build:
  `curl`, `git`, `tar`, `gzip`, `xz`, `unzip`, and the C toolchain of FR-003. `curl` comes from
  #1435 and `git`, `tar` and `gzip` from the existing image; this feature adds only `xz` and `unzip`
  (and FR-003's toolchain).
- **FR-018**: System packages MUST stay out of a session's reach (no root, no `sudo`; feature 806).
  The sandbox page MUST say that a session can install user-level tools itself and that they
  persist in the sandbox home, name the defaults of FR-015, and say that system packages come from
  the basic image or an own image built `FROM` it.

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
- **Sandbox home toolchain**: the tools mise and the language installers put under the sandbox
  home, by a setup command or by a session; persistent per daemon (FR-016) and shared by its
  worktrees.

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
- **SC-006**: Every tool a session installed itself (Story 5) is found and runs, with nothing
  downloaded again, after a container restart and after the container's replacement, in every run
  of the check.

## Out of scope

- The per-project setup command, its UI and the toolchain `PATH` (#1289).
- `gh`, `curl`, `wget` and `jq` in the basic image (#1435).
- The session environment issue #1438.
- Installing any project's toolchain (Rust, uv, node, ffmpeg) into the image itself: it installs
  per daemon through setup or by a session (Story 5).
- A per-project image choice in the app (deferred by #1289; a second container daemon covers it).
- Host and remote (SSH) daemons, which use the user's own environment.
- Caching toolchain downloads across different daemons.
- Installing system packages from a session (`apt`, root, `sudo`): they come from the image.
- Serialising installs that sessions start themselves (only setup runs take the setup turn).

## Assumptions

- `mise` is distributed as one self-contained binary per architecture with published checksums, so
  it can be added to the image without a piped installer.
- mise installs Rust through rustup into the home, and the Rust components (formatter, linter,
  rust-analyzer) can be declared in `mise.toml`.
- `uv` installed by mise does not provide `python3` on `PATH` by default, so the image ships
  `python3` (also needed by the repository's hooks before any setup runs).
- The image's `python3` (Debian's) has no `pip` and is marked externally managed, so Python
  packages and tools install through `uv` or a mise-installed Python, whose `pip install --user`
  writes to `~/.local`.
- `ffmpeg` installs through mise's conda backend without root, as `mise.toml` already says.
- The sandbox home is persistent per daemon and writable by the session user, including on
  Kubernetes. Checked in the code on 2026-10-26: a container daemon's home is the host folder
  `<daemon state dir>/sandbox-home` (`crates/micold-core/src/sandbox/mod.rs:496-500`, `:841`; per
  daemon `crates/micold-core/src/daemons.rs:984-1010`), created by the client before the container
  (`crates/micold-client/src/shell/sandbox.rs:321-331`) and bind-mounted read-write at the home path
  (`crates/micold-core/src/sandbox/argv.rs:289-297`), so it outlives a container replaced for a
  different image (`crates/micold-core/src/sandbox/lifecycle.rs:324-331`). A Kubernetes daemon's
  home is `/data/home` on its PersistentVolumeClaim (`crates/micold-core/src/kube/manifest.rs:21-26`,
  `:238-240`); stopping or recreating the pod deletes only the pod
  (`crates/micold-core/src/kube/lifecycle.rs:187-191`, `crates/micold-core/src/kube/start.rs:543-548`),
  and only removing the daemon deletes the claim (`crates/micold-core/src/kube/lifecycle.rs:93-117`).
  FR-016 turns this into a tested requirement.
- A new basic image reaches users through the existing hash, publish and pin steps (feature 806);
  existing containers move to it on their next restart.
- Two `mise install` runs into one sandbox home at the same time do not corrupt each other (mise
  locks its install folders; rustup's behaviour under concurrent installs is less certain). The plan
  verifies this and, if it does not hold, serialises setup's installs per daemon.
- The 250 MB size budget is a starting estimate for a Debian compiler toolchain plus `python3`; the
  plan measures it.
