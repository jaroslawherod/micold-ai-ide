# Data Model: Session Usage, Context Meter, Subagents and Background Tasks (#944)

Core types are in the new render-free module `micold_core::structured::insights` (research R1)
unless marked otherwise. Nothing here is persisted or sent on the wire: #938's `StructuredEvent`,
`SequencedEvent` and `StructuredEvents` message are reused unchanged, and feature 490's
`UsageReading` gains a producer, not a field.

## SessionInsights (core; one per window and structured session)

| Field | Meaning |
|---|---|
| `sync: Continuity` | the replace-or-continue state shared with `Transcript` (`next_seq`, last message live, holds anything; research R1) |
| `omitted: u64` | events before the first held one (the service's `omitted` plus those this value dropped) |
| `held: usize`, `first_held: u64` | how many events are held and the first held `seq`; past `MAX_EVENTS` the oldest leave and `omitted` grows, as in `Transcript` (seqs are contiguous, #938 wire) |
| `digests: VecDeque<(u64, Digest)>` | the events that matter here, by `seq` (research R8) |
| `usage: UsageState` | turns and totals (below) |
| `context: ContextReading` | below |
| `tasks: TaskTable` | below |
| `threads: ThreadIndex` | below |

Operations: `apply(omitted, &[SequencedEvent], live)`; `context() -> &ContextReading`;
`turn(result_seq) -> Option<&TurnUsage>`; `totals() -> &SessionTotals`; `tasks()`,
`background_tasks()`, `running_background() -> usize`, `task_for_tool(tool_use_id)`;
`thread(parent) -> Option<&ThreadSummary>`, `threads_owned_by(Owner)`, `owner(parent, &Transcript)
-> Owner`, `unknown_threads(&Transcript)`.

Invariant **I1** (research R8): after any sequence of messages, equal to a value built by one
`live: false` message of the held events with the same `omitted`.

### Digest (private)

`AssistantUsage { model, used }` (main scope only) · `Result { tokens, cost_cum, models_cum }` ·
`TurnEndedWithoutResult` · `Init` · `Autocompact { threshold: Option<u64> }` · `Task(TaskEvent)` ·
`LiveSet(Vec<LiveTask>)` · `Parented { parent, tool_uses: Vec<(id, name)> }`. Unreadable or
malformed fields leave the figure `None`; a line of another shape is no digest (FR-024).

## Turn usage

### Tokens

`{ input: Option<u64>, output: Option<u64>, cache_read: Option<u64>, cache_creation: Option<u64> }`
from the `Result`'s typed `Usage`. "No token figures" = all four `None`.

### ModelFigures

`{ input, output, cache_read, cache_creation: u64, cost_usd: f64, context_window: Option<u64> }`
from one `modelUsage` entry; an entry missing a number is skipped.

### TurnUsage (per held `Result`, keyed by its `seq`)

| Field | Meaning |
|---|---|
| `result_seq` | the `Result` event's `seq` — #939's footer entry is `EntryKey::At { seq, block: 0 }` with this `seq` |
| `tokens: Option<Tokens>` | as reported (FR-006) |
| `cost_usd: Option<f64>` | FR-009's difference; `None` when the previous result is not held and `omitted > 0`, or the result has no `total_cost_usd` |
| `models: Option<Vec<(String, ModelFigures)>>` | FR-009's per-model differences, same rule |
| `restarted: bool` | a cumulative figure fell: figures taken as they are |
| `counted: bool` | `tokens.is_some() && cost_usd.is_some()` |

### SessionTotals

`{ cost_usd: f64, tokens: Tokens (sums, each Some when any counted turn reports it), models:
Vec<(String, ModelFigures)>, turns_counted: u64, turns_left_out: u64, partial: bool }`.
`turns_left_out` counts held `Result`s not counted plus held `TurnEndedWithoutResult` notes.
`partial = omitted > 0` (FR-008).

## ContextReading

`{ used: Option<u64>, model: Option<String>, window: Option<u64>, threshold: Option<u64> }`
(research R2). Derived view state:

| Function | Rule |
|---|---|
| `state() -> ContextState` | `NoData` (no main response held) · `TokensOnly { used }` (window unknown) · `Measured { used, window, percent, threshold: Option<u64>, warning: bool }` |
| `percent` | `used × 100 / window`, whole, rounded half up |
| `warning` | `used ≥ threshold`, else `used × 10 ≥ window × 9` (FR-003) |

## TaskTable

`HashMap<String, Task>` plus insertion order.

| Field | Meaning |
|---|---|
| `task_id` | key |
| `tool_use_id: Option<String>` | the call that started it (FR-021) |
| `description: String` | from `task_started`/`task_updated`, else the report's `summary`, else the live-set entry's |
| `kind: Option<TaskKind>` | `Command` (`local_bash`) · `Subagent` (`local_agent`) · `None` otherwise |
| `background: bool` | `is_backgrounded`, a later `patch.is_backgrounded`, or known only from a report or live set |
| `state: TaskState` | `Running` · `Finished` · `Failed` · `Stopped` · `EndedWithoutReport` |
| `outcome: Option<String>` | the notification's `summary` |
| `activity: Option<String>` | latest `task_progress` summary / description / last tool name |

Transitions (research R6): `Running →(task_updated completed | notification completed) Finished`;
`→(failed) Failed`; `→(killed | stopped) Stopped`; `Running →(Init, or absent from a later live set
while background, or TurnEndedWithoutResult while foreground) EndedWithoutReport`. A terminal state
changes only by a later `task_started` of the same `task_id` (a resumed run) back to `Running`.
View overlay: `display_state(cli_running: bool)` maps `Running` to `EndedWithoutReport` when the
CLI is not running (FR-022).

## ThreadIndex (subagents)

| Item | Meaning |
|---|---|
| `threads: HashMap<String, ThreadSummary>` | by parent tool use id |
| `tool_owner: HashMap<String, String>` | tool use id → the thread whose message holds it |

`ThreadSummary { parent, first_seq, messages: u64, tool_calls: u64, latest_tool: Option<String> }`.

`Owner` = `Main` · `Thread(String)` · `Unknown` (research R7). `owner(parent, &main_transcript)`:
`Main` when `main.entry(&EntryKey::Tool(parent))` exists, else `Thread(q)` when
`tool_owner[parent] = q`, else `Unknown`.

`ThreadView { summary: &ThreadSummary, state: SubagentState, activity: Option<String> }` with
`SubagentState` = `Running` · `Finished` · `Failed` · `Stopped` · `EndedWithoutResult`, from the
`Subagent` task of `tool_use_id == parent`, else from the parent card's `ToolStatus` (passed in).

## Plan-usage source (core, `micold_core::plan_usage`)

| Function | Rule |
|---|---|
| `rate_limit_event::parse(raw: &str, obtained_at: i64) -> Option<UsageReading>` | research R4; `None` for a line that is not a `rate_limit_event` or has no readable window |
| `limit_window(key, used_percent: f64, resets_at: Option<&Value>, obtained_at) -> Option<LimitWindow>` | the per-window rule `status_line::window` applies today, shared |

## Formatting (core)

`micold_core::format::group_digits(u64) -> String` (moved from `features/worktree_form.rs`;
research R9); `insights::figures`: `turn_figures(&TurnUsage) -> Vec<String>` (cost with "est.",
"in N · out N"), `turn_details(&TurnUsage) -> Vec<String>` (cached input, one line per model,
the estimate note), `totals_line(&SessionTotals)`, `totals_details(&SessionTotals)`,
`context_label(&ContextState)`, `context_details(&ContextState)`, `task_label(&Task, cli_running)`,
`thread_summary_line(&ThreadView)`. Absent figures yield no string (FR-004, FR-008).

## View state (client, `features/structured_insights.rs`; per window)

| Item | Meaning |
|---|---|
| `State.sessions: HashMap<SessionId, SessionExtras>` | one per structured session the window has heard of; removed by `forget` |
| `SessionExtras.insights: SessionInsights` | R1 |
| `SessionExtras.open_threads: HashMap<String, Transcript>` | nested transcripts of open threads only (research R7); absent = folded (FR-015, FR-019) |
| `SessionExtras.tasks_open: bool` | the background-task list unfolded |
| `SessionExtras.seen: (u64 rebuilds, Option<u64> omitted)` | of the main transcript at the last apply, to rebuild open threads after a main rebuild or eviction |

Operations (render-free, tested in `tests/features_structured_insights.rs`): `apply(session,
omitted, &events, live, &main_transcript)`; `toggle_thread(session, parent, &main_transcript)`;
`thread(session, parent) -> Option<&Transcript>`; `toggle_tasks(session)`; `forget(session)`.

## Messages (client)

`InsightsMsg { ToggleThread { session, parent: String }, ToggleBackgroundTasks { session } }`,
carried by the session message family beside #939's `TranscriptMsg`.

## Daemon

| Item | Meaning |
|---|---|
| `DaemonState::offer_plan_usage(session, reading) -> bool` | switch check, `note_plan_usage`, `PlanUsageChanged` broadcast; shared by `hooks.rs` and the structured feed (research R4) |
| plan-usage feed | `UnboundedSender<(SessionId, UsageReading)>` held by `DaemonState`, drained by a task with `Arc<DaemonState>` |
| `AiCliProvider::structured_env() -> &'static [(&str, &str)]` | default `&[]`; Claude: `[("CLAUDE_CODE_FORWARD_SUBAGENT_TEXT", "1")]` (research R5) |
