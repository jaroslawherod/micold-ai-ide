Title: #1732: JSON-over-WebSocket client API with generated TypeScript types and JSON Schema, origin checks, resume

## Summary

M1 of the #1732 plan: a read-only daemon request for session content (research R9), so the
orchestrator's `/api/ws` can read a session without `Attach`, which is refused `ProjectBusy` while a
desktop holds the project and would block it.

- `ClientMsg::ReadStructuredEvents { project, session }`: answered with the history as `live: false`
  `StructuredEvents`, or `DaemonMsg::StructuredEventsRefused { session, reason }` (`NotFound`,
  `TerminalSession`). No attachment, lock, view or subscription.
- Wire change: `PROTOCOL_VERSION` 83 to 84 (schema hash changes); version tests updated; noted in
  `docs/daemon.md`.
- Test: `crates/micold-daemon/tests/orchestrator/read_structured_events.rs` (desktop holds the project and is
  not displaced; terminal session; unknown session; never-run session).

## Test plan

- `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator read_structured_events`
- `scripts/gate.sh`

Refs #1732

🤖 Generated with [Claude Code](https://claude.ai/code/session_01NCdYvPHWVQoP2N2Q2Mrn47)

https://claude.ai/code/session_01NCdYvPHWVQoP2N2Q2Mrn47
