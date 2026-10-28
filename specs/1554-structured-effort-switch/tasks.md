# Tasks: Structured Session Reasoning Effort — pick, show and keep the effort level

**Input**: Design documents from `specs/1554-structured-effort-switch/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md),
[data-model.md](data-model.md), [contracts/](contracts/), [quickstart.md](quickstart.md)

**Tests**: Mandatory (Constitution I). In every phase a test task comes before the implementation
task it covers and must fail first. Draw code is the GUI exception, validated by quickstart Part B.

**Documentation**: `docs/daemon.md` (wire and control requests) in the milestone that changes them
(M2, M3); the user guide (FR-014) in M4 (the picker, the marker, keeping the choice) and M5
(`/effort`), the milestones that make each user-visible.

**Organization**: every user story spans the core, the daemon and the window. Phases follow the
layers of plan.md *Delivery*, one phase per milestone, and each task carries the story it serves.
Plan *Delivery* named four milestones; this file cuts five (see *Milestones*): the daemon is split
along US1 scenario 4 (busy), and the window along the stories (US1 picker, US2 `/effort`), so that
no milestone exceeds about 800 changed lines and no UI is half-wired on `main`.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: the user story (US1–US3) the task serves; Setup and Foundational tasks have none

Paths: `core` = `crates/micold-core`, `daemon` = `crates/micold-daemon`,
`client` = `crates/micold-client`.

---

## Phase 1: Setup — recorded fixtures (M1; research R9)

- [x] T001 Record with `scripts/capture-stream-json.sh --control <json>` against a logged-in Claude
  Code, with `CLAUDE_EFFORT` unset, into `core/tests/fixtures/stream-json/`:
  `effort-change.jsonl` (`apply_flag_settings {"effortLevel":"low"}`, `get_settings`,
  `apply_flag_settings {"effortLevel":null}`, `get_settings`, one user turn) and
  `effort-launch.jsonl` (`-- --effort high`, `get_settings`). Scrub host paths, account identifiers
  and socket paths as the existing fixtures are; add one row per file to the fixtures `README.md`
  naming the requests and the CLI version.

## Phase 2: Foundational — core effort types, control, commands, notes (M1)

- [x] T002 [P] Unit tests in `core/src/structured/effort.rs` (new, `#[cfg(test)]` module) per
  data-model.md *Core: `micold_core::structured::effort`*: `EffortLevel::parse` lowercases first and
  accepts "lowercase ASCII letters only, 1–16 bytes, not `default`, `auto` or `ultracode`" (refuses
  blank, 17 bytes, `-x`, `x-high`, `default`, `Auto`, `ultracode`); serde `try_from` refuses the
  same; `EffortChoice::default()` is `Default` and `is_default()`; `effort_support` finds the model
  whose `value == model.cli_value()` (Default model choice → `default`), returns `None` for a typed
  name the CLI did not list and for a listed model without effort fields; `offered_levels` table
  (supported with levels → those in the CLI's order; `supported: false` → `EFFORT_LEVELS`; `None` →
  `EFFORT_LEVELS`); `effective_level` table (`Default` → `None`; level with unknown support → it;
  supported and listed → it; not listed → `None`; `supported: false` → `None`); `availability`
  table (`Applies`, `ModelHasNoEffort`, `LevelNotForModel(level)`); `parse_effort_arg` table
  (`high`, ` Max `, `default`, `AUTO` → choices; `extreme`, `ultracode`, `high on`, a level not in
  `offered` → `EffortArgError` whose `Display` is "Use /effort with one of: default, low, medium,
  high, xhigh, max" built from the offered levels) (FR-002, FR-003, FR-007, FR-010a, FR-011)
- [x] T003 Implement T002 in `core/src/structured/effort.rs` (`EffortLevel`, `EFFORT_LEVELS`,
  `EffortChoice`, `EffortSupport`, `effort_support`, `offered_levels`, `effective_level`,
  `EffortAvailability`, `availability`, `parse_effort_arg`, `EffortArgError`, `EffortRefusal`) and
  export the module from `core/src/structured/mod.rs`
- [x] T004 [P] [US1] [US2] Tests in `core/tests/structured_commands.rs`: `parse_initialize` on
  `command-initialize.jsonl` fills every `ReportedModel.effort` with `supported: true` and the five
  levels in order; a model object without `supportsEffort`/`supportedEffortLevels` → `effort: None`;
  a level name `EffortLevel::parse` refuses is dropped; the session command list built from that
  fixture (which reports the CLI's `effort` built-in) has `/effort` exactly once, kind `App`, with
  the description "Set the reasoning effort for later turns"; the fallback list has it once too
  (FR-002, FR-003, FR-009)
- [x] T005 [P] [US2] Unit tests in `core/src/structured/commands/mod.rs`: `classify_draft` reads
  `/effort` and `/effort   ` as `Effort(None)`, `/effort max` as `Effort(Some("max"))`,
  `/effort  Max ` as `Effort(Some("Max"))`, and `say /effort max` as `Send` (FR-007, Edge Case
  "Empty input")
- [x] T006 [P] [US1] Unit tests in `core/src/structured/control.rs`: `encode_apply_effort(id,
  Some(high))` and `encode_apply_effort(id, None)` produce exactly the two lines of
  contracts/claude-effort.md (`"effortLevel":"high"` / `"effortLevel":null`, trailing newline);
  `parse_apply_effort` maps `Success(_)` → `Ok(())` and `Error { message, .. }` →
  `Refused { message }`. Tests in `core/tests/structured_parse.rs`: every `control_response` line of
  `effort-change.jsonl` and `effort-launch.jsonl` (T001) parses, and the `apply_flag_settings`
  answers read as `Ok` through `parse_apply_effort` (FR-004, FR-005, FR-006)
- [x] T007 Implement T004–T006: `ReportedModel.effort: Option<EffortSupport>` (`#[serde(default)]`)
  in `core/src/structured/event.rs`, filled by `parse_initialize`; `EFFORT_COMMAND` and
  `APP_COMMANDS = [MODEL_COMMAND, EFFORT_COMMAND]` replacing the single-name kind rule and fallback
  entry in `core/src/structured/commands/mod.rs`; `DraftCommand::Effort(Option<String>)`;
  `encode_apply_effort` and `parse_apply_effort` in `core/src/structured/control.rs`. New match arms
  outside core keep today's behaviour until M5: `daemon/src/state.rs` `SessionUserMessage`
  classification treats `Effort(_)` as `Send`, the switch-kind classification treats it like
  `Model(_)`, and `client/src/features/command_palette.rs` maps it to `Classified::Message`
- [x] T008 [P] [US1] Tests: in `core/tests/structured_transcript.rs`, an `EffortChanged { effort }`
  note → one `EntryKind::EffortChanged { effort, refused: None }` at that point, an `EffortRefused
  { effort, message }` note → one entry with `refused: Some(message)`; in `core/src/structured/saved.rs`
  unit tests, a version-1 event file holding the existing `CommandsReported` encoding (pinned bytes
  written through `SavedReportedModelV1`) still reads, with every model's `effort: None`, a new
  write uses `CommandsReportedV2` and round-trips `effort`, `EffortChanged` and `EffortRefused`
  round-trip, and `EVENTS_FORMAT_VERSION` is still 1; in `core/tests/protocol_roundtrip.rs`, both
  notes and a `ReportedModel` with `effort` round-trip in a `StructuredEvents` frame (FR-004,
  FR-005, FR-010)
- [x] T009 Implement T008: `StructuredEvent::EffortChanged { effort: EffortChoice }` and
  `EffortRefused { effort: EffortChoice, message: String }` in `core/src/structured/event.rs`;
  `SavedReportedModelV1` and the appended `CommandsReportedV2`, `EffortChanged`, `EffortRefused`
  variants (after `KindSwitched`) in `core/src/structured/saved.rs`; `EntryKind::EffortChanged` in
  `core/src/structured/transcript/entry.rs` built in `transcript/build.rs`. The exhaustive match in
  `client/src/ui/transcript/entries.rs` gets an arm that draws nothing until M4 draws the marker
  (no note is written before M2, and no window can pick before M4)
- [x] T010 Bump `PROTOCOL_VERSION` by one in `core/src/protocol/version.rs` and re-pin
  `core/tests/schema_hash.rs` (contracts/wire.md *Events (M1)*)

## Phase 3: Daemon — keep the choice, launch with it, apply it when idle (M2; US3, US1 scenarios 3, 6, 7)

- [x] T011 [P] [US3] Tests: in `core/tests/session_store.rs`, `StoredSession.effort` round-trips a
  level, a legacy record without the field reads `Default`, `Default` is not written
  (`skip_serializing_if`), a stored name `EffortLevel` refuses reads `Default`, `schema_version` is
  unchanged; unit tests in `core/src/provider.rs` and `core/src/terminal.rs`: Claude's `effort_args`
  is `["--effort", level]` for `Some` and empty for `None`, every other provider's is empty, and
  `launch_args` puts `--effort <level>` after `--model` in both kinds and adds nothing for `None`; in
  `core/tests/protocol_roundtrip.rs`, `SetSessionEffort { req, session, effort }` and
  `SessionSummary.effort` round-trip and a summary without the field decodes as `Default`
  (FR-010, FR-011, FR-012)
- [x] T012 [US3] Implement T011: `Session.effort` (`Default` for a new session) in
  `core/src/session.rs`; `StoredSession.effort` in `core/src/store.rs`; `LaunchSpec.effort:
  Option<EffortLevel>` and the `effort_args` call in `terminal::launch_args`
  (`core/src/terminal.rs`); `AiCliProvider::effort_args` (default none, Claude `--effort`) in
  `core/src/provider.rs`; `SessionSummary.effort` and `ClientMsg::SetSessionEffort` in
  `core/src/protocol/messages.rs`; `PROTOCOL_VERSION` +1 and `core/tests/schema_hash.rs` re-pinned
  (contracts/wire.md *Changed (M2)*, *New (M2)*)
- [x] T013 [P] [US1] Extend the stand-in in `daemon/tests/support/structured.rs` (research R9): an
  `efforts <level>…` directive answering every later `apply_flag_settings` from the stdin reader —
  a listed level or `null` → `success` with no `response` body, anything else → `error`
  "apply_flag_settings: …", `efforts` alone refuses all; an `initialize-effort <value>
  <none|level,…>` directive rewriting that model's `supportsEffort`/`supportedEffortLevels` in the
  replayed `initialize` answer (`none` → `supportsEffort: false`)
- [x] T014 [US1] [US3] Tests in `daemon/tests/structured_effort.rs` (new) against the stand-in: an
  idle running session picked `high` → one `apply_flag_settings {"effortLevel":"high"}` line, the
  same pid, no `--resume`, an `EffortChanged` note, `OperationOk`, and two connected clients both
  see `SessionSummary.effort` `high` within 1 s (US1 scenario 3, SC-001, SC-002); picking `Default`
  → a `"effortLevel":null` line and `Default` shown (FR-006); a refused idle pick (`efforts` alone)
  → `OperationError` with the CLI's reason, the choice unchanged, the session still running
  (FR-005); a pick after the CLI process ended → `OperationError`, unchanged; a model without
  effort support (`initialize-effort default none`) picked `high` → stored, a `null` line written
  (FR-003); a session not running → stored, an `EffortChanged` note, `OperationOk`; a terminal-kind
  session → stored, `OperationError` (`Refused`, "Effort is changed from the structured view; the
  choice is used at the next launch"); a Copilot session → `OperationError` (`Refused`, "Reasoning
  effort is for Claude Code sessions"), nothing stored (FR-012); an unknown session → `NotFound`;
  picks from two clients in turn → the last one held and shown by both; a pick on session A leaves
  session B's choice and stdin untouched (US3 scenario 4); a running busy session in M2 →
  `OperationError` ("The session is busy; the effort can be changed when the turn ends"), nothing
  stored or written (interim, replaced by M3)
- [x] T015 [US3] Tests in `daemon/tests/structured_effort.rs` for launches (SC-004): `xhigh` stored
  while not running, then started → argv has `--effort xhigh`; restarted → `--effort xhigh` and
  `--resume` the same conversation; restored after a daemon restart → `--effort xhigh`; switched to
  the terminal (PTY stand-in) → the terminal argv has `--effort xhigh`; switched back → again; a
  stored level the session model is known not to support (`initialize-effort default low,medium`,
  stored `max`, restart) → no `--effort`, the session runs (FR-010a); a new session → no
  `--effort` (FR-011); every existing structured and terminal argv assertion passes unchanged
  (SC-006)
- [x] T016 Implement T014–T015: `Catalog::set_effort` in `daemon/src/catalog.rs`; `reported_models()`
  beside `EventStream::command_list()`, `StructuredSession::ask_effort` / `effort_answer` and
  `EffortAsk` in `daemon/src/structured.rs` (over `send_control`, as `ask_model` / `model_answer`);
  `DaemonState::pick_effort` with `EffortPicks { latest, kept }` and `SpawnPlan.effort` (the
  effective level from the latest `CommandsReported` models and the record's model choice, R3) in
  `daemon/src/state.rs`; `LaunchSpec.effort` at both `LaunchSpec` sites in `daemon/src/state.rs`
  (`start_session_gated`, `respawn_primary`); dispatch of
  `SetSessionEffort` in `daemon/src/server.rs`; `SessionSummary.effort` filled from the record
- [x] T017 [P] Describe `SetSessionEffort`, its answers per session state, `--effort` at launch, the
  effective level (FR-010a) and the `apply_flag_settings` request beside the `SetSessionModel`
  section of `docs/daemon.md`

## Phase 4: Daemon — a pick during a turn, and a model change (M3; US1 scenario 4, FR-003)

- [x] T018 [US1] Tests in `daemon/tests/structured_effort.rs`: a pick `low` during a turn →
  `OperationOk` at once, the summary shows `low`, no `apply_flag_settings` line until the turn's
  `result`, then exactly one, written after a `set_model` deferred in the same turn, and an
  `EffortChanged` note after the turn (US1 scenario 4, SC-001); two picks in one turn → only the
  last is written; a deferred pick the CLI refuses (`efforts high`, pick `low`) → an
  `EffortRefused` note with the reason and the choice put back to the one before, unless another
  pick came since (FR-005); a pick while a permission request is pending → the request stays
  pending, the level applies from the next turn (Edge Case); the M2 interim busy refusal is gone
- [x] T019 [US1] Tests in `daemon/tests/structured_effort.rs` (research R4): choice `high`, an
  accepted model change to a model without `high` (`initialize-effort <value> low,medium`) → an
  `apply_flag_settings` `null` line, no note; a change back to a model that lists `high` → a `high`
  line; with choice `Default`, a model change writes nothing; a model change during a turn defers
  the effort line to the turn's end after the `set_model` line (FR-003, Edge Case "Model change to
  one that does not support the chosen level")
- [x] T020 Implement T018–T019: `StructuredSession::defer_effort` and `deferred_effort:
  Mutex<Option<Option<EffortLevel>>>` in `daemon/src/structured.rs`, the turn-end hook writing the
  deferred model first, then the deferred effort, before the input driver's next delivery;
  `EffortPicks.deferred: Option<DeferredEffort { chosen, previous }>`, `finish_effort_pick`,
  `follow_effort_answer` and the re-apply on `ModelAnswer::Accepted` in `daemon/src/state.rs`
- [x] T021 [P] Describe the turn-end write of a deferred effort (after a deferred model), the
  `EffortRefused` note and the re-apply after a model change in `docs/daemon.md`

## Phase 5: Window — the effort picker and the marker (M4; US1, US3 visible)

- [x] T022 [P] [US1] Tests in `client/tests/features_effort_picker.rs` (new): `effort_label` →
  `Effort: Default` / `Effort: high`; `EffortPickerState` rows are Default then `offered_levels`
  in order (five when none reported; the reported ones when the session's latest `CommandsReported`
  lists some), the current choice marked; open, close, highlight move and pick; `availability_note`
  → "This model does not support reasoning effort; Claude Code's default applies" /
  "high is not available for this model; Claude Code's default applies" / none; the control is
  offered only for a structured Claude Code session (none for another CLI, FR-012); and, in a unit test in
  `client/src/ui/transcript/entries.rs` beside `model_changed_text`'s,
  `effort_changed_text(&EffortChoice, Option<&str>)` →
  "Reasoning effort: high from here on" / "Reasoning effort: Default from here on" / "Reasoning
  effort high was not applied: <reason>"; a pick sends
  `SetSessionEffort` once, and an `OperationError` answer shows its message as text with the choice
  unchanged (FR-001–FR-003, FR-005)
- [x] T023 [US1] Implement T022: `client/src/features/effort_picker.rs` (new) exported from
  `client/src/features/mod.rs`; `set_effort` and the picker open/close/highlight/pick messages in
  `client/src/shell/composer.rs` beside `set_model`; the `SetSessionEffort` answers in
  `client/src/shell/daemon_sync.rs`
- [x] T024 [US1] Draw (GUI exception): the effort trigger beside `model_control` in
  `client/src/ui/composer.rs` on the shared `MenuSelect` (`ui/material/menu_select.rs`) with
  caller-held openness and highlight, the rows' "current" mark in words, the availability note line
  and tooltip; the marker in `client/src/ui/transcript/entries.rs` with the model marker's layout, its text from
  `effort_changed_text` (tested in T022), defined in `entries.rs` beside `model_changed_text`
  (research R7, R8)
- [x] T025 [US1] [US3] User guide `docs/user-guide/worktrees-and-sessions.md`: rename *Commands and
  the model* to *Commands, the model and effort*; describe the effort picker beside the model, one
  sentence per level (`low` … `max`) and Default, the unsupported-model wording, that a change
  applies from the next answer (a turn in progress keeps its level), that the choice is kept across
  restarts and a switch of kind, and that an effort change made inside the terminal screen is not
  seen (FR-014)
- [x] T026 [US1] [US3] Quickstart Part B 1–3, 6 and 7a in light and dark theme through the
  `visual-pass` skill, evidence in `specs/1554-structured-effort-switch/visual-pass/`

## Phase 6: `/effort` in the composer (M5; US2)

- [x] T027 [P] [US2] Tests in `client/tests/features_command_palette.rs`: sending `/effort max` or
  `/effort Max` → `SendIntent::SetEffort(Level(max))` and no user message; `/effort default` and
  `/effort auto` → `SetEffort(Default)`; `/effort` and `/effort   ` → the effort picker opens,
  `SendIntent::Nothing`; `/effort extreme` and `/effort ultracode` → `SendIntent::Nothing`, the
  draft kept, the `EffortArgError` text shown; a session whose model reports only `low, high` →
  `/effort max` refused naming `default, low, high`; on a busy session `/effort high` sends
  `SetSessionEffort` at once and queues nothing (FR-007, FR-008, SC-003); `/eff` lists one
  `/effort` row under "In this app" (FR-009)
- [x] T028 [US2] Tests in `daemon/tests/structured_effort.rs`: `SessionUserMessage` with
  `/effort high`, `/effort`, `/effort extreme` or `/effort default`, idle and busy →
  `OperationError` ("/effort is handled by the app"), nothing written to stdin, nothing queued
  (SC-003, contracts/wire.md *Changed behaviour (M5)*)
- [x] T029 Implement T027–T028: `SendIntent::SetEffort` and the `DraftCommand::Effort` arms
  (replacing M1's `Classified::Message`) in `client/src/features/command_palette.rs`, routed to
  `set_effort` in `client/src/shell/composer.rs`; `UserMessageRefusal` for `Effort(_)` (replacing
  M1's `Send`) in `daemon/src/state.rs`
- [x] T030 [US2] User guide `docs/user-guide/worktrees-and-sessions.md` *Commands, the model and
  effort*: `/effort <level>`, `/effort default` and `auto`, `/effort` alone opening the picker, a
  wrong value naming the accepted ones, acted on at once while busy (FR-014)
- [x] T031 [US2] Quickstart Part B 4–5 and 7b in light and dark theme through the `visual-pass` skill,
  evidence in `specs/1554-structured-effort-switch/visual-pass/`

## Phase 7: Polish (close unit)

- [x] T032 Run quickstart Part A and record the results in
  `specs/1554-structured-effort-switch/quickstart.md`

## Phase 8: Daemon — a refused deferred pick is never asked again (M6; FR-005, found at close)

Found by the close unit's quickstart Part A run: `a_refused_idle_pick_after_a_refused_deferred_one_goes_back_to_the_level_before_both`
failed about once in 100 runs with `efforts_asked` `[Some("low"), Some("high"), Some("low")]`.
The idle pick's answer is settled (`settle_effort_ask` → `reapply_effort`, state.rs) before the
deferred pick's refusal, written and answered first, has put the catalog's choice back, so the
re-apply asks the CLI again for the refused deferred level.

- [x] T033 [US1] Test in `daemon/tests/structured_effort.rs` that makes the order deterministic:
  the idle pick's answer settled while the deferred pick's refusal is not yet handled (hold that
  answer, as `hold_turn_end` holds the turn's end) → no `apply_flag_settings` for the refused
  deferred level after the two picks; once the refusal is handled the choice is the one before
  both, and the CLI is asked at most for that level (FR-005). Fails on main
- [x] T034 Implement T033 in `daemon/src/state.rs`: `reapply_effort` does not re-apply a level
  whose deferred pick is written and unanswered; the deferred answer's handling re-applies the
  choice the catalog then holds when an idle settle was held back for it
- [x] T035 [P] Stand-in (`daemon/tests/support/structured.rs`): the script's leading `models` and
  `efforts` directives take effect before stdin is read, so a request written as soon as the
  process started is answered (closes the `Rig::start` race behind the "the CLI's answer never
  happened" and "Claude did not answer the effort change" flakes in `structured_effort`)

---

## Dependencies & Execution Order

- Phase 1 → Phase 2: T006 reads T001's recordings.
- M1 (Phases 1–2) → M2 (Phase 3) → M3 (Phase 4) → M4 (Phase 5) → M5 (Phase 6). Each milestone
  builds only on earlier ones.
- Within a phase, each implementation task follows the test tasks it names.
- M6 (Phase 8) after M5. T032 runs in the close unit after M6.

## Parallel Opportunities

- M1: T002, T004, T005, T006 and T008 are in different files; T001 runs beside T002/T005.
- M2: T011 and T013 together; T017 beside the daemon tests.
- M3: T021 beside T018–T019.
- M4/M5: T022 and T027 are in different files, but M5 follows M4 on `main`.

## Implementation Strategy

Core first against recorded fixtures (M1), then the daemon keeps and applies the choice for idle
and stopped sessions (M2) and for busy ones and model changes (M3), all proven by stand-in
integration tests with nothing visible yet. M4 makes the picker and the marker visible (US1, US3),
M5 makes `/effort` in the composer work (US2). Each milestone leaves `main` green.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot). All build on
`main` as it is (plan *Integration points*). T032 (Polish, no code) is left to the close unit
(milestones rule 4). Until M5, `/effort` sent from the composer behaves as today (sent as text);
from M1 the palette lists it once under "In this app" (FR-009), which is no worse than today
(it is sent as text and has no effect either way). M1–M3 change no user-guide topic: their PRs carry
the `docs-not-needed` label (M1 touches `micold-user-facing` client files only to add match arms).

### M1 — Core: effort levels, support, control lines, `/effort` classification and notes 🎯 MVP

- **Tasks**: T001–T010
- **Deliverable**: `micold_core` validates effort levels and `/effort` arguments, works out the
  offered and effective level from the CLI's reported support, encodes and reads
  `apply_flag_settings`, lists `/effort` once as an app command, classifies `/effort` drafts, and
  turns effort notes into transcript markers that survive saved history; the recordings are in the
  tree
- **Satisfies**: FR-002, FR-003, FR-007, FR-009 (core); FR-004/FR-005 markers; FR-010 saved
  history; US2 scenario 5
- **Verify**: `cargo test -p micold-core --lib -- structured::effort structured::control
  structured::commands structured::saved`; `cargo test -p micold-core --test structured_parse
  --test structured_commands --test structured_transcript --test protocol_roundtrip --test
  schema_hash`
- **Depends on**: —
- **Tier**: full

### M2 — Daemon: keep the choice, launch with it, apply it to an idle session

- **Tasks**: T011–T017
- **Deliverable**: a `SetSessionEffort` request changes an idle or stopped Claude session's effort
  through `apply_flag_settings` (or stores it for the next launch), every client sees the choice
  within 1 s, and every launch in either kind carries the effective `--effort`, all shown by the
  daemon's stand-in tests
- **Satisfies**: US1 scenarios 3, 6, 7 and US3 scenarios 1–4 at the daemon; FR-003–FR-006,
  FR-010, FR-010a, FR-011, FR-012, FR-013 (except busy and `/effort`); SC-002, SC-004
- **Verify**: `cargo test -p micold-daemon --test structured_effort`; `cargo test -p micold-core
  --test session_store --test protocol_roundtrip --test schema_hash`
- **Depends on**: M1
- **Tier**: full

### M3 — Daemon: a pick during a turn, and a model change

- **Tasks**: T018–T021
- **Deliverable**: a pick on a busy session is answered at once and written at the turn's end
  (after a deferred model), a refused deferred pick is put back with a marker, and a model change
  re-applies the effective level, shown by the daemon's stand-in tests
- **Satisfies**: US1 scenario 4; FR-003 (model change), FR-004, FR-005 (deferred); SC-001;
  Edge Cases "Model change …", "Level picked while a permission request is pending", "Two windows"
- **Verify**: `cargo test -p micold-daemon --test structured_effort --test structured_model`
- **Depends on**: M2
- **Tier**: full

### M4 — Window: the effort picker beside the model, and the marker

- **Tasks**: T022–T026
- **Deliverable**: a structured Claude session's status row shows `Effort: <choice>` beside the
  model; the picker lists Default and the offered levels, says when the model does not support the
  level, and a pick changes the session's effort with a transcript marker; the user guide describes
  it
- **Satisfies**: US1 scenarios 1–7, US3 (visible); FR-001–FR-003, FR-014 (picker part); SC-005
  (two clicks)
- **Verify**: `cargo test -p micold-client --test features_effort_picker`; `cargo test -p
  micold-client --lib -- ui::transcript::entries`; quickstart Part B 1–3,
  6, 7a (`specs/1554-structured-effort-switch/visual-pass/`)
- **Depends on**: M3
- **Tier**: full

### M5 — `/effort` in the composer

- **Tasks**: T027–T031
- **Deliverable**: `/effort <level>`, `/effort default|auto` and `/effort` alone work from the
  composer, idle or busy, a wrong value names the accepted ones and keeps the draft, and no form of
  `/effort` reaches the CLI as a user message; the user guide describes it
- **Satisfies**: US2 scenarios 1–6; FR-007, FR-008, FR-013 (`/effort` cases), FR-014 (`/effort`
  part); SC-003, SC-005 (one message)
- **Verify**: `cargo test -p micold-client --test features_command_palette`; `cargo test -p
  micold-daemon --test structured_effort`; quickstart Part B 4–5, 7b
- **Depends on**: M4
- **Tier**: light

### M6 — A refused deferred pick is never asked again

- **Tasks**: T033–T035
- **Deliverable**: when the CLI refuses a deferred pick and an idle pick made after it, the daemon
  never asks the CLI again for the refused deferred level, whatever order the two answers are
  handled in; the effort tests no longer race the stand-in's start
- **Satisfies**: FR-005; SC-006 (the gate stays green)
- **Verify**: `cargo test -p micold-daemon --test structured_effort` (the T033 test, and the whole
  binary in a loop of 100 runs with no failure)
- **Depends on**: M5
- **Tier**: full (concurrency in `state.rs`)
