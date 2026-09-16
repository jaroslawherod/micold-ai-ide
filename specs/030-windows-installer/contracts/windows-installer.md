# Contract: the Windows installer

**Feature**: [../spec.md](../spec.md) | Normative for:

- `packaging/windows/micold-ai-ide.iss`
- `scripts/windows-installer.sh`
- the `windows-installer` mise task

Covers FR-001 to FR-016.

## Build interface

```text
mise run windows-installer                 # host arch, release profile
scripts/windows-installer.sh [--arch x64|arm64] [--out-dir DIR]
```

| Input | Source |
|---|---|
| Binaries | `cargo build --release --locked -p micold-client --bin micold-ai-ide -p micold-daemon --bin micold-daemon --target <triple>` via `scripts/build-lock.sh` |
| Version | Read from `[workspace.package] version` in `Cargo.toml`; passed as `iscc /DAppVersion=<v> /DArch=<arch> /DBinDir=<dir>` |
| Compiler | `iscc.exe` located via `ISCC` env, then `%ProgramFiles(x86)%\Inno Setup 6\ISCC.exe`, then PATH. Missing: the task fails with a message naming the Inno Setup download. It never installs it silently. |

**Output**: `<out-dir>/micold-ai-ide-<version>-<arch>-setup.exe`. The default out-dir is
`<target-dir>/windows-installer/`.

On a non-Windows host the task exits non-zero with "the Windows installer is built on Windows". It
does not try Wine or cross-compilation (R9).

## Script directives (normative values)

| Directive | Value | Requirement |
|---|---|---|
| `AppId` | `{{1B19A6AC-4C91-4033-88EA-F7F283127C8A}` | FR-006, FR-008: pinned forever; a test asserts the literal |
| `AppName` | `Micold AI IDE` | FR-003 |
| `AppVersion` / `VersionInfoVersion` | `{#AppVersion}` | FR-013 |
| `AppPublisher` | `Micold AI IDE contributors` | Installed apps listing |
| `DefaultDirName` | `{autopf}\Micold AI IDE` | FR-004 (`%LOCALAPPDATA%\Programs` under lowest privileges) |
| `DefaultGroupName` / `DisableProgramGroupPage` | `Micold AI IDE` / `yes` | FR-003 |
| `PrivilegesRequired` | `lowest` | FR-004 |
| `PrivilegesRequiredOverridesAllowed` | *(absent)* | FR-004: the user is never offered an all-users (elevated) install |
| `ArchitecturesAllowed` | x64: `x64compatible and not arm64`; arm64: `arm64` | FR-015 |
| `ArchitecturesInstallIn64BitMode` | same as `ArchitecturesAllowed` | FR-015 |
| `AppMutex` | `Local\MicoldAIIDE` | FR-009 |
| `CloseApplications` | `force` | FR-009 |
| `RestartApplications` | `no` | FR-011: never relaunch or autostart |
| `SetupIconFile` / `UninstallDisplayIcon` | `assets\icon\icon.ico` / `{app}\micold-ai-ide.exe` | FR-003 |
| `OutputBaseFilename` | `micold-ai-ide-{#AppVersion}-{#Arch}-setup` | FR-001 |
| `LicenseFile` | `LICENSE` | |
| `WizardStyle` | `modern` | |
| `SignTool` | *(absent)* | FR-010 |

`[Files]`, with exactly these two entries (FR-002, FR-012):

```text
Source: "{#BinDir}\micold-ai-ide.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#BinDir}\micold-daemon.exe"; DestDir: "{app}"; Flags: ignoreversion
```

`[Icons]`:

- `{autoprograms}\Micold AI IDE` pointing at `{app}\micold-ai-ide.exe`.
- A desktop icon, only under the optional unchecked task `desktopicon`.

`[Run]`: one `postinstall nowait skipifsilent unchecked` entry, "Launch Micold AI IDE". The
checkbox is unchecked by default, so there is no implicit launch.

`[Registry]`: *(none)*. No Run key, no PATH edit, no file associations (FR-011, spec Assumptions).

`[UninstallDelete]`: `Type: filesandordirs; Name: "{localappdata}\micold-ai-ide\run"`, and nothing
else (FR-007).

`[Code]`, for R12:

- `PrepareToInstall` and `InitializeUninstall` call `StopDaemon()`. `InitializeUninstall` skips it while
  the app mutex exists, because the uninstaller checks `AppMutex` only afterwards and may refuse.
  `CurUninstallStepChanged(usUninstall)` calls it again, past that check and before files are removed
  (revised after #358: a refused uninstall had already stopped the daemon).
- `StopDaemon()` reads the pid record and checks that the process image is under `{app}`, or equals
  `micold-daemon.exe` for a legacy location.
- It terminates the process and waits up to 5 s.
- It returns an error string if the daemon is still alive. Setup then shows that string and offers
  Retry.
- Before stopping, the wizard's ready page states: "Running sessions will be stopped."

## Behaviour

| # | Scenario | Expected | Verified by |
|---|---|---|---|
| I1 | Fresh install as a standard user | No UAC prompt. Files at `%LOCALAPPDATA%\Programs\Micold AI IDE\`. Start menu entry. HKCU uninstall key with `DisplayVersion=<version>`. | CI smoke (x64 and arm64), quickstart M1 |
| I2 | Launch from Start menu | Window opens with no console. Daemon pipe appears within 20 s. About shows `<version>`. | CI smoke (pipe, no conhost), quickstart M2 |
| I3 | Install vN+1 over vN while the app and a session run | Mutex prompt, then confirmation naming session stop. Old daemon stopped. One uninstall entry, now vN+1. Settings kept. | quickstart M4 |
| I4 | Re-run the same vN installer | Completes as a repair. Still one entry. | CI smoke (installs twice), quickstart M4 |
| I5 | Uninstall from Installed apps | Daemon stopped. Install dir, shortcut, uninstall key and `run\` removed. `%APPDATA%\micold-ai-ide` and `%LOCALAPPDATA%\micold-ai-ide\data` untouched. | CI smoke, quickstart M5 |
| I6 | x64 package on ARM64 Windows (and the reverse) | Setup refuses with Inno's architecture message. Nothing is written. | quickstart M6 (manual) |
| I7 | Silent install and uninstall (`/VERYSILENT /SUPPRESSMSGBOXES /NORESTART`) | Exit code 0. Same end state as I1 and I5. | CI smoke |
| I8 | Install path contains spaces or non-ASCII (custom dir) | App and daemon run. The daemon resolves as the sibling. | quickstart M3 |

## Guard tests (run on every OS, text scans)

| Test | Asserts |
|---|---|
| `micold-client/tests/packaging_excludes_showcase.rs::windows_violations` | `[Files]` `Source:` basenames are exactly `{micold-ai-ide.exe, micold-daemon.exe}`, with no `*` or `?` in any source, after stripping `;` comments (FR-012) |
| `micold-core/tests/windows_installer_is_per_user.rs` | `PrivilegesRequired=lowest` is present. `PrivilegesRequiredOverridesAllowed` is absent. `AppId` equals the pinned GUID. `RestartApplications=no`. No `[Registry]` section. |
| `micold-core/tests/windows_installer_version_is_injected.rs` | `AppVersion` references `{#AppVersion}`, not a literal, and `scripts/windows-installer.sh` reads the workspace version |
