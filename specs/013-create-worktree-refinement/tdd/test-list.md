---
feature: 013-create-worktree-refinement
loop: outside-in
profile: .specify/memory/tdd-profile.md
spec_criteria: 3 # scoped to BUG-001: FR-010a, FR-010b, SC-006 (the rest of 013 closed before this list existed)
planned_at: f63834f7
updated_at: f63834f7
suite_baseline: green # 3276 passed, 0 failed, 6 ignored, 326 binaries at f63834f7
---

# Test List: Worktree Creation & Deletion Flow Refinement — BUG-001 (the create overlay holds its ground mid-create)

**Scope.** Feature 013 closed before this extension was installed, and this list was written for
bugfix BUG-001 only (`bugs/BUG-001.md`, tasks T032–T041). It covers `FR-010a`, `FR-010b` and
`SC-006`. The feature's other criteria shipped before the list existed and are not re-derived here.

Derived from `spec.md` (FR-010a, FR-010b, SC-006, the two BUG-001 Edge Cases) and `plan.md`
§ "Bugfix BUG-001". The code was read only to place behaviors and find existing tests.

## Outer loop: acceptance behaviors

**Entry point.** No end-to-end GUI runner exists. The highest level a cargo test reaches is the
binary's `App` in `crates/micold-client/src/main_tests.rs`: a real `Outbox`, a create sent through
`shell::daemon_sync::send_worktree_create`, the daemon's reply fed through
`shell::daemon_sync::on_daemon_event`, and the dismissal paths the window uses —
`Message::EscapePressed` for the key, `app::on_escape` for the scrim (what `ui/mod.rs` wires into
the modal's `on_dismiss`). The rendered result is the T039 visual pass.

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| A1  | With a create in flight, Escape and the scrim leave the form open and still `Creating` | FR-010a, SC-006 | example | PENDING | |
| A2  | With a create in flight, the in-dialog Cancel closes the form; the daemon's later failure reply reaches the user as a notification carrying its message | FR-010b, SC-006 | example | PENDING | |
| A3  | Against a repository with submodules: scrim click and Escape mid-create change nothing; Cancel closes the dialog; the outcome arrives as a notification | FR-010a, FR-010b, SC-006 | example | PENDING | visual pass, T039 |

## Inner loop: unit behaviors

### `crates/micold-client/src/features/worktree_form.rs` — `AddWorktreeDialog` dismissal

Tests in `crates/micold-client/tests/add_worktree_dismissal.rs`, through `State::update` and the
public `overlay::registry` / `app::on_escape` entry points.

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U1  | An open form that is `Editing` is an ordinary dialog: Escape and scrim both yield `Msg::Cancelled` | FR-010a (the "no creation in progress" side) | example | DONE | `crates/micold-client/tests/add_worktree_dismissal.rs::an_idle_form_is_dismissed_by_escape_and_the_scrim` |
| U2  | An open form that is `Creating` is a `NonDismissibleDialog`: `registry::escape` and `app::on_escape` both yield nothing | FR-010a | example | DONE | `crates/micold-client/tests/add_worktree_dismissal.rs::a_form_whose_create_is_in_flight_is_dismissed_by_neither_escape_nor_the_scrim` |
| U3  | A `Creating` form over an open Settings draft: `app::on_escape` still yields nothing — it must not fall through to the Settings draft's cancel | FR-010a | example | DONE | `crates/micold-client/tests/add_worktree_dismissal.rs::a_create_in_flight_over_a_settings_draft_does_not_fall_through_to_the_draft` |
| U4  | A create that failed returns the form to `Editing`, and Escape dismisses it again | FR-010a (the span ends with the operation) | example | DONE | `crates/micold-client/tests/add_worktree_dismissal.rs::a_form_whose_create_failed_is_dismissed_by_escape_again` |
| U5  | The in-dialog Cancel (`Msg::Cancelled`) closes a `Creating` form | FR-010a ("the only way out") | characterization | DONE | `crates/micold-client/tests/add_worktree_dismissal.rs::the_in_dialog_cancel_closes_a_form_whose_create_is_in_flight` |

### `crates/micold-client/src/features/worktree_form.rs` — outcomes after the form closed

Tests in `crates/micold-client/tests/add_worktree_dismissal.rs`, observing
`state.notifications.queue.visible()`.

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U6  | Cancel on a `Creating` form, then `CreateFailed(msg)`: an error notification carries `msg` | FR-010b | example | DONE | `crates/micold-client/tests/add_worktree_dismissal.rs::a_create_failing_after_cancel_is_reported_as_an_error_notification` |
| U7  | Cancel on a `Creating` form after a stage was reported, then `CreateFailed`: the notification names that stage's label | FR-010b, FR-009 | example | DONE | `crates/micold-client/tests/add_worktree_dismissal.rs::a_failure_after_cancel_names_the_stage_it_failed_at` |
| U8  | `CreateStageChanged` after the form was cancelled mid-create still updates the stage the failure will name | FR-010b, FR-009 | example | DONE | `crates/micold-client/tests/add_worktree_dismissal.rs::a_stage_reported_after_cancel_is_the_one_a_failure_names` |
| U9  | Cancel on a `Creating` form, then `Created(worktree)`: an info notification names the worktree's `dir_name` | FR-010b | example | DONE | `crates/micold-client/tests/add_worktree_dismissal.rs::a_create_succeeding_after_cancel_is_reported_naming_the_worktree` |
| U10 | With the form open, `CreateFailed` and `Created` raise no notification (the in-overlay presentation stands alone) | FR-010b ("no duplicate notification") | example | DONE | `crates/micold-client/tests/add_worktree_dismissal.rs::outcomes_with_the_form_open_raise_no_notification` |
| U11 | Cancel on an `Editing` form, then a stray `CreateFailed`: no notification — nothing was running | FR-010b (boundary: only a create cancelled mid-flight is reported) | example | DONE | `crates/micold-client/tests/add_worktree_dismissal.rs::a_stray_failure_after_an_idle_form_was_cancelled_raises_no_notification` |

### `crates/micold-client/src/shell/daemon_sync.rs` — interrupted create with no form

| id  | behavior | traces | kind | state | test |
| --- | -------- | ------ | ---- | ----- | ---- |
| U12 | A disconnect while a create is pending and the form is closed raises a notification naming the worktree | FR-010b (interrupted case) | example | DONE | `crates/micold-client/src/main_tests.rs::a_create_with_no_form_open_is_still_reported` (010 BUG-020) |

## Invariants and edge cases still to place

- None.

## Out of scope

- Aborting the create from the dialog: FR-010b says Cancel closes the overlay only; an abort needs
  its own rollback design (`bugs/BUG-001.md`, Patch Record).
- Reopening the form while an earlier cancelled create is still running: the daemon's outcome
  messages carry no operation identity past `daemon_sync`, so the reopened form can absorb the
  earlier create's result. This was already reachable through a scrim click before BUG-001; it is
  a follow-up, not part of this fix.
- Relabelling Cancel while a create runs: decided against by the user (ledger D3); the button stays
  "Cancel".

## Verification commands

Copied verbatim from `.specify/memory/tdd-profile.md` at planning time:

- Single test: `scripts/build-lock.sh cargo test --test {file} {name} -- --exact` (assert on the
  `N passed` count — a filter matching nothing exits 0)
- Single test in `src/`: `scripts/build-lock.sh cargo test -p {crate} --lib {module}::tests::{name} -- --exact`
- File: `scripts/build-lock.sh cargo test --test {file}`
- Full suite: `scripts/build-lock.sh cargo test --workspace`
- Fast subset: `scripts/build-lock.sh cargo test -p micold-core --all-targets`
- Coverage: none (no `cargo-llvm-cov`)
- Mutation: none (no `cargo-mutants`) — deliberate-mutant spot checks by hand
