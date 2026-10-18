# Data Model: Structured Session Mode (#938)

## SessionKind (core, persisted, on the wire)

| Variant | Meaning |
|---|---|
| `Terminal` (default) | today's PTY session |
| `Structured` | pipes + stream-json; Claude Code only |

- `Session.kind`: set by `Session::start_new_of_kind(location, provider, kind)` and
  `Session::restored_of_kind(…, kind)`; `start_new` and `restored` keep their signatures and
  delegate with `Terminal`, so existing callers are untouched (SC-004); no setter (FR-002).
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
| `OversizedOnWire` | `original_bytes: u64` | wire only: stands in, under its own `seq`, for an event whose encoding exceeds `MAX_FRAME_LENGTH / 2` (R6); history keeps the event whole |

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

`{ seq: u64, event: StructuredEvent }`. `seq` starts at 0 per session and never repeats within a
history; a history restored after a daemon restart continues its numbering (R14), one with nothing
restored starts at 0 again. It is what makes "no gap, no duplicate" checkable (FR-017, FR-021a,
SC-005, SC-007).

## EventHistory (daemon memory, per session)

| Field | Meaning |
|---|---|
| `events: VecDeque<SequencedEvent>` | retained, oldest first |
| `next_seq: u64` | seq for the next append |
| `bytes: usize` | retained text size |
| `omitted: u64` | how many were evicted |

Rules: append assigns `seq`; while `events.len() > MAX_EVENTS || bytes > MAX_BYTES` pop front,
`omitted += 1` (FR-018). `snapshot()` → `(omitted, Vec<SequencedEvent>)`. Invariant: the first
retained event's `seq` is `omitted`, and `next_seq == omitted + events.len()`. Lives across
stop/start within a daemon (FR-021a); dropped on session delete. While saving is on it is saved as
`SavedEvents` and rebuilt by `EventHistory::restored(omitted, events: Vec<StructuredEvent>)`, which
numbers the events from `omitted` and applies the current bounds (R14).

## SavedEvents (core, file payload; `micold_core::structured::saved`)

The postcard payload inside a `<session uuid>.events` file, framed as 041's saved-history file
with magic `MICOLDSE` and `EVENTS_FORMAT_VERSION = 1` (R14).

| Type | Fields / variants | Rules |
|---|---|---|
| `SavedEvents` | `omitted: u64`, `events: Vec<SavedEvent>` | oldest first; `omitted + events.len()` fits in `u64` |
| `SavedEvent` | `Line(String)` · `Unreadable(String)` · `TurnEndedWithoutResult` | `Line` is a CLI-sourced event's `raw`, parsed again by `parse_line` on load; `Unreadable` keeps an unreadable line's text and kind; `OversizedOnWire` is never saved |

- `encode_events(omitted, &[SequencedEvent]) -> Vec<u8>` and
  `decode_events(&[u8]) -> Result<(u64, Vec<StructuredEvent>), DamageReason>`; read cap
  `MAX_EVENTS_FILE_BYTES = MAX_BYTES + 1 MiB`. Damage reasons are 041's `DamageReason`
  (`TooLarge`, `NotAHistory`, `OtherVersion`, `Truncated`, `Checksum`, `Malformed`), never a
  partial result (FR-021c).

## HistoryStore additions (core, `terminal_history::store`)

| Item | Rule |
|---|---|
| `FileKind { Terminal, Events }` | extension `.history` / `.events`; temporary `.<name>.tmp` for both |
| `save_events(id, omitted, &[SequencedEvent])` | as `save`: `Skipped` when off, forgotten or no directory; `Unchanged` on the same checksum; encoding outside the mutex |
| `load_events(id) -> EventsLoad` | `None` · `Events { omitted, events }` · `Damaged(DamageReason)`; `None` when off, unlisted, forgotten or the file is in `undeleted` |
| `set_enabled(false)`, `purge`, `forget`, `sweep`, `retry_deletions` | cover both kinds; `sweep(keep)` keeps `<id>.history` and `<id>.events` of every id in `keep` |
| `last_written` | keyed by `(SessionId, FileKind)` |

## Daemon state additions (`state.rs`, `history.rs`, `structured.rs`)

| Item | Rule |
|---|---|
| `event_streams` entry | created by one call used by the structured spawn and the view path: absent → `load_events` off the state lock, then insert if still absent (R14); `Damaged` → empty, one `warn!`, schedule due |
| `EventStream::restored(omitted, events)` | the stream around `EventHistory::restored`; `EventStream::saved_snapshot()` gives `(omitted, events)` under the history lock |
| Saver event schedules | `HashMap<SessionId, SaveSchedule>`, count = the history's `next_seq`; walked on the saver tick; marked due when the setting is turned on or after a damaged load |
| Process end | a structured process end saves its stream after its readers are joined |
| Orderly stop | `save_all_live` saves every stream with unsaved events inside `STOP_SAVE_BOUND` |

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
