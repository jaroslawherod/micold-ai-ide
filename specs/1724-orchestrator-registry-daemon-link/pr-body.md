Milestone M6 of #1724 (part of #1722): the orchestrator never dials a daemon, proven three ways.

- `tests/orchestrator/no_outbound.rs` (T041): a source scan fails on a whole-word `connect`, `Stream::connect`, `open_pipe`, `dial`, a `connect_*` helper or a path into a dialing module (comments, strings and char literals stripped) under the orchestrator's source in `micold-daemon` and `micold-core`; only the `singleton::acquire` self-probe is allowed.
- `tests/orchestrator/netns.rs` (T042): Linux only; re-runs `link_and_list` and `heartbeat` under `unshare -rn`, checks the block is real (`NetworkUnreachable`, an outer loopback listener unreachable), and re-runs them under `strace -e trace=connect` asserting every connect is to `orchestrator.sock`. Skips with a stated reason where the tool or user namespaces are unavailable.
- T043: quickstart Parts A and B recorded; `docs/daemon.md` and the orchestrator user guide state the rule; `docs/SUMMARY.md` already links the page.

Tests: `cargo test -p micold-daemon --test orchestrator` 40 passed; fmt and workspace clippy `-D warnings` clean. Two `micold-core` permission tests fail only under the container's root user and do not touch this change.
