---
feature: 1726-app-via-orchestrator
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against (no override or preset exists)
verified_at: 5811fa2 # short SHA audited
behaviors: 18 # test-bearing tasks; there is no tdd/test-list.md, so the unit is the task
proven: 3
likely: 1
test_after: 14
no_test: 0
high_smells: 1
criteria_total: 17 # acceptance scenarios US1.1-US5.2
criteria_covered: 14
criteria_partial: 3 # US1.2, US2.2, US4.3
mutation_score: unmeasured # skipped by instruction: disk-limited cloud
mutants_survived: unmeasured
suite: 283 passed, 0 failed, 1 ignored (targeted runs only, not the workspace suite; about 5 min including builds)
---

# TDD Verification: The desktop app talks only to the orchestrator (#1726)

**Verdict: FAIL.** 14 of 18 test-bearing tasks have no recorded red and a squashed history that cannot show ordering, so they are `TEST_AFTER` by the fail-closed rule; the cycle log itself admits it for M3 and has no M1 or M2 section at all.

Independence: this audit ran in a fresh session and did not write these tests. The smell pass was done directly, with no subagent. Every cited line was opened as it stands at `5811fa2`.

Rubric note: the feature has no `tdd/test-list.md` (it was not planned through `speckit-tdd-plan`). The rubric lists that as `BLOCKED`; the skill says to audit against `spec.md` anyway. Behaviors here are the tasks in `tasks.md` that carry a test, and `traces` are taken from the task text. The `FAIL` conditions below stand independently of that.

## Test-first evidence

History shape: M1 (`2f62cbf`), M2 (`5ee726f`) and M3 (`0deffa8`, merged as `f6aa5b8`) are one squashed commit each, tests and source together. M4 kept per-task commits (`c0456cd`, `60fe5c1`, `b5d5ab8`, `6c34270`), each adding a test and its source in the same commit.

| Behavior (task) | Class | Evidence |
| --- | --- | --- |
| T002 opaque form, T004a serde round trips, T005 `RouteTable`, T007 `merge_lists` (M1) | TEST_AFTER | `cycle-log.md` has no M1 section; one squashed commit `2f62cbf` holds test and source |
| T009 requests/streams/refusals, T012a link_drop/slow_client, T013 outage (M1) | TEST_AFTER | same; no red recorded |
| T014 lists, T016 bench (M2) | TEST_AFTER | no M2 section; squashed `5ee726f` |
| T017 single connection (M3) | TEST_AFTER | `cycle-log.md:5-11` "no failing run was captured before the implementation" |
| T019a reconnect (M3) | TEST_AFTER | `cycle-log.md:12`, same admission |
| T020a wire id in rendered config (M3) | TEST_AFTER | `cycle-log.md:13`, same admission |
| T029 route to provisioned daemon (M4) | TEST_AFTER | `cycle-log.md:29` "not captured (written with `routed_to`)" |
| T031 provisioned routing (M4) | TEST_AFTER | `cycle-log.md:25-26`: waived for most cases. One case (late refusal, `orchestrator_provisioned.rs:267-279`) has a real red, so that case alone is `LIKELY`; FR-009 re-route is waived (`:26`) |
| T023 `hold_link` argv | PROVEN | `cycle-log.md:22` compile-error red; test and source both in `c0456cd` |
| T025 holder step | PROVEN | `cycle-log.md:23` compile-error red; `c0456cd` |
| T027 `remote hold-link` | PROVEN | `cycle-log.md:28` assertion red; `60fe5c1` |
| T033 `ssh_real_link` routed case | LIKELY | `cycle-log.md:27` red in CI and locally; the order is not in git history (same commit `6c34270` as the fix); test needs `sshd`, not run here |

Counts: 3 `PROVEN`, 1 `LIKELY` (T033), 14 `TEST_AFTER` (T031's one real-red case is noted above but the task is counted `TEST_AFTER`), 0 `NO_TEST`.

Discrepancy between sources: none found where the log claims a red the history contradicts. The log's honesty is the problem, not its accuracy.

Existing tests changed by the feature (the highest-signal check):

- `crates/micold-daemon/tests/orchestrator/not_hosted.rs`: deleted `a_session_start_is_refused_not_hosted` and `a_session_list_is_empty`. Their behavior (the orchestrator refuses `SessionCreate` with "not hosted") is the thing T011 replaced by routing; the replacement is `refusals.rs` and `requests.rs`. Judged a legitimate behavior change, not a weakening. Residual: nothing now asserts what an orchestrator-addressed `Attach` without a daemon target returns.
- `crates/micold-client/tests/daemon_config_validate.rs:266-274`: `assert_eq!(back.file, file)` became equality with `wire_id` copied from the actual value, plus `assert!(back.file.wire_id.is_some())`. Presence only; the id's format is pinned elsewhere (`daemon_config_render.rs:498`). Not a weakening, noted.
- `protocol_roundtrip.rs:998` and `schema_hash.rs:352`: version constants 82 to 83, matching `version.rs`. Correct.
- Seven golden files and `fixtures/daemon_config_delivery_host.golden.toml` gained `[orchestrator_link] enabled = true`. Additive, matches T020a and `drift-M3.md`. Whether they were regenerated and accepted unread cannot be told from history; the diff is three lines each.
- No test was skipped, renamed out of a filter, or excluded by config. No coverage or mutation threshold was lowered.

`tasks.md` against evidence: every task is `[x]`. With no test list there is no `DONE` set to compare; the cycle log is the only record, and it covers T017, T019a, T020a, T023, T025, T027, T029-T033 only.

## Findings

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | HIGH | 14 behaviors have no red evidence (`TEST_AFTER`): M1 T002-T013, M2 T014/T016, M3 T017/T019a/T020a, M4 T029/T031. Nothing shows these tests could fail before the code | `tdd/cycle-log.md:5-13,24-29`; squashed `2f62cbf`, `5ee726f`, `0deffa8` |
| 2 | HIGH | The SC-003 benchmark is `#[ignore]` in the committed state, so no gate runs it; the budget is checked only by hand and recorded in a PR body. Catalogue severity is fixed (`Empty or always-skipped`); it is by design under T016, and the test has real assertions when run | `tests/orchestrator_route/bench.rs:83`; `pr-m2.md:22` |
| 3 | MED | `a_terminal_stream_arrives_through_the_relay` passes on any opaque frame for conn 1; it checks neither the terminal bytes nor their order. No test routes a structured-event or progress stream, though FR-004 and US2.2 name both. Order is shown only with `Ping`/`Pong` | `tests/orchestrator_route/streams.rs:110-119`, `:13-27` |
| 4 | MED | "Delivered to that daemon only" (FR-003, US2.1) is not asserted: the test accepts `OperationOk` or `OperationError` and never checks the other daemon received nothing | `tests/orchestrator_route/requests.rs:30-50` |
| 5 | MED | List reasons `needs-update` and `error: <message>` have no end-to-end test; only `offline` and `timeout` do. `list.rs:44` maps `NeedsUpdate` untested | `tests/orchestrator_route/lists.rs` (no case); `src/orchestrator/list.rs:44` |
| 6 | MED | One 90-line test runs seven scenarios in sequence on a process-wide orchestrator that it restarts midway (Eager test, order dependence). It also leaves uncovered: holder end then re-provision (only `held.ended()` is awaited), status-cell text, pod relay drop, and no-client-connection on the daemon socket. The last three are disclosed in `drift-M4.md:44-48`; the first is not | `tests/orchestrator_provisioned.rs:201-293`; T031 text |
| 7 | MED | US4.3 / FR-009 "the app re-reads the target after `OutcomeUnknown`" is covered only as route-end classification and a reconnect handshake, never as the actor re-reading. `drop(state)` after a comment about the daemon keeping its state asserts nothing | `src/orchestrator_conn.rs:458-480`; `tests/orchestrator_single_connection.rs:127` |
| 8 | MED | Fixed sleeps and wall-clock bounds: `sleep(500ms)` before linking, `sleep(200ms)` to show "still holding", `expect_quiet(300ms)` negative waits, and `< 2.5 s`, `< 5 s` limits that depend on machine load | `orchestrator_single_connection.rs:144`; `remote_hold_link.rs:34`; `requests.rs:53`; `lists.rs:101`; `slow_client.rs:41`; `link_full.rs:36` |
| 9 | LOW | Two harnesses build the same daemon config and `spawn_with` call (`config()`/`Daemon::link_state`, `link_local`/`link_as`); the `sh -c` stand-ins for a refused `-R` are copied between `provision/mod.rs` tests and `orchestrator_provisioned.rs` | `orchestrator_route/main.rs:52-83`; `orchestrator_world/mod.rs:131-143`; `orchestrator_single_connection.rs:21-33`; `provision/mod.rs:2124-2150` vs `orchestrator_provisioned.rs:107-135` |
| 10 | LOW | The T033 case returns silently where `sshd` is absent, so it reads as a pass in the cloud. This follows the `ssh_real` convention, so it is not counted as a HIGH, but it was not run here | `ssh_real/ssh_real_link.rs:750-756` |

Smell catalogue result: no assertion-free, tautological, doubled-subject, re-implemented-expectation, conditional-assert or self-approving-snapshot test found in the new files. The hub tests (`hub_queues.rs`) and the route and merge unit tests assert specific values; `assert_accounted` computes its expectation independently of the hub. Style matches the profile: `#[cfg(test)]` modules in source, integration files under `tests/`, shared harness via `#[path]`, hand-written fakes (`FakeDaemon`, `fake_host::welcoming_bridge`, `Fake` ssh runner). Assertions carry messages.

Properties: isolation is good except finding 6. Determinism: findings 8 and the real-process spawns in client tests (the orchestrator is a real child). Speed: the targeted suites run in under 15 s of test time each. Specificity: weak in finding 6.

## Mutation results

Not run. The instruction for this audit was to skip mutation and deliberate mutants (disk-limited cloud). Test strength is unmeasured: no mutant was applied, no score exists, no survivor was triaged. `mise run mutants-diff` is the profile's tool (`cargo-mutants`, not installed here).

## Traceability

Criteria are the acceptance scenarios of `spec.md`. "E2E" means through a real orchestrator and real or fake linked daemons.

| Criterion | Tests | End to end |
| --- | --- | --- |
| US1.1 union with refs | `lists::a_list_holds_every_daemons_rows...`, `list_merge` units | Yes |
| US1.2 named reasons | `lists` (offline, timeout), `list_merge` units | Partial: `needs-update`, `error` not E2E (finding 5) |
| US1.3 no daemon | `lists::no_daemon_linked_is_an_empty_list_not_an_error` | Yes |
| US2.1 request to owner only | `requests::*` | Yes, "only" not asserted (finding 4) |
| US2.2 streams in order | `streams::frames_arrive_in_the_order...`, `a_terminal_stream_arrives_through_the_relay`, `slow_client`, `link_full` | Partial: terminal content, structured and progress streams not asserted (finding 3) |
| US2.3 unknown ref `not_found` | `refusals::an_unknown_daemon_is_not_found` | Yes |
| US2.4 takeover seen by both | `streams::a_takeover_is_seen_by_both_clients` | Yes |
| US2.5 `daemon offline`, re-attach | `link_drop::*` | Yes |
| US3.1 start and connect | `orchestrator_single_connection::the_app_starts_the_orchestrator...` | Yes |
| US3.2 existing orchestrator reused | indirect: `welcome` calls after start; no case starts from a running one | Yes (indirect) |
| US3.3 two starts, one orchestrator | `two_concurrent_starts_leave_one_orchestrator` | Yes |
| US3.4 named start failure | `an_orchestrator_that_cannot_start_is_named` | Yes |
| US4.1 session survives outage | `outage::a_session_survives...`, `after_the_orchestrator_is_killed...` | Yes |
| US4.2 reconnect and re-read | `after_the_orchestrator_is_killed...` | Yes (handshake only; no list re-read asserted) |
| US4.3 `unknown`, then re-read | `link_drop`, `hub_queues`, `orchestrator_conn::tests` | Partial: the re-read is not tested (finding 7) |
| US5.1, US5.2 latency | `bench::bench_routed_p95...` | Yes, but `#[ignore]` (finding 2) |

Functional requirements beyond the scenarios: FR-007 provisioned (`orchestrator_provisioned`, `ssh_real_link`, `remote_hold_link`, `argv`, `provision` units), FR-011 (`client_routing_contract.rs`, 5 tests), FR-013 (`daemon_config_render` units, goldens), FR-010 (`outage`). SC-002 (existing client suites pass through the orchestrator) rests on T021's evidence in `drift-M3.md` and was not re-run.

Untested criteria: none outright; three partial (above). Tests tracing to nothing: none found. `traces` could not be checked against a list; each task's named test was confirmed to exist and pass in the targeted runs below, except `ssh_real_link` (needs `sshd`).

## What was not audited

- Mutation testing and deliberate mutants: skipped on instruction (disk-limited cloud). Test strength is unknown. This alone caps the verdict at `PASS_WITH_GAPS`, and `FAIL` stands on findings 1 and 2.
- The workspace suite was not run (`mise run gate` is CI's; a `--workspace` run has filled this disk). Targeted runs only: `micold-core --lib` for `orchestrator::`, `protocol::routing`, `ssh::argv`, `provision::` (241 passed); `micold-core --test main client_routing` (6 passed); `micold-daemon --test orchestrator_route` (19 passed, 1 ignored bench); `--test remote_hold_link` (1); `micold-client --test main orchestrator_single_connection::` (6); `--test orchestrator_provisioned` (1); `micold-client --lib` for `orchestrator_conn`, `daemon_config_render`, `daemon::tests::container` (9). Baseline failures: none seen.
- Coverage (`mise run coverage-diff`) was not run.
- `ssh_real_link` (needs `sshd`), `kube_real`, the `sandbox_real_*` acceptance suite, and the `--ignored bench` run were not executed.
- SC-002's existing suites (`daemon_routing`, `catalog_two_daemons`, `notifications`, `attention_notify`) were not re-run.
- Test files not read line by line: `micold-core/src/daemon_config/tests.rs` (+180 lines), `micold-client/src/daemon.rs` unit tests beyond the one at `:3382`, `provision/mod.rs` and `argv.rs` tests beyond the new ones, `daemon_config_delivery_host.rs` beyond the new case; `micold-daemon/src/orchestrator/{hub,client,list}.rs` source behavior was not read.
- Documentation tasks (T001, T022, T034) and quickstart Part B (visual pass): out of scope for a test audit.
- Performance and load behavior beyond the bench: no run.
