# Contract: the transcript model (`micold_core::structured::transcript`)

The public surface #939's client uses and #940–#944 build on. Types: [data-model.md](../data-model.md).

## Construction

`Transcript::new()` is `Scope::Main`: an event is *top-level* when it has no `parent_tool_use_id`.
`Transcript::nested(tool_use_id)` is `Scope::Subagent(id)`: top-level means a parent of `id`; other
events are ignored. "No parent" in the rules below reads "top-level" under either scope.

## Input

`Transcript::apply(omitted: u64, events: &[SequencedEvent], live: bool)` takes one
`DaemonMsg::StructuredEvents` message as #938's wire contract defines it:

- A `live: false` message **rebuilds** the transcript from its events (and `omitted`) unless it
  continues the history in progress: the transcript was built only from `live: false` messages so
  far or is empty, and the message's first `seq` equals `next_seq`. An empty `live: false` message
  is a fresh, empty transcript.
- A `live: true` message appends.
- Then the bound applies: past `MAX_EVENTS` held events, the oldest leave, `omitted` grows by their
  number, and entries opened by them leave the front.

## Event → entry rules

| Event | Effect |
|---|---|
| `Init` | `SessionStart` entry |
| `User`, no parent: `Text` | `UserMessage` entry |
| `User`, no parent: `ToolResult` | fills the card `tool_use_id` (`Done`/`Failed`), else `OrphanResult` entry |
| `User`/`Assistant`, no parent: `Other` block | `Raw { UnknownBlock }` entry |
| `Assistant`, no parent: `Text`/`Thinking`/`ToolUse` | supersedes the open stream's next streamed block in place, else a new entry (research R3) |
| `User`/`Assistant` with `parent_tool_use_id` | no entry; `+1` on that card's `subagent_messages`, else on the `SubagentNotice` |
| `StreamDelta` with `parent_tool_use_id` | absorbed, not counted |
| `StreamDelta` `message_start` | opens the stream (closes an open one) |
| `StreamDelta` `content_block_start` | new streaming entry of the block's type (text, thinking, tool card `Running`) |
| `StreamDelta` `content_block_delta` text / thinking / input JSON | appends to the entry of its `index` |
| `StreamDelta` `content_block_delta` `signature_delta`, `content_block_stop`, `message_delta`, `ping` | absorbed |
| `StreamDelta` `message_stop` | closes the stream |
| `StreamDelta` of another type, or another delta type | `Raw { UnknownStreamEvent }` entry |
| `Result` | `Footer` entry; closes the stream; this turn's `Running` cards → `NoResult` |
| `TurnEndedWithoutResult` | `TurnEndedWithoutResult` entry; closes the stream (partials kept); `Running` cards → `NoResult` |
| `OversizedOnWire` | `Oversized` entry |
| `Unrecognised` / `Unreadable` | `Raw` entry with the original text |

## Guarantees (tested in `crates/micold-core/tests/structured_transcript.rs`)

- **G1 Accounting (SC-001)**: for every recorded fixture, every event maps to an entry, an
  absorption or a count; every tool use is one card holding its result.
- **G2 Streaming (SC-002)**: replaying a fixture one event at a time, after each event every entry
  before the streamed one is equal (key, revision, content) to before; the final entries equal those
  built from the same fixture with its `StreamDelta` lines removed, `streaming` flags aside.
- **G3 Stable keys (FR-013, FR-021)**: rebuilding from the same history yields the same keys.
- **G4 Bound (FR-020)**: at most `MAX_EVENTS` held; `omitted` counts every dropped event.
- **G5 Sub-agents (FR-024)**: removing every event with a parent from a fixture leaves the entries
  unchanged except the counts.

## Seams for later issues

| Issue | Uses |
|---|---|
| #940 | `ToolCard.name`/`input`, `ResultPart`s, `EntryKey` + `revision` as its render-cache key ([block-renderers.md](block-renderers.md)) |
| #941 | `UserMessage` entries from echoed user events; a queued-message entry would be a new `EntryKind` |
| #942 | a permission-prompt entry would be a new `EntryKind`, attached to its card by tool use id via `tools` |
| #943 | `SessionStart.model` |
| #944 | `subagent_events(id)`, `ToolCard.subagent_messages`, `Transcript::nested(id)` (the same rules with events whose parent is `id` as top-level, research R7), `TurnFooter` (per-turn cost, kept as is) |
