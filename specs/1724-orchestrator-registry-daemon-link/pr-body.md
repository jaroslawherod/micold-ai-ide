Milestone M6 of #1724 (part of #1722): the orchestrator opens no outbound connection, proven two ways. Closes #1724.

- `tests/orchestrator/no_outbound.rs` (T041): a source scan fails on a whole-word `connect`, `Stream::connect` or `open_pipe` (comments and string literals stripped) under the orchestrator's source in `micold-daemon` and `micold-core`. Has its own test of the scanner.
- `tests/orchestrator/netns.rs` (T042): Linux only; re-runs `link_and_list` and `heartbeat` under `unshare -rn` and checks a TCP connect fails inside the namespace. Skips with a stated reason where user namespaces are unavailable. Takes the child-count lock so `not_hosted` and `restart` stay stable.
- T043: quickstart Parts A and B recorded in `quickstart.md`; `docs/daemon.md` and the orchestrator user guide state the no-outbound rule; `docs/SUMMARY.md` already links the page.

Tests: `cargo test -p micold-daemon --test orchestrator` 39 passed (3 runs), fmt and clippy `-D warnings` clean. Two `micold-core` permission tests fail only under the container's root user and do not touch this change.
