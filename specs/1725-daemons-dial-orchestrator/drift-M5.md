# Plan drift: M5

Paths changed beyond those the tasks of M5 name, with the reason.

- UNPLANNED `crates/micold-daemon/tests/orchestrator_dial/outage.rs`: one in-process test,
  `a_stale_forward_socket_is_a_quiet_connect_failure_and_a_new_forward_links_within_16_s`, for scenario
  3's "a connect failure, not a refusal, backing off quietly" on a paused clock: it captures the log at
  every level, which the `ssh_real` test (the remote daemon's log file at its configured level) cannot,
  and it runs in CI's default suite, where the `ssh_real` suite does not.
- MISSING `crates/micold-daemon/src/remote.rs`, `crates/micold-daemon/src/orchestrator_link/mod.rs`:
  T039 named them for "stale-socket removal in `prepare-link`, a refused connect is `NotLinked` with no
  log line per retry". Both were already true when T038's tests ran: `prepare_link_at` removes
  `orchestrator.sock` (M4, T033) and `run` maps an absent or refused connect to `NotLinked` and
  `after_failure` without logging (M1, M3). T038 failed only on the test harness's hand-moved clock
  (tdd/cycle-log.md, cycle 15), so T039 changed nothing ("change nothing T038 does not need").
