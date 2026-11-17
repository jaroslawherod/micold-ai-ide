# Quickstart: client WebSocket API (#1732)

## Part A: automated
1. `scripts/api-types.sh` regenerates the schema and TS types; `scripts/api-types.sh --check` fails when stale (also run by `scripts/gate.sh` and CI).
2. `scripts/gate.sh` (cloud) or `mise run gate`: core and daemon tests including `tests/orchestrator/ws_*.rs`.
3. `pnpm --filter @micold/protocol typecheck` and `mise run client-test`.
4. `mise run client-e2e-stack`: starts the real stack and runs Playwright in Chromium, Firefox, WebKit.

## Part B: manual visual pass
1. `mise run local-stack` (orchestrator `127.0.0.1:8730`, web client `http://127.0.0.1:8731`; override with `MICOLD_LOCAL_API_PORT`, `MICOLD_LOCAL_WEB_PORT`).
2. Open the web client: the seeded daemon and two sessions are listed; open one: content shown, refresh works.
3. Stop the orchestrator process: "reconnecting" then "cannot reach the orchestrator" with Retry; restart it: list returns.
4. Switch the language to Polish: every new string is translated.
5. Run it again in a second terminal: the port conflict is reported and exits.
6. Ctrl+C the stack: all three processes stop.
