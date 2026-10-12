# Contract: sub-agent lifecycle hooks (#552)

## C1 Registration

`settings_json` (`hooks.rs`) adds `SubagentStart` and `SubagentStop`, `matcher: ""`, same `http`
hook shape, URL and bearer token as the five existing events. The hooks test that asserted
`SubagentStop` absent now asserts both present.

## C2 Payload handling (`classify_hook`)

| Body | Class | Effect |
|---|---|---|
| `hook_event_name` `SubagentStart`, string `agent_id` | `Agent(Started{agent_id, agent_type})` | upsert run `Running` |
| `SubagentStop`, string `agent_id` | `Agent(Stopped{agent_id, agent_type, agent_transcript_path})` | upsert run `Stopped` |
| either event with missing/non-string/empty `agent_id` | `Ignored` | `200`, logged at debug, nothing recorded |
| invalid JSON / no event name | `Invalid` | `400` (unchanged) |

The origin is the session of the authenticated `/hook/<uuid>` URL. A uuid unknown to the catalog
fails token authentication already (no token), so no phantom session is possible.

## C3 Isolation

An `Agent` class never calls `note_activity`. The FSM state, attention and notification paths are
unchanged by both events (FR-007). Only `note_agent_signal(session, signal) -> bool` runs; on
`true` the daemon calls `broadcast_catalog()` so clients see `SessionSummary.agents`.

## C4 Persistence

`note_agent_signal` mutates the stored session and persists through the catalog's existing write
path; it touches only the named session (FR-014).

## C5 Logging

The body is never logged; at most the session id and event name at debug.
