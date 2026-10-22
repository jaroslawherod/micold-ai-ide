# Tasks: Structured Session Commands — slash commands, model switching and the terminal fallback

**Input**: Design documents from `specs/943-structured-commands-fallback/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md),
[data-model.md](data-model.md), [contracts/](contracts/), [quickstart.md](quickstart.md)

**Tests**: Mandatory (Constitution I). In every phase a test task comes before the implementation
task it covers and must fail first. Draw code is the GUI exception, validated by quickstart Part B.

**Documentation**: `docs/daemon.md` (wire and control requests) in the milestone that changes them
(M3, M4, M6); the user guide (FR-012, FR-027) in M8 (palette, model picker) and M9 (switching, the
terminal-only table), the milestones that make each user-visible.

**Organization**: every user story spans the core, the daemon and the window, and the window side
needs work not on `main` (#941's composer and queue, #942's `PromptCard`; plan *Integration
points*). Phases therefore follow the layers of plan.md *Delivery*, one phase per milestone, and
each task carries the story it serves. Phase 2 holds the core types every story needs.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: the user story (US1–US5) the task serves; Setup and Foundational tasks have none

Paths: `core` = `crates/micold-core`, `daemon` = `crates/micold-daemon`,
`client` = `crates/micold-client`.

---

## Phase 1: Setup — recorded fixtures (FR-026, research R13)

- [X] T001 [P] Extend `scripts/tests/capture-stream-json.test.sh`: with a stand-in `claude` on
  `PATH` that echoes its stdin, `scripts/capture-stream-json.sh --control '<json>' …` writes each
  `--control` line (repeatable, in order) to the CLI's stdin before the first user message, and the
  scratch repo it prepares holds `.claude/skills/greet/SKILL.md` and `.claude/commands/hello.md`;
  fails until T002
- [X] T002 Extend `scripts/capture-stream-json.sh` with the repeatable `--control <json>` option and
  the scratch project skill `greet` and custom command `hello` (T001 passes); the existing scrub
  step is reused, not copied
- [X] T003 Record with T002 against a logged-in Claude Code into
  `core/tests/fixtures/stream-json/`: `command-initialize.jsonl` (P-1), `command-skill.jsonl`
  (`/greet Ada`), `command-custom.jsonl` (`/hello world`), `command-compact.jsonl`
  (`/compact keep greetings` after one turn), `command-clear.jsonl` (one turn, `/clear`, one turn:
  the second `init` has a new `session_id`), `command-builtin-output.jsonl` (`/context`),
  `command-unknown.jsonl` (`/nosuchcmd`), `model-change.jsonl` (`set_model` `haiku` success, then
  one turn), `model-refused.jsonl` (`set_model` `no-such-model-xyz`, `error_code` `not_found`),
  `auth-failure.jsonl` (P-5: empty `HOME`, invalid `ANTHROPIC_API_KEY`) and
  `resume-after-clear.jsonl` (`--resume <post-clear id>`); add a row per file to
  `core/tests/fixtures/stream-json/README.md` with the CLI version (`2.1.307` or the one recorded)
  and the argv

---

## Phase 2: Foundational — events and notes every story needs (blocking)

- [X] T004 [P] Tests in `core/tests/structured_parse.rs` over the T003 fixtures: `Init` gains
  `slash_commands`, `skills`, `terminal_slash_commands` (each absent → empty); `Assistant` gains
  `model` (`"<synthetic>"` in `command-builtin-output.jsonl`) and `error`
  (`"authentication_failed"` in `auth-failure.jsonl`); `User` gains `is_synthetic` (the summary line
  of `command-compact.jsonl`); `system`/`compact_boundary` → `CompactBoundary { trigger,
  pre_tokens, post_tokens, raw }` (`manual`, 37839-like numbers from the fixture);
  `conversation_reset` → `ConversationReset { trigger: Some("clear"), raw }`; unknown keys ignored;
  every line still makes exactly one event (feature 938 FR-014); in `core/tests/structured_transcript.rs`
  the new events and notes make no entry and no raw row (until Phase 4 gives them entries)
- [X] T005 Implement T004 in `core/src/structured/event.rs` and `core/src/structured/parse.rs`
- [X] T006 [P] Tests for the daemon notes `CommandsReported { commands: Vec<ReportedCommand>,
  models: Vec<ReportedModel> }`, `ModelChanged { model }`, `ModelRefused { model, message }`,
  `KindSwitched { to: SessionKind }`: unit tests in `core/src/structured/saved.rs` that a history
  holding each round-trips through `encode_events` and its decoder unchanged and that `parse_line`
  never yields them; `line_len()` is `0` for each (data-model *Events*); round trip of each in a
  `StructuredEvents` frame in `core/tests/protocol_roundtrip.rs`
- [X] T007 Implement T006: the four variants and `ReportedCommand { name, description:
  Option<String>, argument_hint: Option<String>, aliases: Vec<String>, builtin: bool }`,
  `ReportedModel { value, display_name: Option<String>, description: Option<String> }` in
  `core/src/structured/event.rs`; their `SavedEvent` variants in `core/src/structured/saved.rs`;
  the transcript builder (`core/src/structured/transcript/build.rs`) absorbs the new events and
  notes (no entry, no raw row) until Phase 4; `CompactBoundary`/`ConversationReset` no longer reach
  the transcript as raw rows
- [X] T008 Bump `PROTOCOL_VERSION` by one in `core/src/protocol/version.rs` and re-pin
  `core/tests/schema_hash.rs` (contracts/wire.md *Events*; on a rebase conflict take the higher + 1,
  I-8)

---

## Phase 3: Core — command list, terminal-only, classify, palette, model, control (M1)

- [X] T009 [P] [US4] Unit tests in `core/src/structured/commands/terminal_only.rs`: `TERMINAL_ONLY`
  holds at least `config`, `mcp`, `agents`, `login`, `logout`, each with a non-empty `why`, no
  duplicate name; `CHECKED_AGAINST` is the recorded CLI version; `BUILT_INS` holds `compact`,
  `clear`, `context`, `usage` and none of `TERMINAL_ONLY` (FR-011)
- [X] T010 [P] [US1] [US2] Tests in `core/tests/structured_commands.rs` (new):
  `commands::parse_initialize(&ControlOutcome)` on `command-initialize.jsonl`'s answer gives every
  command with name, description, argument hint, aliases and `builtin`, and every model with
  `value`, `display_name`, `description`; entries with a blank or non-string `name`/`value` are
  dropped; descriptions are cut to 512 bytes at a char boundary; an `Error` outcome → empty.
  `CommandList::from_history` (R1): a name in the latest `init`'s `skills` → `Skill`, else
  `builtin: true` → `BuiltIn`, else `Custom`; `/model` is `App`; the alias `settings` of `config`
  and every `terminal_slash_commands` name are terminal-only; every `TERMINAL_ONLY` entry is listed
  even when unreported; a later `CommandsReported` replaces an earlier one (Edge Cases *Command list
  changes*); a skill and a custom command of one name are both kept; init lists alone → names
  without descriptions, kinds `Skill`/`Custom`; neither → `fallback()` with `ListSource::Fallback`
  (FR-007, FR-009, FR-011)
- [X] T011 [P] [US1] [US3] [US4] Unit tests in `core/src/structured/commands/mod.rs`:
  `classify_draft` table — `hello` → `Send`; `/` → `Send`; `/ x` → `Send`; `/speckit-plan #1` →
  `Send`; `/nosuchcmd` → `Send`; `/model` and `/model   ` → `Model(None)`; `/model haiku` →
  `Model(Some("haiku"))`; `/mcp`, `/settings`, `/doctor` (CLI-marked) → `TerminalOnly(name)`;
  `tell me about /mcp` → `Send`; `parse_invocation` keeps args as typed, including quotes, line
  breaks and non-ASCII text, and is `None` for the `Send` cases that are not `/name` (FR-001,
  FR-003, FR-010, FR-018)
- [X] T012 Create `core/src/structured/commands/{mod.rs,terminal_only.rs}` (`CommandKind`,
  `Command`, `CommandList`, `ListSource`, `TerminalOnly`, `TERMINAL_ONLY`, `CHECKED_AGAINST`,
  `BUILT_INS`, `parse_initialize`, `from_history`, `fallback`, `DraftCommand`, `classify_draft`,
  `CommandInvocation`, `parse_invocation`) until T009–T011 pass; register in
  `core/src/structured/mod.rs`
- [X] T013 [P] [US2] Tests in `core/tests/command_palette.rs` (new): `palette_query(draft, caret)`
  is `Some` for `/`, `/sp` (caret at end), `None` for `x/sp`, `/ sp`, `/sp x` with the caret after
  the space, an empty draft; `filter` orders FR-008's tiers (equal, prefix, word-prefix after `-`
  `_` `:`, rest), case-insensitive, alphabetical within a tier; an empty query lists all grouped
  Skill, Custom, BuiltIn, App; SC-002 for every name in `command-initialize.jsonl` and the
  fixtures' `init` lists (its shortest unique prefix lists it first; its full name lists it first);
  `accept(name)` gives `/<name> ` with the caret after it; 200 recorded-shape items filtered 1,000
  times, every single filter under SC-003's 100 ms (the maximum, not only the mean)
- [X] T014 Implement `core/src/structured/commands/palette.rs` (`palette_query`, `filter`,
  `PaletteSelection` over `typeahead::move_highlight`, `accept`) until T013 passes
- [X] T015 [P] [US3] Unit tests in `core/src/structured/model.rs`: `ModelName::parse` refuses blank,
  whitespace-holding and > 128-byte names (`ModelRefusal::Invalid`); `launch_args(Default)` is
  empty, `launch_args(Named("haiku"))` is `["--model", "haiku"]`; `MODEL_SHORTCUTS` = `sonnet`,
  `opus`, `haiku`; `session_model` is the latest of `Init.model` and `ModelChanged.model` over a
  history; `ModelChoice` serde: `Default` skipped, a missing field reads `Default`
- [X] T016 Implement `core/src/structured/model.rs` (`ModelChoice`, `ModelName`, `ModelRefusal`,
  `launch_args`, `MODEL_SHORTCUTS`, `session_model`) until T015 passes; register in
  `core/src/structured/mod.rs`
- [X] T017 [P] [US1] [US3] Unit tests in `core/src/structured/control.rs`: `encode_initialize(id)`
  and `encode_set_model(id, &choice)` give exactly the one-line JSON of
  [contracts/claude-commands.md](contracts/claude-commands.md) ending in `\n` (`Default` →
  `"model":"default"`); `parse_set_model` on `model-change.jsonl`'s answer → `Ok("claude-haiku-…")`,
  on `model-refused.jsonl`'s → `Err(Refused { message: "Model 'no-such-model-xyz' not found", code:
  Some("not_found") })`
- [X] T018 Implement T017 in `core/src/structured/control.rs`

---

## Phase 4: Core transcript — command turns, markers, eras (M2; the chip needs #941 M1, I-1)

- [x] T019 [P] [US1] Tests in `core/tests/structured_transcript.rs`: `command-builtin-output.jsonl`
  → the synthetic assistant text after the command's user turn is one `CommandOutput { text }`
  holding the markdown text unchanged, never an `AssistantText`; `command-compact.jsonl` → one
  `Compacted { trigger: Some("manual"), summary: Some(…) }`, the `is_synthetic` summary is not a
  user entry, every entry before it kept; `command-clear.jsonl` → one `Cleared`, entries before it
  `era 0`, after it `era 1`; `command-unknown.jsonl` → the model's reply as `AssistantText`;
  `auth-failure.jsonl` → its `AssistantText` has `auth_failed: true`, every other fixture `false`;
  a saved history of each restores to the same transcript (FR-004–FR-006, FR-022a)
- [x] T020 [US3] [US5] Tests in `core/tests/structured_transcript.rs`: a `ModelChanged` note →
  `ModelChanged { model, refused: None }`, a `ModelRefused` note → `refused: Some(message)`;
  `KindSwitched { to: Terminal }` then `KindSwitched { to: Structured }` → exactly one
  `TerminalPeriod` between the entries before and after, with no entry repeated or missing; an
  unclosed `to: Terminal` at the end → one `TerminalPeriod`; two periods → two (FR-015, FR-016,
  FR-023)
- [x] T021 Implement T019–T020 in `core/src/structured/transcript/entry.rs` (`CommandOutput`,
  `Compacted`, `Cleared`, `ModelChanged`, `TerminalPeriod`, `AssistantText.auth_failed`,
  `Entry.era`) and `core/src/structured/transcript/build.rs`
- [x] T022 [US1] Tests in `core/tests/structured_transcript.rs`: a `UserSent` note (#941 M1) with
  `/greet Ada` → `UserMessage { command: Some(CommandInvocation { name: "greet", args: "Ada" }) }`;
  plain text → `command: None`; the command's description and argument hint are those of the
  `CommandList` held at that point of the history (a later `CommandsReported` does not change an
  earlier chip) (FR-003)
- [x] T023 Implement T022 in `core/src/structured/transcript/build.rs` and `entry.rs`
- [x] T024 [US1] Draw the new `EntryKind` arms in `client/src/ui/transcript/entries.rs` with the
  existing pieces only (caption for `Cleared`, `ModelChanged`, `TerminalPeriod`; #940's markdown
  for `CommandOutput`; caption plus the summary as markdown for `Compacted`; `/name args` as the
  user text for a chip), each line's text from a render-free function with a unit test (as
  `oversized_text`); `client/src/features/transcript_rich.rs` arms likewise; M8 (T058) replaces the
  command, compaction, clear and model markers with `DisclosureCard`, M9 (T064) the
  `TerminalPeriod` caption (GUI exception for the draw glue)

---

## Phase 5: Daemon — command list at spawn, app-handled commands refused (M3)

- [x] T025 [US1] Extend the stand-in in `daemon/tests/support/structured.rs`: answer an
  `initialize` control request with `command-initialize.jsonl`'s recorded answer (a directive
  makes it silent or an `error`); replay a named fixture for a given user-message text
  (`/compact` → `command-compact.jsonl`, …); record every stdin line in order
- [x] T026 [P] [US1] Tests in `daemon/tests/structured_session.rs` and `daemon/tests/structured_permission_mode.rs`:
  extracting `StructuredSession::send_control(request_id, line) -> Result<ControlOutcome,
  ControlFailed>` leaves every existing `set_permission_mode` test passing unchanged; a stopped
  process → `ControlFailed::Stopped`, no answer within `CONTROL_ANSWER_BOUND` → `NoAnswer` (I-4)
- [x] T027 Implement T026 in `daemon/src/structured.rs`: `send_control`, `ControlFailed { NoAnswer,
  Stopped, Unsent }`, `set_permission_mode` built on it, `ModeChangeFailed` from `ControlFailed`
  plus `ControlOutcome::Error`
- [x] T028 [P] [US1] [US2] Tests in `daemon/tests/structured_commands.rs` (new): after a structured
  spawn the stand-in's first stdin line is one `initialize` request, before any user message; its
  answer is appended as one `CommandsReported` note, seen by a connected client and by a client
  that connects later (handshake replay, FR-009); a silent or refused `initialize` adds no note
  and the session keeps running and takes messages; a first prompt `/greet Ada` reaches stdin
  unchanged as one user message (FR-002); `/hello world`, `/speckit-plan #1` and `/nosuchcmd` sent
  as `SessionUserMessage` reach stdin unchanged (FR-001, SC-001 at the daemon); two structured
  sessions each get their own note (Principle II)
- [x] T029 [US3] [US4] Tests in `daemon/tests/structured_commands.rs`: `SessionUserMessage`
  with `/model`, `/model haiku`, each `TERMINAL_ONLY` name and `/settings` → `OperationError`
  ("/model is handled by the app" / "/<name> needs the terminal"), the stand-in's stdin unchanged,
  no event appended (FR-010, FR-018, SC-006)
- [x] T030 Implement T028–T029: write `encode_initialize` through `send_control` right after each
  structured spawn and append `CommandsReported` from `parse_initialize` in
  `daemon/src/structured.rs` / `daemon/src/state.rs`; classify `SessionUserMessage` text with
  `classify_draft` over the session's `CommandList::from_history` before it reaches the process,
  in `daemon/src/state.rs`
- [x] T031 [P] Describe the `initialize` request at spawn, the `CommandsReported` note and the
  `SessionUserMessage` refusals in `docs/daemon.md`

---

## Phase 6: Daemon — model choice, `--model`, `set_model` (M4)

- [x] T032 [P] [US3] Tests in `core/tests/session_store.rs` and `core/tests/protocol_roundtrip.rs`:
  a legacy `StoredSession` without `model` loads `Default`; `Named` round-trips and `Default` is
  not written; `SessionSummary.model`, `SetSessionModel { req, session, model }` round-trip;
  `schema_hash` re-pinned after `PROTOCOL_VERSION` +1 (FR-017)
- [x] T033 [P] [US3] [US5] Unit tests in `core/src/session.rs` and `core/src/provider.rs` (Claude's
  launch arguments): `Session::resume_handle()` returns the session id; a `LaunchSpec` with
  `model: Named("haiku")` gives `--model haiku` in both kinds and with `Default` no `--model`;
  `LaunchMode::Resume` uses `LaunchSpec.resume_handle` for `--resume` (FR-017, FR-020)
- [x] T034 Implement T032–T033: `Session.model`, `Session::resume_handle()` in `core/src/session.rs`;
  `StoredSession.model` in `core/src/store.rs`; `LaunchSpec.model`, `.resume_handle` in
  `core/src/terminal.rs` and their use in `core/src/provider.rs`; `SessionSummary.model`,
  `SetSessionModel` in `core/src/protocol/messages.rs`; `PROTOCOL_VERSION` +1 in
  `core/src/protocol/version.rs`; every `LaunchMode::Resume` site in `daemon/src/state.rs`,
  `daemon/src/server.rs`, `daemon/src/ops.rs` and `daemon/src/mcp/tools.rs` fills the spec from
  `Session::resume_handle()` and the model choice
- [x] T035 [US3] Extend the stand-in in `daemon/tests/support/structured.rs`: answer `set_model`
  from a scripted accept list with `model-change.jsonl`'s success shape, refuse others with
  `model-refused.jsonl`'s error; report the accepted model in the next `init`; record its argv
- [x] T036 [P] [US3] Tests in `daemon/tests/structured_model.rs` (new): idle `SetSessionModel` →
  one `set_model` stdin line, same pid, a `ModelChanged` note seen by two clients within 1 s
  (SC-004), `OperationOk`, the summary's `model` and the catalog file hold the choice; busy → no
  control line until the turn's `result`, then the line before any further user message, the turn
  in progress unchanged, `OperationOk` at once (FR-015); refused idle → `OperationError` with the
  CLI's text and the summary back at the previous choice; refused deferred → a `ModelRefused` note
  (FR-016); not running → persisted only and the next launch's argv has `--model`; a terminal
  session → `OperationError` and the choice persisted; invalid name → `OperationError`; two
  concurrent picks → the last one wins in the summary and on stdin (Edge Cases); restart and
  daemon restore → argv has `--model <choice>` and `--resume <id>` (FR-017); a pending #942
  request stays pending across a pick (Edge Cases); session A's pick never touches session B
- [x] T037 Implement T036: `Catalog::set_model` in `daemon/src/catalog.rs`; the per-session
  `PendingModel` slot, the idle write through `send_control`, the turn-end write on #942's
  `Result` path, the notes and the restore on refusal in `daemon/src/state.rs`; dispatch
  `SetSessionModel` in `daemon/src/server.rs`
- [x] T038 [P] Update the structured argv assertion in `daemon/tests/structured_session.rs` only as
  plan *Superseded assertions* allows (nothing for `Default`), and describe `SetSessionModel`, the
  deferred write and the model notes in `docs/daemon.md`

---

## Phase 7: Daemon — conversation handle after `/clear` (M5, provisional: ledger decision 2)

- [ ] T039 [P] [US1] [US5] Tests in `core/tests/session_store.rs` and unit tests in
  `core/src/session.rs`: `StoredSession.conversation` absent → `None`, round-trips, not written when
  `None`; `resume_handle()` returns `conversation` when set, else the session id (FR-005a)
- [ ] T040 [US1] Extend the stand-in in `daemon/tests/support/structured.rs`: after a `/clear` user
  message, the next `init` carries a new `session_id` (from `command-clear.jsonl`) and the stand-in
  writes a transcript file under that id in its config dir
- [ ] T041 [P] [US1] [US5] Tests in `daemon/tests/structured_resume.rs`: after `/clear` the
  catalog's `conversation` is the new id; a restart and a daemon restore launch with
  `--resume <new id>` and the same `SessionId`; the discovery pass adopts no second session for the
  post-clear transcript (`known_session_ids` includes the handle); a session without `/clear`
  resumes its own id as before
- [ ] T042 Implement T039–T041: `Session.conversation`, `resume_handle()` returning it
  (`core/src/session.rs`), `StoredSession.conversation` (`core/src/store.rs`),
  `Catalog::set_conversation` and the handle in `known_session_ids` (`daemon/src/catalog.rs`), the
  latest `init`'s `session_id` recorded when it differs (`daemon/src/state.rs`)

---

## Phase 8: Daemon — switching kind (M6; needs #941 M2, I-2)

- [ ] T043 [P] [US4] Tests in `core/tests/protocol_roundtrip.rs`: `SessionSwitchKind { req,
  session, to, command, confirmed }` and `ErrorKind::NeedsConfirmation` round-trip; `schema_hash`
  re-pinned after `PROTOCOL_VERSION` +1
- [ ] T044 Implement T043 in `core/src/protocol/messages.rs` and `core/src/protocol/version.rs`;
  update the `Session::start_new_of_kind` doc comment in `core/src/session.rs` (only
  `Catalog::set_kind` changes a kind; replaces 938 FR-002)
- [ ] T045 [P] [US4] [US5] Tests in `daemon/tests/structured_switch.rs` (new), with the PTY stand-in
  for the terminal side: structured → terminal → structured keeps the `SessionId`, each launch's
  argv is the other kind with `--resume <resume_handle()>` and `--model <choice>`, the structured
  one with the permission mode and the terminal one with no `--permission-mode` (FR-020, FR-024);
  the kind is persisted, in the next catalog snapshot to every client, and kept across a daemon
  restart (US5 scenario 6); a switch with `command: Some("/mcp")` enters `/mcp` into the PTY after
  readiness (FR-022), none without it; a never-started session starts `Fresh` (FR-020); a resume
  that fails is reported as today and the kind stays (US5 scenario 7); a Copilot session →
  `Refused`; `command` with `to: Structured` or a non-terminal-only command → `InvalidInput`;
  a same-kind switch → `OperationOk`, no restart
- [ ] T046 [US4] [US5] Tests in `daemon/tests/structured_switch.rs`: busy, a pending #942
  request, or a terminal session whose activity is `Working`, unconfirmed → `NeedsConfirmation`
  and nothing changes; `confirmed: true` → switches (FR-021); two concurrent switches, 50
  iterations → one stop, one start (FR-025); a `SessionUserMessage` during a switch is queued on
  hold and delivered once after **Send queued** (FR-025); the queue is held and undelivered while
  terminal, and still held after the switch back (FR-024); the history gets `KindSwitched`
  notes bracketing the period and is kept and saved (**Save terminal history**) while terminal
  (FR-023); events, queue and model counts unchanged across switch, restart and daemon restart
  (SC-005); switching session A never touches B; after a switch to terminal, a successor transcript
  the PTY stand-in writes as a continuation (a `/clear` typed in the terminal) is not adopted as a
  new session (BUG-904's check; spec clarification 4)
- [ ] T047 [P] [US1] Test in `daemon/tests/structured_commands.rs`: a command sent while busy is
  queued by #941's input and delivered unchanged when its turn comes (US1 scenario 8, FR-001)
- [ ] T048 Implement T045–T047: `Catalog::set_kind` (`daemon/src/catalog.rs`); `switch_kind` with
  the `switching` slot, confirmation, queue hold, notes, stop as `SessionStop` does and
  `ops::start_with_prompt(…, LaunchMode::Resume, command)` (`daemon/src/state.rs`,
  `daemon/src/ops.rs`); dispatch `SessionSwitchKind` (`daemon/src/server.rs`)
- [ ] T049 [P] Update the 938 FR-002 tests that assert a kind never changes so they assert only
  `Catalog::set_kind` changes it (plan *Superseded assertions*); describe `SessionSwitchKind` and
  the held queue in `docs/daemon.md`

---

## Phase 9: Client — render-free palette, picker, switch state (M7; needs #941 M4, I-3)

- [ ] T050 [P] [US2] Tests in `client/tests/features_command_palette.rs` (new): typing `/` in an
  empty draft opens `PaletteState` with the session's `CommandList` grouped; `sp` narrows and
  selects the first; Up/Down move; Tab, Enter and a click fill `/<name> ` and close without
  sending; Escape closes, leaves the draft and does not interrupt the turn; deleting the `/` or
  moving the caret out closes; `palette_key` returns `None` while closed so `composer_key` runs;
  no match → the "no command matches" state and Enter sends as typed; a terminal-only row carries
  its words; a fallback list carries its note (US2 scenarios 1–8, FR-007–FR-010)
- [ ] T051 [US3] Tests in `client/tests/features_command_palette.rs`: `ModelPickerState` lists
  Default plus `CommandsReported.models`, else `MODEL_SHORTCUTS`, marks the current
  `session_model`, accepts a typed name; picking sends `SetSessionModel`; sending `/model` opens
  the picker and sends nothing, `/model haiku` sends `SetSessionModel(haiku)` (FR-013, FR-014,
  FR-018); the header text (a render-free function) says the session model as last reported and
  "your choice" or Default; after a pick on a busy session it keeps the reported model, with the
  new choice, until the `ModelChanged` note arrives (US3 scenario 4)
- [ ] T052 [P] [US4] [US5] Tests in `client/tests/features_session.rs`: sending or picking a
  terminal-only command opens `SwitchOffer { to: Terminal, command, reason }` and sends nothing;
  dismissing keeps the draft; confirming sends `SessionSwitchKind`; `NeedsConfirmation` opens the
  confirmation step and confirming resends with `confirmed: true`; an `auth_failed` entry offers
  `/login`; a kind change in a catalog snapshot drops the old terminal emulator and attaches to the
  new primary, and the window's draft survives (FR-010, FR-021, FR-022a, FR-024)
- [ ] T053 [US4] Tests in `client/tests/features_session.rs`: a render-free
  `switch_entries(state, session)` gives **Open in terminal** for a structured Claude session and
  **Open in structured view** for a terminal Claude session (switched or created as terminal), each
  sending `SessionSwitchKind` with no command (US4 scenarios 3–4), and nothing for Copilot, Pi,
  Codex or OpenCode sessions (FR-019, US4 scenario 7)
- [ ] T054 Implement T050–T053 in `client/src/features/command_palette.rs` (new),
  `client/src/features/session.rs`, `client/src/shell/daemon_sync.rs` and `client/src/app.rs`

---

## Phase 10: Window — palette, model picker, markers, guide (M8; needs #941 M6, I-3)

- [ ] T055 [P] [US2] Tests: `client/tests/gates/command_palette_fits.rs` (new, registered in
  `client/tests/layout_snapshot.rs`) — rows, group headers and the "opens in terminal" tag fit at
  the composer's narrowest width in both themes; the showcase completeness and builder-API tests
  list `CommandPalette`
- [ ] T056 [US2] Create the shared `CommandPalette` in `client/src/ui/material/command_palette.rs`
  from `material::picker`'s row and panel, with a builder API, register it in
  `client/src/ui/material/mod.rs`, add its showcase entry under `client/src/showcase/` and its theme
  snapshot (T055 passes)
- [ ] T057 [US2] [US3] Draw the palette in #941's composer completion slot and the model (with
  "your choice"/Default, the text from T051's tested function) and the `Typeahead` model picker in
  the composer's status row (`client/src/ui/composer.rs`) (GUI exception for the draw glue only)
- [ ] T058 [US1] [US3] Replace M2's plain markers in `client/src/ui/transcript/entries.rs` with
  #939's `DisclosureCard`: the command chip with its description behind it, `CommandOutput`,
  `Compacted` with the summary collapsed, `Cleared` and earlier eras drawn as "before the clear",
  `ModelChanged`/refused; every caption and label comes from a render-free function with a unit
  test, as in T024 (GUI exception for the draw glue only)
- [ ] T059 [US2] [US3] User guide `docs/user-guide/worktrees-and-sessions.md` *Structured
  sessions*: the palette (keys, groups, "opens in terminal"), the model picker, `/model`, a model
  changed inside the terminal not being seen (FR-027)
- [ ] T060 Quickstart Part B 1, 2, 4 and 5 in both themes, evidence in
  `specs/943-structured-commands-fallback/visual-pass/` (`visual-pass` skill)

---

## Phase 11: Window — switching, offers, terminal-only table (M9; #942 M7 for `PromptCard`, M8 for SC-007, I-6)

- [ ] T061 [P] [US4] Test `core/tests/terminal_only_doc.rs` (new): the table between the marker
  comments in `docs/user-guide/worktrees-and-sessions.md` has exactly the names, reasons and
  version of `TERMINAL_ONLY` and `CHECKED_AGAINST` (FR-012)
- [ ] T062 [US4] [US5] User guide: **Open in terminal** and **Open in structured view**, what a
  switch keeps (conversation, queue on hold, drafts, model, permission mode), the terminal period
  marker, the terminal-only table with why and the checked version (T061 passes; FR-012, FR-027)
- [ ] T063 [US4] Draw the switch offer and `client/src/ui/confirm_switch_kind.rs` (new, on `Dialog`
  like `confirm_delete.rs`); the offer uses `PromptCard` if #942 M7 is on `main` when this task is
  implemented, else `Dialog` (one implementation, chosen then; no dead branch); **Open in
  terminal** in the transcript header and `session_menu_items` (`client/src/ui/mod.rs`) taken from
  T053's `switch_entries`; the auth-failure offer beside the error (GUI exception for the draw glue
  only)
- [ ] T064 [US4] [US5] Unit-test then draw: the held-queue line text ("<n> queued messages wait for
  the structured view") and the `TerminalPeriod` marker text (the conversation continued in the
  terminal; those turns are not shown here and are in the terminal's scrollback) as render-free
  functions with unit tests, replacing T024's plain `TerminalPeriod` caption; draw **Open in structured view** (from `switch_entries`) and the
  held-queue line in `client/src/ui/terminal.rs` and the marker in
  `client/src/ui/transcript/entries.rs` (GUI exception for the draw glue only)
- [ ] T065 Quickstart Part B 3 (SC-007, needs #942 M8) and 6–9 (8: a sandboxed session switches
  inside its sandbox), evidence in `specs/943-structured-commands-fallback/visual-pass/`

---

## Phase 12: Polish (close unit)

- [ ] T066 Run quickstart Part A and record the results in
  `specs/943-structured-commands-fallback/quickstart.md`

---

## Dependencies & Execution Order

- Phase 1 → Phase 2 → Phase 3 (M1); then Phases 4 → … → 11, each a milestone on `main` before the
  next starts. Phase 4 also waits for #941 M1 (I-1), Phase 8 for #941 M2 (I-2), Phase 9 for #941 M4,
  Phase 10 for #941 M6 (I-3), Phase 11 for #942 M7 (I-6, else `Dialog`).
- Phases 5 (M3) and 6 (M4) build on Phase 3 only; Phase 4 (M2) is held whole until #941 M1 is on
  `main` (its chip tasks T022–T023 need `UserSent`), and M3–M6 do not wait for it (nothing in them
  reads a transcript entry); Phase 7 (M5) on Phase 6.
- Within a phase, each test task precedes the implementation it names (Constitution I).
- Stories: US1 and US2 (P1) carry the command list, classification and palette; US3 the model;
  US4 and US5 the switch, which reuses US3's model choice and M5's handle.

## Parallel Opportunities

- Phase 2: T004 and T006 (different files). Phase 3: T009, T010, T011, T013, T015, T017.
- Phase 4: T019 and T020 share a file and are written in sequence; T024 after T021.
- Phase 5: T026 beside T028–T029 (different files). Phase 6: T032 and T033. Phase 8: T043 beside
  T045–T047.
- Phase 9: T050–T051 share a file, T052–T053 another.

## Implementation Strategy

MVP is M1: the core knows a session's commands, which need the terminal, how a draft is classified,
how the palette orders names, what model a session runs, and the control lines, all against
recorded fixtures. M2–M7 add one layer each, observable through their own tests; nothing is visible
to users before M8 (palette, model picker, markers, with their guide), and M9 completes the
terminal fallback. M5 is provisional (ledger decision 2): reverting it alone returns to 582
FR-017's resume rule.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot). M1, M3, M4 and M5
build on `main` as it is. **M2 needs #941 M1; M6 #941 M2; M7 #941 M4; M8 #941 M6; M9 #942 M7 (and
#942 M8 for SC-007's manual check)** — the orchestrator holds each until its dependency is on
`main`. T066 (Polish, no code) is left to the close unit (milestones rule 4).

### M1 — Core: commands, terminal-only, palette, model and control against recorded fixtures 🎯 MVP

- **Tasks**: T001–T018
- **Deliverable**: `micold_core` parses every recorded command, model and auth fixture, builds a
  session's command list with kinds and terminal-only marks (or the fallback), classifies a draft
  as send, `/model` or terminal-only, orders palette matches by FR-008's tiers within the SC-003
  budget, validates model choices, and encodes and reads `initialize`/`set_model`; the capture
  script records control requests and the fixtures are in the tree
- **Satisfies**: FR-007–FR-011 (core), FR-014 (shortcuts), FR-018 (classify), FR-026 (fixtures);
  SC-002, SC-003; US2 scenarios 2, 5–8 at the model
- **Verify**: `cargo test -p micold-core --test structured_parse --test structured_commands --test
  command_palette --test structured_transcript --test protocol_roundtrip --test schema_hash`;
  `cargo test -p micold-core --lib -- structured::commands structured::model structured::control
  structured::saved`; `bash scripts/tests/capture-stream-json.test.sh`
- **Depends on**: —
- **Tier**: full

### M2 — Core transcript: command chips, command output, compaction, clear, model and terminal markers

- **Tasks**: T019–T024
- **Deliverable**: the transcript shows a command turn as `/name args` with its description, a
  built-in's output as markdown, compaction and clear markers (earlier turns kept, eras), model
  change/refusal markers, one terminal-period marker per switch pair, and flags an auth failure
- **Satisfies**: FR-003–FR-006, FR-015/FR-016 (marker), FR-022a (flag), FR-023 (marker); SC-001
  (transcript side); US1 scenarios 3–6
- **Verify**: `cargo test -p micold-core --test structured_transcript`;
  `cargo test -p micold-client --lib -- transcript`
- **Depends on**: M1; **#941 M1 on `main`** (`UserSent`, I-1)
- **Tier**: full

### M3 — Daemon: command list at spawn; `/model` and terminal-only never reach the CLI

- **Tasks**: T025–T031
- **Deliverable**: every structured spawn asks the CLI for its commands and models and keeps the
  answer in the event history (replayed to late clients); commands reach the CLI unchanged;
  `/model` and terminal-only commands are refused with a reason, stdin untouched; `send_control`
  serves #942's mode change unchanged
- **Satisfies**: FR-001, FR-002, FR-009, FR-010 (daemon), FR-018 (daemon); SC-001 (daemon), SC-006;
  US1 scenarios 1–2, 7
- **Verify**: `cargo test -p micold-daemon --test structured_commands --test structured_session`
  `--test structured_permission_mode`
- **Depends on**: M1
- **Tier**: full

### M4 — Daemon: model choice persisted, launched with `--model`, changed live

- **Tasks**: T032–T038
- **Deliverable**: `SetSessionModel` changes a running session's model without restarting it
  (deferred past a busy turn), refusals leave it unchanged with the CLI's reason, the choice is in
  the summary and the catalog and every launch carries `--model`; every `--resume` goes through
  `Session::resume_handle()`
- **Satisfies**: FR-013 (summary), FR-015–FR-017; SC-004; US3 scenarios 3–4, 7–8 at the daemon
- **Verify**: `cargo test -p micold-daemon --test structured_model --test structured_session`;
  `cargo test -p micold-core --test session_store --test protocol_roundtrip --test schema_hash`
- **Depends on**: M3
- **Tier**: full

### M5 — Daemon: resume the conversation a `/clear` started (provisional)

- **Tasks**: T039–T042
- **Deliverable**: after `/clear` in a structured session, restart and daemon restore resume the
  cleared conversation, under the same session, with no second session adopted
- **Satisfies**: FR-005a; US1 scenario 5 (resume part); SC-005 (handle)
- **Verify**: `cargo test -p micold-daemon --test structured_resume`;
  `cargo test -p micold-core --test session_store`
- **Depends on**: M4. **Provisional — ledger decision 2 (pending jaro)**: under decision B this
  milestone is dropped (or reverted alone); under C it is replaced
- **Tier**: full

### M6 — Daemon: switch a Claude session between structured and terminal

- **Tasks**: T043–T049
- **Deliverable**: `SessionSwitchKind` restarts a Claude session in the other kind resuming its
  conversation, with its identity, model, mode (structured only), held queue and history kept,
  confirmation when work is in progress, one switch under concurrency, a terminal-only command
  typed into the terminal; other CLIs refused
- **Satisfies**: FR-001 (queued while busy), FR-019–FR-025 (daemon); SC-005, SC-008; US4 scenarios
  2–7, US5 scenarios 1, 3, 5–7 at the daemon
- **Verify**: `cargo test -p micold-daemon --test structured_switch --test structured_commands`;
  `cargo test -p micold-core --test protocol_roundtrip --test schema_hash`; `scripts/gate.sh`
  (SC-008)
- **Depends on**: M4, M5 (or without it under decision B); **#941 M2 on `main`** (queue, hold,
  input driver; I-2)
- **Tier**: full

### M7 — Client: render-free palette, model picker and switch offer

- **Tasks**: T050–T054
- **Deliverable**: the window's state opens, filters and fills the palette, opens the model picker
  and sends model changes, offers the terminal for terminal-only commands and auth failures, asks
  for confirmation, and follows a kind change keeping drafts (no draw code yet)
- **Satisfies**: FR-007–FR-010, FR-013, FR-014, FR-018, FR-021, FR-022a, FR-024 at the state level;
  US2 scenarios 1–8, US3 scenarios 1–2, 5, US4 scenarios 1, 3, 5
- **Verify**: `cargo test -p micold-client --test features_command_palette --test features_session`
- **Depends on**: M2, M4, M6; **#941 M4 on `main`** (composer state, `composer_key`; I-3)
- **Tier**: full

### M8 — Window: command palette, model picker, transcript markers, user guide

- **Tasks**: T055–T060
- **Deliverable**: a user types `/` in a structured session and picks a command from the shared
  `CommandPalette`, sees and changes the model, and sees command chips and markers; the user guide
  describes the palette and the model picker; quickstart Part B 1, 2, 4 and 5 pass
- **Satisfies**: US1 scenarios 3–6, US2, US3 end to end; FR-027 (palette, picker); Constitution VIII
- **Verify**: `cargo test -p micold-client --test layout_snapshot --test showcase_completeness --test
  material_builder_api`; quickstart Part B 1, 2, 4 and 5 in `visual-pass/`;
  `scripts/check-user-guide-updated.sh`
- **Depends on**: M7; **#941 M6 on `main`** (composer completion slot; I-3)
- **Tier**: full

### M9 — Window: Open in terminal / structured view, offers, terminal-only table

- **Tasks**: T061–T065
- **Deliverable**: a user switches a Claude session to the terminal (from the view, the menu, a
  terminal-only command or an auth failure) and back, with confirmation when work is in progress
  and the held-queue line; the guide lists the terminal-only commands, checked by a test;
  quickstart Part B 3 and 6–9 pass
- **Satisfies**: US4, US5 end to end; SC-007; FR-012, FR-019 (window), FR-022a, FR-027 (switching)
- **Verify**: `cargo test -p micold-core --test terminal_only_doc`; quickstart Part B 3 and 6–9 in
  `visual-pass/`; `scripts/check-user-guide-updated.sh`
- **Depends on**: M8; **#942 M7 on `main`** for `PromptCard` (else `Dialog`), #942 M8 for SC-007's
  manual check (I-6)
- **Tier**: full
