# Autopilot ledger — #776 one-session-per-worktree

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 776 — "One session per worktree, chosen at start" (issue #776, verbatim in the issue); labels: enhancement, in-progress, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #776
- **Worktree branch**: feat/776_one-session-per-worktree-chosen-at-start
- **Started**: 2026-10-12
- **Phase**: milestone M1 (implementing, handed over)
- **Next step**: finish M1 per Handover

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #791 | Design (spec, plan, tasks) | merged | 6b60f3201a2e4dacdfc8d7d626ea55384eb959a0 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T013 | full | Second session refused by daemon (RPC, create_session, review send); wire fields + v41 | | in progress |
| M2 | T014–T016, T021 | full | create_worktree / WorktreeCreate define the session at start | | pending |
| M3 | T017–T020, T022, T036 | full | Dialog CLI/prompt fields; no add action on occupied rows | | pending |
| M4 | T023–T030 | full | Resume replaces with confirmation; attach cap | | pending |
| M5 | T031–T035 | full | Legacy catalogs intact; start refused while another is live | | pending |

## Decisions

- Tasks: optional hooks (docguard review, tdd plan) skipped; test tasks are ordered first by hand. All wire additions and the v41 bump land in M1 so later milestones need no second bump. Foundational+US1 server half is 13 tasks; kept whole (no split gives an observable deliverable earlier). Polish has no code: quickstart results are left to the close unit.

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | aa9ff7d5e93d7636fe58e91e03abeb878a2c199b:13165b3a9e4938a42528aae5df288475aa63ddae | CHANGES: 3 MAJOR (fixed) |
| Spec | 2 | 9b306f1b27f7cb5bce4eeae32d4c6e9eee1ae183:13165b3a9e4938a42528aae5df288475aa63ddae | CHANGES: 1 MAJOR (fixed) |
| Spec | 3 | 31cbda77df6487d072050ff54a55b6de41868a3e:13165b3a9e4938a42528aae5df288475aa63ddae | CLEAN (1 MINOR: agent-initiated provider resume, left to clarify/plan) |
| Plan | 1 | a0ffca5dedb4e146bf5ab61d75fca3fcbc39b919:6a7d599bd7d6d380ce4c8afc405b01095c336d91 | CHANGES: 2 MAJOR, 3 MINOR (all fixed) |
| Plan | 2 | 7a0314a89407314e315a9bc5c4df676b7896a95f:6a7d599bd7d6d380ce4c8afc405b01095c336d91 | CHANGES: 1 MAJOR (fixed), 2 MINOR (fixed) |
| Plan | 3 | f7336d39da2163aeb5245509456a3a91207ec911:6a7d599bd7d6d380ce4c8afc405b01095c336d91 | CLEAN (2 MINOR, fixed, prose only) |
| Tasks | 1 | 88f01682a9aa66bc17d3ab75057ec889afc7f777:0b295551eba2928d4ff1de381dbd709c9c9e17f3 | CLEAN (3 MINOR, fixed; analyze 5 MEDIUM fixed or spec prose clarified) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

M1 unit 1 hit the 150k cap mid-implementation. Branch is reset to origin/main (branch-start done, #791 merged) and the WIP is committed locally, NOT pushed, no PR. NOTHING HAS BEEN COMPILED OR RUN YET: the shared build lock was held by another worktree the whole time.

Written (uncompiled), per task:
- T001/T003/T004 `crates/micold-core/src/occupancy.rs` (+ `pub mod occupancy;` in lib.rs): ExistingSession, Occupancy, StartBlock, occupancy, start_block, refusal_text(existing, &location), start_block_text, with unit tests.
- T002/T005 wire: `SessionDefinition::new` + `WorktreeCreate.session`, `WorktreeCreated.session` (messages.rs); `AttachItem::Session{id,replace}`, `RefuseReason::LocationHasSession`, `AttachResult.existing` (attach.rs); PROTOCOL_VERSION 41 + history (version.rs); roundtrip tests in `tests/protocol_roundtrip.rs`. Struct literals in daemon/client src and tests patched (script; check clippy/rustfmt, and `cargo fmt --all` already ran once).
- T009/T010 daemon: `catalog.rs` `CreateSessionError::{Occupied(ExistingSession, SessionLocation), Io}` (+ Display, From<io::Error>, `location_of`, `Catalog::occupancy`), `state.rs` create_session/occupancy passthrough, `ops.rs` `create_session_with_prompt` -> `Result<_, CreateSessionError>` and new `start_with_prompt`, `server.rs` SessionCreate -> `OperationError{Refused, refusal_text, detail: Some(existing id)}`, `mcp/tools.rs` early Conflict check (after policy, before CLI check) + in-lock mapping via `occupied()`; core `mcp/tools.rs` description sentence.
- T011 `state.rs` `review_send_to_new_session`: reuses the entry's own session (LaunchMode::Resume via `ops::start_with_prompt`), else creates.
- T012 client: `shell/daemon_sync.rs` shows the refusal verbatim for `PendingOp::CreateSession`; test `a_refused_session_create_shows_the_refusal_text_and_changes_nothing` in `src/main_tests.rs` (placed there, not features_session.rs, because the dispatch is in the shell binary).
- T007 tests: `mcp_create_session.rs::a_second_session_in_a_worktree_is_a_conflict_naming_the_existing_one`, `mcp_tools_catalog.rs::create_session_says_a_worktree_has_at_most_one_session`.

Still to do for M1:
1. `crates/micold-daemon/tests/review_send.rs` (T008): the python edit failed, nothing applied. Needed: stand-in `claude` script must parse `--resume` as well as `--session-id` (sid var); `started_session` helper currently asserts "not a known id" -- make it plain and add `started_new_session` (asserts new) for the Default-entry test (line ~732) ; tests on entry `wt` (sandbox has sid(1), sid(2) both at wt, idle) now resume sid(1): update `us3_s1_with_no_session_running_one_starts...` (expect sid(1), sessions_for len stays 3) and rewrite `us3_s4_an_ended_session_is_not_resumed_and_a_new_one_starts` as R10 (ended session started again, no second one); check lines ~591, 696, 770, 918 callers (r5 / fr018 / fr021 tests use started_session too).
2. T006: create `crates/micold-daemon/tests/one_session_per_worktree.rs` (pattern: `busy_session_start.rs` with `support/conn.rs` `connect`, or `support/mcp.rs` `state_over`/`session`): second SessionCreate refused for Idle and Running (use `catalog.mark_session_running` before DaemonState::new), catalog unchanged, Default root refused, 100 concurrent SessionCreate on an empty worktree -> exactly one non-Refused (99 `ErrorKind::Refused`) and one session in the catalog, archived-only worktree accepts one. Kill any started sessions at the end.
3. T013 docs: `docs/user-guide/agent-tools.md` (create_session conflict naming the existing session; at most one session) and `docs/user-guide/worktrees-and-sessions.md` (one session per worktree; refusal names the existing one).
4. Compile and fix: `mise run test-core`, then daemon/client tests (`scripts/build-lock.sh cargo test -p micold-daemon -p micold-client`). Expect existing tests that create two sessions in one location through `Catalog::create_session`/`create_session` (mcp_create_session.rs helpers, session_start.rs, session_survival.rs etc.) to now fail: move them to distinct locations. `tests/schema_hash.rs` may need nothing. rustfmt/clippy -D warnings. Tick T001-T013 in tasks.md once each is done (none ticked yet).
5. Then verify.md, review.md (A high + B), gate (full `mise run gate`), PR (pr.md) with title `feat(776): refuse a second session in a worktree (#776)`.

Notes: TDD red phase was not recorded (tests and implementation were written together; no tdd/ dir exists since the tdd plan hook was skipped). Review rounds: none yet.

## Open escalation

None.

## Follow-ups not done

None yet.
