# Cycle Log: 1289 — sandbox project setup

Append only. Newest last. Every entry's `red` block is the evidence that the test existed and failed
before the implementation. Commands ran with `CARGO_INCREMENTAL=0` and debuginfo 0 (disk was short);
output is copied from the session's logs.

## M1 cycle 1: A1 the toolchain `PATH` order (T001 → T002)

- test: `crates/micold-core/tests/toolchain_path.rs`, all 14 rows of contract toolchain-path P3 and
  the FR-003/FR-004 cases (`p3_offered_bins_then_local_bin_then_default_shims_then_rest`,
  `p3_mise_data_dir_names_the_shims_folder`, `p3_xdg_data_home_names_the_shims_folder_without_mise_data_dir`,
  `p3_a_home_folder_already_in_rest_moves_to_its_place`, `p3_no_offered_bins_still_adds_the_home_folders`,
  `mise_data_dir_wins_over_xdg_data_home`, `empty_mise_data_dir_and_xdg_data_home_count_as_unset`,
  `an_empty_home_counts_as_unset`, `without_home_both_home_folders_are_left_out`,
  `without_home_mise_data_dir_still_names_the_shims_folder`, `a_folder_repeated_in_rest_keeps_only_its_first_position`,
  `an_offered_bin_repeated_in_rest_is_dropped_from_rest`, `an_empty_rest_adds_no_empty_entry`,
  `folders_are_named_whether_or_not_they_exist`)
- red: `scripts/build-lock.sh cargo test -p micold-core --test toolchain_path` ->
  `error[E0432]: unresolved import micold_core::sandbox::toolchain_path` at
  `crates/micold-core/tests/toolchain_path.rs:8:27` (`could not find toolchain_path in sandbox`), exit 101
- green: T002 adds `core/src/sandbox/toolchain_path.rs`; same command ->
  `test result: ok. 14 passed; 0 failed`
- commit: `65b4a48c`

## M1 cycle 2: A2 (a), (b), (c) and A3 the daemon gives every sandbox spawn that `PATH` (T003, T004 → T005)

- tests (`crates/micold-daemon/src/state.rs`, `offered_tests`):
  A2 (a) `a_sandbox_spawn_path_puts_the_home_folders_after_the_offered_bins`,
  A2 (b) `an_empty_offer_still_adds_the_home_folders`,
  A2 (c) `without_an_offer_the_resolved_path_is_unchanged`,
  A3 `a_home_copy_of_a_cli_is_not_offered_ahead_and_a_shim_resolves`
- red: `scripts/build-lock.sh cargo test -p micold-daemon --lib offered` -> `4 passed; 4 failed`:
  - A2 (a) `state.rs:8487:9` `left: "/tmp/.tmp2VAGiH/claude-bin:/usr/local/bin:/usr/bin:/bin"`,
    `right: "/tmp/.tmp2VAGiH/claude-bin:/home/u/.local/bin:/home/u/.local/share/mise/shims:/usr/local/bin:/usr/bin:/bin"`
  - A2 (b) `state.rs:8512:9` `left: "/usr/bin:/bin"`, `right: "/home/u/.local/bin:/opt/mise/shims:/usr/bin:/bin"`
    (the early return on an empty offer)
  - A2 (c) `state.rs:8456:14` `a PATH in the spawn environment`. A2 (c) checks FR-008, which the
    code before T005 already met, so this failure is not evidence for T005: its red is the mutant
    in M1 cycle 6
  - A3 `state.rs:8563:9` `left: None`, `right: Some("/tmp/.tmpIF6Pap/home/.local/share/mise/shims")`
- green: T005 `with_sandbox_path`; same command -> `test result: ok. 8 passed; 0 failed`
- commit: `65b4a48c`

## M1 cycle 3: review A-F1 a folder holding the separator drops only itself

- tests (`crates/micold-core/tests/toolchain_path.rs`):
  `a_home_holding_the_separator_leaves_out_only_the_home_folders` (`HOME=/a:b`),
  `an_xdg_data_home_holding_the_separator_leaves_out_only_the_shims_folder` (`XDG_DATA_HOME=/a:/b`)
- red (tests added, `toolchain_path.rs` as at `65b4a48c`):
  `scripts/build-lock.sh cargo test -p micold-core --test toolchain_path` -> `14 passed; 2 failed`:
  - `toolchain_path.rs:171:5` `left: "/usr/local/bin:/usr/bin:/bin"`, `right: "/s/ai-clis/claude/bin:/usr/local/bin:/usr/bin:/bin"`
  - `toolchain_path.rs:178:5` `left: "/usr/local/bin:/usr/bin:/bin"`, `right: "/s/ai-clis/claude/bin:/home/u/.local/bin:/usr/local/bin:/usr/bin:/bin"`
- green: each candidate is kept only if `join_paths([folder])` succeeds; same command ->
  `test result: ok. 16 passed; 0 failed`

## M1 cycle 4: review A-F3 an offered CLI must be in its own `bin` folder

- test: `state.rs` `an_offered_cli_missing_from_its_bin_is_not_offered_through_a_home_copy`
  (offered bin empty, then missing; `claude` only in `$HOME/.local/bin`)
- red (`only_offered` as at `65b4a48c`): `scripts/build-lock.sh cargo test -p micold-daemon --lib offered`
  -> `8 passed; 1 failed`, `state.rs:8621:9` panicked: `bin lacks it`
- green: `only_offered` keeps a CLI only when `provider().is_available(bin)`; the cycle 5 run below
  -> that test `ok`

## M1 cycle 5: review A-F2 the A2 (a) and A3 tests do not depend on the test process's `MISE_DATA_DIR`

- tests: A2 (a) and A3 above; their scripts now export `MISE_DATA_DIR=` (empty counts as unset)
- red (without that export, `MISE_DATA_DIR=/inherited-mise` in the test process):
  `MISE_DATA_DIR=/inherited-mise scripts/build-lock.sh cargo test -p micold-daemon --lib offered`
  -> `7 passed; 2 failed`:
  - A2 (a) `state.rs:8509:9` `left: ".../claude-bin:/home/u/.local/bin:/inherited-mise/shims:/usr/local/bin:/usr/bin:/bin"`,
    `right: ".../claude-bin:/home/u/.local/bin:/home/u/.local/share/mise/shims:/usr/local/bin:/usr/bin:/bin"`
  - A3 `state.rs:8589:9` `left: None`, `right: Some("/tmp/.tmpvxHSLW/home/.local/share/mise/shims")`
- green: with the export, same command -> `test result: ok. 9 passed; 0 failed`

## M1 cycle 6: A2 (c) a daemon without `offered.json` keeps the resolved `PATH` (FR-008), mutant red

- test: `state.rs` `without_an_offer_the_resolved_path_is_unchanged`
- red (mutant: `with_sandbox_path`'s `let Some(offered) = self.offered_clis() else { return resolved; }`
  -> `let offered = self.offered_clis().unwrap_or(Offered { version: 1, clis: Vec::new() })`):
  `scripts/build-lock.sh cargo test -p micold-daemon --lib without_an_offer_the_resolved_path_is_unchanged`
  -> `state.rs:8555:9` `left: "/home/u/.local/bin:/home/u/.local/share/mise/shims:/a:/usr/bin:/bin"`,
  `right: "/a:/usr/bin:/a:/bin"` (1 failed). Restored from a copy; `git diff` unchanged
- green: no production change

### Red CI: another daemon's offer in a shared state directory (`sandbox_real_history`, FR-008)

- test: `state.rs` `another_daemons_offer_in_a_shared_state_directory_changes_nothing`
- red (fix line removed from `offered_clis`):
  `cargo test -p micold-daemon --lib offered_tests` -> `left: "/var/lib/micold-ai-ide/ai-clis/ClaudeCode@1/bin:/home/u/.local/bin:/home/u/.local/share/mise/shims:<host-bin>:/usr/bin:/bin"`,
  `right: "<host-bin>:/usr/bin:/bin"` (1 failed)
- green: `offered_clis` returns `None` for an offer naming a `bin` outside its `ai-clis` dir; 10/10
  `offered` tests pass, `scripts/gate.sh` exit 0

## M2 cycle 1: A4 model — command, run record, transitions, labels, reason lines (T008 → T009)

- tests: `crates/micold-core/tests/setup_model.rs` (13): blank → `None`, trimmed with inner
  newlines; every Running → ended row succeeds, Running → Running and every finish from an ended
  state refused (record unchanged); `ended_at` is `Some` exactly when not running (also refused on
  deserialize); U2 tag texts; SC-004 reason lines; C2 JSON shape; postcard round trip; plain-string
  command, blank one refused
- red: `scripts/build-lock.sh cargo test -p micold-core --test setup_model` ->
  `error[E0432]: unresolved import micold_core::setup` (no module)
- green: `core/src/setup/mod.rs` -> `test result: ok. 13 passed; 0 failed`

## M2 cycle 2: A4 output — lines, `\r`, ANSI, lossy UTF-8, 10,000-line cap (T010 → T011)

- tests: `crates/micold-core/tests/setup_output.rs` (8)
- red: `cargo test -p micold-core --test setup_output` -> `error[E0433]: cannot find setup in micold_core`
- green: `core/src/setup/output.rs` (no ANSI helper existed: `grep -rn '\x1b\['` found only the
  paste markers) -> `test result: ok. 8 passed; 0 failed`

## M2 cycle 3: A4 shell — `shell_line` rows and `bash_on_path` (T012 → T013)

- tests: `crates/micold-core/tests/setup_shell.rs` (4)
- red: `cargo test -p micold-core --test setup_shell` -> `error[E0432]: unresolved import micold_core::setup`
- green: `core/src/setup/shell.rs`, `provider::resolve_on_path` made `pub(crate)` ->
  `test result: ok. 4 passed; 0 failed`

## M2 cycle 4: A5 `StreamingChild` — per-stream order, stdin EOF, tree terminate, exit kinds (T014 → T015)

- tests: `crates/micold-core/tests/streaming_child.rs` (7 on Unix; a `#[cfg(windows)]` module covers
  order, stdin EOF and terminate through `powershell.exe -Command`, run by CI's Windows job)
- red: `cargo test -p micold-core --test streaming_child` -> `error[E0432]: unresolved imports
  micold_core::process::ExitKind, micold_core::process::Stream, micold_core::process::StreamingChild`
- green: `StreamingChild`, `Terminator`, `ExitKind` in `process.rs`; the group kill / job kill is
  `TreeKill`, shared with `run_bounded` -> `test result: ok. 7 passed; 0 failed`; `run_bounded`'s
  consumers green: `--test env_include_resolve` 13/13, `--test env_include` 6/6, `--lib process` ok
- mutant red (group kill → leader only: `libc::kill(-(pid…))` → `libc::kill(pid…)`):
  `unix::terminate_ends_the_command_and_every_process_it_started ... FAILED` — `the grandchild 8630
  outlived the terminate` (6 passed; 1 failed). Restored from a copy
- mutant red (stdin `Stdio::null()` → `Stdio::piped()`): `cargo test --test streaming_child -- stdin`
  hung until `timeout 90` killed it (exit 143). Restored from a copy

## M2 cycle 5: A14 `setup_commands` in `settings.json`, kept by the daemon's write (T016 → T017)

- tests: `settings.rs` `setup_commands_tests` (5: round trip with the C3 shape, empty map writes no
  key, a file without the key loads empty and keeps `settings_version` 4, a blank entry is dropped
  and the rest kept, `SetupCommandChange` sets/removes one project); `catalog.rs`
  `the_service_write_keeps_a_setup_command`
- red: before T017 neither compiles (`Settings` has no `setup_commands`, no `SetupCommandChange`).
  Mutant (on-disk mirror `setup_commands: settings.setup_commands.clone()` → `BTreeMap::new()`):
  `cargo test -p micold-core --lib -- settings` -> `a_setup_command_round_trips_through_the_file ...
  FAILED` (43 passed; 1 failed); `cargo test -p micold-daemon --lib -- catalog` ->
  `the_service_write_keeps_a_setup_command ... FAILED`, `left: None, right: Some(SetupCommand("mise
  trust && mise install"))`. Restored from a copy
- green: field on `Settings` and `StoredSettings` (`known_values`, skip when empty),
  `SetupCommandChange` -> `--lib -- settings` 44/44, daemon `--lib -- catalog` 13/13. The client's
  `ValidSettings::into_settings` and `set_changed` name the field (a save never writes it), and
  `saving_a_theme_keeps_every_other_setting` now holds a setup command
