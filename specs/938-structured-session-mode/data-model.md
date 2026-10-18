# Data Model: Structured Session Mode (#938)

## SessionKind (core, persisted, on the wire)

| Variant | Meaning |
|---|---|
| `Terminal` (default) | today's PTY session |
| `Structured` | pipes + stream-json; Claude Code only |

- `Session.kind`: set by `Session::start_new(location, provider, kind)` and
  `Session::restored(…, kind)`; no setter (FR-002).
- `StoredSession.kind`: `#[serde(default, skip_serializing_if = "SessionKind::is_terminal")]`;
  absent → `Terminal` (FR-001, FR-004). No `schema_version` bump.
- `SessionSummary.kind`, `SessionCreate.kind`: `#[serde(default)]` (FR-020).
- Validation: `SessionKind::check(provider)` → `Err(KindRefusal::NotSupported(provider))` for
  `Structured` with a provider whose `structured_args()` is `None` (FR-003). Applied in
  `Catalog::create_session` before any record is written.

## StructuredEvent (core, on the wire)

One per line read (FR-014), plus daemon notes.

| Variant | Fields (at least) | Source |
|---|---|---|
| `Init` | `session_id`, `model`, `cwd`, `tools: Vec<String>`, `raw` | `{"type":"system","subtype":"init"}` |
| `Assistant` | `message_id`, `blocks: Vec<ContentBlock>`, `parent_tool_use_id`, `raw` | `type: assistant` |
| `User` | `blocks: Vec<ContentBlock>`, `parent_tool_use_id`, `raw` | `type: user` |
| `Result` | `subtype`, `is_error`, `duration_ms`, `num_turns`, `total_cost_usd`, `usage: Option<Usage>`, `result: Option<String>`, `session_id`, `raw` | `type: result` |
| `StreamDelta` | `event_type` (e.g. `content_block_delta`), `index`, `delta: Option<Delta>`, `parent_tool_use_id`, `raw` | `type: stream_event` |
| `Unrecognised` | `raw: String` (the whole line) | valid JSON, unknown type or shape mismatch (FR-012) |
| `Unreadable` | `text: String` | not JSON, or a partial last line (FR-013) |
| `TurnEndedWithoutResult` | — | daemon note: process ended mid-turn |

`raw` is the original line on every CLI-sourced variant, so nothing is lost when the model is
narrower than the CLI (FR-011, FR-015). Typed fields are `Option` where the CLI may omit them.

### ContentBlock

| Variant | Fields |
|---|---|
| `Text` | `text` |
| `Thinking` | `thinking` |
| `ToolUse` | `id`, `name`, `input: String` (JSON text) |
| `ToolResult` | `tool_use_id`, `content: String` (text, or JSON text of a block list), `is_error: bool` |
| `Other` | `raw: String` |

Pairing: `ToolUse.id == ToolResult.tool_use_id` (SC-001). Helper
`structured::pair_tool_calls(&[SequencedEvent]) -> Vec<(seq_use, Option<seq_result>)>` is what the
fixture tests assert on.

## SequencedEvent

`{ seq: u64, event: StructuredEvent }`. `seq` starts at 0 per session per daemon lifetime and never
repeats; it is what makes "no gap, no duplicate" checkable (FR-017, SC-005).

## EventHistory (daemon memory, per session)

| Field | Meaning |
|---|---|
| `events: VecDeque<SequencedEvent>` | retained, oldest first |
| `next_seq: u64` | seq for the next append |
| `bytes: usize` | retained text size |
| `omitted: u64` | how many were evicted |

Rules: append assigns `seq`; while `events.len() > MAX_EVENTS || bytes > MAX_BYTES` pop front,
`omitted += 1` (FR-018). `snapshot()` → `(omitted, Vec<SequencedEvent>)`. Lives across
stop/start within a daemon (FR-021a); dropped on session delete; never persisted.

## TurnState (daemon memory, per live session)

`Idle → InTurn` on an accepted user message; `InTurn → Idle` on a `Result` event; `InTurn → Idle`
plus a `TurnEndedWithoutResult` event on process exit/restart. A new live entry starts `Idle`.

### UserMessageRefusal

`Blank` · `NotRunning` · `TurnInProgress` · `NotStructured` — each with a user sentence (FR-008).

## RefusedRequest (wire)

`Input` · `Resize` · `Scrollback` · `Interrupt` — a terminal-only request sent to a structured
session's primary (FR-019).

## Lifecycle

Unchanged `SessionLifecycle`; a structured session moves through the same states via the same
`supervise_session_exit` policy (FR-009). Its primary is `Proc::Structured`.
