## Milestone M3 of 3 — app via orchestrator
(specs/1726-app-via-orchestrator)

**Deliverable**: the app starts the orchestrator when absent, reaches its host daemon only through
it, reconnects after an outage and re-reads, and writes `[orchestrator_link]` (and `[daemon] id`)
into the daemons it provisions; the user guide says so.
**Verify**: `cargo test -p micold-client --test main orchestrator_single_connection daemon_routing catalog_two_daemons notifications attention_notify daemon_config`;
`cargo test -p micold-client --lib orchestrator_conn daemon_config_render`
**Tasks**: T017–T022 · **Satisfies**: US3 scenarios 1–4; US4 (app half); FR-007–FR-009, FR-013, SC-002, SC-004, SC-005

## What changed
- New `crates/micold-client/src/orchestrator_conn.rs`: the app's one orchestrator connection.
  Each actor gets a `Transport::Routed` in-memory pipe in place of a socket; the module wraps its
  frames into opaque routed frames on `(target, conn)` (header `req` set for requests) and hands
  answers back, so the actor's handshake, pump and keepalive are unchanged. Opened on first use
  through `connect_or_spawn_with`, which starts `micold-daemon orchestrator` when nothing answers;
  the orchestrator's singleton makes concurrent starts converge.
- Reconnect (FR-009): when the connection drops every route ends; each actor's existing
  `reconnect_loop` opens a new route, which reopens the connection (starting the orchestrator
  again) and re-handshakes: the fresh `Welcome` is the re-read. `StreamEnded` and `OutcomeUnknown`
  end the route the same way, so an unknown outcome re-reads the target instead of guessing.
- Host actor (`daemon.rs`): starts the host daemon as before when its endpoint is cold, then
  handshakes on a route to `local`; an orchestrator that cannot start fails with its cause in the
  connection banner (e.g. `cannot start /usr/bin/micold-daemon: No such file or directory`),
  never falling back to the daemon socket.
- Core: `connect_or_spawn_with` (the `connect_or_spawn` loop with connect and spawn as
  parameters), `spawn::orchestrator_command` / `spawn_detached` (error names the program).
- Wire ids (FR-013, R6): `FileSettings::wire_id`, `slugify(name)-<8 hex>`, generated on add and on
  load for older entries, kept across Settings saves; rendered as `[daemon] id`. Every rendered
  `daemon.toml` ends with `[orchestrator_link] enabled = true` (the host links as `local`).
- User guide: `docs/user-guide/orchestrator-mode.md`, *The desktop app connects through it* and
  *When the orchestrator cannot start*.

## Scope note
Only the host daemon's actor is routed. SSH, container and Kubernetes actors still use the
connection their provisioning step returns, though their config now links them under their wire
id; routing them needs each placement to reach the orchestrator socket (see `drift-M3.md`).
Follow-up recommended.

## Tests
`orchestrator_single_connection` (real orchestrator process, real daemon linked in-process with
no socket of its own): started within 5 s and two routes over one connection; two concurrent
starts leave one orchestrator; a missing binary is named; killed orchestrator → route ends,
restarted, re-handshake reaches the daemon. T021 suites pass unchanged (they open no socket).

🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_01NCdYvMwxxd3jeGha6xeqC9
