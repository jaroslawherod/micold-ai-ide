# Data model: 802

No persisted or wire entity changes.

## Client state (`features/session.rs`, `SessionState`)
| Field | Type | Meaning |
|---|---|---|
| `additional_target` | `Option<SessionLocation>` (exists) | location whose parallel-session confirmation is open |
| `additional_provider` | `Option<AiCli>` (new) | CLI the person chose for the start that raised it; `None` uses `provider_for_start(None)` |

Cleared with `additional_target` on confirm, cancel and `clear_for_dialog`.

## `PendingOp::CreateSession` (`shell/daemon_sync.rs`)
Unit variant becomes `CreateSession { location: SessionLocation, provider: AiCli, additional: bool }`.
`additional: true` for the confirmed create; `automatic: true` (from `StartRequested`) for the startup auto-start, whose refusal keeps the notice. Struct shape: `{ location, provider, additional, automatic }`. `additional: true` for the confirmed create, which never reopens the dialog on a refusal.

## Transitions
Start requested -> (occupied known) confirmation | (send) -> reply ok: created | reply Refused+id: confirmation.
Confirmation -> confirm: flagged create; cancel: nothing.
