# Packaging for Windows

This page explains how `micold-ai-ide-<version>-<arch>-setup.exe` is produced, how to reproduce it
on your own Windows machine, and how the daemon behaves once it is installed.

The normative descriptions live with the feature that introduced this:
`specs/030-windows-installer/contracts/windows-installer.md`
for the installer, `windows-endpoint.md` beside it for the daemon's pipe, and `release-artifacts.md`
for the job graph. This page is the working explanation of all three.

## The two scripts

One script builds the installer and one installs it and checks the result. Nothing else composes
an installer. The CI legs and the release workflow call the same two scripts a contributor does.

| Script | Does | Runs on |
|---|---|---|
| `scripts/windows-installer.sh [--arch x64\|arm64] [--out-dir DIR]` | Builds both release exes for the arch and compiles `packaging/windows/micold-ai-ide.iss` into `micold-ai-ide-<version>-<arch>-setup.exe` | Windows, in Git Bash |
| `scripts/windows-install-smoke.sh <setup.exe>` | Installs silently per user, launches the app, waits for the daemon's pipe, checks no console window, checks the uninstall entry | Windows, in Git Bash |

Both refuse to run anywhere else. Their host and argument checks are driven on Linux by
`scripts/tests/windows-installer.test.sh` and `scripts/tests/windows-install-smoke.test.sh`.

### Building locally

You need [Inno Setup 6](https://jrsoftware.org/isdl.php) and Git Bash (it comes with Git for
Windows).

```sh
mise run windows-installer          # the host arch, into the shared target directory
scripts/windows-installer.sh --arch arm64 --out-dir dist
```

The script looks for `ISCC.exe` in `$ISCC` first, then in the installer's default location, then on
`PATH`. The version comes from `[workspace.package]` in `Cargo.toml` and is passed to the compiler
as `/DAppVersion`, so the `.iss` never contains one. `crates/micold-core/tests/windows_installer_version_is_injected.rs`
keeps it that way.

Each package installs only on its own architecture. An x64 setup refuses an ARM64 machine and an
ARM64 setup refuses an x64 one, both with Inno Setup's own message.

### Running the smoke test by hand

```sh
scripts/windows-install-smoke.sh dist/micold-ai-ide-*-x64-setup.exe
```

It installs into your own `%LOCALAPPDATA%\Programs\Micold AI IDE`, starts the app, and stops what it
started when it finishes. It states each assertion as it goes. On a machine where you already use
the app, run it from a second local account instead, because the daemon's pipe is per user.

## What the `.iss` does

`packaging/windows/micold-ai-ide.iss` is the whole installer. Its comments cite the requirement
behind each directive. The ones that matter when changing it:

- **`AppId` must never change.** Windows keys the uninstall entry on it. A new value would install a
  second copy beside the old one instead of upgrading it, and the old copy's uninstaller would stay
  in Settings.
- **Per user, no UAC.** `PrivilegesRequired=lowest` installs under `%LOCALAPPDATA%\Programs`.
  `PrivilegesRequiredOverridesAllowed` is deliberately absent, so setup never offers an all-users
  install. `crates/micold-core/tests/windows_installer_is_per_user.rs` guards both.
- **Exactly two files.** `micold-ai-ide.exe` and `micold-daemon.exe`, named one by one. Never use a
  wildcard, because the showcase binary is built into the same directory.
  `crates/micold-client/tests/packaging_excludes_showcase.rs` guards this.
- **A running app blocks setup.** The client and daemon each hold the mutex `Local\MicoldAIIDE`
  (`micold_core::process::APP_MUTEX_NAME`) for their lifetime, and `AppMutex` makes setup ask the
  user to close the app. `CloseApplications=force` lets Restart Manager close a window that is still
  open.
- **The daemon is stopped explicitly.** Restart Manager does not close the windowless daemon, and a
  live daemon keeps its exe locked. So `StopDaemon` in `[Code]` runs before install and uninstall. It
  reads the pid record and stops that process only if its image is `micold-daemon.exe`. The ready
  page tells the user this ends their sessions. `crates/micold-core/tests/windows_installer_in_use.rs`
  guards this.
- **Uninstall keeps the user's data.** It removes only `%LOCALAPPDATA%\micold-ai-ide\run`, the
  daemon's runtime directory. Settings and session data stay.

## CI and release

| Where | What runs |
|---|---|
| `ci.yml`, job `test`, `windows-latest` leg | The render-free core and daemon suites, then `Package the Windows installer` and `Install and launch the Windows installer` for x64, then `Release exes are GUI-subsystem (Windows)` |
| `ci.yml`, job `windows-arm64-package` | The same packaging and smoke steps on `windows-11-arm`. The test matrix has no ARM64 Windows leg, so this is its own job |
| `release.yml`, job `windows` | Builds, smoke-tests and uploads both setup executables to the draft release |

Both CI legs upload the setup executable as a workflow artifact, kept for 7 days, for manual testing.
`ci-complete` gates on both.

In the release, `publish` lists `windows` in `needs:`. If either architecture fails, the release
stays an unpublished draft rather than going out without its Windows downloads.
`crates/micold-core/tests/release_publishes_complete_sets.rs` keeps that list complete. The
installers are unsigned, so `publish` appends `.github/release-notice-windows.md` to the release
body, which explains how to get past SmartScreen.

`Release exes are GUI-subsystem (Windows)` reads the PE header of both release exes and asserts
subsystem `2`. A console-subsystem exe opens a console window beside the app, and the daemon would
open one of its own every time it starts.

## The daemon on Windows

On Linux and macOS the daemon listens on a Unix socket. On Windows it listens on a named pipe, and
everything that protects the socket on Unix has a Windows counterpart:

| | Unix | Windows |
|---|---|---|
| Endpoint | `$XDG_RUNTIME_DIR/micold/daemon.sock` (Linux), `$HOME/.micold/run/d.sock` (macOS) | `\\.\pipe\Micold.Daemon.<user-SID>` |
| Who may connect | directory mode `0700` | a protected DACL, `D:P(A;;GA;;;<sid>)`: full access for the owner, nothing inherited |
| Single instance | `flock` on `daemon.lock` beside the socket | the pipe itself: creating it fails while another daemon holds it |
| Pid record | `daemon.lock` | `%LOCALAPPDATA%\micold-ai-ide\run\micold-daemon.pid` |
| Stopping it | `SIGTERM` to the recorded pid | `TerminateProcess`, only if the pid's image is `micold-daemon.exe` |
| Session teardown | the process group | a job object with `KILL_ON_JOB_CLOSE`, so a shell's grandchildren die with it |

Both the pipe name and the DACL use the SID from the process token, never `%USERNAME%` or any
other environment variable, so nothing in the environment can choose whose pipe a process opens.
The image check matters because a stale pid record can name a pid that Windows has since reused.

The code is in `crates/micold-core/src/endpoint.rs` (name and pid record),
`crates/micold-daemon/src/singleton.rs` (bind and DACL), `crates/micold-core/src/spawn.rs`
(stop), `crates/micold-core/src/win_job.rs` and `crates/micold-daemon/src/platform/windows.rs`
(job objects), and `crates/micold-core/src/process.rs` (the app mutex).

## Tests not run on Windows

Every daemon test file runs on the Windows leg except the four below. Each still starts with
`#![cfg(unix)]`, and the line above it gives the reason (`crates/micold-core/tests/daemon_tests_gate_with_reason.rs`
fails a gate that has none).

All four have one cause in common. On Unix a test gives each daemon a private endpoint and data
directory through `XDG_RUNTIME_DIR` and `XDG_DATA_HOME`. On Windows the pipe name comes from the
user's SID and the data directory is a known folder, and neither reads the environment. A test there
would bind, or write into, the signed-in user's own daemon and data.

| File | Why it stays Unix |
|---|---|
| `crates/micold-daemon/tests/client_restart_input.rs` | Seeds `projects.json` through `XDG_DATA_HOME`, so on Windows it would overwrite the user's own project catalog. It also stops its daemon with `kill` and `fuser`. |
| `crates/micold-daemon/tests/idle_stop.rs` | Isolates each daemon's endpoint through `XDG_RUNTIME_DIR` and its log through `XDG_DATA_HOME`, so on Windows its concurrently running daemons would share the user's own. It also probes liveness with `libc::kill(pid, 0)`. |
| `crates/micold-daemon/tests/idle_teardown.rs` | Isolates each daemon's endpoint through `XDG_RUNTIME_DIR`, so on Windows its daemons would bind the user's own. It also probes liveness with `libc::kill`, checks the lock with `libc::flock`, and checks that the socket file is unlinked. |
| `crates/micold-daemon/tests/pi_launch_wiring.rs` | Keeps the materialised Pi component out of the real data directory through `XDG_DATA_HOME`, so on Windows it would write into the user's own. Its recording `pi` is a `#!/bin/sh` script. |

A test-only override for the Windows endpoint and data directory would let all four run there. Until
one exists, what they cover is checked on Linux and macOS only.
