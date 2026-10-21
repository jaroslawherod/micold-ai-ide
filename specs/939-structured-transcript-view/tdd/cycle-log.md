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
