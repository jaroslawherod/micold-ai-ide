---

description: "Task list for 802 soft session limit confirmation"
---

# Tasks: Soft Session Limit - Confirm Before a Parallel Session

**Input**: `specs/802-soft-limit-confirm/` (spec, plan, research, data-model, contracts, quickstart)
**Tests**: mandatory (Constitution I); each test task comes before the code it covers.

## Phase 1: Setup

- [X] T001 Read `crates/micold-client/src/main_tests.rs` (`a_refused_session_create_shows_the_refusal_text_and_changes_nothing`, ~1426) and `crates/micold-client/src/shell/daemon_sync.rs` (`on_session_start_requested` ~2516, `PendingOp` ~84, matches at ~232, ~1237, ~1735) to pick the test harness and the reducer-decision seam (no file change).

## Phase 2: Foundational

- [X] T002 Add `additional_provider: Option<AiCli>` to `SessionState` in `crates/micold-client/src/features/session.rs`, cleared with `additional_target` on confirm, cancel and `clear_for_dialog`; `additional_requested` keeps the provider; `additional_request()` uses it (`None` uses `provider_for_start(None)`). Test first in `crates/micold-client/tests/additional_session_provider.rs`: requested keeps the provider; decline clears target and provider.
- [X] T003 Change `PendingOp::CreateSession` to `{ location: SessionLocation, provider: AiCli, additional: bool, automatic: bool }` in `crates/micold-client/src/shell/daemon_sync.rs` and update the matches (~232, ~1237, ~1735) and `crates/micold-client/src/main.rs` (~351); add `automatic: bool` to `SessionMsg::StartRequested` in `crates/micold-client/src/features/session.rs`, set by the startup auto-start in `main.rs` (~355) and handled at `main.rs` ~873. Behaviour unchanged; existing tests stay green.

## Phase 3: User Story 1 - Creating where one exists asks, it does not refuse (P1) MVP

**Independent test**: a start request for an occupied location opens the confirmation and sends nothing; a stale `Refused` reply opens it.

- [X] T004 [US1] Tests first, failing, in `crates/micold-client/src/main_tests.rs`: (a) start for an occupied location opens the confirmation and sends no `SessionCreate`; (b) `Refused` with a session-id `detail` and `additional == false` opens it, no error notice; (c) other `Refused` replies and an `automatic` refusal still `notify_error`; (d) confirm sends exactly one `SessionCreate { allow_additional_session: true }` with the chosen CLI; cancel sends nothing and the catalog is unchanged; (e) blocked start and row entry still open it (regression); (f) with several sessions the dialog names the main one. Rewrite `a_refused_session_create_shows_the_refusal_text_and_changes_nothing` to the new behaviour.
- [X] T005 [P] [US1] Test first in `crates/micold-client/tests/confirm_additional_session_generic.rs`: the dialog draws generic text ("already has a session") when the catalog lacks the occupant.
- [X] T006 [US1] Pre-check: put the open-or-send decision in a reducer function in `crates/micold-client/src/features/session.rs` (`State::start_offered(location)` false opens the confirmation with the chosen provider); make `on_session_start_requested` in `crates/micold-client/src/shell/daemon_sync.rs` a thin dispatcher over it.
- [X] T007 [US1] Refusal fallback: in the `PendingOp::CreateSession` error arm of `crates/micold-client/src/shell/daemon_sync.rs` recognise `ErrorKind::Refused` with a session-id `detail` (typed fields, not message text) for `additional == false && !automatic` and open the confirmation with the stored location and provider; otherwise `notify_error`.
- [X] T008 [P] [US1] Generic text when the occupant is absent from the catalog in `crates/micold-client/src/ui/confirm_additional_session.rs`.
- [X] T009 [US1] User guide: person is asked to confirm a second session, in `docs/user-guide/worktrees-and-sessions.md`.

## Phase 4: User Story 2 - Both sessions run in parallel after confirming (P1)

**Independent test**: after confirm, two un-archived sessions, original still main (776 behaviour, regression).

- [X] T010 [US2] Regression test in `crates/micold-client/src/main_tests.rs` (or `crates/micold-client/tests/`): after the confirmed flagged create, the catalog lists both sessions with the original main and the new one selected; stopping or deleting one leaves the other. Passes with T006-T007; if it fails, fix in `crates/micold-client/src/shell/daemon_sync.rs`.

## Phase 5: User Story 3 - Agent create has a documented rule (P2)

**Independent test**: `mcp_tools_catalog.rs` pins the new description text; daemon refusal tests unchanged.

- [x] T011 [US3] Test first (observed failing before T012): extend `crates/micold-core/tests/mcp_tools_catalog.rs` to pin that `create_session` and `start_session` descriptions say a person is asked to confirm and an agent is refused unless `allow_additional_session: true`, and why.
- [x] T012 [US3] Update the `create_session` and `start_session` descriptions in `crates/micold-core/src/mcp/tools.rs`.
- [x] T013 [P] [US3] Document the agent rule and its reason in `docs/user-guide/agent-tools.md`.
- [x] T014 [US3] Confirm no daemon change: run `crates/micold-daemon` `mcp_create_session.rs` refusal and flagged-create tests unchanged (FR-005, FR-007). The delete-while-open and disconnected-while-open edge cases rest on those existing 776 daemon tests and T004(d); no new test.

## Dependencies

T001 -> T002 -> T003 -> T004/T005 -> T006/T007/T008 -> T009, T010. US3 (T011-T014) has no code dependency on US1; M2 follows M1 for review size only.

## Implementation Strategy

M1 = Phase 1-4 (the client behaviour, its tests and the worktrees guide). M2 = US3 (descriptions and agent guide). Quickstart §B is manual and optional; the close unit records it.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Person start on an occupied location asks to confirm 🎯 MVP

- **Tasks**: T001–T010
- **Deliverable**: Starting a session in a worktree that already has one opens the parallel-session confirmation (also on a stale refusal) instead of an error; confirm creates the second session with the chosen CLI, cancel creates nothing.
- **Satisfies**: US1 acceptance scenarios 1–4; US2 acceptance scenarios 1–2; FR-001, FR-002, FR-003, FR-004, FR-007
- **Verify**: `mise run test` (the new `main_tests.rs` cases and `additional_session_provider`, `confirm_additional_session_generic` tests); `mise run gate`
- **Depends on**: —
- **Tier**: full

### M2 — Agent rule documented

- **Tasks**: T011–T014
- **Deliverable**: `create_session`/`start_session` descriptions and the agent-tools guide state the confirm-for-person, refuse-for-agent rule and why.
- **Satisfies**: US3 acceptance scenarios 1–3; FR-005, FR-006
- **Verify**: `mise run test-core` (`mcp_tools_catalog.rs`); daemon `mcp_create_session.rs` tests
- **Depends on**: M1
- **Tier**: light

## Phase 6: TDD remediation

Source: `tdd/verification.md` (verdict FAIL at audit time). T015-T021 cleared in the close PR; T022 resolved by recording the gap below.

- [x] T015 [US1] Finding 4: add a test in `crates/micold-client/src/main_tests.rs` that a `Refused` reply with a UUID `detail` to a flagged (`additional: true`) create stays a notice and opens no confirmation; it must fail when `&& !additional` is removed at `crates/micold-client/src/shell/daemon_sync.rs:1787`. Proof: `cargo test -p micold-client --bin micold-ai-ide main_tests::` with the mutant applied then restored.
- [x] T016 [US2] Finding 1: rewrite `after_confirming_both_sessions_are_listed_and_deleting_one_leaves_the_other` (`main_tests.rs:1687`) to drive the daemon's catalog push for the second session and a real delete instead of editing `workspace.sessions` in the test; proof: `cargo test -p micold-client --bin micold-ai-ide after_confirming_both`.
- [x] T017 [US1] Finding 2 (B17): make `with_several_sessions_the_confirmation_names_the_main_one` (`main_tests.rs:1672`) call `ui::confirm_additional_session::dialog` (or expose its text) and assert the main session's label appears and a helper's does not; it must fail with the occupant name replaced by a constant at `ui/confirm_additional_session.rs:77`.
- [x] T018 [US1] Finding 3: restore the notice-text value check in `other_refusals_and_an_automatic_refusal_stay_notices` (`main_tests.rs:1594`): `assert_eq!` of the visible notice to the service message, and split the two scenarios into two tests (finding 7).
- [x] T019 [US1] Finding 5: add cases for a non-`Refused` kind with a UUID `detail` and a `Refused` with a non-UUID `detail`, both staying notices (`daemon_sync.rs:1782-1785`).
- [x] T020 [US1] Finding 6: test that the startup auto-start sends `StartRequested { automatic: true }` (`main.rs:359`) and that an automatic start at an occupied location is not turned into the confirmation (`daemon_sync.rs:2593`).
- [x] T021 [US1] Findings 8 and 9: assert in `the_blocked_start_and_the_row_entry_still_open_the_confirmation` (`main_tests.rs:1644`) that the confirmation opens for the blocked start, and drop `a_free_location_offers_the_start` (`tests/additional_session_provider.rs:45`) as redundant.
- [x] T022 [US3] Finding 10: either add a test that an agent create on an occupied location produces no `additional_target` in any window state, or record in `spec.md` that this clause rests on the daemon tests; and record a cycle-log red for future cycles so the next audit can classify PROVEN. Resolved: the gap is recorded in `tdd/verification.md`; no cycle-log red is kept.
