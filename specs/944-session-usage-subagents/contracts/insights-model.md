# Contract: the session insights model (`micold_core::structured::insights`)

The core surface #944's client uses. Types: [../data-model.md](../data-model.md). Decisions:
[../research.md](../research.md).

## Depends on (#939, on main since its M1–M2)

| #939 item | Used for |
|---|---|
| `Transcript::subagent_events(id)` | the events of an open thread (R7) |
| `Transcript::nested(id)` + `apply` | a thread's entries, with the main transcript's rules and cards |
| `Transcript::entry(&EntryKey::Tool(id))` | `owner(parent)`: is the parent card held in the main transcript |
| `Transcript::rebuilds()`, `omitted_notice()` | when to rebuild open threads |
| `ToolCard.status` (`ToolStatus`) | subagent state when no task event names the call |
| `EntryKind::Footer`, `EntryKey::At { seq, .. }` | the footer of the `Result` at `seq` ↔ `SessionInsights::turn(seq)` |
| `EntryKind::SubagentNotice` | the place the `Unknown` threads' placeholders are drawn |
| `format_cost` | turn and total cost text |
| `Transcript::apply`'s replace-or-continue rule | extracted to `structured::sync::Continuity`, used by both |

## Input

`SessionInsights::apply(omitted: u64, events: &[SequencedEvent], live: bool)`: the same message,
the same replace-or-continue rule and the same `MAX_EVENTS` bound as `Transcript::apply`.

## Event → effect rules

| Event | Effect |
|---|---|
| `Assistant`, no parent, `raw.message.usage` readable | context `used` and `model` (R2) |
| `Assistant`/`User` with `parent_tool_use_id` P | thread P: `+1` message; each `ToolUse` block → `tool_calls`, `latest_tool`, `tool_owner[id] = P` (R7) |
| `Result` | a `TurnUsage` at its `seq` (R3); its `modelUsage` windows update `context.window` for the current model |
| `TurnEndedWithoutResult` | a left-out turn; running foreground tasks → `EndedWithoutReport` |
| `Init` that starts the CLI process (it carries `startup_timing`; the `init` of each later turn of the same process does not) | running tasks → `EndedWithoutReport` (FR-022) |
| `Unrecognised` `autocompact_state` | `context.threshold` (`value.threshold` when `value.enabled`) |
| `Unrecognised` `system` `task_started` / `task_updated` / `task_progress` / `task_notification` / `background_tasks_changed` | the task table (R6) |
| `Unrecognised` `rate_limit_event` | nothing here: the daemon reads it ([plan-usage-source.md](plan-usage-source.md)) |
| anything else, or a field of the wrong kind | nothing; never an error (FR-024) |

## Guarantees (tested in `crates/micold-core/tests/structured_insights.rs`)

- **I1 Fresh-build equality (FR-023)**: for every fixture, applying it one event per message,
  in chunks, and as one history message, with and without a bound small enough to evict, yields
  the value a single history message of the held events yields.
- **I2 Figures (SC-001)**: for every recorded fixture, `context()` equals the last main
  response's usage sum and its model's `contextWindow`; each `turn(seq)` token figure equals its
  `Result`'s `usage`.
- **I3 Totals (SC-002)**: for 1–50 turns (fixture turns repeated with rising cumulative cost), a
  restart (cost drops) and a resume (`resume-init` after `text-turn`), totals equal the sum of the
  counted turns, and no turn's cost is counted twice.
- **I4 Attribution (SC-003)**: in every subagent fixture, every parented message is counted in its
  parent's thread and none elsewhere; `owner` places each thread under the card holding its parent,
  at every depth; a parent not held → `Unknown`.
- **I5 Tasks (FR-020–FR-022)**: `background-command` shows the task `Running` then `Finished` with
  its summary; a process-start `Init` and a live set without it end it; a notification for an unknown task makes
  an entry.
- **I6 Tolerance (SC-006)**: `insights-tolerance.jsonl` applies without panic and yields absent
  figures where fields are malformed.

## Client seam (#944's own)

`features::structured_insights` ([../data-model.md](../data-model.md) *View state*) is applied
after #939's `structured_transcript` for the same message, and reads the main `Transcript` it
holds. It never changes #939's state.
