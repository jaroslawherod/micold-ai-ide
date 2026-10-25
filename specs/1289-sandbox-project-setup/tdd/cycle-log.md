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

## M2 cycle 6: review A round 1 fixes — line length cap, pre-reap group kill, Drop, EINTR (A-F1–A-F4)

- tests: `setup_output.rs` `a_line_with_no_end_is_cut_at_max_line_bytes`; `streaming_child.rs`
  `a_natural_exit_still_ends_what_it_left_running`, `dropping_it_without_run_ends_and_reaps_its_tree`;
  `process.rs` `an_interrupted_read_is_retried_not_taken_for_end_of_file`
- red: `--test setup_output` -> `error[E0432]: unresolved import ...::MAX_LINE_BYTES` (A-F1);
  `--lib -- interrupted` -> `an_interrupted_read_is_retried_not_taken_for_end_of_file ... FAILED`,
  `left: [] right: "one two"` (A-F4: EINTR taken for EOF); `--test streaming_child` ->
  `dropping_it_without_run_ends_and_reaps_its_tree ... FAILED` (8 passed; 1 failed: the leader was
  not reaped, A-F3). `a_natural_exit_still_ends_what_it_left_running` was green before (it guards the
  A-F2 change); the pid-reuse race itself is not reproducible in a test
- green: `MAX_LINE_BYTES` (64 KiB): an unfinished line that reaches it is kept as a line and a new
  one starts; `TreeKill::leader_exited` (`waitid` + `WNOWAIT`) and the group kill before the reaping
  `try_wait`, under the `reaped` lock (Windows kills the job after the reap); `impl Drop for
  StreamingChild` kills and reaps when not reaped; one reader loop `read_chunks` (retries
  `Interrupted`) under `drain` and `stream_to` -> `--lib` 1588 passed, `--test streaming_child` 9/9,
  `--test setup_output` 9/9
- mutant red (pre-reap kill disabled: `if false && ...leader_exited()`): `--test streaming_child` ->
  `a_natural_exit_still_ends_what_it_left_running ... FAILED` — `the grandchild 9917 outlived the
  run`. Restored from a copy

## M2 cycle 7: review A round 2 MINORs — 40 MiB worst case, cut keeps a character whole

- tests: `setup_output.rs` `the_buffer_is_bounded_even_for_binary_output`,
  `a_cut_line_does_not_split_a_character`
- red: `--test setup_output` -> both FAILED (9 passed; 2 failed): the worst case was 10,000 × 64 KiB
  (about 640 MiB), and a cut at the limit split `é` into U+FFFD halves
- green: `MAX_LINE_BYTES` 64 KiB → 4 KiB; the cut waits up to `CHAR_SLACK` (3) continuation bytes;
  `keep` cuts a decoded line back to `MAX_LINE_BYTES + 3` at a char boundary (invalid bytes widen to
  U+FFFD) -> 11/11. FR-021's 10,000-line floor holds; the worst case is about 40 MiB. An escape
  sequence open at the cut is still split (accepted, as the review allowed)
- mutant red (decoded cut removed): `the_buffer_is_bounded_even_for_binary_output ... FAILED`;
  (continuation slack off): `a_cut_line_does_not_split_a_character ... FAILED`. Restored from a copy

## M3 cycle 1: A15 the setup messages travel, protocol 69 (T018 → T019)

- tests: `crates/micold-core/src/protocol/messages.rs` `setup_wire_tests`:
  `the_client_setup_messages_encode_and_decode`, `the_daemon_setup_messages_encode_and_decode`,
  `the_setup_messages_bump_the_protocol_to_69`
- red: `cargo test -p micold-core --lib -- setup_wire_tests` -> `error[E0599]: no variant named
  ProjectSetupSet found for enum messages::ClientMsg` (and `SetupRun`, `SetupStop`,
  `SetupOutputWatch`, `SetupOutputUnwatch`, `SetupChanged`, `SetupOutput`; `SessionLocation` not in
  scope), 14 errors
- green: T019 adds the five `ClientMsg` and two `DaemonMsg` variants, `SessionLocation` gains
  `Hash, Serialize, Deserialize`, `PROTOCOL_VERSION` 68 → 69 (`protocol_roundtrip.rs` and
  `schema_hash.rs` pins follow); `cargo test -p micold-core --lib -- protocol` -> `32 passed`

## M3 cycle 2: A6, A8, A9, A10 the daemon runs setup; A7 the MCP result (T020–T024 → T025–T029)

- tests: `crates/micold-daemon/tests/setup_run.rs` (A6
  `a_new_worktree_runs_the_setup_command_without_waiting_for_it`, `a_session_starts_while_setup_runs`;
  A8 `a_failing_command_reports_its_exit_status`, `a_run_whose_folder_is_gone_could_not_start`,
  `with_no_command_a_create_runs_nothing`; A10 `only_a_worktree_the_app_creates_runs_setup`; A9
  `run_and_stop_setup`), `crates/micold-daemon/tests/mcp_create_worktree_setup.rs` (A7, 2 tests)
- red: `cargo test -p micold-daemon --test setup_run --test mcp_create_worktree_setup` ->
  `mcp_create_worktree_setup`: `error[E0599]: no method named set_setup_command found for struct
  Arc<DaemonState>`; `setup_run` (`-- setup`): 4 FAILED at `setup_run.rs:86` `the request is
  answered` (the server ignored `ProjectSetupSet`; the other three wait on the same answer)
- green: T025 `daemon/src/setup.rs` (`SetupRuns`, the run task, 5 unit tests written beside it),
  T026 `ops::create_worktree` starts setup and returns `SetupStart`, T027/T028 the five handlers and
  the disconnect unwatch, T029 the `setup` field -> `setup_run` 7 passed, `mcp_create_worktree_setup`
  2 passed; `mcp_create_worktree` 43 passed after its exact-row expectations gained `"setup": "none"`

## M3 cycle 3: review A round 1 fixes (A-F1, A-F2, A-F4)

- tests: `daemon/src/setup.rs` `a_backlog_bigger_than_a_frame_goes_out_in_several_encodable_batches`
  (A-F1: 5,000 × 4,000-byte lines, a flush and a watch from 0 each send several contiguous
  `SetupOutput`s, every one under `MAX_FRAME_LENGTH`); `daemon/src/state.rs`
  `setup_lock_tests::keeping_setup_output_does_not_wait_on_the_state_lock` (A-F2: a push while the
  state lock is held finishes within 2 s)
- red: `cargo test -p micold-daemon --lib -- setup` -> 2 FAILED (`setup.rs:547` the frame-size
  assertion; `state.rs:8801` "the push waited on the state lock")
- green: `send_from` splits a backlog or flush into batches of at most `BATCH_BYTES` (1 MiB) of line
  bytes; `SetupRuns` behind its own `DaemonState.setup` mutex (order `setup` → `inner`; `inner`
  only when there is something to send; `deregister` unwatches after releasing `inner`); A-F4
  `ops::create_worktree` calls `setup::start_in` with the created worktree's path (no red test: the
  refreshed-list miss cannot be provoked without fault injection) -> `-- setup` 8 passed


## M4 cycle 1: setup records and logs in the store (T031, T035)

- tests: `core/src/store.rs` `setup_store_tests` (paths by project and location, round trip of
  record/log/counter, a header-less log starts at 0, `remove_setup` of one location or the
  project, a corrupt record quarantined)
- red: `cargo test -p micold-core --lib -- setup_store` -> `error[E0599]: no method named
  setup_record_path found for struct JsonFileStore` (34 errors)
- green: `JsonFileStore::setup_dir`, `setup_counter_path`, `setup_record_path`, `setup_log_path`,
  `load_setup`, `save_setup_record`, `write_setup_log`, `append_setup_log`, `remove_setup`,
  `load_setup_counter`/`save_setup_counter`, `SetupLog`, `StoredSetup`, trait
  `ProjectStore::setup_files` -> 5 passed. `OutputBuffer::restored` and its test
  `a_restored_buffer_keeps_the_logs_tail` were written together (no separate red run)

## M4 cycle 2: restart, unwind, delete, attach (T032–T034, T036–T039)

- tests: `daemon/tests/setup_run.rs` A11 `a_running_record_found_at_start_is_interrupted_with_its_output`,
  `an_unwind_during_a_run_interrupts_it_and_ends_its_processes`,
  `the_run_id_after_a_restart_is_past_every_record_and_the_counter`; A12
  `deleting_a_worktree_stops_its_setup_first_and_forgets_it` (also delete -> recreate same
  `dir_name` -> setup runs again, not Busy, fresh id: M3 review A-F3); A13
  `attaching_later_gets_each_locations_last_run`
- red (against M3's daemon): 5 FAILED — `setup_run.rs:154` output never arrives (nothing loaded),
  `:759` no record after unwind, `:810` id after restart, `:833` no record file, `:928` the message
  after `Attached` is not a `SetupChanged`
- green: `SetupStore` (counter, record, log writes off both locks), `SetupRuns::loaded`,
  `interrupt_all`, `stop_waiter`/`stop_and_wait`, `forget`, `forget_project`, `on_attach`;
  `unwind` interrupts first; `delete_worktree` stops and waits before git, forgets after;
  `Attach` sends the runs right after `Attached` -> `setup_run` 12 passed, daemon `-- setup` 10
  passed
- after green, unit tests `a_log_past_its_bound_is_cut_back_to_the_buffer` and
  `after_an_interrupt_the_runs_own_end_is_dropped` pin the compaction and the dropped late end
  (they passed on first run)

## M4 cycle 3: review round 1 fixes (A-F1–A-F4, B-F1, B-F2)

- B-F2 red for the compaction: with `log_write`'s `LOG_COMPACT_AT` branch broken (`if false && …`),
  `a_log_past_its_bound_is_cut_back_to_the_buffer` FAILED at `setup.rs:1102` *the log is cut
  back*; branch restored, it passes
- tests (`daemon/src/setup.rs` unit): A-F1 `a_shell_spawned_after_an_unwind_is_ended_and_nothing_starts_after_it`;
  A-F4 `forgetting_a_live_run_ends_its_tree`, `a_location_being_deleted_starts_no_run`; A-F3
  `a_record_write_that_fails_still_replaces_the_log`; A-F2 (and B-F1)
  `a_write_issued_before_the_last_one_written_is_dropped`
- red (same run as B-F2's, against facc3d6c): A-F1 FAILED at the `spawned` assert (it returned
  `stop_requested`, still `false` after `interrupt_all`); A-F4 forget FAILED (the forgotten `sleep
  10` exited 0, not by signal); A-F3 FAILED (the log still held `old`: the record's failed save
  skipped the `Replace`). `a_location_being_deleted_starts_no_run` and
  `a_write_issued_before_the_last_one_written_is_dropped` were red by not compiling
  (`begin_delete`, `Begun::persist`, `Persist::seq` did not exist)
- green: `Persist::seq` issued under the setup lock and `SetupStore::in_order` drops a write older
  than the last written or removed for the location (`interrupt` now rewrites the log whole, so a
  dropped stale line write loses nothing); `write` saves record and log independently;
  `interrupt_all` sets `stopping` and `stop_requested`, `spawned` returns `!running ||
  stop_requested`, `start` refuses (`Busy`) while stopping or while a delete holds the location;
  `stop_and_wait` returns a `DeleteHold` (`begin_delete`/`end_delete`) kept to the end of
  `delete_worktree`; `forget` ends a still-running tree -> setup unit 13 passed, `setup_run` 12,
  `mcp_create_worktree_setup` 2

## M5 cycle 1: A16 reducer and sync, A17 Settings › Project, G1 row tag (T040–T048)

- tests: `client/tests/features_setup.rs` (A16, 9), `features_settings.rs` mod
  `project_setup_command` (A17, 7) and the rail count, `setup_tag_geometry.rs` (G1, 3);
  `micold-material` `text_area::rows_set_the_floor_and_the_ceiling`
- red: A16 and A17 first red by not compiling (`features::setup`, `ProjectDraft`,
  `Msg::SetupCommandAction` did not exist). G1 red for a real reason on the first run of the
  finished code: the Default row's title moved from y 206.05 to 205.2 with a plain `Tag` (its role
  line is taller than the second line's text), so the tag now uses `Tag::max_width`, whose label
  takes the second line's height. Mutation red after green (one run, then restored): dedup broken
  (`notified.insert` ignored) -> `a_failed_run_notifies_once_with_the_reason`,
  `a_failure_learned_in_the_catch_up_notifies_once` FAILED; `ProjectDraft::follow` never reloading ->
  `the_field_loads_the_open_projects_command`, `switching_the_open_project_reloads_the_field`,
  `a_refresh_keeps_what_the_user_typed` FAILED; Default row chip dropped ->
  `the_default_row_gets_the_tag_too` FAILED
- green: `features::setup` (`update`, `failure_notice`, `row_tag`, `on_connect`, `on_change`),
  `ProjectDraft` with `follow`/`change`, `SettingsSection::Project`, `ui/settings/project.rs`,
  `TextArea::monospace`/`rows`, `TreeItem::trailing_chip`, `shell/setup.rs` (connect, bind, clone
  and save sync; `SetupChanged` routed) -> features_setup 9, features_settings 155,
  setup_tag_geometry 3, settings_sections 24, settings_rail 6, text_area 15 passed

## M5 cycle 2: the client's architecture guards and the layout fixture

- red (full `cargo test -p micold-client`): `feature_registration_cost` (`settings` named
  `setup`; `shell/setup.rs` named no `features::setup::Msg`; `setup` had `pub enum Msg` and no
  entry point), `feature_write_isolation` (`state.runs` read as the Run in parallel feature's),
  `layout_snapshot` (the Settings rail gained **Project**)
- green: `shell/setup.rs` `update(&mut App, features::setup::Msg) -> Task<Message>` (shape B) and
  `changed` builds the message; the reducer's map is `summaries`; `ProjectDraft::follow` keys the
  map itself; fixture regenerated (`UPDATE_LAYOUT_SNAPSHOT=1`): only the Settings rail's states
  change, one row more -> both guards, layout_snapshot 127, features_setup 9, setup_tag_geometry 3
