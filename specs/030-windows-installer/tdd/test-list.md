---
feature: 030-windows-installer
loop: outside-in
profile: .specify/memory/tdd-profile.md
spec_criteria: 15
planned_at: 61cc0318
updated_at: 61cc0318
suite_baseline: green
---

# Test List: Windows Installation Package

Trace ids are:

- `US<n>-AS<m>`: acceptance scenario m of user story n in `spec.md`.
- `FR-0xx` and `SC-0xx`: functional requirements and success criteria in `spec.md`.
- `E<n>.<m>` and `I<n>`: contract rows in `contracts/windows-endpoint.md` and `contracts/windows-installer.md`.

**Where a red can be observed.** This Linux host compiles `cfg(windows)` code (`cargo check --target x86_64-pc-windows-msvc`), but it cannot run it. Behaviours marked **win** in the `where` column go red and green only on the `windows-latest` or `windows-11-arm` CI legs of the pushed feature branch. Their cycle-log evidence is the CI run URL and the decisive log line. Behaviours marked **any** run locally.

## Outer loop: acceptance behaviors

The entry points are:

- the installer and the installed app, driven by `scripts/windows-install-smoke.sh` on the Windows CI legs;
- the release workflow and the docs, whose real artefacts are text, checked by guard tests over the committed files;
- the local packaging script.

| id  | behavior | traces | kind | where | state | test |
| --- | --- | --- | --- | --- | --- | --- |
| A1  | A silent per-user install of the setup exe exits 0. It leaves `micold-ai-ide.exe` and `micold-daemon.exe` in `%LOCALAPPDATA%\Programs\Micold AI IDE\` and a Start menu `Micold AI IDE.lnk`. | US1-AS1, FR-002, FR-003, FR-004, I1, I7 | example | win | PENDING | |
| A2  | The installed client, launched detached, has no `conhost.exe` child | US1-AS2, FR-005, SC-005 | example | win | PENDING | |
| A3  | Launching the installed client makes `\\.\pipe\Micold.Daemon.<SID>` appear within 20 s, and the daemon has no `conhost.exe` child | US1-AS3, FR-005, FR-020, I2 | example | win | PENDING | |
| A4  | The HKCU uninstall key's `DisplayVersion` equals the workspace version | US1-AS4, FR-006, FR-013 | example | win | PENDING | |
| A5  | The committed `packaging/windows/micold-ai-ide.iss` passes `windows_violations`: its `[Files]` ship exactly the two exes | US1-AS5, FR-012 | example | any | DONE | `crates/micold-client/tests/packaging_excludes_showcase.rs::the_windows_installer_contains_no_showcase` |
| A6  | Re-running the installer over a running install exits 0 and leaves exactly one `{1B19A6AC-…}_is1` uninstall key | US2-AS1, FR-008, I4 | example | win | PENDING | |
| A7  | A silent uninstall removes the install dir, `.lnk`, uninstall key and `%LOCALAPPDATA%\micold-ai-ide\run` | US2-AS2, FR-006, SC-003, I5 | example | win | PENDING | |
| A8  | After uninstall, the markers seeded in `%APPDATA%\micold-ai-ide\data` and `%LOCALAPPDATA%\micold-ai-ide\data` still exist | US2-AS3, FR-007, I5 | example | win | PENDING | |
| A9  | Installing while the daemon runs ends with the old daemon pid gone and the new exes in place | US2-AS4, FR-009, FR-023, I4 | example | win | PENDING | |
| A10 | `release.yml` has a `windows` job whose matrix covers `x64`/`windows-latest` and `arm64`/`windows-11-arm`, and it runs `gh release upload` | US3-AS1, FR-014, FR-015 | example | any | DONE | `crates/micold-core/tests/release_publishes_complete_sets.rs::a_windows_job_uploads_both_setup_executables` |
| A11 | `release.yml` `publish.needs` contains `windows` | US3-AS2, FR-014, SC-002 | example | any | DONE | `crates/micold-core/tests/release_publishes_complete_sets.rs::publish_waits_for_the_windows_job` |
| A12 | `site/stage.sh` fails and names the asset when `install-windows.md` links a setup exe missing from `MICOLD_RELEASE_ASSETS` | US3-AS3, FR-017 | example | any | DONE | `scripts/tests/site-stage.test.sh`, "a Windows guide linking a setup exe the release lacks fails the stage" + "the failure names the missing setup exe" |
| A13 | On the x64 Windows CI leg, `scripts/windows-installer.sh --arch x64` produces `micold-ai-ide-<version>-x64-setup.exe` | US3-AS4, FR-016, FR-018, SC-007 | example | win | PENDING | |
| A14 | `docs/install.md` no longer says there is no packaged build for Windows | US4-AS1, FR-017, SC-006 | example | any | DONE | `crates/micold-core/tests/install_guide_windows.rs::install_page_no_longer_says_windows_has_no_package` |
| A15 | `docs/user-guide/install-windows.md` states that sessions do not survive logging out unless the service runs in a container | US4-AS2, FR-017 | example | any | DONE | `crates/micold-core/tests/install_guide_windows.rs::windows_guide_limits_say_sessions_end_at_logout` |

## Inner loop: unit behaviors

### `crates/micold-core/tests/daemon_tests_gate_with_reason.rs` (guard over `crates/micold-daemon/tests/`)

| id  | behavior | traces | kind | where | state | test |
| --- | --- | --- | --- | --- | --- | --- |
| U21 | A daemon test file whose `#![cfg(unix)]` is not directly preceded by a `// unix-only:` line is reported | FR-025 | example | any | DONE | `crates/micold-core/tests/daemon_tests_gate_with_reason.rs::unix_gated_daemon_test_files_state_a_reason` |

### `crates/micold-core/src/endpoint.rs`

| id  | behavior | traces | kind | where | state | test |
| --- | --- | --- | --- | --- | --- | --- |
| U1  | The Windows `socket_path` is `\\.\pipe\Micold.Daemon.<SID>`, where `<SID>` matches `^S-1-[0-9-]+$` | FR-020, FR-022, E1 | example | win | PENDING | |
| U2  | The Windows `lock_path` ends in `micold-ai-ide\run\micold-daemon.pid`, and its parent directory exists after resolving | FR-023, E1 | example | win | PENDING | |
| U3  | Resolving the endpoint twice yields equal endpoints (win: on Unix the test would race parallel tests that set `XDG_RUNTIME_DIR`) | FR-020, FR-022, E1 | example | win | PENDING | |

### `crates/micold-daemon/src/singleton.rs`

| id  | behavior | traces | kind | where | state | test |
| --- | --- | --- | --- | --- | --- | --- |
| U4  | The bound pipe's DACL is protected (`SE_DACL_PROTECTED`) | FR-021, SC-009, E2.1 | example | win | PENDING | |
| U5  | The bound pipe's DACL has exactly one ACE, allowing the current token's user SID | FR-021, SC-009, E2.1 | example | win | PENDING | |
| U6  | Of two simultaneous starters on one endpoint, exactly one is `Bound` and the other `AlreadyRunning` | FR-022, E3.1 | example | any | DONE | `crates/micold-daemon/tests/daemon_singleton.rs::two_simultaneous_starters_converge_on_one_daemon` (Unix; Windows run pending T028) |
| U7  | Acquiring after the previous listener is dropped returns `Bound` | FR-020, FR-022, E3.2 | example | any | DONE | `crates/micold-daemon/tests/daemon_singleton.rs::acquire_after_drop_rebinds` (Unix; Windows run pending T028) |

### `crates/micold-daemon/src/server.rs` (pid record)

| id  | behavior | traces | kind | where | state | test |
| --- | --- | --- | --- | --- | --- | --- |
| U8  | While the real daemon runs, `lock_path` holds its pid followed by a newline | FR-023, E4.1 | example | any | DONE | `crates/micold-daemon/tests/daemon_stop.rs::pid_record_lifecycle` |
| U9  | After the daemon exits cleanly, `lock_path` no longer exists | FR-023, E4.1 | example | any | BLOCKED | no clean-exit path exists to observe, and unlinking the Unix `flock` file is unsafe (cycle-log cycle 6) |

### `crates/micold-core/src/spawn.rs` (stop)

| id  | behavior | traces | kind | where | state | test |
| --- | --- | --- | --- | --- | --- | --- |
| U10 | `stop_running_daemon` on a live daemon returns `Ok(true)`, and the endpoint refuses connections within 5 s | FR-023, E4.2 | example | any | DONE | `crates/micold-daemon/tests/daemon_stop.rs::stop_running_daemon_ends_endpoint` (Unix; Windows run pending) |
| U11 | `terminate_daemon` on a pid whose image is not `micold-daemon.exe` returns `InvalidData` and leaves the process running | FR-023, E4.3 | example | win | PENDING | |
| U12 | `stop_running_daemon` with a pid record for a non-live endpoint returns `Ok(false)` | FR-023, E4.4 | example | any | DONE | `crates/micold-core/src/spawn.rs::tests::stale_pid_record_is_ignored` |

### `crates/micold-daemon/src/platform/windows.rs` and `crates/micold-daemon/src/supervisor.rs`

| id  | behavior | traces | kind | where | state | test |
| --- | --- | --- | --- | --- | --- | --- |
| U13 | Killing a Regular session ends a grandchild its shell started, within 5 s | FR-020, E5.1 | example | win | PENDING | |

### `crates/micold-core/src/process.rs`

| id  | behavior | traces | kind | where | state | test |
| --- | --- | --- | --- | --- | --- | --- |
| U14 | On Windows, a command passed through `no_window` still spawns and exits 0, and the pinned `CREATE_NO_WINDOW` equals `0x0800_0000` | FR-024, E6.3 | example | win | BLOCKED | `crates/micold-core/src/process.rs::tests::no_window_sets_flag`, compile-checked; implemented before any Windows run (cycle 8 notes), awaiting the CI leg |
| U15 | On Unix, a command passed through `no_window` spawns and exits 0 | FR-024 (parity, Principle VI) | example | any | DONE | `crates/micold-core/src/process.rs::tests::no_window_is_noop_elsewhere` |
| U16 | While the `announce_running` marker is held, `OpenMutexW("Local\\MicoldAIIDE")` succeeds | FR-009, E7.1 | example | win | PENDING | |
| U17 | After the marker is dropped, `OpenMutexW("Local\\MicoldAIIDE")` fails | FR-009, E7.1 | example | win | PENDING | |

### `crates/micold-core/tests/background_spawns_hide_console.rs` (guard over `crates/*/src`)

| id  | behavior | traces | kind | where | state | test |
| --- | --- | --- | --- | --- | --- | --- |
| U18 | Every `Command::new(` in production sources is reported unless it passes through `no_window(`, is allowlisted with a reason, or is under `#[cfg(not(windows))]`/`#[cfg(unix)]` | FR-024, SC-005, E6.2 | example | any | DONE | `crates/micold-core/tests/background_spawns_hide_console.rs::every_background_spawn_hides_its_console` |

### `crates/micold-daemon/src/main.rs` and `crates/micold-client/src/main.rs`

| id  | behavior | traces | kind | where | state | test |
| --- | --- | --- | --- | --- | --- | --- |
| U19 | A daemon that fails at startup exits non-zero, and its log file contains a `fatal:` line | FR-005, E6.4 | example | any | DONE | `crates/micold-daemon/tests/fatal_startup_is_logged.rs::fatal_startup_error_reaches_the_log_file` |
| U20 | Release builds of `micold-ai-ide.exe` and `micold-daemon.exe` both have PE subsystem 2 (GUI) | FR-005, SC-005, E6.1 | example | win | PENDING | |

### Existing daemon tests un-gated on Windows (`crates/micold-daemon/tests/`)

These tests already exist and pass on Unix. On Windows they are compiled out today. The red is the same test failing when it first runs on the Windows leg.

| id  | behavior | traces | kind | where | state | test |
| --- | --- | --- | --- | --- | --- | --- |
| U22 | A client cold-starts a real daemon that outlives the client (`autospawn.rs`) | FR-020, FR-025, E3.3 | example | win | PENDING | |
| U23 | A session survives the client disconnecting and is reattached (`session_survival.rs`) | FR-020, FR-025, SC-008 | example | win | PENDING | |
| U24 | A started session streams output to the attached client (`session_start.rs`) | FR-020, FR-025 | example | win | PENDING | |
| U25 | A second view of a running session receives its stream (`stream_view.rs`) | FR-020, FR-025 | example | win | PENDING | |
| U26 | Sessions of different projects do not see each other's output (`session_isolation.rs`) | FR-020, FR-025 | example | win | PENDING | |

### `packaging/windows/micold-ai-ide.iss`, guarded by `crates/micold-client/tests/packaging_excludes_showcase.rs`

| id  | behavior | traces | kind | where | state | test |
| --- | --- | --- | --- | --- | --- | --- |
| U27 | `windows_violations` reports a `[Files]` `Source:` containing `*` or `?` | FR-012 | example | any | DONE | `crates/micold-client/tests/packaging_excludes_showcase.rs::a_windows_installer_with_a_wildcard_source_fails` |
| U28 | `windows_violations` reports a `[Files]` source naming `micold-showcase` | FR-012 | example | any | DONE | `crates/micold-client/tests/packaging_excludes_showcase.rs::a_windows_installer_that_ships_the_showcase_fails` |
| U29 | `windows_violations` reports a `[Files]` section that lacks `micold-daemon.exe` | FR-002 | example | any | DONE | `crates/micold-client/tests/packaging_excludes_showcase.rs::a_windows_installer_without_the_daemon_fails` |
| U60 | `windows_violations` reports a `[Files]` source whose basename is neither `micold-ai-ide.exe` nor `micold-daemon.exe` | FR-012 | example | any | DONE | `crates/micold-client/tests/packaging_excludes_showcase.rs::a_windows_installer_that_ships_an_unlisted_file_fails` (added in cycle 12 from T029 text) |

### `packaging/windows/micold-ai-ide.iss`, guarded by `crates/micold-core/tests/windows_installer_is_per_user.rs`

| id  | behavior | traces | kind | where | state | test |
| --- | --- | --- | --- | --- | --- | --- |
| U30 | The `.iss` declares `PrivilegesRequired=lowest`, and `PrivilegesRequiredOverridesAllowed` is absent | FR-004 | example | any | DONE | `crates/micold-core/tests/windows_installer_is_per_user.rs::installs_without_elevation` |
| U31 | The `.iss` `AppId` is exactly `{{1B19A6AC-4C91-4033-88EA-F7F283127C8A}` | FR-006, FR-008 | example | any | DONE | `crates/micold-core/tests/windows_installer_is_per_user.rs::app_id_is_pinned` |
| U32 | The `.iss` has no `[Registry]` section | FR-011 | example | any | DONE | `crates/micold-core/tests/windows_installer_is_per_user.rs::writes_no_registry_entries` |
| U33 | The `.iss` declares `RestartApplications=no` | FR-011 | example | any | DONE | `crates/micold-core/tests/windows_installer_is_per_user.rs::never_relaunches_the_app` |
| U34 | The `.iss` has no `SignTool` directive | FR-010 | example | any | DONE | `crates/micold-core/tests/windows_installer_is_per_user.rs::is_not_signed` |
| U35 | The `.iss` `ArchitecturesAllowed` is `x64compatible and not arm64` for `Arch == "x64"` and `arm64` for `Arch == "arm64"` | FR-015 | example | any | DONE | `crates/micold-core/tests/windows_installer_is_per_user.rs::architecture_follows_the_arch_define` |
| U38 | The `.iss` `AppMutex` value equals the mutex name in `crates/micold-core/src/process.rs` | FR-009 | example | any | DONE | `crates/micold-core/tests/windows_installer_in_use.rs::app_mutex_matches_the_running_app` |
| U39 | The `.iss` declares `CloseApplications=force` | FR-009 | example | any | DONE | `crates/micold-core/tests/windows_installer_in_use.rs::closes_the_app_window_when_the_user_continues` |
| U40 | The `.iss` `[Code]` `PrepareToInstall` and `InitializeUninstall` each call `StopDaemon` | FR-009, FR-023 | example | any | DONE | `crates/micold-core/tests/windows_installer_in_use.rs::install_and_uninstall_stop_the_daemon_first` |
| U41 | The `.iss` `[UninstallDelete]` has exactly one entry, `{localappdata}\micold-ai-ide\run` | FR-007 | example | any | DONE | `crates/micold-core/tests/windows_installer_in_use.rs::uninstall_removes_only_the_runtime_dir` |
| U42 | No `[UninstallDelete]` or `[InstallDelete]` entry names `{userappdata}` or `{localappdata}\micold-ai-ide\data` | FR-007 | example | any | DONE | `crates/micold-core/tests/windows_installer_in_use.rs::never_deletes_user_data` |
| U61 | The `.iss` `[Code]` `UpdateReadyMemo` adds `Running sessions will be stopped.` to the ready page | FR-009 | example | any | DONE | `crates/micold-core/tests/windows_installer_in_use.rs::ready_page_says_sessions_will_stop` |

### `packaging/windows/micold-ai-ide.iss`, guarded by `crates/micold-core/tests/windows_installer_version_is_injected.rs`

| id  | behavior | traces | kind | where | state | test |
| --- | --- | --- | --- | --- | --- | --- |
| U36 | `AppVersion` and `VersionInfoVersion` are `{#AppVersion}`, and no literal semver appears in the `.iss` | FR-013 | example | any | DONE | `crates/micold-core/tests/windows_installer_version_is_injected.rs::version_is_the_injected_define` |
| U37 | `OutputBaseFilename` is `micold-ai-ide-{#AppVersion}-{#Arch}-setup` | FR-001 | example | any | DONE | `crates/micold-core/tests/windows_installer_version_is_injected.rs::setup_file_name_carries_version_and_arch` |

### `scripts/windows-installer.sh`, tested by `scripts/tests/windows-installer.test.sh`

| id  | behavior | traces | kind | where | state | test |
| --- | --- | --- | --- | --- | --- | --- |
| U43 | On a non-Windows `uname`, the script exits non-zero with `the Windows installer is built on Windows` | FR-016 | example | any | DONE | `scripts/tests/windows-installer.test.sh` "refuses to run off Windows" |
| U44 | `--arch` values other than `x64` or `arm64` are rejected with a non-zero exit | FR-015, FR-016 | example | any | DONE | `scripts/tests/windows-installer.test.sh` "rejects --arch '<value>'" |
| U45 | `iscc` receives `/DAppVersion=` equal to `[workspace.package] version` in `Cargo.toml` | FR-013 | example | any | DONE | `scripts/tests/windows-installer.test.sh` "passes the workspace version to iscc" |
| U46 | `iscc` receives `/DArch=<arch>` and `/DBinDir=<target>/<triple>/release` | FR-015, FR-016 | example | any | DONE | `scripts/tests/windows-installer.test.sh` "passes /DArch=<arch> to iscc; points iscc at the <triple> release binaries" |
| U47 | When no `iscc` is found, the error names the Inno Setup download URL | FR-016 | example | any | DONE | `scripts/tests/windows-installer.test.sh` "names the Inno Setup download when no iscc is found" |
| U48 | cargo is invoked as `build --release --locked -p micold-client --bin micold-ai-ide -p micold-daemon --target <triple>` | FR-002, FR-016 | example | any | DONE | `scripts/tests/windows-installer.test.sh` "builds the app and the daemon for <triple>" |

### `.github/workflows/ci.yml`

| id  | behavior | traces | kind | where | state | test |
| --- | --- | --- | --- | --- | --- | --- |
| U50 | Every CI job, including a new `windows-arm64-package`, is in `ci-complete.needs` | FR-018 | example | any | DONE | `crates/micold-core/tests/ci_gate_covers_every_job.rs::every_job_is_covered_by_the_gate` (existing, generic over job ids) |

### `.github/workflows/release.yml` and `.github/release-notice-windows.md`

| id  | behavior | traces | kind | where | state | test |
| --- | --- | --- | --- | --- | --- | --- |
| U51 | The set of release jobs that run `gh release upload` equals `publish.needs` minus `release-please` | FR-014, SC-002 | example | any | DONE | `crates/micold-core/tests/release_publishes_complete_sets.rs::publish_waits_for_every_job_that_attaches_an_artifact` (028's rule is inclusion, not equality: `image-manifest` gates `publish` and uploads nothing) |
| U52 | `.github/release-notice-windows.md` has at most 5 non-empty lines | FR-010 | example | any | DONE | `crates/micold-core/tests/release_notice_windows.rs::notice_is_at_most_five_lines` |
| U53 | The release notice mentions `x64`, `ARM64`, `More info` and `Run anyway`, and links `install-windows` | FR-010, FR-015 | example | any | DONE | `crates/micold-core/tests/release_notice_windows.rs::notice_names_arches_smartscreen_steps_and_guide` |
| U57 | The `publish` job references `release-notice-windows.md` | FR-010 | example | any | DONE | `crates/micold-core/tests/release_notice_windows.rs::publish_appends_the_windows_notice` |

### `site/stage.sh`, tested by `scripts/tests/site-stage.test.sh`

| id  | behavior | traces | kind | where | state | test |
| --- | --- | --- | --- | --- | --- | --- |
| U54 | `site/stage.sh` passes when both setup exes named in `install-windows.md` are in `MICOLD_RELEASE_ASSETS` | FR-017 | example | any | DONE | `scripts/tests/site-stage.test.sh`, "a Windows guide linking both setup exes the release carries stages" |

### Docs, guarded by `crates/micold-core/tests/install_guide_windows.rs`

| id  | behavior | traces | kind | where | state | test |
| --- | --- | --- | --- | --- | --- | --- |
| U56 | `docs/install.md` links `user-guide/install-windows.md` | FR-017 | example | any | DONE | `crates/micold-core/tests/install_guide_windows.rs::install_page_links_the_windows_guide` |
| U58 | `install-windows.md` has the headings `Download`, `Install`, `Upgrading`, `Removing` and `Limits` | FR-017, SC-006 | example | any | DONE | `crates/micold-core/tests/install_guide_windows.rs::windows_guide_has_the_lifecycle_headings` |
| U59 | `install-windows.md` mentions `SmartScreen` and `Smart App Control` | FR-010, FR-017 | example | any | DONE | `crates/micold-core/tests/install_guide_windows.rs::windows_guide_names_smartscreen_and_smart_app_control` |

## Invariants and edge cases still to place

- **FR-003 icon.** The installed exe shows the project icon. Embedding it is build glue (T034). The only observation today is manual quickstart M1. A smoke check for an `RT_GROUP_ICON` resource would make this automatic; it is not in tasks.md yet.
- **I8.** An install path with spaces or non-ASCII characters still resolves the sibling daemon. Covered by manual quickstart M3 only.

## Out of scope

- macOS packaging: FR-019, feature 028 (PR #284).
- Code signing and SmartScreen reputation: FR-010. The installer ships unsigned by decision, and the SmartScreen dialog is checked manually (M1).
- Upgrading from an *older* version (N → N+1): CI builds one version, so A6 covers same-version repair over a running install. N → N+1 and the in-use prompt wording are manual M4.
- The x64 installer refusing on ARM64 (I6): Inno's own architecture check, pinned by U35's directive guard, and observed manually (M6).
- A second Windows account connecting (SC-009): CI has one account. U4 and U5 pin the DACL; the cross-account refusal is manual M7.
- The 10-minute survival (SC-008) and "Restart service" through the UI (FR-023 UI path): manual M8 and M9. The mechanisms are U23 and U10.
- Time-to-install (SC-001) and following the guide unaided (SC-006): manual M1 and M10.
- A real release publish: fork dry-run in quickstart. A10, A11 and U51 pin the workflow structure.
- Daemon test files T027 leaves wholly Unix-gated: their reasons are recorded by U21's guard and listed in `docs/development/windows-packaging.md` (T060).

## Verification commands

Copied from `.specify/memory/tdd-profile.md` at 61cc0318:

- Single test: `scripts/build-lock.sh cargo test -p {crate} {target} {name} -- --exact`
  - `{target}` is `--lib` or `--test <file-stem>`.
  - The output **must** contain `running 1 test`, because a zero match still exits 0.
- File: `scripts/build-lock.sh cargo test -p {crate} --test {file}`
- Full suite: `mise run test` (2793 passed, 2 ignored at baseline)
- Fast subset (core only): `mise run test-core`
- Windows compile check: `cargo check --target x86_64-pc-windows-msvc -p micold-core -p micold-daemon -p micold-client`
- macOS compile check: `cargo check --target aarch64-apple-darwin -p micold-core -p micold-daemon`
- Shell tests: `scripts/tests/<name>.test.sh`
- Windows-only (**win**) tests: the `windows-latest` / `windows-11-arm` CI legs of the pushed branch
- Coverage: none (`cargo-llvm-cov` not installed)
- Mutation: none (`cargo-mutants` not installed); use deliberate mutants
