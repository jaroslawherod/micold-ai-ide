# Cycle Log: Windows Installation Package

Append only. Newest last. Every entry's `red` block is the evidence that the test existed and
failed before the implementation.

## Baseline

- suite: `mise run test` -> 2793 passed, 0 failed, 2 ignored (181 s including build-lock wait)
- commit: `61cc0318`
- recorded: cycle 0, before any change

## Cycle 1: U21 a Unix-gated daemon test file with no stated reason is reported

- test: `crates/micold-core/tests/daemon_tests_gate_with_reason.rs::unix_gated_daemon_test_files_state_a_reason` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --test daemon_tests_gate_with_reason unix_gated_daemon_test_files_state_a_reason -- --exact`
  -> `these daemon test files are compiled out on Windows with no stated reason, so the Windows CI leg silently skips them: ["activity_ended.rs", "activity_pipeline.rs", "autospawn.rs", ...` (1 failed)
- green: a `// unix-only: pending Windows triage (030 T026/T027)` line above each of the 26 `#![cfg(unix)]` lines in `crates/micold-daemon/tests/` (T005). Same command -> 1 passed
- refactor: none needed
- commit: uncommitted (the session commits only when the user asks)

## Cycle 2: U7 acquiring after the previous listener is dropped returns `Bound`

- test: `crates/micold-daemon/tests/daemon_singleton.rs::acquire_after_drop_rebinds` (new). The file's whole-file `#![cfg(unix)]` was removed, a per-platform `test_endpoint` was added (a unique `\\.\pipe\Micold.Test.*` name on Windows), and only `a_stale_socket_from_a_crash_is_reclaimed` stays `#[cfg(unix)]` with a `// unix-only:` reason (T009).
- red: none on this host. The Unix implementation already satisfies it; `cargo test -p micold-daemon --test daemon_singleton acquire_after_drop_rebinds -- --exact` -> 1 passed on the first run. The intended red is the Windows leg, where `acquire` opens `temp_dir()` as a file and fails; that run is pending the push in T028.
- deliberate mutant 1: removing both socket unlinks in `singleton.rs` still passed, because `interprocess` reclaims the socket name on listener drop. That mutant does not violate the behaviour.
- deliberate mutant 2: `std::mem::forget(self._lock.try_clone())` in `BoundListener::drop` (the endpoint lock outlives the daemon) hung the test. The test was then bounded with `REBIND_BUDGET` (5 s), and the same mutant gave
  `acquire after drop must not wait on the previous daemon's endpoint: Elapsed(())` (1 failed, 5.00s). `singleton.rs` was restored from a byte copy; `git diff` on it is empty.
- green: `cargo test -p micold-daemon --test daemon_singleton` -> 4 passed
- refactor: the timeout was added to the test, as above
- commit: uncommitted

## Cycle 3: U6 two simultaneous starters converge on one `Bound`

- test: `crates/micold-daemon/tests/daemon_singleton.rs::two_simultaneous_starters_converge_on_one_daemon` (existing, now compiled on every platform by cycle 2's un-gating)
- red: none on this host; it is an existing passing Unix test. It is credited on Unix. Its Windows run is pending the push in T028.
- green: `cargo test -p micold-daemon --test daemon_singleton` -> 4 passed
- refactor: none
- commit: uncommitted

## Cycle 4: U8 while the real daemon runs, `lock_path` holds its pid followed by a newline

- test: `crates/micold-daemon/tests/daemon_stop.rs::pid_record_lifecycle` (new file, all platforms, no gate). Spawns the real `micold-daemon` with an isolated `HOME`/`XDG_*` (Unix) and kills only that child on drop. Cases share a `tokio::sync::Mutex`, because the resolver reads process env and Windows has one endpoint.
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test daemon_stop pid_record_lifecycle -- --exact` -> `running 1 test`, then
  `assertion \`left == right\` failed: lock_path must hold the running daemon's pid followed by a newline` with `left: "1283350"` and `right: "1283350\n"` (1 failed)
- green: `server.rs` writes `format!("{}\n", std::process::id())` (T018, partly). Same command -> 1 passed
- refactor: none
- notes: the case name is the contract's (E4.1), but it covers only the write half. The removal half is U9, which is BLOCKED (see cycle 6's notes). Full suite deferred to a batched run before the next report.
- commit: uncommitted

## Cycle 5: U12 `stop_running_daemon` with a pid record for a non-live endpoint returns `Ok(false)`

- test: `crates/micold-core/src/spawn.rs::tests::stale_pid_record_is_ignored` (new). The record holds `0x7FFF_FFF0`, above every pid ceiling, so a wrong answer signals nobody.
- red: `scripts/build-lock.sh cargo test -p micold-core --lib spawn::tests::stale_pid_record_is_ignored -- --exact` -> `running 1 test`, then
  `a pid record whose endpoint is not live is a leftover, not a daemon to stop; got Ok(true)` (1 failed)
- green: `running_daemon_pid` returns `None` unless a synchronous `interprocess` connect to `socket_path` succeeds (T019, partly). Same command -> 1 passed
- refactor: none
- notes: this test was written while cycle 4's first build waited on another worktree's build lock. It was not run, and no implementation was touched, until cycle 4 was green.
- commit: uncommitted

## Cycle 6: U10 `stop_running_daemon` on a live daemon returns `Ok(true)` and the endpoint refuses within 5 s

- test: `crates/micold-daemon/tests/daemon_stop.rs::stop_running_daemon_ends_endpoint` (new)
- red: none on this host. The Unix `SIGTERM` path already satisfies it; the same single-test command -> 1 passed on the first run. The intended red is the Windows leg, where `terminate_daemon` returns `Unsupported`; that run is pending the push.
- deliberate mutant: the Unix `libc::kill` replaced by `0` (the stop is reported but never sent) ->
  `the endpoint still accepts connections 5s after the stop` (1 failed, 5.09s). `spawn.rs` was restored from a byte copy; `cmp` reports it identical.
- green: same command -> 1 passed
- refactor: none
- notes, U9 BLOCKED: "removed on clean exit" has no clean exit to observe. `serve_interprocess` returns only on an accept error, and the only stops are `SIGTERM` (no handler, so the default action) and `TerminateProcess`. A removal guard would be dead code on both platforms. On Unix `lock_path` is also the `flock` file, and unlinking a held lock file lets a second daemon lock a new inode while the first still runs. The stale-record case (U12) already makes a leftover record harmless. Needs a decision: drop U9 and the "removes on clean exit" wording in E4.1, `data-model.md` and R4, or add a shutdown path to the daemon.
- commit: uncommitted

## Cycle 7: U15 on Unix, a command passed through `no_window` spawns and exits 0

- test: `crates/micold-core/src/process.rs::tests::no_window_is_noop_elsewhere` (new). `process.rs` was created with `todo!()` bodies for `no_window` and `announce_running`, and declared in `lib.rs` (T014).
- red: `scripts/build-lock.sh cargo test -p micold-core --lib process::tests::no_window_is_noop_elsewhere -- --exact` -> `running 1 test`, then
  `panicked at crates/micold-core/src/process.rs:15:5: not yet implemented` (1 failed)
- green: the `#[cfg(not(windows))]` `no_window` returns the command unchanged. The Windows arm stays `todo!()` for U14. Same command -> 1 passed
- refactor: none
- notes: the red is the stub's panic, not an assertion. A pass-through function has no wrong value to assert against short of the stub. The `tokio::process::Command` twin in T014 is not added: no source uses `tokio::process`, and the workspace `tokio` has no `process` feature, so the twin would need a feature change for no caller.
- commit: uncommitted

## Cycle 8: U18 a background spawn that can run on Windows and skips `no_window` is reported

- test: `crates/micold-core/tests/background_spawns_hide_console.rs::every_background_spawn_hides_its_console` (new). Exempts functions gated `cfg(unix)`, `cfg(not(windows))`, `cfg(target_os = "linux")` or `cfg(target_os = "macos")`, and allowlists `spawn.rs` `spawn_detached_daemon` with its reason. The allowlist entry must name a real spawn.
- red: `scripts/build-lock.sh cargo test -p micold-core --test background_spawns_hide_console every_background_spawn_hides_its_console -- --exact` -> `running 1 test`, then the offenders
  `env_include.rs:363 (in baseline_env)`, `env_include.rs:387 (in attempt_env)`, `git.rs:114 (in run_git)`, `git.rs:160 (in branch_exists)`, `git.rs:308 (in submodule_update_init_recursive)`, `sandbox/exec.rs:102 (in run)`, `sandbox/exec.rs:121 (in run_streaming)` (1 failed). These are exactly the sites T013 predicted. `env_include.rs:323` (`bash`, `cfg(not(windows))`) and `logout_survival.rs:195` (`cfg(target_os = "linux")`) were correctly exempt.
- green: the seven spawns go through `micold_core::process::no_window` (T022). Same command -> 1 passed. `cargo check --target x86_64-pc-windows-msvc -p micold-core --tests` is clean.
- refactor: none
- notes, U14 deviation: routing real spawns through a `todo!()` Windows arm would have broken every Windows `git` call. So the Windows test `process::tests::no_window_sets_flag` was written first and compile-checked against the stub, which failed with `E0425 cannot find value CREATE_NO_WINDOW`. That is a compile error, not a red. The Windows arm (`creation_flags(CREATE_NO_WINDOW)`) was then implemented. U14's red was never observed and its green is unobserved until the Windows CI leg runs, so U14 stays BLOCKED on that run. Treat it as test-after.
- commit: uncommitted

## Cycle 9: U19 a daemon that fails at startup exits non-zero and logs a `fatal:` line

- test: `crates/micold-daemon/tests/fatal_startup_is_logged.rs::fatal_startup_error_reaches_the_log_file` (new). Runs the real binary detached (stdio null, no `JOURNAL_STREAM`, so the file sink) with `MICOLD_LISTEN_ADDR=not-an-addr`.
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test fatal_startup_is_logged fatal_startup_error_reaches_the_log_file -- --exact` -> `running 1 test`, `the log at /tmp/.tmppTVWQb/data/micold-ai-ide/micold-daemon.log must record the fatal startup error` (the log held only `ERROR ... failed to bind the sandbox listener addr=not-an-addr error=invalid socket address`). The non-zero exit assertion already held.
- green: `crates/micold-daemon/src/main.rs` appends `micold-daemon: fatal: {e}` to `logging::default_log_path()` before the existing `eprintln!` and exit 1 (T023). Same command -> 1 passed.
- refactor: none
- notes: the full suite was run once before this cycle, covering cycles 4–8: `mise run test` -> 2800 passed, 0 failed, 2 ignored. The full run for this cycle is batched with the next ones. The Windows run of this test writes to the real user log (the data dir takes no env input); that run is on the CI leg.
- commit: uncommitted

## Cycle 10: U27 `windows_violations` reports a `[Files]` `Source:` containing `*` or `?`

- test: `crates/micold-client/tests/packaging_excludes_showcase.rs::a_windows_installer_with_a_wildcard_source_fails` (new, with `HEALTHY_ISS`, `WINDOWS_SHIPPED` and a `windows_violations` stub returning `Vec::new()`)
- red: `scripts/build-lock.sh cargo test -p micold-client --test packaging_excludes_showcase a_windows_installer_with_a_wildcard_source_fails -- --exact` -> `running 1 test`, `a wildcard \`Source:\` must be reported by value, got []`
- green: `iss_file_sources` collects `[Files]` `Source:` values, skipping `;` comment lines. `windows_violations` reports any value containing `*` or `?`. Same command -> 1 passed.
- refactor: none
- notes: the U19 test file was written while the full suite held the build lock, and was not run until the suite finished.
- commit: uncommitted

## Cycle 11: U28 `windows_violations` reports a `[Files]` source naming `micold-showcase`

- test: `crates/micold-client/tests/packaging_excludes_showcase.rs::a_windows_installer_that_ships_the_showcase_fails` (new)
- red: `scripts/build-lock.sh cargo test -p micold-client --test packaging_excludes_showcase a_windows_installer_that_ships_the_showcase_fails -- --exact` -> `running 1 test`, `a \`Source:\` naming the showcase must be reported, got []`
- green: sources for which `names_showcase` holds (the existing Debian-side predicate) are reported. Same command -> 1 passed.
- refactor: none
- commit: uncommitted

## Cycle 12: U29 `windows_violations` reports a `[Files]` section that lacks `micold-daemon.exe`

- test: `crates/micold-client/tests/packaging_excludes_showcase.rs::a_windows_installer_without_the_daemon_fails` (new)
- red: `scripts/build-lock.sh cargo test -p micold-client --test packaging_excludes_showcase a_windows_installer_without_the_daemon_fails -- --exact` -> `running 1 test`, `an installer that does not ship the daemon must be reported, got []`
- green: every name in `shipped` must be the basename of some source, split on `\\` or `/`. Same command -> 1 passed.
- refactor: none
- notes: T029 also says "fails on any basename other than the two". No behavior on the list covered that, so it was appended as U60 rather than implemented here. A5 (the real `.iss`) waits for T035.
- commit: uncommitted

## Cycle 13: U60 `windows_violations` reports a source whose basename is not one of the two shipped exes

- test: `crates/micold-client/tests/packaging_excludes_showcase.rs::a_windows_installer_that_ships_an_unlisted_file_fails` (new)
- red: `scripts/build-lock.sh cargo test -p micold-client --test packaging_excludes_showcase a_windows_installer_that_ships_an_unlisted_file_fails -- --exact` -> `running 1 test`, `a \`Source:\` outside the shipped set must be reported, got []`
- green: a third arm reports a source whose `basename` is not in `shipped`. The per-source checks became an `else if` chain (showcase, then wildcard, then unlisted), so each source gives one violation, and U27/U28 still find theirs. Same command -> 1 passed. The whole file -> 12 passed.
- refactor: the basename split was extracted into `fn basename`, shared with the missing-binary check.
- commit: uncommitted

## Cycle 14: A5 the committed `.iss` passes `windows_violations` and ships exactly the two exes

- test: `crates/micold-client/tests/packaging_excludes_showcase.rs::the_windows_installer_contains_no_showcase` (new; reads `packaging/windows/micold-ai-ide.iss`)
- red: `scripts/build-lock.sh cargo test -p micold-client --test packaging_excludes_showcase the_windows_installer_contains_no_showcase -- --exact`. With no file it failed on `read .../micold-ai-ide.iss: No such file or directory (os error 2)`, which is not an assertion red. With a stub `.iss` (header comments, `[Setup]`, `AppName=`) it failed on the assertion `packaging/windows/micold-ai-ide.iss does not ship \`micold-ai-ide.exe\`` (and the same for `micold-daemon.exe`).
- green: added `[Files]` with the two `{#BinDir}\` entries from the contract. Same command -> 1 passed.
- refactor: none
- notes: the `.iss` is being grown one directive per behavior (U30–U42). The directives no behavior covers (icons, `[Run]`, `DefaultDirName`, …) stay with T035 in `/speckit-implement`.
- commit: uncommitted

## Cycle 15: U30 `PrivilegesRequired=lowest` and no `PrivilegesRequiredOverridesAllowed`

- test: `crates/micold-core/tests/windows_installer_is_per_user.rs::installs_without_elevation` (new file; `directive_values` scans `Name=Value` lines, skipping `;` comments)
- red: `scripts/build-lock.sh cargo test -p micold-core --test windows_installer_is_per_user installs_without_elevation -- --exact` -> `running 1 test`, `the installer must run as the signed-in user, with no UAC prompt (FR-004)` `left: []` `right: ["lowest"]`
- green: `PrivilegesRequired=lowest` in `[Setup]`. Same command -> 1 passed.
- refactor: none
- notes: the absence assertion held from the start. Mutant: inserting `PrivilegesRequiredOverridesAllowed=dialog` gave `the user must never be offered an all-users (elevated) install (FR-004)` `left: ["dialog"]`. The file was restored from a copy, and cmp confirmed it.
- commit: uncommitted

## Cycle 16: U31 `AppId` is exactly the pinned GUID

- test: `crates/micold-core/tests/windows_installer_is_per_user.rs::app_id_is_pinned`
- red: `scripts/build-lock.sh cargo test -p micold-core --test windows_installer_is_per_user app_id_is_pinned -- --exact` -> `running 1 test`, `AppId is pinned forever ...` `left: []` `right: ["{{1B19A6AC-4C91-4033-88EA-F7F283127C8A}"]`
- green: added `AppId={{1B19A6AC-4C91-4033-88EA-F7F283127C8A}`. Same command -> 1 passed.
- refactor: none
- commit: uncommitted

## Cycle 17: U33 `RestartApplications=no`

- test: `crates/micold-core/tests/windows_installer_is_per_user.rs::never_relaunches_the_app`
- red: `scripts/build-lock.sh cargo test -p micold-core --test windows_installer_is_per_user never_relaunches_the_app -- --exact` -> `running 1 test`, `setup must not relaunch the app it closed ...` `left: []` `right: ["no"]`
- green: added `RestartApplications=no`. Same command -> 1 passed.
- refactor: none
- commit: uncommitted

## Cycle 18: U32 no `[Registry]` section

- test: `crates/micold-core/tests/windows_installer_is_per_user.rs::writes_no_registry_entries`
- red: none. The test passed on its first run, because the script has no `[Registry]` section, and an absence rule has nothing to implement. Mutant: appending a `[Registry]` HKCU Run-key entry gave `the installer must not write registry values of its own ... found ["[Registry]"]`. Restored from a copy, and cmp confirmed it.
- green: n/a (no production change)
- refactor: none
- commit: uncommitted

## Cycle 19: U34 no `SignTool` directive

- test: `crates/micold-core/tests/windows_installer_is_per_user.rs::is_not_signed`
- red: none, for the same reason as cycle 18. Mutant: inserting `SignTool=signtool` gave `left: ["signtool"]` (1 failed). Restored from a copy, and cmp confirmed it.
- green: n/a
- refactor: none
- commit: uncommitted

## Cycle 20: U35 `ArchitecturesAllowed` follows `{#Arch}`

- test: `crates/micold-core/tests/windows_installer_is_per_user.rs::architecture_follows_the_arch_define`. `preprocess(script, arch)` evaluates the script's `#if Arch == ".."`/`#elif`/`#else`/`#endif`/`#ifndef`/`#error` and panics on any other condition. Covers x64 and arm64, plus `ArchitecturesInstallIn64BitMode` (contract table).
- red: `scripts/build-lock.sh cargo test -p micold-core --test windows_installer_is_per_user architecture_follows_the_arch_define -- --exact` -> `running 1 test`, `/DArch=x64 must allow exactly \`x64compatible and not arm64\` ...` `left: []`
- green: added the `#if Arch == "x64"` / `#elif Arch == "arm64"` / `#else #error` block. The whole file -> 6 passed.
- refactor: none
- notes: the red stopped at the x64 case, so the arm64 case was never seen failing on its own. Its values were added in the same green.
- commit: uncommitted

## Cycle 21: U36 `AppVersion`/`VersionInfoVersion` are `{#AppVersion}`, and there is no literal semver

- test: `crates/micold-core/tests/windows_installer_version_is_injected.rs::version_is_the_injected_define` (new file)
- red: `scripts/build-lock.sh cargo test -p micold-core --test windows_installer_version_is_injected version_is_the_injected_define -- --exact` -> `running 1 test`, `AppVersion must be \`{#AppVersion}\`, passed by the build from Cargo.toml (FR-013)` `left: []` `right: ["{#AppVersion}"]`
- green: added `AppVersion={#AppVersion}` and `VersionInfoVersion={#AppVersion}`. Same command -> 1 passed.
- refactor: none
- notes: the no-semver half passed before green. Mutant: inserting `AppVerName=Micold AI IDE 0.4.1` gave `the .iss must not spell out a version ...` `left: ["0.4.1"]`. Restored from a copy, and cmp confirmed it -> 1 passed.
- commit: uncommitted

## Cycle 22: U37 `OutputBaseFilename=micold-ai-ide-{#AppVersion}-{#Arch}-setup`

- test: `crates/micold-core/tests/windows_installer_version_is_injected.rs::setup_file_name_carries_version_and_arch`
- red: `scripts/build-lock.sh cargo test -p micold-core --test windows_installer_version_is_injected setup_file_name_carries_version_and_arch -- --exact` -> `running 1 test`, `the setup exe must be named for its version and architecture ...` `left: []` `right: ["micold-ai-ide-{#AppVersion}-{#Arch}-setup"]`
- green: added the directive. Both installer test files -> 6 passed, 2 passed.
- refactor: none. `iss_path`/`iss`/`directive_values` now repeat across two test files; each integration test is its own crate and the repo has no shared tests module, so they stay.
- commit: uncommitted

## Cycle 23: U38 `AppMutex` equals the app's mutex name

- test: `crates/micold-core/tests/windows_installer_in_use.rs::app_mutex_matches_the_running_app` (new file). It compares against `micold_core::process::APP_MUTEX_NAME`, not a second literal, so the two cannot drift.
- red: `scripts/build-lock.sh cargo test -p micold-core --test windows_installer_in_use app_mutex_matches_the_running_app -- --exact` -> `running 1 test`, `setup detects a running app only through the mutex the app holds (FR-009)` `left: []` `right: ["Local\\MicoldAIIDE"]`
- green: `AppMutex=Local\MicoldAIIDE` in `[Setup]`. Same command -> 1 passed.
- refactor: none
- notes: the test needs a symbol to resolve, so `pub const APP_MUTEX_NAME: &str = "Local\\MicoldAIIDE"` was added to `process.rs` before the red. `announce_running` is still `todo!()` (U16/U17 are Windows-only and blocked on CI).
- commit: uncommitted

## Cycle 24: U39 `CloseApplications=force`

- test: `crates/micold-core/tests/windows_installer_in_use.rs::closes_the_app_window_when_the_user_continues`
- red: `scripts/build-lock.sh cargo test -p micold-core --test windows_installer_in_use closes_the_app_window_when_the_user_continues -- --exact` -> `running 1 test`, `setup must close a still-open app window through Restart Manager ...` `left: []` `right: ["force"]`
- green: added the directive. The file -> 2 passed.
- refactor: none
- commit: uncommitted

## Cycle 25: U40 `PrepareToInstall` and `InitializeUninstall` call `StopDaemon`

- test: `crates/micold-core/tests/windows_installer_in_use.rs::install_and_uninstall_stop_the_daemon_first`. `code_section` strips `//` comments; `routine_body` runs from the header to the first unindented `end;`.
- red: `scripts/build-lock.sh cargo test -p micold-core --test windows_installer_in_use install_and_uninstall_stop_the_daemon_first -- --exact` -> `running 1 test`, `[Code] must define \`PrepareToInstall\` (FR-009, FR-023)`
- green: added a `[Code]` section with `StopDaemon` (the pid record, then a PowerShell image-checked `Stop-Process` with a 5 s wait; with no record, `taskkill` scoped to the user), `PrepareToInstall` and `InitializeUninstall`, per T044. The three installer test files -> 3, 6 and 2 passed; `packaging_excludes_showcase` -> 13 passed.
- refactor: none
- notes: the test only pins the call graph. Whether `StopDaemon` compiles in ISCC and actually stops a daemon is verified only by the Windows smoke (A6–A9, T043/T045), which is blocked on a CI push. Mutant: replacing `Error := StopDaemon();` with `Error := '';` gave `\`InitializeUninstall\` must call \`StopDaemon\` ...` (1 failed). Restored from a copy, and cmp confirmed it.
- commit: uncommitted

## Cycle 26: U41 `[UninstallDelete]` is exactly the runtime dir

- test: `crates/micold-core/tests/windows_installer_in_use.rs::uninstall_removes_only_the_runtime_dir`
- red: `scripts/build-lock.sh cargo test -p micold-core --test windows_installer_in_use uninstall_removes_only_the_runtime_dir -- --exact` -> `running 1 test`, `uninstall must remove the daemon's runtime dir and nothing else of the user's (FR-007)` `left: []`
- green: added `[UninstallDelete]` with `Type: filesandordirs; Name: "{localappdata}\micold-ai-ide\run"`. in_use -> 4 passed, per_user -> 6 passed.
- refactor: none
- commit: uncommitted

## Cycle 27: U42 no delete entry names user data

- test: `crates/micold-core/tests/windows_installer_in_use.rs::never_deletes_user_data`
- red: none. It passed on its first run, because an absence rule has nothing to implement.
  - Mutant 1: adding `Type: filesandordirs; Name: "{localappdata}\micold-ai-ide\data"` under `[UninstallDelete]` gave `[UninstallDelete] must not delete the user's settings or session data (FR-007); found [...]`.
  - Mutant 2: adding an `[InstallDelete]` `{userappdata}\micold-ai-ide` entry gave `[InstallDelete] must not delete ...`.
  - Both were restored from a copy, and cmp confirmed it -> 1 passed.
- green: n/a
- refactor: none
- notes: added U61 (the ready page states "Running sessions will be stopped."). It is in the contract's `[Code]` bullets and T044, but had no behavior.
- commit: uncommitted

## Cycle 28: U61 the ready page says "Running sessions will be stopped."

- test: `crates/micold-core/tests/windows_installer_in_use.rs::ready_page_says_sessions_will_stop`
- red: `scripts/build-lock.sh cargo test -p micold-core --test windows_installer_in_use ready_page_says_sessions_will_stop -- --exact` -> `running 1 test`, `[Code] must define \`UpdateReadyMemo\` to add the notice to the ready page (FR-009)`
- green: added `UpdateReadyMemo`, which rebuilds the standard memo and appends the notice. in_use -> 6 passed, per_user -> 6, version -> 2.
- refactor: the first green built the memo from a Pascal array literal. That assignment form is not certain to compile in Pascal Script, so it became plain `if`s. Re-run -> all green.
- notes: like U40, ISCC compilation is proven only on the Windows CI leg.
- commit: uncommitted

## Cycle 29: U43 refuses to run off Windows

- test: `scripts/tests/windows-installer.test.sh`, case "refuses to run off Windows" (new suite). `uname`, `cargo` and `iscc` are stubbed on PATH, and the stubs record their args in a temp dir. `MICOLD_NO_BUILD_LOCK=1`. It is already covered by the CI `scripts/tests/*.test.sh` glob.
- red: `scripts/tests/windows-installer.test.sh` (a shell suite has no single-case filter; it runs the whole file, which then held 1 case) -> `FAIL  refuses to run off Windows` / `want a non-zero exit on uname Linux, got 0`. It ran against a stub `scripts/windows-installer.sh` (`exit 0`), because a missing script is not a valid red.
- green: a `uname -s` case accepting `MINGW*|MSYS*|CYGWIN*`, and otherwise `the Windows installer is built on Windows` with exit 1 -> 1 case, 0 failures.
- refactor: the case's inline checks became `expect_refusal`. Mutant: the `*)` arm renamed to `NOPE)` gave `FAIL ... want a non-zero exit, got 0`. Restored -> 0 failures.
- commit: uncommitted

## Cycle 30: U44 `--arch` other than x64/arm64 is rejected

- test: `scripts/tests/windows-installer.test.sh`, cases "rejects --arch 'x86' / 'aarch64' / 'ARM64' / ''" (also: cargo not run)
- red: `scripts/tests/windows-installer.test.sh` -> `FAIL  rejects --arch 'x86'` `want a non-zero exit, got 0` (4 failures of 5)
- green: argument parsing (`--arch`, `--out-dir`; the default arch comes from `PROCESSOR_ARCHITECTURE`), and the arch maps to a triple, otherwise `--arch must be x64 or arm64` with exit 2 -> 5 cases, 0 failures.
- refactor: none
- commit: uncommitted

## Cycle 31: U45 iscc gets `/DAppVersion=` from `[workspace.package] version`

- test: `scripts/tests/windows-installer.test.sh`, case "passes the workspace version to iscc". The suite reads the version itself (`0.12.1`). `expect_args` was added.
- red: `scripts/tests/windows-installer.test.sh` -> `FAIL  passes the workspace version to iscc` `want iscc arguments containing \`/DAppVersion=0.12.1 \`, got: <not invoked>`
- green: a sed-read version, `iscc="${ISCC:-iscc}"`, and `"$iscc" /DAppVersion= /DArch= /O<out> <iss>` -> 6 cases, 0 failures.
- refactor: none
- notes: the green also passed `/DArch`, a step ahead of U46. The U46 entry has its mutant. Mutant: `/DAppVersion=0.0.0` gave `FAIL ... got: /DAppVersion=0.0.0 ...`. Restored, and cmp confirmed it.
- commit: uncommitted

## Cycle 32: U46 `/DArch=<arch>` and `/DBinDir=<target>/<triple>/release`

- test: `scripts/tests/windows-installer.test.sh`, cases "passes /DArch=x64|arm64 to iscc" and "points iscc at the <triple> release binaries". The target dir comes from `scripts/build-lock.sh --print-target-dir`.
- red: `scripts/tests/windows-installer.test.sh` -> `FAIL  points iscc at the x86_64-pc-windows-msvc release binaries` `want ... /DBinDir=/home/jaro/workspaces/micold-ai-ide/target-shared/x86_64-pc-windows-msvc/release `. That is 2 failures; the `/DArch` cases passed from cycle 31.
- green: `bin_dir="$target_dir/$triple/release"` passed as `/DBinDir`. The paths go through `winpath` (`cygpath -w` when present), and iscc runs under `MSYS2_ARG_CONV_EXCL='*'` so Git Bash does not rewrite `/D` switches -> 10 cases, 0 failures.
- refactor: none
- notes: `/DArch` passed on its first run. Mutant: hard-coding `/DArch=x64` gave `FAIL  passes /DArch=arm64 to iscc`. Restored, and cmp confirmed it. The `cygpath`/MSYS behavior is proven only on the Windows CI leg.
- commit: uncommitted

## Cycle 33: U47 no iscc -> the error names the Inno Setup download

- test: `scripts/tests/windows-installer.test.sh`, case "names the Inno Setup download when no iscc is found" (with the stub removed, `ISCC=`, and cargo not run)
- red: `scripts/tests/windows-installer.test.sh` -> `FAIL  names the Inno Setup download when no iscc is found` `want output containing \`https://jrsoftware.org/isdl.php\`, got: ... line 82: iscc: command not found`
- green: resolve `$ISCC`, then `${ProgramFiles(x86)}/Inno Setup 6/ISCC.exe`, then PATH; otherwise the message names `https://jrsoftware.org/isdl.php`, with exit 1 -> 11 cases, 0 failures.
- refactor: none
- commit: uncommitted

## Cycle 34: U48 the cargo invocation

- test: `scripts/tests/windows-installer.test.sh`, cases "builds the app and the daemon for <triple>" (x64, arm64)
- red: `scripts/tests/windows-installer.test.sh` -> `FAIL  builds the app and the daemon for x86_64-pc-windows-msvc` `want cargo arguments containing \`build --release --locked -p micold-client --bin micold-ai-ide -p micold-daemon --target x86_64-pc-windows-msvc\`, got: <not invoked>`
- green: `scripts/build-lock.sh cargo build --release --locked -p micold-client --bin micold-ai-ide -p micold-daemon --target $triple`, run after iscc is resolved. The default out-dir is `<target>/windows-installer`, and the output path is printed -> 13 cases, 0 failures.
- refactor: none
- notes: Mutant: dropping `--bin micold-ai-ide` gave 2 failures. Restored, and cmp confirmed it. shellcheck is not installed here, so it was not run.
- commit: uncommitted

## Session note: main not integrated

- PR #284 (028) merged; `origin/main` is 35 commits ahead, and HEAD has no commits of its own. Six uncommitted files overlap main's changes, plus `.specify/memory/tdd-profile.md`. Fast-forwarding and 3-way-merging the uncommitted work was refused by the permission classifier, so nothing was integrated.
- Deferred until main is in: U51, A10, A11 and U57 (`release.yml`: main adds the macOS job, the notice step and `release_publishes_complete_sets.rs`), the T054 `ci.yml` docs-job line (main edits the same lines), and the T037 mise task (main edits `mise.toml`).

## Cycle 35: U53 the release notice names the arches, the SmartScreen steps and the guide

- test: `crates/micold-core/tests/release_notice_windows.rs::notice_names_arches_smartscreen_steps_and_guide`
- red: `scripts/build-lock.sh cargo test -p micold-core --test release_notice_windows notice_names_arches_smartscreen_steps_and_guide -- --exact` -> `running 1 test` ... `missing ["x64", "ARM64", "More info", "Run anyway", "install-windows"]`. The notice file existed but was empty.
- green: `.github/release-notice-windows.md`, 3 non-empty lines: which setup exe to pick, **More info** then **Run anyway**, and a link to the install-windows guide -> 1 passed.
- refactor: none
- commit: uncommitted

## Cycle 36: U52 the release notice is at most 5 non-empty lines

- test: `crates/micold-core/tests/release_notice_windows.rs::notice_is_at_most_five_lines`
- red: none. It passed on its first run, because cycle 35's notice was already 3 lines.
- mutant: appending 3 lines to the notice gave `must stay within 5 non-empty lines (FR-010); it has 6`. Restored with cp, and cmp confirmed it.
- refactor: none
- notes: test-after relative to the notice text, with the mutant as evidence.
- commit: uncommitted

## Cycle 37: A12 site staging fails on a setup exe the release lacks

- test: `scripts/tests/site-stage.test.sh`, cases "a Windows guide linking a setup exe the release lacks fails the stage" and "the failure names the missing setup exe". The fixture's `install-windows.md` links both exes, but `MICOLD_RELEASE_ASSETS` lacks arm64.
- red: none. It passed on its first run: `site/stage.sh` already scans every staged page (028).
- mutant: in `site/stage.sh`, turning the membership case into `*) ;;` gave `FAIL  a Windows guide linking a setup exe the release lacks fails the stage` and `FAIL  the failure names the missing setup exe`. Restored, and cmp confirmed it.
- green: no source change (T052 needed none); the page is in `docs/SUMMARY.md`, and `site/checks/page-set.sh` passes with 18 pages.
- commit: uncommitted

## Cycle 38: U54 site staging passes with both setup exes present

- test: `scripts/tests/site-stage.test.sh`, case "a Windows guide linking both setup exes the release carries stages"
- red: none. It passed on its first run (existing behavior).
- mutant: `file="${link##*-}"` gave `FAIL  a Windows guide linking both setup exes the release carries stages` / `release v9.9.9 does not carry setup.exe`. Restored, and cmp confirmed it.
- commit: uncommitted

## Cycle 39: U58 the Windows guide has the lifecycle headings

- test: `crates/micold-core/tests/install_guide_windows.rs::windows_guide_has_the_lifecycle_headings`
- red: `... --test install_guide_windows windows_guide_has_the_lifecycle_headings -- --exact` -> `running 1 test` ... `missing ["Download", "Install", "Upgrading", "Removing", "Limits"], found ["Installing on Windows"]`. The page held only its title.
- green: `docs/user-guide/install-windows.md` gets Before you start, Download, Install, Upgrading, Removing (with What uninstall keeps), and Limits (unsupported architecture only). This covers T041 and T046 -> 1 passed.
- commit: uncommitted

## Cycle 40: U59 the guide names SmartScreen, Smart App Control and building from source

- test: `crates/micold-core/tests/install_guide_windows.rs::windows_guide_names_smartscreen_and_smart_app_control`
- red: `... windows_guide_names_smartscreen_and_smart_app_control -- --exact` -> `running 1 test` ... `missing ["Smart App Control", "from source"]`
- green: the Limits section gets bullets for Smart App Control (no per-app override; build from source) and for AppLocker/WDAC -> 2 passed.
- notes: `from source` is asserted with U59 because it is the Smart App Control workaround T054 lists.
- commit: uncommitted

## Cycle 41: A15 the Limits section says sessions end at logout, except in the container

- test: `crates/micold-core/tests/install_guide_windows.rs::windows_guide_limits_say_sessions_end_at_logout`
- red: `... windows_guide_limits_say_sessions_end_at_logout -- --exact` -> `running 1 test` ... `must say sessions do not survive logging out ... mentioning \`logging out\``
- green: Limits bullets for logging out (linking `sandboxed-daemon.md` and `../daemon.md`) and for one service per account -> 3 passed.
- commit: uncommitted

## Cycle 42: A14 install.md no longer says Windows has no package

- test: `crates/micold-core/tests/install_guide_windows.rs::install_page_no_longer_says_windows_has_no_package`
- red: `... install_page_no_longer_says_windows_has_no_package -- --exact` -> `running 1 test` ... `still says ["no packaged build for macOS or Windows"]`
- green: `docs/install.md` "macOS and Windows" becomes `## Windows` (download table, SmartScreen steps, guide link), `## macOS` (the "no packaged build" wording narrowed to macOS; 028 did not change this page) and `## Build from source` -> 4 passed. `site/checks/links.sh --sources` passes, including the `#build-from-source` fragment.
- commit: uncommitted

## Cycle 43: U56 install.md links the Windows guide

- test: `crates/micold-core/tests/install_guide_windows.rs::install_page_links_the_windows_guide`
- red: none. It passed on its first run (the link was written in cycle 42).
- mutant: changing the link to `(user-guide/install-macos.md)` gave `must link the Windows guide as \`(user-guide/install-windows.md)\``. Restored, and cmp confirmed it.
- commit: uncommitted

## Cycle 44: A10 the release workflow has a Windows job that uploads both setup executables

- test: `crates/micold-core/tests/release_publishes_complete_sets.rs::a_windows_job_uploads_both_setup_executables`
- red: `scripts/build-lock.sh cargo test --test release_publishes_complete_sets a_windows_job_uploads_both_setup_executables -- --exact` -> `running 1 test` ... `missing ["arch: x64", "runner: windows-latest", "arch: arm64", "runner: windows-11-arm"], upload jobs ["deb", "macos"]`
- green: `.github/workflows/release.yml` gets the `windows` job after `macos` (T050: x64/windows-latest and arm64/windows-11-arm legs, pinned checkout/toolchain/cache, `windows-installer.sh`, `windows-install-smoke.sh`, `gh release upload`) -> the test passes. The file's other gate, `publish_waits_for_every_job_that_attaches_an_artifact`, now fails with `not in \`publish\`'s \`needs:\`: ["windows"]`, which is U51 firing on the real workflow; cycle 45 closes it.
- notes: the smoke step calls `scripts/windows-install-smoke.sh`, which T033 has not created yet. The release job cannot succeed until it exists.
- commit: uncommitted

## Cycle 45: A11 publish waits for the Windows job

- test: `crates/micold-core/tests/release_publishes_complete_sets.rs::publish_waits_for_the_windows_job`
- red: `... --test release_publishes_complete_sets publish_waits_for_the_windows_job -- --exact` -> `running 1 test` ... `` `publish` must wait for `windows` (FR-014, SC-002); its `needs:` is ["release-please", "deb", "macos", "image-manifest"] ``
- green: `publish.needs` becomes `[release-please, deb, macos, windows, image-manifest]`; the header comment names the Windows job -> 8 passed.
- commit: uncommitted

## Cycle 46: U51 every uploading job gates publish

- test: `crates/micold-core/tests/release_publishes_complete_sets.rs::publish_waits_for_every_job_that_attaches_an_artifact` (028's, merged from main)
- red: observed in cycle 44 on the real workflow, before `windows` joined `needs:`: `these release.yml jobs upload a release asset but are not in \`publish\`'s \`needs:\`: ["windows"]`. Green since cycle 45.
- notes: the row said "equals `publish.needs` minus `release-please`". 028's rule is inclusion, and deliberately so: `image-manifest` gates `publish` without uploading, and `a_job_that_uploads_nothing_needs_no_entry` pins that. No new test; the existing one serves FR-014 and SC-002 as written.
- commit: uncommitted

## Cycle 47: U57 publish appends the Windows notice

- test: `crates/micold-core/tests/release_notice_windows.rs::publish_appends_the_windows_notice`
- red: `... --test release_notice_windows publish_appends_the_windows_notice -- --exact` -> `running 1 test` ... `the \`publish\` job in release.yml must append \`.github/release-notice-windows.md\` to the release body (FR-010); it does not reference it`
- green: 028's "Add the macOS trust notice" step becomes "Add the macOS and Windows trust notices" and loops over both notice files, one `gh release edit --notes-file` -> 3 passed.
- commit: uncommitted


## Cycle 48: U62 the smoke refuses a non-Windows host

- test: `scripts/tests/windows-install-smoke.test.sh` "refuses to run off Windows"
- red: `scripts/tests/windows-install-smoke.test.sh` against an empty, executable `scripts/windows-install-smoke.sh` -> `FAIL  refuses to run off Windows` / `want exit 1, got 0`
- green: the `uname -s` case (MINGW*/MSYS*/CYGWIN*, else exit 1) -> 1 case, 0 failures.
- notes: U62 and U63 were added to the list here. T033 describes only the Windows-side assertions, but the script's host and argument decisions run on any host, so they are pinned locally like `windows-installer.sh`'s U43/U44.
- commit: uncommitted

## Cycle 49: U63 the smoke refuses a bad argument

- test: `scripts/tests/windows-install-smoke.test.sh` "refuses to run without a setup executable", "refuses more than one setup executable", "names a setup executable that does not exist"
- red: `scripts/tests/windows-install-smoke.test.sh` -> 3 failures, each `want exit 2, got 0`
- green: the `$# -ne 1` usage check and the `[ ! -f "$exe" ]` check -> 4 cases, 0 failures.
- notes: the Windows-side body (T033 steps 1-8, behaviors A1-A4) was written after this, as assertions only, and has not run: it can only go red or green on a Windows runner (T040). Deviations from T033's text, each forced by Git Bash: the SID comes from `WindowsIdentity::GetCurrent()` rather than `whoami /user /fo csv /nh`, and the `DisplayVersion` from `Get-ItemProperty` rather than `reg query`, because MSYS2 rewrites `/switch` arguments into paths. The client is launched with `Start-Process -PassThru` without redirection, because redirecting a console-subsystem child hands it PowerShell's console and would make the conhost assertion vacuous. That also removes stderr, so there is no graphics-surface exemption: every client exit fails, which is stricter than T033.
- commit: uncommitted

## Cycle 50: U50 the ARM64 packaging job is gated

- test: `crates/micold-core/tests/ci_gate_covers_every_job.rs::every_job_is_covered_by_the_gate`
- red: after adding the `windows-arm64-package` job (T039), `scripts/build-lock.sh cargo test -p micold-core --test ci_gate_covers_every_job every_job_is_covered_by_the_gate -- --exact` -> `running 1 test` ... `these ci.yml jobs are not in \`ci-complete\`'s \`needs:\` ... ["windows-arm64-package"]`
- green: `ci-complete.needs` gains `windows-arm64-package`, with `WINARM`, `check windows-arm64 "$WINARM"` and a summary row -> 3 passed.
- notes: T038's x64 step in `test` and T039's job both exist, but T033/T038/T039 carry A1-A4, which stay PENDING until T040 observes the Windows legs.
- commit: uncommitted

## Cycle 51: guard over main's new unix-gated daemon tests

- test: `crates/micold-core/tests/daemon_tests_gate_with_reason.rs::unix_gated_daemon_test_files_state_a_reason`
- red: PR #314's first CI run on every OS, then locally after merging `origin/main` (b94a6d8f): `... --test daemon_tests_gate_with_reason` -> `running 1 test` ... `compiled out on Windows with no stated reason ... ["pi_launch_wiring.rs", "supervision_slow_crash_loop.rs"]`
- green: a truthful `// unix-only:` line above each gate (both write a `#!/bin/sh` script made executable with `PermissionsExt`) -> 1 passed.
- notes: the files came from main (features 029 and 005), not from this branch; the guard is 030's.
- commit: see the merge follow-up commit

## Cycle 52: U64 build-lock.sh runs without flock

- test: `scripts/tests/build-lock.test.sh` "runs the command unlocked when flock is missing"
- red: `scripts/tests/build-lock.test.sh` -> `want exit 3 (the command's), got 127; output: .../build-lock.sh: line 84: flock: command not found`. The same exit 127 ended `windows-arm64-package` in PR #314's first CI run.
- green: when `flock` is not on `PATH`, the wrapper says so and runs the command unlocked -> 1 case, 0 failures; every shell suite passes and a locked `scripts/build-lock.sh true` still runs.
- commit: see the merge follow-up commit

## Cycle 53: U65 windows-installer.sh resolves a relative target dir

- test: `scripts/tests/windows-installer.test.sh` "resolves a relative CARGO_TARGET_DIR before handing it to iscc"
- red: PR #314's second CI run (01f5e2f4), both Windows packaging legs, after cargo built: `Error on line 59 in ...\packaging\windows\micold-ai-ide.iss: Source file "...\packaging\windows\target\aarch64-pc-windows-msvc\release\micold-ai-ide.exe" does not exist.` Locally: `scripts/tests/windows-installer.test.sh` -> `FAIL  resolves a relative CARGO_TARGET_DIR before handing it to iscc` ... `got: ... /DBinDir=target/aarch64-pc-windows-msvc/release ...`
- green: an `absolute` helper resolves the target dir against `$PWD`, as cargo does -> 14 cases, 0 failures.
- commit: see the follow-up commit

## Cycle 54: U66 windows-installer.sh resolves a relative out dir

- test: `scripts/tests/windows-installer.test.sh` "resolves a relative --out-dir before handing it to iscc"
- red: `scripts/tests/windows-installer.test.sh` -> `FAIL  resolves a relative --out-dir before handing it to iscc` ... `got: ... /O../../../../../../../tmp/tmp.BMUif2O6Dp/out ...`
- green: `out_dir` goes through the same `absolute` helper -> 15 cases, 0 failures.
- notes: not yet observed on CI, where compile stopped at cycle 53's error before `/O` mattered. Without it, `--out-dir dist` would put the setup exe beside the `.iss`, and the smoke's `dist/*-setup.exe` would name a file that does not exist.
- commit: see the follow-up commit

## Cycle 55: correct U46's expectation for a relative target dir

- test: `scripts/tests/windows-installer.test.sh` "points iscc at the <triple> release binaries" (U46)
- red: PR #314's third CI run (5bc16f1c), `fmt + clippy` shell suites, where `CARGO_TARGET_DIR: target`: `FAIL` ... `want iscc arguments containing \`/DBinDir=target/aarch64-pc-windows-msvc/release \`, got: ... /DBinDir=/home/runner/work/micold-ai-ide/micold-ai-ide/target/aarch64-pc-windows-msvc/release`. Locally with `CARGO_TARGET_DIR=target scripts/tests/windows-installer.test.sh` -> 2 failures.
- green: the test's own `TARGET_DIR` is resolved from `$ROOT` when relative, the rule U65 set -> 15 cases, 0 failures both with and without `CARGO_TARGET_DIR=target`; every shell suite passes under `CARGO_TARGET_DIR=target`.
- notes: a test change, not a weakening. U46 took `--print-target-dir` verbatim, which is only right when that path is absolute; the local shared dir always was, so cycle 53 missed it.
- commit: see the follow-up commit

## Cycle 56: correct U48, the daemon's bin must be named too

- test: `scripts/tests/windows-installer.test.sh` "builds the app and the daemon for <triple>" (U48)
- red: PR #314's fourth CI run (3f532f1c), `package + smoke (windows-11-arm)`: `Error on line 60 in C:\a\micold-ai-ide\micold-ai-ide\packaging\windows\micold-ai-ide.iss: Source file "C:\a\micold-ai-ide\micold-ai-ide\target\aarch64-pc-windows-msvc\release\micold-daemon.exe" does not exist.` (line 59, `micold-ai-ide.exe`, was found). With the expectation corrected, `scripts/tests/windows-installer.test.sh` -> `FAIL  builds the app and the daemon for x86_64-pc-windows-msvc` ... `got: build --release --locked -p micold-client --bin micold-ai-ide -p micold-daemon --target x86_64-pc-windows-msvc` -> 2 failures.
- green: `--bin micold-daemon` added to the script's cargo invocation -> 15 cases, 0 failures; every shell suite passes, also under `CARGO_TARGET_DIR=target`.
- notes: a test change, not a weakening. cargo applies a `--bin` filter to every selected package, so `-p micold-daemon` without its own `--bin` built no exe; `.claude/skills/visual-pass/SKILL.md` already records this. U48, the contract and T025's command are corrected to match.
- commit: see the follow-up commit

## Cycle 57: the Windows install smoke runs on CI; A3 is red on the endpoint stub

- test: `scripts/windows-install-smoke.sh`, run by `.github/workflows/ci.yml` on `build + test (windows-latest)` and `package + smoke (windows-11-arm)` (A1, A2, A3, A4, A13)
- red: PR #314's fifth CI run (3138b577), both legs, at step 6: x64 `windows-install-smoke.sh: FAIL: \\.\pipe\Micold.Daemon.S-1-5-21-3699639565-2515463329-295617607-500 did not appear within 20s (I2)`; arm64 `windows-install-smoke.sh: FAIL: \\.\pipe\Micold.Daemon.S-1-5-21-2750905264-1129093905-494693804-500 did not appear within 20s (I2)`. A3 is `RED`.
- green: steps 1-5 passed on both legs, so A1, A4 and A13 are `DONE`: x64 `Built: /d/a/micold-ai-ide/micold-ai-ide/dist/micold-ai-ide-0.14.0-x64-setup.exe` (A13); the Inno log's `Installation process succeeded.`, both exes and `Micold AI IDE.lnk` created, `installed 0.14.0 to /c/Users/runneradmin/AppData/Local/Programs/Micold AI IDE` (A1, A4); `== launched the client, pid 3696` (x64) and `pid 4164` (arm64).
- notes: the red is the one planned for. `crates/micold-core/src/endpoint.rs`'s Windows `resolve()` is still a stub, so the client cannot reach the daemon (T016-T021). A2 is not reached: the conhost check is step 7, after the pipe. T033/T038/T039/T040 stay open until A2 and A3 are green. No code change in this cycle.
- commit: see the follow-up commit

## Cycle 58: the install smoke is non-blocking until the endpoint lands

- test: none changed. `scripts/windows-install-smoke.sh` and its assertions are untouched.
- red: n/a. A3 stays `RED` per cycle 57.
- green: n/a.
- notes: the user chose to merge PR #314 with A3 still red. `ci-complete` is required and `--admin` cannot bypass it, so both "Install and launch" steps in `.github/workflows/ci.yml` carry `continue-on-error: true`. The x64 build was split into its own blocking step, "Package the Windows installer", so a broken setup exe still fails CI. T040 now says to remove both `continue-on-error`s. This is a deliberate, user-approved gate relaxation, not a green.
- commit: see the follow-up commit

## Cycle 59: U67 dropping a session returns on Windows

- test: `crates/micold-daemon/src/supervisor.rs::windows_tests::dropping_a_session_returns` (new)
- red: PR #332's CI run 34819876930 (41915aef), `build + test (windows-latest)`, step `Test (daemon, Windows)`: `test supervisor::windows_tests::dropping_a_session_returns ... FAILED`, then `test supervisor::windows_tests::kill_reaps_grandchild has been running for over 60 seconds` and `The action 'Test (daemon, Windows)' has timed out after 20 minutes.` The failure message itself was not printed, because libtest prints it at the end and the step timed out first. The test's only assertion is `dropping the session did not return within 10s`.
- green: `PtySession`'s `Drop` closes the PTY master before it joins the reader thread; `master` is now `Mutex<Option<..>>`. Linux: `cargo test -p micold-daemon --lib supervisor` -> 6 passed. Windows green is pending the next CI run.
- notes: found by the hang in run 34813428322 (353214b5), where U13 never finished. Under ConPTY the reader's pipe reaches EOF only when the pseudoconsole closes, not when the child exits. `Drop` joined first, so every Windows session teardown hung, and U13 hung while unwinding. Added mid-loop as its own behavior. The Windows daemon step gained `timeout-minutes: 20`.
- commit: see the follow-up commit
