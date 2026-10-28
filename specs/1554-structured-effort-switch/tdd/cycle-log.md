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
