# Tasks: Container sessions can build a mise project

**Input**: Design documents from `specs/1439-container-dev-toolchain/`

**Bugfix**: 2026-10-27 — [BUG-1500](bugs/BUG-1500.md) Updated from bugfix patch (Phase 10b, T043–T048).

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md),
[data-model.md](data-model.md), [contracts/](contracts/), [quickstart.md](quickstart.md)

**Before any milestone starts**: implementation waits until **#1435** (`gh`, `curl`, `jq` in the
basic image) and **#1289** (per-project setup command, toolchain `PATH`, setup runner and its user
guide section) have both merged into `main`. The design PR may merge before them. Each milestone's
branch is rebased onto `main` with both present; on a conflict in a file research R11 lists, keep
both sides and recompute `BASIC_IMAGE_CONTENT_HASH` (`scripts/basic-image-hash.sh`).

**FR-007** is provisionally option A (the six X11/Wayland dev libraries in the basic image, ledger
D3), awaiting the user's decision. Its work is milestone **M2** alone. Under option B, only M2 is rewritten,
per plan D2 / research R7 (no apt layer; AS4 uses a test-built `fixture.pc`; T030's own-image
example also adds the six packages); M1, M3 and M4 do not change.

**Tests**: Constitution I: every behaviour has a failing test before its implementation, in the
same milestone. What cannot be automated cheaply (the full gate in a container, the own-image
example, size, Kubernetes) is quickstart Part B, run by hand and recorded in the ledger.

**Documentation**: Constitution VII: each milestone that changes what a user sees updates the user
guide in the same PR.

**Cross-platform**: the image is built for `linux/amd64` and `linux/arm64` by `basic-image.yml`;
daemon code (M3) is platform-neutral and tested on all three CI platforms.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: US1–US5 from spec.md

---

## Phase 1: Setup

- [x] T001 Confirm #1435 and #1289 are merged (`git log origin/main` shows both), rebase the branch onto `origin/main`, and check their changes are present: `curl`, `jq`, `gh` in `packaging/basic/Containerfile`'s package step; `crates/micold-daemon/src/setup.rs` setup runner and `crates/micold-core/src/sandbox/toolchain_path.rs`. Record both merge SHAs in `specs/1439-container-dev-toolchain/autopilot.md`.
- [x] T002 Take the mise pin per plan D1a, before any file names a version: on a machine or `workflow_dispatch` job that reaches `github.com` (this cloud container cannot, research R1), download `SHASUMS256.txt`, `SHASUMS256.asc` and `SHASUMS256.txt.minisig` of the chosen release (the newest at the time); **verify step 2 against mise's docs as they stand then** (`docs/installing-mise.md`, `SECURITY.md` "Release gpg key" in jdx/mise; plan D1a quotes the GPG fingerprint `24853EC9F655CE80B48E6C3A8B81C9D17413A06D` and minisign key read on 2026-10-25 through GitHub code search, not yet run end to end): import the GPG key by fingerprint, `gpg --decrypt SHASUMS256.asc > signed.txt` (clearsigned) with a good signature by that fingerprint, `minisign -Vm SHASUMS256.txt -P <documented key>`, `cmp signed.txt SHASUMS256.txt`; extract exactly the two raw `mise-v<ver>-linux-x64` / `-linux-arm64` lines from `signed.txt`; cross-check each hash against the `mise` binary in `npm pack @jdxcode/mise-linux-{x64,arm64}@<ver>`. Any failure stops the milestone (escalate if no machine can reach GitHub). Record version, both hashes, GPG fingerprint, minisign key, npm result, who ran it and where in the ledger's *Decisions*.

---

## Phase 2: User Story 1, part 1 — the image carries mise, a C toolchain and python3 (Priority: P1) 🎯 MVP

**Goal**: in a container on the basic image built from the branch, `mise` (pinned), `cc`/`c++`/`make`/`pkg-config` and `python3` work before any setup (AS1.1, AS1.6), checksum-verified at build (FR-002).

**Independent test**: `mise run image && mise run test-sandbox` runs `sandbox_real_basic_image` green; `scripts/tests/basic-image-hash.test.sh` green.

### Tests (write first, see them fail)

- [x] T003 [P] [US1] Add the guard cases of contract B4 to `scripts/tests/basic-image-hash.test.sh` (reuse its `copy_basic` helper): the real `Containerfile` downloads mise only in the same `RUN` as a `sha256sum -c` of `mise.sha256` (a temp copy with the check removed fails); `mise.sha256` has exactly two lines `<64 lowercase hex>  mise-v<version>-linux-x64` / `-linux-arm64` with one version (temp copies with three lines, uppercase hex, mixed versions each fail); `packaging/basic/README.md` names that version (a temp README without it fails); every existing refusal (key/token/`.ssh`/daemon `COPY`/`ADD`, `npm`/`npx`/`pnpm`/`yarn` install, `curl|wget … | sh`) still refuses.
- [x] T004 [P] [US1] Extend `crates/micold-core/tests/sandbox_real_basic_image.rs` (feature `sandbox-real-runtime`) per quickstart A5 without FR-007: add `mise`, `cc`, `c++`, `make`, `pkg-config`, `python3`, `xz`, `unzip` to `PRESENT` (FR-017); `mise --version` prints the version read from `packaging/basic/mise.sha256` (first field after `mise-v`, so a bump needs no Rust change); a C file compiles and links with `cc` and runs; `python3 -c 'print(1)'` prints `1`; the same checks as uid 10001 with `--read-only` and a writable `/data/home`; `ABSENT` unchanged (FR-006).

### Implementation

- [x] T005 [US1] Create `packaging/basic/mise.sha256` from T002's two lines, in data-model's format ("exactly two lines, `sha256sum` format; one version on both lines; 64 lowercase hex").
- [x] T006 [US1] In `packaging/basic/Containerfile`, add `gcc g++ libc6-dev pkgconf xz-utils unzip` to the existing `apt-get install` list (one layer, research R2/R3/R14; `make` and `python3` are already there from #1435), with a comment naming #1439, FR-003, FR-004 and FR-017 in the file's style.
- [x] T007 [US1] In `packaging/basic/Containerfile`, add the mise step of contract B3: `ARG TARGETARCH`; `COPY mise.sha256 /tmp/mise.sha256`; one `RUN` that maps `amd64`→`x64`, `arm64`→`arm64` (anything else exits non-zero), reads the version from `/tmp/mise.sha256` (not a build `ARG`), `curl -fsSLo` the release binary, `sha256sum -c --ignore-missing`, checks the file exists, `install -m 0755` to `/usr/local/bin/mise`, removes the temp files. Comment names #1439 and FR-001/FR-002.
- [x] T008 [US1] Update `packaging/basic/README.md` (plan D6): new contents (C toolchain, `pkg-config`, `python3`, `mise`), the pinned mise version and that `mise.sha256` holds its checksums, and an "Update mise" section repeating plan D1a's five steps, then write `mise.sha256`, recompute the content hash, publish, pin.
- [x] T009 [US1] Update the "Which image it runs on" section of `docs/user-guide/sandboxed-daemon.md`: the basic image now carries `mise`, a C/C++ toolchain with `pkg-config`, and `python3` (one sentence; the full building section is T030).
- [x] T010 [US1] Recompute `BASIC_IMAGE_CONTENT_HASH` in `crates/micold-core/src/sandbox/basic_image.rs` with `scripts/basic-image-hash.sh` (after T013 too, so it covers the entrypoint change).

---

## Phase 3: User Story 3 — what a session sees on PATH is what exists (Priority: P2)

**Goal**: the sandbox home has `~/.local/bin` and the mise shims folder before the first session; existing contents untouched (FR-011, AS3.1–3.2).

**Independent test**: `scripts/tests/basic-entrypoint.test.sh` green; `sandbox_real_toolchain_path` shows both folders in a fresh sandbox.

### Tests (write first)

- [x] T011 [P] [US3] Add cases to `scripts/tests/basic-entrypoint.test.sh` (contract B5), each on a temp home: both folders created; shims folder follows `MISE_DATA_DIR`, else `XDG_DATA_HOME`, else `$HOME/.local/share/mise/shims` (empty counts as unset); a file already in each folder is unchanged; an unwritable home prints one line on stderr and the entrypoint carries on to its next step. The existing `MICOLD_HOME=/home/jaro` case keeps passing.
- [x] T012 [P] [US3] Extend `crates/micold-daemon/tests/sandbox_real_toolchain_path.rs`: in a new sandbox, before any setup, `test -d` succeeds for `~/.local/bin` and the shims folder (quickstart A6); and with an executable `mise` stub placed in `~/.local/bin`, a session's `command -v mise` resolves to the stub, not `/usr/local/bin/mise` (edge case "A `mise` already in `~/.local/bin`": #1289's `PATH` order keeps the user's copy first).

### Implementation

- [x] T013 [US3] In `packaging/basic/entrypoint.sh`, after `export HOME` and before sshd starts, compute the shims folder by 1289's rule and `mkdir -p "$HOME/.local/bin" "<shims>"`, guarded so `set -e` does not stop the entrypoint; on failure one line to stderr (plan D3).
- [x] T014 [US3] In `docs/user-guide/sandboxed-daemon.md` "Tools installed in the sandbox's home", replace "Both home folders are on the `PATH` even when they do not exist yet" with: on the basic image both folders exist before the first session (an own image's entrypoint may not create them; they are still on `PATH`).
- [ ] T015 Record quickstart B4 (a changed hex digit fails the build at the mise step) and B5 (compressed size per architecture vs the previous pin; over 250 MB is a blocker) in the ledger, and the size in `packaging/basic/README.md`.

---

## Phase 4: User Story 1, part 2 — pkg-config finds the gate's dev libraries (FR-007, provisional A)

**Goal**: `pkg-config` finds the six X11/Wayland libraries; `pkg-config --libs x11` prints `-lX11` (AS1.4, contract B2).

**Independent test**: `sandbox_real_basic_image` checks the six `.pc` files and `-lX11`.

- [x] T016 [US1] Test first: add `DEV_PKG_CONFIG = ["xkbcommon", "wayland-client", "x11", "xcursor", "xrandr", "xi"]` to `crates/micold-core/tests/sandbox_real_basic_image.rs`; `pkg-config --exists` succeeds for each, `pkg-config --libs x11` prints `-lX11`, also as uid 10001 `--read-only`.
- [x] T017 [US1] In `packaging/basic/Containerfile`, a separate `RUN apt-get install -y --no-install-recommends libxkbcommon-dev libwayland-dev libx11-dev libxcursor-dev libxrandr-dev libxi-dev` layer after the mise step, its comment naming FR-007 and `ci.yml:270-272` (plan D2).
- [x] T018 [US1] One paragraph in `packaging/basic/README.md` and one sentence in `docs/user-guide/sandboxed-daemon.md` "Which image it runs on": the image carries the X11/Wayland development libraries this repository's tests link.
- [x] T019 [US1] Recompute `BASIC_IMAGE_CONTENT_HASH` in `crates/micold-core/src/sandbox/basic_image.rs`.
- [x] T019b [US1] Re-measure B5 from the image basic-image.yml publishes and update the README size and the ledger (open; done with T042, the pin PR). Done: growth 159.7 MB (amd64) and 154.1 MB (arm64) compressed, ≤ 250 MB; recorded in the ledger (M1 notes, D18) and the pin PR, not in `packaging/basic/README.md`, which the content hash covers (D18).

---

## Phase 5: User Story 1, part 3 — one setup at a time on a sandbox daemon (edge case "Concurrent setup", Principle II)

**Goal**: setups on an app-owned sandbox daemon run one shell at a time, in order; a waiting run says so and can be stopped at once (contract setup-turn S1–S4).

**Independent test**: `cargo test -p micold-daemon setup_turn` (quickstart A4).

### Tests (write first; every test name contains `setup_turn`)

- [x] T020 [US1] In `crates/micold-daemon/src/state.rs`'s tests module (beside the `offered.json` tests near line 8457): `setup_turn` returns the turn when the daemon has an `offered.json` of its own and `None` without one (S1).
- [x] T021 [US1] In `crates/micold-daemon/src/setup.rs`'s `mod tests` (or a new `crates/micold-daemon/tests/setup_turn.rs` if the runner needs a full `DaemonState`): on a sandbox daemon two runs never overlap and the second starts after the first ends; three runs keep the order they asked; the second's output begins with `Waiting for another setup on this daemon to finish…` through the same buffer, log and `SetupOutput` path; a run that gets the turn at once prints nothing extra; on a host daemon two runs overlap (S2).
- [x] T022 [US1] Same file as T021: **Stop setup** on a waiting run ends it `Stopped` at once and never spawns; `stop_and_wait` (worktree delete) and a daemon stop end a waiting run `Stopped` / `Interrupted` at once; a waiting run stopped before its turn comes does not spawn; stopping the holder lets the first waiting run spawn; the turn is released after a failed, not-started and panicking run (S2, S3).

### Implementation

- [x] T023 [US1] `crates/micold-daemon/src/state.rs`: `DaemonState` gains `setup_turn: Arc<tokio::sync::Mutex<()>>` and an accessor that returns it only when `offered_clis()` is `Some` (data model "Setup turn"); after M3 review A-F1 also only when the daemon runs in its container (`set_in_sandbox`, contract S1).
- [x] T024 [US1] `crates/micold-daemon/src/setup.rs`: `Entry` keeps a `watch` sender the stop paths signal; `SetupRuns` gains the operation that records the waiting line (same path as `push`, no I/O in `SetupRuns`); `spawn_run` takes the turn before `spawn_blocking(run_blocking …)`, joins the turn's queue at start (`join_turn` polls `lock_owned` once); if pending, records the waiting line and awaits the lock in a `select!` with the stop signal, a stopped waiting run goes straight to `finish` without spawning, and the guard lives until the run has ended. `SetupRuns::stop`, `stop_and_wait` and the daemon-stop path signal it (plan D4). No protocol or `SetupState` change.
- [x] T025 [US1] `docs/user-guide/worktrees-and-sessions.md`, the setup section 1289 added: one sentence: on a container or Kubernetes daemon one setup runs at a time; a waiting run's output says it waits for another setup; **Stop setup** on the running one lets the next start (plan D6).

---

## Phase 6: User Story 1, part 4 — a mise project's toolchain installs itself end to end (Priority: P1)

**Goal**: setup `mise trust && mise install` installs a fixture project's Rust toolchain into the sandbox home; its C build script compiles; restart and a second worktree reuse it (AS1.2, AS1.3, AS1.5, FR-005, SC-003).

- [x] T026 [P] [US1] Create the fixture `crates/micold-daemon/tests/fixtures/mise-c-crate/` (`mise.toml` pinning rust with the minimal profile, `Cargo.toml` with no crates.io dependency, `build.rs` that runs `cc` on `hello.c` and links it, `src/main.rs` and one test calling the C function).
- [x] T027 [US1] Create `crates/micold-daemon/tests/sandbox_real_mise_setup.rs` (feature `sandbox-real-runtime`, needs network; reuse `sandbox_real_support`) per quickstart A7: setup succeeds as the non-root session user, writing nothing outside the sandbox home and the worktree (FR-010); `cargo build` and `cargo test` succeed in a terminal; `command -v cargo` resolves under the sandbox home; still found after a container restart; a second worktree's setup ends in under 60 s and its output shows no download; a worktree with no `mise.toml` runs the same setup command to success (edge case "Empty project"); a setup stopped with **Stop setup** while the toolchain downloads leaves a home in which the next setup run of the same worktree succeeds (edge case "Network failure during setup": the failure report itself is #1289's, this proves a partial install does not break the rerun). Green after T005–T013 (needs the published or `:dev` image).

---

## Phase 7: User Story 2 — this repository's gate runs in a container session (Priority: P1)

**Goal**: `mise install` alone provides every tool the gate and the instructions need; the setup command is documented; `mise run gate` exits 0 in a container (FR-008, FR-009, FR-010, SC-001, SC-002).

- [x] T028 [P] [US2] Test first: create `scripts/tests/mise-toolchain.test.sh` (quickstart A3): `mise.toml`'s `[tools]` declares `rust` with components `rustfmt`, `clippy`, `rust-analyzer`, and `uv`; a temp copy without `rust-analyzer` fails.
- [x] T029 [US2] `mise.toml`: `rust = { version = "stable", components = "rustfmt,clippy,rust-analyzer" }` (research R8); `uv`, `node`, `ffmpeg` unchanged. `CONTRIBUTING.md` "Set up": the container daemon setup command `mise trust && mise install`, the same one the sandbox page recommends (plan D5).

---

## Phase 8: User Story 4 — the user guide says what the basic image can build (Priority: P2)

- [x] T030 [US4] `docs/user-guide/sandboxed-daemon.md`: a "Building projects in the basic image" section under "Which image it runs on" (plan D6): what the image carries (contract B1, B2), the setup command `mise trust && mise install` for a mise project, installs land in the sandbox home and persist and are shared by the daemon's worktrees, one setup at a time per sandbox daemon, what it does not carry (a project's toolchain, other system libraries), and a complete own-image `Containerfile` `FROM` the basic image that adds `libpq-dev`.
- [ ] T031 [US4] Run quickstart B3: build T030's example with `docker build` and `podman build` exactly as written; a daemon on it starts sessions and `pkg-config --libs libpq` prints `-lpq`. Record in the ledger.

---

## Phase 9: User Story 2 — this repository's gate in a container, by hand (Priority: P1; in M4)

**Goal**: US2 AS1–AS4, FR-009, SC-001, SC-002 proved on `micold-basic:dev` built with M1–M2 (quickstart B1), and concurrent setups of this repository (B2) and a Kubernetes pod (B6) checked. Results recorded in the ledger.

- [ ] T032 Quickstart B1 on `micold-basic:dev` (or the pinned image after D7's pin PR): setup ends **Set up**, `command -v cargo rust-analyzer uv python3 gh` prints five paths, `mise run gate` exits 0, the `LSP` tool answers a `hover`; record wall times. A tool the gate needs that is missing goes into `mise.toml` (user-space) or escalates (system tool outside FR-003/FR-007), plan D5.
- [ ] T033 Quickstart B2 (two concurrent setups of this repository, the waiting line, `rustup component list --installed` lists all three components, `cargo build` in both sessions at once succeeds (edge case "Several sessions on one daemon"), Stop setup on a waiting run) and B6 (Kubernetes pod), recorded in the ledger.

_T032 and T033 are carried past the M4 PR (ledger D14): they run after T042 publishes and pins the image, by the owner locally (B6 on his cluster), and stay open until then._

_B6 (T033's Kubernetes part, T041's first half) is automated in CI's kind job (`kube-cluster`) as `crates/micold-client/tests/kube_real_toolchain.rs`, per the owner's "for kubernetes setup use kind" (ledger D19, 2026-10-27): **B6 passed** in PR #1515's `kube-cluster` job (run 40471592663, job 122085505358). T033 stays open for B2 only._
---

## Phase 10: User Story 5 — an AI session installs the build tools it needs itself, and they stay (Priority: P1; amendment 2026-10-26)

**Goal**: with no setup command, a session installs user-level tools with mise, `npm install -g`, `cargo install`, `uv tool install`, `go install` and `pip install --user` (under a mise Go and Python) as the session user; every binary is found by later sessions and survives a restart, a container replacement and a pod recreation (FR-014–FR-018, SC-006; contract sandbox-install-env I1–I5).

**Independent test**: `cargo test -p micold-core install_env`, `cargo test -p micold-daemon install_env` (quickstart A8), the kube test of A9, and `mise run test-sandbox` → `sandbox_real_self_install` (A10).

### Tests (write first; every unit test name contains `install_env`; A9's test already exists)

- [x] T034 [P] [US5] In `crates/micold-core/src/sandbox/toolchain_path.rs`'s tests module: `install_env_defaults` with a `HOME` returns `NPM_CONFIG_PREFIX=$HOME/.local`, `CARGO_INSTALL_ROOT=$HOME/.local`, `GOBIN=$HOME/.local/bin`; a variable reported set is left out and the others stay; an empty value counts as unset; a set `npm_config_prefix` leaves out `NPM_CONFIG_PREFIX`; no `HOME` (or an empty one) returns none (contract I2).
- [x] T035 [P] [US5] In `crates/micold-daemon/src/state.rs`'s tests module, beside the `offered.json` / `with_sandbox_path` tests: on a daemon with its own `offered.json` a spawn environment carries the three defaults under the resolved `HOME`; a value set by environment-include is kept unchanged; a value in the daemon's own environment is not overridden; a daemon without an `offered.json` gets none of them (I1); the `PATH` it orders is the same as without this change (1289's P2 untouched).
- [x] T036 [P] [US5] A9 evidence, no new test: confirm the existing `recreate_deletes_the_drifted_pod_and_creates_a_new_one_keeping_the_volume` in `crates/micold-core/src/kube/start_tests.rs:388` (the tests module of `start.rs`) still asserts that only the pod is deleted and the claim is adopted (one object created), and record its name in the ledger as FR-016's Kubernetes evidence. Add a PVC assertion there only if the test has lost one.
- [x] T037 [US5] Create `crates/micold-daemon/tests/sandbox_real_self_install.rs` (feature `sandbox-real-runtime`, needs network; reuse `sandbox_real_support` and the fixture of T026) per quickstart A10: a sandbox daemon with no setup command; in a session terminal `mise use -g uv@<pin>`, Rust `<pin>` with the minimal profile (a `rust = { version, profile = "minimal" }` entry added to the global mise config, `~/.config/mise/config.toml`, then `mise install rust`: `mise use` has no profile flag, D16), `npm install -g` of a local tarball, `cargo install --path` of the fixture crate, `uv tool install` of one small pinned package, `mise use -g go@<pin>` then `go install` of a local module the test writes (`go.mod`, `main.go`, no dependencies), `mise use -g python@<pin>` then `pip install --user` of one small pinned package, each as the non-root user and keeping nothing outside the sandbox home (FR-014); a new session in a second worktree finds each binary under `~/.local/bin` or the shims (FR-015); after a container restart, after a replacement onto a second tag of the same image (`DifferentImage`), and after a replacement onto the no-outbound posture (checked offline), each binary is found and runs (FR-016, SC-006); `apt-get install -y libpq-dev` fails (AS5.6). Green after T038–T039 and M1's image.

### Implementation

- [x] T038 [US5] `crates/micold-core/src/sandbox/toolchain_path.rs`: pure `install_env_defaults(home, is_set)` per plan D8 / contract I2 (no filesystem access, like `toolchain_path`).
- [x] T039 [US5] `crates/micold-daemon/src/state.rs` `with_sandbox_path`: after ordering `PATH`, append `install_env_defaults` to `resolved.vars`, using the function's existing `var` closure for `HOME` and for `is_set` (plan D8). No protocol change; host and remote daemons unchanged.
- [x] T040 [US5] `docs/user-guide/sandboxed-daemon.md` "Tools installed in the sandbox's home": the paragraph of plan D6 (M5): a session installs user-level tools itself, the three defaults and that the environment's own value wins, installs persist across restarts, image updates and pod recreation for the life of the daemon, a tool built against an image library (a `cargo install` linking OpenSSL, an npm native module) may need reinstalling after an image update, session installs are not serialised (rerun a failed one), system packages come from the image or an own image (FR-018).
- [ ] T041 Quickstart B6 second half (AS5.5: a session's installs survive the pod's recreation) and B7 (macOS and Windows hosts), recorded in the ledger. B6 second half: `kube_real_toolchain` in the kind job (D19), **passed** (PR #1515, run 40471592663); T041 stays open for B7 only.

---

## Phase 10b: The agent is told (BUG-1500; FR-019, Story 5 scenario 7)

**Goal**: on a container or Kubernetes daemon, each start writes `~/.micold/AGENTS.md` and a marked block in Claude Code's and Codex's user-level instructions; user text kept; host and remote daemons write nothing; a failed write is logged, not fatal (plan D10).

**Independent test**: `cargo test -p micold-core agent_guide`, `cargo test -p micold-daemon agent_guide`, and `sandbox_real_toolchain_path` in the real-runtime job.

### Tests (write first; every unit test name contains `agent_guide`)

- [ ] T043 [P] [US5] `crates/micold-core/src/sandbox/agent_guide.rs` tests module: `merge_block` into an empty file gives the block alone; into a file with user text appends it after one blank line, the user text unchanged; into a file with an older block replaces only that block, text before and after unchanged; applied twice equals applied once; a begin marker without an end marker counts as no block (a new block is appended, nothing else changes); `guide_files` honours non-empty `CLAUDE_CONFIG_DIR` and `CODEX_HOME` and falls back to `~/.claude/CLAUDE.md`, `~/.codex/AGENTS.md`, an empty value counting as unset; the guide names `sudo`, `/tmp`, `mise use -g`, `npm install -g`, `cargo install`, `uv tool install`, `go install`, `pip install --user` and the three FR-015 variables; the block names the guide's path.
- [ ] T044 [P] [US5] `crates/micold-daemon/src/state.rs` tests module, beside the `install_env` tests: a daemon with its own `offered.json` writes the guide and merges the block into both instruction files under the given `HOME`, keeping a pre-existing user line; a second call leaves each file byte-identical; a daemon without one writes nothing; an instruction file that cannot be written (its folder is a file) does not stop the others and does not panic.
- [ ] T049 [P] [US5] `crates/micold-core/src/kube/manifest.rs` tests module: a pod allowing the Claude Code sign-in has an init container `home-dirs` on the pod's image, with the main container's security context and resources, the `data` volume at `DATA_MOUNT`, and command `["mkdir", "-p", "/data/home/.claude"]` (plus `/data/home` for a mount directly in the home, such as `.gitconfig`); a pod allowing no credential that mounts under the home has no init container; the pod stamp is unchanged by it.
- [ ] T045 [US5] `crates/micold-daemon/tests/sandbox_real_toolchain_path.rs`: after the session opens, `~/.micold/AGENTS.md` exists and `~/.claude/CLAUDE.md` contains the begin marker, read through a split-literal tag (wrapped-echo pitfall). Must compile under `--features sandbox-real-runtime`.

### Implementation

- [ ] T046 [US5] `crates/micold-core/src/sandbox/agent_guide.rs` (+ `pub mod` in `sandbox/mod.rs`): `GUIDE`, markers, `pointer_block`, `merge_block`, `guide_files` per plan D10.
- [ ] T047 [US5] `crates/micold-daemon/src/state.rs` `write_agent_guide` and its call in `crates/micold-daemon/src/server.rs` startup per plan D10.
- [ ] T050 [US5] `crates/micold-core/src/kube/manifest.rs` `pod`: the `home-dirs` init container per plan D10.
- [ ] T048 [US5] `docs/user-guide/sandboxed-daemon.md` "Tools installed in the sandbox's home": a paragraph saying the daemon writes `~/.micold/AGENTS.md` and the marked block in `~/.claude/CLAUDE.md` and `~/.codex/AGENTS.md` at each start, keeps the rest of those files, and that host and remote daemons write nothing (FR-019).

---

## Phase 11: Publish and pin (follow-up PR; close unit)

- [x] T042 Publish and pin per feature 806 (plan D7): after the last image milestone merges, `basic-image.yml` publishes; a follow-up PR sets `BASIC_IMAGE` in `crates/micold-core/src/sandbox/basic_image.rs`.

---

## Dependencies & Execution Order

- T001 → T002 → everything else. #1435 and #1289 merged before T001.
- Phase 2 (T003–T010) and Phase 3 (T011–T015) form M1; T010 runs last in M1 (hash covers the entrypoint).
- Phase 4 (M2) needs M1's Containerfile. Phase 5 (M3) needs only T001 (daemon code), so M3 may run before M2 if FR-007 is still undecided.
- Phases 6–9 (M4) need M1 (tools in the image), M2 (the gate's dev libraries, for T032) and M3 (concurrent setups are safe). T027, T032 and T033 run against `:dev` built with M1–M2; T032 runs after T029 (the components it checks).
- Phase 10 (M5) needs M1 (`mise`, `xz`, `unzip` in the image) and M4 (T026's fixture crate, for T037). T034–T036, T038–T039 need nothing else and may be written earlier.
- Phase 11 (T042) after M5 merges, the last `packaging/basic/` change being M2's.

### Parallel opportunities

- T003 ∥ T004 ∥ T011 ∥ T012 (different files). T026 ∥ T028. T020 ∥ T021 (different files). T034 ∥ T035 ∥ T036.

## Implementation Strategy

M1 first: once merged, a container on the branch's image has mise, a C toolchain and python3 and true `PATH` folders. Then M2 (FR-007, swappable for option B), M3 (setup turn), M4 (end-to-end proof, this repository and its gate in a container, user guide), M5 (a session's own installs, and that they stay). Only the publish-and-pin follow-up (T042) goes to the close unit.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Pinned mise, C toolchain, python3 and home folders in the basic image 🎯 MVP

- **Tasks**: T001–T015
- **Deliverable**: a container started from the basic image built from `main` has the pinned `mise`, `cc`/`c++`/`make`/`pkg-config` and `python3` before any setup, as any uid and with `--read-only`, and the sandbox home's `~/.local/bin` and mise shims folder exist before the first session.
- **Satisfies**: US1 AS1, AS6; US3 AS1–AS2; FR-001, FR-002, FR-003 (toolchain), FR-004, FR-006, FR-011, FR-013, FR-017; edge cases "checksum mismatch fails the build", "A `mise` already in `~/.local/bin`"
- **Verify**: `scripts/tests/basic-image-hash.test.sh`; `scripts/tests/basic-entrypoint.test.sh`; `mise run image && mise run test-sandbox` (`sandbox_real_basic_image`, `sandbox_real_toolchain_path`); quickstart B4, B5 recorded in the ledger
- **Depends on**: — (#1435, #1289 merged)
- **Tier**: full

### M2 — The gate's X11/Wayland dev libraries in the basic image (FR-007, provisional option A)

- **Tasks**: T016–T019
- **Deliverable**: in the basic image built from `main`, `pkg-config` finds the six libraries and `pkg-config --libs x11` prints `-lX11`.
- **Satisfies**: US1 AS4 (image half); FR-003 (pkg-config finds image libraries), FR-007
- **Verify**: `mise run image && mise run test-sandbox` (`sandbox_real_basic_image`, `DEV_PKG_CONFIG`); B5 re-measured
- **Depends on**: M1
- **Tier**: light (one apt layer, one test constant, the pattern of M1's T006/T004; plan D2 names where)

### M3 — One setup at a time on a sandbox daemon

- **Tasks**: T020–T025
- **Deliverable**: on a container or Kubernetes daemon, a second setup waits for the first, says so in its output, and can be stopped at once; host daemons are unchanged.
- **Satisfies**: edge case "Concurrent setup"; contract setup-turn S1–S4; Principle II
- **Verify**: `cargo test -p micold-daemon setup_turn` (in `mise run gate`)
- **Depends on**: — (code); after M1 in order of merge
- **Tier**: full (concurrency in the daemon)

### M4 — A mise project builds end to end; this repository and the user guide

- **Tasks**: T026–T033
- **Deliverable**: setup `mise trust && mise install` builds a fixture Rust crate with C code in a container sandbox and reuses the toolchain on restart and in a second worktree; this repository's documented setup command makes `mise run gate` exit 0 in a container session on the basic image built from `main` (`:dev`), with `cargo`, `rust-analyzer`, `uv`, `python3`, `gh` found; the sandbox page documents building in the basic image with an own-image example that builds.
- **Satisfies**: US1 AS2, AS3, AS5; US2 AS1–AS4; US4 AS1–AS2; FR-005, FR-008, FR-009, FR-010, FR-012; SC-001, SC-002, SC-003, SC-005; edge cases "Empty project", "Network failure during setup", "Several sessions on one daemon", "Kubernetes daemons"
- **Verify**: `scripts/tests/mise-toolchain.test.sh`; `mise run test-sandbox` (`sandbox_real_mise_setup`); quickstart B1 (setup ends Set up, five tools resolve, `mise run gate` exits 0, `LSP` answers), B2, B3, B6 recorded in the ledger
- **Depends on**: M1, M2 (the gate links the six dev libraries), M3
- **Tier**: full

### M5 — A session installs its own build tools, and they stay (amendment 2026-10-26)

- **Tasks**: T034–T041
- **Deliverable**: on a container or Kubernetes daemon with no setup command, a session installs tools with mise, `npm install -g`, `cargo install`, `uv tool install`, `go install` and `pip install --user` (under a mise Go and Python) without root; each binary is found by later sessions in any worktree and still runs after a container restart, a container replacement and a pod recreation.
- **Satisfies**: US5 AS1–AS6; FR-005 (session half), FR-014, FR-015, FR-016, FR-018 (FR-017's packages ship in M1's T006); SC-006; edge cases "A session installs while setup or another session installs", "The environment already names an install location", "An image update changes a system library" (T040), "An AI CLI updates itself through npm" (contract I4), "A daemon whose network posture blocks outbound connections", "Hosts whose container runtime shares folders through a VM"
- **Verify**: `cargo test -p micold-core install_env`; `cargo test -p micold-daemon install_env`; `cargo test -p micold-core recreate_deletes_the_drifted_pod_and_creates_a_new_one_keeping_the_volume` (A9, existing); `mise run image && mise run test-sandbox` (`sandbox_real_self_install`); quickstart B6 second half and B7 recorded in the ledger
- **Depends on**: M1, M4 (T026's fixture)
- **Tier**: full (spawn environment of sandbox daemons; real-runtime test)

Phase 11 (T042, publish and pin per D7) is the only task outside the milestones: the close unit opens the pin PR after M5 merges. SC-004 is measured in M1 (T015) and M2 (T019).
