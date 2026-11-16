## Milestone M2 of 3 — app via orchestrator
(specs/1726-app-via-orchestrator)

**Deliverable**: one list request through the orchestrator returns every linked daemon's rows and
names an offline or hung daemon within 2.5 s; the benchmark shows routing within SC-003.
**Verify**: `cargo test -p micold-daemon --test orchestrator_route lists`;
`cargo test -p micold-daemon --test orchestrator_route -- --ignored bench --nocapture`
**Tasks**: T014–T016 · **Satisfies**: US1 scenarios; US5; FR-001, FR-002, SC-001, SC-003

## What changed
- `ClientMsg::OrchList` is answered with `DaemonMsg::OrchListed` (was a placeholder error). New
  `crates/micold-daemon/src/orchestrator/list.rs`: every registry record asked in parallel, each
  online daemon under a 2 s deadline (`Hello` on an orchestrator-owned conversation of its link,
  rows from the `Welcome` catalogue, conversation closed afterwards; nothing cached); offline and
  needs-update named without asking; answers merged by core's `merge_lists`. Answered in a
  background task so a slow daemon never stalls the client's other frames.
- Fix found by the `offline` case: on a link's EOF the orchestrator ended its routes but kept the
  link registered in the hub until the liveness deadline ran out, so a route opened in that window
  went into a closed socket (no refusal; a list saw `timeout`). The link is now unhooked at EOF
  (generation-checked), so it is refused / listed `offline` at once.

## SC-003 benchmark (debug build, cloud container, two runs)
| | direct p95 | routed p95 | delta | budget |
|---|---|---|---|---|
| 1,000 requests (Ping→Pong) | 157–237 µs | 560–751 µs | +0.40–0.51 ms | 5 ms |
| 1,000 4 KiB chunks at 100/s | 2.36–2.58 ms | 3.15–3.25 ms | +0.66–0.79 ms | 16 ms |

## Agent review
Fresh-context diff review (Opus), round 1: **CLEAN** on head 4e21273c. Two minor follow-ups, not blocking: a link dropped mid-ask is reported as `timeout` rather than `offline` (list.rs:111); concurrent `OrchList` per client is uncapped (client.rs:188).

## Local gate
fmt and `clippy --workspace --all-targets -D warnings` clean at cdf9174dbd9c2777d88e092a101e165e5b4f7a47; `orchestrator_route` (19 + bench)
and `orchestrator` (139) pass. `cargo nextest` is not installable here, so the workspace ran with
`cargo test --workspace --no-fail-fast`: the only failures are environmental (container runs as
root, so the unreadable-file / unremovable-path tests in micold-core main, worktree_leftovers,
daemon settings and sessions cannot fail a read; no ssh-keygen for
`session_extras::tests::a_signed_commit_succeeds_with_the_delivered_key`).

Refs #1726

---
🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_01NCdYvKXkNibdwhj94DjcSK
