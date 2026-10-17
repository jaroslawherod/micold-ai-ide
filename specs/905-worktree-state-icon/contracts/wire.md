# Contract: `SessionSummary::paused_on_user` (protocol 48)

**Feature**: 905 | Research: [R1–R3](../research.md)

## W1 — The field

`SessionSummary` gains `paused_on_user: bool`, `#[serde(default)]`. `PROTOCOL_VERSION` 47 → 48,
with a doc line in `crates/micold-core/src/protocol/version.rs` naming this feature.

## W2 — Its value (service)

- `true` exactly while the session's live `TurnClock::paused_on_user()` is `true`.
- `false` for a session with no live entry, and in `catalog.rs` `session_summary` (the durable
  path never carries it).

## W3 — When it is sent

`State::note_activity` returns `true` (the caller broadcasts the catalog) when the activity signal
changed or `paused_on_user` changed. Hook sequence and expected values:

| Events | `activity` | `paused_on_user` | Broadcast at last event |
|---|---|---|---|
| `UserPromptSubmit`, `Notification` | `AwaitingInput` | `true` | yes (signal changed) |
| … then `PreToolUse` | `Working` | `false` | yes |
| … `Notification`, then `Stop` | `AwaitingInput` | `false` | **yes** (flag changed, signal not) |
| `UserPromptSubmit`, `Stop` | `AwaitingInput` | `false` | yes |
| `Notification` with no turn | `AwaitingInput` | `false` | yes (signal changed) |

## W4 — Client

`catalog_sync.rs` copies it into `Session::paused_on_user` wherever it copies `activity`.

## W5 — Unchanged

Attention events, `unread`, notification kinds and their counting (features 039, 575, 613) are
unchanged: `began_waiting` still compares signals only.
