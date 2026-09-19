# Cycle Log: Worktree Creation & Deletion Flow Refinement — BUG-001

Append only. Newest last. Every entry's `red` block is the evidence that the test
existed and failed before the implementation.

## Baseline

- suite: `scripts/build-lock.sh cargo test --workspace` -> 3276 passed, 0 failed, 6 ignored
  (326 binaries)
- commit: `f63834f7`
- recorded: cycle 0, before any change. Spec, plan and tasks edits for BUG-001 were uncommitted in
  the working tree at the time; they touch no code.

## Cycle 1: U1 an idle form is dismissed by Escape and the scrim

- test: `crates/micold-client/tests/add_worktree_dismissal.rs::an_idle_form_is_dismissed_by_escape_and_the_scrim` (new)
- red: none on arrival. `scripts/build-lock.sh cargo test --test add_worktree_dismissal an_idle_form_is_dismissed_by_escape_and_the_scrim -- --exact`
  -> passed on first run (existing behavior, the boundary side of FR-010a).
  Deliberate mutant: an unconditional `.protecting_input()` in `AddWorktreeDialog::dismissal` -> `FAILED`,
  `left: None` / `right: Some(WorktreeForm(Cancelled))` at `add_worktree_dismissal.rs:30`. Mutant reverted with `git checkout`.
- green: no implementation change.
- refactor: none needed
- commit: `fix(013): the add-worktree dialog ignores Escape and the scrim while a create runs (BUG-001)` (with cycle 2)
- notes: green on arrival; it guards cycle 2, whose protection must not reach an idle form

## Cycle 2: U2 a form whose create is in flight is dismissed by neither Escape nor the scrim

- test: `crates/micold-client/tests/add_worktree_dismissal.rs::a_form_whose_create_is_in_flight_is_dismissed_by_neither_escape_nor_the_scrim` (new)
- red: `scripts/build-lock.sh cargo test --test add_worktree_dismissal a_form_whose_create_is_in_flight_is_dismissed_by_neither_escape_nor_the_scrim -- --exact`
  -> `left: Some(WorktreeForm(Cancelled))` / `right: None` at `add_worktree_dismissal.rs:60` (1 failed)
- green: `AddWorktreeDialog { creating }`, built by `open_in` from the form's status; `dismissal()` adds
  `.protecting_input()` while `Creating`. Both tests in the file pass.
  Crate suite `scripts/build-lock.sh cargo test -p micold-client` -> 1793 passed, 0 failed, 2 ignored
  (139 binaries), U1 included. An earlier run was killed with exit 137 (oomd, memory pressure from
  another worktree's release build) and is not counted.
- refactor: none needed
- commit: `fix(013): the add-worktree dialog ignores Escape and the scrim while a create runs (BUG-001)`
- notes: per the milestone brief, a cycle's suite is the client crate; the workspace suite runs once before the PR

## Cycle 3: U3 a create in flight over a Settings draft does not fall through to the draft

- test: `crates/micold-client/tests/add_worktree_dismissal.rs::a_create_in_flight_over_a_settings_draft_does_not_fall_through_to_the_draft` (new)
- red: `scripts/build-lock.sh cargo test -p micold-client --test add_worktree_dismissal a_create_in_flight_over_a_settings_draft_does_not_fall_through_to_the_draft -- --exact`
  -> `left: Some(Settings(Cancelled))` / `right: None` at `add_worktree_dismissal.rs:94` (1 failed)
- green: `app::on_escape` asks `registry::topmost` first; an open surface answers for itself (its
  Escape rule, which is nothing for a protected one), and the Settings fallback runs only when no
  registered surface is open. File -> 3 passed. Crate suite `scripts/build-lock.sh cargo test -p micold-client`
  -> 1802 passed, 0 failed, 2 ignored (139 binaries; the count is after the rebase onto `origin/main`)
- refactor: none needed
- commit: `fix(013): a protected dialog stops Escape reaching the Settings draft behind it (BUG-001 U3)`
- notes: `ui/mod.rs` wires the scrim's `on_dismiss` only when `on_escape` answers, so the scrim is inert during a create

## Cycle 4: U4 a form whose create failed is dismissed by Escape again

- test: `crates/micold-client/tests/add_worktree_dismissal.rs::a_form_whose_create_failed_is_dismissed_by_escape_again` (new)
- red: none on arrival. `scripts/build-lock.sh cargo test -p micold-client --test add_worktree_dismissal a_form_whose_create_failed_is_dismissed_by_escape_again -- --exact`
  -> `1 passed`: `open_in` reads the status afresh, and `create_failed` already returns the form to `Editing`.
  Deliberate mutant: `create_failed` no longer sets `status = Editing` -> `left: None` /
  `right: Some(WorktreeForm(Cancelled))` at `add_worktree_dismissal.rs:114` (1 failed). Mutant reverted with `git checkout`.
- green: no implementation change. Crate suite -> 1803 passed, 0 failed, 2 ignored (139 binaries)
- refactor: none needed
- commit: `test(013): a failed create's form is dismissible again (BUG-001 U4)`
- notes: green on arrival; it pins the end of the protected span

## Cycle 5: U5 the in-dialog Cancel closes a form whose create is in flight

- test: `crates/micold-client/tests/add_worktree_dismissal.rs::the_in_dialog_cancel_closes_a_form_whose_create_is_in_flight` (new, characterization)
- red: none on arrival. `scripts/build-lock.sh cargo test -p micold-client --test add_worktree_dismissal the_in_dialog_cancel_closes_a_form_whose_create_is_in_flight -- --exact`
  -> `1 passed`: `cancelled` closes the form whatever its status.
  Deliberate mutant: `cancelled` returns early while `Creating` -> panicked at `add_worktree_dismissal.rs:136`
  ("Cancel must close the add-worktree form even while its create runs", 1 failed). Mutant reverted with `git checkout`.
- green: no implementation change. Crate suite -> 1804 passed, 0 failed, 2 ignored (139 binaries)
- refactor: none needed
- commit: `test(013): Cancel still closes the add-worktree form mid-create (BUG-001 U5)`
- notes: the view's Cancel button sends `Msg::Cancelled` unconditionally (`ui/worktree_form.rs:292`)

## Cycle 6: U6 a create failing after Cancel is reported as an error notification

- test: `crates/micold-client/tests/add_worktree_dismissal.rs::a_create_failing_after_cancel_is_reported_as_an_error_notification` (new)
- red: `scripts/build-lock.sh cargo test -p micold-client --test add_worktree_dismissal a_create_failing_after_cancel_is_reported_as_an_error_notification -- --exact`
  -> panicked at `add_worktree_dismissal.rs:173`: `a create that failed after its form closed must still be reported (FR-010b)` (1 failed)
- green: `worktree_form::State` gains `cancelled_mid_create`, set by `cancelled` when the form it
  closes is `Creating`; `create_failed` takes it and, with no form open, returns
  `notifications::error(message)` instead of writing the form's error line. File -> 6 passed.
  First crate-suite run -> 1 failed: `root_state_is_shared::component_local_paths_are_pinned_or_moved`
  (`state.worktree_form.cancelled_mid_create` — written only by `worktree_form`). The flag cannot
  move into the form component — it exists only while that component is gone — so it joins
  `COMPONENT_LOCAL`, pinned by this cycle's test. Crate suite -> 1805 passed, 0 failed, 2 ignored (139 binaries)
- refactor: none needed
- commit: `fix(013): a create that fails after Cancel is reported as a notification (BUG-001 U6)`

## Cycle 7: U7 a failure after Cancel names the stage it failed at

- test: `crates/micold-client/tests/add_worktree_dismissal.rs::a_failure_after_cancel_names_the_stage_it_failed_at` (new)
- red: `scripts/build-lock.sh cargo test -p micold-client --test add_worktree_dismissal a_failure_after_cancel_names_the_stage_it_failed_at -- --exact`
  -> panicked at `add_worktree_dismissal.rs:206`: `the notification must name the stage the create failed at ("Setting up submodules"), got "git failed to fetch a submodule"` (1 failed)
- green: the flag becomes `cancelled_create: Option<CancelledCreate { mode, stage }>`, recorded from
  the form as Cancel closes it; the failure text is `Creating the worktree failed at "<stage label>": <message>`
  (or without the stage when none was reported). `COMPONENT_LOCAL` follows the rename. File -> 7 passed.
  Crate suite -> 1806 passed, 0 failed, 2 ignored (139 binaries)
- refactor: none needed; the rename was the green step's
- commit: `fix(013): a failure after Cancel names the stage it failed at (BUG-001 U7)`
