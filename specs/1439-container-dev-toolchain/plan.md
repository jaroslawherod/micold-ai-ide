# Implementation Plan: Container sessions can build a mise project

**Branch**: `claude/project-thread-xudko5` | **Date**: 2026-10-25 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/1439-container-dev-toolchain/spec.md`

## Summary

The basic image gains what a session needs to install and build a project's toolchain without root:
a checksum-pinned `mise` binary, a C/C++ toolchain with `pkg-config`, `python3`, and (FR-007,
provisional option A) the six X11/Wayland development libraries this repository's gate links. Its
entrypoint creates the two home folders 1289 puts on `PATH`, so `command -v` tells the truth before
any setup. A project then gets its toolchain from its setup command (`mise trust && mise install`,
feature 1289) into the persistent sandbox home. Because rustup fails or loses updates when two
installs meet in one home (research R5, probed), the daemon gives setup runs on a sandbox daemon one
turn at a time. This repository declares its Rust components in `mise.toml`, documents its
container setup command, and the user guide and the image README say what the basic image can build
and how to extend it with an own image.

**Amendment 2026-10-26 (Story 5)**: with no setup command, a session installs the tools a build needs
itself. The image adds `xz-utils` and `unzip` for installers' archives (FR-017); on sandbox daemons
the daemon gives every spawn `NPM_CONFIG_PREFIX`, `CARGO_INSTALL_ROOT` and `GOBIN` under the home
when unset, so `npm install -g`, `cargo install` and `go install` land in `~/.local/bin` (D8,
contract [sandbox-install-env.md](contracts/sandbox-install-env.md)); the home already persists
across restarts and replacements (research R12), which new tests pin down (D9). Milestone M5.

## Technical Context

**Language/Version**: Rust stable (`rust-toolchain.toml`), POSIX `sh` (entrypoint), Bash (shell
tests), Containerfile (BuildKit / buildah).

**Primary Dependencies**: Debian trixie packages (`gcc`, `g++`, `make`, `libc6-dev`, `pkgconf`,
`python3`, six `lib*-dev`); the mise release binary (jdx/mise, pinned); `tokio::sync::Mutex` in the
daemon. No new crate.

**Storage**: `packaging/basic/mise.sha256` (new, in the hashed image directory). No app state, no
settings field, no protocol change.

**Testing**: shell tests in `scripts/tests/` (gate); daemon unit/integration tests (gate); the
real-runtime suite behind `sandbox-real-runtime` (`mise run test-sandbox`); quickstart Part B by
hand.

**Target Platform**: the basic image for `linux/amd64` and `linux/arm64`, run by Docker or Podman
on Linux, macOS and Windows hosts, and by Kubernetes (feature 686). Daemon code is
platform-neutral.

**Project Type**: desktop app (Rust workspace) + container image packaging.

**Performance Goals**: a second worktree's setup on a daemon finishes in under 1 minute (SC-003).

**Constraints**: image growth ≤ 250 MB compressed per architecture (SC-004; estimate ~156 MB,
research R6); no root, `sudo` or writes outside the sandbox home and worktree (FR-010); the image
stays free of keys, tokens, the daemon and AI CLIs (FR-006).

**Scale/Scope**: one image directory, one entrypoint, two daemon functions (setup turn, install
defaults) and one pure core function, `mise.toml`, three docs.

## Dependencies and collisions

Implementation starts after **#1289** (setup command and toolchain `PATH`) and **#1435** (`gh`,
`curl`, `jq` in the basic image) have merged; this plan assumes their changes are present and the
branch is rebased onto `main` before M1. Files both this feature and they change, and what to keep
on a conflict, are in [research R11](research.md#r11-what-runs-before-1289-and-1435-land). The
mise download step uses #1435's `curl`.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | How |
|---|---|---|
| **I. Test-First** | PASS | Each requirement has a failing test first: guard and entrypoint shell tests (B3–B5), `mise-toolchain.test.sh` (FR-008), daemon tests for the setup turn (S1–S3), real-runtime tests for the built image and the setup flow. What cannot be automated cheaply (the full gate in a container, the own-image example, size) is quickstart Part B, recorded in the ledger. |
| **II. Multi-Session** | PASS | The setup turn (contract S2) keeps concurrent setups on one daemon from corrupting the shared home; sessions never wait for it (S4). The toolchain is shared read-mostly by every session, by design (FR-005). Installs a session starts itself are not serialised (spec edge case; a failed concurrent install is rerun), documented in the guide. |
| **III. Worktree Integration** | PASS | Setup still runs per app-created worktree (1289); this feature adds no location and no git step. |
| **IV. Local-First** | PASS | No app state leaves the device and nothing in the app needs a remote service. The image build downloads mise at publish time; a project's toolchain is downloaded by the user's own setup command, which they configure. A daemon whose home already holds the toolchain works offline. |
| **V. Rust + iced** | PASS | The only app code is daemon Rust (`setup.rs`, `state.rs`); waiting is `Running` + one output line, so no new state can contradict the existing ones. Packaging stays in `packaging/basic/` as feature 806 set it up. |
| **VI. Cross-Platform** | PASS | The image is built and checked for both architectures by `basic-image.yml`; the mise step maps `TARGETARCH` and fails on anything else. Docker and Podman both build it (quickstart B3, B4). The setup turn is a tokio mutex, compiled and tested on all three CI platforms; it applies only where `offered.json` exists. A session's own installs on a VM-shared home (macOS, Windows) are checked by hand (quickstart B7). |
| **VII. Documentation** | PASS | Same change: `docs/user-guide/sandboxed-daemon.md` (what the basic image builds, the setup command, the own-image example), `packaging/basic/README.md` (contents, pin, update steps), `CONTRIBUTING.md` (the container setup command), `worktrees-and-sessions.md` (one setup at a time on a sandbox daemon), and the sandbox page's "Tools installed in the sandbox's home" (a session's own installs, the defaults, what stays out of reach; M5). |
| **VIII. Reusable UI** | PASS (N/A) | No UI change. |

Post-design re-check: unchanged, no violation. Complexity Tracking is empty.

## Design

### D1. The image (FR-001 to FR-004, FR-006; contract [basic-image-build.md](contracts/basic-image-build.md))

`packaging/basic/Containerfile`, after #1435's package step:

1. Add `gcc g++ libc6-dev pkgconf xz-utils unzip` to the existing `apt-get install` list (one layer;
   research R2, R3, R14). `make` and `python3` are already in it from #1435.
2. A new step: `ARG TARGETARCH`; `COPY mise.sha256 /tmp/mise.sha256`; a `RUN` that maps the arch,
   reads the version from `mise.sha256`, downloads the release binary with `curl -fsSLo`, runs
   `sha256sum -c --ignore-missing`, checks the file exists, installs `/usr/local/bin/mise` 0755 and
   removes the temp files (B3, research R1).
3. A comment on each step naming #1439 and the requirement, in the file's existing style.

`packaging/basic/mise.sha256` (new): the pinned release (data model). The real-runtime test reads
the version from it so a bump needs no Rust change.

### D1a. Taking the pin (FR-002; first task of M1, before `mise.sha256` is written)

The hashes must come from somewhere other than the build that checks them: the CI image build only
compares the download with the committed hash, so it proves the pin was applied, not that it is
right. The implementing unit (or the maintainer, if no unit can reach GitHub) takes the pin once,
on a machine or a `workflow_dispatch` job that can reach `github.com` (this cloud container cannot,
research R1):

1. `gh release download v<ver> -R jdx/mise -p SHASUMS256.txt -p SHASUMS256.asc -p
   SHASUMS256.txt.minisig` into an empty folder.
2. Verify both signatures with keys taken from mise's documentation, never from the release itself
   (`docs/installing-mise.md` and `SECURITY.md` "Release gpg key" in jdx/mise; values below as read
   there on 2026-10-25 at `b9e15069`, re-read at run time, and stop if they differ):
   - GPG: `SHASUMS256.asc` is **clearsigned** (mise's `scripts/release.sh` runs `gpg --clearsign`),
     not a detached signature. Import the key by fingerprint
     `24853EC9F655CE80B48E6C3A8B81C9D17413A06D` (e.g. from `keys.openpgp.org`), check
     `gpg --fingerprint` shows exactly that, then `gpg --decrypt SHASUMS256.asc > signed.txt`; GPG
     must report a good signature by that fingerprint.
   - minisign (the check mise's docs give): `minisign -Vm SHASUMS256.txt -P
     RWTC3g8W3z4RZK3V3qv7fa1QY4JEWyBtqIHW+85QlJpZc5yG+uNYNBSZ` (`minisign.pub` in the repository).
   - `signed.txt` and `SHASUMS256.txt` must be identical (`cmp`).
   A bad or missing signature or a difference stops the step; no pin is written. Step 3 reads
   `signed.txt`, the text GPG verified.
3. Extract exactly the two raw-binary lines:
   `grep -E '  (\./)?mise-v<ver>-linux-(x64|arm64)$' signed.txt`, normalised to
   `<hash>  mise-v<ver>-linux-<arch>` (data model: exactly two lines). Archives (`.tar.gz`,
   `.tar.zst`), `-musl` builds and other platforms are left out.
4. Cross-check each hash against a second origin: `npm pack @jdxcode/mise-linux-x64@<ver>` and
   `@jdxcode/mise-linux-arm64@<ver>` (reachable from this container, R1), unpack each into an empty
   folder and `sha256sum` the `mise` binary inside. Both must equal the lines from step 3; a
   mismatch stops the step.
5. Record in the ledger's *Decisions*: version, both hashes, the GPG fingerprint and minisign key
   used, the npm cross-check result, who ran it and where.

The README "Update mise" section (D6) repeats these five steps for every bump.

### D2. FR-007 development libraries — isolated (provisional option A)

A separate `RUN apt-get install` layer with the six packages, after D1, its comment naming FR-007
and CI's list (`ci.yml:270-272`). Its test constant (`DEV_PKG_CONFIG` in
`sandbox_real_basic_image.rs`), its README paragraph and its user-guide sentence are each one block.
It is milestone **M2** on its own. **Switching to option B** (own image) touches only: that layer,
that constant, that paragraph and sentence, and AS4's fixture, which becomes a test-built
`fixture.pc` (research R7). Option C (a "dev" variant) would be a new spec round, not a switch.

### D3. Home folders (FR-011; contract B5)

`packaging/basic/entrypoint.sh`, after `export HOME`: compute the shims folder by 1289's rule and
`mkdir -p` it and `$HOME/.local/bin`; on failure print one line to stderr and continue (`set -e` is
on, so the command is guarded). The existing entrypoint test passes `MICOLD_HOME=/home/jaro`, which
does not exist on a test host: the guard keeps that test working, and new cases use a temp home.

### D4. Setup turn (Principle II; contract [setup-turn.md](contracts/setup-turn.md))

- `crates/micold-daemon/src/state.rs`: `DaemonState` gains `setup_turn: Arc<tokio::sync::Mutex<()>>`
  and an accessor that returns it only when `offered_clis()` is `Some` (S1).
- `crates/micold-daemon/src/setup.rs`: `spawn_run` takes the turn before `spawn_blocking(run_blocking
  …)`. When `try_lock` fails, it records the waiting line through a new `SetupRuns` operation (same
  path as `push`: buffer, log write, watchers), then awaits the lock in a `select!` with the run's
  stop signal. A stopped waiting run goes straight to `finish` with `Ended` meaning stopped (or
  interrupted on daemon stop), and never spawns (S3). The guard lives until `run` has ended.
- `SetupRuns::stop`, `stop_and_wait` and the daemon-stop path signal the waiting run (a `watch`
  sender kept in `Entry`, data model). `SetupRuns` itself still does no I/O.

### D5. This repository (FR-008, FR-009, FR-010)

- `mise.toml`: `rust = { version = "stable", components = "rustfmt,clippy,rust-analyzer" }`
  (research R8). `uv`, `node`, `ffmpeg` unchanged (R9).
- `CONTRIBUTING.md` "Set up": the container daemon setup command, `mise trust && mise install`, and
  that it is also what the sandbox page recommends.
- Nothing in the gate needs root in a container: `scripts/gate.sh` already runs as a normal user
  when not root. Tools the gate's workspace tests expect beyond the image and `mise.toml` are found
  in quickstart B1; any found there is either added to `mise.toml` (a user-space tool) or to the
  image (a system tool, a spec-level change escalated if outside FR-003/FR-007).

### D6. Documentation (FR-012, FR-013)

- `docs/user-guide/sandboxed-daemon.md`: a "Building projects in the basic image" section under
  "Which image it runs on": what the image carries (B1, B2), the setup command for a mise project,
  that installs land in the sandbox home and persist, one setup at a time per sandbox daemon, what
  it does not carry, and a complete own-image `Containerfile` `FROM` the basic image that adds
  `libpq-dev` (quickstart B3).
- `packaging/basic/README.md`: the new contents, the pinned mise version and where its checksums
  live, and "Update mise" steps: D1a's five steps (download `SHASUMS256.txt`, `.asc` and `.txt.minisig`,
  verify the GPG and minisign signatures with the documented keys, keep only the two raw `linux-x64`/`linux-arm64` lines, cross-check against the npm
  platform packages, record provenance), then write `mise.sha256`, update the content hash,
  publish, pin; plus the measured size (B5).
- `docs/user-guide/sandboxed-daemon.md` "Tools installed in the sandbox's home" (M5, FR-018): a
  session can install user-level tools itself (mise, `npm install -g`, `cargo install`, `uv tool
  install`, `go install`; pip through `uv` or a mise Python), the three defaults and that a value
  the environment sets wins, that installs persist across restarts, image updates and pod
  recreation for the life of the daemon (a tool built against an image library, such as a `cargo
  install` linking OpenSSL, may need reinstalling after an image update), that installs a session
  starts are not serialised, and
  that system packages come from the image or an own image (link to the building section).
- `docs/user-guide/worktrees-and-sessions.md`: the setup section 1289 adds gets one sentence on S2:
  one setup at a time on a sandbox daemon, a waiting run's output says it is waiting for another
  setup, and stopping the running setup (Stop setup) lets the next one start (S3).

### D8. A session's own installs (FR-014, FR-015, FR-017; contract [sandbox-install-env.md](contracts/sandbox-install-env.md))

- `crates/micold-core/src/sandbox/toolchain_path.rs`: a pure `install_env_defaults(home, is_set)`
  beside `toolchain_path`, returning the I2 variables whose `is_set` is false (with
  `npm_config_prefix` counting for `NPM_CONFIG_PREFIX`), none without a `HOME`. Pure, like
  `toolchain_path`: no filesystem.
- `crates/micold-daemon/src/state.rs` `with_sandbox_path` (`:1162`): after it orders `PATH`, it adds
  those defaults to `resolved.vars`, using its existing `var` closure (resolved environment, else
  the daemon's own) for both `HOME` and `is_set`. Same `offered_clis()` predicate, so host and remote
  daemons are unchanged (I1); same caller `spawn_env_for`, so every spawn gets them (I4). No new
  helper for the lookup: the closure is reused.
- The image half (FR-017) is D1 step 1.

### D9. The home persists (FR-016)

No code change (research R12). Evidence: the existing unit test
`recreate_deletes_the_drifted_pod_and_creates_a_new_one_keeping_the_volume`
(`crates/micold-core/src/kube/start_tests.rs:388`, the tests module of `start.rs:866`) already shows
that a drifted pod recreated with `DriftChoice::Recreate` deletes only the pod (`deletes` = the pod)
and adopts the claim (one object created); no new unit test. The real-runtime `sandbox_real_self_install` (quickstart A10)
checks a session's installs after a restart, a replacement for another image tag, and a replacement
onto the no-outbound posture, run offline.

### D7. Publish and pin

Per feature 806: the PR that changes `packaging/basic/` updates `BASIC_IMAGE_CONTENT_HASH`;
`basic-image.yml` builds both architectures on the PR (proving each download matches the committed
pin; the pin's own provenance is D1a) and publishes on merge;
a follow-up pin PR sets `BASIC_IMAGE`. Story 2's acceptance "on the default image" holds after the
pin; before it, quickstart B1 runs on `micold-basic:dev` built from the branch, as the spec allows.

## Requirement map and test layers

| Req | Where | Test layer |
|---|---|---|
| FR-001 | D1 step 2; B1 | real-runtime `sandbox_real_basic_image` (A5): `mise --version` = pinned |
| FR-002 | D1 step 2, D1a; B3, B4 | shell `basic-image-hash.test.sh` (A1); build failure by hand (B4); CI image build |
| FR-003 | D1 step 1; B1 | real-runtime A5 (compile and link C); A7 (Rust crate with a C build script) |
| FR-004 | D1 step 1; B1 | real-runtime A5 (`python3 -c`, before setup) |
| FR-005 | D4, D8, data model "Sandbox home toolchain" | real-runtime A7 (setup half: restart, second worktree); A10 (session half) |
| FR-006 | B6; D1 keeps the guard | shell A1 (refusals kept); real-runtime A5 (`ABSENT` unchanged; uid 10001, `--read-only`); B6 |
| FR-007 | D2; B2 | real-runtime A5 (six `.pc`, `-lX11`) |
| FR-008 | D5 | shell `mise-toolchain.test.sh` (A3); B1 |
| FR-009 | D5, D7 | quickstart B1 (gate exits 0, five tools, LSP) |
| FR-010 | D5; B1 | real-runtime A5 (`--read-only`) and A7 (non-root uid) |
| FR-011 | D3; B5 | shell `basic-entrypoint.test.sh` (A2); real-runtime A6 |
| FR-012 | D6 | docs CI (links); quickstart B3 (example builds) |
| FR-013 | D6 | shell A1 (README names the pinned version) |
| Edge: concurrent setup | D4; [setup-turn.md](contracts/setup-turn.md) | daemon tests A4; quickstart B2 |
| Edge: network failure during setup | #1289 reports the failure; mise/rustup resume a partial install | real-runtime A7 (a setup stopped mid-download, then a rerun succeeds) |
| Edge: several sessions on one daemon | read-only use of the shared toolchain; cargo's file locks guard `~/.cargo`, each worktree has its own `target` | quickstart B2 (`cargo build` in two sessions at once) |
| Edge: a `mise` already in `~/.local/bin` | #1289's `PATH` order | real-runtime A6 (a stub in `~/.local/bin` wins) |
| FR-014 | D8; D1 step 1 | real-runtime A10 (mise, `npm install -g`, `cargo install`, `uv tool install`, `go install` under a mise Go, `pip install --user` under a mise Python; each as the non-root user, keeps nothing outside the home) |
| FR-015 | D8; contract I1–I4 | core + daemon unit tests A8; real-runtime A10 (a new session in a second worktree finds each binary) |
| FR-016 | D9; research R12 | kube unit test A9; real-runtime A10 (restart, image-tag replacement, offline posture replacement); quickstart B6 (pod recreated), B7 |
| FR-017 | D1 step 1 | real-runtime A5 (`xz`, `unzip` in `PRESENT`) |
| FR-018 | D6 (M5 paragraph) | real-runtime A10 (`apt-get install` fails); docs CI |
| Edge: a session installs while setup or another session installs | not serialised; rerun | user guide (D6 M5 paragraph) |
| Edge: the environment already names an install location | contract I2 | unit tests A8 |
| Edge: an image update changes a system library | documented (R12) | user guide (D6 M5 paragraph: reinstall such a tool) |
| Edge: an AI CLI updates itself through npm | accepted: offered bins precede `~/.local/bin` on `PATH` (1289 contract), `only_offered` filters availability (`state.rs:1200`) | contract I4 (no test; nothing changes) |
| Edge: no-outbound posture | sessions cannot download; installed tools keep working | real-runtime A10 (offline check) |
| Edge: VM-shared home (macOS, Windows) | Principle VI | quickstart B7 |
| SC-001 | D5, D7 | quickstart B1 (after the documented setup command ends Set up, `mise run gate` exits 0 with no manual install step) |
| SC-002 | D1, D5 | quickstart B1 (`command -v cargo rust-analyzer uv python3 gh` prints five paths in each session opened) |
| SC-003 | D4 | real-runtime A7 (< 60 s, no download) |
| SC-004 | research R6 | quickstart B5, recorded |
| SC-005 | D6 | quickstart B3 step 1 (the section lists contents, limits, and an own-image example that builds) |
| SC-006 | D9 | real-runtime A10 |

No client or geometry-gate layer: no UI changes. No visual pass.

## Suggested milestones (for the tasks unit)

| ID | Deliverable | Requirements |
|---|---|---|
| M1 | Pin taken first (D1a: signed `SHASUMS256.txt`, two lines, npm cross-check, provenance in the ledger); image: toolchain, `python3`, pinned `mise`, home folders; guard, entrypoint and real-runtime tests; README with "Update mise"; hash | FR-001–FR-004, FR-006, FR-011, FR-013 |
| M2 | FR-007's six dev libraries (isolated layer); AS4 fixture `x11`; hash | FR-003 (AS4), FR-007 |
| M3 | Setup turn on sandbox daemons; `worktrees-and-sessions.md` sentence | edge case, Principle II |
| M4 | `mise.toml` components; `CONTRIBUTING.md`; sandbox page section and own-image example; real-runtime `sandbox_real_mise_setup`; quickstart B1–B6 | FR-005, FR-008–FR-010, FR-012, SC-001–SC-005 |
| M5 | Install defaults (D8), pod-recreation test (D9), real-runtime `sandbox_real_self_install`, guide paragraph; quickstart B6 second half, B7. `xz-utils`/`unzip` ride in M1's apt layer | FR-014–FR-018, SC-006 |

M1 and M2 each change `packaging/basic/` and so the content hash; one publish after M2 (or after the
last image milestone) and one pin PR are enough.

## Project Structure

### Documentation (this feature)

```text
specs/1439-container-dev-toolchain/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── basic-image-build.md
│   ├── sandbox-install-env.md
│   └── setup-turn.md
└── tasks.md             # tasks unit
```

### Source Code (repository root)

```text
packaging/basic/
├── Containerfile             # toolchain + python3 (D1), mise step (D1), FR-007 layer (D2)
├── mise.sha256               # NEW: pinned mise release (B3)
├── entrypoint.sh             # home folders (D3)
└── README.md                 # contents, pin, update steps, size (D6)
crates/micold-core/src/sandbox/basic_image.rs        # BASIC_IMAGE_CONTENT_HASH (+ BASIC_IMAGE in the pin PR)
crates/micold-core/tests/sandbox_real_basic_image.rs # PRESENT, version, compile, pkg-config, read-only uid 10001
crates/micold-daemon/src/state.rs                    # setup_turn (D4); install defaults in with_sandbox_path (D8)
crates/micold-daemon/src/setup.rs                    # spawn_run waits for the turn; waiting line; stop of a waiting run (D4)
crates/micold-daemon/tests/setup_turn.rs             # NEW: S1–S3 (or in setup.rs's tests module, beside the existing ones)
crates/micold-daemon/tests/sandbox_real_toolchain_path.rs  # folders exist before setup (A6)
crates/micold-daemon/tests/sandbox_real_mise_setup.rs      # NEW: Story 1 end to end (A7)
crates/micold-daemon/tests/fixtures/mise-c-crate/          # NEW: fixture project (mise.toml, Cargo.toml, build.rs, hello.c)
crates/micold-daemon/tests/sandbox_real_self_install.rs    # NEW: Story 5 (A10)
crates/micold-core/src/sandbox/toolchain_path.rs           # install_env_defaults (D8)
scripts/tests/basic-image-hash.test.sh   # mise cases (B4)
scripts/tests/basic-entrypoint.test.sh   # home folder cases (B5)
scripts/tests/mise-toolchain.test.sh     # NEW: FR-008
mise.toml                                # rust components (D5)
CONTRIBUTING.md                          # container setup command (D5)
docs/user-guide/sandboxed-daemon.md      # building section, own-image example (D6)
docs/user-guide/worktrees-and-sessions.md  # one setup at a time on a sandbox daemon (D6)
```

**Structure Decision**: packaging changes stay in `packaging/basic/` (feature 806's layout); the
only app code is in `micold-daemon`, beside 1289's setup runner; tests sit beside the existing
suites they extend.

## Complexity Tracking

None.
