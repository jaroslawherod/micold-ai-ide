# TDD cycle log — 939 structured transcript view

No `tdd/test-list.md` was planned for this feature (the tasks unit did not run `speckit.tdd.plan`);
the loop follows tasks.md's test-first order instead: each test task written before its
implementation task, run red, then made green.

## M1 (T001–T015) — transcript model for finished events

- **Red**: T002's shared `tests/support/stream_json.rs` in place, `structured_parse` 7/7 green.
  With T004, T006, T007, T010, T011, T012 written against stubbed `figures`, `summary`, `result` and
  `build::push` (`todo!()`): `cargo test -p micold-core --lib structured::transcript` 0 passed,
  14 failed; `cargo test -p micold-core --test structured_transcript` 0 passed, 25 failed (each on
  its stub's `todo!`).
- **Green**: T005, T008, T009, T013, T014, T015 implemented. One test failed first
  (`a_history_message_continuing_the_history_in_progress_appends`: a fresh transcript took a
  continuing first message without its `omitted`); fixed by treating a transcript holding nothing
  as not in progress. Lib 14/14, `structured_transcript` 25/25, `structured_parse` 7/7.
- **Refactor**: none needed beyond `cargo fmt`; clippy `-D warnings` clean on `micold-core`.
- **Notes**: cycles were batched per milestone, not one behavior per commit.

## M2 (T016–T019) — streaming, supersede, bound and re-sync

- **Red**: T016/T017 written first, with synthetic stream-event helpers added to
  `tests/support/stream_json.rs` and a `Transcript::held()` accessor; `cargo test -p micold-core
  --test structured_transcript` 28 passed, 8 failed (streaming growth, supersede, raw stream rows,
  G1 over the recorded fixtures, the bound). The G2 replay, the sub-agent-delta and G3 rebuild
  tests passed already: with deltas absorbed, M1 built the same entries.
- **Green**: T018 (`stream.rs` `OpenStream`, supersede in `build.rs`) and T019 (`Transcript::bound`
  after each pushed event) implemented; `structured_transcript` 36/36.
- **Refactor**: the streaming-flag match shared by `grow` and `close` (`is_streaming`,
  `streaming_flag`); `cargo fmt`.

## M3 (T020–T023) — the window's transcript state

- **Red**: T020/T021 written first in `tests/features_structured_transcript.rs`, against
  `features/structured_transcript.rs` with every `State` operation and `Follow::scrolled` stubbed
  (`todo!()`): 1 passed (the scroll id), 16 failed, each on its stub.
- **Green**: T022 (the feature: fold set, `Follow`, `END_SLACK`, `FollowChange`, `Scroll`,
  `TranscriptMsg`) and T023 (`app::State::structured_events` / `forget_structured` /
  `update_transcript`, `session::Msg::Transcript`, `shell/transcript.rs`, `ui::transcript::
  transcript_scroll_id`); `features_structured_transcript` 17/17. The binary-side half of T021
  (the wire's `StructuredEvents`, the scroll tasks' presence via `Task::units`, the close path) is
  in `src/main_tests.rs::structured_transcript`, because the shell lives in the binary: 3/3, written
  with the wiring.
- **Refactor**: none beyond `cargo fmt`.

## M4 (T024–T030) — shared disclosure card, icons, monospace text

- **Red**: T024, T025, T026 written first (icons pinned in `tests/icons.rs` and `tests/icons_font.rs`;
  `Text::monospace` and the `AccordionSection` slots as stored-but-undrawn builder steps;
  `DisclosureCard`'s `From` as `todo!()`; its outline, ripple, contrast and header-fits gates):
  `cargo test -p micold-client --lib ui::material` 714 passed, 8 failed (the six card tests on the
  stub, the slots drawn in the header row, monospace's face); `layout_snapshot` 72 passed, 1 failed
  (`disclosure_card_header_fits`). The icon variants (T027) went in beside their pins, so T024 was
  red by compilation only.
- **Green**: T027–T030 implemented; `--lib ui::material` 722/722; `icons` 6/6, `icons_font` 11/11,
  `layout_snapshot` 73/73 (the committed fixture unchanged: a section without slots draws its old
  header row), `material_builder_api` 11/11, `showcase_completeness` 43/43, `showcase_captions` 7/7,
  `showcase_state` 29/29.
- **Refactor**: the section's header row split into `plain` (unchanged) and `slotted`; `cargo fmt`.
