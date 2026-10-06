# Implementation Plan: Notify Only When a Session Needs Attention, With Per-Kind Settings and Icons

**Branch**: `claude/project-thread-8kdqkn` | **Date**: 2026-10-06 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/613-notification-kinds/spec.md`

## Summary

Feature 039 raises one desktop notification for every change of a session into awaiting input. This
feature gives each such change, and each error ending, one of four **notification kinds**, decided
by the session service at the moment it happens, and lets a per-kind switch decide whether it is
notified. Out of the box only **Needs permission**, **Session error** and **Long task finished** are
on.

- **Classification** (service). A render-free `TurnClock` in `micold_core::attention` follows each
  live session's turn from the activity events the service already receives (Claude Code hooks,
  Copilot's event log, Pi's activity log): a `Notification` while the session works is **Needs
  permission**; a `Stop` is **Long task finished** when the turn lasted at least
  `LONG_TASK_THRESHOLD` (60 s, one named constant, research R2) and **Turn finished** otherwise. The
  kind travels with 039's grant (`AttentionGranted { kind }`).
- **Error endings** (service). A crash-loop give-up (where every unrecovered abnormal exit ends,
  a failed respawn included) and a CLI-reported error (Copilot `session.error`) are **Session
  error**. The service sends one `SessionErrorNotice` to one
  window, chosen as 039 picks a reveal target, when the kind is on, the session is not in view and
  a window is connected (research R4). Unread state is untouched.
- **Settings** (service-owned). `Settings::notification_kinds`, one boolean per kind with its own
  serde default, beside 039's `desktop_notifications`, which stays the master switch. Applied where
  039 applies the master switch: when the event is noted and when it is claimed.
- **Text and icons** (client). The title states the kind (`notification_text(kind, …)`). Four new
  `Icon` variants from the bundled Material Symbols font. The client rasterises each one from that
  same font as a white glyph on a tile whose colour holds 3:1 against white and black, and hands it
  to each OS backend: `image-data` on Linux, a PNG file on Windows (`appLogoOverride`) and macOS
  (attachment). Settings shows each kind as the shared `Checkbox` with a new `.icon(…)` builder
  method, under the master switch, disabled while the master switch is off.

## Technical Context

**Language/Version**: Rust, stable toolchain (workspace `rust-toolchain.toml`), edition as the workspace

**Primary Dependencies**: iced (GUI); `serde` (settings, wire); existing backends `zbus` (Linux),
`tauri-winrt-notification` 0.8.1 (Windows, `Toast::icon`), `mac-usernotifications` 0.3.1 (macOS,
`Notification::image_path`); `ttf-parser` (moves from dev- to normal dependency of `micold-client`)
and `tiny-skia` 0.11 with its `png-format` feature (already in `Cargo.lock` through
`iced_tiny_skia`; the feature adds the `png` crate, research R7)

**Storage**: the application's local `settings.json` (`micold_core::settings::Settings`), written by
the service (`persist_service_settings`); rasterised icon PNGs under the client's per-user data
directory (`directories::ProjectDirs` data dir, as `shell/startup.rs` resolves it). No network.

**Testing**: `cargo test` — core unit tests (`micold-core`, `mise run test-core`), daemon unit and
integration tests (`crates/micold-daemon/tests/`), client `tests/` for render-free logic (text,
rasteriser, settings reducer, backend request mapping), protocol round-trip and schema-hash tests,
geometry/contrast gates for the Settings row, and quickstart §B visual pass for what needs eyes.

**Target Platform**: Linux (X11, Wayland), macOS, Windows desktop; service on the host or in the
sandbox container

**Project Type**: desktop application (iced client + local session service)

**Performance Goals**: classification is O(1) per activity event; the icons are rasterised once per
client run (four glyphs, at most 256×256), not per notification

**Constraints**: offline; no new decision logic in render glue (Principle I exception); core stays
platform-agnostic (Principle VI); protocol versioning per 039 research R10 (no `#[serde(default)]`
on wire types)

**Scale/Scope**: tens of concurrent sessions; four fixed kinds

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | How |
|---|---|---|
| **I. Test-First** | PASS | Every decision is in tested render-free code: `TurnClock` and `NotificationKind` (core), `Views` kind-aware grant and error-notice target (daemon unit tests), the service's note/claim/notice paths (daemon `tests/`), `notification_text`, the rasteriser and its contrast/distinctness checks, the Settings reducer and the backends' request mapping (client `tests/`). Only the Settings row's rendering and the backends' system calls are glue, covered by quickstart §B. |
| **II. Multi-Session** | PASS | `TurnClock` lives on each live session; the pending kinds in `Views` are keyed by `SessionId`; one event yields one kind for one session (spec Edge Cases "Many sessions at once"). No state is shared between sessions. |
| **III. Worktree Integration** | PASS (n/a) | No file or VCS operation; the notification names the worktree as 039 does. |
| **IV. Local-First** | PASS | The switches live in the local `settings.json`; icons are rasterised from the bundled font and written to the local data dir; nothing leaves the device. |
| **V. Rust + iced** | PASS | Rust only. `NotificationKind` is a closed enum, so a fifth or missing kind is a compile error; `NotificationKinds` is a struct of four named booleans, not a map, so a kind without a switch is unrepresentable. |
| **VI. Cross-Platform** | PASS | Kinds, defaults and switches are platform-free core/service logic. The one platform difference — how an icon reaches the OS — sits inside the existing `shell/desktop_notify/{linux,macos,windows}.rs` backends behind `DesktopNotifier`. CI builds and tests all three. |
| **VII. Documentation** | PASS | `docs/user-guide/settings.md` (§Desktop notifications: kinds, defaults, switches, master switch, unread independence) and `docs/user-guide/icons.md` (the four kind icons) change in the milestone that ships each behaviour. |
| **VIII. Components** | PASS | The kind row is the shared `Checkbox` extended with a chainable `.icon(Icon)` builder method, not a new widget; the icons are `Icon` variants of the shared vocabulary. The showcase's Checkbox section gains the icon variant in both themes, enabled and disabled. |

Post-design re-check (after Phase 1): unchanged, all PASS. No Complexity Tracking entries.

## Project Structure

### Documentation (this feature)

```text
specs/613-notification-kinds/
├── plan.md              # This file
├── research.md          # Phase 0 decisions
├── data-model.md        # Phase 1 entities
├── quickstart.md        # Phase 1 validation guide (Part A automated, Part B visual)
├── contracts/
│   ├── wire.md          # protocol additions (versions 30–32, one per milestone)
│   ├── classification.md# TurnClock rules, error endings, service note/claim/notice rules
│   └── notification.md  # text, icons, backends, Settings row
└── tasks.md             # Phase 2 (tasks unit)
```

### Source Code (repository root)

```text
crates/micold-core/src/
├── attention.rs            # + NotificationKind, NotificationKinds, LONG_TASK_THRESHOLD,
│                           #   TurnClock, TurnChange; notification_text gains `kind`
├── settings.rs             # + Settings::notification_kinds (per-field serde defaults)
└── protocol/
    ├── messages.rs         # AttentionGranted.kind, DaemonMsg::SessionErrorNotice,
    │                       #   DaemonSettings.notification_kinds, SettingsSet.notification_kinds
    └── version.rs          # PROTOCOL_VERSION 29 → 30 → 31 → 32 (M1, M2, M3)

crates/micold-daemon/src/
├── activity.rs             # ActivityEvent::Ended gains `error: bool`; copilot_event marks
│                           #   session.error; HookKind → TurnChange mapping
├── attention.rs            # Views: pending kind per (session, seq), kind-aware grant/note,
│                           #   error_notice_target
├── state.rs                # LiveSession.turn; note_activity classifies; the give-up path and
│                           #   an error `Ended` send SessionErrorNotice; set_notification_kinds
├── catalog.rs              # settings accessors and persistence of notification_kinds
├── hooks.rs                # classify_hook: "SubagentStop" → Ignored; settings_json drops
│                           #   SubagentStop: HooksMap.subagent_stop field and its doc comment
│                           #   (FR-024, research R3)
└── server.rs               # SettingsSet.notification_kinds

crates/micold-client/src/
├── icons.rs                # + Icon::{NeedsPermission, SessionError, LongTaskFinished, TurnFinished}
├── notification_icon.rs    # new, render-free: kind → icon, rasterise (tiny-skia + ttf-parser),
│                           #   tile colours, contrast/distinctness helpers
├── features/attention.rs   # DesktopNotification.kind; error notice → notification
├── features/settings.rs    # draft + Msg::NotificationKindToggled(kind, bool)
├── app.rs                  # attention_notification(session, kind)
├── shell/daemon_sync.rs    # AttentionGranted{kind}, SessionErrorNotice
├── shell/desktop_notify/   # mod.rs writes icon files once per run; linux.rs image-data hint;
│                           #   windows.rs Toast::icon; macos.rs image_path
├── ui/material/checkbox.rs # + .icon(Icon) builder method
├── ui/settings/environment.rs # four kind rows under Desktop notifications
└── showcase/sections/controls.rs # Checkbox with icon, enabled/disabled, both themes

docs/user-guide/settings.md, docs/user-guide/icons.md
specs/010-daemon-session-persistence/contracts/hooks.md  # SubagentStop no longer registered/ignored
```

**Structure Decision**: the existing three-crate split. Pure decisions in `micold-core`
(classification, kinds, defaults, text) and in the daemon's pure `Views`; I/O in the daemon's
`state.rs` and the client's `shell/`; the one new client module (`notification_icon.rs`) is
render-free so `tests/` reach it.

## Requirement map

| FR | Where |
|---|---|
| FR-001, FR-002, FR-003 | `TurnClock` (contract [classification.md](./contracts/classification.md) C1–C6); `note_activity` |
| FR-004 | C7–C9: `ActivityEvent::Ended { error }`, the crash-loop give-up path |
| FR-005, FR-006 | C10–C12: `Views::note_event`/`grant` with the kind's switch; 039 claim rules unchanged |
| FR-007 | C13–C14: `SessionErrorNotice` to one window, only while one is connected |
| FR-008 | [notification.md](./contracts/notification.md) T1–T3: `notification_text(kind, …)` |
| FR-009, FR-012 | notification.md S1–S4: Settings rows, disabled under the master switch |
| FR-010, FR-011 | data-model `NotificationKinds`; `settings.rs` serde defaults; wire W5 |
| FR-013 | C10–C12 (decided at note and claim time, read live from the catalog); C15 (turning on) |
| FR-014 | `NotificationKinds` has no provider dimension |
| FR-015, FR-016, FR-017 | notification.md I1–I8: `Icon` variants, rasteriser, backends |
| FR-018 | C10: unread and `attention_seq` set before and independent of the kind |
| FR-024 | C17: `classify_hook` ignores `SubagentStop`; research R3 consumer table |
| FR-019, FR-020 | 039's reveal and N4 apply unchanged; `SessionErrorNotice` reuses the same seam |
| FR-021 | platform-free classification; backends only differ in icon delivery |
| FR-022 | `Checkbox::icon`, showcase Checkbox section, `Icon` vocabulary |
| FR-023 | `docs/user-guide/settings.md`, `docs/user-guide/icons.md` |

## Test strategy

| Layer | Covers |
|---|---|
| Core unit (`micold-core`, `mise run test-core`) | `TurnClock` rules C1–C6 incl. threshold boundary, pause inclusion, unknown start; `NotificationKind::ALL` order, names, descriptions, defaults; `NotificationKinds::is_on`; settings load: fresh, pre-feature file, unreadable file (FR-010); `notification_text` per kind (FR-008); wire round-trip and schema hash |
| Daemon unit (`attention.rs`, `activity.rs`, `hooks.rs`) | kind-aware `note_event`/`grant`, pending-kind pruning, `error_notice_target`; `copilot_event` error flag; `classifies_hook_event_names` asserts `SubagentStop` → `Ignored`, the settings test asserts it is not registered (FR-024) |
| Daemon integration (`crates/micold-daemon/tests/`) | over a real connection: kinds per hook sequence (SC-001 shape), a `SubagentStop` POST mid-turn and while paused changes no signal, unread or kind and a long turn across it is still **Long task finished** (FR-024), switches off/on (SC-002, FR-013), master off, unread unchanged by switches (SC-003), error notice on give-up and Copilot `session.error`, none on clean exit/close, none with no window, settings persistence across service restart (SC-006) |
| Client `tests/` | rasteriser: tile contrast ≥3:1 vs white and black, glyph vs tile, pairwise distinctness of the four masks at 16×16; `Icon` codepoints present in the font; settings reducer toggles and master-off disabling; backend request mapping carries the icon (Linux hint, Windows image entry, macOS image path); the granted/error notification carries its kind |
| Geometry/contrast gates | the Settings kind row: icon role contrast ≥3:1 in light and dark (`composition_contrast`), row anatomy |
| Quickstart §B (visual pass) | the four notifications on a real desktop, icons recognisable in light and dark desktop themes, Settings rows and showcase |
