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
