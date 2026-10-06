# Contract: notification text, kind icons, backends, Settings rows

**Feature**: 613 | Code: `micold_core::attention::notification_text`, `crates/micold-client/src/{icons.rs,
notification_icon.rs, features/attention.rs, features/settings.rs, app.rs, shell/daemon_sync.rs,
shell/desktop_notify/*, ui/material/checkbox.rs, ui/settings/environment.rs,
showcase/sections/controls.rs}`. Extends 039's [desktop-notification.md](../../039-session-attention-notifications/contracts/desktop-notification.md).

## Text (FR-008)

| # | Rule |
|---|---|
| T1 | `notification_text(kind, project, worktree, session)`: titles `<session> needs permission`, `<session> stopped with an error`, `<session> finished a long task`, `<session> finished its turn`; body `<project> — <worktree>`. |
| T2 | No other text: no CLI output, no error reason, no permission text (039 N2). |
| T3 | `SessionErrorNotice` builds its notification through the same `attention_notification(session, kind)` path as a grant, with `SessionError`; a session the window cannot name (unknown project or session) shows nothing. |

## Icons (FR-015 – FR-017, FR-022)

| # | Rule |
|---|---|
| I1 | `Icon::{NeedsPermission, SessionError, LongTaskFinished, TurnFinished}` are added to `Icon::ALL` with codepoints of Material Symbols `pan_tool`, `error`, `task_alt`, `chat_bubble`, verified present in the bundled font by `tests/icons_font.rs`. |
| I2 | `notification_icon::icon(kind) -> Icon` is the only kind→icon mapping; Settings and the backends both use it. |
| I3 | `notification_icon::render(kind, px) -> Rgba` draws the kind's glyph in white, centred, on a rounded-square tile in the kind's tile colour. Each tile colour has relative luminance in [0.10, 0.30] (≥3:1 against white and black; white glyph ≥3:1 on it). |
| I4 | Distinctness gate: the four glyph masks rendered in one colour at 16×16 differ pairwise in at least 10% of their pixels. |
| I5 | Linux: the request carries hint `image-data` `(iiibiiay)` = width, height, rowstride, has-alpha `true`, 8, 4, RGBA bytes of `render(kind, 64)`. |
| I6 | Windows and macOS: once per run the shell writes `render(kind, 256)` as PNG to `<data dir>/notification-icons/<kind>.png` (overwriting); Windows calls `Toast::icon(path, IconCrop::Square, <kind name>)`, macOS `Notification::image_path(path)`. If a file cannot be written, the notification is shown without it (FR-016), and the failure is logged once per run like 039 N4. |
| I7 | The backends' pure request builders take the kind and are tested for the icon they carry; the system call stays glue. |
| I8 | The showcase's Checkbox section shows the four kind rows (icon, name, note), checked and unchecked, enabled and disabled, in light and dark. |

## Settings rows (FR-009, FR-012, FR-014)

| # | Rule |
|---|---|
| S1 | Below **Desktop notifications**, one row per `NotificationKind::ALL`: `Checkbox::new(kind.name(), on, roles).icon(icon(kind))` inside `field_note(…, Some(kind.description()))`, indented one spacing step. |
| S2 | While the draft's `desktop_notifications` is false, the rows get no `on_toggle` (disabled) and show their stored values. |
| S3 | Toggling sends `SettingsMsg::NotificationKindToggled(kind, on)`, which edits the draft; saving sends `SettingsSet { notification_kinds: Some(draft) }`. |
| S4 | `Checkbox::icon(Icon)` is a chainable `self`-consuming builder method; the glyph is drawn in the label's colour role, so it inherits the existing light/dark contrast gate. |
