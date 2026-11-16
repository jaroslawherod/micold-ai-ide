# TDD cycle log, #1726

## M3

Honest record: T017, T019a and T020a were written in the same working session as their code and
landed in one commit (0deffa8b6); no failing run was captured before the implementation. Red
evidence is therefore not available from history. Green evidence, on the round-1 fixes:

| Task | Test | Red | Green |
|---|---|---|---|
| T017 | `orchestrator_single_connection::the_app_starts_the_orchestrator_and_reaches_the_daemon_over_one_connection`, `two_concurrent_starts_leave_one_orchestrator`, `an_orchestrator_that_cannot_start_is_named` | not captured (written with the code) | ok |
| T019a | `orchestrator_single_connection::after_the_orchestrator_is_killed_the_app_starts_it_again_and_re_handshakes`; `orchestrator_conn::tests::stream_ended_and_outcome_unknown_end_the_route` | not captured (written with the code) | ok |
| T020a | `daemon_config_render::tests::a_provisioned_daemon_links_to_the_orchestrator_under_its_wire_id`, `an_entry_saved_without_a_wire_id_gets_one_once` | not captured (written with the code) | ok (gate) |

Round-1 review fixes (dead-link route, full actor queue) are concurrency races with no
deterministic test added; covered by the existing reconnect test staying green.

## M4

| Task | Test | Red | Green |
|---|---|---|---|
| T023 | `ssh::argv::tests::the_link_holder_carries_the_admitted_forward_and_runs_hold_link_never_a_bridge` | compile error: no `hold_link` | ok |
| T025 | `provision::tests::the_holder_admits_the_link_opens_hold_link_and_returns_on_ready`, `a_holder_whose_forward_cannot_bind_names_it_not_bound`, `a_refused_link_opens_no_holder` | compile error: no `LinkHolder`/`hold_link` | ok |
| T027 | `remote_hold_link::hold_link_prints_ready_touches_no_daemon_and_exits_on_stdin_eof` | assertion: usage text, no `ready` | ok |
| T029 | `orchestrator_single_connection::a_route_to_a_provisioned_daemon_waits_for_its_link_and_reaches_it`, `a_daemon_that_never_links_fails_the_route_naming_its_wire_id`; `daemon::tests::container_and_pod_options_carry_the_orchestrator_link` | not captured (written with `routed_to`); first run failed for a real reason: the orchestrator answers `StreamEnded{NotFound}`, not EOF, fixed in `not_linked` | ok |

