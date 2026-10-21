# Research: Structured Session Transcript View (#939)

Code pointers are as of `origin/main` at plan time (`7c076ce2`). #938's model is in
`crates/micold-core/src/structured/` (`event.rs`, `parse.rs`, `history.rs`), its plain log in
`crates/micold-client/src/features/structured_log.rs` and `ui/terminal.rs::structured_log_body`.

## R1. Where the transcript is built

- **Decision**: a new render-free module `micold_core::structured::transcript`. Its `Transcript`
  takes the same `StructuredEvents` messages the plain log takes (`apply(omitted, &[SequencedEvent],
  live)`), keeps the window's bounded event history, and maintains the ordered entries
  incrementally. The client keeps one `Transcript` per session plus its view state (fold set,
  follow-tail, scroll offset) in a render-free feature module `features/structured_transcript.rs`,
  which replaces `features/structured_log.rs`. Drawing is in `ui/transcript/`.
- **Rationale**: every decision of the spec (pairing, streaming, supersede, bounds, sub-agent
  counting, summaries, footer figures) is decision logic, so Principle I puts it in tested pure
  code; core is where the recorded fixtures already live (`tests/fixtures/stream-json/`), and
  #940–#944 then extend one model rather than a client-private one.
- **Alternatives**: build entries in the client feature module — rejected: the fixtures and parser
  are core's, and #944's usage work also reads results; build them in the daemon and send entries —
  rejected: the spec's *Client only* constraint (no new protocol message) and the per-window fold
  state need the events in the window anyway.

## R2. Entry identity and change tracking

- **Decision**: each entry has an `EntryKey` that is stable across a rebuild from the same history:
  a tool-call card is keyed by its tool use id (`EntryKey::Tool(String)`); every other entry by the
  `seq` of the event that opened it and the block's position in that message
  (`EntryKey::At { seq, block }`; for a streamed block, the `content_block_start`'s `seq` and
  `index`). Each entry also carries a `revision: u64` bumped on every change to it. Entries sit in a
  `VecDeque` addressed by a monotonic `EntryId` (index = id − first id), so eviction from the front
  is O(1) and an in-place update is O(1).
- **Rationale**: FR-013/FR-021 keep fold state "for cards still present" across a re-sync, so the
  fold key must not depend on position; #938's history numbering is stable for the same history
  (`EventHistory` invariant, R14 of #938), so `seq` is. `revision` is the cache key for per-entry
  lazy drawing (R12) and for #940's derived render cache (R10).
- **Alternatives**: index in the entry list — rejected, shifts on eviction and rebuild; a content
  hash — rejected, changes while a block streams.

## R3. Streaming and the finished message (FR-014, FR-015, SC-002)

What the recorded fixtures show (CLI 2.1.302): per message, `stream_event` `message_start` →
per block `content_block_start {index, content_block}` → `content_block_delta`s → the finished
`assistant` event (one block per event, the message's id) → `content_block_stop` →
`message_delta` → `message_stop`. The finished `assistant` event arrives **before** its
`content_block_stop`.

- **Decision**: an open *stream* starts at `message_start` (its `message.id` read from `raw`).
  `content_block_start` creates the entry at once — a text entry, a thinking card, or a tool card
  with its id and name read from the start event's `raw` `content_block` — and records it as the
  stream's block `index`. Deltas append to the entry of their `index` (`Delta::Text` /
  `Delta::Thinking` / `Delta::InputJson`). A finished `Assistant` event whose `message_id` matches
  the open stream (or either is unknown) supersedes the stream's streamed blocks **in order**: its
  k-th block replaces the content of the next not-yet-finished streamed block, in place, keeping
  that entry's key and position (the spec's assumption "matched by their position"). A finished
  block with no streamed counterpart (no stream open, a different message id, or more blocks than
  were streamed) appends a new entry. `message_stop`, `Result` and `TurnEndedWithoutResult` close
  the stream; a streamed block never finished keeps its partial text (edge case *Deltas without a
  finished message*).
- **Rationale**: matching by position is what the CLI's ordering guarantees; replacing in place
  keeps every earlier entry untouched (FR-015, FR-016) and makes "transcript from deltas ==
  transcript without deltas" checkable (SC-002), since the finished text wins.
- **Alternatives**: replace on `message_stop` — rejected, the finished event comes first and would
  be shown twice in between; key by `index` alone — rejected, `index` restarts at 0 per message;
  ignore deltas when a finished message follows — rejected, FR-014 needs growth before it.

## R4. Which events are absorbed, which are raw rows (FR-003, SC-001)

- **Decision**: modelled (they produce or change an entry, or are absorbed by the stream state):
  `Init`, `User`/`Assistant` blocks `Text`, `Thinking`, `ToolUse`, `ToolResult`, `Result`,
  `TurnEndedWithoutResult`, `OversizedOnWire`, and the stream-control `StreamDelta`s
  (`message_start`, `content_block_start`, `content_block_delta` with a typed delta or a
  `signature_delta`, `content_block_stop`, `message_delta`, `message_stop`, `ping`). Everything else
  is a raw row in its place: `Unrecognised` (e.g. `rate_limit_event`, `system` subtypes other than
  `init`), `Unreadable`, a `ContentBlock::Other`, a `StreamDelta` of another `event_type`, and a
  `Delta::Other` of another delta type. A raw row shows its whole original text, wrapped, as a
  muted monospace caption (`Text::monospace()`, R11) with a leading "Raw event" label, so it is
  distinct from conversation entries.
- **Rationale**: SC-001 counts every event as accounted for; the stream-control events are
  accounted for by the entries they build. `signature_delta` (thinking signature) carries nothing
  readable.
- **Alternatives**: hide the CLI's known-but-unmodelled lines (`rate_limit_event`, `status`,
  `task_summary`) — rejected by FR-003 ("no event may be silently dropped"); fold raw rows —
  rejected, FR-010 folds only cards.

## R5. Tool-call cards (FR-007, FR-008, FR-011, FR-020)

- **Decision**: a `ToolUse` (finished, or from `content_block_start`) creates or completes the card
  keyed by its id; an index `HashMap<String, EntryId>` finds it. A `ToolResult` in a `User` event
  (no parent tool use) fills the card's result and sets status `Done` or `Failed` (`is_error`), in
  place. A `Result` or `TurnEndedWithoutResult` sets every card of that turn still `Running` to
  `NoResult`. A result whose id has no card (its use was evicted, or never seen) becomes an
  `OrphanResult` entry, styled as a result without its call, never as an error.
  Result content: #938 keeps a block-list result as its JSON text, which is indistinguishable from
  a text result that happens to be JSON, so the transcript reads the shape from the `User` event's
  `raw` (`message.content[i].content` is a string or an array) and splits an array into
  `ResultPart::Text` and `ResultPart::Unshown { kind }` (e.g. `image`).
- **Rationale**: pairing by id is #938's rule (`pair_tool_calls`); deciding the content shape from
  `raw` is exact and needs no model or wire change (*Client only*).
- **Alternatives**: extend `ContentBlock::ToolResult` with a shape flag — rejected: a postcard wire
  change, out of scope; guess from the JSON text — rejected, misreads text results.

## R6. One-line input summary (FR-009)

- **Decision**: `input_summary(name, input_json) -> String` in core, by tool name: `Bash` →
  `command`; `Read`, `Write`, `Edit`, `MultiEdit` → `file_path`; `NotebookEdit` → `notebook_path`;
  `Grep`, `Glob` → `pattern`; `WebFetch` → `url`; `WebSearch` → `query`; `Task`, `Agent` →
  `description`. Any other tool, a missing field, or input that is not a JSON object (also a
  partial input while it streams) → the generic summary: the input's top-level `key: value` pairs
  joined by `, ` (strings unquoted), or the text itself when it is not an object. Whitespace runs,
  newlines included, collapse to one space and the result is capped at 240 characters with `…`; the
  card header draws it in the shared `Ellipsized` primitive, which cuts it to the width with an
  ellipsis, so it never wraps.
- **Rationale**: the cap bounds the shaping cost of a huge command; the widget does the exact cut.
- **Alternatives**: per-tool summary structs — rejected, one string is all the header shows and
  #940 renders the full input through its renderer anyway.

## R7. Sub-agent events (FR-024, for #944)

- **Decision**: an `Assistant` or `User` event with `parent_tool_use_id: Some(id)` creates no entry
  and changes no text, status or footer; it adds one to `subagent_messages` of the card keyed `id`,
  or, when no such card is held, to the transcript's single `SubagentNotice` entry (created at the
  first such event, in its place, and counting thereafter). A `StreamDelta` with a parent is
  absorbed: neither shown nor counted, and it never touches the main stream state. The events stay
  in the transcript's event history, and `Transcript::subagent_events(tool_use_id)` returns them in
  order. The builder is scoped: `Transcript::new()` (`Scope::Main`) treats events without a parent as
  top-level (this feature), and `Transcript::nested(tool_use_id)` (`Scope::Subagent(id)`) treats
  events whose parent is `id` as top-level — parented deltas included, so a sub-agent's stream runs
  through the same `stream.rs` — and everything else as foreign. #944 nests a sub-agent by building
  `Transcript::nested(id)` from `subagent_events(id)`; the event → entry rules are not duplicated.
- **Rationale**: the spec's clarifications; counting messages, not deltas, because a finished
  message supersedes its deltas.
- **Alternatives**: drop them — rejected, FR-024 keeps them for #944.

## R8. Bounds, eviction and re-sync (FR-004, FR-020, FR-021)

- **Decision**: `Transcript` keeps at most `MAX_EVENTS` (5,000, #938's bound) events, as the plain
  log does. When appending exceeds it, the oldest events leave, `omitted` grows by their number, and
  every entry whose opening `seq` is now below the first held `seq` leaves the front (a card keyed by
  its tool use leaves with its `ToolUse`'s event). The `OmittedNotice` is not an entry of the deque:
  it is derived (`omitted > 0`) and drawn first. The replace-or-continue rule of
  `structured_log::State::apply` moves into `Transcript::apply` unchanged (a `live: false` message
  rebuilds unless it continues the history in progress, wire.md of #938); a rebuild builds from the
  message's events from scratch. Fold state survives because it is keyed by `EntryKey` in the
  client's view state, not stored in entries; keys no longer present are pruned on rebuild.
- **Rationale**: the same bound and rule the plain log keeps today (FR-020), so the window's memory
  does not grow with the switch; dropping by opening `seq` makes "entries they made leave the top"
  exact.
- **Alternatives**: rebuild the whole transcript on each eviction — rejected, O(n) per event at the
  bound (SC-004); keep entries past the bound while the user is scrolled away — rejected, not in
  the spec and unbounded.

## R9. Follow-tail and scroll (FR-016–FR-018, SC-003)

- **Decision**: per window and session, `Follow::{Following, Detached { offset }}` in the client
  feature, starting `Following` (FR-018). The view's `material::Scrollable` has a per-session `Id`
  (`transcript-<session uuid>`) and reports `on_scroll_metrics(offset, viewport, content)`; the pure
  `Follow::scrolled(offset, viewport, content)` is at the end iff `offset + viewport + END_SLACK >=
  content` (`END_SLACK = 4` px, whole pixels, so momentum and fractional trackpad steps land on the
  end on every platform). A report away from the end detaches; one at the end follows. Following is
  held by iced's own scrollable state: `operation::snap_to(id, RelativeOffset::END)` stores a
  *relative* offset that is resolved against the new content height at each layout, so growth keeps
  the end in view with no task per event; a user scroll turns it into an absolute offset, so growth
  below no longer moves visible content (FR-016). The shell issues `snap_to(END)` when a session
  becomes Following (jump-to-latest, scrolling back to the end). Showing a transcript (first show,
  a session or pane switch, a tiled pane remounting the scrollable) is detected at layout, not in
  the update that switches: the scrollable sets `on_viewport_resize`, which fires on its first
  layout, as `TranscriptMsg::Shown { session }`; its handler issues `snap_to(END)` when `Following`
  (FR-018) or `scroll_to(AbsoluteOffset { y: offset })` when `Detached`, so the task never runs
  before the new scrollable exists. The "jump to latest" control shows while `Detached`.
- **Rationale**: relying on the scrollable's relative offset avoids a race between a snap task and
  the next layout; detection is one pure function testable without a GUI (SC-003, Principle VI).
- **Alternatives**: `Scrollable::anchor_bottom` — rejected: it keeps the distance from the bottom,
  so growth moves the content a detached user is reading; a `snap_to` after every event — rejected,
  one task per delta and a frame where the end is out of view. The relative-offset behaviour is
  pinned by a client test in the view milestone; if iced does not keep it, the fallback is that
  `snap_to` is re-issued after each applied message while `Following`.

## R10. The block renderer seam (FR-022, for #940)

- **Decision**: `ui/transcript/blocks.rs` defines one input type per block kind — `AssistantText`,
  `ThinkingText`, `ToolInput { tool, input_json }`, `ToolResult { tool, parts, is_error }` — each
  with the entry's `EntryKey`, `revision` and `streaming` flag, and a `BlockRenderers` struct of one
  `fn` per kind (`fn(&AssistantText<'_>, &RenderCtx<'_>) -> Element<'static, Message>`, …).
  `BlockRenderers::PLAIN` draws plain wrapped text for all four; `ui/transcript/` builds entries and
  calls the renderer for each block, never drawing block content itself. `RenderCtx` carries the
  roles and a reference to a per-window derived render cache (`BlockCache`, empty in #939) keyed by
  `(SessionId, EntryKey, revision)`, which #940 fills in update after each applied message (iced's
  markdown items must be parsed outside `view`).
- **Rationale**: FR-022 asks for a replaceable unit per block kind that leaves building, ordering,
  folding, streaming and scrolling untouched; a struct of typed `fn`s is swappable per kind, has no
  dynamic dispatch cost per frame, and the cache slot means #940 needs no change to the seam's shape.
- **Alternatives**: a trait object per kind — workable, but no state is needed that a `fn` plus the
  cache cannot carry; one renderer over a block enum — rejected, #940 would replace all kinds to
  change one.

## R11. Shared components (Principle VIII)

- **Decision**:
  - **Cards**: a new `material::DisclosureCard` — a surface-container card whose header is the
    existing `AccordionSection` header (a full-width text `Button`: Space/Enter with focus, focus
    ring, ripple, chevron) extended with `.leading(Icon)` and a `.trailing(Element)` slot, a one-line
    `Ellipsized` summary, and its body revealed by the existing `Accordion`. Builder:
    `DisclosureCard::new(title, on_toggle, roles).leading(icon).summary(s).trailing(status)
    .open(bool).body(element).failed(bool).into()`; `.failed(true)` draws the error outline.
  - **Status and failure**: `IconLabel` (icon + text label + tint) for card status and footer
    outcome; failure = `Icon::ToolFailed` + label "Failed" + the error role, so greyscale still
    reads (FR-012, SC-005).
  - **Footer**: a row of `IconLabel` and caption `Text`, composed in `ui/transcript/` from shared
    primitives (no new widget).
  - **Jump to latest**: the existing `Button::filled(…).leading(Icon::JumpToLatest)`, floated over
    the scrollable's bottom edge with iced `stack`.
  - **Icons** (new `Icon` variants in `icons.rs`, Material Symbols Outlined is shipped with full
    coverage): `Thinking` (`psychology`), `ToolCall` (`build`), `ToolRunning` (`progress_activity`),
    `ToolDone` (`check_circle`), `ToolFailed` (`error`), `ToolNoResult` (`do_not_disturb_on`),
    `JumpToLatest` (`arrow_downward`); `tests/icons.rs` pins their codepoints. `build`,
    `check_circle` and `error` are already `TypeChore`'s, `CardDone`'s and `SessionError`'s
    codepoints, and no two variants share one, so `ToolCall`, `ToolDone` and `ToolFailed` take the
    font's alias codepoints for the same glyphs (`U+F8CD`, `U+E92D`, `U+E001`), as `ActiveMarker` and
    `Unavailable` do (M4).
  - **Monospace text**: a `.monospace()` builder step on the shared `Text` (`Font::MONOSPACE`, as
    `DiffView` uses) for tool input, results and raw rows.
  - `DisclosureCard` gets a showcase entry (`src/showcase/catalogue.rs`, `tests/showcase_completeness.rs`).
- **Rationale**: the header behaviour FR-010 needs (pointer and keyboard toggle) already exists in
  `AccordionSection`; reusing it keeps one focus/press implementation.
- **Alternatives**: a transcript-local card widget — rejected by the component-reuse gate; using
  `AccordionSection` as is — rejected, it has no leading icon, status slot or card surface.

## R12. Performance (SC-004)

- **Decision**: each entry is drawn inside iced `lazy`, keyed by `(EntryKey, revision, open,
  scheme)`, so a delta rebuilds only the streaming entry's element and the other entries' element
  trees and text paragraphs are reused; column drawing already skips children outside the
  viewport. Building is incremental (R2, R8). SC-004 is measured by a new release-only test
  `crates/micold-client/tests/transcript_open_budget.rs`: it builds a 5,000-event transcript from
  repeated fixture turns with every card open, lays it out and draws one frame headless with the
  test support's renderer, and asserts under 1 s; and replays deltas into it, asserting each
  update + view + layout under 100 ms. It is `#[ignore]` in the debug suite and run in
  `.github/workflows/ci.yml`'s release-budget step beside `typeahead_budget` (the BUG-003 pattern).
- **Rationale**: iced 0.14 has no variable-height virtual list (`VirtualRows` needs fixed row
  heights); lazy entries are the least machinery that meets the budget.
- **Alternatives**: `VirtualRows` — rejected, entries have variable, width-dependent heights;
  measuring heights for a custom windowed list — kept as the fallback if the budget test fails.

## R13. The plain log's retirement; no protocol change

- **Decision**: in the milestone that shows the transcript, `features/structured_log.rs`, its test
  `tests/features_structured_log.rs`, `ui/terminal.rs::structured_log_body` and core's
  `log_line`/`omitted_line` (used only by the plain log) are removed; `DaemonMsg::StructuredEvents`
  is routed to the transcript feature in `shell/daemon_sync.rs` instead; the session-forget paths
  that call `structured_log.forget` call the transcript's. `PROTOCOL_VERSION` is unchanged.
- **Rationale**: FR-001 replaces the log; *Client only* forbids a wire change.

## R14. Footer figures (FR-005)

- **Decision**: `TurnFooter` holds `outcome` (`Success`/`Error`), `duration_ms`, `cost_usd`,
  `num_turns`, `error_text` as `Option`s from the `Result` event; core formats them:
  duration `"6.7 s"` below a minute, `"1 min 12 s"` above; cost `"$0.0372"` (four decimals) below
  $1, `"$1.24"` above; `"1 turn"` / `"2 turns"`. A figure the result omits is left out. The error
  text is the result's `result` text when present, else its `subtype` (e.g. `error_max_turns`).
- **Rationale**: the spec's assumptions on units; formatting in core keeps it tested.

## R15. Tiled panes and several windows (FR-019, Principle II)

- **Decision**: `ui/panes.rs` draws the transcript for a pane whose terminal is a structured
  session's primary (today it falls through to the empty-terminal message), the same as the single
  pane in `ui/terminal.rs`. View state is keyed by `SessionId` in the window's state, so two
  sessions side by side never share fold or scroll state, and two panes of one session share it.
  Each window holds its own feature state, as with the plain log.
- **Rationale**: the spec's *Several windows* edge case; the daemon already streams events to every
  viewing client (#938 FR-016).
