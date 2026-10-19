---
description: "Task list for Structured Session Mode (#938)"
---

# Tasks: Structured Session Mode — Claude over stream-json beside the Terminal Mode

**Input**: Design documents from `specs/938-structured-session-mode/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md),
[data-model.md](data-model.md), [contracts/](contracts/), [quickstart.md](quickstart.md)

**Tests**: Per Constitution Principle I (Test-First Development, NON-NEGOTIABLE), test tasks are
MANDATORY. In every phase the test tasks come first and must fail before the implementation task
they cover starts.

**Documentation**: Per Constitution Principle VII, the user guide is updated in the milestone that
makes the structured kind user-selectable (T047, T056, M8).

**Cross-platform**: Per Constitution Principle VI, line splitting, parsing, history, turn gating and
log text live in render-free `micold-core` and never branch on the OS; the pipe spawn goes through
`platform::ProcessTree` (R3).

**Organization**: Tasks are grouped by user story. US2 (P1) comes before US1 (P1) because US1's
daemon forwards what US2's parser produces. US6 is the selection-and-display story added for
FR-022, FR-022a and FR-023. US7 (P2, FR-021a–FR-021c) was added by the design revision that
answered clarification Q3 with "saved under Save terminal history"; its tasks are T049–T055, and
T056 joins US6. Its phase comes before US6's, as the UI milestone is last.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: the user story the task belongs to (US1–US7)

---

## Phase 1: Setup (recorded fixtures)

**Purpose**: the recorded CLI output every parser and daemon test replays (FR-024, R11).

- [X] T001 [P] Write `scripts/capture-stream-json.sh <name> "<prompt>" [--resume <id>]`: in a fresh
  scratch git repo it runs `claude -p --input-format stream-json --output-format stream-json
  --verbose --include-partial-messages` with `--session-id <new uuid>` (or `--resume <id>`), writes
  the prompt to stdin as one line `{"type":"user","message":{"role":"user","content":[{"type":"text","text":<prompt>}]}}`,
  saves stdout verbatim to `crates/micold-core/tests/fixtures/stream-json/<name>.jsonl`, and scrubs
  account identifiers, e-mail addresses, the session uuid and the home/scratch paths to fixed
  placeholders (`<SESSION_ID>`, `<HOME>`, `<CWD>`, `<ACCOUNT>`) with `sed`. Add
  `scripts/tests/capture-stream-json.test.sh` that runs the scrub step (factored as a function the
  test can source) on a canned line and asserts each placeholder replaces its secret.
- [X] T002 Record with T001 and the logged-in `claude` (2.1.302 in the dev container) the five
  fixtures FR-024 requires: `text-turn` (plain answer), `tool-turn` (a prompt that makes the CLI
  call `Read` or `Bash` once), `partial-deltas` (a longer answer, so `stream_event` deltas appear),
  `error-result` (a turn that ends in an error `result`, e.g. `--max-turns 1` with a prompt that
  needs a tool; any CLI-produced error subtype qualifies), `resume-init` (a second run with
  `--resume` of the `text-turn` conversation). Write
  `crates/micold-core/tests/fixtures/stream-json/README.md`: CLI version, capture date, each
  fixture's prompt and flags, that the files are scrubbed stdout, and that `tolerance.jsonl` (T003)
  is synthetic. No logged-in CLI available:
  escalate (R11 risk); never substitute hand-written files for these five.
- [X] T003 [P] Write the synthetic
  `crates/micold-core/tests/fixtures/stream-json/tolerance.jsonl` (SC-002; T002's README names it
  synthetic): in order, a real `system`/`init` line, a line of unknown `type`
  (`"type":"future_event"`), an `assistant` line carrying an extra unknown field at top level and
  inside a block, an `assistant` line with no `message` field, a `system` line with an unknown
  `subtype`, an assistant message containing a content block of unknown `type`, a line that is not
  JSON (`not json at all`), a valid `result` line terminated by `\r\n`, and a last line with no
  trailing newline.

---

## Phase 2: Foundational (session kind)

**Purpose**: the kind every story reads (FR-001, FR-002, FR-003, FR-004; R1, R2).

**⚠️ CRITICAL**: no user-story phase starts before this phase is done.

- [X] T004 [P] Write failing tests in `crates/micold-core/tests/session_kind.rs`:
  `SessionKind::default()` is `Terminal`; `Session::start_new_of_kind(location, provider, kind)` and
  `Session::restored_of_kind(…, kind)` keep the kind they are given, `Session::start_new` and
  `Session::restored` (signatures unchanged) give `Terminal`, and `Session` has no kind setter;
  `SessionKind::Structured.check(p)` is `Ok` for Claude Code and
  `Err(KindRefusal::NotSupported(p))` for Copilot, Pi, Codex and OpenCode, whose sentence names the
  CLI and says only Claude Code sessions can be structured; `Terminal.check(p)` is `Ok` for every
  provider; a `StoredSession` JSON without `kind` loads as `Terminal`, a `Terminal` record
  serialises with no `kind` key ("`#[serde(default, skip_serializing_if =
  "SessionKind::is_terminal")]`"), and a `Structured` record round-trips; the catalog's
  `schema_version` is unchanged.
- [X] T005 [P] Write a failing test in the test module of `crates/micold-core/src/provider.rs`:
  `AiCliProvider::structured_args()` is `Some(["-p", "--input-format", "stream-json",
  "--output-format", "stream-json", "--verbose", "--include-partial-messages"])` for Claude Code
  and `None` for every other provider.
- [X] T006 Make T004 and T005 pass: add `SessionKind { #[default] Terminal, Structured }` with
  `is_terminal`, `check` and `KindRefusal` in `crates/micold-core/src/session.rs`; add
  `Session::start_new_of_kind` and `Session::restored_of_kind`, and make `start_new` and
  `restored` delegate to them with `SessionKind::Terminal`, so their ~300 existing callers stay
  untouched (SC-004); add
  `StoredSession.kind` in `crates/micold-core/src/store.rs`; add `structured_args` (default `None`)
  in `crates/micold-core/src/provider.rs`. The whole workspace builds and `mise run gate` is green.

**Checkpoint**: the kind exists, persists and is checked; nothing uses it yet.

---

## Phase 3: User Story 2 — Nothing the CLI writes breaks the session (P1), parser part

**Goal**: every line the CLI writes becomes exactly one event, typed when known, kept whole
otherwise (FR-011–FR-015, FR-024).

**Independent Test**: `cargo test -p micold-core --test structured_parse` — every line of every
fixture becomes one event in order, known types typed, tool calls paired, the tolerance fixture
loses nothing.

### Tests for User Story 2 (parser) ⚠️

- [X] T007 [P] [US2] Write failing unit tests in `crates/micold-core/src/structured/lines.rs` for
  `LineSplitter` (R4): a line split across two `push` chunks comes out whole; one trailing `\r` is
  removed and a second is kept; an 8 MiB line comes out whole and unsplit; `finish()` yields a
  trailing partial line and nothing when the buffer is empty; invalid UTF-8 is decoded lossily
  rather than failing; an empty line between two lines is yielded as an empty line.
- [X] T008 [P] [US2] Write failing tests in `crates/micold-core/tests/structured_parse.rs` (SC-001,
  SC-002): for each of the five recorded fixtures, splitting the file with `LineSplitter` and
  parsing each line with `parse_line` gives exactly one event per line, in order, with no
  `Unreadable` among them and no `Unrecognised` for a line of a known type (the CLI also writes
  `active_goal`, `rate_limit_event`, other `system` subtypes: those are `Unrecognised`, SC-001); `text-turn` starts with `Init` (non-empty
  `session_id`, `model`, `cwd`) and ends with `Result { is_error: false, .. }` carrying
  `duration_ms`, `total_cost_usd` and `usage`; `tool-turn` has an `Assistant` `ToolUse` (`id`,
  `name`, `input`) and `pair_tool_calls` pairs every `ToolUse` with a `User` `ToolResult` of the
  same `tool_use_id`; `partial-deltas` has `StreamDelta` events with `event_type` and a text
  delta; `error-result` ends with `Result { is_error: true, .. }`; `resume-init`'s `Init`
  `session_id` equals `text-turn`'s. For `tolerance.jsonl`: unknown `type`, the message-less
  `assistant` and the unknown `system` subtype are `Unrecognised { raw }` with `raw` equal to the
  line; the extra-field `assistant` is a typed `Assistant`; the unknown block is
  `ContentBlock::Other { raw }`; `not json at all` is `Unreadable { text }`; the CRLF `result` is a
  typed `Result`; the last line (from `finish()`) is one event; every CLI-sourced variant's `raw`
  equals its line; no panic.

### Implementation for User Story 2 (parser)

- [X] T009 [US2] Make T007 pass: `crates/micold-core/src/structured/lines.rs` (`LineSplitter::push(&[u8]) ->
  Vec<String>`, `finish() -> Option<String>`), with `crates/micold-core/src/structured/mod.rs`
  and `pub mod structured;` in `crates/micold-core/src/lib.rs`.
- [X] T010 [US2] Make T008 pass: `crates/micold-core/src/structured/event.rs`
  (`StructuredEvent::{Init, Assistant, User, Result, StreamDelta, Unrecognised, Unreadable,
  TurnEndedWithoutResult, OversizedOnWire}`, `ContentBlock::{Text, Thinking, ToolUse, ToolResult,
  Other}`, `Usage`, `Delta`, `SequencedEvent { seq: u64, event }`, all `Serialize +
  Deserialize`, fields as in data-model.md, `Option` where the CLI may omit them, no
  `deny_unknown_fields`) and `crates/micold-core/src/structured/parse.rs` (`parse_line(&str) ->
  StructuredEvent` per R9: `Value` first, dispatch on `type`/`subtype`, typed deserialisation
  failure → `Unrecognised { raw }`; `ToolUse.input` and a block-list `ToolResult.content` kept as
  JSON text) plus `pair_tool_calls(&[SequencedEvent])`.

**Checkpoint**: M1 — the parser is proven on recorded fixtures.

---

## Phase 4: User Story 1 — Run a Claude session in structured mode (P1), scenarios 1–3

**Goal**: the daemon runs a structured Claude session over pipes and streams its events, typed and
in order, to every viewing client; sessions created without a kind are terminal sessions as
today (FR-003, FR-005, FR-006, FR-006a, FR-010, FR-014, FR-016, FR-017, FR-020; US2 scenario 5).
The lifecycle of the new primary process (US5 scenarios 1–2, FR-009) is built here, so its tests
are here too.

**Independent Test**: `cargo test -p micold-daemon --test structured_session` against the stand-in
CLI.

### Tests for User Story 1 (scenarios 1–3) ⚠️

- [x] T011 [P] [US1] Write failing unit tests in `crates/micold-core/src/structured/history.rs`
  for `EventHistory` (R5): `append` assigns `seq` from 0, strictly increasing; with bounds
  `with_bounds(max_events, max_bytes)` the oldest events are popped while "`events.len() >
  MAX_EVENTS || bytes > MAX_BYTES`" and `omitted` counts them; `snapshot()` returns `(omitted,
  events)`; `bytes` tracks the retained line text; the defaults are `MAX_EVENTS = 5_000` and
  `MAX_BYTES = 64 MiB`.
- [x] T012 [P] [US1] Write failing round-trip tests in `crates/micold-core/tests/structured_wire.rs`:
  `ClientMsg::SessionCreate` JSON without `kind` decodes with `SessionKind::Terminal`;
  `SessionSummary.kind` round-trips both kinds and defaults to `Terminal` when absent;
  `DaemonMsg::StructuredEvents { session, omitted, events, live }` round-trips with one event of
  every `StructuredEvent` variant.
- [x] T013 [P] [US1] Write the stand-in CLI in `crates/micold-daemon/tests/support/structured.rs`
  (R12), compiled the way `support/history.rs`'s `fake_cli` is and registered in
  `crates/micold-daemon/tests/support/mod.rs`: it writes its argv to `argv-record`, its cwd to
  `cwd-record` and the variables `TERM`, `PATH` and every `MICOLD_*` to `env-record` in its cwd;
  appends every stdin line to `stdin-record`; replays the fixture named in `.fake-stream` line by
  line to stdout, honouring the directives `pause <ms>`, `await-stdin` (block until one more stdin
  line), `stderr <text>` (write to stderr) and `exit <code>`; idles when the script ends.
- [x] T014 [US1] Write failing tests in `crates/micold-daemon/tests/structured_session.rs`:
  (a) a structured Claude session's argv begins with the seven structured args, carries
  `--session-id <id>`, the `--settings` hook file and `--mcp-config` last exactly as a terminal
  session at the same location (the `--settings` hook file is the only source of the activity and
  attention signals, so its presence keeps them working), and contains none of `--permission-mode`, `--allowedTools`,
  `--disallowedTools`, `--dangerously-skip-permissions`, `--permission-prompt-tool` (FR-005,
  FR-006, FR-006a); its `cwd-record` and `env-record` equal a terminal session's at that location
  (FR-006); (b) a client that views the session's `Primary` with `SetViewedTerminals` receives
  `StructuredEvents` whose events are the fixture's lines, one typed event each, in order, with
  strictly increasing `seq`, for each of the five recorded fixtures (SC-001); a second viewing client receives the same (FR-016);
  (c) with `pause 500` before the `result` line, the `Assistant` event reaches the client before
  the `result` is written (SC-006); (d) replaying `tolerance.jsonl` delivers every line, the
  unknown and unreadable ones kept, the trailing partial line as an event at exit, and the session
  keeps running until the stand-in exits (FR-014, SC-002); (e) a `stderr claude-stderr-marker`
  directive appears in the daemon's captured log and in no event (FR-010, US2 scenario 5, US5
  scenario 6); (f) `SessionCreate` with no `kind` creates a terminal session on the PTY path
  (US1 scenario 3); (g) `SessionCreate { kind: Structured }` for Copilot is refused with the
  `KindRefusal` sentence and writes no catalog record (FR-003, US5 scenario 3); (h)
  `SessionSummary.kind` is `Structured` for the structured session and `Terminal` for a terminal
  one (FR-020, US5 scenario 5); (i) two structured sessions running at once, in the same worktree
  and in two worktrees, each deliver only their own fixture's events (Principle II).

- [x] T015 [US5] Write failing tests in `crates/micold-daemon/tests/structured_lifecycle.rs`
  for the lifecycle the `Proc::Structured` primary of T020 must have (US5 scenarios 1–2, FR-009):
  the lifecycle states observed for stop, kill and restart of a structured session equal those of
  a terminal session in the same order; a stand-in that runs `exit 1` at once on every start is
  stopped by crash-loop protection after the same number of starts as a terminal session; kill
  ends the stand-in's process tree (its pid is gone); a missing `claude` binary reports the same
  failure as for a terminal session (edge case); events written before an exit stay in the
  history.

### Implementation for User Story 1 (scenarios 1–3)

- [x] T016 [US1] Make T011 pass: `crates/micold-core/src/structured/history.rs`.
- [x] T017 [US1] Make T012 pass in `crates/micold-core/src/protocol/messages.rs`:
  `SessionCreate.kind`, `SessionSummary.kind` (both `#[serde(default)]`),
  `DaemonMsg::StructuredEvents`; bump `PROTOCOL_VERSION` in
  `crates/micold-core/src/protocol/version.rs` by one (51 → 52 if `main` is still at 51) with a
  doc line naming feature 938's first wire delta; fix every exhaustive match the new variant
  breaks (`LSP findReferences`).
- [x] T018 [US1] Generalise the PTY spawn's environment hygiene in
  `crates/micold-daemon/src/supervisor.rs`: turn `strip_inherited_terminal_identity` and
  `prefer_process_path` into one env-edit list both a `portable_pty::CommandBuilder` and a
  `std::process::Command` can apply (R3); existing PTY tests pass unchanged (SC-004).
- [x] T019 [US1] Add `crates/micold-daemon/src/structured.rs` (`StructuredSession`, registered in
  `crates/micold-daemon/src/lib.rs` or `main.rs` beside `supervisor`): spawn with piped
  stdin/stdout/stderr, `process::no_window`, `process_group(0)` on Unix, the env-edit list of T018,
  and `platform::ProcessTree` adoption as `PtySession::spawn_answering` does; a stdout reader
  thread feeding `LineSplitter` → `parse_line` → a callback, calling `finish()` at EOF; a stderr
  reader thread logging each line with `tracing::warn!(session = %id, "claude stderr: {line}")`
  and never producing an event; `ChildStdin` behind a mutex for later writes; `is_alive`, exit
  outcome, `kill` and teardown with the same semantics as `PtySession`.
- [x] T020 [US1] Wire the kind through `crates/micold-daemon/src/catalog.rs` and
  `crates/micold-daemon/src/state.rs`: `Catalog::create_session` takes the kind and applies
  `SessionKind::check` before writing any record; the live primary becomes `Proc::{Pty,
  Structured}`, and every terminal-only operation on `Proc::Structured` returns `None` (refusal
  messages come in T036); `start_session_gated` branches on the kind, building argv as
  `structured_args()` + `launch_args(spec)` + `activity_launch_for` + `tool_server_launch_for`
  (last), with the same `with_launch_env`, `ensure_cwd_exists`, availability and
  recorded-conversation checks; a `SessionId → (EventHistory, broadcast::Sender<SequencedEvent>)`
  map on `DaemonState` outside `LiveSession`, appended to and broadcast from the reader callback;
  supervision (`supervise_session_exit`) treats both `Proc` variants alike.
- [x] T021 [US1] Make T014 pass in `crates/micold-daemon/src/server.rs`: `SessionCreate` passes its
  kind, summaries carry it, and viewing a structured session's `Primary` starts a
  `stream_events` task instead of `stream_view`: under the history lock take the snapshot and
  subscribe in the same critical section, send the history as `live: false` (first message carries
  `omitted`), then each new event as its own `live: true` message, skipping any `seq` ≤ the
  snapshot's last (R6).

**Checkpoint**: M2 — a structured session runs and streams in the daemon; not selectable in the UI.

---

## Phase 5: User Story 1 — Run a Claude session in structured mode (P1), scenarios 4–5

**Goal**: the first prompt and later user messages reach the CLI, once per turn, with refusals
(FR-007, FR-008).

**Independent Test**: `cargo test -p micold-daemon --test structured_session` (user-message cases)
and `cargo test -p micold-core structured::turn`.

### Tests for User Story 1 (scenarios 4–5) ⚠️

- [x] T022 [P] [US1] Write failing unit tests in `crates/micold-core/src/structured/turn.rs` and
  `crates/micold-core/src/structured/user_message.rs` (R7): `accept(text, running, turn)` refuses
  `""`, `"  \n\t"` with `Blank`, a stopped session with `NotRunning`, `InTurn` with
  `TurnInProgress`, and otherwise moves `Idle → InTurn`; a `Result` event moves `InTurn → Idle`;
  `on_exit(InTurn)` returns `Idle` plus a `TurnEndedWithoutResult` event, `on_exit(Idle)` none;
  every `UserMessageRefusal` (`Blank`, `NotRunning`, `TurnInProgress`, `NotStructured`) has a user
  sentence; `encode_user_message(text)` is exactly
  `{"type":"user","message":{"role":"user","content":[{"type":"text","text":<text>}]}}` + `\n`, and
  decoding it back yields the original text for multi-line, quoted and non-ASCII input.
- [x] T023 [P] [US1] Write a failing round-trip test in `crates/micold-core/tests/structured_wire.rs`
  for `ClientMsg::SessionUserMessage { req, session, text }`.
- [x] T024 [US1] Add failing cases to `crates/micold-daemon/tests/structured_session.rs`: a
  structured session created with a first prompt writes it as the first `stdin-record` line, and
  after stop/start and `SessionRestart` the record still holds it once (FR-007, US1 scenario 4); a
  `SessionUserMessage` with multi-line, quoted, non-ASCII text is written as one line that decodes
  to the same text, answered `OperationOk`, and the stand-in's response events (`await-stdin` then
  the rest of `text-turn`) reach two viewing clients (FR-008, US1 scenario 5); refused with the
  refusal sentence: a blank message, a message to a stopped session, a message before the turn's
  `result`, the second of two messages sent back to back, a message to a terminal session; after
  the `result`, a message is accepted again ("running then idle after the turn result"); a
  stand-in `exit 1` mid-turn yields a `TurnEndedWithoutResult` event and, once restarted, the next
  message is accepted; the text of a message never appears in the daemon log; a message to one of two running
  structured sessions reaches only that session's `stdin-record`.

### Implementation for User Story 1 (scenarios 4–5)

- [x] T025 [US1] Make T022 pass: `crates/micold-core/src/structured/turn.rs` and
  `crates/micold-core/src/structured/user_message.rs`.
- [x] T026 [US1] Make T023 pass: `ClientMsg::SessionUserMessage` in
  `crates/micold-core/src/protocol/messages.rs`; bump `PROTOCOL_VERSION` by one in
  `crates/micold-core/src/protocol/version.rs` with its doc line.
- [x] T027 [US1] Make T024 pass in `crates/micold-daemon/src/state.rs` and
  `crates/micold-daemon/src/server.rs`: a `TurnState` per live structured session (new live entry
  starts `Idle`); `SessionUserMessage` runs `accept` under the state lock and writes the encoded
  line to `ChildStdin` off the lock, replying `OperationOk`/`OperationError`; the reader callback
  sets `Idle` on `Result`; process exit or restart applies `on_exit` and appends its event to the
  history; never log the text.
- [x] T028 [US1] Make the first-prompt case of T024 pass (T027 and T028 together make T024 pass) in `crates/micold-daemon/src/ops.rs`:
  `start_with_prompt` delivers a structured session's prompt through the same accept-and-write
  path right after the spawn (no readiness wait, R8); supervision respawns, `SessionStart` and
  `SessionRestart` pass no prompt.

**Checkpoint**: M3 — US1 complete in the daemon.

---

## Phase 6: User Story 3 — Resume a structured session (P2)

**Goal**: a restart or a daemon restore resumes the same conversation in structured mode (FR-004,
FR-021, FR-021a).

**Independent Test**: `cargo test -p micold-daemon --test structured_resume`.

- [X] T029 [US3] Write failing tests in `crates/micold-daemon/tests/structured_resume.rs`: a new
  structured session's first argv has `--session-id <id>` and no `--resume` (US3 scenario 2);
  once its conversation is recorded (set up as the terminal resume tests do), stop/start and
  `SessionRestart` launch with the structured args and `--resume <same id>` (US3 scenario 1,
  SC-003); a daemon restart over the saved catalog (`service_saving`) lists the session with
  `kind: Structured` and starts it with `--resume <same id>` (US3 scenarios 1, 3; FR-004); the
  first prompt is not written again after either (FR-007); within one daemon, stop/start keeps
  earlier events in the history, while after a daemon restart with no history store (saving off,
  the test daemons' default) the history starts at the resume (FR-021a; the saving-on case is
  T054's); a resume whose conversation is missing on disk fails with the same lifecycle report
  and message as a terminal session's (US3 scenario 4).
  The cases red before T030 are the daemon-restart ones: until T030, a restored session comes
  back terminal (harmless while the kind is unreachable from the UI, before M8). Stop/start and
  `SessionRestart` resume through the `launch_args(spec)` path T020 shares, so those cases pin
  behaviour M2 built.
- [X] T030 [US3] Make T029 pass: restore passes `StoredSession.kind` into `Session::restored` in
  `crates/micold-daemon/src/state.rs` (and `crates/micold-daemon/src/catalog.rs` where the record is
  read), and the structured start path uses the same `launch_args(spec)` resume decision as the
  terminal one.
  Amended at M4: no source change was needed (ledger Decision 11): M1's
  `StoredSession::into_session` already passes the kind into `Session::restored_of_kind`, and M2's
  structured start already takes the terminal `LaunchSpec`; T029's cases are test-after, each
  pinned by a deliberate mutant (cycle-log 9).

**Checkpoint**: M4 — structured sessions survive restarts.

---

## Phase 7: User Story 4 — Late and multiple clients see the same stream (P2)

**Goal**: history-then-live with no gap and no duplicate, a bounded history with an omitted
marker, and frames that stay under the wire limit (FR-016, FR-017, FR-018; SC-005).

**Independent Test**: `cargo test -p micold-daemon --test structured_history`.

- [X] T031 [US4] Write failing tests in `crates/micold-daemon/tests/structured_history.rs`: for N
  in {0, 1, 7, the bound}, a client viewing after N events receives exactly those N events in
  `live: false` messages, then live events, with `seq` contiguous and none repeated (SC-005, US4
  scenario 1); two viewing clients both receive each new event (US4 scenario 2); with the history
  bound lowered through a test constructor (`DaemonState` with `EventHistory::with_bounds`), the
  oldest events are evicted and a client viewing afterwards gets `omitted > 0` in its first
  message (US4 scenario 3, FR-018); a viewer whose broadcast receiver lags (channel capacity
  lowered the same way) receives a fresh `live: false` snapshot and never a seq gap; an event whose
  encoding exceeds `MAX_FRAME_LENGTH / 2` is sent alone as `OversizedOnWire { original_bytes }`
  with its `seq`, and the daemon logs it; a history larger than `MAX_FRAME_LENGTH / 2` arrives in
  several `live: false` messages; deleting the session drops its history.
- [X] T032 [US4] Make T031 pass in `crates/micold-daemon/src/server.rs` and
  `crates/micold-daemon/src/state.rs`: chunk `live: false` messages under `MAX_FRAME_LENGTH / 2`,
  replace an oversize event by `OversizedOnWire` and `tracing::warn!` it, re-snapshot on
  `RecvError::Lagged`, drop the history map entry on session delete, and add the test-only bounds
  and capacity constructor.

**Checkpoint**: M5 — late and multiple viewers proven.

---

## Phase 8: User Story 5 — A structured session behaves like a session, and refuses what it cannot do (P2)

**Goal**: explicit refusals of terminal-only requests (FR-019); lifecycle parity was proven in
M2 (T015).

**Independent Test**: `cargo test -p micold-daemon --test structured_lifecycle`.

- [x] T033 [P] [US5] Write a failing round-trip test in
  `crates/micold-core/tests/structured_wire.rs` for `DaemonMsg::SessionRequestRefused { session,
  request, reason }` with each `RefusedRequest` (`Input`, `Resize`, `Scrollback`, `Interrupt`).
- [x] T034 [US5] Add failing tests to `crates/micold-daemon/tests/structured_lifecycle.rs`:
  `SessionInput`, `SessionResize`, `ScrollbackRequest` and `SessionInterrupt` addressed to the
  structured session's primary each produce one `SessionRequestRefused` with a reason to the
  sending client only, and the same requests to one of its shell instances are served (US5
  scenario 4, FR-019).
- [x] T035 [US5] Make T033 pass: `DaemonMsg::SessionRequestRefused` and `RefusedRequest` in
  `crates/micold-core/src/protocol/messages.rs`; bump `PROTOCOL_VERSION` by one in
  `crates/micold-core/src/protocol/version.rs` with its doc line.
- [x] T036 [US5] Make T034 pass in `crates/micold-daemon/src/server.rs`: send
  `SessionRequestRefused` where a terminal-only request targets `Proc::Structured`.

**Checkpoint**: M6 — the daemon side of the feature is complete.

---

## Phase 9: User Story 7 — A structured session's events survive a daemon restart (P2)

**Goal**: while **Save terminal history** is on, each structured session's event history is saved
beside the terminal histories through 041's store and schedule, and restored after a daemon
restart; off, nothing is written (FR-021a–FR-021c, SC-007; research R14).

Task IDs T049–T056 were allocated after T048 so the merged T001–T048 keep their numbers; the
phase sits before US6 because the UI milestone ships last.

**Independent Test**: `cargo test -p micold-core --test terminal_history_store`, `cargo test -p micold-core --lib structured::` and
`cargo test -p micold-daemon --test structured_saved_history`.

- [x] T049 [P] [US7] Write failing unit tests in `crates/micold-core/src/structured/saved.rs`:
  `encode_events`/`decode_events` round-trip a history holding every saved event kind (each
  CLI-sourced event comes back equal after `parse_line` of its `raw`; an `Unreadable` whose text is
  valid JSON stays `Unreadable`; `TurnEndedWithoutResult` kept; `omitted` kept); `OversizedOnWire`
  is never written; each damage reason — a terminal `.history` file's bytes (`NotAHistory`),
  another version, cut short, wrong checksum, payload that does not decode, bytes left over,
  `omitted + len` overflowing, over `MAX_EVENTS_FILE_BYTES` — returns its `DamageReason` and no
  events; a pinned version-1 fixture decodes, so a change of encoding fails until
  `EVENTS_FORMAT_VERSION` is bumped.
- [x] T050 [P] [US7] Write failing unit tests in `crates/micold-core/src/structured/history.rs`:
  `EventHistory::restored(omitted, events)` numbers the events from `omitted`, the next append gets
  `omitted + len`, and lower bounds evict further and add to `omitted`.
- [x] T051 [P] [US7] Write failing tests in `crates/micold-core/tests/terminal_history_store.rs` for
  the events file kind: `save_events`/`load_events` round trip; `Unchanged` on the same bytes;
  `Skipped` when off, forgotten or with no directory and `create_dir` false; `load_events` is
  `None` when off, forgotten or undeleted; `set_enabled(false)` and `purge` delete `.events` and
  `.events.tmp`; `forget` deletes both kinds; `sweep(keep)` keeps `<kept id>.events` and deletes
  others; a session's `.history` and `.events` saves do not mark each other `Unchanged`. Every
  existing test in the file keeps passing unchanged.
- [x] T052 [US7] Make T049 and T050 pass: new `crates/micold-core/src/structured/saved.rs`
  (registered in `structured/mod.rs`), `EventHistory::restored` in
  `crates/micold-core/src/structured/history.rs`, and the shared `frame`/`unframe` split out of
  `crates/micold-core/src/terminal_history/format.rs` with the terminal format's bytes and its
  pinned fixture test unchanged (`FORMAT_VERSION` stays 1).
- [x] T053 [US7] Make T051 pass in `crates/micold-core/src/terminal_history/store.rs`: `FileKind`,
  `save_events`, `load_events`, both extensions in the deletion, sweep and retry paths, and
  `last_written` keyed by session and kind.
- [x] T054 [US7] Write failing tests in `crates/micold-daemon/tests/structured_saved_history.rs`
  (a private history directory and the service setup of `tests/support/history.rs`): saving on, a
  structured session writes events, the daemon stops in order, a new daemon over the same catalog
  and directory restores it: a viewer of the stopped session gets the saved events with their
  `seq`s, and after a start the resumed run's events follow with `seq` continuing, none missing or
  repeated (US7 scenario 1, SC-007); a session stopped (process-end save) and the daemon then
  dropped without an orderly stop is restored the same way (scenario 2); saving off, no `.events`
  file exists after events, a stop and a restart, and the restarted history starts at the resume
  (scenario 3, SC-007); `SettingsSet { save_terminal_history: Some(false) }` deletes every
  `.events` file at once and the in-memory history a viewer then gets is unchanged; turning it on
  again saves the running history (scenario 4); removing the session deletes its `.events` file
  (scenario 5); a damaged `.events` file gives an empty history, one warning naming the session and
  the reason, and is replaced after the next save (scenario 6, FR-021c); an `<id>.events` file of a
  session not in the catalog, and its left-over `.<id>.events.tmp`, are deleted when the daemon starts
  (scenario 7, FR-021b), and with saving off at start every `.events` file is deleted; a removal or `save_terminal_history: Some(false)` racing a save leaves no
  `.events` file (scenario 8); turning saving on writes the whole in-memory history at the next save
  (scenario 9); one saver pass writes the `.events` file of a running session after a new event and
  none for a session with no new event since its last save (count = `next_seq`; the one-minute
  bound itself is 041's `SaveSchedule`, already tested); a terminal session writes no `.events`
  file and a structured one no `.history` file.
- [x] T055 [US7] Make T054 pass: in `crates/micold-daemon/src/state.rs` one call that creates an
  `event_streams` entry by loading off the state lock and inserting if still absent, used by the
  structured spawn and the view path, with `forget`, `sweep` and the setting change reaching the
  events files and marking stream schedules due; in `crates/micold-daemon/src/structured.rs`
  `EventStream::restored` and `saved_snapshot`; in `crates/micold-daemon/src/history.rs` the
  saver's event-stream schedules (count = `next_seq`), the save at a structured process end after
  its readers are joined, and the streams in `save_all_live`; in `crates/micold-daemon/src/server.rs`
  the view of a structured session whose stream is not loaded yet goes through that call.

**Checkpoint**: M7 — structured event history survives a daemon restart while saving is on.

---

## Phase 10: User Story 6 — Choose the structured kind and see its events (P3)

**Goal**: the "Claude Code (structured)" chevron entry, the row marker, the read-only log pane and
the user guide (FR-022, FR-022a, FR-023).

**Independent Test**: `cargo test -p micold-client --test structured_start_menu --test
structured_log --test layout_snapshot`, then quickstart §B.

### Tests for User Story 6 ⚠️

- [x] T037 [P] [US6] Write failing unit tests in `crates/micold-core/src/structured/log_line.rs`:
  `log_line(&SequencedEvent) -> String` gives one line per variant (init with model, assistant
  text/thinking/tool call by name, user tool result with its call id and error flag, result with
  outcome and duration, stream delta with its type, unrecognised with its `type` when it has one,
  unreadable with its text, turn ended without result, oversized on wire with its size); no line
  contains `\n` (multi-line text is flattened); `omitted_line(n)` names how many earlier events
  were omitted.
- [x] T038 [P] [US6] Write failing reducer tests in
  `crates/micold-client/tests/structured_start_menu.rs`: `start_menu_entries` lists "Claude Code
  (structured)" right after "Claude Code", with the same availability, only when Claude Code is
  supported; `start_affordance_offers_a_choice` is true with Claude Code as the only CLI; picking
  the entry sends `SessionCreate { kind: Structured }`; the plain start press and every other
  entry send `Terminal`.
- [x] T039 [P] [US6] Write failing reducer tests in `crates/micold-client/tests/structured_log.rs`:
  the first `live: false` message (on first view, or after a `live: true`) replaces the session's
  lines, later `live: false` messages append, `live: true` appends, `omitted > 0` puts the
  omitted line first, and two sessions' logs stay separate.
- [x] T040 [P] [US6] Write the failing geometry gate
  `crates/micold-client/tests/gates/structured_row_marker_fits.rs` (registered in
  `crates/micold-client/tests/layout_snapshot.rs` with `#[path = …]`): a structured session's
  sidebar row marker stays inside its row at the narrowest sidebar width.

### Implementation for User Story 6

- [x] T041 [US6] Make T037 pass: `crates/micold-core/src/structured/log_line.rs`.
- [x] T042 [US6] Make T038 pass in `crates/micold-client/src/features/session.rs`
  (`StartMenuEntry.kind`, `start_menu_entries`, `start_affordance_offers_a_choice`,
  `StartRequested` carrying the kind), `crates/micold-client/src/shell/daemon_sync.rs`
  (`SessionCreate.kind`), and every place that maps start-menu entries into the chevron's list or
  builds or handles `StartRequested`: `crates/micold-client/src/ui/mod.rs` (the chevron list from
  `start_menu_entries`), `crates/micold-client/src/ui/sidebar.rs` (the plain press and the chevron
  pick) and `crates/micold-client/src/main.rs` (its constructions and the handler); find them all
  with `LSP findReferences` on `StartRequested`. Existing tests that build `StartRequested`
  (`tests/session_start_press.rs`, `tests/session_default_no_worktree.rs`, `tests/app_state.rs`)
  get `kind: SessionKind::Terminal` added mechanically and must keep passing (SC-004); the
  `start_menu_entries` assertions in `src/main_tests.rs` and `tests/directory_availability.rs` are
  updated for the new "Claude Code (structured)" entry.
- [x] T043 [US6] Make T039 pass: new render-free `crates/micold-client/src/features/structured_log.rs`
  (registered in `features/mod.rs`) and routing of `StructuredEvents` to it in
  `crates/micold-client/src/shell/daemon_sync.rs`.
- [x] T044 [US6] Make T040 pass: the "structured" caption on a structured session's row in
  `crates/micold-client/src/ui/sidebar.rs`, with the existing row text roles (R10), and the same
  caption wherever else the client lists sessions with their AI CLI (find them with
  `LSP findReferences` on `SessionSummary.provider`); T038 asserts the row model's kind for each.
- [x] T045 [US6] Draw a structured session's pane in `crates/micold-client/src/ui/terminal.rs` as
  the read-only log of T043 (shared `Text` in a `Scrollable`, no input box, no keystroke routing),
  reusing shared primitives; a missing one goes into `ui/material/` with a builder API (R10).
- [ ] T046 [US6] Run quickstart [Part B](quickstart.md) steps 1–6 with the `visual-pass` skill and
  save its evidence under `specs/938-structured-session-mode/visual-pass/`.
- [x] T047 [US6] Add a "Structured sessions" section to
  `docs/user-guide/worktrees-and-sessions.md` (FR-023): what the structured kind is, how to choose
  it from the start-session chevron, that it is Claude Code only, the plain log it shows, and what
  it does not yet do with the issue for each (#939 transcript view, #940 rich rendering, #941
  prompt input and interrupt, #942 permission prompts, #943 slash commands and switching kinds,
  #944 usage and subagents); and that, while **Save terminal history** is on, the log is kept
  across a restart of the background service.
- [x] T056 [US6] Test first: in `crates/micold-client/tests/features_settings.rs` assert that
  **Save terminal history**'s note names structured sessions' events (fails on today's text); then
  make it pass by moving the note's literal in `crates/micold-client/src/ui/settings/terminal.rs`
  into a `pub const` the test reads and extending it, and extend the setting's entry in
  `docs/user-guide/settings.md`: it also keeps structured sessions' events on disk, and turning it
  off deletes them (FR-023).

**Checkpoint**: M8 — the structured kind is user-selectable and documented.

---

## Phase 11: Polish & Cross-Cutting Concerns

No code changes; done by the close unit in the close PR.

- [ ] T048 Run quickstart [Part A](quickstart.md) on `main` and record the results in
  `specs/938-structured-session-mode/quickstart.md`.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)** → **Foundational (Phase 2)** → **US2 parser (Phase 3)** → **US1 sc 1–3
  (Phase 4)** → **US1 sc 4–5 (Phase 5)** → US3 (6), US4 (7), US5 (8) → US7 (9) → **US6
  (Phase 10)** → Polish (11).
- T002 needs T001; T008 needs T002 and T003.

### User Story Dependencies

- **US2 (parser)**: Foundational only.
- **US1**: US2 (the daemon forwards parsed events).
- **US3, US4, US5**: US1 scenarios 1–3; US3 also needs US1 scenarios 4–5 (first prompt not
  resent).
- **US7**: US3 (a restored session keeps its kind) and US4 (a restored history reaches a late
  viewer in frames under the limit).
- **US6**: US1–US5 and US7 (the UI exposes a finished daemon side).

### Within Each User Story

- Tests first, failing; core before daemon; wire before the daemon handler that sends it.

### Parallel Opportunities

- T001 ∥ T003; T004 ∥ T005; T007 ∥ T008; T011 ∥ T012 ∥ T013; T022 ∥ T023; T049 ∥ T050 ∥ T051;
  T037 ∥ T038 ∥ T039 ∥ T040.

## Parallel Example: User Story 1 (scenarios 1–3)

```text
Task: "T011 EventHistory unit tests in crates/micold-core/src/structured/history.rs"
Task: "T012 wire round-trip tests in crates/micold-core/tests/structured_wire.rs"
Task: "T013 stand-in CLI in crates/micold-daemon/tests/support/structured.rs"
```

## Implementation Strategy

### MVP First

M1 (parser on recorded fixtures) and M2 (daemon streams a structured session) are the foundation
the rest of epic #945 consumes; M2 is the first point where a test client sees live events.

### Incremental Delivery

Each milestone below merges to `main` on its own. The structured kind stays unreachable from the
UI until M8; terminal sessions are unchanged throughout (SC-004). Each milestone that changes the
wire bumps `PROTOCOL_VERSION` once (M2, M3, M6).

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Tolerant parser over recorded fixtures 🎯 MVP

- **Tasks**: T001–T010
- **Deliverable**: `micold-core` turns every line of five recorded `claude` stream-json fixtures
  into one typed event, in order, with tool calls paired, and keeps unknown and malformed lines
  whole; sessions carry a persisted kind (always terminal so far).
- **Satisfies**: US2 acceptance scenarios 1–4; FR-001, FR-002, FR-004 (core), FR-011–FR-015,
  FR-024; SC-002 (parser), SC-004
- **Verify**: `cargo test -p micold-core --test structured_parse --test session_kind` and
  `scripts/tests/capture-stream-json.test.sh`
- **Depends on**: —
- **Tier**: full

### M2 — The daemon runs a structured session and streams its events

- **Tasks**: T011–T021
- **Deliverable**: a daemon test client sees a structured Claude session, spawned over pipes with
  the terminal session's arguments, cwd and environment, stream every fixture line as a typed
  event, in order, live; stderr goes to the log only; non-Claude structured requests are refused.
- **Satisfies**: US1 acceptance scenarios 1–3; US2 scenario 5; US5 scenarios 1, 2, 3, 5, 6;
  FR-003, FR-005, FR-006, FR-006a, FR-009, FR-010, FR-014 (CLI lines), FR-016,
  FR-017 (single attach), FR-020, FR-025; SC-001, SC-002, SC-004, SC-006
- **Verify**: `cargo test -p micold-daemon --test structured_session --test structured_lifecycle`
- **Depends on**: M1
- **Tier**: full

### M3 — First prompt and user messages

- **Tasks**: T022–T028
- **Deliverable**: a structured session receives its first prompt once and accepts a client's
  user message per turn, refusing blank, not-running and mid-turn messages with a reason.
- **Satisfies**: US1 acceptance scenarios 4–5; FR-007, FR-008, FR-014 (turn-ended note), FR-025
- **Verify**: `cargo test -p micold-daemon --test structured_session` and
  `cargo test -p micold-core structured::`
- **Depends on**: M2
- **Tier**: full

### M4 — Resume a structured session

- **Tasks**: T029–T030
- **Deliverable**: a restarted or daemon-restored structured session resumes the same Claude
  conversation in structured mode and keeps its kind; a new one never resumes.
- **Satisfies**: US3 acceptance scenarios 1–4; FR-004, FR-021, FR-021a, FR-025; SC-003
- **Verify**: `cargo test -p micold-daemon --test structured_resume`
- **Depends on**: M3
- **Tier**: full

### M5 — Late and multiple viewers

- **Tasks**: T031–T032
- **Deliverable**: a client viewing a structured session late gets its bounded history then live
  events with no gap or duplicate, told when events were omitted, in frames under the wire limit.
- **Satisfies**: US4 acceptance scenarios 1–3; FR-014 (oversized stand-in), FR-016,
  FR-017 (late attach), FR-018; SC-005
- **Verify**: `cargo test -p micold-daemon --test structured_history`
- **Depends on**: M2
- **Tier**: full

### M6 — Refuse terminal-only requests

- **Tasks**: T033–T036
- **Deliverable**: input, resize, scrollback and interrupt requests to a structured session's AI
  process are refused with a reason; its shell instances still serve them.
- **Satisfies**: US5 acceptance scenario 4; FR-019
- **Verify**: `cargo test -p micold-daemon --test structured_lifecycle`
- **Depends on**: M2
- **Tier**: full

### M7 — Event history survives a daemon restart

- **Tasks**: T049–T055
- **Deliverable**: with Save terminal history on, a daemon test client viewing a structured session
  after a daemon restart receives its earlier events, then the resumed run's, numbered without a
  gap or duplicate; with it off nothing is written; turning it off or removing the session deletes
  the saved file.
- **Satisfies**: US7 acceptance scenarios 1–6; FR-021a, FR-021b, FR-021c, FR-025; SC-007
- **Verify**: `cargo test -p micold-core --test terminal_history_store`, `cargo test -p micold-core --lib structured::` and
  `cargo test -p micold-daemon --test structured_saved_history`
- **Depends on**: M4, M5
- **Tier**: full

### M8 — Choose it in the app

- **Tasks**: T037–T047, T056
- **Deliverable**: the start-session chevron offers "Claude Code (structured)"; the session's row
  is marked and its pane shows a read-only line-per-event log; the user guide describes it.
- **Satisfies**: US6 acceptance scenarios 1–4; FR-022, FR-022a, FR-023
- **Verify**: `cargo test -p micold-client --test structured_start_menu --test structured_log
  --test layout_snapshot`, then quickstart §B (visual-pass evidence in
  `specs/938-structured-session-mode/visual-pass/`)
- **Depends on**: M3, M4, M5, M6, M7
- **Tier**: full

T048 (Polish) changes no code and is left to the close unit.
