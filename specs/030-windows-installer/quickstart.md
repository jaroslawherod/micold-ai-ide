# Quickstart: validating the Windows installation package

**Feature**: [spec.md](spec.md) | Contracts:

- [windows-endpoint](contracts/windows-endpoint.md)
- [windows-installer](contracts/windows-installer.md)
- [release-artifacts](contracts/release-artifacts.md)

Part A is automated: it runs on every change in CI and can be reproduced locally. Part M is manual,
done once before merge on real Windows machines, and covers what a headless runner cannot show.

## Part A: automated

### A1. Guard tests (any OS, including Linux)

```bash
mise run test-core     # windows_installer_is_per_user, windows_installer_version_is_injected,
                       # background_spawns_hide_console, daemon_tests_gate_with_reason,
                       # release_publishes_complete_sets
cargo test -p micold-client --test packaging_excludes_showcase
```

**Expected**: all tests pass. Each test fails if the property it guards is broken, for example:

- a wildcard `Source:` in the `.iss`
- a bare `Command::new("git")` without `no_window`
- `windows` missing from `publish.needs`

### A2. Cross-check that Windows arms compile (from Linux)

```bash
cargo check --target x86_64-pc-windows-msvc -p micold-core -p micold-daemon   # needs the target installed
cargo check --target aarch64-apple-darwin  -p micold-core -p micold-daemon     # memory: cfg arms break macOS too
```

**Expected**: both targets compile. A2 is a fast pre-push check; CI (A3) is authoritative.

### A3. Windows daemon suite (Windows, x64 or ARM64)

```powershell
mise run test-core
cargo test -p micold-daemon --all-targets
```

**Expected**: everything in [windows-endpoint](contracts/windows-endpoint.md) E1–E7 passes. In
particular:

- `windows_pipe_acl` shows one ACE, for your SID.
- `daemon_singleton` converges on one daemon.
- `daemon_stop` ends the endpoint within 5 s.
- `autospawn` connects to a real spawned daemon.

### A4. Build, install, launch and uninstall (Windows)

```bash
mise run windows-installer                      # → <target-dir>/windows-installer/micold-ai-ide-<v>-<arch>-setup.exe
scripts/windows-install-smoke.sh <that .exe>    # what CI runs
```

**Expected**: the smoke script exits 0, having checked in order:

1. The silent install exits with 0.
2. Both exes are in `%LOCALAPPDATA%\Programs\Micold AI IDE\`. The Start menu `.lnk` exists. The HKCU
   uninstall key has `DisplayVersion` equal to the workspace version.
3. Both release exes have PE subsystem 2 (GUI).
4. The installed client starts, and `\\.\pipe\Micold.Daemon.<SID>` appears within 20 s.
5. The client process has no `conhost.exe` child.
6. Installing the same version again (a repair) still leaves one uninstall key.
7. The silent uninstall exits with 0. The install dir, `.lnk`, key and `run\` are gone, and a marker
   file pre-seeded in `%APPDATA%\micold-ai-ide\data\` is still present.

## Part M: manual (before merge)

Use a Windows 11 x64 machine or VM with a **standard (non-admin)** account, and an ARM64 device or VM
for M6. Download both setup executables from the PR's CI artifacts, or build them with A4.

| # | Steps | Pass when |
|---|---|---|
| M1 | Double-click the x64 setup. At SmartScreen, click *More info*, then *Run anyway*. Accept the defaults. | No UAC prompt appears at any point. Wizard finishes. "Micold AI IDE" is in Start and in Settings → Apps → Installed apps with the right version. (SC-001, SC-002) |
| M2 | Launch from Start. Open a project and start a Regular session, then an AI session with `claude` on PATH. Run `git status` in the terminal. Switch worktrees. | No console window appears at launch or on any action. Sessions stream. Help → About shows the installer's version. (US1, SC-003, SC-004) |
| M3 | Uninstall, then reinstall into `C:\Users\<you>\Apps\Mïcold Test\` (spaces and non-ASCII). Repeat M2. | Same result as M2. |
| M4 | With the app open and a session running, run the setup of a *newer* build (bump the version locally). Choose Retry after closing the app, or continue when prompted. | The prompt names the running app and says sessions will stop. After upgrade there is one Installed-apps entry, showing the new version. Settings and projects kept. Relaunch shows no "Restart service" banner, because the old daemon was stopped. (US2, SC-006) |
| M5 | Uninstall from Installed apps with the app open. | The app and daemon are stopped. `%LOCALAPPDATA%\Programs\Micold AI IDE` is gone. `%APPDATA%\micold-ai-ide` is still there. Reinstall: projects are remembered. (FR-007) |
| M6 | On ARM64, run the x64 setup, then the arm64 setup. | The x64 setup refuses with an architecture message. The arm64 setup installs, and M2 passes. (FR-015) |
| M7 | Create a second standard account. With account A's app running, sign into B via *Switch user*. In PowerShell as B, run `[System.IO.Pipes.NamedPipeClientStream]::new('.', 'Micold.Daemon.<A-SID>').Connect(2000)`. | B's connect throws `UnauthorizedAccessException`. B can install and run its own copy independently. (FR-021, FR-022, SC-009) |
| M8 | Close the window with a session running. Wait 10 minutes. Relaunch. | The session is still running with its output intact. (SC-008) |
| M9 | Settings → Restart service. | The daemon restarts, and the client reconnects without a console flash. (FR-023) |
| M10 | Follow `docs/user-guide/install-windows.md` word for word on a fresh VM. | Every step matches what the screen shows, including SmartScreen wording. (US4) |

Record results in the PR description, one line per M-row.

## Release dry-run (optional, maintainers)

Push a tag to a fork with release-please disabled. Or run `release.yml` via `workflow_dispatch`, if
it is enabled on the fork, against a draft release.

**Pass when**:

- the draft shows both setup executables alongside the `.deb`s (and `.dmg`, once 028 has merged);
- forcing the arm64 leg to fail (for example `exit 1` in the smoke step) leaves the release a
  draft (SC-005).
