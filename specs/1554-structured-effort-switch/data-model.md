# Data Model: Structured Session Reasoning Effort (#1554)

Names are Rust items; **new** marks an item that does not exist on `main`. Research references:
[research.md](research.md).

## Core: `micold_core::structured::effort` (**new** module)

| Item | Shape | Rules | FR |
|---|---|---|---|
| `EffortLevel` | newtype over `String`; serde `try_from`/`into` `String` | lowercase ASCII letters only, 1–16 bytes, not `default`, `auto` or `ultracode`; `EffortLevel::parse(&str)` lowercases first; checked again when read back from the store | FR-002, FR-007 |
| `EFFORT_LEVELS` | `[&str; 5]` = `low, medium, high, xhigh, max` | least to most; offered when none are reported | FR-002 |
| `EffortChoice` | `Default` \| `Level(EffortLevel)`; `#[default] Default` | `is_default()`; a store skips `Default` | FR-010, FR-011 |
| `EffortSupport` | `{ supported: bool, levels: Vec<EffortLevel> }` | from a reported model's `supportsEffort` / `supportedEffortLevels`; a name `EffortLevel::parse` refuses is dropped | FR-003 |
| `effort_support(models: &[ReportedModel], model: &ModelChoice) -> Option<&EffortSupport>` | | the model whose `value == model.cli_value()`; none listed, or listed without effort fields → `None` (unknown) | FR-002, FR-010a |
| `offered_levels(Option<&EffortSupport>) -> Vec<EffortLevel>` | | `supported` with ≥ 1 level → those levels in the CLI's order; otherwise `EFFORT_LEVELS` | FR-002, D4 |
| `effective_level<'a>(&'a EffortChoice, Option<&EffortSupport>) -> Option<&'a EffortLevel>` | | `Default` → `None`; `Level(l)` → `Some(l)` when support is `None` or `supported && levels ∋ l`; else `None` | FR-003, FR-010a, FR-011 |
| `EffortAvailability` | `Applies`, `ModelHasNoEffort`, `LevelNotForModel(EffortLevel)` | `availability(&EffortChoice, Option<&EffortSupport>)`: the view's words (R7) | FR-003 |
| `parse_effort_arg(arg: &str, offered: &[EffortLevel]) -> Result<EffortChoice, EffortArgError>` | | trimmed, any case; `default`/`auto` → `Default`; in `offered` → `Level`; else error | FR-007 |
| `EffortArgError` | `{ given: String, accepted: Vec<String> }`; `Display`: "Use /effort with one of: default, low, medium, high, xhigh, max" (the offered levels) | | FR-007 |
| `EffortRefusal` | `Refused { message }`, `NoAnswer` | built by `parse_apply_effort` | FR-005 |
| `launch_level` → `LaunchSpec.effort` | see *Session and store* | | FR-010 |

## Core: events, control, commands, transcript

| Item | Change | FR |
|---|---|---|
| `ReportedModel` (`event.rs`) | `+ effort: Option<EffortSupport>` (`#[serde(default)]`), filled by `commands::parse_initialize` from `supportsEffort` / `supportedEffortLevels`; either absent → `None` | FR-002, FR-003 |
| `StructuredEvent::EffortChanged` **new**, daemon note | `{ effort: EffortChoice }`: the choice from here on | FR-004, FR-006 |
| `StructuredEvent::EffortRefused` **new**, daemon note | `{ effort: EffortChoice, message: String }`: a deferred change the CLI refused | FR-005 |
| `SavedEvent` (`saved.rs`) | existing `CommandsReported` keeps its encoding through **new** `SavedReportedModelV1` (`value`, `display_name`, `description`); **new** variants appended after `KindSwitched`: `CommandsReportedV2 { commands, models: Vec<ReportedModel> }` (every new write), `EffortChanged`, `EffortRefused`. `EVENTS_FORMAT_VERSION` stays 1 | FR-010 |
| `control::encode_apply_effort(id, Option<&EffortLevel>)` **new** | `{"type":"control_request","request_id":…,"request":{"subtype":"apply_flag_settings","settings":{"effortLevel":"<level>"\|null}}}\n` | FR-004, FR-006 |
| `control::parse_apply_effort(&ControlOutcome) -> Result<(), EffortRefusal>` **new** | `Success(_)` → `Ok`; `Error { message, .. }` → `Refused { message }` | FR-005 |
| `commands::APP_COMMANDS` **new** | `[MODEL_COMMAND, EFFORT_COMMAND]`; `EFFORT_COMMAND = "effort"`; the kind rule and the fallback list use it, so `/effort` is one `App` entry with the app's description | FR-009 |
| `DraftCommand::Effort(Option<String>)` **new** | `classify_draft`: `/effort` + blank rest → `Effort(None)`, else `Effort(Some(rest trimmed))` | FR-007 |
| `EntryKind::EffortChanged` **new** (`transcript/entry.rs`) | `{ effort: EffortChoice, refused: Option<String> }` from the two notes (`transcript/build.rs`) | FR-004, FR-005 |

Both new events and the `ReportedModel` field are wire-visible (`StructuredEvents` frames):
`PROTOCOL_VERSION` +1 in M1.

## Core: session and store

| Item | Change | FR |
|---|---|---|
| `Session.effort` (`session.rs`) | `EffortChoice`, persisted; set only through `Catalog::set_effort`; `Default` for a new session | FR-010, FR-011 |
| `StoredSession.effort` (`store.rs`) | `#[serde(default, skip_serializing_if = "EffortChoice::is_default")]`; no `schema_version` bump; a legacy record reads `Default`; a stored name `EffortLevel` refuses reads `Default` | FR-010 |
| `LaunchSpec.effort` (`terminal.rs`) | `Option<EffortLevel>`: the **effective** level at this launch (R3) | FR-010, FR-010a |
| `AiCliProvider::effort_args(&self, Option<&EffortLevel>) -> Vec<String>` **new** (`provider.rs`) | default impl: none; `ClaudeProvider`: `["--effort", level]` for `Some` | FR-010, FR-012 |
| `terminal::launch_args` | appends `effort_args(spec.effort.as_ref())` after `model_args` | FR-010 |

## Wire (`protocol/messages.rs`)

See [contracts/wire.md](contracts/wire.md): `SessionSummary.effort`, `SetSessionEffort`.

## Daemon state

| Item | Shape | Rules | FR |
|---|---|---|---|
| `EffortPicks` **new** | per session `{ latest: u64, kept: u64, deferred: Option<DeferredEffort> }` under the state lock | as `ModelPicks`: a pick kept only if no later pick was kept first | Edge Cases (two windows) |
| `DeferredEffort` **new** | `{ chosen: EffortChoice, previous: EffortChoice }` | refused → put back to `previous` unless another pick since | FR-005 |
| `StructuredSession.deferred_effort` **new** (`structured.rs`) | `Mutex<Option<Option<EffortLevel>>>` (outer: something to write; inner: the level or null) | written by the turn-end hook after the deferred model; replaced by each later pick | FR-004 |
| `StructuredSession::ask_effort` / `defer_effort` / `effort_answer` **new** | mirror `ask_model` / `defer_model` / `model_answer` over `send_control` | | FR-004, FR-005 |
| `EffortAsk` **new** | `{ choice: EffortChoice, deferred: bool }` by request id | its answer becomes `EffortChanged`, or `EffortRefused` when deferred and refused (as `model_note`) | FR-004, FR-005 |
| `SpawnPlan.effort` | `Option<EffortLevel>`: `effective_level(record.effort, effort_support(latest CommandsReported models, record.model))` | R3 | FR-010a |

## Client (render-free)

| Item | Where | Holds | FR |
|---|---|---|---|
| `EffortPickerState` **new** | `features/effort_picker.rs` | `open`, `highlight`, rows: Default + `offered_levels`, the current choice, `EffortAvailability` | FR-001–FR-003 |
| `effort_label(&EffortChoice) -> String` | same | `Effort: Default` / `Effort: <level>` | FR-001 |
| `availability_note(&EffortAvailability) -> Option<String>` | same | the words of R7 | FR-003 |
| `SendIntent::SetEffort(EffortChoice)` **new** | `features/command_palette.rs`, beside `SetModel` | `/effort <value>` resolved by `parse_effort_arg`; `/effort` alone opens the effort picker and a wrong value shows `EffortArgError`, both `SendIntent::Nothing` with the draft kept | FR-007 |
| `set_effort` | `shell/composer.rs` beside `set_model` | sends `ClientMsg::SetSessionEffort`, shows an `OperationError` text | FR-005 |

## State transitions: one pick

```text
                         ┌─ not Claude ───────────────► refused (nothing stored)
pick(session, choice) ───┼─ terminal kind ────────────► stored; "applies at the next launch"
                         ├─ not running ──────────────► stored + EffortChanged; OK
                         ├─ running, idle ─ ask ──┬─ success ─► stored + broadcast + EffortChanged; OK
                         │                        └─ error/NoAnswer/Stopped ─► unchanged; error text
                         └─ running, busy ─ stored + broadcast; OK; at turn end write ─┬─ success ─► EffortChanged
                                                                                      └─ error ───► EffortRefused; put back
```
