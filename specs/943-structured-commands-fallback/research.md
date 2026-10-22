# Research: Structured Session Commands (#943)

Each decision: what was chosen, why, and what was rejected. **P-n** marks a probe run on
2026-10-22 against the real CLI (Claude Code `2.1.307`, the cloud dev container, `claude -p
--input-format stream-json --output-format stream-json --verbose --session-id <new uuid>` in a
scratch git repo). The probe scripts are not part of the change; the recorded fixtures of R13 are
their committed form, re-recorded in M1 with the capture script.

- **P-1** `initialize` control request on stdin before any user message: answered at once, no
  `init` line, no model call.
- **P-2** `/context`, `/clear`, `/model haiku`, `/compact` (empty conversation), `/mcp`, `/config`,
  `/login`, `/doctor`, `/nosuchcmd`, each sent as one user message's text.
- **P-3** `--replay-user-messages --model haiku`: one plain turn, a project custom command
  `/hello world` (`.claude/commands/hello.md`), a project skill `/greet Ada`
  (`.claude/skills/greet/SKILL.md`), then `/compact keep greetings`.
- **P-4** `set_model` control requests: `haiku`, `default`, no `model`, `no-such-model-xyz`.
- **P-5** an empty `HOME` and an invalid `ANTHROPIC_API_KEY`, one user message.

## R1. Where the session command list comes from (FR-007, FR-009, FR-011)

- **Decision**: two sources, merged in core `structured::commands::CommandList`:
  1. Every `init` line (P-2, P-3: the CLI writes one before **every** turn, not only the first)
     carries `slash_commands` (names), `skills` (names; a subset of `slash_commands`) and
     `terminal_slash_commands` (names the CLI marks terminal-only; P-2: `doctor`, `color`,
     `focus`, `reload-plugins`). `Init` gains the three lists.
  2. The daemon writes one `initialize` control request (`{"subtype":"initialize"}`) right after
     each structured spawn. The answer (P-1) holds `commands: [{name, description, argumentHint,
     aliases?, builtin?}]` and `models: [{value, resolvedModel, displayName, description}]` (R7),
     before the first user message. The daemon turns it into a daemon note
     `CommandsReported { commands, models }` appended to the event history (data-model), so the
     list is in the history whose events the session holds (FR-009) and reaches every window the
     way every event does: no new broadcast.
- **Kind** (FR-007): a name in the latest `init`'s `skills` → **Skill**; else `builtin: true` in
  the `initialize` answer → **Built-in**; else → **Custom command**. With no `initialize` answer
  (refused, or no answer within the 10 s control bound of #942), the `init` lists alone are used:
  names without descriptions, kind Skill or Custom (FR-007: "description when it reports one").
- **Rejected — the `init` lists only**: no descriptions, no argument hints, no built-in flag, and
  nothing at all until the first turn (the `init` line comes only with a user message, P-1); FR-002
  scenario 8 would then always show the fallback for a fresh session.
- **Rejected — reading skill and command files from disk** (`.claude/skills`, `~/.claude/commands`,
  plugin directories): duplicates the CLI's resolution rules (plugins, claude.ai-synced skills,
  aliases), and is wrong in a sandbox, where the CLI's view of the files is the container's.

## R2. Terminal-only list (FR-010, FR-011, FR-012)

- **Decision**: core `structured::commands::TERMINAL_ONLY: &[TerminalOnly { name, why }]` with
  `CHECKED_AGAINST: &str` (the Claude Code version). At least `config`, `mcp`, `agents`, `login`,
  `logout` (FR-011). A command is terminal-only when its name **or one of its aliases**
  (`settings` is an alias of `config`, P-1) is in `TERMINAL_ONLY` or in the latest `init`'s
  `terminal_slash_commands`. The palette lists every `TERMINAL_ONLY` entry even when the CLI does
  not report it (P-2: `login` is not in a structured session's `slash_commands` at all).
- **Observed (P-2)**: in structured mode `/login` answers "isn't available in this environment",
  `/config` prints a `key=value` usage text, `/mcp` prints the server list: each a text answer
  with no model turn, none doing what the user wanted. That is the spec's "does nothing useful";
  the documented list stays as FR-011 names it.
- **Doc sync (FR-012)**: the user guide holds the list as a table between two marker comments; a
  core test reads the guide with `include_str!` and asserts the table's names, reasons and version
  equal `TERMINAL_ONLY` and `CHECKED_AGAINST`. One source of truth checked in CI (Principle VII).
- **Rejected — only the CLI's marking**: it marks none of the five FR-011 names (P-2).

## R3. Sending a command (FR-001, FR-002, US1 scenario 8)

- **Decision**: a command is sent as the user message's text, unchanged, through #941's send path
  (`SessionUserMessage` → `SessionInput::submit` → `encode_user_message`). No new message. Core
  `commands::classify_draft(text, &CommandList) -> DraftCommand` decides `Send`,
  `Model(Option<String>)` (`/model`, `/model <name>`) or `TerminalOnly(name)` (FR-010, FR-018). The
  window acts on `Model`/`TerminalOnly` and never sends them. The daemon applies the same
  classification in `SessionUserMessage` before submit and answers `OperationError` naming the
  reason for `Model`/`TerminalOnly`, so neither reaches the CLI or the queue from any client
  (SC-006, Edge Cases *A command in the queue*).
- **Observed (P-3)**: the CLI expands custom commands and skills itself; `/hello world` was
  answered "hello world". A command the CLI does not know is handed to the model as text (P-2:
  `/nosuchcmd` got a model reply) — US1 scenario 7 needs nothing from the app.
- **Rejected — app-side expansion** of skills or commands: the CLI already does it, identically in
  both kinds (FR-002).

## R4. Showing a command's turn (FR-003, FR-006)

- **Observed (P-3)**: the CLI never writes the expanded instructions to stdout. With
  `--replay-user-messages` it echoes only `<command-message>hello</command-message>
  <command-name>/hello</command-name><command-args>world</command-args>`; without it, nothing (#941
  records the sent text as its `UserSent` note).
- **Decision**: the transcript shows a user turn whose text `classify_draft` reads as a command as
  a command chip (`/name` and its arguments as typed); the disclosure behind it holds the command's
  description and argument hint from the `CommandList` held at that point. The expanded
  instructions are never shown raw, because they are never in the stream. The spec's FR-003 and
  US1 scenario 3 are amended accordingly ("expanded instructions, when the CLI reports them";
  ledger *Decisions*).
- **Built-in output (P-2)**: a command answered without a model turn produces an `assistant` line
  whose `message.model` is `"<synthetic>"` with one text block (markdown, e.g. `/context`'s table),
  then a `result` with `num_turns: 0`. `Assistant` gains `model: Option<String>`; the transcript
  shows a synthetic assistant text that follows a command user turn as `CommandOutput { text }`,
  drawn with #940's markdown renderer. No `<local-command-stdout>` markup reaches stdout without
  `--replay-user-messages` (P-3 shows it only as a replayed user line), so the app does not pass
  that flag.
- **Rejected — `--replay-user-messages`**: doubles every user message (#941 already records
  `UserSent`) and carries CLI markup FR-006 forbids.

## R5. `/compact` and `/clear` in the transcript (FR-004, FR-005)

- **Observed (P-3)**: `/compact` → `system/status` `status: "compacting"`, then `system/status`
  `compact_result: "success"` (or `"failed"` with `compact_error`, P-2), then a fresh `init`, then
  `system/compact_boundary` with `compact_metadata { trigger: "manual"|"auto", pre_tokens,
  post_tokens, … }`, then a `user` line with `isSynthetic: true` holding the summary text, then the
  turn's `result`. A failure is also answered as a synthetic assistant text ("Not enough messages
  to compact.").
- **Observed (P-2)**: `/clear` → a `conversation_reset` line `{ new_conversation_id, trigger:
  "clear" }`, a `result`, and at the next turn an `init` whose `session_id` is new
  (`9a714e62…` after `903577ea…`); that id names the CLI's new transcript file and is what
  `--resume` takes. (`new_conversation_id` is a different uuid and names no file.)
- **Decision**: new events `CompactBoundary { trigger, pre_tokens, post_tokens }` and
  `ConversationReset { trigger }`; `User` gains `is_synthetic`. The transcript builder turns a
  boundary plus the synthetic user text that follows it into one `Compacted { trigger, summary }`
  entry (the summary collapsed), and a reset into `Cleared`. Entries carry `era: u32`, bumped at
  each `Cleared`; the window draws entries of an earlier era as "before the clear". Turns before
  either marker stay (FR-004, FR-005). The session is busy through a `/compact` because it is a
  turn (#941's turn state) — no extra state.
- **Rejected — dropping entries before a compaction**: the spec keeps them visible.

## R6. Conversation handle after `/clear` (FR-005a) — PROVISIONAL (ledger decision 2)

- **Decision**: the daemon records, per structured session, the `session_id` of the latest `init`
  as the session's **conversation handle** whenever it differs from the recorded one, persists it
  in the catalog (`StoredSession.conversation`, `None` = the session's own id) and passes it to
  every later `--resume` (restart, daemon restore, switch of kind). `Catalog::known_session_ids`
  also returns each session's handle, so the discovery pass in `state.rs` counts it as known, so the post-clear
  transcript is never adopted as a second session even when BUG-904's `starts_as_continuation`
  check would miss it.
- **Isolation**: `Session::resume_handle()` and `LaunchSpec.resume_handle` land in M4 with the
  model (returning the session id), and every `--resume` site goes through them there. M5 adds
  only the `conversation` field, its catalog setter, the `init` tracking, `resume_handle()`
  returning the field, and one discovery line. Reverting
  to decision B (582 FR-017 everywhere) is reverting M5; C (an app-handled `/clear`) would replace
  M5 and add a `TerminalOnly`-like arm to `classify_draft`. Nothing in M1–M4 or M6–M9 reads the
  handle except through `Session::resume_handle()`, which returns the session id when the field is
  `None`.
- **Rejected — following `new_conversation_id`**: P-2 shows it is not the id `--resume` takes.
- **Rejected — reading the CLI's store to find the successor**: the `init` line already names it.

## R7. Model switching (FR-013–FR-018)

- **Observed (P-4)**: `{"type":"control_request","request_id":…,"request":{"subtype":"set_model",
  "model":"haiku"}}` → `success` with `response: {"model":"claude-haiku-5-5"}`; `"default"` or no
  `model` → `success` with the default's resolved model; an unknown name → `error` with `error:
  "Model 'no-such-model-xyz' not found"`, `error_code: "not_found"` and the unchanged `model`. The
  process keeps running; the next `init` reports the new model (P-2's `/model haiku` too).
- **Decision**:
  - Core `structured::model::ModelChoice` (`Default` | `Named(String)`, validated non-blank, no
    whitespace, ≤ 128 bytes), `encode_set_model(id, &ModelChoice)` (`Default` → `"default"`),
    `parse_set_model(&ControlOutcome) -> Result<String, ModelRefusal>`; `launch_args(&ModelChoice)`
    → `["--model", name]` or nothing. Persisted as `StoredSession.model` (serde default `Default`,
    skipped when `Default`), passed at every launch in **both** kinds (FR-017, US5 scenario 5).
  - Daemon `SetSessionModel { req, session, model }`: persists and broadcasts the choice at once
    (`SessionSummary.model`), then, when the CLI is running and **idle**, writes `set_model` through
    #942's `ControlWaiters` and waits up to the 10 s control bound. **Busy**: the request is
    deferred to the turn's end and written before #941's input driver delivers the next queued
    message, so the turn in progress keeps its model (FR-015, US3 scenario 4). Success appends a
    daemon note `ModelChanged { model }` (the resolved name) to the event history — the transcript
    marker and the session model in one (FR-013, FR-015). Refusal restores the previous choice,
    persists and broadcasts it, and answers `OperationError` with the CLI's text (FR-016); a
    deferred change (busy) was already answered `OperationOk`, so its refusal is a daemon note
    `ModelRefused { model, message }` every window shows. Not running: persisted only, used at
    next launch. The answer timing follows #942's `SetPermissionMode` (contracts/wire.md).
  - Concurrent picks (Edge Cases): the per-session model slot is a single `Option<ModelChoice>`
    replaced under the state lock; the writer task always sends the slot's latest value, so the last
    pick to reach the daemon wins.
  - Picker contents (FR-014): `CommandsReported.models` (`displayName`, `description`); none
    reported → `MODEL_SHORTCUTS` (`sonnet`, `opus`, `haiku`) plus Default; a typed name is
    accepted and checked by the CLI.
- **Rejected — sending `/model <name>` as text**: it works (P-2) but gives no structured refusal,
  cannot be deferred past a busy turn, and would change the model with no persisted choice.
- **Rejected — restarting with `--model`**: FR-015 forbids ending the process.

## R8. Switching kind (FR-019–FR-025)

- **Decision**: one daemon operation `switch_kind(session, to, command, confirmed)`:
  1. Refuse a non-Claude provider (`SessionKind::Structured.check`, FR-019) and a same-kind switch
     (no-op `Ok`).
  2. Unless `confirmed`, refuse with `OperationError` `NeedsConfirmation` when a turn is in
     progress, a #942 request is pending, or a terminal session's activity is `Working` (FR-021);
     the window asks and resends with `confirmed: true`.
  3. Under the state lock set `switching` (a second request while set answers `Ok` with no action
     for the same target, `OperationError` "switching" otherwise, FR-025), hold #941's queue,
     append the note `KindSwitched { to }` to the event history, set the catalog kind
     (`Catalog::set_kind`, persisted, broadcast in the catalog snapshot).
  4. Stop the process as `SessionStop` does (both kinds already end it on every platform, feature
     938 FR-036), then `ops::start_with_prompt(…, LaunchMode::Resume, prompt)` with the
     conversation handle (R6), the model choice (R7), and for structured the permission mode
     (#942); a terminal launch gets no permission-mode override (#942 FR-018). `prompt` is the
     terminal-only command for a switch to terminal (FR-022) — the first-prompt path already waits
     for the terminal CLI's readiness. A session with no recorded conversation starts `Fresh`
     (FR-020), by the existing `has_recorded_conversation` check.
  5. Clear `switching`. A start failure is reported as a failed resume is today; the kind stays
     (US5 scenario 7).
- **Event history across the terminal period**: the structured history is kept and saved (under
  **Save terminal history**) while the session is terminal, as is the terminal scrollback. The two
  `KindSwitched` notes bracket the period; the transcript builder renders the pair as one
  `TerminalPeriod` entry (FR-023). The resumed run's `init` follows the second note, so there is no
  gap or duplicate: nothing from the terminal period is in the event history to duplicate.
- **Queue and drafts (FR-024)**: #941's queue is held at step 3 and stays undelivered while the
  kind is terminal (#941's driver delivers only to a structured process); the terminal view shows
  its length. Drafts are window state keyed by session and are not dropped on a kind change.
- **Messages during a switch (FR-025)**: `SessionUserMessage` while `switching` goes to #941's
  queue (held), never to a process.
- **Rejected — a new session for the other kind**: breaks the session's identity, its sidebar row,
  its sandbox and its terminals (FR-020).
- **Rejected — keeping both processes**: two CLIs writing one conversation.

## R9. Authentication failure (FR-022a)

- **Observed (P-5)**: the CLI writes a synthetic assistant line with top-level `"error":
  "authentication_failed"` and text "Invalid API key · …", then a `result` with `is_error: true`.
- **Decision**: `Assistant` gains `error: Option<String>`; the transcript marks that entry
  `auth_failed`, and the window offers **Open in terminal** with `/login` beside it.

## R10. Command palette (FR-007, FR-008, FR-008a, SC-002, SC-003)

- **Decision**: core `structured::commands::palette::filter(&CommandList, query) -> Vec<usize>`:
  case-insensitive *contains*; tiers exactly as FR-008 (equal, prefix, word-prefix after `-` `_`
  `:`, rest), alphabetical within a tier. Navigation reuses `micold_core::typeahead::{
  move_highlight, clamp_highlight}`. Opening and closing (FR-007, US2 scenarios 4, 7): open while
  the draft is `/` + a name with no space and the caret is inside it.
- **Rejected — `typeahead::rank`**: its single-edit and subsequence tiers list names FR-008 says
  must not match, and orders by offset, not FR-008's tiers.
- **Performance (SC-003)**: 200 items × one lowercase compare per key is far below 100 ms; a core
  test filters 200 recorded-shape items 1,000 times and asserts the mean, as #941's timing test.

## R11. Window pieces and shared components (Principle VIII)

- **Decision**: a new shared `CommandPalette` in `ui/material/` built from `material::picker`'s
  `Row` and panel (as `Typeahead` and `Select` are), with group headers and a trailing text tag
  ("opens in terminal") — words, not colour (US2 scenario 6). The model picker is the existing
  `Typeahead` (Default, models, typed name). The switch offer reuses #942's `PromptCard` when it is
  on `main`, else `Dialog` (I-3). The switch confirmation is `ui/confirm_switch_kind.rs` on
  `Dialog`, like `confirm_delete.rs`. Transcript markers (compacted, cleared, model, terminal
  period, command output) reuse #939's `DisclosureCard` and #940's markdown. Session menu entries
  go into `ui/mod.rs::session_menu_items`; the transcript header gets **Open in terminal**, the
  terminal view an **Open in structured view** button and the held-queue line.
- **Client kind change**: the client already chooses the pane body from `SessionSummary.kind`
  (`ui/terminal.rs::is_structured_primary`); a kind change in a catalog snapshot must also drop the
  window's emulator for the old terminal primary and attach to the new one. Render-free part in
  `features/session.rs` (reducer test), draw glue unchanged.

## R12. Testing against the stand-in CLI (FR-026)

- **Decision**: extend #938's stand-in (`crates/micold-daemon/tests/support/structured.rs`) with:
  answer `initialize` from a recorded answer; answer `set_model` (accept a scripted list, refuse
  others with the recorded error); replay a fixture per command text (`/compact` →
  `command-compact.jsonl`, …); write an `init` with a new `session_id` after `/clear`. Daemon suites
  `structured_commands.rs`, `structured_model.rs`, `structured_switch.rs`. Switching tests use the
  existing PTY stand-in for the terminal side (the one the terminal-session tests use) and assert
  its argv (`--resume <handle>`, `--model`).

## R13. Recorded fixtures (FR-026)

- **Decision**: M1 records with `scripts/capture-stream-json.sh` (extended to write control
  requests from a `--control <json>` option, as #942's capture script does) into
  `crates/micold-core/tests/fixtures/stream-json/`: `command-initialize.jsonl` (P-1),
  `command-skill.jsonl` (P-3's `/greet`, with a project skill and custom command in the scratch
  repo), `command-custom.jsonl`, `command-compact.jsonl`, `command-clear.jsonl`,
  `command-builtin-output.jsonl` (`/context`), `command-unknown.jsonl`, `model-change.jsonl` (P-4,
  one turn after a successful change), `model-refused.jsonl`, `auth-failure.jsonl` (P-5),
  `resume-after-clear.jsonl` (`--resume <post-clear id>`). README rows added.
