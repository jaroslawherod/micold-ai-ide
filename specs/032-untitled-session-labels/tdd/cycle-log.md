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
- mutant (cycle 2's discrimination, checked from cold context because its red was lost; each mutant
  applied to a clean tree, run, then restored with `git checkout --`):
  - `claude_first_turn` returns `None` for every prefix → `ai_cli_provider.rs` 15 passed, **2
    failed** (`the_label_is_the_shaped_first_turn_of_the_sessions_own_transcript`,
    `the_title_and_the_label_are_read_from_the_same_file_and_never_mixed`: `left: None` /
    `right: Some("The first typed turn")`) and `untitled_session_labels.rs` 11 passed, **7 failed**
    (A1–A4 and U66, U67, U65 included: `left: Pending` / `right: Derived("…")`, `left: 0` /
    `right: 4`).
  - the record classifier is skipped (`ClaudeTurn::of(&text)` → `ClaudeTurn::Prompt`, so every
    candidate record counts as typed text) → `first_turn_label.rs` 13 passed, **9 failed**: the
    command cases (U24, U25, U26, U27, U28, U29, U35), the inserted-text case (U31) and the
    nothing-typed case (U36).
  - Not killed by either mutant, and so **not** evidence of cycle 2's tests: U30, U32, U33, U34 and
    the shaping and prefix behaviours of cycle 1.
- suite: `mise run gate` (fmt → clippy core → clippy workspace `-D warnings` → `cargo test
  --workspace` → `scripts/tests/*.test.sh`) → `EXIT=0`, 3408 passed, 0 failed, 331 binaries.
- quickstart §B1: `MICOLD_LABEL_CORPUS=1 … --test first_turn_label_corpus -- --ignored` → 83
  transcripts: 67 titled, 16 labelled, **0 neither** (SC-002). The four BUG-001 rows read four
  different labels, `9a536c7e` reading `/speckit-autopilot` (SC-001, SC-005). Recorded in
  `evidence/quickstart-b.md`.
- refactor: none; the widening removed a duplicated `Pending` filter rather than adding code.

## Cycle 4 — US1 Copilot: `name:`, else `summary:`, else the first turn (U37–U43, U50–U55; A5, A6)

Outside-in, in one grouped cycle per file, as cycles 1–3 were: the acceptance tests were written and
observed failing first, then the two unit groups, then the implementation.

- tasks: T031–T038, T045
- tests, written and run before any implementation existed:
  - `crates/micold-core/tests/fixtures/first_turn/copilot/*` (T031, 6 event logs + 6 `workspace.yaml`)
  - `crates/micold-daemon/tests/untitled_session_labels.rs` (T034) — A6
    `a_listed_copilot_session_with_only_a_summary_is_named_by_it_and_never_labelled`, A5
    `a_listed_copilot_session_with_neither_key_reads_its_first_typed_turn`, and US2 for Copilot
    `a_labelled_copilot_session_reads_the_name_its_workspace_file_gained`
  - `crates/micold-core/tests/first_turn_label.rs` (T032) — U37–U43 and a nothing-typed guard
  - `crates/micold-core/tests/copilot_provider.rs` (T033) — U50–U55
- red (outer), `scripts/build-lock.sh cargo test --test untitled_session_labels copilot`:

  ```
  assertion `left == right` failed: an older Copilot's `summary:` is that session's title (FR-016, C7.1)
    left: Pending
   right: Named("The summary an older Copilot wrote")
  test result: FAILED. 0 passed; 3 failed; 0 ignored; 0 measured; 18 filtered out
  ```

- red (units), `scripts/build-lock.sh cargo test -p micold-core --test first_turn_label` and
  `--test copilot_provider`, against a `copilot_first_turn` stub returning `None`:

  ```
  the_first_user_message_copilot_did_not_source_itself_is_the_label
    left: None  right: Some("Add the login page")
  test result: FAILED. 23 passed; 7 failed; 0 ignored; 0 measured; 0 filtered out

  a_name_outranks_a_summary_and_a_summary_stands_in_for_a_missing_name
    left: None  right: Some("The summary an older Copilot wrote")
  the_label_is_the_first_turn_of_the_sessions_own_event_log
    left: None  right: Some("The tenth record is the first turn")
  test result: FAILED. 24 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out
  ```

- **mutant** (the three tests that passed on their first run, so their red could not be observed):
  - `a_block_summary_and_a_file_with_neither_key_are_no_title` — dropping the `|`/`>` guard in
    `read_yaml_scalar` gives `left: Some("|-") right: None`. Killed.
  - `a_first_turn_past_the_read_bound_is_no_label` — `.take(u64::MAX)` in `read_prefix` gives
    `left: Some("Typed past the bound") right: None`. Killed.
  - `a_copilot_log_with_nothing_typed_in_it_has_no_label` — dropping the `data.source` filter gives
    `left: Some("skill context") right: None`. Killed.

  Each mutant was restored with `git checkout --` and the file re-verified clean afterwards; the
  commit was made **before** the probes (`efd49a08`), so nothing probe-related can reach a diff.
- green: `copilot_first_turn` + `copilot_turn_text` in `crates/micold-core/src/first_turn.rs` (C4.1
  `source`/`isAutopilotContinuation`, C4.2 `content`, C4.3 first non-empty, C4.4 nothing stripped);
  `CopilotProvider::read_title` = `name:` else `summary:` and `read_yaml_scalar`'s block-scalar guard
  (C7.1–C7.3); `CopilotProvider::read_label` = bounded prefix + `copilot_first_turn` (T036). T013's
  placeholder "Copilot derives no label" assertion in `ai_cli_provider_seam.rs` was deleted, as that
  task said it would be. User guide (T037) and the Copilot arm of the corpus probe (T038).
  `cargo test -p micold-core --test first_turn_label --test copilot_provider --test
  ai_cli_provider_seam --test ai_cli_provider`: 17 + 12 + 26 + 30 passed, 0 failed.
- outer loop closed: `cargo test --test untitled_session_labels` — 21 passed, 0 failed (A5, A6 and
  the Copilot US2 case among them), so T045 holds.
- suite: `mise run gate` (see the milestone's ledger entry).
- refactor: none. `copilot_first_turn` reuses `complete_lines`, `shape_label` and `read_prefix`
  unchanged — the C2/C5 layers were built provider-agnostic in cycle 2, and this cycle is the
  evidence that they are.
- notes: the evidence probe (T038) is a **report**, not a verdict: SC-008/SC-009 are decided by the
  listed-session fixture tests (D10). Its run over the development machine's 286 Copilot session
  directories reads 139 titled, 2 labelled, 145 with neither — before this change the ~44 old
  `summary:`-only sessions among them would have read as untitled.

## Cycle 5 — US3: a running session gets its label too (U69; U68, U70, A10, A11 as guards)

- tests: `crates/micold-daemon/tests/untitled_session_labels.rs`, four new tests over a **live**
  session — catalog-known *and* registered in the runtime registry with a real PTY, the
  `register_cat` / titler harness copied from `activity_pipeline.rs` (T028):
  - `a_running_untitled_session_reads_its_label_on_the_tick_after_its_first_prompt` (A10
    prompt-hook-first, U68)
  - `a_spinner_drained_before_the_prompt_hook_still_gets_the_label` (A10 spinner-first, U69)
  - `a_running_labelled_session_switches_to_the_title_its_terminal_reports` (A11)
  - `an_idle_tick_that_changed_nothing_reads_no_records` (U70)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test untitled_session_labels` →
  24 passed, 1 failed.
  - U69 `a_spinner_drained_before_the_prompt_hook_still_gets_the_label`, at the assertion that is
    the behaviour: `assertion left == right failed: the drain that changed the activity must
    re-arm the lookup too (C6.3b)` `left: 0` / `right: 1`. The spinner moved the session to
    `Working`, so the `UserPromptSubmit` hook that followed changed nothing, `note_activity`
    returned `false`, and `recover_live_session_names` had no candidate to read — the first turn
    would have waited for the end of the turn (research R9, plan review F4).
  - The first red of the same test was the harness, not the behaviour (`the braille spinner glyph
    must reach the FSM`): `sessions_for` is the catalog alone and never carries activity, so the
    helper now reads the broadcast snapshot (`catalog_snapshot`), as `activity_pipeline.rs` does.
  - U68, U70, A10 (prompt-hook-first) and A11 were **green on arrival**: `note_activity` already
    re-armed the flag and cycle 2 already made a label count toward the broadcast (C6.3a). They
    are kept as guards over the half of FR-010 that shipped in M1 — U70 is the one that pins the
    bound SC-006 rests on, that an idle tick reads no provider store at all.
- green: `crates/micold-daemon/src/state.rs` `drain_signals` sets `live.name_stale = true` in the
  `SpinnerObserved` branch whenever the FSM's signal changed, beside the `out.changed` it already
  set there (T029, C6.3b). Three lines of comment cite FR-010 and R9.
  `scripts/build-lock.sh cargo test -p micold-daemon --test untitled_session_labels` → 25 passed,
  0 failed.
- outer loop closed: the US3 acceptance tests pass with the full suite (T044); see the gate below.
- suite: `mise run gate` — see the milestone's ledger entry for the passing SHA (recorded there
  rather than here, because the gate is run once per milestone, after cycle 5a below).
- refactor: none. The change is one field assignment in a branch that already existed; the honest
  alternative — routing the drain's spinner through `note_activity` — would take the state lock
  twice per tick per session on the 250 ms path, which is the very thing `drain_signals` is
  written to avoid (module invariant, research R2).

## Cycle 5a — the spinner that arrives before anything is typed (U72; T047)

Opened by M3's review A, which found that cycle 5's re-arm is **one-shot per live session**.

- test: `crates/micold-daemon/tests/untitled_session_labels.rs`
  `a_spinner_seen_before_the_first_prompt_does_not_cost_the_session_its_label` (U72, A10's third
  order). The titler is drained to `Working` while the conversation is still empty — the FSM's one
  and only `Unknown → Working` move, since `SpinnerObserved` is a no-op from every other state
  (H1a) — the recovery pass then finds nothing and clears the flag, and only then does the user
  type.
- red: `scripts/build-lock.sh --no-lock cargo test -p micold-daemon --test untitled_session_labels
  a_spinner_seen_before` → `assertion left == right failed: the first prompt re-arms the lookup
  even when it changes no badge (C6.3c), or the row reads "New session" until the end of the turn`
  `left: 0` / `right: 1`. So a CLI that draws a spinner while it starts up — connecting its servers,
  loading its plugins — spent the session's only spinner-driven re-arm before the prompt, and
  nothing re-armed the lookup again until the closing `Stop`: the row read "New session" for the
  whole first turn, the delay FR-010 exists to prevent.
- green: `crates/micold-daemon/src/state.rs` `note_activity` re-arms on a `UserPromptSubmit` hook
  whether or not the signal changed (T047, C6.3c) — the prompt is the event that wrote the turn,
  and it fires at most once per turn, so SC-006's bound is unmoved. Contract C6.3c and research R9
  amended to say so.
- notes: the reviewer's own observation that cycle 5's test cannot catch this is right, and is why
  U72 is a separate behaviour rather than a stronger assertion on the same test: cycle 5's test
  writes the transcript *before* letting the spinner fire, so its single transition lands after the
  records exist.
- refactor: none.

## Cycle 6 — close-phase mutation evidence: attempted, not obtained (2026-09-26)

Opened by the close phase's `speckit-tdd-verify` (`tdd/verification.md`, verdict FAIL), to supply the
red that cycle 2 lost to a rate-limit kill. **It did not succeed, and this entry says so rather than
leaving the gap to be inferred from a missing section.** Nothing below is evidence; the mutant table
is a design read off the code, which is a head start for whoever runs it, not a result.

- what was asked: a killed deliberate mutant for each behaviour with neither a recorded red nor a
  demonstrated kill — U30, U32, U33, U34 (cycle 2, missed by cycle 3's two mutants), U46–U49, U61,
  U62 (cycle 2, no mutation evidence at all), and A7, A8 (`example` rows that passed on arrival).
- what happened: one mutant was applied (`|| flag("isMeta")` deleted from `claude_candidate_text`,
  targeting U30) and its test run never reached a compiled binary — it queued behind another
  worktree's build for the whole attempt, and by the time the lock came free the file had been
  restored, so the `30 passed` it printed was unmutated code. **Zero valid mutants ran.** An earlier
  attempt in the same phase was abandoned when two audit instances collided in this worktree and one
  reverted the other's mutant mid-run; that collision also left a stray duplicate of the
  `if !unlabelled` guard in `state.rs`, which was found and removed before the close PR (it is not in
  `origin/main` and it is not in the PR).
- **unproven, carried as `tasks.md` T048–T050 and as ledger follow-ups**: U30, U32, U33, U34, U46,
  U47, U48, U49, U61, U62, A7, A8. They pass today and every contract clause they cover was
  re-verified against the code in the close phase's `speckit-converge` pass; what is missing is
  evidence that they *discriminate*, which is a different claim and is not made here.
- designed mutants, from reading the code only (predictions, not observations):

  | # | file, function | the one-line change | predicted to kill |
  |---|---|---|---|
  | 1 | `micold-core/src/first_turn.rs` `claude_candidate_text` | delete `\|\| flag("isMeta")` | U30 |
  | 2 | `first_turn.rs` `content_text` | `part_type(part) == Some("text")` → `!=` | U32 |
  | 3 | `first_turn.rs` `claude_first_turn` | `if label.is_some() { return label; }` → `return label;` | U33 |
  | 4 | `first_turn.rs` `claude_first_turn` | a parse error aborts instead of being skipped | U34 |
  | 5 | `micold-core/src/provider.rs` `ClaudeProvider::read_label` | body → `self.read_title(..)` | U46, U49 |
  | 6 | `first_turn.rs` `read_prefix` | `.take(LABEL_BUDGET_BYTES)` → `.take(u64::MAX)` | U48 (U21 too) |
  | 7 | `micold-daemon/src/state.rs` `discover_external_sessions` | read `read_label` before `read_title` | U61 |
  | 8 | `state.rs` `recover_session_names` candidate filter | negate the `Named` exclusion | U62 (A1, A2, A9, U63 too) |
  | 9 | `micold-daemon/src/catalog.rs` `record_session_name` | match `Derived(_)` where it matches `Named(current) if current == name` | A8 (U60, U63, A9, U75 too) |

- three findings that stand on code reading alone, and are worth more than the mutants would have
  been:
  - **A7 is defended three times over, so mutant 8 alone would survive it.** `a_titled_session_is_
    never_given_a_label` is held up by (a) the recovery candidate filter excluding `Named`, (b)
    `RecoveryCandidate::of` setting `unlabelled = matches!(label, Pending)`, so even a `Named`
    candidate that slipped the filter is asked for a title only, and (c) `set_derived_label`
    refusing a non-`Pending` session. Negating (a) leaves (b) returning `None` — that test writes no
    `ai-title` — so the pass still returns 0 and nothing changes. A7's red needs two simultaneous
    changes, or a test that observes the read. Layer (c) is the one already covered, by cycle 1's U3
    mutant. Defence in depth is why FR-005 does not rest on caller care (plan, *Post-Design
    Re-check*), and it is also why a single mutation cannot express its absence.
  - **U47 looks unkillable by one small mutation.** `a_missing_transcript_has_no_label_and_does_not_
    fail` asserts `None` for a file that is not there; no single small change makes text out of
    nothing. Recorded as a negative behaviour whose value is regression protection, not
    discrimination.
  - **The read-layer guard is untested, verified.** Deleting `if !unlabelled { return None; }`
    (`state.rs:1355`) **survives** U64: the already-`Derived` session's records are re-read, the
    catalog then refuses the second label, and the pass still returns 0 with the label unchanged —
    exactly the two things U64 asserts. The guard exists to avoid the read itself (its doc comment;
    SC-006). Observing it needs a read-counting provider, and `DaemonState` builds its providers from
    the environment with no injection seam (`tasks.md` *Daemon tests*), so closing it means adding
    production structure for a test. `verification.md` finding 6; a ledger follow-up, not this PR.
- suite after every restore: `crates/micold-core/src/first_turn.rs` restored with
  `git checkout --`, and the stray duplicate of the `if !unlabelled` guard removed from `state.rs`.
  `git diff origin/main -- crates/` was then read in full, in the worktree **and** in the index, and
  is empty: the close phase changes no code at all. `mise run gate` passed on that tree, 0 failures;
  the SHA is recorded in `autopilot.md` and in the close PR body.
