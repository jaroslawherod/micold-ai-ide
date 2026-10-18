# Research: Structured Session Mode (#938)

Code pointers are as of `origin/main` at plan time.

## R1. Where the kind lives and how it is fixed

- **Decision**: `micold_core::session::SessionKind { #[default] Terminal, Structured }`, a field
  `Session.kind` set only by the constructors: new `start_new_of_kind` and `restored_of_kind`,
  to which `start_new` and `restored` delegate with `Terminal` (about 300 existing callers, mostly
  tests, stay untouched, SC-004). `SessionKind::check(provider) -> Result<(), KindRefusal>` refuses
  structured for any provider whose `structured_args()` is `None` (FR-003).
- **Rationale**: `provider` already proves the pattern ("set at construction, no setter", session.rs
  `Session.provider` doc). FR-002 then holds by shape.
- **Alternatives**: a new `TerminalMode` variant — rejected, `TerminalMode` says which process is
  *displayed* and changes at runtime; the kind never changes and the Regular shells still exist in
  a structured session (spec edge case).

## R2. Launch arguments

- **Decision**: `AiCliProvider::structured_args(&self) -> Option<&'static [&'static str]>`, default
  `None`; Claude Code returns `["-p", "--input-format", "stream-json", "--output-format",
  "stream-json", "--verbose", "--include-partial-messages"]`. The structured spawn uses the same
  `LaunchSpec`, the same `launch_args(spec)` (`--session-id`/`--resume`), the same
  `activity_launch_for` (`--settings` hook file) and `tool_server_launch_for` (`--mcp-config`, last),
  with the structured args placed first. Same `with_launch_env` environment, same `ensure_cwd_exists`,
  same availability and recorded-conversation checks in `start_session_gated` (FR-006, US3 #4).
- **Rationale**: one seam decides who may be structured (FR-003); everything else is shared, so a
  resume, a missing CLI, a missing conversation and a missing folder report exactly as today.
- **Permissions (FR-006a)**: no `--permission-mode`, no `--allowedTools`, no
  `--dangerously-skip-permissions`. A test asserts the argv contains none of them.
- **Alternatives**: a separate structured `LaunchSpec`/arg builder — rejected, resume and hook
  arguments would be duplicated and could drift from the terminal path. A per-provider flag on
  `SessionKind` — rejected, the provider owns its argv.

## R3. Spawning over pipes

- **Decision**: `micold-daemon/src/structured.rs::StructuredSession`, spawned with
  `std::process::Command` (stdin/stdout/stderr piped, `process::no_window`, on Unix
  `process_group(0)` so `platform::ProcessTree::adopt(pid)` reaches the group as it does for a PTY
  child; on Windows the job object is adopted right after spawn as in `PtySession::spawn_answering`).
  Same environment hygiene as the PTY path: `strip_inherited_terminal_identity` and
  `prefer_process_path` are generalised from `CommandBuilder` to a small env-edit list both spawns
  apply. Liveness, exit outcome, kill and teardown mirror `PtySession` (`Supervised`-style child +
  tree under one mutex).
- **Threads**: a stdout reader (bytes → `LineSplitter` → `parse_line` → history + broadcast), a
  stderr reader (each line to `tracing::warn!(session, "claude stderr: …")`, never into events,
  FR-010), and the writer is the `ChildStdin` behind a mutex, written off the state lock.
- **Alternatives**: a `tokio::process` child — rejected, the PTY path is thread-based and
  supervision polls `is_alive()` synchronously; keeping the same shape lets `Proc` be one enum.

## R4. Line splitting

- **Decision**: core `LineSplitter` accumulates bytes, yields every `\n`-terminated line with one
  trailing `\r` removed (Windows CRLF), never splits a long line, and `finish()` yields a trailing
  partial line at EOF, which parses as an unreadable line (edge case). Invalid UTF-8 is decoded
  lossily into an unreadable line's text.
- **Rationale**: pure, platform-agnostic, unit-testable (Principle VI).
- **Alternatives**: `BufRead::lines` in the daemon thread — rejected, it fails the read on invalid
  UTF-8 (losing the line, FR-014) and keeps the `\r`; a max line length — rejected, tool results run
  to several MiB and the spec says lines are read whole.

## R5. Event history bound and sequence

- **Decision**: per session `EventHistory { next_seq: u64, events: VecDeque<SequencedEvent>,
  bytes: usize, omitted: u64 }`, bounded by both `MAX_EVENTS = 5_000` and `MAX_BYTES = 64 MiB` of
  retained line text; eviction drops from the front and adds to `omitted`. Each event carries a
  monotonic `seq` assigned at append. The map `SessionId → EventHistory` lives on `DaemonState`
  outside `LiveSession`, so stop/start within one daemon keeps it (FR-021a); deleting the session
  drops it. Saving it to disk, and restoring it after a daemon restart, is R14.
- **Rationale**: a count alone does not bound memory with multi-MiB tool results; bytes alone lets
  thousands of tiny deltas pile up. Partial-message deltas are many per turn, hence 5,000.
- **Alternatives**: unbounded history — rejected, FR-018. A time-based window — rejected, an idle
  session would lose its whole history. Spilling evicted events to disk — rejected: the bound is
  what keeps a restored history and its file bounded too (R14).

## R6. Carrying events: no gap, no duplicate, frame size

- **Decision**: viewing a structured session's `Primary` (`SetViewedTerminals`, the existing view
  mechanism) starts `stream_events` instead of `stream_view`. Under the history lock it takes a
  snapshot and subscribes to the session's `tokio::sync::broadcast` channel in the same critical
  section; it sends `DaemonMsg::StructuredEvents { session, omitted, events, live: false }` chunks,
  then forwards live events, skipping any `seq` ≤ the snapshot's last (FR-017, SC-005). A
  broadcast `Lagged` re-sends a fresh snapshot (never a silent gap). Chunks are cut so one frame's
  JSON stays under `MAX_FRAME_LENGTH / 2`; a single event larger than that is sent alone with its
  content replaced by a `truncated_on_wire` notice and its byte length, and the daemon logs it —
  the parser and history still hold it whole (spec: "read whole" is about the parser).
- **Live sending**: one message per event as it is read (SC-006), no tick batching; the client
  channel is already unbounded and ordered.
- **Alternatives**: polling a change counter like `stream_view` — rejected, it batches by tick and
  needs the seq bookkeeping anyway.

## R7. Turn state and user messages

- **Decision**: core `TurnState { Idle, InTurn }`, per live session. `accept(text, running, turn)
  -> Result<(), UserMessageRefusal>` refuses `Blank`, `NotRunning`, `TurnInProgress` (FR-008).
  Accepting sets `InTurn`; a `result` event sets `Idle`; process exit or restart resets to `Idle`
  and appends a daemon-originated `TurnEndedWithoutResult` event to history if it was `InTurn`
  (edge case). The check-and-set runs under the state lock, so of two simultaneous messages the
  first wins and the second sees `TurnInProgress`. Encoding: `{"type":"user","message":{"role":
  "user","content":[{"type":"text","text":<text>}]}}` + `\n`, `serde_json` escaping (multi-line,
  quotes, non-ASCII unchanged).
- **Wire**: correlated `ClientMsg::SessionUserMessage { req, session, text }` →
  `OperationOk`/`OperationError` with the refusal sentence.
- **Alternatives**: queueing messages sent mid-turn — rejected, FR-008 refuses them and queueing
  is #941's input design. Turn state in the client — rejected, two clients could both send.

## R8. First prompt

- **Decision**: `ops::start_with_prompt` branches on kind: a structured session delivers the prompt
  through the same `accept` + write path right after the spawn (stream-json input is read from the
  start, so no trust/readiness wait). Only the create path passes a prompt; supervision respawns,
  `SessionStart` and `SessionRestart` pass `None`, so the prompt goes once in the session's life
  (FR-007).
- **Alternatives**: passing the prompt as a `-p` argument — rejected, `--input-format stream-json`
  reads the prompt from stdin, and an argv prompt would be resent on every respawn (FR-007).

## R9. Parsing approach and dependencies

- **Decision**: `parse_line(&str) -> StructuredEvent`: `serde_json::from_str::<Value>`; failure →
  `Unreadable { text }`. Then dispatch on `type` (and `subtype` for `system`) into typed structs
  deserialized from the `Value` with `#[serde(default)]` only where the CLI omits a field and no
  `deny_unknown_fields` anywhere (FR-015); a typed deserialization failure → `Unrecognised { raw }`
  holding the original line (FR-012). Unknown content blocks → `ContentBlock::Other { raw }`.
  Known `type`s: `system`/`init`, `assistant`, `user`, `result`, `stream_event`. Other `system`
  subtypes are `Unrecognised` (kept whole).
- **Rationale**: `serde_json` is already a core dependency (`first_turn.rs`); no new crate.
- **Alternatives**: one typed `#[serde(tag = "type")]` enum deserialized straight from the line —
  rejected, one bad field loses the whole line instead of keeping it `Unrecognised`. `Value`-only
  events — rejected, every consumer would re-parse. A schema crate (`schemars`/generated types) —
  rejected, no published schema for the CLI's stream and a new dependency.

## R10. Client

- **Decision**: `StartMenuEntry` gains `kind: SessionKind`; `start_menu_entries` appends a
  "Claude Code (structured)" entry after Claude Code when Claude Code is supported, with the same
  availability; `start_affordance_offers_a_choice` becomes "more than one entry", so the chevron
  shows when Claude Code is the only CLI (FR-022). `SessionMsg::StartRequested` and
  `SessionCreate` carry the kind; every other path sends `Terminal`. A render-free
  `features/structured_log.rs` keeps per-session lines from `StructuredEvents` (snapshot replaces,
  live appends, `omitted` → leading marker line). `micold_core::structured::log_line` makes the one
  line per event. The pane draws those lines read-only with shared `Text` in a `Scrollable`; the
  sidebar row shows a "structured" caption with the existing row text roles.
- **Alternatives**: a separate toggle in the new-session flow — rejected by clarification Q2/Q4.

## R11. Recorded fixtures

- **Decision**: capture with the real CLI (`claude` 2.1.302 is installed in the dev container) by
  `scripts/capture-stream-json.sh <name> <prompt>` which runs the structured argv in a scratch git
  repo and saves stdout verbatim to `crates/micold-core/tests/fixtures/stream-json/<name>.jsonl`;
  the README records the CLI version and date. Required set (FR-024): `text-turn`,
  `tool-turn` (a `Read`/`Bash` call and result), `partial-deltas`, `error-result`, `resume-init`;
  plus a hand-built `tolerance.jsonl` (unknown type, extra fields, missing field, non-JSON line,
  CRLF) clearly marked synthetic (SC-002). Fixtures are scrubbed of account ids/paths by the script
  (replace with placeholders) before commit.
- **Risk**: capture needs a logged-in CLI. If the implementing unit has none, it escalates (cannot
  satisfy FR-024 with synthetic data).
- **Alternatives**: hand-written fixtures only — rejected, FR-024 requires recorded ones.
  Capturing in CI — rejected, CI has no account (FR-025).

## R12. Stand-in CLI for daemon tests

- **Decision**: `tests/support/structured.rs` compiles a stand-in `claude` (same rustc approach as
  `support/history.rs`) that: writes its argv to `argv-record`; replays a fixture named in
  `.fake-stream` (one line per `\n`, with optional `pause <ms>` and `exit <code>` directives),
  waiting for one stdin line before each turn after the first when told to; appends every stdin
  line to `stdin-record`; writes `stderr <text>` directives to stderr. No network, no account
  (FR-025).
- **Alternatives**: a shell-script stand-in — rejected, not portable to Windows (Principle VI). Mocking
  the spawn inside the daemon — rejected, it would skip the pipe, thread and process-tree code
  under test.

## R13. Protocol version

- **Decision**: bump `PROTOCOL_VERSION` once per milestone that changes the wire (M2 → 52, M3,
  M6; tasks.md *Milestones*), since each milestone merges separately and `tests/schema_hash.rs`
  ties every wire edit to a bump; new fields `#[serde(default)]`. An older client is
  refused by the existing handshake (spec edge case).
- **Alternatives**: no bump with defaulted fields only — rejected, an old client would silently
  drop `StructuredEvents` and show an empty pane for a structured session.

## R14. Saving the event history across a daemon restart

Clarification Q3, FR-021a–FR-021c, US7. Feature 041 (`specs/041-terminal-scrollback-persistence/`)
already saves a terminal session's history; this reuses its setting, its directory, its store and
its schedule, and adds only a second kind of file.

- **Setting**: `Settings.save_terminal_history` (041 data-model §8), unchanged on disk and on the
  wire. No new setting (clarification Q3), so no `SETTINGS_VERSION` or `PROTOCOL_VERSION` change.
- **Where**: the same directory, `history_dir()` (`terminal-history` under the per-user local data
  directory), and the same `HistoryStore`, which gains a second file kind: `<session uuid>.events`
  beside `<session uuid>.history`, written through `.<name>.tmp` by `owner_only::write` as today.
  `set_enabled(false)`, `purge`, `forget`, `sweep`, `retry_deletions` and the `undeleted`/`unlisted`
  rules cover both extensions; `last_written` is keyed by session and file kind. One mutex then
  orders an event save against the setting being turned off and against a removal (041 R9), and
  the sandbox needs nothing new: the container already has that directory (041 R15, same
  `create_dir` rule).
- **Format**: the 041 frame — magic, version, payload length, payload, SHA-256 of all of those,
  read capped and checked in the same order (041 contracts/saved-history-file.md §2, §4) — with its
  own magic `MICOLDSE` and `EVENTS_FORMAT_VERSION = 1`. The frame code in
  `terminal_history/format.rs` is split into a private `frame(magic, version, payload)` /
  `unframe(bytes, magic, version, cap)` that both formats use, so the terminal format's bytes and
  its pinned fixture test do not change. The payload is postcard
  `SavedEvents { omitted: u64, events: Vec<SavedEvent> }`, with
  `SavedEvent = Line(String) | Unreadable(String) | TurnEndedWithoutResult`: a CLI-sourced event is
  saved as its `raw` line and parsed again by `parse_line` on load; an unreadable line keeps its
  kind even when its text happens to parse (a partial last line); `OversizedOnWire` is wire only
  and never in a history. Read cap `MAX_EVENTS_FILE_BYTES = MAX_BYTES + 1 MiB` (the retained text
  plus postcard's per-event length and tag). A file over the cap, with another magic (a terminal
  history renamed), another version, a short body, a wrong checksum, a payload that does not
  decode, bytes left over or `omitted + events.len()` overflowing is `Damaged` with the existing
  `DamageReason`s, never partly loaded (FR-021c).
- **Sequence numbers**: `seq` starts at 0 and eviction is from the front only, so the first retained
  event's `seq` is always `omitted` and the next is `omitted + len`. The file therefore stores no
  `seq`. Core `EventHistory::restored(omitted, events)` rebuilds the history with those numbers
  under the current bounds (evicting further when the bounds are lower), so numbering continues
  after a restart and a viewer sees no duplicate at the seam (FR-021a). With nothing restored, a
  history starts at 0 in each daemon run, as before.
- **When it is saved**: the 041 saver tick (`SAVER_TICK` 5 s) also walks the event streams; each
  has a `SaveSchedule` (041 data-model §4) whose count is the history's `next_seq`, so a save is due
  only after a new event and at most every `SAVE_SPACING` (30 s); a write whose checksum equals the
  last one is `Unchanged`. A structured process end saves after its readers are joined, so a
  `TurnEndedWithoutResult` note is in the file. The orderly stop's `save_all_live` saves the event
  streams with the terminals, inside the same `STOP_SAVE_BOUND`. Encoding runs off the state lock
  on the blocking pool, from a snapshot taken under the history lock. Failures are logged once per
  session and reason and retried after `SAVE_SPACING`, as for terminals.
- **When it is restored**: lazily, the first time a daemon run needs the session's stream: its
  structured spawn, or a client viewing its primary (the view path from R6). Both go through one
  `DaemonState` call that, when `event_streams` has no entry, loads the file off the state lock and
  then inserts if still absent, so two first users agree on one stream. `Damaged` gives an empty
  history, one `warn!` with the session and the reason, and a schedule marked due so the next save
  replaces the file (FR-021c). Lazy, because a daemon with many stopped structured sessions should
  not read every history into memory at start.
- **Setting changes, removal, start**: off deletes every `.events` file with the `.history` files
  and leaves every in-memory history as it is (041's rule for `carried`); on marks every stream's
  schedule due. Removing a session forgets it in the store (both files) and drops its stream. At
  daemon start, `sweep(keep)` keeps `<id>.events` for the same non-archived sessions it keeps
  `<id>.history` for; saving off at start `purge`s both kinds.
- **Privacy**: the lines hold what the session showed (tool results, file contents), as a terminal
  history does: owner-only files in the local data directory, gated by the same setting whose note
  and guide entry are extended in the milestone that makes the kind selectable (FR-023).
- **Alternatives**: a separate setting — rejected by clarification Q3. A separate directory and
  store — rejected, it would duplicate 041's deletion, sweep and ordering rules and need a second
  sandbox path. Saving the typed events (postcard of `StructuredEvent`) — rejected, every change to
  the event model in #939–#944 would make saved files unreadable; the raw line is the CLI's own
  vocabulary and is re-parsed by the current parser. An append-only journal — rejected for now:
  eviction would need compaction, and rewriting the whole file through a rename is 041's
  crash-safe rule; the cost is at most one write of the bounded history per 30 s, only when it
  changed. Reading the CLI's own transcript after a restart — rejected: a different vocabulary (no
  partial deltas, no daemon notes), a file the app does not own, and not what the user chose.
