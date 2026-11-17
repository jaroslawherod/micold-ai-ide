## Milestone M4 of 6 — client-ws-api (specs/1732-client-ws-api)

**Deliverable**: `mise run local-stack` starts the orchestrator, a seeded daemon and the web client; the client lists real daemons and sessions, shows connection states, reconnects with backoff and resume, marks unreachable sessions, opens session content, in English and Polish.
**Verify**: `mise run client-test`, then quickstart Part B steps 1–4
**Tasks**: T027–T037 · **Satisfies**: US1 scenarios 1–4, US2 scenario 3, US3 scenario 3, FR-013–017, FR-014a, FR-014b, SC-007

## What changed
- `@micold/protocol/real`: `BrowserTransport` (close codes, `visibilitychange`), `RealOrchestratorApi` over `/api/ws` (subscribe, resume from `{epoch, seq}`, `resync_needed` then a fresh snapshot, ping 20 s / dead 40 s, backoff 500 ms to 15 s with full jitter, `offline` after 8 attempts, Retry), a local auth stand-in (the API has no sign-in until #1733). Backoff shared with `ConnectionManager`, which gains `connecting`.
- Web client: the real API is the default build (`EXPO_PUBLIC_ORCHESTRATOR_URL`, default `ws://127.0.0.1:8730/api/ws`); the mock only with `EXPO_PUBLIC_API_MODE=mock` (`mise run client-web-mock`). Connection banner with connecting, reconnecting (attempt) and unreachable with Retry; unreachable marker on session rows; session content view (snapshot on open, Refresh, empty, failed by error code, truncated). Every new string in `en.json` and `pl.json`; the server `message` is never shown.
- `scripts/local-stack.sh` (`mise run local-stack`) with a seeded project, worktree and two sessions, ports checked first, one trap stopping all; `scripts/tests/local-stack.test.sh`.
- User guide page `docs/user-guide/web-client.md`.

## Agent review
- Review A (code-review, high): run by the orchestrator.
- Review B (conformance, sonnet): run by the orchestrator.

## Local gate
Local gate: scoped only, full gate left to CI. `scripts/gate.sh` fmt and clippy (core, workspace) green, then stopped at `cargo nextest` (not installed in the container); no Rust file changed in this PR. `pnpm --filter @micold/client lint typecheck test` (217 tests) and `@micold/protocol typecheck` and its 73 tests green; web export and bundle check green (no mock in the export); `scripts/tests/*.test.sh` green. Not run here: the real daemon end to end and the visual pass (quickstart Part B, M6).

Refs #1732

🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_01NCdYvMniqxBrPRp51iTYQJ
