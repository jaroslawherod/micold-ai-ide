# Cycle Log: #1663 — highlighted user turns (M1)

Append only. Newest last. Tests were written before the code they cover; this was not run through
the `tdd.run` hook, so the log is kept by hand and says what was and was not seen red.

## Cycle 1 — fill and contrast (T002)

- test: `crates/micold-core/tests/tokens.rs::the_user_turn_fill_stands_apart_and_keeps_text_readable`.
- red: none. It characterises existing tokens (no code to write), so it passed on the first run;
  measured values are in `research.md` (R1 addendum). A mutation (`surface_container_low` as the
  fill) fails the "stands apart from the card fill" assertion by construction (ratio 1.0 < 1.05).

## Cycle 2 — the primitive (T003–T005)

- tests: `crates/micold-material/tests/user_turn.rs` (3 tests).
- red: written before `UserTurn`, `user_turn_surface` and `user_turn_edge` existed (compile
  failure on the missing items, which the task counts as red). The first build was run after both
  were added, so the compile failure itself was not captured. green: 3 passed.

## Cycle 3 — the transcript wiring (T006–T007)

- tests: `crates/micold-client/tests/transcript_entry_view.rs`, six tests (typed and delivered,
  attachments and empty text, assistant not highlighted, command folded/unfolded with fold
  state kept and back-to-back turns, sub-agent thread).
- red: `scripts/build-lock.sh cargo test -p micold-client --test main transcript_entry_view` before
  the wrap in `entries.rs`: 22 passed, 4 failed (typed/delivered, attachments/empty, command,
  sub-agent: "lit ... 0" surface pixels; the sub-agent folded count was incidental 23). The
  assistant-only test passed, as a guard.
- green: after wrapping both `UserMessage` branches: 26 passed.

## Cycle 4 — TDD remediation (T011–T014, `tdd/verification.md`)

- tests: `transcript_entry_view.rs` gains `a_tool_card_is_not_highlighted` and
  `command_output_is_not_highlighted` (FR-005); `bare` now checks the edge too; the pixel
  thresholds are named (`EDGE_MIN`, `is_two_turns`); the command test is split into one test per
  behaviour. `micold-material/tests/user_turn.rs` asserts the edge's left-only corner radius and
  draws the element (`--features test-support`) to check the 4dp edge and the content offset.
- The new FR-005 tests pass against the existing code, as negatives of already-correct behaviour
  (no red by design); their strength is by reasoning, no mutant was run: a tool card or output wrapped in
  `UserTurn` adds thousands of surface pixels, far over the thresholds.
- FR-008 / SC-003: `features_transcript_selection` (`scripts/build-lock.sh cargo test -p
  micold-client --test main features_transcript_selection`) 42 passed; core is untouched by this
  remediation, so `mise run test-core` is covered by the PR's full gate (`mise run gate`).
