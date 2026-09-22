# Cycle Log: A session the AI CLI never titled still gets a label

Append only. Newest last. Every entry's `red` block is the evidence that the test existed and
failed before the implementation.

## Baseline

- suite: `scripts/build-lock.sh cargo test --workspace` -> 3333 passed, 0 failed, 6 ignored (327 binaries)
- commit: `fa19e553` (specs-only commits after it do not touch code)
- recorded: cycle 0, before any change

## Cycle 1 — Phase 2 foundations (U1–U23, U44, U45, U56–U60), grouped

- tests:
  - `crates/micold-core/tests/session_label_kinds.rs` (U1–U6, new)
  - `crates/micold-core/tests/session_name_round_trip.rs` (U7–U10)
  - `crates/micold-core/tests/schema_hash.rs` `the_wire_changes_for_this_feature_cost_exactly_one_version_bump` and `protocol_auth.rs` `the_protocol_version_is_fourteen` (U11)
  - `crates/micold-core/tests/protocol_roundtrip.rs` `a_session_summary_carrying_a_derived_label_round_trips_on_both_wires` (U12)
  - `crates/micold-client/tests/session_title_sync.rs` `derived_labels::*` (U13–U15)
  - `crates/micold-core/tests/first_turn_label.rs` (U16–U23, new)
  - `crates/micold-core/tests/ai_cli_provider_seam.rs` (U44, U45)
  - `crates/micold-daemon/tests/untitled_session_labels.rs` (U56–U60, new)
- red: `scripts/build-lock.sh cargo test --no-fail-fast --test session_label_kinds --test session_name_round_trip --test schema_hash --test protocol_roundtrip --test session_title_sync --test first_turn_label --test ai_cli_provider_seam --test untitled_session_labels`. Stubs only: `Derived` rendered as Pending, `set_derived_label`, `read_prefix`, `shape_label`, the fake's `read_label` and `record_session_label` all returned "nothing".
  - U1 `a_derived_label_displays_its_text_like_a_title`: `left: "New session"` / `right: "Why does the sidebar read New session?"`
  - U2 `a_pending_session_takes_a_derived_label_and_reports_the_change`: panicked at session_label_kinds.rs:34
  - U7 `a_derived_label_is_saved_as_a_label_and_loads_back_derived`: `a label is stored under its own key (C8.1)` `left: None` / `right: Some("/speckit-autopilot")`
  - U11 schema_hash: `left: 13` / `right: 14`
  - U13 `a_derived_label_is_adopted_onto_a_pending_row`: `left: Pending` / `right: Derived("/speckit-autopilot")`; U15 `a_pending_summary_never_clears_a_derived_label`: same shape
  - U16 `whitespace_and_line_break_runs_collapse_to_one_space_and_ends_are_trimmed`: `left: None` / `right: Some("Fix the flaky login test")`; U18, U19, U20: `left: None`; U21 `the_prefix_is_at_most_one_mebibyte`: panicked at first_turn_label.rs:100 (`expect`); U22: `left: None`
  - U44 `the_fake_answers_a_label_it_was_given_and_never_offers_it_as_a_title`: `with_label mirrors with_title (C1.3)` `left: None`
  - U56–U59: the four `record_session_label` tests panicked (`left: Pending` / `right: Derived(..)`, `result.is_err()`)
- passed on arrival, each checked with a deliberate mutant (killed, then restored): U3, U4 (mutant: `set_derived_label` without the `Pending` guard; 2 tests failed); U5, U17, U23 (stub already returned nothing, which is the asserted outcome); U6, U60 (`set_title` / `record_session_name` already replace any label; guards); U8, U9, U10 (mutant: label-first precedence, no empty check, `label` without `skip_serializing_if`; 3 tests failed); U12 (guard: postcard carries the new variant); U14 (Named was already adopted); U45 (mutant: Pi `read_label` returning `read_title`; failed).
- green: `Derived` + `set_derived_label` (session.rs); `StoredSession.label` (store.rs); `PROTOCOL_VERSION` 14; client adopts `Named | Derived`; `first_turn::{read_prefix, shape_label}` with `unicode-segmentation`; required `AiCliProvider::read_label` (Claude stub `None` until T022, Copilot `None` until M2, Pi `None`, fake `with_label`); `Catalog::record_session_label`. Targeted binaries all green.
- suite: `scripts/build-lock.sh cargo test --workspace --no-fail-fast` -> 3361 passed, 2 failed, 6 ignored (330 binaries). Both failures were real regressions of this change, fixed in the cycle: `protocol_auth.rs` pinned version 13 (renamed to `the_protocol_version_is_fourteen`, asserts 14); `service_capability_fakes.rs` FR-016 gate rejected `MinimalProvider::read_label` for ignoring `self` (it now answers from a `labels` map, exercised in its test). Re-run of those three binaries: 11 + 13 + 12 passed, 0 failed.
- refactor: none needed; `read_prefix`'s complete-line cut is shared with `claude_first_turn` in cycle 2.
- commit: see `git log` (`feat(032): derived session label kind, store, wire and catalog seam`)
- notes:
  - Deviation: cycles grouped per task pair (tests of T003/T005/…/T015 red in one batched build, then green in one), not one behavior per build. Reason: every build waits on the shared `target-shared` lock behind other worktrees, and one suite run takes about 6 minutes. Red evidence per behavior is still the real output above.
  - Test fix, before the implementation change: U7 first asserted `!contains_key("title")`. The store has always written `"title": null` for an untitled session, so the assertion was over-specified; it now asserts "absent or null", with the reason in its message.
  - Baseline: the in-session baseline run was stopped deliberately; the recorded baseline at `fa19e553` (3333 passed) and green main CI at `6e21c19a` stand in for it.

## Cycle 2 — US1 `claude`: the first turn becomes the label (U24–U36, U46–U49, U61, U62, U66, U67; A1–A4)

- tasks: T017–T025, T042
- tests:
  - `crates/micold-core/tests/fixtures/first_turn/claude/*.jsonl` (T017, 11 synthetic fixtures)
  - `crates/micold-core/tests/first_turn_label.rs` `claude_first_turn` cases (U24–U36)
  - `crates/micold-core/tests/ai_cli_provider.rs` `the_label_is_the_shaped_first_turn_…`,
    `a_missing_transcript_has_no_label_…`, `a_first_turn_past_the_read_bound_…`,
    `the_title_and_the_label_are_read_from_the_same_file_and_never_mixed` (U46–U49)
  - `crates/micold-daemon/tests/untitled_session_labels.rs` A1–A4, U61, U62, U66, U67
- red: **the per-behaviour red output of this cycle was lost.** The session that ran it was killed by
  a rate limit mid-T022 and never wrote this entry; its tests and implementation arrived in one
  commit (`feat(032): label untitled claude sessions with their first typed turn`), so git history
  does not separate them either. The tests were therefore re-checked for discrimination from cold
  context in cycle 3 with a deliberate mutant (see cycle 3, *mutant*), rather than claimed as
  test-first. Treat cycle 2 as **verified by mutation, not by recorded red** — a Principle I
  deviation, recorded here rather than papered over.
- green: `claude_first_turn` in `crates/micold-core/src/first_turn.rs` (C3: candidate filter,
  injected-wrapper prefixes, command name/args, follower rule C3.5a–b′, first non-empty source);
  `ClaudeProvider::read_label` (C2 + C3); `discover_external_sessions` title→label→pending (C6.1) and
  `record_recovered_names` label branch counting toward the broadcast (C6.2, C6.3, C6.3a) in
  `crates/micold-daemon/src/state.rs`; the user guide (T024); the corpus probe (T025).
- suite: see cycle 3 (`mise run gate`, one run for both cycles).
- refactor: none.

## Cycle 3 — US2: a title still wins, and replaces a label (U63; U64, U65, U71, A7–A9 as guards)

- tasks: T026, T027, T043
- tests: `crates/micold-daemon/tests/untitled_session_labels.rs` —
  `a_titled_session_is_never_given_a_label` (A7), `a_labelled_session_reads_the_title_its_records_gained`
  (A9, U63), `a_label_is_never_derived_a_second_time` (U64), `an_observed_terminal_title_replaces_a_label_and_is_persisted`
  (A8), `a_title_and_a_label_racing_for_one_session_end_named` (U65),
  `nothing_but_the_recovery_path_writes_a_label` (U71)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test untitled_session_labels -- --test-threads=1`
  → `17 passed; 1 failed`.
  - U63 `a_labelled_session_reads_the_title_its_records_gained`: panicked at
    `untitled_session_labels.rs:588`, `assertion left == right failed: a labelled session is still
    asked for a title, so the row is broadcast (C6.2, C6.3a)` `left: 0` / `right: 1` — the pass
    filtered its candidates to `Pending`, so the `Derived` session was never asked for the title its
    records had gained.
  - Guards, passing on arrival as the test list says they may (A7, A8, U65, U71 and, before the
    widening, U64 — it was then excluded by the same `Pending` filter and is a real candidate only
    after it):
    - A7, A8, U65: `record_session_name` / `record_session_label` already enforce the precedence.
    - U71: the call-site scan already found exactly one writer.
- green: `recover_session_names` and `recover_live_session_names` in
  `crates/micold-daemon/src/state.rs` take every non-archived session whose label is **not** `Named`
  (C6.2); the label read stays behind `unlabelled` so a `Derived` session is asked for a title only
  (FR-007). `prunable_session_cwds` stays `Pending`-only, with the reason in its doc comment
  (data-model invariant 4).
