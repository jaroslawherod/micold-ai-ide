# Plan drift, M3

| Path | Reason |
|---|---|
| `crates/micold-core/src/orchestrator/liveness.rs` | `Heartbeat::is_silent`: the `Silent` link state of T028's `Registry::claim` reads it. |
| `crates/micold-core/src/orchestrator/mod.rs` | Re-exports of `Claim`, `ClaimFailure`, `LinkVerdict` and `RefuseReason` for T028 and T029. |
| `crates/micold-daemon/src/orchestrator/query.rs` | T028: the listing carries `orchestrator_version` for `needs-update` records. |
| `crates/micold-daemon/tests/orchestrator/support.rs` | T027 harness: `announce_raw` (old-format preamble), `hello_with`/`hello_at` (schema hash, package and protocol version in `Hello`), `expect_closed`. |
