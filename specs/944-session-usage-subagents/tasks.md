---
description: "Tasks for #944 — structured session usage, context meter, subagents and background tasks"
---

# Tasks: Structured Session — Usage, Context Meter, Subagents and Background Tasks

**Input**: `specs/944-session-usage-subagents/` — [plan.md](plan.md), [spec.md](spec.md),
[research.md](research.md), [data-model.md](data-model.md), [contracts/](contracts/),
[quickstart.md](quickstart.md)

**Tests**: mandatory (Constitution I). Every behaviour task has a failing test task before it.
Draw code in `ui/transcript/session_bar.rs`, the X1–X5 additions and the `ProgressBar` paint step
use the GUI exception: geometry gates and the quickstart Part B visual pass cover them.

**Documentation**: feature 490's Settings note and both *Claude plan usage* guide sections change in
M2, with the plan-usage feed; *Structured sessions* changes in M6, when the meter, figures, threads
and background tasks become visible (Constitution VII, FR-027).

**Cross-platform**: core never branches on the OS; figures are formatted by one core function
(research R9); the environment variable is set through `Command::env` on every platform. The daemon
tests run the POSIX stand-in CLI of `crates/micold-daemon/tests/support/structured.rs` (unix-only,
as `structured_session.rs` already is).

**Fixtures**: the recorded files in `crates/micold-core/tests/fixtures/stream-json/` (Claude Code
2.1.302) hold usage, `modelUsage`, `autocompact_state` and `rate_limit_event`, but no subagent and
no background task. T003 records four new files with Claude Code 2.1.306 or later (research R11);
it needs a logged-in `claude` on `PATH`. Core tests reuse the helpers of
`crates/micold-core/tests/support/stream_json.rs` (`events_of`, `lines_of`, the synthetic-event
builder) and extend them there rather than copying them.

**#939 dependency**: M1–M4 need only what is on `main` today (#939 M1–M2 core included).
**M5 needs #939 M3** (`features::structured_transcript` and its routing) and **M6 needs #939 M4, M5
and M6** (`DisclosureCard`, the transcript view, the pane). Do not start M5 or M6 before those
#939 milestones are merged (plan *Dependency on #939*). If #939's merged names differ from those
used below, M5/M6 adapt to #939's shape (contracts/transcript-extensions.md E-5); the core
contract does not change.

**Organization**: phases follow the plan's delivery order (*Delivery*): the core model first, then
the daemon feed, the remaining core rules, the shared UI, the client's view state and the drawing
last, so no half-wired UI reaches `main` (milestones.md rule 6). Each task carries the story it
serves.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: US1 (context meter), US2 (turn and session usage), US3 (subagent threads), US4
  (plan-usage feed), US5 (background tasks)

---

## Phase 1: Setup

- [x] T001 Create the render-free module `crates/micold-core/src/structured/insights/` with empty
  `mod.rs`, `digest.rs`, `usage.rs`, `context.rs`, `tasks.rs`, `subagents.rs`, `figures.rs`; add
  `pub mod insights;` to `crates/micold-core/src/structured/mod.rs`; create
  `crates/micold-core/src/format.rs` and add `pub mod format;` to `crates/micold-core/src/lib.rs`;
  create an empty `crates/micold-core/tests/structured_insights.rs` (plan *Project Structure*)
- [x] T002 [P] Make the capture script forward subagent text (research R5, R11): first add a
  failing assertion to `scripts/tests/capture-stream-json.test.sh` that the CLI the script runs
  sees `CLAUDE_CODE_FORWARD_SUBAGENT_TEXT=1`, then export the variable in
  `scripts/capture-stream-json.sh`; the script's other arguments stay as they are
- [x] T003 Record the fixtures of research R11 (FR-028) with `scripts/capture-stream-json.sh` (Claude Code
  2.1.306 or later) into `crates/micold-core/tests/fixtures/stream-json/`: `subagent-turn.jsonl`
  (a foreground `Agent` call that reads a file), `nested-subagent.jsonl` (a subagent allowed the
  `Agent` tool through `--agents`, which starts its own), `background-command.jsonl` (a
  `run_in_background` `Bash` command that finishes, with its `task_notification`),
  `background-subagent.jsonl` (an `Agent` call with `run_in_background`); write the synthetic
  `insights-tolerance.jsonl` by hand with malformed `usage`, `modelUsage`, `autocompact_state`,
  `rate_limit_event`, `task_*` and `background_tasks_changed` lines (a number as a string, a
  negative number, a missing object, a wrong type); add one row per file to the fixtures'
  `README.md` table (CLI version, prompt, what it covers). If a recording shows that nested
  subagent events are **not** marked with their own parent tool call, record that in the ledger
  (spec *Assumptions* fallback, research R7) before M3 starts. With no logged-in `claude`
  2.1.306+ available, escalate (category: needs a human step); do not substitute synthetic
  recordings, which FR-028 forbids

---

## Phase 2: Foundational (blocks every story)

- [x] T004 Extract the replace-or-continue rule (research R1): first write unit tests
  (`#[cfg(test)]`) in a new `crates/micold-core/src/structured/sync.rs` for `Continuity` — an empty
  `live: false` message replaces; a `live: false` message whose first `seq` equals `next_seq`
  after a non-live message that held something continues; one that does not continue replaces; a
  `live: true` message continues; `next_seq` follows the last event — then implement `Continuity {
  next_seq, last_live, holds_anything }` with one decision method used before appending a message's
  events, add `mod sync;` (crate-visible) to `crates/micold-core/src/structured/mod.rs`, and make
  `Transcript::apply` in `crates/micold-core/src/structured/transcript/mod.rs` use it with no
  behaviour change: `cargo test -p micold-core --test structured_transcript` stays green unchanged
- [x] T005 [P] Unit tests (`#[cfg(test)]`) in `crates/micold-core/src/format.rs` for
  `group_digits(u64) -> String`: `0` → "0", `999` → "999", `1000` → "1,000", `37141` →
  "37,141", `1000000` → "1,000,000", `u64::MAX` grouped (research R9)
- [x] T006 Move `thousands` from `crates/micold-client/src/features/worktree_form.rs` to
  `micold_core::format::group_digits` in `crates/micold-core/src/format.rs` and call it from
  `worktree_form.rs` (no copy left behind); T005 passes and
  `cargo test -p micold-client --test features_worktree_form` stays green unchanged
- [x] T007 Write failing I1 tests in `crates/micold-core/tests/structured_insights.rs`
  (contracts/insights-model.md *I1*, FR-023) over every recorded fixture: applying it one event
  per `live: true` message, in chunks of 3 after an initial `live: false` history, and as one
  `live: false` message all give a `SessionInsights` equal (`PartialEq`) to the one built by a
  single `live: false` message of the held events with the same `omitted`; a second `live: false`
  message that does not continue replaces the state; past `MAX_EVENTS` (filler events, as
  `structured_transcript.rs` T017 builds them) the oldest leave, `omitted` grows by the number
  dropped, and the value still equals a fresh build of the held events with that `omitted`
- [x] T008 Implement `SessionInsights` in `crates/micold-core/src/structured/insights/mod.rs` and
  the `Digest` enum in `crates/micold-core/src/structured/insights/digest.rs` per data-model.md
  *SessionInsights* and *Digest*: `sync: Continuity`, `omitted`, `held`, `first_held`,
  `digests: VecDeque<(u64, Digest)>`; `new()`, `apply(omitted, &[SequencedEvent], live)` (T004's
  rule; one digest per event that matters, unreadable fields → `None`, a line of another shape →
  no digest, never an error, FR-024); the `MAX_EVENTS` bound counting held events (not digests)
  that drops digests older than `first_held` and refolds the derived state from the digests still
  held, at most once per applied message (research R8); derive `PartialEq, Debug, Clone`. The
  derived state is empty in this task; T007 passes on it

**Checkpoint**: an insights value applies messages exactly as `Transcript` does; the rules come next.

---

## Phase 3: User Story 1 — context meter, core (P1) 🎯 MVP

**Goal**: the session's context reading — used, window, threshold, percent, warning — from its
events.

**Independent Test**: `cargo test -p micold-core --test structured_insights` (context cases) and
`cargo test -p micold-core --lib structured::insights::figures`.

### Tests for User Story 1 ⚠️ write first, see them fail

- [x] T009 [P] [US1] Context tests in `crates/micold-core/tests/structured_insights.rs`
  (contracts/insights-model.md *I2*, research R2, FR-001, FR-005 — the reading changes on the very
  event, O(1), with no timer): for every recorded fixture `context()` equals
  the last main-scope `Assistant`'s `input_tokens + cache_creation_input_tokens +
  cache_read_input_tokens + output_tokens` (`tool-turn`: `2 + 8515 + 28624 + 16`) and the window
  is `modelUsage[<that model>].contextWindow` of the latest held `Result` (`1000000`); threshold
  `384000` from `autocompact_state` when `value.enabled`, none when disabled, absent or malformed
  (FR-002); `state()` is `NoData` with no main response held, `TokensOnly { used }` before the
  first `Result` and for a model the results do not list (FR-004), `Measured { percent, .. }`
  otherwise with `percent` rounded half up; `warning` at `used ≥ threshold`, and at
  `used × 10 ≥ window × 9` with no threshold, not below (FR-003); a larger then a smaller main
  response move the reading both ways (compaction, FR-005); a parented (subagent) `Assistant`
  never moves it

### Implementation for User Story 1

- [x] T010 [US1] Implement `ContextReading { used, model, window, threshold }`, `ContextState::
  {NoData, TokensOnly { used }, Measured { used, window, percent, threshold, warning }}` and
  `state()` in `crates/micold-core/src/structured/insights/context.rs` (a `Measured` cannot be
  built without a window, Constitution V); the `AssistantUsage { model, used }` (main scope only,
  read from `raw.message.usage`/`raw.message.model`), `Autocompact { threshold }` and the
  `Result`'s per-model `contextWindow` digests in `digest.rs`; `SessionInsights::context()` in
  `mod.rs`, folded in the refold too; T009 and T007 pass

**Checkpoint**: the meter's figures exist in core for every fixture.

---

## Phase 4: User Story 2 — turn and session usage, core (P1)

**Goal**: each turn's tokens, cost and per-model figures (FR-009 differences) and the session
totals; display strings for every figure of US1 and US2.

**Independent Test**: `cargo test -p micold-core --test structured_insights` (usage cases) and
`cargo test -p micold-core --lib structured::insights`.

### Tests for User Story 2 ⚠️ write first, see them fail

- [x] T011 [US2] Usage tests in `crates/micold-core/tests/structured_insights.rs`
  (contracts/insights-model.md *I2*, *I3*, research R3): each `turn(seq)` of a fixture's `Result`
  has the result's `usage` tokens (FR-006, SC-001); `text-turn` then `resume-init` gives the second
  turn cost `0.0954 − 0.0371` and per-model output `20 − 14` (FR-009, D5); 1, 2 and 50 turns
  (fixture turns repeated with rising cumulative cost) total the sum of the counted turns, none
  twice (SC-002); a cumulative figure lower than the previous → `restarted`, figures as they are; a
  first held result with `omitted == 0` → figures as they are; with `omitted > 0` and no previous
  held → `cost_usd` and `models` `None`, turn not counted; evicting the previous result past
  `MAX_EVENTS` changes that turn to not counted; a `Result` without `usage` → `tokens` `None`, not
  counted; a `TurnEndedWithoutResult` → `turns_left_out + 1`; `totals().partial == (omitted > 0)`
  (FR-008); per-model totals sum the counted turns' per-model figures
- [x] T012 [P] [US2] Unit tests (`#[cfg(test)]`) in
  `crates/micold-core/src/structured/insights/figures.rs` for the strings of data-model.md
  *Formatting*: `turn_figures` → `"$0.0052 est."` (through #939's `format_cost`) and
  `"in 4 · out 106"` (grouped by `group_digits`); `turn_details` → cached input line, one line per
  model with its tokens and cost, and the note "estimated at API list prices; a subscription plan
  does not bill this amount" (FR-026); `totals_line` → `"$0.14 est. · 3 turns"`; `totals_details`
  → each token figure, per-model rows, the left-out count, "covers the turns still held" when
  `partial`; `context_label` → `"37,141 / 1,000,000 · 4%"`, `"No data yet"` for `NoData`, tokens
  only for `TokensOnly`, and "Near compaction" when `warning`; `context_details` names the
  threshold; an absent figure yields no string (FR-004, FR-008)

### Implementation for User Story 2

- [x] T013 [US2] Implement `Tokens`, `ModelFigures`, `TurnUsage` and `SessionTotals` per
  data-model.md *Turn usage* in `crates/micold-core/src/structured/insights/usage.rs` (a
  `modelUsage` entry missing a number is skipped; `counted = tokens.is_some() &&
  cost_usd.is_some()`), the `Result { tokens, cost_cum, models_cum }` and
  `TurnEndedWithoutResult` digests in `digest.rs`, and `turn(result_seq)` and `totals()` on
  `SessionInsights`, recomputed in the refold after eviction; T011 and T007 pass
- [x] T014 [US2] Implement `turn_figures`, `turn_details`, `totals_line`, `totals_details`,
  `context_label` and `context_details` in `crates/micold-core/src/structured/insights/figures.rs`
  using `micold_core::format::group_digits` and `structured::transcript::format_cost`; T012 passes
- [x] T015 [US1] [US2] Tolerance tests and fixes (I6, SC-006, FR-024): in
  `crates/micold-core/tests/structured_insights.rs`, `insights-tolerance.jsonl` applies without a
  panic, every malformed usage, `modelUsage` and `autocompact_state` field leaves its figure
  `None`, and the well-formed lines around it still count; fix `digest.rs` until it passes

**Checkpoint**: M1 complete — meter, turn and total figures and their text exist in core.

---

## Phase 5: User Story 4 — structured sessions feed the plan-usage indicator (P2)

**Goal**: each `rate_limit_event` a running structured CLI writes becomes a feature 490 reading,
and the CLI forwards subagent text.

**Independent Test**: `cargo test -p micold-core --test plan_usage_rate_limit_event` and
`cargo test -p micold-daemon --test plan_usage_structured --test structured_session`.

### Tests for User Story 4 ⚠️ write first, see them fail

- [ ] T016 [P] [US4] Core tests in a new `crates/micold-core/tests/plan_usage_rate_limit_event.rs`
  (contracts/plan-usage-source.md *Parse*): every recorded fixture's `rate_limit_event` parses to
  two windows, "5-hour" and "Weekly", with `used_tenths = round(utilization × 1000)` and their
  `resetsAt`; `None` for a line that is not JSON, of another `type`, or without
  `rate_limit_info.unifiedWindows`; a window with `utilization` a string, negative, beyond `u32`
  after scaling, or `resetsAt` before `obtained_at` is dropped, and an event with no usable window
  → `None`; the top-level `utilization`/`resetsAt` are not read; the tolerance fixture's
  `rate_limit_event` lines give `None` without a panic
- [ ] T017 [P] [US4] Daemon tests in a new `crates/micold-daemon/tests/plan_usage_structured.rs`
  (contracts/plan-usage-source.md *Guarantees*, SC-004), with the stand-in CLI of
  `tests/support/structured.rs` replaying `text-turn.jsonl` and the setting helpers of
  `tests/plan_usage_setting.rs` reused, not copied: **Show Claude plan usage** on → a test client
  receives `PlanUsageChanged` with the fixture's two windows within 10 s (FR-010); off → none and
  no reading kept (FR-011); a structured session restored from a saved history containing the
  event → no `PlanUsageChanged` (FR-013); an unreadable event leaves the current reading unchanged
  (FR-012); the event stays in the session's event history in every case (US4 s3)
- [ ] T018 [P] [US4] In `crates/micold-daemon/tests/structured_session.rs`, assert the stand-in
  sees `CLAUDE_CODE_FORWARD_SUBAGENT_TEXT=1` in its environment and #938's seven structured
  arguments unchanged (research R5, D8); in `crates/micold-core/src/provider.rs` unit tests, the
  Claude provider's `structured_env()` is `[("CLAUDE_CODE_FORWARD_SUBAGENT_TEXT", "1")]` and every
  other provider's is empty

### Implementation for User Story 4

- [ ] T019 [US4] Extract `status_line::window`'s per-window rule into
  `plan_usage::limit_window(key, used_percent: f64, resets_at: Option<&Value>, obtained_at) ->
  Option<LimitWindow>` in `crates/micold-core/src/plan_usage.rs`, called by
  `crates/micold-core/src/plan_usage/status_line.rs`; add `pub mod rate_limit_event;` and
  implement `parse(raw: &str, obtained_at: i64) -> Option<UsageReading>` in
  `crates/micold-core/src/plan_usage/rate_limit_event.rs`; T016 passes and
  `plan_usage_status_line`, `plan_usage_user_status_line` and `plan_usage` tests stay green unchanged
- [ ] T020 [US4] Add `AiCliProvider::structured_env() -> &'static [(&'static str, &'static str)]`
  (default `&[]`; Claude: forward subagent text) in `crates/micold-core/src/provider.rs`, and apply
  it in `StructuredSession::spawn` in `crates/micold-daemon/src/structured.rs` after
  `apply_env_hygiene` and before `spec.env`; T018 passes
- [ ] T021 [US4] Daemon feed (research R4): extract `DaemonState::offer_plan_usage(session,
  reading) -> bool` (switch check, `note_plan_usage`, `PlanUsageChanged` broadcast) from the status
  handler in `crates/micold-daemon/src/hooks.rs` into `crates/micold-daemon/src/state.rs` and call
  it from `hooks.rs`; add the plan-usage feed (`UnboundedSender<(SessionId, UsageReading)>` held by
  `DaemonState`, drained by a task spawned at service start with an `Arc<DaemonState>`); in
  `spawn_structured`'s `on_event`, parse each `StructuredEvent::Unrecognised { raw }` with
  `"type":"rate_limit_event"` through `rate_limit_event::parse(raw, unix_secs())` and send a
  reading to the feed; an unreadable event logs one debug line per (session, reason) until that
  session's next accepted reading (feature 490 FR-016, as `hooks.rs` logs a rejected status);
  T017 passes and `plan_usage_route`, `plan_usage_setting`, `status_relay` pass unchanged (SC-007)
- [ ] T022 [US4] Documentation (FR-027): `docs/user-guide/settings.md` *Claude plan usage* and
  `docs/user-guide/worktrees-and-sessions.md` *Claude plan usage* say that structured sessions are
  a usage source too (the CLI's plan-limit event, read locally, no request); the Settings note of
  **Show Claude plan usage** in `crates/micold-client/src/ui/settings/environment.rs` says the
  same, with any test asserting that text updated

**Checkpoint**: M2 complete — the app-bar indicator learns from structured sessions.

---

## Phase 6: User Stories 3 and 5 — task table and thread index, core

**Goal**: background tasks and subagent state from the CLI's `task_*` lines; every parented event
attributed to its thread and every thread placed under its parent, at any depth.

**Independent Test**: `cargo test -p micold-core --test structured_insights` (task and thread
cases) on the T003 recordings.

### Tests ⚠️ write first, see them fail

- [ ] T023 [P] [US5] Task tests in `crates/micold-core/tests/structured_insights.rs`
  (contracts/insights-model.md *I5*, research R6): `background-command` shows the task `Running`
  (kind `Command`, `background`, its description and `tool_use_id`) then `Finished` with its
  `summary` as outcome, and `running_background()` drops 1 → 0 (FR-020); `background-subagent`
  gives a `Subagent` background task; `task_for_tool(tool_use_id)` finds the task (FR-021); an
  `Init` ends every `Running` task as `EndedWithoutReport`, as does a later
  `background_tasks_changed` without it for a background task (FR-022); `TurnEndedWithoutResult`
  ends a running foreground task, not a background one (FR-018); a `task_notification` for an
  unknown `task_id` creates an entry from it (spec Edge Cases); a later `task_started` of the same
  `task_id` reopens it as `Running`; `task_progress` sets `activity` from `summary`, else
  `description`, else `last_tool_name`; `display_state(false)` shows `Running` as
  `EndedWithoutReport` and leaves the table unchanged; with no background task,
  `background_tasks()` is empty (US5 s5); the tolerance fixture's task lines are skipped (I6)
- [ ] T024 [US3] Thread tests in `crates/micold-core/tests/structured_insights.rs`
  (contracts/insights-model.md *I4*, research R7, SC-003): in `subagent-turn` and
  `nested-subagent`, every `Assistant`/`User` event with a `parent_tool_use_id` is counted in that
  parent's `ThreadSummary` and in no other; `tool_calls` and `latest_tool` follow its `ToolUse`
  blocks; with the main `Transcript` built from the same events, `owner(parent, &main)` is `Main`
  for the outer thread and `Thread(outer)` for the inner one, at every depth (FR-016); a parent not
  held in the main transcript or any thread (evicted, or a synthetic unknown id) → `Unknown`, and
  `unknown_threads(&main)` lists it (FR-017); a thread's `ThreadView.state` comes from the
  `Subagent` task whose `tool_use_id` is the parent, else from the parent card's `ToolStatus`
  (`Running`/`Done`/`Failed`/`NoResult` → running/finished/failed/ended without a result,
  FR-018); `activity` is the task's activity, else the latest tool name; I1 (T007) now also
  compares tasks and threads. If T003 recorded that nested events are not marked with their own
  parent, test the spec's fallback instead: unmarked events go to the nearest marked ancestor
- [ ] T025 [P] [US3] [US5] Unit tests (`#[cfg(test)]`) in
  `crates/micold-core/src/structured/insights/figures.rs` for `task_label(&Task, cli_running)`
  ("Running", "Finished", "Failed", "Stopped", "Ended without a report", with the outcome when
  reported) and `thread_summary_line(&ThreadView)` ("Subagent · running · 3 tool calls · Reading
  notes.txt"; no activity once ended; "1 tool call" singular)

### Implementation

- [ ] T026 [US5] Implement `TaskTable`, `Task`, `TaskKind::{Command, Subagent}`,
  `TaskState::{Running, Finished, Failed, Stopped, EndedWithoutReport}` and `display_state` per
  data-model.md *TaskTable* in `crates/micold-core/src/structured/insights/tasks.rs`; the
  `Task(TaskEvent)`, `LiveSet(Vec<LiveTask>)` and `Init` digests in `digest.rs`; `tasks()`,
  `background_tasks()`, `running_background()` and `task_for_tool()` on `SessionInsights`, folded
  in the refold; T023 passes
- [ ] T027 [US3] Implement `ThreadIndex`, `ThreadSummary`, `Owner::{Main, Thread(String),
  Unknown}`, `SubagentState` and `ThreadView` per data-model.md *ThreadIndex* in
  `crates/micold-core/src/structured/insights/subagents.rs`; the `Parented { parent, tool_uses }`
  digest in `digest.rs`; `thread(parent)`, `threads_owned_by(Owner)`, `owner(parent, &Transcript)`,
  `unknown_threads(&Transcript)` and a `thread_view(parent, Option<ToolStatus>)` on
  `SessionInsights`, folded in the refold; T024 passes
- [ ] T028 [US3] [US5] Implement `task_label` and `thread_summary_line` in
  `crates/micold-core/src/structured/insights/figures.rs`; T025 passes

**Checkpoint**: M3 complete — all of the core contract (I1–I6) holds on the recordings.

---

## Phase 7: Shared components (US1 meter, US3/US5 icons)

**Goal**: the shared `ProgressBar` can mark a threshold and warn; the two new icons exist.

**Independent Test**: `cargo test -p micold-client --lib ui::material::progress_bar` and
`cargo test -p micold-client --test icons --test showcase_completeness --test material_builder_api`.

### Tests ⚠️ write first, see them fail

- [ ] T029 [P] [US1] In-crate tests in `crates/micold-client/src/ui/material/progress_bar.rs`
  (research R12, FR-002, FR-003): `.marker(Ratio)` places a 2 px tick inside the track at the
  ratio (0, a middle value and 1 stay inside); `.warning(true)` fills in the `error` role and the
  fill passes `composition_contrast` against the track in both schemes; without either step the bar
  draws as before; add the two builder steps to `crates/micold-client/tests/material_builder_api.rs`
  if it lists `ProgressBar`'s steps
- [ ] T030 [P] [US3] [US5] In `crates/micold-client/tests/icons.rs`, add codepoint cases for
  `Icon::Subagent` and `Icon::BackgroundTask` (unique, present in the icon font as
  `tests/icons_font.rs` checks)

### Implementation

- [ ] T031 [US1] Implement `.marker(Ratio)` and `.warning(bool)` on `material::ProgressBar` in
  `crates/micold-client/src/ui/material/progress_bar.rs`; T029 passes
- [ ] T032 [P] [US3] [US5] Add `Icon::Subagent` and `Icon::BackgroundTask` to
  `crates/micold-client/src/icons.rs` (variant, list, codepoint); T030 passes
- [ ] T033 [US1] Showcase: extend the `progress_bar` entry in
  `crates/micold-client/src/showcase/sections/surfaces.rs` with a marked bar and a warning bar;
  `showcase_completeness`, `showcase_captions` and `layout_snapshot` stay green (regenerate the
  snapshot only for the changed entry)

**Checkpoint**: M4 complete — the meter's parts are in the material library and the showcase.

---

## Phase 8: View state in the client (US3 fold, US5 list fold) — needs #939 M3

**Goal**: each window keeps a `SessionInsights` per structured session beside #939's
`SessionTranscript`, with its open threads and background-list fold.

**Independent Test**: `cargo test -p micold-client --test features_structured_insights`.

### Tests ⚠️ write first, see them fail

- [ ] T034 [US3] Reducer tests in a new `crates/micold-client/tests/features_structured_insights.rs`
  (data-model.md *View state*, contracts/insights-model.md *Client seam*): `apply(session, omitted,
  &events, live, &main_transcript)` after #939's transcript for the same message gives the same
  `SessionInsights` as core does; a thread is folded by default and `toggle_thread` opens it, and
  `thread(session, parent)` then holds exactly `Transcript::nested(parent)` of
  `subagent_events(parent)` (FR-014, FR-015); later live events naming the parent reach an open
  thread and not a folded one; after a main rebuild or eviction (`rebuilds()` or
  `omitted_notice()` changed) an open thread is rebuilt from `subagent_events`; two `State` values
  (two windows) applying the same messages hold the same insights and independent fold sets
  (FR-019); `toggle_tasks` folds and unfolds the background-task list; two sessions stay
  isolated (Principle II); `forget(session)` drops the session; a turn's footer figures are found
  by the footer entry's `EntryKey::At { seq, .. }` through `turn(seq)` (FR-006); with the session's
  CLI not running, its running tasks and threads show as ended (FR-018, FR-022)

### Implementation

- [ ] T035 [US3] Implement the render-free `crates/micold-client/src/features/structured_insights.rs`
  (`State`, `SessionExtras { insights, open_threads, tasks_open, seen }`, `apply`, `toggle_thread`,
  `thread`, `toggle_tasks`, `forget`) and register it in `crates/micold-client/src/features/mod.rs`;
  T034 passes and `tests/features_are_render_free.rs` stays green
- [ ] T036 [US3] Wire it: `State.structured_insights` in `crates/micold-client/src/app.rs`; in
  `crates/micold-client/src/shell/daemon_sync.rs` route each `StructuredEvents` message to it right
  after #939's `structured_transcript` (passing that session's main `Transcript`) and follow #939's
  forget paths; add `InsightsMsg { ToggleThread { session, parent }, ToggleBackgroundTasks {
  session } }` beside #939's `TranscriptMsg` in the session message family, handled in the shell
  by calling the feature; nothing draws it yet (no half-wired UI). Cover the routing with the
  test that covers #939's routing of the same message in `shell/daemon_sync.rs` (extend it, do not
  copy it): after a `StructuredEvents` message both features hold the session; forgetting the
  session drops it from both

**Checkpoint**: M5 complete — every window holds the figures, threads and tasks, unseen.

---

## Phase 9: The session bar and X1–X5 drawing — needs M4, M5 and #939 M4–M6

**Goal**: the meter, totals, background tasks, footer figures and subagent threads appear in the
transcript pane.

**Independent Test**: `cargo test -p micold-client --test layout_snapshot` (with the new gate) and
quickstart Part B.

### Tests ⚠️ write first, see them fail

- [ ] T037 [US1] Geometry gate `crates/micold-client/tests/gates/session_bar_fits.rs`, wired into
  `crates/micold-client/tests/layout_snapshot.rs`: at the narrowest transcript pane width the
  meter, the totals and the running-task count stay on one row inside the pane, the warning icon
  and "Near compaction" included (SC-005); `layout_snapshot` gains a transcript pane state with the
  session bar and one open thread

### Implementation

- [ ] T038 [US1] [US2] [US5] X1 (contracts/transcript-extensions.md): the session bar in a new
  `crates/micold-client/src/ui/transcript/session_bar.rs` — `ProgressBar` with `.marker` and
  `.warning`, `context_label` (no bar for `TokensOnly`, "No data yet" for `NoData`), the warning
  `Icon::UsageWarning` + "Near compaction", `totals_line`, and while any run "N background tasks
  running"; details through the shared `Tooltip` (`context_details`, `totals_details`); under it
  an `AccordionSection`, folded by default and sent `InsightsMsg::ToggleBackgroundTasks`, with one
  row per background task (kind icon, description, `task_label` + icon, outcome); drawn at the top
  of `ui::transcript::view` in `crates/micold-client/src/ui/transcript/mod.rs`; T037 passes
- [ ] T039 [US2] X2 (first confirm the footer #939 M5 merged: its view model and its "session
  total" label; adapt to its shape per E-5): after #939's duration and "session total" in the footer entry
  (`crates/micold-client/src/ui/transcript/entries.rs`), the turn's `turn_figures` from
  `turn(seq)`, with `turn_details` (cached input, per-model rows, the estimate note) in its tooltip
  (FR-006, FR-026); a turn without figures adds nothing
- [ ] T040 [US3] X3 and X5 in `crates/micold-client/src/ui/transcript/entries.rs`: a tool-call card
  whose id has a thread draws, inside its body, a nested `DisclosureCard` headed by
  `thread_summary_line`, folded by default, toggled by `InsightsMsg::ToggleThread`, whose body draws
  the open thread's nested `Transcript` entries through #939's per-entry draw functions and the
  window's `BlockRenderers`, indented by the shared spacing token, recursively at any depth
  (FR-014–FR-016); the `SubagentNotice` entry is replaced in place by one placeholder
  `DisclosureCard` per `unknown_threads` entry, headed "Subagent of an earlier step (<parent id>)",
  opening the same way (FR-017); a failed thread shows "Failed" with `Icon::ToolFailed` (E-4)
- [ ] T041 [US5] X4 in `crates/micold-client/src/ui/transcript/entries.rs`: a tool-call card whose id
  has a task (`task_for_tool`) shows the task's `task_label` and state icon as a trailing
  `IconLabel` in its header (FR-021)
- [ ] T042 [P] [US1] Showcase: a session bar over a fixture (normal, warning, no data, with a
  running background task) in `crates/micold-client/src/showcase/sections/` beside #939's
  transcript entry; `showcase_completeness` stays green
- [ ] T043 [P] [US1] [US2] [US3] [US5] User guide (FR-027): `docs/user-guide/worktrees-and-sessions.md`
  *Structured sessions* describes the context meter and its warning, the turn's cost and tokens
  (an estimate at API list prices) and the session totals, subagent threads under their card and
  the placeholder for an earlier step, and the background-task list and its states
- [ ] T044 [US1] [US2] [US3] [US5] Quickstart Part B (steps 1–8) through the `visual-pass` skill,
  evidence into `specs/944-session-usage-subagents/visual-pass/`; fix what it finds

**Checkpoint**: M6 complete — the issue's acceptance is visible in the pane.

---

## Phase 10: Polish (close unit; no code)

- [ ] T045 Run quickstart Part A in full and `mise run gate`; record the results (SC-007: every
  existing terminal-session and plan-usage test passes unchanged) in the ledger
- [ ] T046 Converge: check every FR and SC of spec.md against plan *Requirement Coverage* and the
  merged tests; tick the tasks

---

## Dependencies & Execution Order

- **Setup (T001–T003)** → **Foundational (T004–T008)** → US1 core (T009–T010) and US2 core
  (T011–T015), which share `digest.rs` and run in that order (the window comes from the `Result`
  digest T013 completes).
- **US4 (T016–T022)** needs M1 (T003's tolerance fixture); the daemon tasks need T019.
- **US3/US5 core (T023–T028)** needs T008 and the T003 recordings; it uses #939's
  `Transcript`, on main.
- **Shared components (T029–T033)** need nothing from this feature.
- **Client state (T034–T036)** needs T028 and **#939 M3**.
- **Drawing (T037–T044)** needs T032–T033, T036 and **#939 M4, M5, M6**.

### Parallel opportunities

- T003 after T002 (it records with the changed script). T002 beside T001; T005 beside T004; T009 beside T012 (T011 shares T009's file).
- T016, T017, T018 in parallel (core test file, daemon test file, existing daemon test).
- T023 beside T025 (T024 shares T023's file); T029 and T030 in parallel; T042 and T043 in parallel.
- M4 (T029–T033) can run at any time beside M1–M3.

## Implementation Strategy

MVP first: M1 gives the meter's and the turn's figures in core, tested on the recordings. M2 ships
the first user-visible change (the plan-usage indicator learns from structured sessions) without
any view. M3 and M4 complete the core and the shared components while #939's client milestones
land. M5 and M6 wait for #939 M3 and M4–M6, then make everything visible at once.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot). M1–M4 depend on
nothing beyond `main`; **M5 needs #939 M3 merged; M6 needs #939 M4, M5 and M6 merged**.

### M1 — Context and usage figures in core 🎯 MVP

- **Tasks**: T001–T015
- **Deliverable**: `micold_core::structured::insights` derives, from any recorded fixture, the
  context reading (used, window, threshold, percent, warning), each turn's tokens and FR-009 cost
  and per-model figures, and the session totals, with their display strings; the capture script
  forwards subagent text and the four new recordings are in the fixtures
- **Satisfies**: US1 scenarios 1–6 and US2 scenarios 1–5 at the model; FR-001–FR-009, FR-023,
  FR-024, FR-026 (strings), FR-028 (recordings); SC-001, SC-002, SC-006 (usage part)
- **Verify**: `cargo test -p micold-core --test structured_insights --test structured_transcript`,
  `cargo test -p micold-core --lib structured::insights format`, `bash
  scripts/tests/capture-stream-json.test.sh`
- **Depends on**: — (main as is)
- **Tier**: full

### M2 — Structured sessions feed plan usage

- **Tasks**: T016–T022
- **Deliverable**: with **Show Claude plan usage** on, the app bar's plan-usage indicator updates
  from a structured session's `rate_limit_event`; structured Claude sessions run with
  `CLAUDE_CODE_FORWARD_SUBAGENT_TEXT=1`; the Settings note and both *Claude plan usage* guide
  sections say structured sessions are a source
- **Satisfies**: US4 scenarios 1–5; FR-010–FR-013, FR-027 (490 part), FR-028 (daemon feed);
  SC-004, SC-007 (plan-usage tests); research R5 (D8)
- **Verify**: `cargo test -p micold-core --test plan_usage_rate_limit_event`, `cargo test -p
  micold-daemon --test plan_usage_structured --test structured_session --test plan_usage_route
  --test plan_usage_setting --test status_relay`
- **Depends on**: M1
- **Tier**: full

### M3 — Task table and subagent thread index in core

- **Tasks**: T023–T028
- **Deliverable**: `SessionInsights` lists background tasks with their states and attributes every
  subagent event to its thread, placed under its parent at any depth, on the new recordings
- **Satisfies**: US3 scenarios 1, 3–6 and US5 scenarios 1–5 at the model; FR-014 (attribution),
  FR-015 (summary), FR-016, FR-017, FR-018, FR-020–FR-022; SC-003, SC-006 (task part)
- **Verify**: `cargo test -p micold-core --test structured_insights` and `cargo test -p
  micold-core --lib structured::insights`
- **Depends on**: M1 (#939 M1–M2, on main)
- **Tier**: full

### M4 — Meter parts and icons in the material library

- **Tasks**: T029–T033
- **Deliverable**: `material::ProgressBar` has `.marker(Ratio)` and `.warning(bool)` with their
  geometry and contrast gates, `Icon::Subagent` and `Icon::BackgroundTask` exist, and the showcase
  shows a marked and a warning bar
- **Satisfies**: FR-002, FR-003 (components); Constitution VIII
- **Verify**: `cargo test -p micold-client --lib ui::material::progress_bar` and `cargo test -p
  micold-client --test icons --test icons_font --test showcase_completeness --test
  material_builder_api --test layout_snapshot`; the showcase's surfaces section shows the bars
- **Depends on**: — (main as is)
- **Tier**: full

### M5 — The window's insights state

- **Tasks**: T034–T036
- **Deliverable**: each window keeps, per structured session, the insights, its open threads (as
  nested transcripts) and the background-list fold, fed from the wire after #939's transcript,
  with `InsightsMsg` handled; nothing draws it yet
- **Satisfies**: US3 scenarios 2, 3, 7 and US5 scenario 4 at the state; FR-014, FR-015 (fold),
  FR-019, FR-022 (overlay), FR-023
- **Verify**: `cargo test -p micold-client --test features_structured_insights --test
  features_are_render_free`
- **Depends on**: M3; **#939 M3**
- **Tier**: full

### M6 — Meter, figures, threads and background tasks in the pane

- **Tasks**: T037–T044
- **Deliverable**: a structured session's pane shows the session bar (meter, totals, running
  background tasks and their list), the turn's cost and tokens in each footer, subagent threads
  folded under their cards at any depth with placeholders for unknown parents, and each
  background task's state on its card; the user guide's *Structured sessions* describes them
- **Satisfies**: US1 scenarios 1–6, US2 scenarios 1–5, US3 scenarios 1–7, US5 scenarios 1–5 in
  the view; FR-001–FR-008, FR-014–FR-018, FR-020–FR-022, FR-026, FR-027; SC-005 (judged by the
  T044 visual-pass evidence)
- **Verify**: `cargo test -p micold-client --test layout_snapshot`; quickstart Part B steps 1–8
  (visual-pass evidence in `specs/944-session-usage-subagents/visual-pass/`)
- **Depends on**: M4, M5; **#939 M4, M5 and M6**
- **Tier**: full

Polish (T045–T046) changes no code: the close unit does it in the close PR.
