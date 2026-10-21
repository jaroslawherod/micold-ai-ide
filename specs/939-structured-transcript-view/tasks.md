---
description: "Tasks for #939 — structured session transcript view"
---

# Tasks: Structured Session Transcript View — read-only assistant text, thinking and tool calls

**Input**: `specs/939-structured-transcript-view/` — [plan.md](plan.md), [spec.md](spec.md),
[research.md](research.md), [data-model.md](data-model.md), [contracts/](contracts/),
[quickstart.md](quickstart.md)

**Tests**: mandatory (Constitution I). Every behaviour task has a failing test task before it.
Draw code in `ui/transcript/` and the pane wiring use the GUI exception: geometry gates, the
release budget test and the quickstart Part B visual pass cover them.

**Documentation**: the user guide changes in M6, the milestone that makes the transcript reachable
(Constitution VII, FR-023).

**Cross-platform**: core never branches on the OS; end-of-scroll detection is in whole pixels with
`END_SLACK` (research R9).

**Fixtures**: the recorded files in `crates/micold-core/tests/fixtures/stream-json/` hold no echoed
user message, no thinking block and no sub-agent event; tests build those cases synthetically from
`StructuredEvent` values or hand-written stream-json lines.

**Organization**: phases follow the plan's delivery order (*Delivery*): the model in core first,
then the client's view state, the shared components, the draw code, and the pane switch last, so
no half-wired UI reaches `main` (milestones.md rule 6). Each task carries the story it serves.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: US1 (read as a conversation), US2 (cards), US3 (streaming and follow-tail)

---

## Phase 1: Setup

- [X] T001 Create the render-free module `crates/micold-core/src/structured/transcript/` with
  empty `mod.rs`, `entry.rs`, `build.rs`, `stream.rs`, `summary.rs`, `result.rs`, `figures.rs`
  and add `pub mod transcript;` to `crates/micold-core/src/structured/mod.rs` (plan *Project
  Structure*)
- [X] T002 [P] Move the fixture helpers `fixture_bytes`, `lines_of`, `events_of` and the fixture
  name list out of `crates/micold-core/tests/structured_parse.rs` into a shared
  `crates/micold-core/tests/support/stream_json.rs` (`#[path]`-included by both test files), and
  add `without_stream_events(&[String]) -> Vec<String>` (drops `"type":"stream_event"` lines) and
  a synthetic-event builder (`user_text`, `assistant_block`, `thinking_block`, `tool_result`,
  `with_parent`) used by every later core test; `structured_parse` stays green

---

## Phase 2: Foundational (blocks every story)

- [X] T003 Define the entry types in `crates/micold-core/src/structured/transcript/entry.rs` per
  [data-model.md](data-model.md) *Entry*: `EntryId`, `Entry { id, key, opened_at, revision, kind }`
  with `key` set only by its constructor (no setter, Constitution V), `EntryKey::{Tool(String),
  At { seq: u64, block: u32 }}`, the closed `EntryKind` (`SessionStart { model, cwd,
  resumed_session }`, `UserMessage { text }`, `AssistantText { text, streaming }`, `Thinking { text,
  streaming }`, `ToolCall(ToolCard)`, `OrphanResult { tool_use_id, parts, is_error }`,
  `Footer(TurnFooter)`, `TurnEndedWithoutResult`, `Oversized { original_bytes }`,
  `SubagentNotice { count }`, `Raw { text, reason }`), `ToolCard { id, name, input, summary,
  status, result: Option<ToolResultView>, subagent_messages, streaming }`, `ToolStatus::{Running,
  Done, Failed, NoResult}`, `ToolResultView { parts, is_error }`, `ResultPart::{Text(String),
  Unshown { kind }}`, `TurnFooter { outcome, duration_ms, cost_usd, num_turns, error_text }`,
  `Outcome::{Success, Error}`, `RawReason::{Unrecognised, Unreadable, UnknownBlock,
  UnknownStreamEvent}`; re-export them from `transcript/mod.rs`
- [X] T004 Write failing tests in `crates/micold-core/tests/structured_transcript.rs` for
  `Transcript::apply`'s replace-or-continue rule ([contracts/transcript-model.md](contracts/transcript-model.md)
  *Input*, unchanged from `features/structured_log.rs::State::apply`): an empty `live: false`
  message is a fresh empty transcript; a `live: false` message continuing `next_seq` of a
  history-only transcript appends; one that does not continue rebuilds and bumps `rebuilds`; a
  `live: true` message appends; `is_empty()` is true with no event held (FR-006);
  `omitted_notice()` is `Some(n)` iff `omitted > 0`, carrying the service's `omitted` (FR-004);
  `entry(&EntryKey)` finds an entry
- [X] T005 Implement `Transcript` in `crates/micold-core/src/structured/transcript/mod.rs`:
  fields of data-model.md *Transcript* (`events`, `omitted`, `next_seq`, `entries` as
  `VecDeque<Entry>` addressed by `EntryId - first_id`, `tools`, `stream`, `turn_cards`,
  `subagent_notice`, `scope`, `rebuilds`), `new()` (`Scope::Main`), `nested(tool_use_id)`
  (`Scope::Subagent`), `apply` (rule of T004, then one `build::push` per event; no bound yet),
  `entries()`, `omitted_notice()`, `is_empty()`, `entry()`, `revision`-bumping helpers used by
  `build.rs`; T004 passes

**Checkpoint**: an empty transcript applies messages; entries come next.

---

## Phase 3: User Story 1 — read a structured session as a conversation, model (P1) 🎯 MVP

**Goal**: finished events become session start, user message, assistant text, footers, notices
and raw rows, in order.

**Independent Test**: `cargo test -p micold-core --test structured_transcript` on `text-turn` and
`error-result` with their stream lines removed, plus `tolerance`.

### Tests for User Story 1 ⚠️ write first, see them fail

- [X] T006 [P] [US1] Unit tests in `crates/micold-core/src/structured/transcript/figures.rs`
  (`#[cfg(test)]`) for research R14: `format_duration` `"6.7 s"` below a minute and
  `"1 min 12 s"` above; `format_cost` four decimals below $1 (`"$0.0372"`) and two above
  (`"$1.24"`); `format_turns` `"1 turn"` / `"2 turns"`; `footer_figures(&TurnFooter)` leaves out
  every figure the result omits (FR-005)
- [X] T007 [US1] Fixture tests in `crates/micold-core/tests/structured_transcript.rs` (after T004,
  same file): `text-turn` without stream lines → `SessionStart` (model, cwd) … `AssistantText` …
  `Footer(Success)` with duration, cost and turns, in order, no `UserMessage` from the recording;
  a synthetic `User` text event → `UserMessage` in place; `error-result` without stream lines →
  `Footer(Error)` whose `error_text` is the result's `result` text, else its `subtype`
  (`error_max_turns`); `tolerance` → every `Unrecognised`/`Unreadable` line a `Raw` entry in its
  place with its original text and `RawReason`, a `ContentBlock::Other` a `Raw { UnknownBlock }`
  (FR-003); `TurnEndedWithoutResult` → its entry in place of a footer (US1 scenario 5);
  `OversizedOnWire` → `Oversized { original_bytes }` (FR-004); entries are in opening-`seq` order
  and keyed `At { seq, block }`

### Implementation for User Story 1

- [X] T008 [P] [US1] Implement `format_duration`, `format_cost`, `format_turns` and
  `footer_figures` in `crates/micold-core/src/structured/transcript/figures.rs`; T006 passes
- [X] T009 [US1] Implement the US1 rows of contracts/transcript-model.md *Event → entry rules* in
  `crates/micold-core/src/structured/transcript/build.rs`: `Init` → `SessionStart`; top-level `User`
  `Text` → `UserMessage`; top-level finished `Assistant` `Text` → `AssistantText` (no stream open:
  a new entry per block, `block` = its position); `Other` block → `Raw { UnknownBlock }`; `Result`
  → `Footer` (`TurnFooter` from the result's fields); `TurnEndedWithoutResult`, `OversizedOnWire`,
  `Unrecognised`, `Unreadable` → their entries; every `StreamDelta` goes to `stream.rs`, which in
  this milestone absorbs it without an entry (M2 replaces it); T007 passes

**Checkpoint**: a finished text turn reads as a conversation in the model.

---

## Phase 4: User Story 2 — tool-call and thinking cards, model (P1)

**Goal**: one card per tool use holding its result and status; thinking cards; one-line
summaries; sub-agent events counted, not shown.

**Independent Test**: `cargo test -p micold-core --test structured_transcript` on `tool-turn`
without stream lines and the synthetic card, thinking and sub-agent cases;
`cargo test -p micold-core --lib structured::transcript`.

### Tests for User Story 2 ⚠️ write first, see them fail

- [X] T010 [P] [US2] Unit tests in `crates/micold-core/src/structured/transcript/summary.rs` for
  research R6's `input_summary(name, input_json)`: `Bash` → `command`; `Read`/`Write`/`Edit`/
  `MultiEdit` → `file_path`; `NotebookEdit` → `notebook_path`; `Grep`/`Glob` → `pattern`;
  `WebFetch` → `url`; `WebSearch` → `query`; `Task`/`Agent` → `description`; an unknown tool, a
  missing field, a non-object or partial input → the generic `key: value, …` summary (strings
  unquoted) or the text itself; whitespace runs including newlines collapse to one space; capped at
  240 characters with `…` (FR-009)
- [X] T011 [P] [US2] Unit tests in `crates/micold-core/src/structured/transcript/result.rs` for
  research R5's `result_parts(user_raw, block, content)`: a string `content` in `raw` → one
  `Text`, even when it is JSON text; an array → its `text` blocks as `Text` parts in order and
  every other block as `Unshown { kind }` (e.g. `image`) (FR-011); `raw` unreadable → one `Text`
  of `content`
- [X] T012 [US2] Fixture and synthetic tests in `crates/micold-core/tests/structured_transcript.rs`
  (after T007, same file): `tool-turn` without stream lines → one `ToolCall` card per tool use,
  keyed `Tool(id)`, at the tool use's place, holding its result, `Done`, no result entry (FR-007,
  US2 scenario 1); an `is_error` result → `Failed` (scenario 3); a tool use alone → `Running`, its
  later result turns the same entry `Done`/`Failed` in place with a higher `revision` and no other
  entry's `revision` changed (FR-008, scenario 4); `Result` or `TurnEndedWithoutResult` sets that
  turn's `Running` cards to `NoResult`, and a late result then sets `Done`/`Failed` (scenario 6);
  a result with no held card → `OrphanResult`, not an error; a `Thinking` block → a `Thinking`
  entry (scenario 5); the card's `summary` is `input_summary` of its input; **G5** (FR-024): an
  `Assistant`/`User` event with a parent adds 1 to that card's `subagent_messages`, or to the
  single `SubagentNotice` (created at the first, in place) when no such card is held, and changes
  no text, status or footer; removing every parented event leaves the entries unchanged except the
  counts; `subagent_events(id)` returns them in order; `Transcript::nested(id)` treats events whose
  parent is `id` as top-level and ignores the rest

### Implementation for User Story 2

- [X] T013 [P] [US2] Implement `input_summary` in
  `crates/micold-core/src/structured/transcript/summary.rs`; T010 passes
- [X] T014 [P] [US2] Implement `result_parts` in
  `crates/micold-core/src/structured/transcript/result.rs` (reads `message.content[block].content`
  from `raw` with `serde_json`); T011 passes
- [X] T015 [US2] Implement the card rules in `crates/micold-core/src/structured/transcript/build.rs`:
  `ToolUse` → card + `tools` index + `turn_cards`; `Thinking` → `Thinking` entry; top-level
  `ToolResult` → fill card (`Done`/`Failed`, `result_parts`) or `OrphanResult`; `Result`/
  `TurnEndedWithoutResult` → `turn_cards` still `Running` → `NoResult`, then clear; parented
  `User`/`Assistant` → count on the card or the `SubagentNotice`, kept in `events`;
  `subagent_events(id)` and `Scope::Subagent` in `transcript/mod.rs`; T012 passes

**Checkpoint**: M1 done — the model holds every finished event of the recorded fixtures.

---

## Phase 5: User Story 3 — streaming, and the bound (core; T017/T019 are the bound edge cases)

**Goal**: deltas grow their entry in place, the finished message supersedes them by position; the
bound, eviction and re-sync keep entries exact and keys stable.

**Independent Test**: `cargo test -p micold-core --test structured_transcript` (G1–G4 on every
fixture as recorded).

### Tests ⚠️ write first, see them fail

- [X] T016 [US3] Streaming tests in `crates/micold-core/tests/structured_transcript.rs`: **G2**
  (SC-002) replaying `partial-deltas`, `tool-turn`, `text-turn`, `error-result` and `resume-init`
  one event at a time, after each event every entry before the streamed one is equal (key,
  revision, content) to before, and the final entries equal those of T007/T012's build without
  stream lines, `streaming` flags aside; each `text_delta` grows the same `AssistantText` (FR-014,
  US3 scenario 1); synthetic `thinking_delta`s and `input_json_delta`s grow a `Thinking` entry and
  a `Running` card created at `content_block_start` (scenario 3), the card's `summary` following
  the partial input; the finished message replaces the streamed text once, in place (FR-015,
  scenario 2); deltas never finished keep their partial text and `TurnEndedWithoutResult` closes
  the stream; a `StreamDelta` of an unknown `event_type` or delta type → `Raw
  { UnknownStreamEvent }`; a parented `StreamDelta` is absorbed, not counted; **G1** (SC-001) over
  every fixture as recorded: every event is an entry, an absorption or a count, every tool use one
  card holding its result
- [X] T017 [US3] Bound and re-sync tests in `crates/micold-core/tests/structured_transcript.rs`
  (after T016, same file): **G4** (FR-020) appending past `MAX_EVENTS` keeps `MAX_EVENTS` events,
  `omitted` counts every dropped one, entries opened by dropped events leave the front, a result
  arriving after its card's tool use left → `OrphanResult`; **G3** (FR-013, FR-021) rebuilding
  from the same history yields the same keys and no duplicate entry; the derived omitted notice
  sums the service's `omitted` and the dropped events

### Implementation

- [X] T018 [US3] Implement `OpenStream` in `crates/micold-core/src/structured/transcript/stream.rs`
  per research R3 and data-model.md *OpenStream*: `message_start` opens (closing an open one),
  `content_block_start` creates the streaming entry from the start event's `raw` `content_block`,
  typed deltas append to the entry of their `index`, `signature_delta`/`content_block_stop`/
  `message_delta`/`ping` absorbed, `message_stop` closes; in `build.rs`, a finished top-level
  `Assistant` block whose `message_id` matches the open stream (or either is unknown) supersedes the
  next unfinished streamed block in place, else appends; `Result` and `TurnEndedWithoutResult` close
  the stream; only the touched entry's `revision` moves; T016 passes
- [X] T019 [US3] Implement the bound in `crates/micold-core/src/structured/transcript/mod.rs`:
  after each `apply`, drop events past `MAX_EVENTS`, grow `omitted`, pop entries whose `opened_at`
  is below the first held `seq` (and their `tools`/`turn_cards`/`subagent_notice` references),
  O(1) per dropped entry (research R8); T017 passes

**Checkpoint**: M2 done — the model is complete and exact on every recorded fixture.

---

## Phase 6: View state in the client (US2 folding, US3 follow-tail)

**Goal**: one transcript per session per window, with its fold set and follow state, fed from the
wire beside the plain log.

**Independent Test**: `cargo test -p micold-client --test features_structured_transcript`.

### Tests ⚠️ write first, see them fail

- [ ] T020 [US2] Reducer tests in `crates/micold-client/tests/features_structured_transcript.rs`
  for data-model.md *View state*: `apply` then `transcript(session)` holds the entries; every card
  starts folded and `toggle` opens and folds it (FR-010); an open card stays open across new events
  and a session switch, and after a rebuild only keys still present stay open (FR-013, FR-021);
  `follow` starts `Following` (FR-018); `scrolled(offset, viewport, content)` away from the end →
  `Detached { offset }` and `FollowChange::Detached`, back within `END_SLACK = 4` px of the end →
  `Following` and `FollowChange::Resumed` (SC-003, FR-017); `jump_to_latest` → `Following`; two
  sessions share no entries, fold or follow state (FR-019); `forget` drops a session
- [ ] T021 [US3] Shell tests in `crates/micold-client/tests/features_structured_transcript.rs`
  (after T020, same file) driving the app's update: a `DaemonMsg::StructuredEvents` reaches the
  session's transcript (and still the plain log); `TranscriptMsg::Toggle` toggles;
  `TranscriptMsg::Scrolled` updates follow; `JumpToLatest`, a `Resumed` change and `Shown` while
  `Following` return a `snap_to(END)` task on `transcript_scroll_id(session)`, `Shown` while
  `Detached` a `scroll_to` of the stored offset (research R9); deleting or forgetting a session
  drops its transcript

### Implementation

- [ ] T022 [US2] Implement `crates/micold-client/src/features/structured_transcript.rs`
  (render-free, as `features/structured_log.rs`): `State { sessions: HashMap<SessionId,
  SessionTranscript> }`, `SessionTranscript { transcript, open: HashSet<EntryKey>, follow }`,
  `Follow::{Following, Detached { offset: u32 }}`, `END_SLACK`, `FollowChange`, and the operations
  of data-model.md; register it in `crates/micold-client/src/features/mod.rs`; T020 passes
- [ ] T023 [US3] Wire it: `State.structured_transcript` in `crates/micold-client/src/app.rs`
  beside `structured_log`; `crates/micold-client/src/shell/daemon_sync.rs` applies each
  `StructuredEvents` message to both and its two forget paths forget both; `TranscriptMsg`
  (data-model.md *Messages*) in the session message family with its handling and the
  `transcript_scroll_id(session)` (`transcript-<session uuid>`) scroll tasks in the shell; T021
  passes

**Checkpoint**: M3 done — the window holds each session's transcript and view state.

---

## Phase 7: Shared components (US2 cards)

**Goal**: the shared pieces the transcript draws with, in the material library and the showcase.

**Independent Test**: `cargo test -p micold-client --test icons --test showcase_completeness
--test material_builder_api --test layout_snapshot` and the in-crate material tests.

### Tests ⚠️ write first, see them fail

- [ ] T024 [P] [US2] Pin the new icons' codepoints in `crates/micold-client/tests/icons.rs`
  (research R11): `Thinking` `psychology`, `ToolCall` `build`, `ToolRunning`
  `progress_activity`, `ToolDone` `check_circle`, `ToolFailed` `error`, `ToolNoResult`
  `do_not_disturb_on`, `JumpToLatest` `arrow_downward`, distinct from each other
- [ ] T025 [P] [US2] In-crate tests for `Text::monospace()` (draws `Font::MONOSPACE`, as
  `DiffView`) in `crates/micold-client/src/ui/material/text.rs` and for `AccordionSection`'s
  `.leading(Icon)` and `.trailing(Element)` (both drawn in the header row, toggle and focus
  unchanged) in `crates/micold-client/src/ui/material/accordion_section.rs`
- [ ] T026 [P] [US2] Tests for `DisclosureCard`: in-crate tests in
  `crates/micold-client/src/ui/material/disclosure_card.rs` (header press and Space/Enter emit
  `on_toggle`, a Tab focus stop like `AccordionSection`'s; folded hides the body, open shows it;
  the summary is one line); its cases in `ui/material/style_outline_discipline.rs` (the failed
  outline is its one border), `ui/material/ripple_clipping.rs` (header ripple inside the card's
  shape) and `ui/material/composition_contrast.rs` (header label, muted summary and failed
  label/icon in the error role legible on the card fill, light and dark; SC-005); new gate
  `crates/micold-client/tests/gates/disclosure_card_header_fits.rs` wired into
  `crates/micold-client/tests/layout_snapshot.rs` like `structured_row_marker_fits.rs` (folded and
  open, failed and not: leading icon, one-line summary and trailing status inside the header row)

### Implementation

- [ ] T027 [P] [US2] Add the seven `Icon` variants in `crates/micold-client/src/icons.rs`; T024
  passes
- [ ] T028 [US2] Add `Text::monospace()` in `crates/micold-client/src/ui/material/text.rs` and
  `.leading(Icon)` / `.trailing(Element)` in
  `crates/micold-client/src/ui/material/accordion_section.rs`; T025 passes
- [ ] T029 [US2] Implement `DisclosureCard` in `crates/micold-client/src/ui/material/disclosure_card.rs`
  with the builder `DisclosureCard::new(title, on_toggle, roles).leading(icon).summary(s)
  .trailing(status).open(bool).body(element).failed(bool)` composed from `AccordionSection`,
  `Accordion` and `Ellipsized` (research R11), registered in
  `crates/micold-client/src/ui/material/mod.rs`; T026 passes
- [ ] T030 [US2] Add the `DisclosureCard` showcase entry (folded, open, running, failed with
  `IconLabel` status) in `crates/micold-client/src/showcase/catalogue.rs` and
  `crates/micold-client/src/showcase/sections/surfaces.rs`; `showcase_completeness` and
  `material_builder_api` pass

**Checkpoint**: M4 done — `DisclosureCard` is in the showcase.

---

## Phase 8: The transcript view and the renderer seam

**Goal**: `ui/transcript/` draws a session's transcript through one renderer per block kind,
lazily per entry, inside the SC-004 budget; shown in the showcase, not yet in the pane.

**Independent Test**: `cargo test -p micold-client --test transcript_renderer_seam --test
transcript_entry_view --test transcript_follow_layout`; `cargo test --release -p micold-client
--test transcript_open_budget`.

### Tests ⚠️ write first, see them fail

- [ ] T031 [P] [US2] Renderer-seam test `crates/micold-client/tests/transcript_renderer_seam.rs`
  (FR-022, contracts/block-renderers.md R-1, R-2): build the view over the `tool-turn` transcript
  with `assistant_text` swapped for a marker renderer; the marker appears once per
  `AssistantText` entry, the other kinds' output, the entry order and the fold state are unchanged
- [ ] T032 [P] [US1] Entry view-model test `crates/micold-client/tests/transcript_entry_view.rs`
  (FR-012): a `Failed` card and an `Error` footer carry the "Failed" label and `Icon::ToolFailed`
  in the error role; `Running`/`Done`/`NoResult` cards their icon and label; a card with
  `subagent_messages > 0` shows the count; a raw row carries the "Raw event" label; an empty
  transcript shows "No output yet" (FR-006); an omitted notice is first when `omitted > 0`
- [ ] T033 [P] [US3] Follow-layout test `crates/micold-client/tests/transcript_follow_layout.rs`
  pinning research R9 headless with the test support's renderer: after `snap_to(END)` the
  scrollable keeps the end in view as content grows; after a user scroll to an absolute offset,
  growth below leaves the visible content where it was (FR-016, US3 scenarios 4–5)
- [ ] T034 [P] [US3] Release budget test `crates/micold-client/tests/transcript_open_budget.rs`
  (`#[cfg_attr(debug_assertions, ignore = …)]`, as `micold-core/tests/typeahead_budget.rs`): a
  5,000-event transcript from repeated fixture turns with every card open lays out and draws one
  frame headless in under 1 s; each replayed delta's update + view + layout under 100 ms (SC-004,
  research R12); add `cargo test --release -p micold-client --test transcript_open_budget` to the
  release-budget step of `.github/workflows/ci.yml`

### Implementation

- [ ] T035 [US2] Implement `crates/micold-client/src/ui/transcript/blocks.rs` per
  contracts/block-renderers.md: `BlockRenderers` (four `fn` fields), `BlockRef`, `AssistantText`,
  `ThinkingText`, `ToolInput`, `ToolResult`, `RenderCtx`, empty `BlockCache` keyed by `(SessionId,
  EntryKey, revision)`, and `BlockRenderers::PLAIN` (shared `Text` at `TypeRole::Body`, wrapped;
  tool input and results `.monospace()`; `Unshown` parts a muted caption `[image not shown]`)
- [ ] T036 [US1] Implement `crates/micold-client/src/ui/transcript/entries.rs`: one function per
  `EntryKind` composing shared components only (`DisclosureCard` for thinking and tool cards with
  the status `IconLabel` and summary, the footer as an `IconLabel` + caption `Text` row from
  `footer_figures`, notices, raw rows as a muted monospace caption under "Raw event"), block
  content always through the `BlockRenderers` value, and the pure header/footer view model T032
  tests
- [ ] T037 [US3] Implement `view(state, session, renderers)` in
  `crates/micold-client/src/ui/transcript/mod.rs`: `material::Scrollable` with
  `.id(transcript_scroll_id(session))`, `.on_scroll_metrics` → `TranscriptMsg::Scrolled`,
  `.on_viewport_resize` → `TranscriptMsg::Shown`; each entry in iced `lazy` keyed by `(EntryKey,
  revision, open, scheme)`; the omitted notice first; "No output yet" when empty; the
  `Button::filled(…).leading(Icon::JumpToLatest)` control floated with `stack` over the bottom edge
  while `Detached`; register `ui::transcript` and add a transcript showcase sample built from
  `tool-turn.jsonl` in `crates/micold-client/src/showcase/sections/surfaces.rs`; T031–T034
  pass (if T034 fails, research R12's fallback applies: escalate before building it)

**Checkpoint**: M5 done — the transcript draws in the showcase within budget.

---

## Phase 9: The pane shows the transcript; the plain log goes

**Goal**: a structured session's pane, single and tiled, shows the transcript; the plain log is
removed; the user guide describes it.

**Independent Test**: `scripts/gate.sh`; quickstart Part B.

### Tests ⚠️ write first, see them fail

- [ ] T038 [P] [US3] Gate `crates/micold-client/tests/gates/transcript_jump_control_fits.rs`
  wired into `crates/micold-client/tests/layout_snapshot.rs` (the jump control lies inside the
  transcript viewport, above its bottom edge, over no part of the scrollbar), and a transcript pane
  state in `layout_snapshot.rs` (a `tool-turn` transcript, one card open, detached) with its
  reviewed fixture
- [ ] T039 [P] [US1] Pane-routing tests in a new `crates/micold-client/tests/transcript_pane.rs`
  (FR-001): a structured session's primary, single and tiled, draws `ui::transcript::view` (no
  plain-log row); a terminal session's pane is unchanged (US1 scenario 6, SC-006)

### Implementation

- [ ] T040 [US1] Route a structured primary (FR-001) to `ui::transcript::view` in
  `crates/micold-client/src/ui/terminal.rs` (removing `structured_log_body` and
  `shows_structured_log`) and in `crates/micold-client/src/ui/panes.rs` for a tiled pane (research
  R15); T038, T039 pass
- [ ] T041 [US1] Remove the plain log (research R13): `crates/micold-client/src/features/structured_log.rs`,
  `crates/micold-client/tests/features_structured_log.rs`, its `features/mod.rs` entry,
  `State.structured_log` in `app.rs` and its calls in `shell/daemon_sync.rs`; then
  `crates/micold-core/src/structured/log_line.rs` and its re-exports in `structured/mod.rs` after
  `LSP findReferences` shows no other user; `PROTOCOL_VERSION` unchanged
- [ ] T042 [US1] Rewrite *Structured sessions* in `docs/user-guide/worktrees-and-sessions.md`:
  the transcript in place of the log — entries, cards and folding, failed marks, raw rows, notices,
  follow-tail and "Jump to latest", sub-agent counts (FR-023)
- [ ] T043 [US3] Run quickstart Part B with the `visual-pass` skill and record it in
  `specs/939-structured-transcript-view/visual-pass/` (FR-010, FR-012, FR-016–FR-019, SC-005)

**Checkpoint**: M6 done — feature complete.

---

## Dependencies & Execution Order

- Phase 1 → Phase 2 → Phases 3–4 (M1) → Phase 5 (M2) → Phase 6 (M3) → Phase 9 (M6).
- Phase 7 (M4) needs no core task, but merges after M3 so one PR is open at a time; Phase 8 (M5) needs M1–M4; Phase 9 (M6) needs M5.
- T004 and every later core test use T002's `tests/support/stream_json.rs`.
- Within a phase: tests before the implementation they cover; tasks on the same file in ID order.
- `build.rs` is touched by T009, T015 and T018 in that order; `structured_transcript.rs` by T004,
  T007, T012, T016, T017 in that order.

### Parallel opportunities

- T002 beside T001; T006 beside T007 (T007 follows T004 in the same file); T010 and T011 beside T012; T013 and T014 together.
- Phase 7: T024, T025, T026 together; T027 beside T028.
- Phase 8: T031–T034 together. Phase 9: T038 beside T039.

## Implementation Strategy

Model first, in core, where the recorded fixtures are (M1, M2); then the window's view state
(M3); the shared components (M4); the draw code behind the renderer seam, measured against SC-004
before it is reachable (M5); and the switch of the pane last (M6), so `main` never shows a
half-built transcript and the plain log keeps working until the transcript replaces it.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Transcript model for finished events 🎯 MVP

- **Tasks**: T001–T015
- **Deliverable**: `micold_core::structured::transcript` turns a finished history into session
  start, user messages, assistant text, thinking and tool cards paired with their results,
  footers, notices and raw rows, with sub-agent events counted on their card
- **Satisfies**: US1 scenarios 1–5 and US2 scenarios 1, 3–6 at the model; FR-002, FR-003, FR-004,
  FR-005, FR-006, FR-007, FR-008, FR-009, FR-011, FR-024
- **Verify**: `cargo test -p micold-core --test structured_transcript --test structured_parse` and
  `cargo test -p micold-core --lib structured::transcript`
- **Depends on**: —
- **Tier**: full

### M2 — Streaming, supersede, bound and re-sync

- **Tasks**: T016–T019
- **Deliverable**: the model grows streamed entries in place, supersedes them with the finished
  message, stays bounded and rebuilds with stable keys; G1–G4 pass on every recorded fixture
- **Satisfies**: US3 scenarios 1–3 at the model; FR-004 (omitted count), FR-014, FR-015, FR-016 (revisions),
  FR-020, FR-021; SC-001, SC-002
- **Verify**: `cargo test -p micold-core --test structured_transcript`
- **Depends on**: M1
- **Tier**: full

### M3 — The window's transcript state

- **Tasks**: T020–T023
- **Deliverable**: each window keeps a transcript per structured session from the wire, with its
  fold set and follow state, and handles `TranscriptMsg` with the scroll tasks (beside the plain
  log, which still draws)
- **Satisfies**: US2 scenario 7, US3 scenarios 5–6 at the state; FR-010, FR-013, FR-017, FR-018,
  FR-019, FR-021; SC-003
- **Verify**: `cargo test -p micold-client --test features_structured_transcript`
- **Depends on**: M2
- **Tier**: full

### M4 — Shared disclosure card, icons, monospace text

- **Tasks**: T024–T030
- **Deliverable**: `material::DisclosureCard`, `AccordionSection` leading/trailing slots,
  `Text::monospace()` and the seven icons exist with their gates, and the card is in the showcase
- **Satisfies**: Constitution VIII; FR-010, FR-012 (components); SC-005 (contrast gate)
- **Verify**: `cargo test -p micold-client --test icons --test showcase_completeness --test
  material_builder_api --test layout_snapshot` and `cargo test -p micold-client --lib
  ui::material`; the showcase's surfaces section shows the card
- **Depends on**: M3
- **Tier**: full

### M5 — Transcript view behind the renderer seam

- **Tasks**: T031–T037
- **Deliverable**: `ui::transcript::view` draws a session's transcript through
  `BlockRenderers::PLAIN`, lazily per entry, within the SC-004 budget, visible in the showcase
  (not yet in the pane)
- **Satisfies**: FR-006, FR-012, FR-016, FR-022; SC-004; US3 scenarios 4–5 (layout behaviour)
- **Verify**: `cargo test -p micold-client --test transcript_renderer_seam --test
  transcript_entry_view --test transcript_follow_layout` and `cargo test --release -p
  micold-client --test transcript_open_budget`
- **Depends on**: M1, M2, M3, M4
- **Tier**: full

### M6 — The pane shows the transcript

- **Tasks**: T038–T043
- **Deliverable**: a structured session's pane, single and tiled, shows the transcript in place of
  the plain log, which is removed; the user guide describes it; quickstart Part B recorded
- **Satisfies**: US1 scenarios 1, 6; US2 scenarios 2, 7; US3 scenarios 4–6; FR-001, FR-017,
  FR-019, FR-023; SC-005, SC-006
- **Verify**: `cargo test -p micold-client --test transcript_pane --test layout_snapshot`;
  `scripts/gate.sh`; quickstart Part B (`specs/939-structured-transcript-view/visual-pass/`)
- **Depends on**: M5
- **Tier**: full
