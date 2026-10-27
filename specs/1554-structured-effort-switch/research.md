# Research: Structured Session Reasoning Effort (#1554)

Every decision mirrors #943's model switching ([its research R7](../943-structured-commands-fallback/research.md#r7-model-switching-fr-013fr-018))
unless a finding below says why not. Probes P-1 and P-2 were run on 2026-10-27 against Claude Code
2.1.311 with `scripts/capture-stream-json.sh` (structured argv, a fresh scratch repo, `CLAUDE_EFFORT`
unset); the raw output stayed in the session scratchpad, and M1 records the fixtures of R9 from the
same requests. The decisive lines are quoted below.

## P-1. Changing effort in a running structured CLI

Stdin, in order, before one user message: `initialize`; `get_settings`;
`apply_flag_settings {"effortLevel":"low"}`; `get_settings`;
`apply_flag_settings {"effortLevel":"extreme"}`; `apply_flag_settings {"effortLevel":null}`;
`get_settings`.

```text
micold-2 get_settings            → success, response.applied.effort = "medium", effective = {}
micold-3 apply_flag_settings low → {"subtype":"success","request_id":"micold-3"}        (no response body)
micold-4 get_settings            → success, applied.effort = "low",
                                   effective = {"effortLevel":"low"},
                                   sources = [{"source":"flagSettings","settings":{"effortLevel":"low"}}]
micold-5 apply_flag_settings "extreme" → success                                         (not validated)
micold-6 apply_flag_settings null      → success
micold-7 get_settings            → success, applied.effort = "medium", effective = {}
```

The process kept running and answered the user message after all of them (one `init`, one
`result`). `initialize`'s `models[]` carry `supportsEffort: true` and `supportedEffortLevels:
["low","medium","high","xhigh","max"]` for every model (`value` `default` and the three shortcuts),
as `command-initialize.jsonl` already records; its `effort` command has `argumentHint:
"<low|medium|high|xhigh|max|auto|ultracode [on|off]>"`.

The CLI's own schema text for this request (strings of the 2.1.311 binary) says the stdin handler
takes `settings` as an object and answers `error` "apply_flag_settings: settings must be an object"
otherwise, and "apply_flag_settings is not supported in this context" when the host registered no
handler. Those are the refusals the app can meet.

## P-2. Effort at launch, and clearing it

`-- --effort high` added to the structured argv; stdin: `get_settings`;
`apply_flag_settings {"effortLevel":null}`; `get_settings`; `apply_flag_settings {"effortLevel":"max"}`;
`get_settings`.

```text
micold-2 get_settings → applied.effort = "high", sources = []     (the flag applies)
micold-3 null         → success
micold-4 get_settings → applied.effort = "medium"                  (null clears the launch flag too)
micold-5 "max"        → success
micold-6 get_settings → applied.effort = "max"
```

`claude --help` lists `--effort <level>  Effort level for the current session (low, medium, high,
xhigh, max)` as a general flag, so the same argument serves the terminal kind.

## R1. How a running session takes a change (spec Assumption 1; FR-004, FR-006)

- **Decision**: a control request, not a relaunch. `apply_flag_settings` with
  `settings: {"effortLevel": "<level>"}` sets a level; `{"effortLevel": null}` returns to Claude
  Code's default (FR-006, P-1 micold-6/7, P-2 micold-3/4). The process and the conversation are
  kept (P-1). It goes through #943's `StructuredSession::send_control` under the 10 s
  `CONTROL_ANSWER_BOUND`.
- **Busy**: deferred to the turn's end exactly as `set_model` is (`StructuredSession::defer_model`,
  `flush_deferred_model`). The CLI accepts control requests during a turn, but effort is a
  per-request parameter, so a change written mid-turn would most likely reach the turn's later API
  requests and alter the turn in progress, which FR-004 and US1 scenario 4 forbid. This is an
  assumption (no probe wrote the request mid-turn); deferral is the choice that meets FR-004
  either way. The turn-end hook writes the
  deferred model first, then the deferred effort, before the input driver's next delivery.
- **Validation is the app's**: the CLI answered `success` to `"extreme"` (P-1 micold-5), so a
  success means only "delivered". The app sends only a level it validated (R2); `ultracode`,
  `auto` and anything else never reach the CLI as a level (`auto` is Default, spec D3).
- **Alternatives rejected**: relaunching with `--resume` while idle (the spec's fallback) — slower,
  loses the process's warm state and needs the switch machinery, and P-1 shows it is not needed;
  writing `/effort high` as a user message — it would be a turn in the transcript, and the CLI's
  structured mode answers built-ins with synthetic text (#943 R4) that the view would have to hide;
  `update_settings` — writes the user's settings files, which would leak one session's choice into
  every other session and the user's own Claude Code (Principle II, FR-011).

## R2. Levels, the choice and what is supported (FR-002, FR-003, FR-007)

- **Decision**: core `structured::effort` (new):
  - `EffortLevel`: a lowercase ASCII name, 1–16 bytes, never `default`, `auto` or `ultracode`
    (so it is never confused with Default or an out-of-scope mode, and never read as a flag).
    `EFFORT_LEVELS` = `low, medium, high, xhigh, max`, least to most, the list offered when the
    CLI reports none. A newtype rather than a five-variant enum, so a level a later CLI reports
    (spec Assumption 4) is offered without an app release.
  - `EffortChoice`: `Default` | `Level(EffortLevel)`.
  - `EffortSupport { supported: bool, levels: Vec<EffortLevel> }`, read from each reported
    model's `supportsEffort` / `supportedEffortLevels` (absent → `None` on the model: unknown).
  - `effort_support(models, &ModelChoice) -> Option<&EffortSupport>`: the reported model whose
    `value` equals the choice's `cli_value()` (`default` for a Default model choice). A typed model
    name the CLI did not list → `None` (unknown).
  - `offered_levels(Option<&EffortSupport>)`: the reported levels in the CLI's order when the model
    supports effort and lists some; else `EFFORT_LEVELS` (none reported, and the clarified
    "does not support" case, where the picker still lists all five, D4).
  - `effective_level(&EffortChoice, Option<&EffortSupport>) -> Option<&EffortLevel>`: `Default` →
    none; a level → itself when support is unknown or lists it; none when the model does not
    support effort or does not list the level (FR-003, FR-010a).
  - `parse_effort_arg(arg, offered) -> Result<EffortChoice, EffortArgError>`: trimmed, any letter
    case; `default` and `auto` → `Default`; a name in `offered` → that level; anything else
    (`ultracode`, `extreme`, `high on`) → `EffortArgError { accepted }` naming `default` and the
    offered levels (FR-007).
- **Why match on the choice, not the resolved model**: `init.model` is the resolved id while
  `initialize`'s `models[].value` are the names `--model` takes; the model *choice* is what both
  share.
- **Alternatives rejected**: a closed enum (drops a level a later CLI adds); filtering the picker to
  the supported levels only when the model does not support effort (contradicts clarification D4).

## R3. Keeping the choice and launching with it (FR-010, FR-010a, FR-011)

- **Decision**: `Session.effort: EffortChoice`, persisted as `StoredSession.effort` (serde default,
  skipped when `Default`, no `schema_version` bump), set only through **new**
  `Catalog::set_effort`, beside `Catalog::set_model`. `LaunchSpec` gains `effort:
  Option<EffortLevel>`, the *effective* level; `terminal::launch_args` appends the provider's
  **new** `AiCliProvider::effort_args` (default: none; Claude: `["--effort", level]`) after
  `model_args`, in both kinds (US3 scenario 2).
- **The effective level at launch**: `DaemonState` builds the `SpawnPlan` from the record and the
  session's event history: the latest `CommandsReported` note's models (read by **new**
  `reported_models()` beside `EventStream::command_list()` in `crates/micold-daemon/src/structured.rs`,
  under the history lock, without copying the history), looked up for the model
  choice being launched (`effort_support`), then `effective_level`. No note yet (a session that
  never ran, or a terminal session created as such) → unknown → the stored level is named. A
  stored choice therefore never fails a launch (FR-010a), and Default names nothing (FR-011), so
  the user's own Claude Code settings apply as today. (That an environment `CLAUDE_EFFORT` also
  still applies is an assumption: the probes ran with it unset. Default adds nothing to the
  launch, so the app changes nothing there either way.)
- **Restore after a daemon restart and switch of kind** reuse the same `SpawnPlan` path
  (`ops::start_with_prompt`), so they need no extra code (SC-004).
- **Alternatives rejected**: sending `apply_flag_settings` right after spawn instead of the flag —
  a first prompt given at spawn (#943 FR-002) could race it; storing the effective level instead of
  the choice — loses the choice across a model change (spec Edge Cases).

## R4. A model change and the chosen level (spec Edge Case "Model change to one that does not support the chosen level")

- **Decision**: when the CLI accepts a model change (the `ModelAnswer::Accepted` path in
  `crates/micold-daemon/src/state.rs`: the `ModelChanged` note → `model_answers` →
  `follow_model_answer`), and the session's choice is a level, the daemon
  computes the effective level for the old and the new model choice; when they differ it writes
  `apply_flag_settings` with the new effective level (null when none), at once if idle (the model
  answer arrives while idle) and otherwise through the same deferral, via `ask_effort` /
  `defer_effort` in `crates/micold-daemon/src/structured.rs`. No note is added: the view
  derives "not available for this model" from the choice and the support (FR-003).
- **Alternatives rejected**: leaving it to the CLI (unrecorded behaviour for an unsupported
  level; every recorded model supports all five, so it cannot be recorded today).

## R5. Applying a pick (FR-004, FR-005, FR-008, Edge Cases)

- **Decision**: **new** `DaemonState::pick_effort`, shaped like `pick_model` with its own
  `latest`/`kept` counters (two windows: the last pick to reach the daemon wins):
  - unknown session → error; provider not Claude Code → refused "Reasoning effort is for Claude
    Code sessions", nothing stored (FR-012);
  - terminal kind → persisted, answered with an error saying it applies at the next launch (as
    `TERMINAL_MODEL_REFUSAL`);
  - not running → persisted, an `EffortChanged` note appended (the marker sits where the choice
    starts to apply: the next turn is after the relaunch), answered `OperationOk`;
  - running, idle → `apply_flag_settings` with the effective level for the session's model
    (null for Default, and null for a level the model does not support); on `success` persisted,
    broadcast, `EffortChanged` note; on `error` or `ControlFailed` (`NoAnswer`, `Stopped`,
    `Unsent`) the choice is unchanged and the requester gets the reason (FR-005);
  - running, busy → persisted and broadcast at once, deferred to the turn's end; the CLI's answer
    then becomes `EffortChanged` or `EffortRefused` (the reason), and a refused deferred pick is
    put back to the choice before it unless another pick came since (as `follow_model_answer`).
  A pending permission request does not count as busy for this purpose beyond what `turn()`
  already says: the request stays pending (spec Edge Case).
- **Shown within 1 s** (FR-004, SC-002): the choice reaches every window in the next catalog
  snapshot (`SessionSummary.effort`), the marker in the session's `StructuredEvents` frames.
- **Alternatives rejected**: no note when not running (US1 scenario 5 asks for a marker at every
  change).

## R6. `/effort` in the composer and the palette (FR-007–FR-009)

- **Decision**: `DraftCommand` gains `Effort(Option<String>)` (`/effort` with a blank rest →
  `None`, else the rest, like `Model`). The client resolves it with `parse_effort_arg` against the
  offered levels: `None` opens the picker, `Ok` sends `SetSessionEffort`, `Err` keeps the draft and
  shows the accepted values; nothing is sent as a user message. The daemon's `SessionUserMessage`
  refuses a text `classify_draft` reads as `Effort(_)` ("/effort is handled by the app"), never
  writes or queues it (SC-003), so a busy session acts at once, never queued (FR-008). `/effort`
  inside a longer message stays `Send` (`parse_invocation` needs a leading `/`).
- **Palette**: `commands/mod.rs` makes the kind rule for `MODEL_COMMAND` a rule over **new**
  `APP_COMMANDS = [model, effort]`: the CLI's `effort` built-in becomes the one `App` entry, and
  the fallback list adds it, with the description "Set the reasoning effort for later turns"
  (FR-009, listed once).

## R7. The picker (FR-001–FR-003, Principle VIII)

- **Decision**: the composer's status row gets an effort trigger beside `model_control`
  (`ui/composer.rs`): the shared `MenuSelect` (`ui/material/menu_select.rs`, feature 1272), whose
  openness and highlight are the caller's — needed because `/effort` opens it. Trigger text
  `Effort: Default` / `Effort: high`; when `effective_level` differs from the choice, a note line
  in the list and the trigger's tooltip say in words "This model does not support reasoning
  effort; Claude Code's default applies" or "high is not available for this model; Claude Code's
  default applies". Rows: Default, then `offered_levels`, the current one marked with a check and
  "current" in words. Render-free state in **new** `features/effort_picker.rs`; no new shared
  component.
- **Alternatives rejected**: `Select` (owns its openness, so `/effort` could not open it);
  `Typeahead` (free text is not wanted; the values are a closed list).

## R8. Transcript marker (FR-004, US1 scenario 5)

- **Decision**: daemon notes `StructuredEvent::EffortChanged { effort: EffortChoice }` and
  `EffortRefused { effort: EffortChoice, message }`, appended to `saved.rs` after the existing
  variants (a version-1 `.events` file keeps its meaning). `transcript/build.rs` turns them into
  **new** `EntryKind::EffortChanged { effort, refused: Option<String> }`, drawn by
  `ui/transcript/entries.rs` beside the model marker: "Reasoning effort: high from here on" /
  "Reasoning effort: Default from here on" / "Reasoning effort high was not applied: <reason>".
- **Saved history and the wire**: `ReportedModel` gains `effort: Option<EffortSupport>`. It is
  inside `SavedEvent::CommandsReported`, whose postcard encoding a new field would change, so
  `saved.rs` freezes the current shape as `SavedReportedModelV1` for the existing variant (read as
  `effort: None`) and appends **new** `SavedEvent::CommandsReportedV2` carrying the new shape,
  which every new write uses. `PROTOCOL_VERSION` +1 in M1 (events, `ReportedModel`) and M2
  (`SetSessionEffort`, `SessionSummary.effort`), `schema_hash.rs` re-pinned each time.

## R9. Tests and fixtures (FR-013)

- **Recordings (M1)**, with `scripts/capture-stream-json.sh --control …` as P-1/P-2 did:
  `effort-change.jsonl` (set `low`, `get_settings`, set null, `get_settings`, one turn) and
  `effort-launch.jsonl` (`-- --effort high`, `get_settings`). README rows name the requests. The
  capture runs with `CLAUDE_EFFORT` unset, and the M1 diff review checks the scrub (no host paths,
  account identifiers or socket paths).
- **Stand-in (M2)**: `crates/micold-daemon/tests/support/structured.rs` gains an `efforts <level>…`
  directive answering every later `apply_flag_settings` from the stdin reader: a level in the list
  or null → `success` (P-1's shape); anything else → `error` with "apply_flag_settings: …"; `efforts`
  alone refuses all. An `initialize-effort <value> <none|level,…>` directive rewrites that model's
  effort fields in the replayed `initialize` answer, so FR-010a and "model without support" are
  tested without a recording no CLI can produce today.
- **Layers**: see plan *Requirement Coverage*.

## R10. What the CLI reports about the level in use (spec Assumption 3)

- **Finding**: `get_settings` answers `applied.effort` — the level the CLI uses, including its own
  default (P-1, P-2). The spec's assumption that the CLI does not report it is wrong for 2.1.311.
- **Decision**: no change to the spec's design: the view shows the choice (FR-001), which is what
  the user controls and what persists. Showing Claude Code's resolved default (`Default (medium)`)
  would need a `get_settings` after each launch and change; recorded under *Follow-ups not done* in
  the ledger, not built.
