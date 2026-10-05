# TDD cycle log: 582 attach provider worktrees and sessions

No `tdd/test-list.md` exists for this feature (the design phase did not run `tdd.plan`); the
test-first tasks of tasks.md (T003 before T004, T005 before T006, T007 before T010, T008 before
T011) served as the list. Red evidence below is the real failure of each test before its code.

## M1

- **T003/T004** `micold-core/tests/protocol_roundtrip.rs` (`every_client_message_json_round_trips`,
  `every_daemon_message_json_round_trips`). Red: `cargo test -p micold-core --test protocol_roundtrip`
  -> `error[E0599]: no variant named AttachDiscover found for enum ClientMsg`. Green: messages added,
  PROTOCOL_VERSION 27 -> 28, `schema_hash.rs` constant moved to 28 (hash is recomputed, not pinned).
- **T007/T010** `micold-core/tests/attach_discovery.rs`. Red: `unresolved import micold_core::attach::attachable_worktrees`.
  Green: 6 passed.
- **T005/T006** `micold-daemon/tests/attach_apply.rs` (catalog level). Red: `no method named attach_worktrees found for struct Catalog`.
  Green: 5 passed.
- **T008/T011** same file (daemon level). Written with the T011 handlers not yet present
  (`AttachDiscover` unhandled); green after `state.attach_discover/attach_apply` and the server arms: 9 passed.
- **T009/T012** `micold-client/tests/attach_dialog.rs`. The reducer and its tests were written in one
  pass, so there is no recorded red for these; the tests were then checked to fail on the specific
  behaviour only by inspection (summary strings, in-flight guard, project guard). Recorded as a gap.
  Green: 13 passed, plus `features_attach.rs` for the isolation guard.

## M2

- **T017/T022** `micold-core/tests/attach_discovery.rs` (`mod resumable`). Red: `unresolved import
  micold_core::attach::discover_resumable`. Green: `discover_resumable(&[StoreView], &DiscoverInput)`.
- **T018/T021** `micold-core/tests/provider_store_dirs.rs`. Red: `no method named store_dirs found`.
  Green: `AiCliProvider::store_dirs` (default: root and listed worktrees; Claude also lists store
  directories under the managed worktree folder) and `last_activity`.
  Deviations from the plan: `store_dirs` takes a third argument `worktrees: &[PathBuf]` and returns
  notes with the directories; `last_activity` is a new trait method; `DiscoverInput.only` lets an
  apply look up one id without reading every title.
- **T019/T023** `micold-daemon/tests/attach_apply.rs`. Red: `no method named attach_session found
  for struct Catalog`, then (state level) the discovery and session-apply tests failed with an empty
  `sessions` list. Green: 17 passed. Two tests seed the transcript after the client attaches,
  because feature 026's adoption pass (kept, plan P3) takes sessions of the root and of startable
  worktrees into the catalog on attach, which correctly removes them from the resumable list.
- **T020/T024** `micold-client/tests/attach_dialog.rs`. Red: `no variant named Resume found for enum
  Msg`, `no method footer_notes`, `no field sessions` (16 compile errors). Green: 21 passed.
- T023 deviation: the "already running" refusal lives in `Catalog::attach_session`, not in `start_session_gated` (which stays an idempotent start).
- Wire: `RefuseReason::AlreadyRunning` was added. `PROTOCOL_VERSION` stays 28 (M1 already moved it for
  this feature and the schema hash is recomputed); `schema_hash` and `handshake` tests pass.

## M3

- **T027/T029** `micold-core/tests/mcp_tools_catalog.rs`, `mcp_policy.rs`. Red: `no variant named
  AttachWorktree found for enum Operation` (and `ListResumableSessions`), 2 errors per test binary.
  Green: catalog 33 passed, policy 20 passed (`Operation::{AttachWorktree, ListResumableSessions}`,
  `PRINCIPLE_III_ATTACH` arm beside rename/delete).
- **T028/T030** `micold-daemon/tests/attach_mcp.rs`. Written with the handlers absent: the daemon
  crate failed to compile (`non-exhaustive patterns: Operation::AttachWorktree`), the red for the
  whole file. Green: 7 passed. Two expectations were corrected after the first run, both against the
  fixture and not the code: the caller's own worktree `b` is already attached, so the by-path test
  uses `c`; a worktree outside the managed directory is not in the project's list unless the user
  included it, so it is `not_found` (contract amended).
- Deviations: `provider` is `claude_code` (the `tool_name` every other tool uses), `total` counts at
  most the 200 newest sessions discovery returns.
- mcp_audit_log.rs enumerates every mutating tool: added attach_worktree success and failure calls (found by the full gate).
- mcp_binding_spawn.rs (catalog list) and mcp_read_latency.rs (SC-004 timing, `list_resumable_sessions` with no arguments) enumerate tools: updated for the two new ones (found by the full gate).

## M4

- **T032/T033** `micold-client/tests/attach_offer.rs` (15 tests). Red: `unresolved import offer_visible`,
  `no variant OfferListed/OfferDismissed/OfferAttachAll/OfferApplied/OfferApplyFailed`, `no field
  offer` (21 compile errors). Green: 15 passed. `no_records` reads the mirrored provenance records
  and `unreadable_projects`, never catalog sessions (026 adoption).
- T034: grep of `ui/material/` found `ConnectionBanner` (title, detail, level, one action) and a
  snackbar (transient; `banner_is_not_a_snackbar.rs` forbids folding). Reused the banner; added
  `.secondary_action` (a text button, actions move under the text) and a showcase specimen.
- Deviation: "Attach all" attaches the attachable worktrees (as the dialog's does); stored sessions
  are resumed one at a time from the dialog (resuming starts a process, not a bulk action at
  start-up). With only sessions found the button reads "Review" and opens the dialog.

## T042 remediation, daemon half (T008/T011 red, recorded after the fact)

- Stub: `DaemonState::attach_discover` and `attach_apply` (`crates/micold-daemon/src/state.rs`) bodies
  replaced by `todo!("red stub")`; restored with `git checkout -- crates/micold-daemon/src/state.rs`.
- Command: `MICOLD_NO_BUILD_LOCK=1 cargo test -p micold-daemon --test attach_apply`. The run did not
  finish: tests that wait for the daemon's answer hang (the `AttachDiscover` handler panics inside
  `spawn_blocking`, so no `AttachReport` is sent). Each test was therefore run alone with `timeout 20`
  on the test binary.
- Result: `a_sandboxed_daemon_reports_the_store_is_not_readable` FAILED (`not yet implemented: red stub`).
  `attaching_again_reports_already_attached`, `attaching_changes_nothing_in_the_worktree` and
  `a_session_of_a_deleted_worktree_is_refused_not_resumed_elsewhere` failed with `OperationError { kind:
  IoFailed, message: "failed to persist the attach", detail: "task 5 panicked with message \"not yet
  implemented: red stub\"" }`. `attached_worktrees_survive_a_restart_and_show_with_the_filter_off`,
  `discovery_lists_this_projects_sessions_and_attaching_one_adds_an_idle_entry`,
  `resuming_a_session_in_an_unattached_worktree_attaches_the_worktree_first` and
  `sixteen_worktrees_attach_in_one_action_within_ten_seconds` hung (timeout). The catalog-level tests pass
  (they do not touch the handlers).
- After restore: 23 passed.
- T041: new `a_worktree_outside_the_managed_directory_is_refused_and_unrecorded` fails with
  `&& !w.included` removed at `catalog.rs:526`.
- T045: the guard split into one test per state (`Starting`, `Running`, `Restarting` refused; `Idle`,
  `Failed` already attached). With the guard changed to `matches!(.., Running)`,
  `a_second_resume_of_a_starting_session...` and `..._restarting_session...` fail.

## Phase 8 (client): T042 reducer red, T039/T043/T049

- **T042 (reducer, T009/T012).** Red: `features/attach.rs::update` stubbed to `Vec::new()` (original kept
  as `update_real`), then `MICOLD_NO_BUILD_LOCK=1 cargo test -p micold-client --test attach_dialog
  --no-fail-fast`. Result: `test result: FAILED. 3 passed; 18 failed`, among them
  `opening_waits_for_the_report_then_lists_it`, `attach_selected_sends_only_the_ticked_rows`,
  `attach_all_sends_every_attachable_row_and_not_the_missing_one`, `resume_sends_one_session_target`,
  `an_applied_batch_closes_the_dialog_and_says_what_happened`. Restored; `git diff --stat crates/*/src`
  shows no change to `features/attach.rs`.
- **T039** `daemon_sync.rs` tests `a_resume_answered_attached_or_already_attached_starts_that_session`,
  `a_resume_refused_as_already_running_starts_nothing`. Fail with `view_and_start(...)` replaced by
  `let _ = id;` (first), and with the outcome match widened to `_` (second).
- **T043** `opening_a_project_asks_for_the_offer_and_the_report_reaches_it`,
  `the_dialog_apply_answer_routes_to_applied`, `the_banner_apply_answer_routes_to_offer_applied`. Each
  fails with its arm removed (`if false` guard on the `AttachApply`, `AttachOfferApply`,
  `AttachOfferDiscover` arms); the first also fails with `request_attach_offer` made a no-op.
- **T049** `attach_offer.rs` now asserts message and `Level` of both notifications;
  `attach_dialog.rs::a_refusal_is_reported_as_an_error` asserts `Level::Error` through the reducer.
  Fail with the failure text changed, `OfferApplyFailed` level `Info`, `summary_level` refusal ->
  `Info`, and the summary text changed. (The `attach_discovery.rs:316` half belongs to the core worker.)
