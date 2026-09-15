# Research: Windows Installation Package

**Feature**: [spec.md](spec.md) | **Plan**: [plan.md](plan.md) | **Date**: 2026-09-13

The Technical Context left no NEEDS CLARIFICATION behind. The work splits into two phases, and this
file records each design decision in phase order.

- **Phase A** (R1–R7) makes the host daemon work on Windows. The installer depends on it.
- **Phase B** (R8–R16) builds the installer and wires it into CI, the release and the docs.

---

## Phase A — the host daemon on Windows

### What is broken today (grounding)

The spec's original assumption was that the app "already works on Windows". That turned out to be
false. The code shows:

| Gap | Where | Effect |
|---|---|---|
| Endpoint resolution is a stub | `crates/micold-core/src/endpoint.rs` `user_sid()` returns `Unsupported` ("lands with … T083/W5") | The client can never resolve the pipe name, so no session ever connects |
| No pid record | `endpoint.rs` sets `lock_path = PathBuf::new()` on Windows; `server.rs` pid write fails with a warning | "Restart service" cannot find the daemon |
| Cannot stop the daemon | `crates/micold-core/src/spawn.rs` `#[cfg(not(unix))] terminate_daemon` returns `Unsupported` | Restart service fails, and the installer cannot stop the old version |
| Cannot reap session trees | `crates/micold-daemon/src/platform/windows.rs` `terminate_process_tree` is a no-op | Killing a session leaves its agent and shell processes behind |
| Singleton lock is bogus | `crates/micold-daemon/src/singleton.rs` Windows `acquire` opens `temp_dir()` as a file | Opening a directory as a file fails on Windows, so bind errors out |
| Console flashes | No `windows_subsystem`; git, powershell and docker spawns lack `CREATE_NO_WINDOW` | The GUI opens a console, and every background command flashes a window |
| Never tested | `ci.yml` runs the daemon crate's tests on Linux only (`cargo test --workspace`, line ~230) | None of the above was ever caught |

### R1. Endpoint name: per-user named pipe keyed by the user SID

**Decision**: Keep the reserved name `\\.\pipe\Micold.Daemon.<user-SID>` and implement `user_sid()`
with `windows-sys`:

1. `OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY)`
2. `GetTokenInformation(TokenUser)`
3. `ConvertSidToStringSidW`
4. `LocalFree`

This needs the `Win32_Security`, `Win32_Security_Authorization` and `Win32_System_Threading`
features on the workspace `windows-sys` entry.

**Rationale**:

- The SID is the one stable, unforgeable per-account identifier. The user name can be renamed or
  collide across domains.
- Named pipes are Windows' native local IPC. `interprocess` 2.4.2, already the transport on Unix,
  supports them with the same `GenericFilePath` name type the code already uses.
- Keying the name by SID gives each account its own daemon (FR-022) and keeps two accounts on one
  machine from colliding (edge case).

**Alternatives considered**:

- **Loopback TCP**: rejected. Any local process of any user can connect, so we would need an
  authentication layer. Port 7727 stays reserved for the sandbox placement.
- **AF_UNIX sockets on Windows 10+**: rejected. Tokio support is incomplete, and the ACL semantics
  differ from what the Unix code assumes.
- **Keying by user name**: rejected for the reasons above.

### R2. Owner-only access: protected DACL on the pipe

**Decision**: Build the listener with
`interprocess::os::windows::local_socket::ListenerOptionsExt::security_descriptor`. The descriptor
comes from `SecurityDescriptor::deserialize` on the SDDL string `D:P(A;;GA;;;<owner-SID>)`:

- `D:P` makes the DACL protected, so it inherits no ACEs.
- A single ACE grants generic-all to the owner's SID.
- Nothing else is granted: no Everyone, Administrators, SYSTEM or LocalService entries.

Remote clients are also rejected. `interprocess` sets `PIPE_REJECT_REMOTE_CLIENTS`
(`named_pipe/listener/create_instance.rs:101`).

**Rationale**:

- The default named-pipe DACL grants read access to Everyone and full control to Administrators and
  LocalSystem. That would let another account open the pipe, which violates FR-021 and SC-009.
- SDDL keeps the rule to one reviewable line. The descriptor is built from the same SID R1 resolves.
- An admin could still take ownership and change the DACL. That matches the Unix model, where root
  can read a 0600 socket, and is out of scope.

**Verification**: an integration test on the Windows CI leg reads the created pipe's descriptor back
with `GetSecurityInfo` and asserts it holds exactly one ACE, for the current user's SID.

**Alternatives considered**:

- **Checking the client SID after connect** (`ImpersonateNamedPipeClient` or peer credentials):
  kept only as a possible second layer. The DACL refuses the open before any bytes flow, which is
  strictly stronger. We do not plan to add it.
- **Building the ACL with `SetEntriesInAclW`**: rejected. It works but takes more unsafe code for
  the same result.

### R3. One daemon per user: first-pipe-instance, not a lock file

**Decision**:

- The Windows `singleton::acquire` drops the fake `_lock` file. Uniqueness comes from the pipe
  itself: `interprocess` creates the first server instance with `FILE_FLAG_FIRST_PIPE_INSTANCE`
  (`create_instance.rs:86`), so a second daemon's bind fails with access-denied (`AddrInUse` or
  `PermissionDenied`), which maps to `Acquisition::AlreadyRunning`.
- `BoundListener._lock` becomes `Option<File>`, which is `None` on Windows.
- The existing live-probe before bind is kept (`is_live`).
- A new test asserts that a second concurrent acquire returns `AlreadyRunning` on all three
  platforms.

**Rationale**:

- A pipe name disappears when its last handle closes, including on crash. So unlike a Unix socket
  file there is nothing stale to reclaim (edge case: daemon crash / stale marker).
- The race between two simultaneous spawns is settled by the kernel.

**Alternatives considered**:

- **A named mutex as the singleton**: rejected. It would duplicate the pipe's guarantee.
  A mutex is still used for installer detection (R12), but that one is advisory.

### R4. Pid record location and stale handling

**Decision**:

- On Windows, `Endpoint.lock_path` becomes
  `%LOCALAPPDATA%\micold-ai-ide\run\micold-daemon.pid`. It is derived from `ProjectDirs` `data_local_dir`'s
  parent plus `run`, and the directory is created on resolve.
- The daemon writes its pid after a successful bind, as `server.rs` already does, and leaves the
  file in place when it stops. It has no clean exit, and on Unix the file is also the `flock` file,
  so unlinking it would let a second daemon lock a new inode (user decision 2026-09-14, U9 dropped).
- `running_daemon_pid` treats a pid as current only when both hold:
  1. the pipe is live (`is_live`), and
  2. the process image for that pid is `micold-daemon.exe` (see R5).

**Rationale**:

- `%LOCALAPPDATA%` is per-user and not roamed. This follows the daemon log's placement.
- Pid reuse is real on Windows. Checking the image name before acting on a stale file avoids
  terminating an unrelated process.

**Alternatives considered**:

- **`%TEMP%`**: rejected, because cleanup tools purge it.
- **Asking the daemon for its pid over IPC**: rejected. A version-mismatched client cannot complete
  the handshake (`handshake.rs:113-133`), and that is exactly the case where Restart service is
  needed.

### R5. Stopping the daemon (FR-023)

**Decision**: Implement the Windows `terminate_daemon(pid)` in `spawn.rs`:

1. `OpenProcess(PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION | SYNCHRONIZE, pid)`
2. `QueryFullProcessImageNameW`. The file name must equal `micold-daemon.exe`; otherwise refuse
   with `InvalidData`.
3. `TerminateProcess(h, 0)`
4. `WaitForSingleObject(h, 5s)`

The existing caller already polls until the endpoint goes dead.

**Rationale**:

- Windows has no SIGTERM for a windowless, consoleless process. `GenerateConsoleCtrlEvent` needs a
  shared console, which the daemon (DETACHED_PROCESS) does not have.
- The daemon's state is already safe against hard termination:
  - Session metadata is persisted as it changes (feature 010).
  - Scrollback loss on kill is the same outcome Unix accepts after SIGTERM's grace period.
- Graceful drain is out of scope, matching Unix, where the SIGTERM handler only exits.

**Alternatives considered**:

- **A protocol `Shutdown` request**: rejected. A mismatched client cannot handshake, and adding a
  pre-handshake message would widen the protocol surface.
- **`taskkill /F /PID`**: rejected. It spawns a process and does no image check.

### R6. Session process-tree cleanup

**Decision**:

- `platform/windows.rs` `terminate_process_tree` assigns each PTY child to a Job Object created with
  `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, and terminates the job.
- `env_include.rs` already has this pattern: `JobHandle` (`:147-210`, in micold-core). It moves to a
  shared `micold-core::win_job` module that both call sites use.
- Assignment happens right after `portable-pty` spawns. `portable-pty` exposes the child's pid, and
  `OpenProcess` plus `AssignProcessToJobObject` covers it. Grandchildren that start afterwards
  inherit the job.

**Rationale**:

- Windows has no process groups that `kill` can target.
- A job kills the whole agent tree (claude, node, shells) in one call. When the daemon itself is
  terminated, the job handles close and the trees die with it, which replaces the no-op.

**Known limit (documented, not fixed)**: a grandchild spawned in the few milliseconds before the
assignment escapes the job. ConPTY's `conhost` wrapper spawns first, so the practical window is the
shell's own start. This is accepted.

**Alternatives considered**:

- **Walking the tree with `CreateToolhelp32Snapshot`**: rejected, because it races with processes
  that are still spawning.

### R7. No console windows (FR-005, FR-024)

**Decision**, in three parts:

1. **Release-only GUI subsystem.** Both bins get
   `#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]`:
   - `crates/micold-client/src/main.rs`
   - `crates/micold-daemon/src/main.rs`

   Debug builds keep their console, so `mise run run` and `mise run daemon` still print. For a
   foreground debug run of a release daemon, `MICOLD_LOG` already routes to the log file.
2. **One no-window spawn helper.** Add `micold_core::process::no_window(&mut Command) -> &mut Command`.
   - On Windows it applies `creation_flags(CREATE_NO_WINDOW)`. It is a no-op elsewhere.
   - It has a `tokio::process::Command` twin.
   - Every background spawn goes through it:
     - `git.rs` run_git, branch_exists and submodule
     - `env_include.rs` run_bounded (powershell)
     - `sandbox/exec.rs` docker and podman
   - A core test enumerates spawn sites with a source scan, the same way
     `packaging_excludes_showcase.rs` scans. It fails if any `Command::new` in
     `crates/*/src/**/*.rs` outside an allowlist (the PTY and the detached daemon spawn) lacks a
     `no_window(` call on the same builder chain.
3. **The daemon spawn stays `DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP`.** `CREATE_NO_WINDOW` is
   ignored when combined with `DETACHED_PROCESS`. With a GUI-subsystem daemon exe there is no
   console to create anyway.

**Fatal startup errors**: once the daemon has no console, the `eprintln!` in `main.rs:8` goes
nowhere. The daemon's `main` therefore also appends the fatal line to the daemon log path before it
exits. A test covers this by invoking the library `run` failure path, not the bin.

**Rationale**:

- `windows_subsystem` is the only way to stop the loader from allocating a console for an exe that
  Explorer or the Start menu launches.
- Children of a GUI-subsystem parent get a new visible console unless they are given
  `CREATE_NO_WINDOW`. That is the "flash".
- ConPTY works from GUI processes, so terminals are unaffected.

**Alternatives considered**:

- **Setting the subsystem in debug builds too**: rejected, because it breaks developer logging.
- **Calling `FreeConsole()` at startup**: rejected. The console window still flashes before it runs.
- **Hiding windows through `STARTUPINFO` `SW_HIDE`**: rejected. `std::process` does not expose it.

---

## Phase B — the installer

### R8. Installer technology: Inno Setup 6

**Decision**: Build a setup `.exe` with Inno Setup 6. The script is `packaging/windows/micold-ai-ide.iss`.

**Rationale**:

- **Preinstalled.** Inno Setup 6.7.1 is on GitHub's `windows-2022`, `windows-2025`/`windows-latest`
  and `windows-11-arm` images, so CI needs nothing extra.
- **Per-user and no admin is first-class** (FR-004):
  - `PrivilegesRequired=lowest`
  - `{autopf}` resolves to `%LOCALAPPDATA%\Programs`
  - HKCU uninstall registration
- **Built-in upgrade, uninstall and in-use handling** (FR-006, FR-008, FR-009): a fixed `AppId`
  means one Installed-apps entry across versions, and `AppMutex` plus Restart Manager detect a
  running app.
- **Silent flags for CI**: `/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /LOG=`.
- **License**: a permissive, zlib-style license. Commercial licensing is requested, not required.
- **Precedent**: Zed ships the same way.

**Alternatives considered**:

| Option | Rejected because |
|---|---|
| WiX / MSI (`cargo-wix`) | `cargo-wix` targets per-machine installs and is poorly maintained. Per-user MSI triggers ICE validation issues. WiX v6+ adds the OSMF maintenance-fee EULA. |
| NSIS | Not preinstalled on `windows-latest`. Files-in-use handling needs third-party plugins. |
| MSIX | Unsigned MSIX will not install without developer mode, which conflicts with FR-010's unsigned decision. |
| Zip archive only | No Start menu entry, no Installed-apps registration and no upgrade, so it fails FR-003, FR-006 and FR-008. |
| winget / Microsoft Store | Out of scope (spec Assumptions). A future winget manifest can point at this installer. |

### R9. Architectures and build strategy (FR-015)

**Decision**: Build and test each architecture natively.

| Arch | Target | Runner | Inno directive |
|---|---|---|---|
| x64 | `x86_64-pc-windows-msvc` | `windows-latest` | `ArchitecturesAllowed=x64compatible and not arm64` / `ArchitecturesInstallIn64BitMode=x64compatible` |
| ARM64 | `aarch64-pc-windows-msvc` | `windows-11-arm` | `ArchitecturesAllowed=arm64` / `ArchitecturesInstallIn64BitMode=arm64` |

**Rationale**:

- `windows-11-arm` is free for public repositories, and this repository is public. Native building
  also lets the ARM64 installer be *installed and launched* in CI, not just compiled.
- `aarch64-pc-windows-msvc` has been Tier 1 since Rust 1.91.
- Inno's setup stub is x86 and runs under emulation on ARM64. That is fine.
- The `and not arm64` clause stops the x64 package from installing on ARM64 through emulation. An
  ARM64 user gets a clear "this package is for a different architecture" message (spec edge case).

**Alternatives considered**:

- **Cross-compiling ARM64 on x64**: works, since the MSVC ARM64 tools are present, but leaves the
  ARM64 installer untested. Kept as a fallback if the ARM runner queue proves unreliable.
- **One installer carrying both architectures**: rejected. It doubles the download for everyone,
  for no user benefit.

### R10. Install layout, registration and user data (FR-002, FR-003, FR-006, FR-007)

**Decision**:

- `DefaultDirName={autopf}\Micold AI IDE`, which gives `%LOCALAPPDATA%\Programs\Micold AI IDE\`.
  Only two files land there: `micold-ai-ide.exe` and `micold-daemon.exe`, as siblings, plus the
  uninstaller. The daemon therefore resolves as the sibling of `current_exe()`, with no
  `MICOLD_DAEMON_BIN` and no PATH lookup.
- The Start menu group `{autoprograms}\Micold AI IDE.lnk` points at the client. There is no desktop
  icon by default; it is available as an optional task, unchecked.
- The uninstaller removes only installed files. `[UninstallDelete]` covers only
  `%LOCALAPPDATA%\micold-ai-ide\run`, the pid record from R4. User data survives uninstall and
  reinstall (FR-007):
  - `%APPDATA%\micold-ai-ide\` holds settings, projects and hooks.
  - `%LOCALAPPDATA%\micold-ai-ide\data\` holds the logs.
- `AppId` is a fixed GUID recorded in the contract. Changing it would create a second Installed-apps
  entry. A test pins it.
- `AppVersion` and `VersionInfoVersion` are passed in with `/DAppVersion=<workspace version>` from
  the build task. Nothing is hardcoded (FR-013).

**Rationale**: this mirrors the `.deb`, which ships the two binaries plus a launcher entry and never
touches `~/.local/share`.

### R11. Executable icon and version resource (FR-003)

**Decision**:

- Add `embed-resource` 3.x as a `build-dependency` of `micold-client` and `micold-daemon`.
- Each crate gets a `build.rs` that, for `CARGO_CFG_TARGET_OS == "windows"`, compiles a `.rc`
  referencing the existing `assets/icon/icon.ico`.
- The installer uses the same `.ico` for `SetupIconFile` and `UninstallDisplayIcon`.

**Rationale**:

- `embed-resource` is MIT-licensed and actively maintained. It no-ops on non-Windows hosts and uses
  `llvm-rc` when cross-compiling, so Linux and macOS builds are unaffected.
- Only the icon is embedded. The About dialog already reads `CARGO_PKG_VERSION`.
- **Vetting (constitution)**: it is a build-time dependency only and adds no runtime code. The
  implementing task records `cargo tree -e build -i embed-resource` output and the licenses of any
  new transitive crates in the PR description before the dependency is added.

**Alternatives considered**:

- **`winresource`**: rejected. It needs a manual target-OS guard and is less maintained.
- **Hand-invoking `rc.exe`**: rejected, because it is not portable to cross builds.

#### Dependency vetting

Recorded for T002 on 2026-09-14, before the dependency is added. `cargo tree -e build -i
embed-resource --target x86_64-pc-windows-msvc`, run in a scratch crate with only
`[build-dependencies] embed-resource = "3"`, printed this with `-f '{p} {l}'` and `-e normal,build`
to show licenses (the crate's own root line omitted):

```text
embed-resource v3.0.11 MIT
├── cc v1.4.6 MIT OR Apache-2.0
│   ├── find-msvc-tools v0.1.12 MIT OR Apache-2.0
│   └── shlex v2.0.1 MIT OR Apache-2.0
├── memchr v2.8.3 Unlicense OR MIT
├── rustc_version v0.4.1 MIT OR Apache-2.0
│   └── semver v1.0.28 MIT OR Apache-2.0
└── toml v1.1.6+spec-1.1.0 MIT OR Apache-2.0
    ├── serde_core v1.0.229 MIT OR Apache-2.0
    ├── serde_spanned v1.1.1 MIT OR Apache-2.0
    │   └── serde_core v1.0.229 MIT OR Apache-2.0
    ├── toml_datetime v1.1.1+spec-1.1.0 MIT OR Apache-2.0
    │   └── serde_core v1.0.229 MIT OR Apache-2.0
    ├── toml_parser v1.1.3+spec-1.1.0 MIT OR Apache-2.0
    │   └── winnow v1.0.4 MIT
    ├── toml_writer v1.1.2+spec-1.1.0 MIT OR Apache-2.0
    └── winnow v1.0.4 MIT
```

- **Last release**: 3.0.11 on 2026-07-02, after 3.0.9 (2026-04-24), 3.0.8 (2026-03-23) and 3.0.7
  (2026-03-15). The repository is `nabijaczleweli/rust-embed-resource`.
- **New to `Cargo.lock`**: `embed-resource`, `toml`, `serde_spanned` and `toml_writer`, all MIT or
  MIT OR Apache-2.0. Every other crate above is already locked, some at an older patch version that
  Cargo may unify upward. No license outside MIT, Apache-2.0 and Unlicense enters.
- **Build time only**: nothing reaches the shipped binaries' runtime code.
- **Added with T034**: adding the dependency also locked `vswhom` 0.1.0, `vswhom-sys` 0.1.3 and
  `winreg` 0.55.0, all MIT. They are `embed-resource`'s dependencies on a Windows MSVC build host,
  which the tree above, resolved on a Linux host, did not show. They find `rc.exe` on the CI
  Windows runners. `toml_parser` moved from 1.1.2 to 1.1.3. A host with no resource compiler (this
  repository's Linux cross-checks) builds the Windows exes without an icon and prints a
  `cargo:warning`; the packaging smoke's A17 check fails such an exe.

### R12. Upgrading or uninstalling while the app or daemon runs (FR-008, FR-009, edge cases)

**Decision**, in three layers:

1. **`AppMutex=Local\MicoldAIIDE`**. The client creates this named mutex at startup, in a
   Windows-only `micold-core::process::announce_running()`. (Revised after A6: the daemon held it
   too at first, and a repair with only the daemon running cancelled at the prompt, before
   `PrepareToInstall` could stop the daemon. See layer 3.) The installer then says
   "Micold AI IDE is running, close it first" and offers Retry or Cancel.
   `Local\` scopes the mutex to the session, so another account's running app does not block this
   account's install.
2. **`CloseApplications=force`, `RestartApplications=no`**. When the user continues, Restart Manager
   closes a still-open client window gracefully.
3. **Explicit daemon stop in `[Code] PrepareToInstall` and `InitializeUninstall`**. Restart Manager
   cannot gracefully close a windowless detached process, and the daemon may hold the exe open. The
   installer therefore:
   - reads `%LOCALAPPDATA%\micold-ai-ide\run\micold-daemon.pid`;
   - terminates the process only if its image path is under `{app}`;
   - otherwise falls back to `taskkill /F /IM micold-daemon.exe /FI "USERNAME eq %USERNAME%"`, scoped
     to the user.

   Running sessions end. The wizard page states this before the user confirms (FR-009).

**Rationale**:

- This keeps the running-app prompt honest: it names the consequence, which is that sessions stop.
- It also closes the stale old-version daemon case. A new client meeting an old daemon would
  otherwise hit a version mismatch, and upgrade always stops the old one.

**Alternatives considered**:

- **Only `CloseApplications=force`**: rejected. It leaves the daemon holding the exe, so file
  replacement fails and the installer asks for a reboot.
- **Scheduling replacement on reboot**: rejected, because it violates "no restart".

### R13. Keeping the showcase out (FR-012)

**Decision**: Extend `crates/micold-client/tests/packaging_excludes_showcase.rs` with
`windows_violations(iss, showcase, shipped)`. It follows PR #284's `macos_violations`:

- The `[Files]` `Source:` entries of the `.iss` file must name exactly `BUNDLED_BINS` with `.exe`.
- No wildcard `Source:` is allowed.
- Comment lines (`;`) are stripped before scanning.

The mise build task also builds only `-p micold-client --bin micold-ai-ide` and `-p micold-daemon`.

**Rationale**: it is the same guard shape the `.deb` and macOS packages use, so one test file owns
the rule.

### R14. CI: per-change package, install and launch (FR-018, FR-025, SC-007)

**Decision**: Make three changes to `ci.yml`.

1. **Daemon tests on Windows.** The `windows-latest` leg of the `test` matrix adds
   `cargo test -p micold-daemon --all-targets`. It runs `singleton`, `server`, `endpoint` and the
   `connect` integration tests against a real pipe (FR-025). Tests that genuinely need Unix keep
   `#[cfg(unix)]`, and each one is listed in the contract.
2. **A new step in the same matrix leg**, "Package, install and launch the Windows installer", with
   `if: runner.os == 'Windows'`. It follows PR #284's in-matrix macOS step, so `ci-complete` and
   `ci_gate_covers_every_job.rs` need no change. The step:
   1. Runs `mise run windows-installer`, or the equivalent script under bash.
   2. Runs `setup.exe /VERYSILENT /SUPPRESSMSGBOXES /NORESTART /LOG=install.log`.
   3. Asserts the installed files and the Start menu `.lnk` exist, and the HKCU uninstall key holds
      the right `DisplayVersion`.
   4. Launches the installed client. It then waits up to 20 s for `\\.\pipe\Micold.Daemon.<SID>` to
      appear, which proves the client spawned the sibling daemon without a console.
   5. Asserts no `conhost.exe` child of the client exists, which is the console-flash check.
   6. Stops the client with `taskkill /PID`, a process this job started itself.
   7. Silently uninstalls, then asserts files and the uninstall key are gone and `%APPDATA%` is
      intact.

   A graphics-surface failure of the headless GUI counts as a pass when the daemon endpoint
   appeared, which is the same escape hatch as PR #284's R12.
3. **An ARM64 smoke leg.** It is a second job, `windows-arm64-package`, on `windows-11-arm`, that
   runs the same script. It must be listed in `ci-complete` `needs`, which
   `ci_gate_covers_every_job.rs` enforces.

**Rationale**:

- Silent install exercises the real installer logic, the same code path a user takes, without a
  desktop.
- Pipe appearance is the observable proof that FR-020 holds in an installed layout.

**Evidence** (T040): in PR #332's CI run 34841225620 (e002ec1b), the daemon's pipe appeared **0 s**
after the installed client started on x64 (`build + test (windows-latest)`) and **1 s** after on
ARM64 (`package + smoke (windows-11-arm)`), against the 20 s bound. Run 34840058051 (bbaa2dab) also
saw 1 s on ARM64. Both legs found no `conhost.exe` under the client or the daemon.

**Alternatives considered**:

- **Adding `windows-11-arm` as a third `os` value of the test matrix**: rejected. It would triple the
  ARM64 runner time by also running the ~35 client gate tests, and the smoke alone is what ARM64
  needs.

### R15. Release: attach before publish (FR-001, FR-014, SC-005)

**Decision**: Add a `windows` matrix job to `release.yml`:

- `include: [{arch: x64, runner: windows-latest}, {arch: arm64, runner: windows-11-arm}]`
- It needs `release-please` and runs if `release_created`.
- Steps: build release, run `iscc`, run the silent install smoke (same script as R14), then
  `gh release upload "$TAG_NAME" <exe> --clobber`.

Assets:

- `micold-ai-ide-<version>-x64-setup.exe`
- `micold-ai-ide-<version>-arm64-setup.exe`

`publish.needs` gains `windows`. `release_publishes_complete_sets.rs` from PR #284 already enforces
that `needs` equals the upload jobs. `.github/release-notice-windows.md` explains SmartScreen, and
publish appends it to the release body next to the macOS notice.

**Naming rationale**:

- The names follow the `.deb` lowercase-hyphen style and state architecture and kind explicitly.
- `site/stage.sh` checks download links against the asset list, so the site's Windows links are
  verified (FR-017).

### R16. Unsigned distribution and its limits (FR-010)

**Decision**: Ship unsigned, as the user chose. `docs/user-guide/install-windows.md` documents:

- **SmartScreen**: "Windows protected your PC", then *More info*, then *Run anyway*. Include a
  screenshot description and why the warning appears.
- **Smart App Control**, when enabled on Windows 11: it blocks unsigned apps with **no** per-app
  override. The only way through is to turn Smart App Control off, and it cannot be turned back on
  without resetting Windows. We state this plainly and recommend building from source for such
  machines.
- **Corporate AppLocker / WDAC policy**: the installer is refused, and the user should talk to IT
  (edge case).
- **Mark of the Web**: `Unblock-File` is *not* needed for the installed exes, because Inno writes
  them fresh.

**Rationale**: Principle VII requires honest user docs. Smart App Control is the one case a user
cannot click through, so it has to be documented rather than discovered.

---

## Coordination with feature 028 (PR #284)

The two features touch the same files:

- `release.yml` publish `needs` and the release-notice step
- `packaging_excludes_showcase.rs`
- `release_publishes_complete_sets.rs`, which exists only in PR #284
- the `ci.yml` docs job's required-page list
- `docs/install.md` and `docs/SUMMARY.md`

**Decision**:

- Whichever PR merges second rebases onto the first. If PR #284 has merged by implementation time,
  030 extends its tests and contract in place.
- If it has not, 030 introduces `release_publishes_complete_sets.rs` and the
  `docs/user-guide/install-<platform>.md` layout itself, with identical shape, so the later rebase
  is a trivial merge.
- The contract `contracts/release-artifacts.md` in this feature is written as the extension of
  028's, adding the Windows rows.
