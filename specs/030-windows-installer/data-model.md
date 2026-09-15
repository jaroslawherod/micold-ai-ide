# Data Model: Windows Installation Package

**Feature**: [spec.md](spec.md) | **Research**: [research.md](research.md)

This feature adds no persisted application state. Its "entities" fall into two groups:

- the runtime records the Windows daemon needs (Phase A), and
- the artefacts and on-disk locations the installer owns (Phase B).

Paths below use Windows environment variables. `<SID>` is the current user's string SID, for
example `S-1-5-21-…-1001`.

## Phase A — runtime records

### Endpoint (existing type, Windows arm filled in)

`micold_core::endpoint::Endpoint { socket_path: PathBuf, lock_path: PathBuf }`

| Field | Windows value | Rule |
|---|---|---|
| `socket_path` | `\\.\pipe\Micold.Daemon.<SID>` | Derived only from the process token's user SID (R1). Never from env or user name. |
| `lock_path` | `%LOCALAPPDATA%\micold-ai-ide\run\micold-daemon.pid` | Parent directory created by `resolve()` (R4). |

**Validation**:

- `resolve()` fails with a descriptive `io::Error` if the SID cannot be read. The client surfaces
  this as the existing "cannot reach the service" state, and it never panics.
- No environment variable influences the Windows endpoint. The Unix arms read `XDG_RUNTIME_DIR` and
  `HOME`; the pipe namespace needs neither.

### Pipe security descriptor (new, transient)

The descriptor is built once per bind. It is never stored.

| Attribute | Value |
|---|---|
| SDDL | `D:P(A;;GA;;;<SID>)` |
| ACE count | exactly 1 |
| Inheritance | protected (`P`), with no inherited ACEs |
| Remote clients | rejected |

**Invariant**: the SID in the ACE equals the SID in `socket_path`. The test in
[contracts/windows-endpoint.md](contracts/windows-endpoint.md) checks both.

### Pid record (existing concept, Windows location new)

| Attribute | Value |
|---|---|
| Content | Decimal pid followed by a newline (same as Unix) |
| Writer | The daemon, after `Acquisition::Bound` |
| Removed | On clean daemon exit, and by the uninstaller (`[UninstallDelete]`) |
| Readers | `spawn::running_daemon_pid` (Restart service), and the installer's `[Code]` section |

**State transitions**:

```text
absent ──daemon binds──> present(pid, live)
present(pid, live) ──daemon exits cleanly──> absent
present(pid, live) ──daemon crashes / killed──> present(pid, stale)
present(pid, stale) ──next daemon binds──> present(newpid, live)   (overwrite)
```

**Validation**: a pid is acted on (terminated) only when both hold:

1. the pipe is live, and
2. `QueryFullProcessImageNameW(pid)` ends in `\micold-daemon.exe`.

A stale record is otherwise ignored (R4, R5).

### Running-app marker (new)

| Attribute | Value |
|---|---|
| Kind | Named mutex `Local\MicoldAIIDE` |
| Created by | The client and the daemon at startup (Windows only); held for the process lifetime |
| Consumed by | The installer's `AppMutex` check (R12) |
| Scope | The logon session. Another account's instance does not register. |

This marker is advisory only. The singleton guarantee remains the pipe's first instance (R3).

### Session job (new, in memory)

| Attribute | Value |
|---|---|
| Kind | Job Object with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` |
| Cardinality | One per PTY session |
| Members | The session's shell or agent process and all descendants started after assignment |
| Lifetime | Closed on session kill, which terminates the tree. Closed implicitly when the daemon exits. |

## Phase B — installer artefacts and locations

### Windows installer (release asset)

| Attribute | Value |
|---|---|
| File name | `micold-ai-ide-<version>-<arch>-setup.exe`, where `<arch>` ∈ {`x64`, `arm64`} |
| `<version>` | `[workspace.package] version` from `Cargo.toml`, passed as `/DAppVersion` |
| Signed | No (FR-010) |
| `AppId` | Fixed GUID (see [contracts/windows-installer.md](contracts/windows-installer.md)). Never changes. |
| Allowed architectures | x64 package: x64 only, not ARM64. arm64 package: ARM64 only. |
| Privileges | Lowest (per-user). Must never request elevation. |

**Validation**:

- The embedded `AppVersion` equals the file name's `<version>`, which equals the client's About
  version (FR-013, SC-004).
- `[Files]` names exactly `micold-ai-ide.exe` and `micold-daemon.exe`, with no wildcards (FR-012).

### Installation (on-disk, owned by the installer)

| Item | Location |
|---|---|
| Install dir | `%LOCALAPPDATA%\Programs\Micold AI IDE\` |
| Client | `…\micold-ai-ide.exe` (icon embedded) |
| Daemon | `…\micold-daemon.exe` (sibling of the client) |
| Uninstaller | `…\unins000.exe`, `…\unins000.dat` |
| Start menu | `%APPDATA%\Microsoft\Windows\Start Menu\Programs\Micold AI IDE.lnk` |
| Registration | `HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\{<AppId>}_is1` (`DisplayName`, `DisplayVersion`, `Publisher`, `DisplayIcon`, `UninstallString`, `QuietUninstallString`) |
| Removed on uninstall | All of the above, plus `%LOCALAPPDATA%\micold-ai-ide\run\` |

**State transitions**:

```text
not installed ──install vN──> installed(vN)
installed(vN) ──install vM (M≠N)──> [stop app+daemon, confirmed] ──> installed(vM)   (one registry entry)
installed(vN) ──install vN again──> installed(vN)  (repair: files rewritten, no duplicate)
installed(vN) ──uninstall──> [stop app+daemon, confirmed] ──> not installed  (user data untouched)
```

### User data (never owned by the installer)

| Item | Location |
|---|---|
| Settings, projects, hooks, client state, client log | `%APPDATA%\micold-ai-ide\data\` |
| Daemon log | `%LOCALAPPDATA%\micold-ai-ide\data\micold-daemon.log` |
| Worktrees | Inside each project's repository (feature 029), never under the install dir |

**Invariant**: no installer or uninstaller action creates, modifies or deletes these paths (FR-007).
The CI smoke test asserts it (R14).

### Release artifact set (extends feature 028)

| Asset | Job |
|---|---|
| `micold-client_<version>-1_amd64.deb` | `deb (amd64)` |
| `micold-client_<version>-1_arm64.deb` | `deb (arm64)` |
| `MicoldAIIDE-<version>-universal.dmg` | `macos` (feature 028) |
| `micold-ai-ide-<version>-x64-setup.exe` | `windows (x64)` |
| `micold-ai-ide-<version>-arm64-setup.exe` | `windows (arm64)` |

**Invariant**: the release is published only when every job in the table succeeded (FR-014). See
[contracts/release-artifacts.md](contracts/release-artifacts.md).
