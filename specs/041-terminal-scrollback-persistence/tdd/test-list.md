---
feature: 041-terminal-scrollback-persistence
loop: outside-in
profile: .specify/memory/tdd-profile.md
spec_criteria: 30 # US1 1–10, US2 1–8, US3 1–6, US4 1–6
planned_at: 320c0a55
updated_at: 320c0a55
suite_baseline: unknown # not run at planning time: the branch holds only specs/041 documents on top of main; the profile's baseline is green (2629 passed at cdc473ab)
---

# Test List: Terminal History That Survives a Session Service Restart

Traces name spec.md ids: `US<story>-<scenario>` for an acceptance scenario, `FR-…`, `SC-…`, `EC-…`
for an edge case of spec.md. `DM` = data-model.md, `HF` = contracts/saved-history-file.md, `ST` =
contracts/setting.md, `SR` = contracts/stop-request.md, `R#` = research.md. The `test` column names
the tasks.md task that writes the test; the same ids stand on those tasks. All states are `PENDING`:
no test exists yet, and the code is all new (no characterization behaviors are needed). Components
that already exist and change (`state.rs`, `supervisor.rs`, `server.rs`) were not read at planning
time; the loop reads their existing tests before the first behavior that changes them.

## Outer loop: acceptance behaviors

One per acceptance scenario of spec.md, in spec order. The profile's acceptance runner
(`sandbox_real_*`) drives the container runtime and is used only for the sandbox rows (U125–U130). The
outer tests are **daemon integration tests** in `crates/micold-daemon/tests/` that start a real
`DaemonState` with a fake CLI and a second `DaemonState` on the same directories as the "service
restart" (tasks.md, Path Conventions). That is the highest entry point reachable without a display or
a container; it is weaker than a real service process, which only T031 (`SIGTERM`) and T061 do. The
settings control (A11) is a **client integration test** at the render-free reader level, weaker than
a rendered screen; the rendered half is quickstart Part B (T076).

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| A1  | After an orderly stop of the service, a start shows all 200 lines in order with colours and styles | US1-1, FR-001, FR-002, SC-001 | example | PENDING | T017, T031, T061 |
| A2  | After a service restart, a start shows one separator `session restarted at <local date time>` below the saved history and above every new line | US1-2, FR-008, FR-009 | example | PENDING | T017 |
| A3  | A session that printed nothing before the restart shows no separator and no blank history after it | US1-3, FR-010 | example | PENDING | T017 |
| A4  | With one earlier separator in the history, a second restart shows output, separator, output, separator in that order | US1-4, FR-011 | example | PENDING | T017 |
| A5  | A history longer than the scrollback limit restores the most recent lines up to the limit, older lines absent | US1-5, FR-012 | example | PENDING | T017 |
| A6  | Two sessions with different output each show only their own history after a restart | US1-6, FR-025 | example | PENDING | T017 |
| A7  | After the service is dropped without an unwind while a session prints, a restart restores the history up to the last save, at most 60 s old | US1-7, FR-003, SC-002 | example | PENDING | T026 |
| A8  | After the service unwinds for the idle stop, a start shows the whole history above the separator, nothing missing | US1-8, FR-002, SC-001 | example | PENDING | T031 |
| A9  | Stop then start in one service run shows the 200 lines, one separator, the new output, nothing missing | US1-9, FR-015, SC-011 | example | DONE | `crates/micold-daemon/tests/history_restart_in_run.rs::a9_stop_then_start_shows_the_earlier_lines_one_separator_and_the_new_output` |
| A10 | A process that exits by itself and is restarted shows its last lines above one separator and the new output below | US1-10, FR-015, FR-002 | example | DONE | `crates/micold-daemon/tests/history_restart_in_run.rs::a10_a_process_that_exits_by_itself_is_restarted_below_its_last_lines` |
| A11 | The Terminal section of Settings holds the save-history control, on by default, with the sentence about disk and deletion | US2-1, FR-026, FR-031 | example | PENDING | T041 |
| A12 | With the setting off, output and a service restart leave no file and the terminal starts empty with no separator | US2-2, FR-028 | example | PENDING | T040 |
| A13 | Turning the setting off while a session prints stops every later write, with no restart | US2-3, FR-027 | example | PENDING | T040 |
| A14 | Turning the setting on saves the running session's history, including output printed while off, within 60 s | US2-4, FR-027 | example | PENDING | T040 |
| A15 | Turning the setting off deletes the files of running and stopped sessions before `SettingsSet` is answered, within 5 s | US2-5, FR-027, SC-008 | example | PENDING | T040 |
| A16 | After turning the setting off, a running terminal's history is unchanged; only the disk copy is gone | US2-6, FR-033 | example | PENDING | T040 |
| A17 | Off, then on, then a service restart shows only what was saved after it was turned on; earlier files do not return | US2-7, FR-033 | example | PENDING | T040 |
| A18 | With the setting off, a stop and start in one run still shows the earlier output above the separator and writes nothing | US2-8, FR-015 | example | PENDING | T040 |
| A19 | A saved history of random bytes: the session starts and runs exactly as one with no saved history | US3-1, FR-016, SC-006 | example | PENDING | T051, T017 |
| A20 | For that session the terminal holds the notice line and none of the file's bytes, and the log holds one warning naming session and reason, also in `RecentErrors` | US3-2, FR-017 | example | PENDING | T051 |
| A21 | A history file with mode `000` gives the same outcome as a damaged one | US3-3, FR-016, FR-017 | example | PENDING | T051 |
| A22 | With one damaged and one intact file, the second session shows its full history | US3-4, FR-018 | example | PENDING | T051 |
| A23 | After a skip, the new output is saved at the next tick and a later restart restores the notice and the new output | US3-5, FR-018 | example | PENDING | T051 |
| A24 | A failing save keeps the session running, logs one warning, and is tried again 30 s later | US3-6, FR-007 | example | PENDING | T026, T051 |
| A25 | Remove (`delete_session`) deletes the saved history before the reply | US4-1, FR-023, SC-007 | example | PENDING | T057 |
| A26 | Close (archive) deletes the saved history before the session disappears | US4-2, FR-023, SC-007 | example | PENDING | T057 |
| A27 | Deleting a worktree or forgetting a project deletes the saved history of each of its sessions | US4-3, FR-023 | example | PENDING | T057 |
| A28 | A file of an unknown id and one of an archived session are gone after a service start | US4-4, FR-024 | example | PENDING | T057 |
| A29 | A removal during a save leaves no saved history on disk | US4-5, FR-023 | example | PENDING | T057 |
| A30 | Stopping a session keeps its file, and the history is restored in the same run and after a restart | US4-6, FR-015 | example | PENDING | T057 |

## Inner loop: unit behaviors

Grouped by the component from `plan.md` and tasks.md that owns them. Each line is one observable
result. No property-based library is in the profile, so invariants (round trip, never wider than
`columns`, never a panic) are `example` tests sampled at the boundaries named in the line.

### `crates/micold-core/src/terminal_history/mod.rs` (T002, T003)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U1 | `validate` accepts a line whose runs' `chars` sum to its character count | DM §1, FR-001 | example | DONE | `crates/micold-core/tests/terminal_history_snapshot.rs::validate_accepts_a_line_whose_runs_sum_to_its_character_count` |
| U2 | `validate` rejects a line whose run sum is one more, and one less, than its character count | DM §1 | example | DONE | `crates/micold-core/tests/terminal_history_snapshot.rs::validate_rejects_a_line_whose_run_sum_is_one_more_or_one_less_than_its_character_count` |
| U3 | `validate` rejects a `text` holding a C0 character (`\u{7}`), a C1 character (`\u{9b}`) and `ESC` | DM §1, FR-016 | example | DONE | `crates/micold-core/tests/terminal_history_snapshot.rs::validate_rejects_a_line_holding_a_c0_a_c1_or_an_escape_character` |
| U4 | `HistoryColor::Basic` accepts 0 and 15 and rejects 16 | DM §1, FR-001 | example | DONE | `crates/micold-core/tests/terminal_history_snapshot.rs::basic_color_accepts_0_and_15_and_rejects_16` |
| U5 | `HistoryColor::Dim` accepts 0 and 7 and rejects 8 | DM §1, FR-001 | example | DONE | `crates/micold-core/tests/terminal_history_snapshot.rs::dim_color_accepts_0_and_7_and_rejects_8` |
| U6 | An empty snapshot is `is_empty()`; one with a line is not | DM §1, FR-010 | example | DONE | `crates/micold-core/tests/terminal_history_snapshot.rs::an_empty_snapshot_is_empty_and_one_with_a_line_is_not` |
| U7 | `StyleFlags` round-trips each of bold, dim, italic, underline, inverse, strikethrough, hidden | DM §1, FR-001 | example | DONE | `crates/micold-core/tests/terminal_history_snapshot.rs::style_flags_round_trip_each_attribute` |

### `crates/micold-core/src/terminal_history/text.rs` (T004, T008, T049, T052)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U8 | At 80 columns `separator_line` is `── session restarted at 2026-10-02 14:31 +02:00 ──` | DM §7, FR-009, EC-Clock | example | DONE | `crates/micold-core/tests/terminal_history_text.rs::at_80_columns_the_separator_is_the_full_text` |
| U9 | At a width narrower than the full text the rules are dropped | DM §7, FR-009 | example | DONE | `crates/micold-core/tests/terminal_history_text.rs::narrower_than_the_full_text_the_rules_are_dropped` |
| U10 | At a width narrower than the text without rules the text is cut to the width | DM §7, FR-009 | example | DONE | `crates/micold-core/tests/terminal_history_text.rs::narrower_than_the_text_without_rules_the_text_is_cut_to_the_width` |
| U11 | The separator is never wider than `columns` and never holds a line break, at widths 1, 2, the text width, and one below it | DM §7, FR-009 | example | DONE | `crates/micold-core/tests/terminal_history_text.rs::the_separator_is_never_wider_than_the_columns_and_never_breaks_the_line` |
| U12 | At 80 columns `notice_line` is `── earlier output could not be restored ──` | DM §7, FR-017 | example | PENDING | T049 |
| U13 | `notice_line` drops the rules, then cuts, when narrower, and is one row | DM §7, FR-017 | example | PENDING | T049 |

### `crates/micold-daemon/src/history.rs`: capture (T005, T009)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U14 | `capture` returns the text and order of history rows then screen rows | DM §1, FR-001, SC-001 | example | DONE | `crates/micold-daemon/src/history.rs::tests::capture_returns_the_history_rows_then_the_screen_rows_in_order` |
| U15 | Each of the 16 basic colours is captured as foreground and as background | DM §1, FR-001 | example | DONE | `crates/micold-daemon/src/history.rs::tests::each_of_the_16_basic_colours_is_captured_as_foreground_and_as_background` |
| U16 | An indexed colour and an RGB colour are captured as foreground and as background | DM §1, FR-001 | example | DONE | `crates/micold-daemon/src/history.rs::tests::an_indexed_and_an_rgb_colour_are_captured_as_foreground_and_as_background` |
| U17 | Each of bold, dim, italic, underline, inverse, strikethrough is captured | DM §1, FR-001 | example | DONE | `crates/micold-daemon/src/history.rs::tests::each_style_flag_is_captured_and_every_underline_kind_is_underline` |
| U18 | Two rows joined by the wrap flag are one `LogicalLine` | DM §1, FR-001 | example | DONE | `crates/micold-daemon/src/history.rs::tests::two_rows_joined_by_the_wrap_flag_are_one_logical_line` |
| U19 | A wide character counts as one character and its spacer is skipped | DM §1, FR-001 | example | DONE | `crates/micold-daemon/src/history.rs::tests::a_wide_character_counts_as_one_character_and_its_spacer_is_skipped` |
| U20 | A zero-width character follows its base character | DM §1, FR-001 | example | DONE | `crates/micold-daemon/src/history.rs::tests::a_zero_width_character_follows_its_base_character` |
| U21 | Trailing empty screen rows are not captured | DM §1, R2 | example | DONE | `crates/micold-daemon/src/history.rs::tests::trailing_empty_screen_rows_are_not_captured` |
| U22 | A `Term` that printed nothing gives an empty snapshot | DM §1, FR-010 | example | DONE | `crates/micold-daemon/src/history.rs::tests::a_term_that_printed_nothing_gives_an_empty_snapshot` |

### `crates/micold-daemon/src/history.rs`: seed (T006, T010, T050, T053)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U23 | Capture after `seed(Seed::History)` equals the input lines followed by the separator in the dim style | DM §6, FR-009, FR-011 | example | DONE | `crates/micold-daemon/src/history.rs::tests::capture_after_a_seed_is_the_input_lines_then_the_separator_in_the_dim_style` |
| U24 | After seeding the screen is blank, the cursor is at home, attributes are reset and the seeded lines are all in the history | DM §6, R17, FR-008 | example | DONE | `crates/micold-daemon/src/history.rs::tests::after_a_seed_the_screen_is_blank_at_home_with_attributes_reset_and_the_lines_in_history` |
| U25 | A snapshot longer than the `Term`'s history limit leaves the most recent lines, and one exactly at the limit leaves all | FR-012 | example | DONE | `crates/micold-daemon/src/history.rs::tests::a_snapshot_longer_than_the_history_limit_leaves_the_most_recent_lines` |
| U26 | Seeding at a narrower width wraps, and a later capture gives the same logical lines | EC-Terminal size | example | DONE | `crates/micold-daemon/src/history.rs::tests::seeding_at_a_narrower_width_wraps_and_a_later_capture_gives_the_same_logical_lines` |
| U27 | `Seed::None` leaves the `Term` untouched | FR-010 | example | DONE | `crates/micold-daemon/src/history.rs::tests::seed_none_leaves_the_term_untouched` |
| U28 | A second seed after more output keeps the first separator | FR-011 | example | DONE | `crates/micold-daemon/src/history.rs::tests::a_second_seed_after_more_output_keeps_the_first_separator` |
| U29 | `Seed::Notice` leaves exactly one line, the notice in the dim style, no separator, the cursor on the row below | DM §6, FR-017 | example | PENDING | T050 |

### In-run restart through `DaemonState` (T007, T011, T012)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U30 | A session with no output shows no separator after stop and start | FR-010, EC-No history | example | DONE | `crates/micold-daemon/tests/history_restart_in_run.rs::u30_a_session_with_no_output_shows_no_separator_after_stop_and_start` |
| U31 | Two stops and starts show two separators in order | FR-011 | example | DONE | `crates/micold-daemon/tests/history_restart_in_run.rs::u31_two_stops_and_starts_show_two_separators_in_order` |
| U32 | Two sessions each show only their own lines after stop and start | FR-025 | example | DONE | `crates/micold-daemon/tests/history_restart_in_run.rs::u32_two_sessions_each_show_only_their_own_lines_after_stop_and_start` |
| U33 | The fake CLI's recorded stdin is empty after a start: the separator is not sent as input | FR-009 | example | DONE | `crates/micold-daemon/tests/history_restart_in_run.rs::u33_the_cli_receives_nothing_on_stdin_at_a_start` |
| U34 | A Regular Terminal instance stopped and started has an empty history | FR-014 | example | DONE | `crates/micold-daemon/tests/history_restart_in_run.rs::u34_a_regular_terminal_stopped_and_started_has_an_empty_history` |
| U35 | A fake CLI that prints `ESC[2J ESC[H` at start leaves the seeded lines and separator in the history | R13, EC-Full-screen | example | DONE | `crates/micold-daemon/tests/history_restart_in_run.rs::u35_a_cli_that_erases_the_screen_at_start_leaves_the_restored_lines` |
| U36 | A fake CLI that enters and leaves the alternate screen leaves them in the primary grid's history | R16, EC-Full-screen | example | DONE | `crates/micold-daemon/tests/history_restart_in_run.rs::u36_a_cli_that_uses_the_alternate_screen_leaves_the_restored_lines_in_the_primary_history` |
| U37 | A second attached client receives the same lines in its first `full` frame | EC-Several windows | example | DONE | `crates/micold-daemon/tests/history_restart_in_run.rs::u37_a_second_window_gets_the_same_lines_in_its_first_full_frame` |
| U38 | The stop-start and self-exit-restart cases pass on a real pseudoconsole under `cfg(windows)` | FR-030, R17 | example | PENDING | T007 (`crates/micold-daemon/tests/history_restart_in_run.rs`: A9, A10, U133, U134 are not `cfg`-gated; red/green on the CI Windows leg) |
| U133 | With a client attached and streaming, a stop keeps the last line the process printed | R4, FR-015 | example | DONE | `crates/micold-daemon/tests/history_restart_in_run.rs::u133_with_a_window_streaming_a_stop_keeps_the_last_line` |
| U134 | With a client attached and streaming, a self-exit and restart keeps the last line | R4, story 1 scenario 10 | example | DONE | `crates/micold-daemon/tests/history_restart_in_run.rs::u134_with_a_window_streaming_a_self_exit_and_restart_keeps_the_last_line` |
| U135 | A fake CLI that leaves a detached grandchild holding the terminal open is stopped with a reply within 3 s and its parsed output is carried (`cfg(unix)`) | R4, FR-005 | example | DONE | `crates/micold-daemon/tests/history_restart_in_run.rs::u135_a_detached_grandchild_does_not_hold_the_stop_and_the_output_is_carried` |

### `crates/micold-core/src/terminal_history/format.rs` (T014, T019)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U39 | Encode then decode returns the same snapshot, for empty, one line, 10,000 lines, every colour kind and every flag | HF §2, FR-001 | example | DONE | `crates/micold-core/tests/terminal_history_format.rs::encode_then_decode_gives_the_same_snapshot` |
| U40 | The encoded header bytes are those of HF §2 | HF §2 | example | DONE | `crates/micold-core/tests/terminal_history_format.rs::the_encoded_bytes_are_the_header_the_payload_and_the_checksum_of_both` |
| U41 | Each row of HF §4's table gives its `DamageReason`: too large, wrong magic, under 52 bytes, version 2, truncated tail, one flipped payload bit, trailing payload bytes, style index out of range, run sum differs, text with `ESC` | HF §4, FR-016 | example | DONE | `crates/micold-core/tests/terminal_history_format.rs::a_file_over_the_size_cap_is_too_large` and the 11 tests after it` |
| U42 | 1,000 random byte strings and every prefix of a valid file decode to `Damaged` without a panic | HF §4, FR-016, SC-006 | example | DONE | `crates/micold-core/tests/terminal_history_format.rs::every_prefix_of_a_valid_file_is_damaged`, `::random_byte_strings_are_damaged_without_a_panic` |
| U43 | The bytes of `fixtures/terminal_history/v1.history` equal the encoding of a fixed snapshot built in the test | HF §5, FR-022 | example | DONE | `crates/micold-core/tests/terminal_history_format.rs::the_v1_fixture_is_the_encoding_of_its_snapshot` |

### `crates/micold-core/src/terminal_history/store.rs`: save and load (T015, T021)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U44 | `save` then `load` gives `History`, in the file `<dir>/<session uuid>.history` | DM §5, HF §1, FR-001 | example | DONE | `crates/micold-core/tests/terminal_history_store.rs::a_saved_snapshot_loads_from_the_file_named_after_the_session`, `…::another_store_on_the_same_directory_loads_what_the_first_saved`, `…::an_empty_snapshot_loads_as_an_empty_history` |
| U45 | `load` of an absent file gives `None` | DM §5, FR-010 | example | DONE | `crates/micold-core/tests/terminal_history_store.rs::a_session_with_no_file_loads_as_none` |
| U46 | `load` of a damaged file gives `Damaged` | DM §5, FR-016 | example | DONE | `crates/micold-core/tests/terminal_history_store.rs::a_damaged_file_loads_as_damaged_with_its_reason`, `…::a_file_over_the_size_cap_loads_as_too_large`, `…::a_save_replaces_a_damaged_file` |
| U47 | `load` of a file with mode `000` gives `Damaged(Unreadable)` (`cfg(unix)`) | US3-3, FR-016 | example | DONE | `crates/micold-core/tests/terminal_history_store.rs::unix::a_file_that_cannot_be_opened_loads_as_unreadable` |
| U48 | A save over an existing file leaves no temporary file | HF §3, FR-006 | example | DONE | `crates/micold-core/tests/terminal_history_store.rs::a_save_over_an_existing_file_leaves_no_temporary_file` |
| U49 | A temporary file left behind before the rename leaves the previous file loadable | FR-006, EC-Killed mid-save | example | DONE | `crates/micold-core/tests/terminal_history_store.rs::a_temporary_file_left_behind_leaves_the_previous_file_loadable` |
| U50 | Two ids make two files and never each other's content | FR-025 | example | DONE | `crates/micold-core/tests/terminal_history_store.rs::two_sessions_have_two_files_and_never_each_others_content` |
| U51 | A second `save` of an equal snapshot returns `Unchanged`, and the file's modification time and inode stay | FR-004 | example | DONE | `crates/micold-core/tests/terminal_history_store.rs::a_second_save_of_an_equal_snapshot_is_unchanged`, `…::unix::an_unchanged_save_leaves_the_file_as_it_is`, `…::the_first_save_of_a_store_writes_even_over_an_equal_file` |
| U52 | With `create_dir = false` and no directory `save` returns `Skipped` and creates nothing; with the directory present it saves | R15, FR-021 | example | DONE | `crates/micold-core/tests/terminal_history_store.rs::a_store_that_does_not_create_its_directory_skips_until_it_exists` |
| U53 | The directory is mode `0700` and the file `0600` (`cfg(unix)`) | FR-020, SC-009 | example | DONE | `crates/micold-core/tests/terminal_history_store.rs::unix::the_directory_is_0700_and_the_file_0600` |
| U54 | `history_dir()` ends in `terminal-history` under `data_local_dir()`, and on Windows is not under `data_dir()` (`cfg(windows)`) | FR-019 | example | DONE | `crates/micold-core/tests/terminal_history_store.rs::unix::the_history_directory_is_terminal_history_under_the_local_data_directory`, `…::on_windows_the_history_directory_is_local_and_not_in_the_roaming_profile` |

### `crates/micold-core/src/owner_only.rs` (T016, T020)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U55 | `write` creates the directory `0700` and the file `0600`, and replaces an existing file through a rename (`cfg(unix)`) | R8, FR-020 | example | DONE | `crates/micold-core/tests/owner_only.rs::unix::write_creates_the_directory_0700_and_the_file_0600`, `…::unix::write_replaces_an_existing_file_through_a_rename`, `…::unix::write_narrows_an_existing_wider_file_and_directory`, `…::write_with_stores_what_the_fill_wrote_and_returns_the_path` |
| U56 | `ensure_dir` creates a missing directory `0700` and tightens an existing looser one (`cfg(unix)`) | R8, FR-020 | example | DONE | `crates/micold-core/tests/owner_only.rs::unix::ensure_dir_creates_a_missing_directory_0700`, `…::unix::ensure_dir_tightens_an_existing_looser_directory` |
| U57 | On Windows the directory and file carry a protected DACL with exactly one entry, for the current user (`cfg(windows)`) | R8, FR-020, FR-030, SC-009 | example | DONE | `crates/micold-core/tests/owner_only.rs::windows::the_written_file_and_its_directory_have_a_protected_dacl_for_the_current_user_only`, `…::windows::rewriting_replaces_the_bytes_and_keeps_the_owner_only_dacl`, `…::windows::ensure_dir_gives_a_missing_and_an_existing_directory_the_owner_only_dacl` |
| U58 | The temporary file is owner-only before any content is written | FR-020 | example | DONE | `crates/micold-core/tests/owner_only.rs::unix::the_temporary_file_is_0600_before_any_content_is_written`, `…::windows::the_temporary_file_has_the_owner_only_dacl_before_any_content_is_written` |
| U59 | `write` into a read-only directory returns the error and leaves no temporary file | FR-007, FR-006 | example | DONE | `crates/micold-core/tests/owner_only.rs::unix::write_into_a_read_only_directory_returns_the_error_and_leaves_no_temporary_file`, `…::a_fill_that_fails_returns_its_error_and_leaves_no_temporary_file`, `…::a_rename_that_fails_returns_the_error_and_leaves_no_temporary_file` |

### Restore after a service restart (T017, T018, T022, T023)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U60 | With a smaller scrollback limit in force at the restore, only the most recent lines up to the new limit are shown | FR-012, EC-Limit changed | example | PENDING | T017 |
| U61 | A process that exits by itself is saved at that exit and restored after a restart | FR-002 | example | PENDING | T017 |
| U62 | A Regular Terminal instance has no file after a stop | FR-014 | example | PENDING | T017 |
| U63 | A file is not read while a carried snapshot exists | DM §6, R4 | example | PENDING | T017 |
| U64 | A saved history of 10,000 lines of 100 characters is loaded and seeded and the session is running no more than 1 s later than with no file | FR-013, SC-004 | example | PENDING | T018 |

### `crates/micold-core/src/terminal_history/schedule.rs` (T025, T027)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U65 | `new(count)` is not due while the output count is unchanged | DM §4, FR-004, SC-003 | example | PENDING | T025 |
| U66 | It is due when the count moved and no save was tried | DM §4, FR-003 | example | PENDING | T025 |
| U67 | It is not due 29.999 s after `saved` with a moved count | DM §4, FR-003 | example | PENDING | T025 |
| U68 | It is due at exactly 30 s after `saved` with a moved count | DM §4, FR-003 | example | PENDING | T025 |
| U69 | Ticking every 5 s for 600 s with a count that always moves gives 20 saves, never more than 21 | DM §4, SC-003 | example | PENDING | T025 |
| U70 | `failed(now)` makes it due again 30 s later with the same count | DM §4, FR-007 | example | PENDING | T025 |
| U71 | `mark_due()` makes it due with an unchanged count, still spaced 30 s from the last attempt | DM §4, FR-027 | example | PENDING | T025 |

### `crates/micold-daemon/src/history.rs`: saver (T026, T028, T029)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U72 | A printing terminal's file changes once per 30 s and holds the output printed before the tick | FR-003 | example | PENDING | T026 |
| U73 | Output is on disk no later than 60 s after it was printed | FR-003, SC-002 | example | PENDING | T026 |
| U74 | An idle terminal's file is not rewritten: modification time and a write counter stay | FR-004, SC-003 | example | PENDING | T026 |
| U75 | Two printing sessions are saved each on its own schedule and neither file holds the other's lines | EC-Several busy sessions, FR-025 | example | PENDING | T026 |
| U76 | A Regular Terminal instance is never saved | FR-014 | example | PENDING | T026 |
| U77 | A save into a read-only directory leaves the session running, logs a warning and is tried again 30 s later | FR-007 | example | PENDING | T026 |
| U78 | Input written and a resize sent during a save reach the fake CLI | FR-005 | example | PENDING | T026 |

### Orderly stop: `server.rs`, `history.rs::save_all_live`, `platform/unix.rs` (T031, T032, T033–T035)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U79 | `unwind` with the idle reason saves every running covered terminal, and a restart restores all 200 lines | FR-002, SC-001 | example | PENDING | T031 |
| U80 | A real service process sent `SIGTERM`, `SIGINT` or `SIGHUP` exits within 5 s and its file holds the last line printed (`cfg(unix)`) | SR §6, FR-002 | example | PENDING | T031 |
| U81 | A save that blocks holds `unwind` no longer than 3 s and the previous file stays | SR §4, FR-006 | example | PENDING | T031 |
| U82 | With a store that refuses to save (`create_dir = false` and no directory), `unwind` writes nothing | FR-028 | example | PENDING | T031 |
| U83 | A terminal with no output since its last save is not rewritten by `unwind` | FR-004 | example | PENDING | T031 |
| U84 | The endpoint is released only after the saves: a second service started during the unwind loads the complete file | SR §4, FR-002 | example | PENDING | T031 |
| U132 | Ten running sessions each holding 10,000 lines of 100 characters are all saved by one `unwind` within its 3 s bound | SR §4, FR-002, SC-001 | example | PENDING | T031 |
| U85 | `stop_requested()` is pending until the process receives `SIGTERM`, then completes; a second signal changes nothing | SR §1 | example | PENDING | T032 |

### `crates/micold-core/src/settings.rs` and the protocol (T037, T038, T042, T043)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U86 | `save_terminal_history` defaults to `true` | ST §1, FR-026 | example | PENDING | T037 |
| U87 | A `settings.json` without the field reads as `true` | ST §1, FR-029 | example | PENDING | T037 |
| U88 | `false` round-trips through the settings file | ST §1, FR-029 | example | PENDING | T037 |
| U89 | `DaemonSettings.save_terminal_history` and `SettingsSet { save_terminal_history: Some(false) }` round-trip | ST §2, FR-029 | example | PENDING | T038 |
| U90 | `SettingsSet` with `None` leaves the value unchanged | ST §2 | example | PENDING | T038 |
| U91 | The pinned schema hash and `PROTOCOL_VERSION` are the new ones | ST §2 | example | PENDING | T038 |

### `crates/micold-core/src/terminal_history/store.rs`: setting and deletion (T039, T044)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U92 | With `enabled = false`, `save` returns `Skipped` and `load` returns `None` | DM §5, FR-028 | example | PENDING | T039 |
| U93 | `set_enabled(false)` deletes every `.history` and temporary file and reports none left | DM §5, FR-027, SC-008 | example | PENDING | T039 |
| U94 | A file that cannot be deleted is reported, kept in the retry set, and removed by `retry_deletions()` once deletable (`cfg(unix)`) | FR-033 | example | PENDING | T039 |
| U95 | `purge()` deletes the same set at service start | FR-033 | example | PENDING | T039 |
| U96 | `set_enabled(true)` restores nothing; a failed deletion stays in the retry set and `load` of its id returns `None` until a new `save` replaced the file | US2-7, FR-033 | example | PENDING | T039 |
| U97 | A `save` racing `set_enabled(false)` from another thread leaves no file | FR-033 | example | PENDING | T039 |

### Setting end to end and client (T040, T041, T045–T047)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U98 | A service that starts with the setting off deletes every file before a session can start | FR-033, EC-Setting off while not running | example | PENDING | T040 |
| U99 | A failed deletion is logged once as a warning with session and reason and retried every 30 s | FR-033, EC-Deletion failed | example | PENDING | T040 |
| U100 | `SettingsChanged` is broadcast with the new value | FR-027, ST §2 | example | PENDING | T040 |
| U101 | The client draft holds `save_terminal_history` from the service's settings | ST §3, FR-029 | example | PENDING | T041 |
| U102 | Toggling the control marks the Terminal section dirty | ST §3, FR-031 | example | PENDING | T041 |
| U103 | Save sends `SettingsSet` with `Some(value)` and no other field changed | ST §3, FR-027 | example | PENDING | T041 |
| U104 | A `SettingsChanged` from the service updates the draft | ST §3 | example | PENDING | T041 |
| U105 | The Terminal section's `SETTINGS` list holds the entry with the label and note of ST §4 | ST §4, FR-026, FR-031 | example | PENDING | T041 |

### Damaged history in the daemon (T051, T054)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U106 | A file with another format version gives the same outcome, with the reason "written by another version" | EC-Newer version, FR-017 | example | PENDING | T051 |
| U107 | A save that keeps failing for the same reason is logged once per service run, and a different reason is logged again | FR-007 | example | PENDING | T051 |

### Removal: `store.rs` and `state.rs` (T056, T057, T058, T059)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U108 | `forget(ids)` deletes those files and no other | DM §5, FR-023 | example | PENDING | T056 |
| U109 | A `save` for a forgotten id returns `Skipped` and leaves no file, also when it began before the `forget` on another thread | US4-5, FR-023 | example | PENDING | T056 |
| U110 | `forget` deletes while `enabled` is false | EC-Removed while off, FR-023 | example | PENDING | T056 |
| U111 | `sweep(keep)` deletes every `.history` file whose id is not in `keep`, every file with another name and every temporary file, and keeps the rest | FR-024 | example | PENDING | T056 |
| U112 | Pruning a never-used session deletes its file and its carried snapshot | FR-023 | example | PENDING | T057 |
| U113 | The carried snapshot of a removed session is dropped | FR-023, FR-025 | example | PENDING | T057 |
| U114 | Removal with the setting off deletes a file left by a failed deletion | EC-Removed while off, FR-023 | example | PENDING | T057 |

### Windows stop request (T061, T062, T063–T065)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U115 | Setting the event `Local\Micold.Daemon.Stop.<SID>` makes a real service exit within 5 s with its file holding the last line (`cfg(windows)`) | SR §6, FR-002, FR-030 | example | PENDING | T061 |
| U116 | `WM_ENDSESSION` sent to the hidden window raises the same request | SR §3, FR-030 | example | PENDING | T061 |
| U117 | The event's DACL has one entry, for the current user | SR §1, FR-020 | example | PENDING | T061 |
| U118 | `stop_running_daemon` against the real service makes it exit with code 0, not the 1 of `TerminateProcess`, its file holding the last line | SR §2 | example | PENDING | T061 |
| U119 | Against a process that ignores the event, `terminate_daemon` falls back to `TerminateProcess` after 5 s | SR §2 | example | PENDING | T062 |
| U120 | With no event to open, `terminate_daemon` falls back at once | SR §3 | example | PENDING | T062 |
| U136 | In the installer's `StopDaemon`, the step that sets the stop event comes before `Stop-Process` and `taskkill` | SR §2 | example | PENDING | T062 |

### Sandbox (T067, T068, T069, T070–T072)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U121 | `MountSet::build` adds the history mount only when the history directory is not inside the state directory, and none otherwise | DM §9, FR-021 | example | PENDING | T067 |
| U122 | The container arguments carry `-e TZ=<zone>` when a zone is given and no `TZ` when none is | DM §9, FR-009 | example | PENDING | T067 |
| U123 | At bring-up, attach included, the host history directory is created through `owner_only::ensure_dir` before the runtime is called | FR-021, FR-020 | example | PENDING | T068 |
| U124 | The zone passed to the container is the host's IANA zone | FR-009 | example | PENDING | T068 |
| U125 | A history saved by a host service is restored by a container service on the same directory, and the reverse | FR-022 | example | PENDING | T069 |
| U126 | A file written by the container is `0600` in a `0700` directory as seen from the host | FR-021, SC-009 | example | PENDING | T069 |
| U127 | After the container is recreated the history is restored | FR-021 | example | PENDING | T069 |
| U128 | `<runtime> stop` on a sandbox with a printing session leaves a file holding the last line | SR §6, FR-002 | example | PENDING | T069 |
| U129 | The separator carries the host's UTC offset | FR-009, FR-021 | example | PENDING | T069 |
| U130 | A container without the directory saves nothing and logs one warning saying to recreate the sandbox | R15, FR-007 | example | PENDING | T069 |

### Echo delay (T074)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U131 | With ten printing sessions, the 95th percentile of keystroke-to-echo with saving on is at most 20 ms above that with saving off | FR-005, SC-005 | example | PENDING | T074 |

## Invariants and edge cases still to place

- Output printed by a CLI that draws full-screen: only the last screen is saved (EC-Full-screen). U35 and U36 pin the seeding side; the saving side rests on U14 and U21 and is observable only in the visual pass (T076).
- A session that resumes a conversation keeps the saved history above the separator unchanged (EC-Resumes a conversation): no test task names it; it follows from U23 and U35, and is checked in the visual pass.

## Out of scope

- Restoring the running processes across a service restart: spec.md out of scope.
- Searching, exporting or viewing saved history outside the terminal: spec.md out of scope.
- Encrypting or redacting saved history: spec.md out of scope.
- Copying saved history to another computer or an online service: spec.md out of scope.
- A per-project or per-session choice, a configurable save interval: spec.md out of scope.
- Keeping the history of a closed or removed session, or hidden on disk while the setting is off: spec.md out of scope.
- Regular Terminal instances (saving, restoring, bringing them back): spec.md out of scope; only the negative behaviors U34, U62 and U76 are listed.
- Starting an AI CLI in a scrolling mode, and showing restored history over a full-screen view: spec.md out of scope.
- Documentation wording (FR-032): the user-guide tasks T013, T024, T030, T036, T048, T055, T060, T066, T073 and T075 are checked by CI's user-guide gate, not by a test on this list.
- SC-010 (the user can tell old from new output from the terminal alone): a judgement of a rendered screen, recorded in quickstart Part B by T076; the separator text and distinction are pinned by U8 and U23.

## Verification commands

Copied verbatim from `.specify/memory/tdd-profile.md` at planning time:

- Single test: `scripts/build-lock.sh cargo test --test {file} {name} -- --exact`
- File: `scripts/build-lock.sh cargo test --test {file}`
- Full suite: `scripts/build-lock.sh cargo test --workspace` (`mise run test`; the merge gate is `mise run gate`)
- Fast subset: `scripts/build-lock.sh cargo test -p micold-core --all-targets` (`mise run test-core`)
- Acceptance (sandbox only): `scripts/build-lock.sh cargo test -p micold-core --features sandbox-real-runtime sandbox_real_ -- --test-threads=1` (this feature's own is `sandbox_real_history`, run by `mise run image && mise run test-sandbox`)
- Coverage, mutation, property: none in the profile
