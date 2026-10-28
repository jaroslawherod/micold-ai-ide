# Cycle Log: #1554 — Structured view reasoning effort

Append only. Newest last. Every entry's `red` block is the evidence that the test failed without
the implementation it covers.

## M1 — core (T001–T010)

- Method: the tests of T002, T004–T006 and T008 were written first, against stubs (`todo!()` bodies
  for the `effort` functions, `encode_apply_effort` and `parse_apply_effort`; `classify_draft`,
  `parse_initialize`, `transcript/build.rs` and `SavedEvent::of` unchanged, so the new notes were
  dropped and `/effort` read as `Send`). The new types and variants were added first, as the tests
  need them to compile. One red run, then the implementation, then one green run.
- runs: `cargo test -p micold-core --lib -- structured::effort structured::control
  structured::commands structured::saved`; `cargo test -p micold-core --no-fail-fast --test
  structured_parse --test structured_commands --test structured_transcript --test
  protocol_roundtrip --test schema_hash`.

### Cycle 1 — fixtures (T001)

- Recorded against Claude Code 2.1.312 with `CLAUDE_EFFORT` unset: `effort-change.jsonl`,
  `effort-launch.jsonl`; README rows added. No test of its own: cycle 3's parse test reads them.

### Cycle 2 — levels, support, offered and effective levels, `/effort` arguments (T002, T003)

- red: lib, 6 of `structured::effort::tests` failed (`support_is_the_reported_model_the_model_choice_names`,
  `the_offered_levels_are_the_reported_ones_or_all_five`,
  `the_effective_level_is_the_choice_where_the_model_takes_it`,
  `availability_says_why_a_choice_does_not_apply`, `an_effort_argument_is_default_or_an_offered_level`,
  `a_level_the_model_is_not_offered_is_refused_naming_the_offered_ones`: `not yet implemented`).
  `EffortLevel::parse`, its serde check and `EffortChoice::default` were written with the types and
  passed at red (no stub possible without breaking compilation of the rest).
- green: all `structured::effort` tests pass.

### Cycle 3 — reported support, `/effort` in the list, draft classification, control lines (T004–T007)

- red: `structured_commands` 3 failed (`every_reported_model_carries_its_effort_support`,
  `effort_fields_that_are_missing_or_unusable_give_unknown_support_or_are_dropped`,
  `effort_is_the_apps_command_once_with_its_description`); `structured_parse` 2 failed
  (`the_effort_recordings_answer_every_request_and_apply_each_change`,
  `an_apply_effort_error_is_the_clis_refusal`); lib `a_draft_is_sent_unless_it_is_model_or_a_terminal_only_command`
  (`"/effort"` → `Send`), `apply_effort_is_the_contracts_one_line`,
  `an_apply_effort_answer_is_applied_or_the_clis_refusal` failed.
- green: all pass. Daemon and client match arms keep today's behaviour (`Effort(_)` sent as typed
  until M5).

### Cycle 4 — effort notes, markers, saved history (T008, T009)

- red: `structured_transcript` `an_effort_note_is_an_effort_marker_and_a_refusal_carries_the_reason`
  failed (no marker); lib `a_history_holding_each_note_round_trips_unchanged` and
  `a_commands_note_is_written_in_the_new_shape_and_keeps_effort` failed (notes dropped, old shape
  written). `a_version_one_commands_note_without_effort_still_reads` (a byte pin) and the
  `protocol_roundtrip` frame test passed at red: both pin shapes, which the types alone give.
- green: all pass.

### Cycle 5 — protocol version (T010)

- `PROTOCOL_VERSION` 71 → 72; `the_protocol_version_is_72` and `FEATURE_026_PROTOCOL_VERSION`
  re-pinned. Structural; no red.

### Cycle 6 — review A M1 findings

- red: `effort_is_the_apps_command_once_with_its_description` (the entry's hint was the CLI's
  `<low|medium|high|xhigh|max|auto|ultracode [on|off]>`) and
  `effort_fields_that_are_missing_or_unusable_give_unknown_support_or_are_dropped` (a model with
  `supportsEffort: false` and no list read `None`) failed.
- green: `/effort` takes the app's argument hint; `supportsEffort: false` alone gives no effort.

## M2 — daemon: keep the choice, launch with it, apply it to an idle session (T011–T017)

- Method: the types, fields and the wire variant were added with their tests, so the core tests'
  red is a compile failure, except the store's lenient read, which got a real red. The daemon
  tests were written beside the implementation; their red is shown by mutation instead: the
  implementation changed so that the behaviour each covers is missing, the suite run, the change
  undone.
- runs: `cargo test -p micold-core --test session_store --test protocol_roundtrip --test
  schema_hash --test terminal_backend`, `cargo test -p micold-core --lib`; `cargo test -p
  micold-daemon --test structured_effort`.

### Cycle 7 — the choice in core: session, store, wire, launch argv (T011, T012)

- red: `session_store` without the lenient read (`deserialize_with` removed):
  `a_record_without_an_effort_or_with_a_bad_level_loads_default` failed (a stored level the app
  refuses failed the whole record). The rest did not compile before `Session.effort`,
  `LaunchSpec.effort`, `AiCliProvider::effort_args`, `SessionSummary.effort` and
  `ClientMsg::SetSessionEffort` existed.
- green: all pass. `PROTOCOL_VERSION` 72 → 73, `the_protocol_version_is_73` and
  `FEATURE_026_PROTOCOL_VERSION` re-pinned.

### Cycle 8 — the stand-in and the daemon (T013–T016)

- red (mutations of the implementation):
  - `launch_effort` names no level: `a_stored_level_is_launched_with_at_every_start_restart_and_restore`
    and `a_switch_of_kind_launches_both_kinds_with_the_stored_level` failed;
  - reported support ignored at launch and at a pick, and no `EffortChanged` note from the CLI's
    answer: `a_stored_level_the_model_does_not_take_launches_with_none`,
    `a_pick_for_a_model_without_effort_is_stored_and_sent_as_the_default` and
    `an_idle_pick_is_one_line_to_the_same_process_and_every_window_shows_it` failed.
- green: all 13 pass.

### Cycle 9 — docs (T017)

- `docs/daemon.md`: `SetSessionEffort` beside `SetSessionModel`. No test.

## M3 — daemon: a pick during a turn, and a model change (T018–T021)

- runs: `cargo test -p micold-daemon --test structured_effort`, `--test structured_permission`.

### Cycle 10 — a busy pick deferred to the turn's end; refused, put back (T018, T020)

- red: with `spawn_effort_answers` and `follow_effort_answer` as empty stubs (so the tests
  compiled), `a_busy_pick_is_answered_at_once_and_written_at_the_turns_end`,
  `of_two_picks_in_one_turn_only_the_last_is_written`,
  `a_deferred_pick_claude_refuses_is_a_note_and_the_choice_goes_back` and
  `a_refused_deferred_pick_is_not_put_back_over_a_later_pick` failed at their first busy pick:
  `OperationError` (`Busy`), M2's interim refusal. `an_effort_pick_leaves_a_pending_request_alone`
  (structured_permission) passed already: the session is idle while the request is pending.
- green: `StructuredSession::defer_effort`, `deferred_effort`, the turn-end `flush_deferred`
  (model, then effort), `EffortAsk { deferred }` → `EffortRefused`, `EffortPicks.deferred`,
  `follow_effort_answer` over the new `effort_answers` feed. The interim test
  `a_busy_sessions_pick_is_refused_for_now` is gone.

### Cycle 11 — a model change re-applies the level (T019, T020)

- red (same stubs): `a_model_change_reapplies_the_effective_level_without_a_note` ("the null line
  never happened") and `a_model_change_during_a_turn_reapplies_the_level_at_the_turns_end` (no
  `apply_flag_settings` between `set_model` and the next message) failed;
  `with_the_default_choice_a_model_change_writes_no_effort` passed (nothing to write either way).
- green: `follow_model_answer` re-applies for the model the CLI runs (`cli_model`, set by the
  reader on an accepted `set_model`); a busy model pick defers the level for the new model right
  after it (`defer_effort_level`). 22/22.

### Cycle 12 — docs (T021)

- `docs/daemon.md`: a busy pick, `EffortRefused`, the re-apply after a model change. No test.

### Cycle 13 — the effort picker, the marker (T022–T024)

- red: `tests/features_effort_picker.rs` (12 tests), `entries.rs`
  `an_effort_marker_names_the_choice_or_the_refusal` and `shell/composer.rs`
  `a_pick_asks_the_service_once_and_a_refusal_is_said_with_the_choice_unchanged` written first;
  they did not compile (no `features::effort_picker`, no `composer::Msg::Effort*`, no
  `effort_changed_text`, no `PendingOp::SessionEffort`).
- green: `features/effort_picker.rs` (`effort_label`, `availability_note`, `EffortPickerState`
  in `composers.effort`, the `State` methods), the four `Msg::Effort*` and `set_effort` in
  `shell/composer.rs`, `PendingOp::SessionEffort` routed as `SessionModel` in
  `shell/daemon_sync.rs`, `effort_changed_text` beside `model_changed_text` (`shown.rs`, which
  `entries.rs` re-exports; the copy path uses it too). 12/12, 1/1, 1/1.
- draw (GUI exception): `effort_control` on `MenuSelect` beside `model_control`; the marker shares
  the model marker's `change_marker`.

### Cycle 14 — guide (T025)

- `docs/user-guide/worktrees-and-sessions.md` *Commands, the model and effort*. No test.

### Cycle 15 — the window takes the catalog's effort (found driving the real client)

- red: `features_session` `a_snapshot_carries_the_effort_choice_into_the_window` failed
  (`left: Some(Default)`, `right: Some(Level("high"))`): `catalog_sync` copied a summary's
  `model` into the window's session but not its `effort`, so after a pick the marker read high
  and the trigger kept `Effort: Default`.
- green: `catalog_sync` copies `effort` beside `model`, for a known session and a new one.
- also: `session_view_has_keyboard` yields while the effort list is open, as for the model picker
  (`composer_fits` `the_open_effort_picker_keeps_the_keyboard_from_the_pane`); the composer tab
  walk and the layout fixture take the new trigger (`composer_tab`, `layout_snapshot`).

### Cycle 16 — `/effort` in the composer, the daemon backstop (T027–T029)

- red: written first, the client tests (`features_command_palette` `/effort` x5, `shell::composer`
  `sending_slash_effort_asks_the_service_at_once…`) did not compile (no `SendIntent::SetEffort`,
  `classify` took no levels). Re-run after the code against the pre-change behaviour (the
  refusal arm `Effort(_) => Ok(())`, no notice clearing): `structured_effort`
  `slash_effort_sent_as_a_message_is_refused_and_never_reaches_the_cli` and
  `a_queued_message_edited_into_slash_effort_is_refused` failed (`refused: OperationOk … Ack`: the
  message was accepted); `slash_effort_alone_opens_the_picker_and_sends_nothing` failed on the
  refusal notice outliving the opened picker (review V1).
- green: `SendIntent::SetEffort`, `Classified::Effort`/`EffortPicker`, `effort_offered`, the
  `send` arm in `shell/composer.rs`, `UserMessageRefusal::EffortIsTheApps` in `refuse_app_handled`
  (also the queue edit), `Composers::clear_notice` on a sent `/effort` and an opened picker.
- guide: T030 paragraph "`/effort` in the message box". No test.


### Cycle 17 — a refused deferred pick is never asked again (T033–T034, M6)

- red: `structured_effort`
  `a_refused_idle_pick_settled_before_the_deferred_refusal_is_handled_never_asks_it_again` (the
  deferred pick's answer held by the test, as `hold_turn_end` holds a turn's end) failed every
  run: `left: [Some("low"), Some("high"), Some("low")]`. The idle pick's settle re-applied the
  catalog's choice, still the refused deferred `low`, and the CLI's last-sent level was already
  undone by the deferred refusal.
- green: `reapply_effort` holds back (`EffortPicks::reapply_held`) while the deferred pick is
  written to this process (`DeferredEffort::process`, `StructuredSession::effort_pick_deferred`)
  and its answer not handled; `follow_effort_answer` re-applies once the choice is put back.
