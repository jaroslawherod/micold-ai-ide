---

description: "Task list for feature 030: Windows installation package"
---

# Tasks: Windows Installation Package

**Input**: Design documents from `specs/030-windows-installer/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md), [data-model.md](data-model.md), [contracts/](contracts/), [quickstart.md](quickstart.md)

**Tests**: Mandatory (Constitution Principle I). The behaviours are listed in [tdd/test-list.md](tdd/test-list.md), and a `[U3]`/`[A1]` marker on a task names the behaviour it serves; `/speckit-tdd-run` ticks marked tasks. In every phase the test tasks come first. Each one must be observed **failing** before its implementation task starts.

- On Windows-only behaviour, the red is observed on the `windows-latest` CI leg: push the failing test on a WIP commit.
- Before pushing any `cfg(windows)` arm, cross-check locally with `cargo check --target x86_64-pc-windows-msvc` and `--target aarch64-apple-darwin`.

**Documentation**: Principle VII. Each user-facing story carries its own doc task.

**Cross-platform**: Principle VI. Windows code sits behind the existing cfg seams (`endpoint.rs`, `singleton.rs`, `platform/windows.rs`) and the new `micold_core::process` module, which is a no-op on Unix.

**Repo rules** (CLAUDE.md and memory):

- Use `mise run <task>` for builds and tests.
- Run `cargo fmt --check` before every push, because the local gate omits it.
- Never kill app instances you did not start, and never `pkill -f`.
- Commit before probing.
- Never use bare `git stash`.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an incomplete task)
- **[Story]**: US1–US4 from spec.md
- Contract row IDs (E1.1, I3, …) refer to [contracts/windows-endpoint.md](contracts/windows-endpoint.md) and [contracts/windows-installer.md](contracts/windows-installer.md).

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: dependencies, guard scaffolding, and a Windows CI leg that runs the daemon suite.

- [X] T001 Add `Win32_Security`, `Win32_Security_Authorization`, `Win32_System_Pipes`, `Win32_System_ProcessStatus` and `Win32_Storage_FileSystem` to the workspace `windows-sys` features in `Cargo.toml` (research R1, R2, R5). Also add `windows-sys = { workspace = true }` as a `[target.'cfg(windows)'.dependencies]` entry of `crates/micold-daemon/Cargo.toml`, and to its dev-dependencies for the ACL test. Verify with `cargo check --target x86_64-pc-windows-msvc -p micold-core -p micold-daemon`.
- [ ] T002 Vet `embed-resource` 3.x (Constitution, Dependencies). Record the output of `cargo tree -e build -i embed-resource`, the license of each new transitive crate, and the last release date in a `## Dependency vetting` section of `specs/030-windows-installer/research.md` under R11. Do not add the dependency yet.
- [ ] T003 [P] Behaviour-preserving refactor: move `JobHandle` from `crates/micold-core/src/env_include.rs:147-210` into a new `crates/micold-core/src/win_job.rs`, `#[cfg(windows)]` and `pub(crate)`. Declare it in `crates/micold-core/src/lib.rs`, and have `env_include.rs` import it. The existing `crates/micold-core/tests/env_include*.rs` must stay green (E5.2).
- [X] T004 [P] [U21] Write the failing guard `crates/micold-core/tests/daemon_tests_gate_with_reason.rs`. It is a text scan of `crates/micold-daemon/tests/*.rs` and fails for any file whose `#![cfg(unix)]` line is not immediately preceded by a line starting with `// unix-only:`. Run `mise run test-core` and see all 26 current offenders reported.
- [X] T005 [U21] Add a `// unix-only: pending Windows triage (030 T0xx)` line above each of the 26 `#![cfg(unix)]` lines in `crates/micold-daemon/tests/`, so T004 goes green. Later tasks replace each placeholder with either removal or a real reason.
- [ ] T006 In `.github/workflows/ci.yml` job `test`, add the step `Test (daemon, Windows)` with `if: runner.os == 'Windows'`, `shell: bash`, `run: cargo test -p micold-daemon --all-targets`, placed after `Build (workspace)`. Add a comment citing FR-025 and noting that the Linux full-workspace step already covers Linux. Push, and record in the PR description which of the 32 ungated files fail on Windows today. That list is the red baseline for Phase 2.

---

## Phase 2: Foundational — the host daemon works on Windows (FR-020 to FR-024)

**Purpose**: nothing in US1–US3 can be shown working until the installed client can start, reach and stop its daemon on Windows. This phase delivers research R1–R7.

**⚠️ CRITICAL**: no installer task (Phase 3+) starts before the checkpoint at the end of this phase.

### Tests first (must fail on the Windows CI leg)

- [ ] T007 [P] [U1] [U2] [U3] E1.1–E1.3: in `crates/micold-core/src/endpoint.rs` tests, replace the `#[cfg(windows)]` stub assertion in `resolve_creates_a_usable_endpoint_pair` with assertions that:
  - `socket_path` equals `\\.\pipe\Micold.Daemon.<SID>`, where `<SID>` matches `^S-1-[0-9-]+$`;
  - `lock_path` ends in `micold-ai-ide\run\micold-daemon.pid`, and its parent directory exists.

  Add `resolve_is_stable`: two calls return equal endpoints.
- [ ] T008 [P] [U4] [U5] E2.1: create `crates/micold-daemon/tests/windows_pipe_acl.rs` (`#![cfg(windows)]`, no gate reason needed). It binds via `singleton::acquire` on a test-unique endpoint, opens the pipe, and calls `GetSecurityInfo(SE_KERNEL_OBJECT, DACL_SECURITY_INFORMATION)`. It asserts that the DACL is protected (`SE_DACL_PROTECTED`), that `AceCount == 1`, and that the one ACE is `ACCESS_ALLOWED` for the current token's user SID.
- [X] T009 [P] [U6] [U7] E3.1–E3.2: in `crates/micold-daemon/tests/daemon_singleton.rs`:
  - Remove the whole-file `#![cfg(unix)]`.
  - Put `#[cfg(unix)]` plus a `// unix-only:` reason on the stale-socket-reclaim and directory-ownership/mode cases.
  - Make the two-simultaneous-starters case platform-neutral.
  - Add `acquire_after_drop_rebinds`: acquire → `Bound`, drop the listener → acquire again → `Bound`.
- [ ] T010 [P] [U8] [U9] [U10] E4.1–E4.2: create `crates/micold-daemon/tests/daemon_stop.rs` (all platforms, no gate) with two cases:
  - `pid_record_lifecycle`: spawn the real `micold-daemon` binary (`env!("CARGO_BIN_EXE_micold-daemon")`) with an isolated endpoint/home. Wait for the endpoint, then assert `lock_path` holds the child's pid followed by a newline. Stop it cleanly and assert the file is removed.
  - `stop_running_daemon_ends_endpoint`: spawn, call `micold_core::spawn::stop_running_daemon(&endpoint)`, and assert it returns `Ok(true)` and the endpoint refuses connections within 5 s.

  Kill only the child this test spawned.
- [ ] T011 [P] [U11] [U12] E4.3–E4.4: in `crates/micold-core/src/spawn.rs` tests:
  - `#[cfg(windows)] terminate_refuses_foreign_image`: spawn `cmd /c ping -n 30 127.0.0.1`, call `terminate_daemon(child.id())`, and assert `ErrorKind::InvalidData` and that the child is still running. Then kill that child.
  - `stale_pid_record_is_ignored` (all platforms): write a pid file for a non-live endpoint and assert `stop_running_daemon` returns `Ok(false)`.
- [ ] T012 [P] [U13] E5.1: in `crates/micold-daemon/src/supervisor.rs` tests, add `#[cfg(windows)] kill_reaps_grandchild`. Start a Regular session whose shell runs `cmd /c start /b ping -t 127.0.0.1`, read the grandchild pid via `CreateToolhelp32Snapshot`, kill the session, and assert the grandchild has exited within 5 s.
- [X] T013 [P] [U18] E6.2: create `crates/micold-core/tests/background_spawns_hide_console.rs`. It walks `crates/*/src/**/*.rs`, strips `//` comments and `#[cfg(test)] mod` bodies, and finds every `Command::new(`. It fails unless `no_window(` appears in the same statement or builder chain.
  - Allowlist exactly `crates/micold-core/src/spawn.rs` `spawn_detached_daemon`, which uses DETACHED_PROCESS, and the PTY builder in `crates/micold-daemon/src/supervisor.rs`, which uses `portable_pty::CommandBuilder`, not `Command::new`.
  - It must report `git.rs:114`, `git.rs:160`, `git.rs:308`, the two `powershell.exe` spawns at `env_include.rs:363,387`, and `sandbox/exec.rs:102,121`. Items under `#[cfg(not(windows))]`, such as `env_include.rs:323` `bash`, are exempt.
- [ ] T014 [P] [U14] [U15] [U16] [U17] E6.3 and E7.1: create `crates/micold-core/src/process.rs` holding only `pub fn no_window(cmd: &mut std::process::Command) -> &mut Command`, a `tokio::process::Command` twin, and `pub fn announce_running() -> Option<RunningMarker>`. Each body is `todo!()`, and the module is declared in `lib.rs`. Its tests:
  - `no_window_sets_flag` (`#[cfg(windows)]`): pins `CREATE_NO_WINDOW == 0x0800_0000` and asserts that a `cmd /c exit 0` spawned through it succeeds.
  - `no_window_is_noop_elsewhere` (`#[cfg(unix)]`): `true` spawns fine.
  - `announce_running_is_visible` (`#[cfg(windows)]`): after the call, `OpenMutexW(SYNCHRONIZE, 0, "Local\\MicoldAIIDE")` returns a non-null handle; after the marker drops, a fresh `OpenMutexW` fails.
- [X] T015 [P] [U19] E6.4: create `crates/micold-daemon/tests/fatal_startup_is_logged.rs` (all platforms). Run the daemon binary with an env that forces a startup failure, for example `MICOLD_LISTEN_ADDR=not-an-addr`. Assert a non-zero exit and that the daemon log file under the test's isolated data dir contains `fatal:`.

### Implementation

- [ ] T016 [U1] [U2] [U3] R1, making T007 pass: implement the Windows `user_sid()` in `crates/micold-core/src/endpoint.rs` via `OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY)`, then `GetTokenInformation(TokenUser)`, `ConvertSidToStringSidW` and `LocalFree`. Use RAII guards for the token handle and the SID string.
  - Set `lock_path` to `ProjectDirs::from("", "", "micold-ai-ide").data_local_dir().parent()/run/micold-daemon.pid`, and `create_dir_all` the `run` directory.
  - Remove the `T083/W5` stub message.
  - The data-model rule applies: "No environment variable influences the Windows endpoint".
- [ ] T017 [U4] [U5] [U6] [U7] R2 and R3, making T008 and T009 pass: in `crates/micold-daemon/src/singleton.rs`, rewrite the Windows `acquire`:
  - Build `SecurityDescriptor::deserialize(format!("D:P(A;;GA;;;{sid})"))` from `endpoint::user_sid()`, which must be made `pub` in micold-core. Pass it to `ListenerOptions::new().name(..).security_descriptor(sd)`.
  - Drop the `File::open(temp_dir())` lock and change `BoundListener._lock` to `Option<std::fs::File>`, `None` on Windows.
  - Keep the `is_live` pre-probe and the `AddrInUse | PermissionDenied → AlreadyRunning` mapping.
  - Fix the comment so it states that `interprocess` sets `FILE_FLAG_FIRST_PIPE_INSTANCE` itself (`create_instance.rs:86`).
- [ ] T018 [U8] [U9] R4, making the T010 `pid_record_lifecycle` case pass: in `crates/micold-daemon/src/server.rs`, write the pid on `Acquisition::Bound` on all platforms. The Windows path is now real, so escalate the warning to an error log if the write fails. Remove `lock_path` on clean exit through a guard that drops after `serve_interprocess` returns.
- [ ] T019 [U10] [U11] [U12] R5, making T010 `stop_running_daemon_ends_endpoint` and T011 pass: in `crates/micold-core/src/spawn.rs`, replace `#[cfg(not(unix))] terminate_daemon`. Add a `#[cfg(windows)]` arm that:
  1. Calls `OpenProcess(PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION | SYNCHRONIZE)`.
  2. Checks `QueryFullProcessImageNameW`; the file name must equal `micold-daemon.exe` case-insensitively, otherwise return `ErrorKind::InvalidData`.
  3. Calls `TerminateProcess` and `WaitForSingleObject(5000)`.

  In `running_daemon_pid`, return `None` when the endpoint is not live, so a stale record is ignored.
- [ ] T020 [U13] [U67] R6, making T012 pass: in `crates/micold-daemon/src/platform/windows.rs`, replace the no-op `terminate_process_tree` with a per-session job built on `micold_core::win_job` (make it `pub` behind `#[cfg(windows)]`).
  - In `crates/micold-daemon/src/supervisor.rs`, right after the `portable-pty` spawn, call `OpenProcess` plus `AssignProcessToJobObject` on the child pid, and store the job with the session.
  - On kill, terminate the job.
  - The job is created with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`.
  - Record the known limit in a code comment: a grandchild spawned before assignment escapes.
  - [U67] In `PtySession`'s `Drop`, close the PTY master before joining the reader thread. Under ConPTY the reader sees EOF only once the pseudoconsole is closed, so joining first never returns.
- [ ] T021 [U14] [U15] [U16] [U17] R7, making T014 pass: implement `crates/micold-core/src/process.rs`.
  - `no_window` calls `std::os::windows::process::CommandExt::creation_flags(CREATE_NO_WINDOW)` on Windows and does nothing elsewhere.
  - `announce_running` calls `CreateMutexW(null, FALSE, "Local\\MicoldAIIDE")` and returns a `RunningMarker` that closes the handle on drop. It returns `None` on Unix.
- [X] T022 [U18] R7, making T013 pass: route every flagged spawn through `micold_core::process::no_window`:
  - `crates/micold-core/src/git.rs` (run_git, branch_exists, submodule update)
  - `crates/micold-core/src/env_include.rs` (run_bounded; keep the Job Object)
  - `crates/micold-core/src/sandbox/exec.rs` (docker/podman)
  - any further hit T013 reports
- [X] T023 [U19] R7, making T015 pass: in `crates/micold-daemon/src/main.rs`, on `Err(e)` also append `micold-daemon: fatal: {e}` to the daemon log path. Resolve that path through the same function `logging` uses, which already exists as `micold_daemon::logging::default_log_path()` (`logging.rs:241`). Then `eprintln!` and exit 1 as today.
- [ ] T024 [U20] R7 (GUI glue, Principle I exception): add `#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]` to `crates/micold-daemon/src/main.rs` and `crates/micold-client/src/main.rs`. Hold `let _running = micold_core::process::announce_running();` as the first statement of both `main`s, for the process lifetime.
- [ ] T025 [U20] E6.1: in `.github/workflows/ci.yml` job `test`, add the step `Release exes are GUI-subsystem (Windows)` with `if: runner.os == 'Windows'` and `shell: pwsh`. It runs `cargo build --release -p micold-client --bin micold-ai-ide -p micold-daemon --bin micold-daemon`. For each of `micold-ai-ide.exe` and `micold-daemon.exe` it reads the PE `e_lfanew` at 0x3C and the `Subsystem` u16 at `e_lfanew + 0x5C`, and asserts it is `2`.
- [ ] T026 [U22] [U23] [U24] [U25] [U26] FR-025: un-gate the files the contract names as required:
  - `crates/micold-daemon/tests/autospawn.rs`
  - `crates/micold-daemon/tests/stream_view.rs`
  - `crates/micold-daemon/tests/session_start.rs`
  - `crates/micold-daemon/tests/session_isolation.rs`
  - `crates/micold-daemon/tests/session_survival.rs`

  For each: remove the whole-file `#![cfg(unix)]` and its placeholder reason. Fix Unix-specific test plumbing, for example `/bin/sh` → the platform shell via `terminal.rs`'s resolver, and path separators. Put `#[cfg(unix)]` plus a `// unix-only:` reason only on cases that genuinely need Unix. Also fix `crates/micold-daemon/tests/support/` helpers these files use.
- [ ] T027 FR-025 triage of the remaining 21 gated files in `crates/micold-daemon/tests/`. For each file, in this order of preference:
  1. un-gate it, or
  2. partly gate it, with per-case reasons, or
  3. keep the whole-file gate, replacing the placeholder with a concrete `// unix-only: <dependency>` reason.

  Append every file still wholly gated to a list in the PR description. T060 carries that list into docs.
- [ ] T028 Cross-check and push: run `cargo fmt --check`, `cargo check --target x86_64-pc-windows-msvc -p micold-core -p micold-daemon -p micold-client`, `cargo check --target aarch64-apple-darwin -p micold-core -p micold-daemon`, and `mise run test`. Push, and confirm that the Windows leg runs T007–T015 green and that Linux and macOS are unchanged.

**Checkpoint**: on `windows-latest`, `cargo test -p micold-daemon --all-targets` is green, including `windows_pipe_acl`, `daemon_singleton`, `daemon_stop`, `autospawn`, `stream_view` and `session_start`. FR-020 to FR-025 hold for a `cargo run` build.

---

## Phase 3: User Story 1 — Install from a release download (P1) 🎯 MVP

**Goal**: one setup `.exe` per architecture. It installs per-user without admin, adds a Start menu entry with an icon, and launches with no console. The installed client starts its sibling daemon.

**Independent Test**: quickstart A4 steps 1–5, run in CI on `windows-latest` and `windows-11-arm`, plus manual M1–M3 and M6.

### Tests for User Story 1 (write first, observe failing) ⚠️

- [X] T029 [P] [US1] [U27] [U28] [U29] [U60] [A5] FR-012: extend `crates/micold-client/tests/packaging_excludes_showcase.rs` with `windows_violations(iss: &str, showcase: &str, shipped: &[&str]) -> Vec<String>`. It follows 028's `macos_violations` shape if present, otherwise a local `BUNDLED_BINS = ["micold-ai-ide", "micold-daemon"]`, and:
  - strips `;` comment lines;
  - collects `Source:` values in `[Files]`;
  - fails on any `*` or `?`, on any basename other than `micold-ai-ide.exe` and `micold-daemon.exe`, and on any mention of `micold-showcase`.

  Include unit cases for a wildcard, the showcase exe, and a missing daemon, plus a case that reads the real `packaging/windows/micold-ai-ide.iss`. The real-file case fails because the file does not exist yet.
- [X] T030 [P] [US1] [U30] [U31] [U32] [U33] [U34] [U35] FR-004, FR-011, FR-006: create `crates/micold-core/tests/windows_installer_is_per_user.rs`, a text scan of `packaging/windows/micold-ai-ide.iss`. It asserts:
  - `PrivilegesRequired=lowest`
  - no `PrivilegesRequiredOverridesAllowed`
  - `AppId={{1B19A6AC-4C91-4033-88EA-F7F283127C8A}` exactly ("pinned forever")
  - `DefaultDirName={autopf}\Micold AI IDE`
  - `RestartApplications=no`
  - no `[Registry]` section
  - no `SignTool`
  - `ArchitecturesAllowed` is conditional on `{#Arch}`: `x64compatible and not arm64` for x64 and `arm64` for arm64
- [X] T031 [P] [US1] [U36] [U37] FR-013: create `crates/micold-core/tests/windows_installer_version_is_injected.rs`. It asserts:
  - `AppVersion={#AppVersion}` and `VersionInfoVersion={#AppVersion}`, with no literal semver anywhere in the `.iss`;
  - `OutputBaseFilename=micold-ai-ide-{#AppVersion}-{#Arch}-setup`;
  - `scripts/windows-installer.sh` reads the version from `[workspace.package]` in `Cargo.toml` and passes `/DAppVersion=`.
- [X] T032 [P] [US1] [U43] [U44] [U45] [U46] [U47] [U48] FR-016: create `scripts/tests/windows-installer.test.sh`, following the style of `scripts/tests/classify-change.test.sh`. Cases, all run with `ISCC`/`cargo` stubbed on PATH:
  - on a non-Windows `uname`, the script exits non-zero with `the Windows installer is built on Windows`;
  - `--arch` rejects values other than `x64|arm64`;
  - the resolved version equals `Cargo.toml`'s `[workspace.package] version`;
  - the `iscc` invocation carries `/DAppVersion=<v> /DArch=<arch> /DBinDir=<dir>`;
  - with no `iscc` found, the error names the Inno Setup download URL;
  - cargo is invoked as `--release --locked -p micold-client --bin micold-ai-ide -p micold-daemon --bin micold-daemon --target <triple>`.

  Wire it into the `.github/workflows/ci.yml` step that loops over `scripts/tests/*.test.sh`, if it is not already globbed.
- [ ] T033 [P] [US1] [A1] [A2] [A3] [A4] FR-018 / I1, I2, I7: create `scripts/windows-install-smoke.sh <setup.exe>`, written first as assertions only. It runs, in order:
  1. `"$exe" /VERYSILENT /SUPPRESSMSGBOXES /NORESTART /LOG=install.log` and asserts exit 0.
  2. Asserts `$LOCALAPPDATA/Programs/Micold AI IDE/micold-ai-ide.exe` and `micold-daemon.exe` exist.
  3. Asserts `$APPDATA/Microsoft/Windows/Start Menu/Programs/Micold AI IDE.lnk` exists.
  4. Asserts `reg query "HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\{1B19A6AC-4C91-4033-88EA-F7F283127C8A}_is1" /v DisplayVersion` equals the workspace version.
  5. Starts the installed client detached and records its pid.
  6. Polls up to 20 s for `\\.\pipe\Micold.Daemon.<SID>`, with the SID from `whoami /user /fo csv /nh` and the check via `powershell -c "Test-Path '\\.\pipe\...'"`.
  7. Asserts no `conhost.exe` has the client or the daemon as parent, via `Get-CimInstance Win32_Process`.
  8. Stops only the pids it started: `taskkill /PID <client> /T /F`, then stops the daemon via its pid record.

  A client exit with a graphics-surface error counts as a pass only when step 6 succeeded, as in 028's R12. On failure, print `install.log` and the daemon log.

### Implementation for User Story 1

- [ ] T034 [US1] R11, after T002 vetting: add `embed-resource = "3"` as a `[build-dependencies]` entry in `crates/micold-client/Cargo.toml` and `crates/micold-daemon/Cargo.toml`.
  - Create `crates/micold-client/build.rs` and `crates/micold-daemon/build.rs`. Each compiles `packaging/windows/app-icon.rc` via `embed_resource::compile(..., embed_resource::NONE)` only when `CARGO_CFG_TARGET_OS == "windows"`, and emits `cargo:rerun-if-changed` for the `.rc` and `assets/icon/icon.ico`.
  - Create `packaging/windows/app-icon.rc` containing `1 ICON "../../assets/icon/icon.ico"`, with the path resolved relative to the `.rc`.
  - Verify with `cargo check --target aarch64-apple-darwin` and on Linux that the build script is a no-op there.
- [X] T035 [US1] [U27] [U28] [U29] [U60] [U30] [U31] [U32] [U33] [U34] [U35] [U36] [U37] [A5] R8–R10, making T029, T030 and T031 pass: create `packaging/windows/micold-ai-ide.iss` with every directive in contracts/windows-installer.md, section "Script directives":
  - `#ifndef AppVersion`, `Arch` and `BinDir` → `#error`
  - `AppId={{1B19A6AC-4C91-4033-88EA-F7F283127C8A}`
  - `PrivilegesRequired=lowest`
  - `DefaultDirName={autopf}\Micold AI IDE`
  - `DisableProgramGroupPage=yes`
  - `ArchitecturesAllowed` / `ArchitecturesInstallIn64BitMode` from `#if Arch == "arm64"`
  - `SetupIconFile=..\..\assets\icon\icon.ico`
  - `UninstallDisplayIcon={app}\micold-ai-ide.exe`
  - `OutputBaseFilename=micold-ai-ide-{#AppVersion}-{#Arch}-setup`
  - `LicenseFile=..\..\LICENSE`
  - `WizardStyle=modern`
  - `[Files]`: exactly the two `Source: "{#BinDir}\…exe"; DestDir: "{app}"; Flags: ignoreversion` lines
  - `[Tasks]`: `desktopicon`, unchecked
  - `[Icons]`: `{autoprograms}\Micold AI IDE` and `{autodesktop}` under `desktopicon`
  - `[Run]`: `postinstall nowait skipifsilent unchecked` launch

  Add no `[Registry]`. `AppMutex`, `CloseApplications` and `[Code]` land in US2.
- [X] T036 [US1] [U43] [U44] [U45] [U46] [U47] [U48] FR-016, making T032 pass: create `scripts/windows-installer.sh` (bash, `set -euo pipefail`), implementing contracts/windows-installer.md, section "Build interface":
  - It refuses on non-Windows `uname -s` (anything not matching `MINGW*|MSYS*|CYGWIN*`).
  - `--arch` defaults from `$PROCESSOR_ARCHITECTURE` (`AMD64` → x64, `ARM64` → arm64), and `--out-dir` defaults to `$(scripts/build-lock.sh --print-target-dir)/windows-installer`.
  - It builds through `scripts/build-lock.sh cargo build ...`.
  - It resolves `iscc` via `$ISCC`, then `"${ProgramFiles(x86)}/Inno Setup 6/ISCC.exe"`, then `command -v iscc`.
  - It runs `iscc /DAppVersion=… /DArch=… /DBinDir=… /O<out-dir> packaging/windows/micold-ai-ide.iss` and prints the output path.
- [X] T037 [US1] FR-016: add `[tasks.windows-installer]` to `mise.toml`, following `[tasks.deb]`: description `Build the Windows setup .exe for the host arch (needs Inno Setup 6) [build-locked]`, and `run = "{{config_root}}/scripts/windows-installer.sh"`.
- [ ] T038 [US1] [A1] [A2] [A3] [A4] [A13] FR-018 / SC-007 x64: in `.github/workflows/ci.yml` job `test`, add the step `Package, install and launch the Windows installer` with `if: runner.os == 'Windows'` and `shell: bash`. It runs `scripts/windows-installer.sh --arch x64 --out-dir dist` and then `scripts/windows-install-smoke.sh dist/micold-ai-ide-*-x64-setup.exe`. Upload `dist/*.exe` with `actions/upload-artifact` (retention 7 days) for manual quickstart M1–M6.
- [ ] T039 [US1] [A1] [A2] [A3] [A4] FR-015 / FR-018 ARM64: add the job `windows-arm64-package` to `.github/workflows/ci.yml`:
  - `name: package + smoke (windows-11-arm)`, `needs: classify`, `if: needs.classify.outputs.docs_only != 'true'`, `runs-on: windows-11-arm`
  - steps: checkout, `dtolnay/rust-toolchain@stable` with `targets: aarch64-pc-windows-msvc`, rust-cache, the two script calls with `--arch arm64`, and upload-artifact

  Then:
  - add it to `ci-complete.needs`;
  - add an env `WINARM: ${{ needs.windows-arm64-package.result }}` and a `check windows-arm64 "$WINARM"` line;
  - confirm `crates/micold-core/tests/ci_gate_covers_every_job.rs` passes.
- [ ] T040 [US1] [A1] [A2] [A3] [A4] [A13] Push and observe both Windows packaging legs green. If a leg fails, fix `.iss`, the scripts, or the Phase 2 code; do not weaken smoke assertions. Remove the `continue-on-error: true` that PR #314 put on both "Install and launch" steps in `.github/workflows/ci.yml`, so the smoke gates `ci-complete` again. Record the observed pipe-appearance time on each architecture in `specs/030-windows-installer/research.md`, R14, as evidence.
- [X] T041 [P] [US1] Docs, FR-017 install part and FR-010: create `docs/user-guide/install-windows.md`, covering:
  - which file to download, with `{{MICOLD_VERSION}}`/`{{MICOLD_TAG}}` download links for `micold-ai-ide-{{MICOLD_VERSION}}-x64-setup.exe` and `-arm64-setup.exe`, and how to tell x64 from ARM64 (Settings → System → About → System type);
  - the SmartScreen "Windows protected your PC" dialog, then **More info**, then **Run anyway**, and why it appears (unsigned);
  - the steps to install without admin rights;
  - what gets installed and where (`%LOCALAPPDATA%\Programs\Micold AI IDE`, Start menu entry, no PATH change, nothing at login);
  - prerequisites: Git for Windows, plus Claude Code or Copilot CLI on PATH.

  Add it to `docs/SUMMARY.md` next to `install.md`.

### Outer loop for User Story 1 (acceptance tests green before the story is complete)

- [X] T065 [US1] [A1] US1-AS1: in the CI smoke run from T038/T039, the silent per-user install exits 0 and leaves both exes plus the Start menu `.lnk`. Record the CI run URL in `specs/030-windows-installer/tdd/cycle-log.md`.
- [ ] T066 [US1] [A2] US1-AS2: in the same smoke run, the installed client has no `conhost.exe` child.
- [ ] T067 [US1] [A3] US1-AS3: in the same smoke run, the daemon pipe appears within 20 s and the daemon has no `conhost.exe` child.
- [X] T068 [US1] [A4] US1-AS4: in the same smoke run, the uninstall key's `DisplayVersion` equals the workspace version.
- [ ] T069 [US1] [A5] US1-AS5: the real-file case of `windows_violations` in `crates/micold-client/tests/packaging_excludes_showcase.rs` passes on the committed `packaging/windows/micold-ai-ide.iss`.

**Checkpoint**: a CI-built setup `.exe` installs and launches on x64 and ARM64 with no console, and the daemon pipe appears. US1 can be demonstrated on its own from the CI artifact.

---

## Phase 4: User Story 2 — Upgrade and uninstall cleanly (P2)

**Goal**: installing over an older or the same version leaves one entry. A running app or daemon is detected and stopped only after confirmation. Uninstall removes program files, shortcut, registration and `run\`, but never user data.

**Independent Test**: quickstart A4 steps 6–7 in CI, plus manual M4 and M5.

### Tests for User Story 2 (write first, observe failing) ⚠️

- [X] T042 [P] [US2] [U38] [U39] [U40] [U41] [U42] [U61] FR-009 and FR-007: extend `crates/micold-core/tests/windows_installer_is_per_user.rs`, or add `crates/micold-core/tests/windows_installer_in_use.rs`. The text scan asserts:
  - `AppMutex=Local\MicoldAIIDE`, the same literal as `process::announce_running`; the test reads both files and compares them;
  - `CloseApplications=force`;
  - `[Code]` defines `PrepareToInstall` and `InitializeUninstall`, and each calls `StopDaemon`;
  - `[UninstallDelete]` has exactly one entry, `Type: filesandordirs; Name: "{localappdata}\micold-ai-ide\run"`;
  - no `[UninstallDelete]` or `[InstallDelete]` entry names `{userappdata}` or `{localappdata}\micold-ai-ide\data`, since user data is never deleted.
- [ ] T043 [P] [US2] [A6] [A7] [A8] [A9] I4 and I5: extend `scripts/windows-install-smoke.sh` with two phases.
  - Before install, seed `$APPDATA/micold-ai-ide/data/smoke-marker` and `$LOCALAPPDATA/micold-ai-ide/data/smoke-marker`.
  - After the launch check, leave the daemon **running** and re-run the same installer silently (repair with a live daemon, I4). Assert exit 0, exactly one `…Uninstall\{1B19A6AC-…}_is1` key (`reg query … /s | grep -c _is1` = 1), and that the old daemon pid is gone.
  - Relaunch the client, wait for the pipe, and leave it running. Run `"$LOCALAPPDATA/Programs/Micold AI IDE/unins000.exe" /VERYSILENT /SUPPRESSMSGBOXES /NORESTART`, and wait for the uninstaller's child process to exit.
  - Assert (I5): the install dir, `.lnk`, uninstall key and `$LOCALAPPDATA/micold-ai-ide/run` are gone, and both smoke markers still exist.

### Implementation for User Story 2

- [X] T044 [US2] [U38] [U39] [U40] [U41] [U42] [U61] R12, making T042 pass: in `packaging/windows/micold-ai-ide.iss`, add `AppMutex=Local\MicoldAIIDE`, `CloseApplications=force`, and `[UninstallDelete] Type: filesandordirs; Name: "{localappdata}\micold-ai-ide\run"`. Add a `[Code]` section:
  - `function StopDaemon(): String`. It reads `ExpandConstant('{localappdata}\micold-ai-ide\run\micold-daemon.pid')` with `LoadStringFromFile`. If a pid is present, it runs `powershell -NoProfile -Command "$p=Get-Process -Id <pid> -EA SilentlyContinue; if($p -and $p.Path -like '*\micold-daemon.exe'){Stop-Process -Id <pid> -Force; $p.WaitForExit(5000)}"` through `Exec(..., SW_HIDE, ewWaitUntilTerminated, rc)`.
    - If no pid record exists, it falls back to `taskkill /F /IM micold-daemon.exe /FI "USERNAME eq <user>"`, with `SW_HIDE`.
    - It returns `''` on success, or `'Micold AI IDE''s session service is still running. Close the application and choose Retry.'` if the process is still alive afterwards.
  - `function PrepareToInstall(var NeedsRestart: Boolean): String` returns `StopDaemon()`.
  - `function InitializeUninstall(): Boolean` calls `StopDaemon()` and shows the message with `MsgBox` on failure. It returns `False` unless in silent mode, so the user can retry.
  - `[Messages]` / `[CustomMessages]` add "Running sessions will be stopped." to the ready-page memo via `UpdateReadyMemo`.
- [ ] T045 [US2] [A6] [A7] [A8] [A9] Push. Observe T043's repair-with-live-daemon and uninstall assertions go green on both Windows packaging legs. If Restart Manager still reports files in use, diagnose from `install.log` and fix `StopDaemon`; do not relax the assertion.
- [X] T046 [P] [US2] Docs, US2: add sections to `docs/user-guide/install-windows.md`:
  - **Upgrading**: run the newer installer; open sessions end after you confirm; projects and settings carry over; one entry in Installed apps.
  - **Removing**: Settings → Apps → Installed apps → Micold AI IDE → Uninstall.
  - **What uninstall keeps**: `%APPDATA%\micold-ai-ide` and `%LOCALAPPDATA%\micold-ai-ide\data`, with how to delete them by hand for a full reset.
  - **Reinstalling the same version** repairs the install.

### Outer loop for User Story 2 (acceptance tests green before the story is complete)

- [ ] T070 [US2] [A6] US2-AS1: in the CI smoke run from T043, the repair over a live install exits 0 and leaves exactly one `_is1` uninstall key.
- [ ] T071 [US2] [A7] US2-AS2: in the same smoke run, uninstall removes the install dir, `.lnk`, uninstall key and `%LOCALAPPDATA%\micold-ai-ide\run`.
- [ ] T072 [US2] [A8] US2-AS3: in the same smoke run, both seeded data markers survive uninstall.
- [ ] T073 [US2] [A9] US2-AS4: in the same smoke run, installing with the daemon running ends with the old daemon pid gone.

**Checkpoint**: CI proves repair over a live daemon and uninstall with user data preserved, on both architectures.

---

## Phase 5: User Story 3 — Every release ships it automatically (P2)

**Goal**: `release.yml` builds, smoke-tests and attaches both setup executables to the draft. `publish` cannot run without them. The site's Windows links are checked against the real assets.

**Independent Test**: `release_publishes_complete_sets` passes. A fork dry-run (quickstart, "Release dry-run") shows both assets on the draft, and a forced arm64 failure leaves it a draft.

### Tests for User Story 3 (write first, observe failing) ⚠️

- [X] T047 [P] [US3] [U51] [A10] [A11] FR-014 / SC-002, coordinating with 028.
  - If `crates/micold-core/tests/release_publishes_complete_sets.rs` exists on `main` (PR #284 merged): rebase onto `main` and add a case asserting that `publish.needs` contains `windows` and that a job named `windows` exists with a matrix `include` of `x64`/`windows-latest` and `arm64`/`windows-11-arm`.
  - Otherwise: create that file with 028's rule, "the set of jobs whose steps contain `gh release upload` equals `publish.needs` minus `release-please`", plus the Windows case. Note in the file header that 028 introduces the same rule.
- [X] T048 [P] [US3] [U52] [U53] [U57] Release notice: create `crates/micold-core/tests/release_notice_windows.rs`. It asserts:
  - `.github/release-notice-windows.md` exists, is at most 5 non-empty lines, and mentions `x64`, `ARM64`, `More info` and `Run anyway`;
  - it links `install-windows`;
  - the `publish` job in `.github/workflows/release.yml` references `release-notice-windows.md`.
- [X] T049 [P] [US3] [A12] [U54] FR-017 site links: extend `scripts/tests/site-stage.test.sh` with a case where `MICOLD_RELEASE_ASSETS` lacks `micold-ai-ide-<v>-arm64-setup.exe` while `docs/user-guide/install-windows.md` links it. Assert `site/stage.sh` fails and names the missing asset. Add a passing case where both setup executables are present.

### Implementation for User Story 3

- [X] T050 [US3] [U51] [A10] [A11] R15, making T047 pass: add a job `windows` to `.github/workflows/release.yml` after `deb`:
  - `name: windows (${{ matrix.arch }})`, `needs: release-please`, `if: ${{ needs.release-please.outputs.release_created == 'true' }}`
  - `strategy.fail-fast: false`, `matrix.include: [{arch: x64, runner: windows-latest, target: x86_64-pc-windows-msvc}, {arch: arm64, runner: windows-11-arm, target: aarch64-pc-windows-msvc}]`
  - `runs-on: ${{ matrix.runner }}`, `permissions: contents: write`
  - steps: checkout, rust-toolchain and rust-cache, pinned by SHA like the `deb` job; `shell: bash` running `scripts/windows-installer.sh --arch ${{ matrix.arch }} --out-dir dist`; `scripts/windows-install-smoke.sh dist/*-setup.exe`; then `gh release upload "$TAG_NAME" dist/micold-ai-ide-*-${{ matrix.arch }}-setup.exe --clobber`
  - a header comment citing FR-014 and contracts/release-artifacts.md

  Change `publish.needs` to include `windows`, and update the file's top comment block to mention the Windows installers.
- [X] T051 [US3] [U52] [U53] [U57] Making T048 pass: create `.github/release-notice-windows.md` (≤ 5 lines: pick x64 or ARM64, SmartScreen **More info → Run anyway**, link to the install-windows page). In the `publish` job, append it to the release body before `gh release edit --draft=false`, using `gh release view "$TAG_NAME" --json body -q .body` concatenated with the notice file, passed to `gh release edit "$TAG_NAME" --notes-file`. If 028's macOS notice step exists, extend that step instead.
- [X] T052 [US3] [A12] [U54] Making T049 pass: make sure `site/stage.sh`'s release-download link scan (line ~209) covers `docs/user-guide/install-windows.md`. It already greps all sources, so only a change in its source set should be needed. Add the new page to the site's page set (`site/checks/page-set.sh` inputs / `docs/SUMMARY.md`, done in T041) so `site/checks/page-set.sh` passes.
- [ ] T053 [P] [US3] Docs, developer-facing (Principle VII): create `docs/development/windows-packaging.md`, covering:
  - how to build locally (`mise run windows-installer`; needs Inno Setup 6 and Git Bash);
  - what the `.iss` does, and why `AppId` must never change;
  - the CI legs (x64 in the `test` matrix, `windows-arm64-package` job) and the release `windows` job, and that it gates `publish`;
  - how to run `scripts/windows-install-smoke.sh` by hand;
  - the Windows daemon endpoint model (SID pipe name, owner-only DACL, pid record, image-checked stop);
  - a "Tests not run on Windows" section, left empty for T060 to fill.

  Add it to `docs/SUMMARY.md`. In `.github/workflows/ci.yml` job `docs`, add `test -f docs/development/windows-packaging.md` to "Required developer docs exist".

### Outer loop for User Story 3 (acceptance tests green before the story is complete)

- [X] T074 [US3] [A10] US3-AS1: `crates/micold-core/tests/release_publishes_complete_sets.rs` passes on the committed `.github/workflows/release.yml`: a `windows` job with the x64 and arm64 matrix runs `gh release upload`.
- [X] T075 [US3] [A11] US3-AS2: the same test passes its `publish.needs` contains `windows` case.
- [X] T076 [US3] [A12] US3-AS3: `scripts/tests/site-stage.test.sh` passes its missing-arm64-asset case against the committed `site/stage.sh`.
- [X] T077 [US3] [A13] US3-AS4: on the x64 Windows CI leg, `scripts/windows-installer.sh --arch x64` produces `micold-ai-ide-<version>-x64-setup.exe`. Record the CI run URL in the cycle log.

**Checkpoint**: the release workflow structurally cannot publish without both Windows assets. The notice and site links are verified by tests.

---

## Phase 6: User Story 4 — Install guide describes the Windows package (P3)

**Goal**: a first-time Windows user can go from the guide to a running session without any other page. The guide no longer says Windows has no packaged build, and it states the limits honestly.

**Independent Test**: quickstart M10, plus the docs CI job and the user-guide-updated gate.

### Tests for User Story 4 (write first, observe failing) ⚠️

- [X] T054 [P] [US4] [A14] [A15] [U56] [U58] [U59] SC-006: create `crates/micold-core/tests/install_guide_windows.rs`, a text scan. It asserts:
  - `docs/install.md` does not contain `no packaged build for macOS or Windows` or `no packaged build for Windows`, and it links `user-guide/install-windows.md`;
  - `docs/user-guide/install-windows.md` contains the headings `Download`, `Install`, `Upgrading`, `Removing` and `Limits`, and mentions `Smart App Control`, `SmartScreen`, `from source` and `logging out`.

  In `.github/workflows/ci.yml` job `docs`, add `test -f docs/user-guide/install-windows.md` to "Required user-guide docs exist".

### Implementation for User Story 4

- [X] T055 [US4] [A15] [U58] [U59] Making T054 pass: complete `docs/user-guide/install-windows.md` with a **Limits** section:
  - Smart App Control, when on, blocks the unsigned installer with no per-app override. Turning it off cannot be undone without resetting Windows. Such users should build from source.
  - Managed machines with AppLocker/WDAC may refuse it; ask IT, or build from source.
  - Sessions survive closing the window but not signing out, unless the service runs in the sandbox container. Link `../daemon.md` and `sandboxed-daemon.md`.
  - Unsupported architecture gives Inno's message.
  - Each Windows account runs its own session service, which no other account can connect to.
- [X] T056 [US4] [A14] [U56] Rewrite the "macOS and Windows" section of `docs/install.md`. Add a `## Windows` section that summarises and links `user-guide/install-windows.md`, with the two download links. Keep the from-source build instructions under a "Build from source" heading. Leave the macOS wording as it is on `main`, so 028 owns it; if 028 has merged, fit into its per-platform layout.
- [ ] T057 [P] [US4] Update `docs/daemon.md`:
  - the Windows endpoint is `\\.\pipe\Micold.Daemon.<SID>`, owner-only;
  - the pid record lives at `%LOCALAPPDATA%\micold-ai-ide\run\micold-daemon.pid`;
  - "Restart service" works on Windows;
  - the logout-survival limit is unchanged.

  Remove any "not yet on Windows" wording about the host placement.

### Outer loop for User Story 4 (acceptance tests green before the story is complete)

- [X] T078 [US4] [A14] US4-AS1: `crates/micold-core/tests/install_guide_windows.rs` passes its `docs/install.md` case: no "no packaged build" wording, and a link to the Windows guide.
- [X] T079 [US4] [A15] US4-AS2: the same test passes its logout-limit case on `docs/user-guide/install-windows.md`.

**Checkpoint**: the docs job is green, and the install guide covers download → running session → upgrade → removal → limits.

---

## Phase 7: Polish & Cross-Cutting Concerns

- [X] T058 Rebase coordination (research, "Coordination"): if PR #284 merged during this work, rebase onto `main`. Reconcile:
  - `release.yml` `publish.needs` and notice steps;
  - `packaging_excludes_showcase.rs`, using 028's shared helpers rather than duplicates;
  - `release_publishes_complete_sets.rs`;
  - the docs-job required-page lists;
  - `docs/install.md`.

  Re-run `mise run test`.
- [X] T059 [P] Update `specs/030-windows-installer/contracts/release-artifacts.md` job graph and artifact table if T058 changed any job name. Update `specs/028-macos-package/contracts/release-artifacts.md` with the Windows rows only if 028 is merged (normative single source).
- [ ] T060 [P] Fill "Tests not run on Windows" in `docs/development/windows-packaging.md` with each file T027 left wholly `#![cfg(unix)]` and its `// unix-only:` reason.
- [ ] T061 Security review of the Phase 2 unsafe code in `endpoint.rs`, `singleton.rs`, `spawn.rs`, `process.rs`, `win_job.rs` and `platform/windows.rs`. Check that every handle is closed via RAII, `LocalFree` runs on every path, there are no panics across FFI, the SDDL is built only from the token SID, and `terminate_daemon` cannot act on a non-`micold-daemon.exe` image. Record the findings in the PR description.
- [ ] T062 Full gate before the final push: `cargo fmt --check`, `mise run test`, `cargo check --target x86_64-pc-windows-msvc --workspace`, `cargo check --target aarch64-apple-darwin --workspace`, and `scripts/tests/windows-installer.test.sh`. Then confirm all CI jobs are green, including `ci complete`, the Windows `test` leg and `windows-arm64-package`.
- [ ] T063 Manual quickstart Part M, rows M1–M10 in `specs/030-windows-installer/quickstart.md`, run on a standard-user Windows 11 x64 VM and an ARM64 device or VM with the CI-uploaded setup executables. Record one result line per row in the PR description. M7 needs a second local account.
- [ ] T064 [P] Update `specs/030-windows-installer/research.md` with any decision that changed during implementation, for example if cross-compiling ARM64 replaced native `windows-11-arm`. Update `plan.md`'s Technical Context if versions moved.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: none. T006 depends on T005, because the guard must be green before the CI leg change.
- **Foundational (Phase 2)**: depends on T001 and T003 through T006. **Blocks every story**, because an installed app whose daemon cannot connect fails US1.
- **US1 (Phase 3)**: depends on Phase 2. T034 depends on T002.
- **US2 (Phase 4)**: depends on US1's `.iss` (T035) and smoke script (T033), and uses `announce_running` (T021, T024).
- **US3 (Phase 5)**: depends on US1's scripts (T036, T033). US2 is not required, but the release smoke gets US2's assertions for free if it lands first.
- **US4 (Phase 6)**: depends on T041 (the page exists) and on T046 (the Upgrading/Removing headings T054 checks).
- **Polish (Phase 7)**: after all stories.

### Within Phase 2

- T007 → T016
- T008 and T009 → T017, which needs T016
- T010 → T018 and T019
- T011 → T019
- T012 → T020, which needs T003
- T013 and T014 → T021 → T022
- T015 → T023
- T024 needs T021
- T026 and T027 need T016–T020
- T028 comes last

### Parallel Opportunities

- **Phase 1**: T003 and T004 in parallel; T002 is independent research.
- **Phase 2 tests**: T007–T015 touch different files and can be written in parallel.
- **Phase 2 implementation**: T020 (daemon platform) and T021/T022 (core process/spawn sites) run in parallel with T016–T019 once their tests exist.
- **US1 tests**: T029–T033 in parallel. T041 (docs) runs in parallel with implementation.
- **After US1**: US2 (T042–T046) and US3 (T047–T053) can proceed in parallel, since they touch different files except the smoke script. T043 edits `windows-install-smoke.sh`, and T050 only calls it.
- **Polish**: T059, T060 and T064 in parallel.

---

## Parallel Example: Phase 2 tests

```text
Task: "T007 endpoint resolve tests in crates/micold-core/src/endpoint.rs"
Task: "T008 pipe ACL test in crates/micold-daemon/tests/windows_pipe_acl.rs"
Task: "T010 daemon stop tests in crates/micold-daemon/tests/daemon_stop.rs"
Task: "T013 console guard in crates/micold-core/tests/background_spawns_hide_console.rs"
Task: "T014 process module tests in crates/micold-core/src/process.rs"
```

## Parallel Example: User Story 1 tests

```text
Task: "T029 windows_violations in crates/micold-client/tests/packaging_excludes_showcase.rs"
Task: "T030 per-user scan in crates/micold-core/tests/windows_installer_is_per_user.rs"
Task: "T031 version scan in crates/micold-core/tests/windows_installer_version_is_injected.rs"
Task: "T032 script tests in scripts/tests/windows-installer.test.sh"
Task: "T033 smoke assertions in scripts/windows-install-smoke.sh"
```

---

## Implementation Strategy

### MVP (Phase 1 → Phase 2 → US1)

1. Setup, then Foundational. **Validate**: the Windows daemon suite is green in CI.
2. US1. **Validate**: both Windows packaging legs are green, and the CI artifact installs and launches on a VM (M1–M2).
3. The MVP is demonstrable from a CI artifact, but it is not releasable yet: nothing attaches it to releases.

### Incremental delivery (all in one PR, per user decision 2026-09-13)

1. US2: repair and uninstall are proven in CI.
2. US3: the release gate and notice.
3. US4: the complete guide.
4. Polish: rebase with 028, security review, manual Part M.

### Commit cadence

- One commit per task or per red → green pair, with Conventional Commit prefixes `feat(030)`, `test(030)`, `ci(030)`, `docs(030)`.
- Push the red on its own commit wherever the red is only observable on Windows CI.

---

## Notes

- Keep each [P] task to different files with no incomplete dependency.
- Never weaken a smoke assertion to get green; fix the cause.
- Process kills in tests and scripts target only pids the test or script itself spawned.
