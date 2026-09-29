# Contract: the tool server endpoint and the per-session binding

**Feature**: 034-daemon-mcp-server | Research: R1–R8, R14

## §1 Endpoint

- One listener per service run, bound to `127.0.0.1:0` (ephemeral), never `0.0.0.0` (FR-001,
  FR-007). In the sandbox placement it binds the container's loopback and is not published.
- Path: `/mcp`. Any other path: `404`, empty body.
- `POST /mcp` with `Content-Type: application/json`, one JSON-RPC 2.0 message per request.
  - Head bound 8 KiB, body bound 1 MiB; larger → `431` / `413`, body drained as the hook receiver
    does (shared HTTP module).
  - Reply: `200` with `Content-Type: application/json` and one JSON-RPC response, or `202` with an
    empty body for a notification.
- `GET /mcp`, `DELETE /mcp`: `405` (no standalone stream, no session to end).
- No `Mcp-Session-Id` is issued; every request is authenticated on its own.

## §2 Authentication (FR-006)

- Header `Authorization: Bearer <credential>`. Missing, malformed or unknown → `401`, empty body,
  identical for every cause (SC-006: reveals nothing, not even whether a session exists).
- The credential maps to exactly one session; every accepted request is attributed to it.
- A credential stops working when its session is deleted or the service exits.
- The body is never logged; the credential is never logged.

## §3 MCP methods

| Method | Answer |
|---|---|
| `initialize` | `protocolVersion`: the client's if in {`2025-03-26`, `2025-06-18`, `2025-11-25`, `2026-07-28`}, else `2026-07-28`; `capabilities: {"tools": {"listChanged": false}}`; `serverInfo: {"name": "micold", "version": <service version>}`; `instructions`: one paragraph naming the scope (own project) and the confirmation policy |
| `notifications/initialized` | `202` |
| `ping` | `{}` |
| `tools/list` | the tools of [mcp-tools.md](./mcp-tools.md), each with `name`, `description`, `inputSchema` (JSON Schema), `annotations.readOnlyHint`/`destructiveHint` |
| `tools/call` | see mcp-tools.md §Results |
| anything else | JSON-RPC error `-32601` |
| malformed JSON | JSON-RPC error `-32700`, id `null` |

## §4 Launch wiring per CLI (FR-002, FR-003)

The config file is `<data_dir>/mcp/<session-uuid>.json`; directory `0700`, file `0600` on Unix,
owner-only protected DACL on Windows (R7). Rewritten at every spawn of the session (the port and,
after a service restart, the credential change).

| CLI | Arguments appended | File content |
|---|---|---|
| Claude Code | `--mcp-config <file>` `--allowedTools mcp__micold` | `{"mcpServers":{"micold":{"type":"http","url":"http://127.0.0.1:<port>/mcp","headers":{"Authorization":"Bearer <cred>"},"timeout":120000}}}` |
| Copilot | `--additional-mcp-config @<file>` `--allow-tool micold` | same entry plus `"tools":["*"]` (shape confirmed by quickstart §B2) |
| Pi | none | none — logged: "no tool server: Pi has no MCP support" |
| Regular terminal | none | none, not logged (not an AI CLI) |

Never passed: `--strict-mcp-config`. Never written: any user or project configuration file.

## §5 When a session is started unbound (FR-004, FR-005)

Exactly one `info` log line per spawn names the session and one reason:
`disabled in settings` · `<CLI> has no MCP support` · `a server named "micold" is already configured
in <path>` · `tool server unavailable` · `could not write the binding: <io error>`.
The session then starts with exactly the arguments it has today.

## §6 Name collision check (R14)

Read-only, before writing the file: Claude — `mcpServers.micold` at the top level or under the
session's project path in `<claude config dir>/../.claude.json` (the file Claude Code keeps beside
its config dir), and in `<cwd>/.mcp.json`; Copilot — `mcpServers.micold` in
`<copilot config dir>/mcp-config.json`. An unreadable or malformed file counts as "not taken".
