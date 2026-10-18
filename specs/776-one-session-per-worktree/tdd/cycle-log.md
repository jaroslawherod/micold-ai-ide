# Cycle log: 776 one session per worktree

**This is a retroactive reconstruction, not a live red-green-refactor.** The milestones were written source and tests together (commits marked "wip ... uncompiled"). For each milestone a detached worktree was made at the parent of the milestone's first source commit; the `(776)` test files (`tests/` only) were overlaid from the milestone's last commit; the task's tests were run with `scripts/build-lock.sh cargo test -p <crate> --test <file>` from inside that worktree. Red for the right reason = compiled against the old source and failed an assertion. No green or refactor step was redone; the green runs are the milestones' own gates.

Inline tests (`#[cfg(test)]` in `occupancy.rs`) cannot be overlaid without the implementation in the same file, so they are TEST_AFTER by construction.

## M1 (base 18616a32b, parent of 952e6e159; tests from 9f9c98148)

Shared compile blocker, `micold-core` tests and `micold-client` unit tests: `occupancy` module, `SessionDefinition`, `WorktreeCreate.session`, `WorktreeCreated.session` do not exist.

- **T002 TEST_AFTER.** `cargo test -p micold-core --test protocol_roundtrip` -> `error[E0432]: unresolved import micold_core::occupancy`; `error[E0432]: unresolved import micold_core::protocol::messages::SessionDefinition`; `error[E0559]: variant ClientMsg::WorktreeCreate has no field named session`. 952e6e159 landed source and tests together; the test does not compile against the parent.
- **T003 TEST_AFTER.** `crates/micold-core/src/occupancy.rs` does not exist at the base (same E0432). Tests are inline with the implementation.
- **T012 TEST_AFTER.** `cargo test -p micold-client --bin micold-ai-ide a_refused_session_create` -> `error[E0559]: variant OperationResult::WorktreeCreated has no field named session` (main_tests.rs does not compile).
- **T006 RED.** `cargo test -p micold-daemon --test one_session_per_worktree` -> 1 passed, 4 failed. `a_second_session_in_a_worktree_is_refused_when_the_first_is_running` panicked: `a second session is refused: OperationOk { req: 1, result: SessionCreated { ... } }`; `a_second_session_in_the_project_root_is_refused`: `assertion failed: matches!(answer(...), DaemonMsg::OperationError ...)`; the idle case and `a_hundred_concurrent_creates_...` failed too. The archived-only case passes (already true at base).
- **T007 RED.** `cargo test -p micold-daemon --test mcp_create_session` -> 19 passed, 1 failed: `a_second_session_in_a_worktree_is_a_conflict_naming_the_existing_one`: `create_session succeeded: {"lifecycle":"running", ...}` (support/mcp.rs:210). `cargo test -p micold-core --test mcp_tools_catalog` -> 41 passed, 1 failed: `create_session_says_a_worktree_has_at_most_one_session` (mcp_tools_catalog.rs:452).
- **T008 RED.** `cargo test -p micold-daemon --test review_send r10` -> `assertion left == right failed: the ended session is started again (R10)`; left: a new SessionId, right: `SessionId(...0001)`.

## M2 (base 60e0ac30a, parent of df57188e6; tests from 6c3f7f398)

- **T037 TEST_AFTER.** `cargo test -p micold-core --test protocol_roundtrip` -> `error[E0559]: variant ClientMsg::SessionStart has no field named allow_additional_session` (also `SessionRestart`, `AttachItem::Session`). df57188e6 is one uncompiled commit with source and tests.
- **T038 TEST_AFTER.** Inline tests in `occupancy.rs`, same file as the implementation.
- **T039 TEST_AFTER.** `cargo test -p micold-daemon --test one_session_per_worktree` -> `error[E0559]: variant ClientMsg::SessionCreate has no field named allow_additional_session`.
- **T040 RED (partial).** `cargo test -p micold-daemon --test mcp_create_session` -> 20 passed, 2 failed. `the_flag_creates_a_session_beside_the_existing_one` (line 523): reply is `{"category":"conflict","message":"this worktree already has a session ..."}`; `the_flag_does_not_skip_the_cli_check_and_must_be_a_boolean` (line 569): `{"category":"invalid_input","message":"create_session has no argument \"allow_additional_session\""}`, expected `service_error`. The catalog half (`cargo test -p micold-core --test mcp_tools_catalog`) does not compile: `error[E0432]: unresolved import micold_core::occupancy::Additional`; `error[E0026]: variant CreateSession does not have a field named additional`.

## M3 (base 9ff950c7d, parent of 8946eb0b9; tests from 5ac2e0a26, wire v44)

- **T014 RED (partial).** `cargo test -p micold-daemon --test mcp_create_session` -> 22 passed, 5 failed: `create_worktree_with_a_definition_creates_the_worktree_and_its_session`, `a_prompt_alone_takes_the_default_cli`, `a_missing_cli_creates_no_worktree_and_no_session`, `a_default_caller_creates_the_worktree_and_its_session_in_it`, `a_second_session_in_a_worktree_is_a_conflict...`. First: `create_worktree failed: invalid_input: create_worktree has no argument "ai_cli"` (support/mcp.rs:203). The RPC half (`one_session_per_worktree.rs::worktree_create_*`) does not compile, see T051.
- **T015 TEST_AFTER.** `cargo test -p micold-core --test mcp_tools_catalog` -> `error[E0432]: unresolved import micold_core::mcp::tools::StartSession`; `error[E0559]: variant CreateWorktreeRequest::Literal has no field named start`. The file at the milestone's end does not compile against the base.
- **T050 TEST_AFTER.** `cargo test -p micold-core --test protocol_roundtrip` -> `error[E0560]: struct SessionSummary has no field named main`; `error[E0599]: no variant named SessionSetMain found for enum ClientMsg`. Inline `occupancy.rs` tests: same file as the implementation.
- **T051 TEST_AFTER.** `cargo test -p micold-daemon --test one_session_per_worktree` -> `error[E0609]: no field main on type SessionSummary`; `error[E0599]: no variant named SessionSetMain found for enum ClientMsg`.

## M4 (base 0d0655eb3, parent of 64d57e3b7; tests from bb0458f6b)

- **T017 TEST_AFTER.** `cargo test -p micold-client --test features_worktree_form` -> `error[E0432]: unresolved import micold_client::features::worktree_form::SessionChoice`; `error[E0560]: struct WorktreeForm has no field named session_cli` / `first_prompt`; `error[E0599]: no method named session_definition`.
- **T018 RED (partial).** `cargo test -p micold-client --bin micold-ai-ide a_created_worktrees_returned_session` (main_tests.rs from 654bf57a2) -> `tests::a_created_worktrees_returned_session_is_selected` panicked: `the worktree's own session is the one in front of the user; left: None; right: Some(SessionId(...))`. The `start_intent` half (`cargo test -p micold-client --test features_session`) does not compile: `error[E0432]: unresolved import ...worktree_form::SessionChoice`; `error[E0599]: no method named start_offered found for struct State`.
- **T043 TEST_AFTER.** `cargo test -p micold-client --test features_sidebar` -> `error[E0599]: no method named has_expander found for struct WorktreeNode` (also `sessions_shown`, `main_session`).

## M5 (base 0c7d6f584, parent of 817d05fb8; tests from 2a1e24637)

- **T046 TEST_AFTER.** `cargo test -p micold-client --test features_session` -> `error[E0599]: no variant ... named AdditionalCancelled found for enum features::session::Msg`; `error[E0609]: no field additional_target on type State`; `error[E0599]: no method named additional_request`. 817d05fb8 landed 8 test files and 7 source files in one commit.
- **T055 TEST_AFTER.** `cargo test -p micold-client --test features_sidebar` -> `error[E0432]: unresolved import micold_client::ui::additional_session_item`; `error[E0603]: function session_menu_items is private`; `error[E0599]: no variant ... MakeMainRequested found for enum Msg`.

## M6 (base a09e52379, parent of 7ef1db4ae; tests from 133f274ee)

- **T023 RED (strengthened test).** As merged, `start_stop_and_restart_keep_the_one_session` passed on the pre-change parent too (20 passed): the stop/start/restart steps held already (FR-008a, FR-009), and its restart step could not fail (the process was live before it). The test was strengthened in the 776 follow-ups chore: stop, `SessionRestart`, wait for a live process that is not the one before the stop (`Arc::ptr_eq`). Red: with the `SessionRestart` arm of `server.rs` made a no-op (`if false && !refuse_blocked_start(..)`), `cargo test -p micold-daemon --test one_session_per_worktree start_stop_and_restart` -> `the restart brings the session back in a new process: Elapsed(())`, 1 failed. Green with the arm restored.
- **T024 TEST_AFTER.** `cargo test -p micold-daemon --test attach_apply` -> `error[E0599]: no method named replace_session found for struct Catalog` (attach_apply.rs:576).
- **T025 RED (partial).** `cargo test -p micold-daemon --test catalog_adoption` -> 18 passed, 1 failed: `adoption_takes_at_most_one_session_per_location_and_none_where_one_exists` (line 785): `only the first session of the free location; left: 3; right: 1`. The core half (`cargo test -p micold-core --test attach_discovery`) does not compile: `error[E0432]: unresolved import micold_core::attach::cap_per_location`; `error[E0277]: the trait bound AttachTarget: Ord is not satisfied`.
- **T027 TEST_AFTER.** `cargo test -p micold-client --test attach_dialog` -> `error[E0609]: no field confirm on type &features::attach::Dialog`; `error[E0599]: no variant ... ConfirmReplace found for enum attach::Msg` (also `ConfirmKeepBoth`).

## M7 (base edae87f2b, parent of d5c9c978b; tests from 362a4209c)

- **T031 TEST_AFTER.** `cargo test -p micold-daemon --test one_session_per_worktree` -> `error[E0599]: no method named start_refusal found for struct Arc<DaemonState>`; `error[E0061]: this method takes 2 arguments but 3 arguments were supplied`. d5c9c978b is "WIP, uncompiled".
- **T032 TEST_AFTER.** `cargo test -p micold-client --test features_session` -> `error[E0425]: cannot find function start_blocked in module features::session`; `error[E0599]: no method named session_start_block_text found for struct State`. Catalog half: `cargo test -p micold-core --test mcp_tools_catalog` -> `error[E0026]: variant Operation::StartSession does not have a field named additional`.
