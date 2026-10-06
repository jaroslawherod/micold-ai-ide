# Quickstart: validating notification kinds

**Feature**: 613 | Contracts: [wire](./contracts/wire.md), [classification](./contracts/classification.md),
[notification](./contracts/notification.md). Data model: [data-model.md](./data-model.md).

## Part A — automated

Run CI's gate (`mise run gate`). The feature's own checks:

| Command | Proves |
|---|---|
| `mise run test-core` | `TurnClock` (C1–C6), kinds and defaults, settings load cases (FR-010), `notification_text` (T1), wire round-trip and schema hash |
| `cargo test -p micold-daemon --lib attention activity` | kind-aware note/grant, pending pruning, error-notice target, Copilot error flag, `SubagentStop` ignored and unregistered (FR-024) |
| `cargo test -p micold-daemon --test attention_events --test attention_claims --test settings_desktop_notifications` | per-kind notification over a real connection: SC-001 sequence, one-kind-on trials (SC-002), unread unchanged (SC-003), switches across a service restart (SC-006), Session error on give-up and on Copilot `session.error`, none with no window |
| `cargo test -p micold-client --test icons_font --test notification_icon` | codepoints in the font (I1), tile contrast and distinctness (I3, I4) |
| `cargo test -p micold-client` (backend request tests) | Linux `image-data`, Windows image entry, macOS image path (I5–I7) |

Expected: all green. SC-007 (container) is covered by `mise run test-sandbox` against `mise run image`
for the daemon integration tests above.

## Part B — visual pass (needs a display)

Prerequisites: a desktop session with a notification service; two Claude Code sessions A and B in
one project; Settings at defaults.

1. With A in view, have B run a turn of ~5 s → no notification; B shows the unread mark.
2. Have B run a turn of ≥ 70 s (`sleep 70` via a tool) → one **Long task finished** notification with
   the circled-tick icon, title `<B> finished a long task`.
3. Have B ask for a permission mid-turn → one **Needs permission** notification, raised-hand icon.
4. Kill B's CLI repeatedly until the service gives up → one **Session error** notification, error
   icon; B is not marked unread by it.
5. Open Settings → under **Desktop notifications** four rows in the order Needs permission, Session
   error, Long task finished, Turn finished, each with its icon and note; the first three checked.
   Turn **Desktop notifications** off → the four rows grey out and keep their marks.
6. Turn **Turn finished** on, **Long task finished** off; repeat steps 1–2 → one notification for the
   short turn, none for the long one. Restart the app → the rows are as left.
7. Switch the desktop between light and dark theme and repeat 2–4 → each icon is recognisable on
   both. Compare each with its Settings row icon.
8. Open the component showcase → Checkbox section shows the kind rows in both themes, enabled and
   disabled.

Repeat 2–4 on macOS and Windows (icon may be absent where the OS does not show app images; the
title must still name the kind).
