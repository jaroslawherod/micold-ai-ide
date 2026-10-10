# Contract: status-line relay and `/status` route (source A)

The only source-specific contract (research R1). Replacing the source replaces this file's
subject; `plan-usage-wire.md` and `usage-indicator.md` stay.

## S1. Settings-file block

With **Show Claude plan usage** on, `settings_json` (`crates/micold-daemon/src/hooks.rs`) emits,
beside the existing `hooks` map:

```json
"statusLine": {
  "type": "command",
  "command": "\"/abs/path/micold-daemon\" status-line \"/abs/path/hooks/<uuid>.status.json\"",
  "padding": 2,
  "refreshInterval": 5
}
```

- S1.1 `padding` and `refreshInterval` appear only when the user's resolved status line sets them
  (data-model `UserStatusLine`).
- S1.2 Paths are double-quoted, with `/` separators on every platform (Git Bash rule).
- S1.3 With the switch off the document has no `statusLine` key and no relay file exists (FR-002).
  The `hooks` map is identical either way.
- S1.4 Typed structs, not `json!` (the BUG-001 rule of `settings_json`).

## S2. Relay process — `micold-daemon status-line <relay-file>`

Dispatched in `crates/micold-daemon/src/main.rs` before the runtime, logging or singleton start;
logic in `crates/micold-daemon/src/status_relay.rs` (testable with injected stdin, stdout, shell
and HTTP).

- S2.1 Reads stdin to EOF. Input over 1 MiB yields no reading (S2.3 skipped); S2.4 still runs.
- S2.2 Reads the relay file. Missing, unreadable or malformed: no reading; S2.4 runs with no user
  command; exit 0.
- S2.3 `status_line::extract(stdin)`; when it yields a `rate_limits` object, POSTs exactly that
  object as the body, `Authorization: Bearer <token>`, to `url`; whole exchange bounded at
  500 ms. The rest of stdin is never sent.
- S2.4 Runs `user_command` (if any) through the platform shell (research R4) with the complete
  stdin, cwd inherited, env plus `MICOLD_STATUS_RELAY=1`; copies its stdout to stdout unchanged,
  as it arrives; stderr inherited.
- S2.3 and S2.4 run concurrently; S2.4's output is never held for S2.3 (FR-017).
- S2.5 Exit status: the user command's, else 0. The relay itself writes nothing to stdout or
  stderr, in any failure (FR-015, FR-020).
- S2.6 Started with `MICOLD_STATUS_RELAY=1` already set: S2.3 skipped, S2.4 runs with no user
  command (recursion guard).

## S3. Route — `POST /status/<session-uuid>`

On the existing loopback hook receiver (026 contracts/hooks.md rules 1–5 apply unchanged).

| Case | Answer |
|---|---|
| Not POST | 405 |
| Path not `/status/<uuid>` or `/hook/<uuid>` | 404 |
| Token missing or wrong (checked before the body) | 403 |
| Body over 16 KiB | 413 |
| Body not a JSON object | 400 |
| Object with no usable window | 200, nothing stored |
| Usable reading | 200, `note_plan_usage` |

- S3.1 The body is never logged. A rejection is logged at `debug` at most once per
  (session, answer) until that session's next accepted reading (FR-016).
- S3.2 While the switch is off, a valid reading is answered 200 and dropped (a running session may
  still carry an old settings file, research R9).
