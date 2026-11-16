# Quickstart: verifying the feature

## Part A: automated (each written failing first)

| Milestone | Test | Proves |
|---|---|---|
| M1 | `micold-core` `routing` tests: opaque form round trip; header-only decode; `req` mismatch `Malformed`; contract example decodes | FR-011 |
| M1 | `RouteTable` unit tests: bijection, `drop_link` returns every route and pending req | FR-006 shape |
| M1 | `merge_lists` unit tests: union, same path on two daemons stays two rows, reasons, `no_daemons` | FR-001 |
| M2 | `orchestrator_route::requests`: real orchestrator + two real daemons; a session create to `b` reaches `b` only, answer to the requester only | FR-003, US2.1 |
| M2 | `orchestrator_route::streams`: terminal output through the orchestrator in order, no loss/dup; two clients see a takeover | FR-004, US2.2, US2.4 |
| M2 | `orchestrator_route::refusals`: unknown id `not_found`; needs-update and offline refused | FR-005 |
| M2 | `orchestrator_route::link_drop`: kill daemon link mid-request: `OutcomeUnknown` 100 runs out of 100; stream ends `daemon-offline`; re-attach after relink gets current state | FR-006, FR-012, SC-006 |
| M2 | `orchestrator_route::slow_client`: a stalled client conn ends `slow-consumer`, another client's stream keeps its rate | FR-004 |
| M2 | `orchestrator_route::outage`: kill orchestrator, session still running on daemon | FR-010 |
| M3 | `orchestrator_route::lists`: two daemons, 100 % rows; one stopped named `offline`; one hung named `timeout` within 2.5 s | FR-001, FR-002, SC-001 |
| M3 | `orchestrator_route::bench` (`#[ignore]`, run in the milestone): 1,000 requests and 1,000 4 KiB chunks at 100/s, direct vs routed, p95 deltas within 5 ms / 16 ms | SC-003 |
| M4 | `micold-client` `orchestrator_single_connection`: headless connect: orchestrator spawned, exactly one client connection, none to a daemon socket; two concurrent starts leave one orchestrator; missing binary gives a named error | FR-007, FR-008, SC-004 |
| M4 | `micold-client` reconnect test: orchestrator killed, client reconnects and actors re-read | FR-009, SC-005 |
| M4 | `daemon_config_render` test: rendered config has `[orchestrator_link] enabled = true` and `[daemon] id` | FR-013 |
| M4 | existing socket-opening client suites run with the client through the orchestrator | SC-002 |

Run: `scripts/gate.sh`; one suite: `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator_route`.

## Part B: manual (M4)

1. No orchestrator running (`pgrep -f 'micold-daemon orchestrator'` empty). Start the app: within
   5 s one orchestrator process runs; `ss -xp` shows the app connected to the orchestrator socket
   only.
2. Open a terminal session on the host daemon, type, see output. Create and remove a worktree.
3. Kill the orchestrator: the app shows reconnecting; the session process still runs; the app
   restarts the orchestrator and the session view matches.
