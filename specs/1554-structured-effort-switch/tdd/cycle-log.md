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
