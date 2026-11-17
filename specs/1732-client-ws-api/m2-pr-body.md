Title: #1732: JSON-over-WebSocket client API with generated TypeScript types and JSON Schema, origin checks, resume

## Milestone M2 of 5 — client WebSocket API (specs/1732-client-ws-api)

**Deliverable**: with `--api-addr` and `--allow-origin` set, the orchestrator serves `/api/ws`: list, read a session's content, subscribe with exact-once resume, app-level heartbeat, typed errors, foreign origins refused with 403.
**Verify**: `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator ws_` and `cargo test -p micold-core --all-targets`
**Tasks**: T001–T002, T005–T019 · **Satisfies**: US2 server-side 1–2; US3 1–2; FR-001–008, FR-011; SC-002, SC-003, SC-005, SC-006

- `micold_core::client_api` (pure): wire types with `serde` + `schemars::JsonSchema` (`messages.rs`), the single read/change table with a schema-checked `KINDS` (`classify.rs`), the 1024-change `ChangeLog` with epoch (`log.rs`), the snapshot `diff` plus a Rust mirror of the `order.ts` reducer and a 2000-case property test (`diff.rs`), the `Origin` allow-list (`origin.rs`), session history to `ContentEntry` (`content.rs`).
- `micold-daemon` orchestrator: `directory.rs` (`LinkDirectory` trait, `InProcess` over `list::gather` and a one-shot `ReadStructuredEvents` conversation; offline daemons keep their rows as unreachable), `feed.rs` (one poller while subscribed, poked on registry change, snapshot and log kept across stop; random 53-bit epoch), `ws.rs` (`tokio-tungstenite` `accept_hdr_async`: 404 other paths, 403 + WARN `refused origin <origin>`; typed errors, binary refused, pong, 40 s idle release, 1008 after 5 malformed, 1011 on internal failure), `config.rs` (`--api-addr`, repeated `--allow-origin`, env twins; non-loopback refused until #1733).
- `list.rs`: `ask` now uses a general `converse` helper shared with the content read.
- Tests: `tests/orchestrator/ws_{origin,protocol,read_session,heartbeat,resume,listener}.rs` with `ws_support.rs`; the real linked daemon moved to `tests/support/linked_daemon.rs` (shared with `orchestrator_route`).
- Docs: `docs/user-guide/orchestrator-mode.md` — `--api-addr`, `--allow-origin`, `MICOLD_ORCHESTRATOR_*`.

## Agent review
- REVIEWS PENDING (run by the orchestrator).

## Local gate
`scripts/gate.sh` — see the ledger for the result.

Refs #1732

🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_01NCdYvP3vvFGZuLAHUEgQDU
