# Contract: classification and when the service lets a window notify

**Feature**: 613 | Code: `micold_core::attention` (`TurnClock`, `NotificationKind`,
`LONG_TASK_THRESHOLD`), `crates/micold-daemon/src/{activity.rs, hooks.rs, attention.rs, state.rs}`.
Data model: [../data-model.md](../data-model.md).

## Turn and kind (FR-001 – FR-003)

| # | Rule |
|---|---|
| C1 | Each live session has one `TurnClock`, created `NotInTurn` with the live entry and dropped with it. |
| C2 | `note_activity` maps the event to a `TurnChange` (data-model, "Mapping") and calls `turn.change(change, clock::now(), LONG_TASK_THRESHOLD)` on every event, before deciding `began_waiting`. |
| C3 | When `began_waiting` holds (039), the attention event's kind is the kind `change` returned. Every path into `AwaitingInput` is a `Notification` or a `Stop`, both of which return a kind, so every event has one (FR-001). |
| C4 | Finished with `now − since ≥ threshold` is `LongTaskFinished`; `<` is `TurnFinished`. `since` is the turn's start, kept through pauses (FR-003). |
| C5 | A turn whose start was not seen starts at the first `Working` change the service saw (spec Edge Cases). |
| C6 | `LONG_TASK_THRESHOLD` is the only definition of 60 s. Nothing else compares a duration with a literal 60. |

## Helper agent stop (FR-024)

| # | Rule |
|---|---|
| C17 | `hooks.rs::classify_hook` returns `HookClass::Ignored` for `"SubagentStop"`: no `ActivityEvent`, no FSM transition, no `TurnChange`, no attention event. `settings_json` does not register `SubagentStop`. |

## Error endings (FR-004)

| # | Rule |
|---|---|
| C7 | `SupervisionAction::GiveUp` in the supervision tick is an error ending. |
| C8 | `ActivityEvent::Ended { error: true }` applied to a live session whose signal was not already `Ended` is an error ending. `copilot_event` sets `error: true` for `session.error` only. |
| C9 | `SupervisionAction::Stop` (clean exit), `Ended { error: false }`, a user stop or close, and an abnormal exit followed by `Restart` are not. |

## Awaiting-input kinds: note and claim (FR-005, FR-006, FR-013, FR-018)

| # | Rule |
|---|---|
| C10 | 039's step is unchanged: `mark_attention` raises `attention_seq` and sets `unread` whatever the kind. Then `Views::note_event(session, seq, kind, notify(kind))`: when `notify(kind)` is false the event is recorded as granted; when true its kind is kept as pending. |
| C11 | `Views::grant(session, seq, current, notify)` grants only when 039's rule holds, `seq` has a pending kind `k`, and `notify(k)` is true **now**; it answers `AttentionGranted { kind: k }`. A refused claim records nothing. |
| C12 | Pending kinds at or below a granted (or recorded-as-granted) sequence are dropped; `forget_session` drops all of the session's. |

## Session error notice (FR-005, FR-007)

| # | Rule |
|---|---|
| C13 | On an error ending, when `notify(SessionError)`, the session is not in view in any window (`Views::is_in_view`), and `Views::error_notice_target()` is `Some(window)`, the service sends `SessionErrorNotice { project, session }` to that window only. Otherwise nothing is sent and nothing is kept. |
| C14 | `error_notice_target`: the last entry of `focus_order` when there is one, else the lowest `ClientId` among connections with a stored `WindowView`, else `None`. |

## Switch changes (FR-012, FR-013)

| # | Rule |
|---|---|
| C15 | When `SettingsSet` turns the master switch on or any kind on, every current `attention_seq` is recorded as granted before the new value is stored (extends 039's `set_desktop_notifications`), so nothing that happened while off is notified afterwards. |
| C16 | Every `notify(kind)` reads the catalog's current settings; no copy is cached per session. |
