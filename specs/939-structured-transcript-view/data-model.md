# Data Model: Structured Session Transcript View (#939)

All types in core are in the new render-free module `micold_core::structured::transcript`
(research R1). No type here is persisted or sent on the wire; #938's `StructuredEvent`,
`SequencedEvent` and `StructuredEvents` message are reused unchanged.

## Transcript (core; one per window and structured session)

| Field | Meaning |
|---|---|
| `events: VecDeque<SequencedEvent>` | the window's history, oldest first, at most `MAX_EVENTS` (R8); #944 reads sub-agent events from it |
| `omitted: u64` | events before the first held one (the service's `omitted` plus those this transcript dropped) |
| `next_seq: u64` | one past the last held `seq`; `0` before any |
| `entries: VecDeque<Entry>` | visible entries in order, excluding the derived omitted notice |
| `first_id: u64` | `EntryId` of `entries[0]`; ids are monotonic within one build |
| `tools: HashMap<String, EntryId>` | tool use id → its card |
| `stream: Option<OpenStream>` | the assistant message being streamed (R3) |
| `turn_cards: Vec<EntryId>` | cards opened since the last turn end, closed to `NoResult` by it |
| `subagent_notice: Option<EntryId>` | the single orphan sub-agent notice (R7) |
| `scope: Scope` | `Main` (no parent = top-level) or `Subagent(tool_use_id)` for #944 (research R7) |
| `rebuilds: u64` | count of rebuilds, so the client can prune fold keys after one |

Operations: `apply(omitted, &[SequencedEvent], live)` (replace-or-continue rule of #938 wire.md,
then one `push` per event, each followed by eviction); `entries()`; `omitted_notice() -> Option<u64>`;
`held() -> usize` (events held, at most `MAX_EVENTS`);
`is_empty()` (no event held: the view shows "No output yet", FR-006); `subagent_events(&str) ->
impl Iterator<Item = &SequencedEvent>`; `entry(EntryKey) -> Option<&Entry>`.

Invariants: `entries` are in the order of their opening `seq`; every held event is accounted for by
exactly one entry, one stream-control absorption, or one count (SC-001); an entry's `key` never
changes; `revision` strictly increases on each change to that entry and no other entry's
`revision` changes when an event affects one entry (FR-016, SC-002).

### OpenStream

`message_id: Option<String>`, `blocks: Vec<(u64 index, Option<EntryId>, finished: bool)>` in
`content_block_start` order; the entry is `None` for a block type the transcript does not stream
(its finished block is shown instead, keeping the positions matched). Closed by `message_stop`, `Result`,
`TurnEndedWithoutResult`, or a new `message_start`.

## Entry

`{ id: EntryId, key: EntryKey, opened_at: u64 (seq), revision: u64, kind: EntryKind }`

### EntryKey

| Variant | For | Stable because |
|---|---|---|
| `Tool(String)` | tool-call cards | the tool use id |
| `At { seq: u64, block: u32 }` | every other entry | `seq` of the opening event (the `content_block_start` for a streamed block) and the block's position in its message (`0` for single-item events) |

### EntryKind

| Variant | Fields | From |
|---|---|---|
| `SessionStart` | `model: Option<String>`, `cwd: Option<String>`, `resumed_session: String` | `Init` |
| `UserMessage` | `text` | `User` `Text` block, no parent |
| `AssistantText` | `text`, `streaming: bool` | `Assistant` `Text` block, or `content_block_start` text + `text_delta`s |
| `Thinking` | `text`, `streaming: bool` | `Thinking` block, or start + `thinking_delta`s |
| `ToolCall` | `ToolCard` | `ToolUse` block, or start + `input_json_delta`s |
| `OrphanResult` | `tool_use_id`, `parts: Vec<ResultPart>`, `is_error` | `ToolResult` whose card is not held (FR-020) |
| `Footer` | `TurnFooter` | `Result` |
| `TurnEndedWithoutResult` | — | daemon note (FR-004) |
| `Oversized` | `original_bytes: u64` | `OversizedOnWire` (FR-004) |
| `SubagentNotice` | `count: u64` | first sub-agent message naming no held card (FR-024) |
| `Raw` | `text: String`, `reason: RawReason` (`Unrecognised`, `Unreadable`, `UnknownBlock`, `UnknownStreamEvent`) | R4 |

### ToolCard

| Field | Meaning |
|---|---|
| `id`, `name` | from the tool use |
| `input: String` | JSON text; grows from `input_json_delta`s, replaced by the finished block's |
| `summary: String` | `input_summary(name, input)` (R6), recomputed when `input` changes |
| `status: ToolStatus` | `Running` · `Done` · `Failed` · `NoResult` |
| `result: Option<ToolResultView>` | `{ parts: Vec<ResultPart>, is_error: bool }` |
| `subagent_messages: u64` | FR-024; the header shows it while above zero |
| `streaming: bool` | input still arriving |

State transitions: `Running →(result, !is_error) Done`; `Running →(result, is_error) Failed`;
`Running →(Result / TurnEndedWithoutResult of its turn) NoResult`; `NoResult →(late result) Done |
Failed` (in place).

### ResultPart

`Text(String)` · `Unshown { kind: String }` (a block-list part it cannot show, named; FR-011).
Derived from the `User` event's `raw` (R5).

### TurnFooter

`outcome: Outcome { Success, Error }`, `duration_ms: Option<u64>`, `cost_usd: Option<f64>`,
`num_turns: Option<u64>`, `error_text: Option<String>` (Error only: `result` text, else
`subtype`). Display strings from `footer_figures(&TurnFooter) -> Vec<String>` (R14): omitted
figures yield no string (FR-005).

## Core functions

| Function | Rule |
|---|---|
| `input_summary(name: &str, input_json: &str) -> String` | R6 |
| `result_parts(user_raw: &str, block: usize, content: &str) -> Vec<ResultPart>` | R5 |
| `format_duration(ms: u64) -> String`, `format_cost(usd: f64) -> String`, `format_turns(n: u64) -> String` | R14 |

## View state (client, `features/structured_transcript.rs`; per window)

| Item | Meaning |
|---|---|
| `State.sessions: HashMap<SessionId, SessionTranscript>` | one per structured session the window has heard of; removed by `forget` (session deleted or forgotten) |
| `SessionTranscript.transcript: Transcript` | R1 |
| `SessionTranscript.open: HashSet<EntryKey>` | unfolded cards; absent = folded (FR-010); pruned to present keys after a rebuild (FR-013, FR-021) |
| `SessionTranscript.follow: Follow` | `Following` (initial, FR-018) · `Detached { offset: u32 }` (R9) |

Operations (render-free, tested in `tests/features_structured_transcript.rs`):
`apply(session, omitted, &events, live)`; `toggle(session, EntryKey)`; `is_open(session, &EntryKey)`;
`scrolled(session, offset, viewport, content) -> FollowChange` (`None` · `Detached` · `Resumed`);
`jump_to_latest(session)`; `follow(session) -> Follow`; `forget(session)`. A `Resumed` change and
`jump_to_latest` ask the shell for `snap_to(END)` (R9).

## Messages (client)

`TranscriptMsg { Toggle { session, key }, Scrolled { session, offset, viewport, content },
JumpToLatest { session }, Shown { session } }` (`Shown` from the scrollable's first-layout
`on_viewport_resize`, research R9), carried by the session message family; the shell turns
`FollowChange::Resumed`, `JumpToLatest` and a transcript pane being shown into scroll operations
on `transcript_scroll_id(session)`.

## Draw-side types (client, `ui/transcript/`; GUI exception)

`BlockRenderers`, `RenderCtx`, `BlockCache` and the per-kind inputs: see
[contracts/block-renderers.md](contracts/block-renderers.md).
