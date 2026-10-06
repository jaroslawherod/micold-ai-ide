# Data model: Notification kinds

**Feature**: 613 | Extends 039's data model (`specs/039-session-attention-notifications/data-model.md`).

## NotificationKind (new, `micold_core::attention`)

Closed enum, `Copy`, serde as snake-case strings on the wire.

| Variant | `name()` | `description()` (Settings note) | Default | Icon |
|---|---|---|---|---|
| `NeedsPermission` | Needs permission | A session stopped to ask for a permission or an answer. | on | `Icon::NeedsPermission` |
| `SessionError` | Session error | A session stopped because of an error. | on | `Icon::SessionError` |
| `LongTaskFinished` | Long task finished | A session finished a turn that took a minute or more. | on | `Icon::LongTaskFinished` |
| `TurnFinished` | Turn finished | A session finished a shorter turn. | off | `Icon::TurnFinished` |

- `NotificationKind::ALL: [NotificationKind; 4]` in the order above (FR-009).
- `default_on(self) -> bool`.
- The description of **Long task finished** is derived from `LONG_TASK_THRESHOLD` (one minute), not
  a second literal.
- The `Icon` mapping lives in the client (`notification_icon::icon(kind)`), since `Icon` is a client
  type.

## NotificationKinds (new, `micold_core::settings` via `attention`)

```rust
pub struct NotificationKinds {
    pub needs_permission: bool,   // serde default true
    pub session_error: bool,      // serde default true
    pub long_task_finished: bool, // serde default true
    pub turn_finished: bool,      // serde default false
}
```

- `Default` = the defaults above; `is_on(kind) -> bool`; `set(kind, on)`.
- Field of `Settings` as `notification_kinds`, `#[serde(default)]` on the field, so a pre-feature
  file keeps `desktop_notifications` and gets these defaults (FR-010). An unreadable file uses
  `Settings::default()`.
- Validation: none beyond serde; a non-boolean value fails that field's parse the way other
  settings fields do (settings load falls back per its existing rules).

## Effective switch

`notify(kind) = settings.desktop_notifications && settings.notification_kinds.is_on(kind)` — the
one predicate the service evaluates at note and at claim (contract C10–C13).

## TurnClock (new, `micold_core::attention`)

Per live session, in memory, owned by the service's `LiveSession` (`turn` field). Reset on service
restart, as activity is (039 H3).

```rust
pub enum TurnChange { PromptSubmitted, Working, AskedUser, Finished }
pub struct TurnClock { state: TurnState }
enum TurnState { NotInTurn, Working { since: Uptime }, Paused { since: Uptime } }
impl TurnClock {
    pub fn change(&mut self, change: TurnChange, now: Uptime, threshold: Duration)
        -> Option<NotificationKind>;
}
pub const LONG_TASK_THRESHOLD: Duration = Duration::from_secs(60);
```

State transitions (returned kind in brackets; `—` = none):

| From \ change | PromptSubmitted | Working | AskedUser | Finished |
|---|---|---|---|---|
| NotInTurn | Working{now} — | Working{now} — | NotInTurn [TurnFinished] | NotInTurn [TurnFinished] |
| Working{s} | Working{now} — | Working{s} — | Paused{s} [NeedsPermission] | NotInTurn [LongTaskFinished if now−s ≥ threshold, else TurnFinished] |
| Paused{s} | Working{now} — | Working{s} — | Paused{s} — | NotInTurn [LongTaskFinished if now−s ≥ threshold, else TurnFinished] |

The returned kind is used only when the service counts an attention event (039 `began_waiting`:
the signal changed into `AwaitingInput`); otherwise it is discarded. `Paused + Finished` (the user
denied and the turn ended) therefore notifies nothing, because the signal stays `AwaitingInput`, but
the clock still ends the turn.

Mapping in the service (`activity.rs`): `Hook(UserPromptSubmit)` → `PromptSubmitted`;
`Hook(PreToolUse)`, and `SpinnerObserved` when it lifted the signal → `Working`;
`Hook(Notification)` → `AskedUser`; `Hook(Stop)` → `Finished` (Claude's `SubagentStop` is
`HookClass::Ignored` from this feature on and never reaches the FSM or the clock, FR-024, research
R3); `Hook(PostToolUse)`,
`ReadyForInput`, `Ended` → no change.

## Attention event (039, extended)

Gains `kind ∈ {NeedsPermission, LongTaskFinished, TurnFinished}`. `attention_seq` and `unread` are
set exactly as in 039, whatever the kind and the switches (FR-018).

## Pending kinds (new, in `Views`, daemon, in memory)

`pending: HashMap<SessionId, Vec<(u64, NotificationKind)>>` — the kind of each event noted while its
kind and the master switch were on and not yet granted. An entry leaves when its sequence (or a
higher one) is granted or recorded as granted, and when the session is forgotten. A claim whose
sequence has no pending kind is not granted (it was noted while off, or the service restarted).

## Error ending (new)

One session ending by the crash-loop give-up or by `ActivityEvent::Ended { error: true }`. Kind
`SessionError`. Not persisted, not sequenced, no unread change. `ActivityEvent::Ended` gains
`error: bool` (daemon-internal; `ActivitySignal::Ended` on the wire is unchanged).

## DesktopNotification (039, extended, client)

Gains `kind: NotificationKind`. The backend maps the kind to its icon (contract
[notification.md](./contracts/notification.md) I5–I7).
