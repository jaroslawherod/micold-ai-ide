# Plan drift, M4

| Path | Reason |
|---|---|
| `crates/micold-core/src/orchestrator/registry.rs` | `Registry::ended(link)`, beside `closed`: US4.2 needs a closed transport to free its id at once, while plan "a drop is silence" keeps the record's health until the deadline. `ended` frees the id and keeps the heartbeat, so the link loop still marks the record `offline` at the deadline and then calls `closed`. The plan's single `closed` could not do both. |
| `crates/micold-core/src/orchestrator/registry.rs` | `claim` takes `now` (the plan's `claim(ann, now, wall)` without `wall`, which nothing reads): the reservation's start, for its expiry at `LIVENESS_DEADLINE`. The expiry is checked on the next claim for the id, so the controllable clock drives it in tests; the `Hello` read timeout bounds it in real time. |
| `crates/micold-daemon/tests/orchestrator/support.rs` | `FakeDaemon::accepted_after_verdict`: switch codecs after a verdict read with `announce`, for the retry loop in `one_link_per_id.rs`. |
| `crates/micold-daemon/tests/orchestrator/heartbeat.rs` | Its local `record_when` helper moved to `support.rs` (Review A round 1 F3) and the test now imports the shared one; no new behaviour. |
