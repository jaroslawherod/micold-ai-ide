# Data Model: Bottom Status Bar

**Feature**: [spec.md](spec.md) | **Research**: [research.md](research.md)

No persisted state, no protocol change. Everything is derived on each render from state that
already exists (`State`, `ConnectionStatus`, `features::sandbox::Sandbox`, `PlanUsage`).

## ItemTone (NEW, `ui/material/status_bar.rs`)

`enum ItemTone { Neutral, Positive, Warning, Error }` — maps to roles `on_surface_variant`,
`success`, `warning`, `error`. Used for a dot's colour and for an item's text colour (text uses
`Neutral` unless the item is a warning word).

## GiveWay (NEW, `ui/material/status_bar.rs`)

`enum GiveWay { Keep, Drop(u8), Shorten(u8) }` — how an item yields when the bar is too narrow
(research R4). All `Drop` items go before any `Shorten` item; within a kind, lower rank first.

| Item | GiveWay |
|---|---|
| Key hints (each) | `Drop(0)` |
| Plan usage | `Drop(1)` |
| CLI | `Shorten(0)` |
| Sandbox | `Shorten(1)` |
| Connection | `Shorten(2)` |

## StatusModel (NEW, `ui/status_bar.rs`, render-free)

```text
StatusModel {
  connection: ItemModel,            // always present (FR-004)
  sandbox:    ItemModel,            // always present (FR-005)
  cli:        Option<ItemModel>,    // Some only while the session view is visible (FR-007)
  usage:      Option<UsageView>,    // Some only with a current reading and the switch on (FR-011)
  hints:      [String; 2],          // "<chord> Overview", "<chord> Jump" (FR-008)
}
ItemModel { text: String, dot: Option<ItemTone>, tone: ItemTone, busy: bool, tooltip: Option<String> }
```

`UsageView` is the existing `plan_usage` view (label, warning, details).

### Connection item (from `ConnectionStatus`)

| ConnectionStatus | text | dot | tooltip | banner (unchanged unless noted) |
|---|---|---|---|---|
| `Connected` | `Daemon connected · {location}` | Positive | — | none |
| `Disconnected` | `Daemon disconnected · reconnecting…` | Error | "The displayed content may be stale." | **none (was a banner)** |
| `Displaced { by }` | `Read-only · another window` | Warning | banner sentence | Take over |
| `ProjectBusy { holder }` | `Read-only · another window` | Warning | banner sentence | Take over |
| `VersionMismatch { .. }` | `Daemon connected · {location}` | Warning | banner sentence | Restart service |
| `BuildMismatch { .. }` | `Daemon connected · {location}` | Warning | banner sentence | Restart service |

`{location}` (research R6): "local" (Host daemon, or no project open) · the daemon's name (other
runtimes) · "no daemon" (`Binding::NoDaemon`, e.g. `Daemon connected · no daemon`).

### Sandbox item (from `Sandbox`)

Checked in this order (a taken fallback wins, as in `sandbox_banner`):

| Condition | text | tone | busy | tooltip | banner |
|---|---|---|---|---|---|
| `fallback.is_some()` | `No sandbox` | Warning | no | `persistent_notice()` | Try the sandbox again |
| `Running(_)` | `Sandbox ready` | Neutral | no | — | none |
| `Disabled` | `Sandbox off` | Neutral | no | — | none |
| `Probing` | `Checking runtime…` | Neutral | yes | `attempt_line()` | none (row removed) |
| `Acquiring(p)` | `Getting image…` (+ ` {p.percent}%`) | Neutral | yes | `attempt_line()` | none (row removed) |
| `Starting` | `Starting sandbox…` | Neutral | yes | `attempt_line()` | none (row removed) |
| `Stale(_)` | `Sandbox out of date` | Warning | no | `persistent_notice()` | Restart the sandbox |
| `Failed(_)` | `Sandbox failed` | Warning | no | `persistent_notice()` | Run without it for now |

Tooltip from `attempt_line()` = `label` then, on a new line, `detail` when present (runtime line and
"Trying again: {reason}").

### CLI item

`CLI: {AiCli::provider().display_name()}` for the session `state.session.active` names, only when
`session_view_visible(state)` (research R8). Tone Neutral, no dot.

## Transitions

None stored. Each render recomputes the model; a state change (connection event, sandbox progress,
pane focus, Overview toggle, plan-usage reading) reaches the bar in the same render that applies it.
The busy ring requests frames only while `busy` is true (idle frame budget, SC-017 of feature 027).
