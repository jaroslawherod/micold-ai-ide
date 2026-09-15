# Implementation Plan: Windows Installation Package

**Branch**: `feat/windows-package` | **Date**: 2026-09-13 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/030-windows-installer/spec.md`

## Summary

Every release will ship a per-user, no-admin Windows installer for x64 and ARM64, alongside the
`.deb` packages (and 028's `.dmg`). Research found the Windows host daemon is still a stub, so the
work runs in two phases. Both ship in one PR, as the user decided on 2026-09-13.

- **Phase A: make the host daemon real on Windows** (FR-020 to FR-025)
  - Resolve the per-user named pipe `\\.\pipe\Micold.Daemon.<SID>` and bind it with an owner-only
    protected DACL.
  - Take the singleton from the pipe's first instance.
  - Keep a pid record under `%LOCALAPPDATA%\micold-ai-ide\run\`, and implement image-checked
    `terminate_daemon` so Restart service works.
  - Reap session trees through Job Objects.
  - Suppress console windows: release-only `windows_subsystem = "windows"` on both binaries, and
    one `no_window` spawn helper for git, powershell and docker.
  - Un-gate the daemon test suite and run it on the Windows CI leg.
- **Phase B: the installer** (FR-001 to FR-019)
  - An Inno Setup 6 script (`packaging/windows/micold-ai-ide.iss`), built by
    `mise run windows-installer`.
  - Installs both exes as siblings into `%LOCALAPPDATA%\Programs\Micold AI IDE`, with a Start menu
    entry, HKCU registration under a pinned `AppId`, and an embedded icon.
  - Detects a running app through `AppMutex` and stops the daemon explicitly before upgrade or
    uninstall. User data is left alone.
  - CI installs, launches and uninstalls it silently on `windows-latest` and `windows-11-arm` for
    every code change.
  - `release.yml` gains a `windows` matrix job that gates `publish`.
  - The docs gain `install-windows.md` (SmartScreen, Smart App Control) and `windows-packaging.md`.

Decisions and alternatives: [research.md](research.md) R1 to R16.

## Technical Context

**Language/Version**: Rust stable, pinned by `rust-toolchain.toml` (≥ 1.91, which makes
`aarch64-pc-windows-msvc` Tier 1). Workspace version 0.12.1. Inno Setup 6 Pascal script for
`[Code]`. Bash for build and smoke scripts; PowerShell only inline in CI for PE and pipe probes.

**Primary Dependencies**:

- Existing:
  - `interprocess` 2.4.2: Windows `ListenerOptionsExt::security_descriptor`,
    `SecurityDescriptor::deserialize`.
  - `windows-sys` 0.61.2: adds the features `Win32_Security`, `Win32_Security_Authorization`,
    `Win32_System_Pipes` (tests) and `Win32_System_ProcessStatus`, if needed for the image name.
  - `portable-pty` 0.9 (ConPTY), `directories` (`ProjectDirs`), `tokio`.
- New:
  - `embed-resource` 3.x, a build-dependency of `micold-client` and `micold-daemon` (vetting in R11).
  - Inno Setup 6.7.x, a CI and maintainer tool that is preinstalled on runners and not a Cargo
    dependency.

**Storage**: None new. A pid file is added under `%LOCALAPPDATA%\micold-ai-ide\run\`. User data
paths are unchanged and are never touched by the installer ([data-model.md](data-model.md)).

**Testing**:

- `cargo test`: unit tests in `micold-core` and the daemon; integration tests in
  `crates/*/tests/`, including new text-scan guards.
- Bash tests under `scripts/tests/` for `windows-installer.sh` argument and version handling.
- A CI smoke script that performs a real silent install, launch and uninstall on Windows runners.
- Manual quickstart Part M.

**Target Platform**:

- Windows 10 1809+ and Windows 11, x64 and ARM64. ConPTY needs 1809 or later.
- Build and CI: `windows-latest` (x64) and `windows-11-arm` (ARM64).
- Linux and macOS must remain unaffected. The cfg arms are cross-checked with `cargo check --target`.

**Project Type**: Desktop application (iced GUI client plus a background session daemon) and its
distribution pipeline.

**Performance Goals**:

- The daemon pipe appears within 20 s of a cold installed launch on a CI runner. The client's
  existing spawn timeout is 10 s locally.
- `stop_running_daemon` ends the endpoint within 5 s.
- The installer adds no measurable startup cost to the app. The mutex creation is O(1).

**Constraints**:

- No admin rights at any step.
- Unsigned (FR-010).
- No console window, ever, in release builds.
- No autostart, no PATH or registry edits beyond Inno's own uninstall key.
- The showcase is never shipped.
- A release is published only with complete artifact sets.
- Must merge cleanly with 028 / PR #284, whichever lands second rebases (research, "Coordination").

**Scale/Scope**:

- 2 installer assets per release.
- About 8 source files touched in Phase A: `endpoint.rs`, `spawn.rs`, `git.rs`, `env_include.rs`,
  `sandbox/exec.rs`, `singleton.rs`, `server.rs`, `platform/windows.rs`, plus the two `main.rs`
  files.
- 26 Unix-gated daemon test files to triage.
- About 6 new files in Phase B, plus 2 docs pages and 2 workflow edits.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

Status is shown before and after Phase 1 design.

- [x] **I. Test-First (NON-NEGOTIABLE)**: PASS, pre and post.
  - Every Phase A behaviour has a named failing-first test ([contracts/windows-endpoint.md](contracts/windows-endpoint.md) E1–E7). Several run as un-gated real two-process tests on Windows CI.
  - Every Phase B rule has a text-scan guard or a smoke assertion ([contracts/windows-installer.md](contracts/windows-installer.md) "Guard tests", I1–I8).
  - The two new `main.rs` lines (`windows_subsystem` attribute, `announce_running()` call) are thin glue under the GUI/process-spawn exception. Their logic (`process::announce_running`, `no_window`) lives in tested `micold-core`.
  - Inno `[Code]` has no unit-test harness. Its one decision (image check, then terminate) mirrors the Rust `terminate_daemon`, which is unit-tested, and is exercised end to end by the CI smoke, which installs twice with a live daemon.
- [x] **II. Multi-Session Support**: PASS. There is no new session state.
  - Job Objects are per session (data-model "Session job").
  - Upgrade stops sessions only after explicit confirmation, and their durable records restore as today.
- [x] **III. Worktree Integration**: PASS, not affected.
  - `no_window` changes only how git is spawned, not what it does.
  - Worktrees are never under the install directory.
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: PASS.
  - The installer is a local file with no network calls and no telemetry.
  - All state stays under `%APPDATA%` and `%LOCALAPPDATA%`.
  - Downloading the installer from GitHub Releases is the user's explicit act.
- [x] **V. Rust + iced Stack**: PASS.
  - All app code is Rust. Inno Setup is a packaging tool, as `cargo-deb` is on Linux, not part of the app.
  - `BoundListener`'s lock becomes `Option<File>`, so "no lock on Windows" is represented, not faked with a directory handle.
- [x] **VI. Cross-Platform Parity**: PASS. This feature *closes* a parity violation: the Windows daemon never worked.
  - Platform code stays behind existing seams (`endpoint.rs`/`singleton.rs` cfg arms, `platform/windows.rs`) and one new abstraction, `micold_core::process` (`no_window`, `announce_running`), which is a no-op on Unix.
  - Core logic does not branch on the OS.
  - CI already covers all three OSes and now also runs daemon tests and a packaging smoke on Windows (x64 and ARM64).
  - Documented residual gap, unchanged by this feature: logout survival stays unsupported on Windows (`docs/daemon.md`).
- [x] **VII. Documentation First-Class**: PASS.
  - Pages: `docs/user-guide/install-windows.md`, `docs/development/windows-packaging.md`, updates to `docs/install.md`, `docs/daemon.md` (the Windows endpoint and stop behaviour) and `docs/SUMMARY.md`.
  - Both new pages are added to the CI docs job's required list, and site download links are verified by `site/stage.sh`.
- [x] **VIII. Reusable UI Component Foundation**: PASS, not affected. No UI is added. The existing "Restart service" banner starts working on Windows unchanged.
- **Distribution (Constraints)**: PASS. This feature is what makes "every release MUST provide builds for Linux, macOS, and Windows" true for Windows.
- **Dependencies (Constraints)**: PASS, conditionally. `embed-resource` must be vetted (license and transitive tree) in the PR before it is added (R11). Inno Setup's license permits this use (R8).

No violations. Complexity Tracking is empty.

## Project Structure

### Documentation (this feature)

```text
specs/030-windows-installer/
├── spec.md
├── plan.md                         # this file
├── research.md                     # R1–R16, coordination with 028
├── data-model.md                   # endpoint, pid record, mutex, job, installer, locations, artifact set
├── quickstart.md                   # Part A automated, Part M manual
├── contracts/
│   ├── windows-endpoint.md         # E1–E7 + daemon-suite un-gating (Phase A)
│   ├── windows-installer.md        # build interface, .iss directives, I1–I8, guard tests (Phase B)
│   └── release-artifacts.md        # extends 028's contract with the Windows jobs
├── checklists/requirements.md
└── tasks.md                        # /speckit-tasks (not created here)
```

### Source Code (repository root)

```text
Cargo.toml                                   # windows-sys: + Win32_Security, _Authorization, _System_Pipes, ...
crates/micold-core/
├── src/endpoint.rs                          # A: user_sid(), Windows lock_path, run\ dir
├── src/spawn.rs                             # A: Windows terminate_daemon (image-checked), stale pid handling
├── src/process.rs                           # A: NEW, no_window(), announce_running()
├── src/win_job.rs                           # A: NEW, JobHandle moved from env_include.rs
├── src/git.rs, src/env_include.rs,
│   src/sandbox/exec.rs                      # A: route spawns through no_window
└── tests/
    ├── background_spawns_hide_console.rs    # A: NEW guard (E6.2)
    ├── daemon_tests_gate_with_reason.rs     # A: NEW guard
    ├── windows_installer_is_per_user.rs     # B: NEW guard
    ├── windows_installer_version_is_injected.rs  # B: NEW guard
    └── release_publishes_complete_sets.rs   # B: from 028 (or introduced here), rule unchanged
crates/micold-daemon/
├── build.rs                                 # B: NEW, embed icon (embed-resource)
├── src/main.rs                              # A: windows_subsystem (release), announce_running, fatal→log
├── src/singleton.rs                         # A: first-pipe-instance acquire, DACL, Option<File> lock
├── src/server.rs                            # A: pid record write/remove on Windows
├── src/platform/windows.rs                  # A: terminate_process_tree via win_job
├── src/supervisor.rs                        # A: assign PTY child to session job
└── tests/
    ├── windows_pipe_acl.rs                  # A: NEW (E2.1)
    ├── daemon_stop.rs                       # A: NEW (E4.1, E4.2)
    ├── fatal_startup_is_logged.rs           # A: NEW (E6.4)
    └── *.rs                                 # A: 26 files triaged per contract "Un-gating"
crates/micold-client/
├── build.rs                                 # B: NEW, embed icon
├── src/main.rs                              # A: windows_subsystem (release), announce_running
└── tests/packaging_excludes_showcase.rs     # B: + windows_violations
packaging/windows/micold-ai-ide.iss          # B: NEW, Inno Setup script
assets/icon/icon.ico                         # existing, now embedded + used by setup
scripts/
├── windows-installer.sh                     # B: NEW, build binaries + iscc
├── windows-install-smoke.sh                 # B: NEW, silent install/launch/uninstall assertions
└── tests/windows-installer.test.sh          # B: NEW, arg/version/non-Windows refusal
mise.toml                                    # B: + [tasks.windows-installer]
.github/
├── workflows/ci.yml                         # A+B: Windows leg runs daemon tests + package smoke; new windows-arm64-package job in ci-complete; docs required pages
├── workflows/release.yml                    # B: windows matrix job; publish.needs += windows; append Windows notice
└── release-notice-windows.md                # B: NEW
docs/
├── user-guide/install-windows.md            # B: NEW (SmartScreen, Smart App Control, upgrade/uninstall, limits)
├── development/windows-packaging.md         # B: NEW (build locally, CI legs, tests not run on Windows)
├── install.md, daemon.md, SUMMARY.md        # B: Windows section → link; endpoint/stop notes
site/                                        # B: Windows download links (validated by stage.sh)
```

**Structure Decision**: this is the existing three-crate workspace. Phase A changes stay behind the
platform seams already in place:

- the cfg arms in `endpoint.rs` and `singleton.rs`
- `crates/micold-daemon/src/platform/`

It adds one small `micold_core::process` module for the only cross-crate Windows concern, console
and mutex handling. Phase B follows the repository's packaging convention:

- the package definition goes in `packaging/<platform>/`
- the build logic goes in `scripts/` with a `mise` task
- the guards go in `crates/*/tests/`

This matches the `.deb` layout, and 028's macOS layout.

## Delivery order (input to `/speckit-tasks`)

1. **A1 Endpoint + ACL + singleton** (E1–E3). Unlocks `autospawn`, `stream_view` and
   `session_start` on Windows CI.
2. **A2 Stop + pid record + job objects** (E4–E5).
3. **A3 Console suppression** (E6–E7). Guard test first, then the helper, then the call sites, then
   `windows_subsystem`.
4. **A4 CI**. The Windows leg runs `cargo test -p micold-daemon`. Triage the remaining gated files.
5. **B1 Icon embedding + `.iss` + guards + `mise run windows-installer`**.
6. **B2 Smoke script + CI legs** (x64 in the matrix, ARM64 job).
7. **B3 Release job + publish gate + release notice**.
8. **B4 Docs + site links**.
9. **Rebase onto 028** if it has merged, then run manual quickstart Part M.

## Complexity Tracking

No constitution violations to justify.
