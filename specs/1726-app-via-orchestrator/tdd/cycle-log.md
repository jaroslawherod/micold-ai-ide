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
